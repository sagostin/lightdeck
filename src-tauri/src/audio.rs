//! Audio input + analysis: user-defined detection bands (each with a level and
//! onset detector), overall levels, energy trend and a beat clock with BPM.

use crate::model::BandDef;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, FftPlanner};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const FFT: usize = 2048;
const HOP: usize = 512;
pub const SPECTRUM_BINS: usize = 96;
/// Spectrum display range; bin i covers SPEC_LO * (SPEC_HI/SPEC_LO)^(i/N).
pub const SPEC_LO: f32 = 20.0;
pub const SPEC_HI: f32 = 20000.0;

#[derive(Debug, Clone, Serialize, Default)]
pub struct BandOut {
    pub id: String,
    /// 0..1 smoothed level (after band gain).
    pub level: f32,
    /// Current onset strength and the adaptive threshold it must beat (same units).
    pub flux: f32,
    pub threshold: f32,
    /// Monotonic hit counter.
    pub hits: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Features {
    pub running: bool,
    pub device: String,
    pub sample_rate: u32,
    pub error: Option<String>,
    /// 0..1 smoothed overall level.
    pub level: f32,
    /// Short-term energy relative to the long-term average (1.0 = average).
    pub energy: f32,
    pub bands: Vec<BandOut>,
    pub beats: u64,
    pub bpm: f32,
    /// 0..1 position within the current beat.
    pub beat_phase: f32,
    pub spectrum: Vec<f32>,
}

impl Features {
    pub fn band(&self, id: &str) -> Option<&BandOut> {
        self.bands.iter().find(|b| b.id == id)
    }
}

pub struct AudioParams {
    gain_bits: AtomicU32,
    sens_bits: AtomicU32,
    bands: Mutex<Vec<BandDef>>,
    bands_version: AtomicU64,
}

impl AudioParams {
    pub fn set(&self, gain_db: f32, sensitivity: f32, bands: &[BandDef]) {
        self.gain_bits.store(10f32.powf(gain_db / 20.0).to_bits(), Ordering::Relaxed);
        self.sens_bits.store(sensitivity.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        let mut b = self.bands.lock().unwrap();
        if b.as_slice() != bands {
            *b = bands.to_vec();
            self.bands_version.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn gain(&self) -> f32 {
        f32::from_bits(self.gain_bits.load(Ordering::Relaxed))
    }
    fn sens(&self) -> f32 {
        f32::from_bits(self.sens_bits.load(Ordering::Relaxed))
    }
}

pub struct Audio {
    pub features: Arc<Mutex<Features>>,
    pub params: Arc<AudioParams>,
    cmd: Sender<Option<String>>,
}

pub fn list_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

impl Audio {
    pub fn start(device: Option<String>, gain_db: f32, sensitivity: f32, bands: &[BandDef]) -> Self {
        let features = Arc::new(Mutex::new(Features::default()));
        let params = Arc::new(AudioParams {
            gain_bits: AtomicU32::new(0),
            sens_bits: AtomicU32::new(0),
            bands: Mutex::new(vec![]),
            bands_version: AtomicU64::new(0),
        });
        params.set(gain_db, sensitivity, bands);
        let (tx, rx) = channel();
        let f = features.clone();
        let p = params.clone();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || audio_thread(rx, f, p))
            .expect("spawn audio thread");
        let _ = tx.send(device);
        Self { features, params, cmd: tx }
    }

    pub fn select_device(&self, device: Option<String>) {
        let _ = self.cmd.send(device);
    }
}

fn audio_thread(rx: Receiver<Option<String>>, features: Arc<Mutex<Features>>, params: Arc<AudioParams>) {
    let mut _stream: Option<cpal::Stream> = None;
    while let Ok(dev) = rx.recv() {
        _stream = None; // drop old stream first
        match open_stream(dev, features.clone(), params.clone()) {
            Ok(s) => _stream = Some(s),
            Err(e) => {
                let mut f = features.lock().unwrap();
                f.running = false;
                f.error = Some(e);
            }
        }
    }
}

fn open_stream(
    want: Option<String>,
    features: Arc<Mutex<Features>>,
    params: Arc<AudioParams>,
) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = match &want {
        Some(name) => host
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().map(|n| &n == name).unwrap_or(false))
            .ok_or_else(|| format!("Input device \"{name}\" not found"))?,
        None => host.default_input_device().ok_or("No default input device")?,
    };
    let name = device.name().unwrap_or_default();
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    let sr = config.sample_rate().0;
    let channels = config.channels() as usize;
    let mut analyzer = Analyzer::new(sr, features.clone(), params);

