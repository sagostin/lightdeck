// Development only: when the UI is opened in a normal browser (not inside Tauri),
// fake the backend so screens can be viewed and checked. Open http://localhost:1420
// while `npm run dev` or `npx tauri dev` is running. Add ?view=<screen> to jump to one,
// and ?demo for the demo show with a fake audio feed (see demo.ts).
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import sample from "./sample-show.json";
import { demoShow } from "./demo";
import { demoDesign } from "./demo-design";
import type { Show } from "../lib/types";

export function installMocks(demo = false) {
  mockWindows("main");
  let show = (demo ? demoShow(structuredClone(sample) as unknown as Show) : structuredClone(sample)) as unknown as Record<string, unknown>;
  const manual: Record<string, Record<string, number>> = {};
  let userControllers: unknown[] = [];
  let library: unknown[] = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "get_show": return show;
      case "save_show": show = a.show as Record<string, unknown>; return null;
      case "get_manual": return manual;
      case "set_manual": {
        const f = a.fixture as string, c = a.channel as string;
        if (a.value === null) delete manual[f]?.[c];
        else (manual[f] ??= {})[c] = a.value as number;
        return null;
      }
      case "clear_manual": {
        if (a.fixture) delete manual[a.fixture as string];
        else for (const k of Object.keys(manual)) delete manual[k];
        return null;
      }
      case "project_info": return { path: null, name: demo ? "Club night" : "Browser preview", recent: [] };
      case "control_state": return { routes: {}, bands: {}, blackout: false, master: 1, manual, active_presets: [], fired: null };
      case "list_audio_devices": return ["Mock input"];
      case "get_band_presets": return [];
      case "zigbee_status": return { enabled: false, mode: "usb", connected: false, bridge_online: null, error: null, state: "Off", coordinator: null, permit_join_until: 0, devices: [], sent_per_sec: 0, backlog: 0, last_send_error: null, log: [] };
      case "midi_status": return { enabled: false, available: [], connected: [], error: null };
      case "get_fixture_library": return library;
      case "save_fixture_library": library = a.library as unknown[]; return null;
      case "fixture_export": return null;
      case "parse_fixture_file": {
        const v = JSON.parse(a.text as string);
        return v.format === "lightdeck-fixture" ? [v.profile] : v.format === "lightdeck-fixtures" ? v.profiles : [v];
      }
      case "get_controller_profiles": return userControllers;
      case "save_controller_profiles": userControllers = a.profiles as unknown[]; return null;
      case "design_check": return { plan: a.plan, warnings: [] };
      case "ai_design":
        return demo ? new Promise((r) => setTimeout(() => r(demoDesign()), 1200)) : Promise.reject("AI isn't available in the browser preview (try ?demo)");
      case "get_ai_settings": return { schema: "anthropic", base_url: "https://api.anthropic.com", api_key: demo ? "demo" : "", model: "claude-opus-5-5", auth: "x-api-key", extra_headers: {}, max_tokens: 16000 };
      case "zigbee_profiles": return [];
      case "plugin:event|listen": return Math.floor(Math.random() * 1e6);
      default: return null;
    }
  });
}
