// flatus — settings popover frontend.
//
// Vanilla TS. No framework. The webview is UI only — synthesis happens in Rust.
// Calls are via `@tauri-apps/api/core` invoke. Settings are loaded on mount and
// pushed back to Rust on every change.

import { invoke } from "@tauri-apps/api/core";

interface Settings {
  personality: string;
  volume: number;
  output: "speakers" | "headphones";
  quiet_start: number | null;
  quiet_end: number | null;
}

const $ = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;

const personalitySel = $<HTMLSelectElement>("personality");
const volumeSlider = $<HTMLInputElement>("volume");
const volumeReadout = $<HTMLSpanElement>("volume-readout");
const outputSel = $<HTMLSelectElement>("output");
const quietStart = $<HTMLInputElement>("quiet-start");
const quietEnd = $<HTMLInputElement>("quiet-end");
const fartNowBtn = $<HTMLButtonElement>("fart-now");

let current: Settings = {
  personality: "default",
  volume: 1,
  output: "headphones",
  quiet_start: null,
  quiet_end: null,
};

async function mount() {
  const personalities = await invoke<string[]>("list_personalities");
  // Drop the static fallback options (used when the page is rendered without
  // a Tauri runtime — e.g. screenshots or web previews) and replace with the
  // live list from Rust.
  personalitySel.innerHTML = "";
  for (const name of personalities) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = name;
    personalitySel.appendChild(opt);
  }

  const loaded = await invoke<Settings>("get_settings");
  current = loaded;
  personalitySel.value = current.personality;
  volumeSlider.value = String(current.volume);
  outputSel.value = current.output;
  quietStart.value = current.quiet_start?.toString() ?? "";
  quietEnd.value = current.quiet_end?.toString() ?? "";
  updateVolumeReadout();

  personalitySel.addEventListener("change", () => {
    current.personality = personalitySel.value;
    push();
  });
  volumeSlider.addEventListener("input", () => {
    current.volume = parseFloat(volumeSlider.value);
    updateVolumeReadout();
    push();
  });
  outputSel.addEventListener("change", () => {
    current.output = outputSel.value as Settings["output"];
    push();
  });
  quietStart.addEventListener("input", () => {
    current.quiet_start = quietStart.value ? parseInt(quietStart.value, 10) : null;
    push();
  });
  quietEnd.addEventListener("input", () => {
    current.quiet_end = quietEnd.value ? parseInt(quietEnd.value, 10) : null;
    push();
  });
  fartNowBtn.addEventListener("click", async () => {
    fartNowBtn.disabled = true;
    try {
      await invoke("fart_now");
    } finally {
      setTimeout(() => (fartNowBtn.disabled = false), 1000);
    }
  });
}

function push() {
  invoke("set_settings", { newSettings: current }).catch((e) => console.error(e));
}

function updateVolumeReadout() {
  volumeReadout.textContent = `${Math.round(current.volume * 100)}%`;
}

mount().catch((e) => console.error(e));
