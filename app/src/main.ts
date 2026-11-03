import { mount } from "svelte";
import "./lib/theme.css";
import App from "./App.svelte";
import { invoke } from "@tauri-apps/api/core";

// Errors in the window go to the terminal the app runs from.
const report = (m: unknown) => invoke("log", { message: String(m) }).catch(() => {});
window.addEventListener("error", (e) => report(`${e.message} @ ${e.filename}:${e.lineno}\n${e.error?.stack ?? ""}`));
window.addEventListener("unhandledrejection", (e) => report(`unhandled: ${e.reason?.stack ?? e.reason}`));

export default mount(App, { target: document.getElementById("app")! });