    let err_features = features.clone();
    let err_fn = move |e: cpal::StreamError| {
        let mut f = err_features.lock().unwrap();
        f.error = Some(e.to_string());
    };

    macro_rules! build {
        ($t:ty, $conv:expr) => {
            device.build_input_stream(
                &config.clone().into(),
                move |data: &[$t], _: &_| {
                    for frame in data.chunks(channels) {
                        let s: f32 = frame.iter().map(|x| $conv(*x)).sum::<f32>() / channels as f32;
                        analyzer.push(s);
                    }
                },
                err_fn,
                None,
            )
        };
    }
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build!(f32, |x: f32| x),
        cpal::SampleFormat::I16 => build!(i16, |x: i16| x as f32 / 32768.0),
        cpal::SampleFormat::U16 => build!(u16, |x: u16| (x as f32 - 32768.0) / 32768.0),
        cpal::SampleFormat::I32 => build!(i32, |x: i32| x as f32 / 2147483648.0),
        other => return Err(format!("Unsupported sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    {
        let mut f = features.lock().unwrap();
        f.running = true;
        f.device = name;
        f.sample_rate = sr;
        f.error = None;
    }
    Ok(stream)
}

struct Range {
    lo: usize,
    hi: usize,
    weight: f32,
    prev: Vec<f32>,
}

struct Band {
    def: BandDef,
    gain: f32,
    ranges: Vec<Range>,
    flux_hist: VecDeque<f32>,
    last_onset: Instant,
    level: f32,
    above: bool,
    hits: u64,
}

impl Band {
    fn new(sr: u32, def: &BandDef, hits: u64) -> Self {
        let bin = |hz: f32| ((hz / sr as f32) * FFT as f32).round().clamp(1.0, (FFT / 2 - 1) as f32) as usize;
        let ranges = def
            .ranges
            .iter()
            .map(|r| {
                let lo = bin(r.lo_hz.min(r.hi_hz));
                let hi = bin(r.hi_hz.max(r.lo_hz)).max(lo + 1);
                Range { lo, hi, weight: r.weight.clamp(0.0, 4.0), prev: vec![0.0; hi - lo] }
            })
            .collect();
        Self {
            def: def.clone(),
            gain: 10f32.powf(def.gain_db / 20.0),
            ranges,
            flux_hist: VecDeque::new(),
            last_onset: Instant::now(),
            level: 0.0,
            above: false,
            hits,
        }
    }

    /// Returns (onset?, detector value, threshold). For transient bands the
    /// detector is spectral flux; for level bands it's the smoothed level.
    fn process(&mut self, mags: &[f32], global_sens: f32) -> (bool, f32, f32) {
        let (mut flux, mut level, mut wsum) = (0.0, 0.0, 0.0);
        for r in &mut self.ranges {
            let (mut f, mut l) = (0.0, 0.0);
            for (i, m) in mags[r.lo..r.hi].iter().enumerate() {
                let v = (1.0 + 10.0 * m * self.gain).ln();
                f += (v - r.prev[i]).max(0.0);
                l += v;
                r.prev[i] = v;
            }
            let n = (r.hi - r.lo) as f32;
            flux += f / n * r.weight;
            level += l / n * r.weight;
            wsum += r.weight;
        }
        if wsum > 0.0 {
            flux /= wsum;
            level /= wsum;
        }
        let target = (level / 0.6).clamp(0.0, 1.0);
        let rel = self.def.release.clamp(0.0, 0.98);
        self.level = if target > self.level { target } else { self.level * rel + target * (1.0 - rel) };

        // band sensitivity, nudged by the global knob (0.5 = neutral)
        let sens = (self.def.sensitivity + (global_sens - 0.5)).clamp(0.0, 1.0);
        let gap_ok = self.last_onset.elapsed().as_millis() > self.def.min_gap_ms as u128;
        let (onset, value, threshold) = if self.def.detect == "level" {
            // Hysteresis: re-arm once the level drops 15% below the threshold.
            let thr = self.def.level_threshold.clamp(0.02, 1.0);
            let crossed = self.level > thr && !self.above;
            if self.level > thr {
                self.above = true;
            } else if self.level < thr * 0.85 {
                self.above = false;
            }
            (crossed && gap_ok, self.level, thr)
        } else {
            let len = self.flux_hist.len().max(1) as f32;
            let mean = self.flux_hist.iter().sum::<f32>() / len;
            let var = self.flux_hist.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / len;
            let k = 2.6 - 2.0 * sens; // 0..1 -> 2.6..0.6 standard deviations
            let thr = (mean + k * var.sqrt()).max(0.004);
            (self.flux_hist.len() > 20 && flux > thr && gap_ok, flux, thr)
        };
        self.flux_hist.push_back(flux);
        if self.flux_hist.len() > 50 {
            self.flux_hist.pop_front();
        }
        if !self.def.enabled {
            // muted: keep tracking so un-muting is instant, but report nothing
            self.level = 0.0;
            return (false, 0.0, threshold);
        }
        if onset {
            self.last_onset = Instant::now();
            self.hits += 1;
        }
        (onset, value, threshold)
    }

