import { mount } from "svelte";
import { app } from "./lib/store.svelte";
import "./app.css";
import App from "./App.svelte";

const inTauri = "__TAURI_INTERNALS__" in window && !!(window as any).__TAURI_INTERNALS__?.invoke;

async function start() {
  // Outside Tauri (a plain browser during development), run against mock data.
  if (import.meta.env.DEV && !inTauri) {
    const { installMocks } = await import("./dev/mock");
    const params = new URLSearchParams(location.search);
    const demo = params.has("demo");
    installMocks(demo);
    const view = params.get("view");
    if (view) app.view = view;
    if (demo) (await import("./dev/demo")).startDemoFeed();
  }
  mount(App, { target: document.getElementById("app")! });
}

start();
