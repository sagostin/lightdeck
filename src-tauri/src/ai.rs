//! Optional AI helper: turns arbitrary fixture docs (manual text, QLC+/OFL/other
//! formats) into LightDeck profiles, and suggests tags. Talks either the
//! Anthropic Messages API or an OpenAI-compatible chat API, through any base
//! URL (so proxies/gateways work).

use crate::model::Profile;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    /// "anthropic" | "openai"
    pub schema: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// "x-api-key" | "bearer" | "none"
    pub auth: String,
    pub extra_headers: HashMap<String, String>,
    pub max_tokens: u32,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            schema: "anthropic".into(),
            base_url: "https://api.anthropic.com".into(),
            api_key: String::new(),
            model: "claude-opus-5-5".into(),
            auth: "x-api-key".into(),
            extra_headers: HashMap::new(),
            max_tokens: 16000,
        }
    }
}

const SYSTEM: &str = r##"You convert DMX lighting fixture documentation into the LightDeck profile JSON format.

Profile shape:
{ "id": string, "manufacturer": string, "model": string,
  "kind": "laser"|"par"|"fog"|"moving_head"|"strobe"|"other",
  "notes": string,
  "channels": [ { "key": snake_case unique string, "name": string,
      "group": "intensity"|"color"|"pan"|"tilt"|"pattern"|"beam"|"effect"|"speed"|"strobe"|"mode"|"fog"|"other",
      "default": 0-255 (a safe resting value; for lasers the off/blank value),
      "capabilities": [ { "min": 0-255, "max": 0-255, "label": string, "color": "#rrggbb" or null, "tags": [string] } ],
      "values": [] } ],
  "modes": [ { "name": string, "channels": [channel keys in DMX order],
      "heads": [ { "name": "P1", "channels": [channel keys] } ] } ] }

Tag vocabulary for capabilities (use only what applies):
- "blocked": values the controller must never send, e.g. built-in auto/sound-active programs that fight external control, undefined/reserved ranges.
- "off": the value range that turns the output dark/off (used for blackout).
- "static", "dynamic": still vs. moving/animated behaviour.
- "strobe", "chase", "rotation", "zoom", "position", "colour_macro".
- "large_projection": ranges likely to throw the image wide/up (risky for low ceilings).
Capabilities must cover 0-255 without overlap where the docs allow. When a fixture has several independent projectors/heads, list one head per projector.
Reply with only the JSON object, no prose and no code fences."##;

pub async fn interpret(settings: &AiSettings, input: &str, instruction: &str) -> Result<Profile, String> {
    let user = format!("{instruction}\n\n--- FIXTURE SOURCE ---\n{input}");
    let text = complete(settings, SYSTEM, &user).await?;
    let json_str = extract_json(&text).ok_or("The model reply did not contain a JSON object")?;
    serde_json::from_str::<Profile>(json_str).map_err(|e| format!("Reply was not a valid profile: {e}"))
}

pub async fn test(settings: &AiSettings) -> Result<String, String> {
    complete(settings, "Reply with the single word: ok", "ping").await
}

async fn complete(s: &AiSettings, system: &str, user: &str) -> Result<String, String> {
    if s.base_url.trim().is_empty() {
        return Err("Set a base URL first".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;
    let base = s.base_url.trim_end_matches('/');
    let mut req = match s.schema.as_str() {
        "openai" => {
            let url = if base.ends_with("/chat/completions") { base.to_string() } else { format!("{base}/chat/completions") };
            client.post(url).json(&json!({
                "model": s.model,
                "max_tokens": s.max_tokens,
                "messages": [ {"role": "system", "content": system}, {"role": "user", "content": user} ]
            }))
        }
        _ => {
            let url = if base.ends_with("/v1/messages") { base.to_string() } else { format!("{base}/v1/messages") };
            let mut body = json!({
                "model": s.model,
                "max_tokens": s.max_tokens,
                "system": system,
                "messages": [ {"role": "user", "content": user} ]
            });
            let mut r = client.post(url).header("anthropic-version", "2026-10-21");
            // Server-side refusal fallback; only on the first-party API (proxies may reject betas).
            if base.contains("api.anthropic.com") {
                body["fallbacks"] = json!("default");
                r = r.header("anthropic-beta", "server-side-fallback-2026-07-01");
            }
            r.json(&body)
        }
    };
    if !s.api_key.is_empty() {
        req = match s.auth.as_str() {
            "bearer" => req.bearer_auth(&s.api_key),
            "none" => req,
            _ if s.schema == "openai" => req.bearer_auth(&s.api_key),
            _ => req.header("x-api-key", &s.api_key),
        };
    }
    for (k, v) in &s.extra_headers {
        req = req.header(k, v);
    }
    let resp = req.send().await.map_err(|e| format!("Request failed: {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| format!("Bad response ({status}): {e}"))?;
    if !status.is_success() {
        let msg = body.pointer("/error/message").and_then(|m| m.as_str()).unwrap_or("unknown error");
        return Err(format!("{status}: {msg}"));
    }
    match s.schema.as_str() {
        "openai" => body
            .pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .map(String::from)
            .ok_or_else(|| "No message content in reply".into()),
        _ => {
            if body["stop_reason"] == "refusal" {
                return Err("The model declined this request".into());
            }
            if body["stop_reason"] == "max_tokens" {
                return Err("Reply was cut off; raise max tokens in AI settings".into());
            }
            let text: String = body["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect();
            if text.is_empty() { Err("Empty reply".into()) } else { Ok(text) }
        }
    }
}

fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}