    fn flux(&self) -> f32 {
        self.flux_hist.back().copied().unwrap_or(0.0)
    }
}

struct Analyzer {
    sr: u32,
    buf: Vec<f32>,
    write: usize,
    filled: usize,
    fft: Arc<dyn rustfft::Fft<f32>>,
    window: Vec<f32>,
    bands: Vec<Band>,
    bands_version: u64,
    features: Arc<Mutex<Features>>,
    params: Arc<AudioParams>,
    level: f32,
    peak: f32,
    e_short: f32,
    e_long: f32,
    onset_env: VecDeque<f32>,
    frames_since_bpm: usize,
    bpm: f32,
    phase: f32,
    beats: u64,
    last_frame: Instant,
}

impl Analyzer {
    fn new(sr: u32, features: Arc<Mutex<Features>>, params: Arc<AudioParams>) -> Self {
        let window = (0..FFT)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FFT as f32).cos())
            .collect();
        let mut a = Self {
            sr,
            buf: vec![0.0; FFT],
            write: 0,
            filled: 0,
            fft: FftPlanner::new().plan_fft_forward(FFT),
            window,
            bands: vec![],
            bands_version: u64::MAX,
            features,
            params,
            level: 0.0,
            peak: 0.05,
            e_short: 0.0,
            e_long: 0.01,
            onset_env: VecDeque::new(),
            frames_since_bpm: 0,
            bpm: 120.0,
            phase: 0.0,
            beats: 0,
            last_frame: Instant::now(),
        };
        a.sync_bands();
        a
    }

    /// Rebuild band detectors when the user edits them, keeping hit counters.
    fn sync_bands(&mut self) {
        let v = self.params.bands_version.load(Ordering::Relaxed);
        if v == self.bands_version {
            return;
        }
        self.bands_version = v;
        let defs = self.params.bands.lock().unwrap().clone();
        let old: Vec<(String, u64)> = self.bands.iter().map(|b| (b.def.id.clone(), b.hits)).collect();
        self.bands = defs
            .iter()
            .map(|d| {
                let hits = old.iter().find(|(id, _)| id == &d.id).map(|(_, h)| *h).unwrap_or(0);
                Band::new(self.sr, d, hits)
            })
            .collect();
    }

    fn push(&mut self, s: f32) {
        let g = self.params.gain();
        self.buf[self.write] = s * g;
        self.write = (self.write + 1) % FFT;
        self.filled += 1;
        if self.filled >= HOP {
            self.filled = 0;
            self.analyse();
        }
    }

    fn analyse(&mut self) {
        self.sync_bands();
        let sens = self.params.sens();
        let ordered: Vec<f32> = (0..FFT).map(|i| self.buf[(self.write + i) % FFT]).collect();
        let mut spec: Vec<Complex<f32>> =
            ordered.iter().zip(&self.window).map(|(s, w)| Complex::new(s * w, 0.0)).collect();
        self.fft.process(&mut spec);
        let mags: Vec<f32> = spec[..FFT / 2].iter().map(|c| c.norm() / (FFT as f32 / 4.0)).collect();

        let mut outs = Vec::with_capacity(self.bands.len());
        let mut beat_onset = false;
        let mut env = 0.0;
        for (i, b) in self.bands.iter_mut().enumerate() {
            let (on, value, thr) = b.process(&mags, sens);
            // The first band (kick by default) anchors the beat clock; the first
            // two bands' flux feed the tempo estimate.
            if i == 0 {
                beat_onset = on;
                env += b.flux() * 2.0;
            } else if i == 1 {
                env += b.flux();
            }
            outs.push(BandOut { id: b.def.id.clone(), level: b.level, flux: value, threshold: thr, hits: b.hits });
        }

        let rms = (ordered.iter().map(|x| x * x).sum::<f32>() / FFT as f32).sqrt();
        self.peak = (self.peak * 0.9995).max(rms).max(0.02);
        let t = (rms / self.peak).clamp(0.0, 1.0);
        self.level = if t > self.level { t } else { self.level * 0.85 + t * 0.15 };

        self.e_short = self.e_short * 0.9 + rms * 0.1;
        self.e_long = (self.e_long * 0.999 + rms * 0.001).max(1e-4);

        // beat tracking: autocorrelation of onset envelope
        let frame_rate = self.sr as f32 / HOP as f32;
        self.onset_env.push_back(env);
        let max_len = (frame_rate * 6.0) as usize;
        while self.onset_env.len() > max_len {
            self.onset_env.pop_front();
        }
        self.frames_since_bpm += 1;
        if self.frames_since_bpm as f32 > frame_rate && self.onset_env.len() == max_len {
            self.frames_since_bpm = 0;
            if let Some(b) = estimate_bpm(&self.onset_env, frame_rate) {
                self.bpm = self.bpm * 0.7 + b * 0.3;
            }
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.phase += dt * self.bpm / 60.0;
        if beat_onset {
            if self.phase < 0.25 {
                self.phase *= 0.5;
            } else if self.phase > 0.75 {
                self.phase = 1.0;
            }
        }
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
            self.beats += 1;
        }

        let mut f = self.features.lock().unwrap();
        f.level = self.level;
        f.energy = self.e_short / self.e_long;
        f.bands = outs;
        f.beats = self.beats;
        f.bpm = self.bpm;
        f.beat_phase = self.phase;
        f.spectrum = log_bins(&mags, self.sr);
    }
}

fn estimate_bpm(env: &VecDeque<f32>, fr: f32) -> Option<f32> {
    let v: Vec<f32> = env.iter().copied().collect();
    let mean = v.iter().sum::<f32>() / v.len() as f32;
    let v: Vec<f32> = v.iter().map(|x| x - mean).collect();
    let (mut best, mut best_lag) = (0.0, 0usize);
    let lag_for = |bpm: f32| (fr * 60.0 / bpm).round() as usize;
    for lag in lag_for(175.0)..=lag_for(75.0) {
        let mut s = 0.0;
        for i in lag..v.len() {
            s += v[i] * v[i - lag];
        }
        if s > best {
            best = s;
            best_lag = lag;
        }
    }
    (best_lag > 0).then(|| {
        let mut bpm = fr * 60.0 / best_lag as f32;
        while bpm < 90.0 {
            bpm *= 2.0;
        }
        while bpm > 180.0 {
            bpm /= 2.0;
        }
        bpm
    })
}

fn log_bins(mags: &[f32], sr: u32) -> Vec<f32> {
    let nyq = sr as f32 / 2.0;
    let ratio = SPEC_HI / SPEC_LO;
    (0..SPECTRUM_BINS)
        .map(|i| {
            let f0 = SPEC_LO * ratio.powf(i as f32 / SPECTRUM_BINS as f32);
            let f1 = SPEC_LO * ratio.powf((i + 1) as f32 / SPECTRUM_BINS as f32);
            let b0 = (((f0 / nyq) * mags.len() as f32) as usize).min(mags.len() - 1);
            let b1 = (((f1 / nyq) * mags.len() as f32) as usize).max(b0 + 1).min(mags.len());
            let m = mags[b0..b1].iter().cloned().fold(0.0, f32::max);
            ((1.0 + 20.0 * m).ln() / 3.0).clamp(0.0, 1.0)
        })
        .collect()
}
