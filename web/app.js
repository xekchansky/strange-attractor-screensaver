// Page glue: the frame loop, the DOM overlay and the URL-backed controls.
//
// The URL is the single source of state. The attractor, the seed and the
// backend live in the query string, so reloading — or sharing the link —
// reproduces the same run. Rust owns the simulation; this file only drives it.

import init, { catalog_json, start } from "./pkg/attractor_wasm.js";

const stage = document.getElementById("stage");
const overlay = document.getElementById("overlay");
const status = document.getElementById("status");
const attractorPicker = document.getElementById("attractor");
const searchInput = document.getElementById("search");
const seedInput = document.getElementById("seed");
const reseedButton = document.getElementById("reseed");

const query = new URLSearchParams(location.search);

/** A fresh 64-bit seed as a decimal string. */
function randomSeed() {
  const words = new Uint32Array(2);
  crypto.getRandomValues(words);
  return ((BigInt(words[0]) << 32n) | BigInt(words[1])).toString();
}

/**
 * Resize the canvas to its CSS size in device pixels.
 * @returns {[number, number]} backing-store size
 */
function fitCanvas() {
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(1, Math.round(stage.clientWidth * ratio));
  const height = Math.max(1, Math.round(stage.clientHeight * ratio));
  if (stage.width !== width || stage.height !== height) {
    stage.width = width;
    stage.height = height;
  }
  return [width, height];
}

/** Show a fatal message instead of a canvas. */
function showStatus(message) {
  status.textContent = message;
  status.hidden = false;
}

/** Rebuild the picker so only entries matching `filter` are offered. */
function fillPicker(entries, selected, filter = "") {
  const needle = filter.trim().toLowerCase();
  const matches = entries.filter(
    (entry) =>
      entry.id === selected ||
      !needle ||
      entry.id.includes(needle) ||
      entry.name.toLowerCase().includes(needle),
  );
  attractorPicker.replaceChildren(
    new Option("Random each time", ""),
    ...matches.map((entry) => new Option(entry.name, entry.id)),
  );
  attractorPicker.value = selected;
}

/** Navigate to the same page with one query parameter changed. */
function navigate(changes) {
  const next = new URLSearchParams(location.search);
  for (const [key, value] of Object.entries(changes)) {
    if (value === null || value === "") {
      next.delete(key);
    } else {
      next.set(key, value);
    }
  }
  location.search = next.toString();
}

// Give the page a seed even when the link carries none, so what is on screen
// can be shared by copying the address bar.
let seed = query.get("seed");
if (!seed || !/^\d+$/.test(seed)) {
  seed = randomSeed();
  query.set("seed", seed);
  history.replaceState(null, "", `?${query.toString()}`);
}
seedInput.value = seed;

const selected = query.get("attractor") || "";
fitCanvas();

// `stage` and `error` are diagnostics: a browser test that times out waiting
// for frames reads them to tell "the bundle loaded", "the GPU refused the
// canvas" and "a frame raised" apart.
const state = {
  frames: 0,
  backend: "starting",
  adapter: "",
  attractorId: selected,
  attractorName: "",
  seed,
  stage: "module",
  error: "",
};
window.__attractorDemo = state;

let demo;
try {
  await init();
  state.stage = "wasm-ready";

  const catalog = JSON.parse(catalog_json());
  fillPicker(catalog, selected);

  searchInput.addEventListener("input", () => {
    fillPicker(catalog, attractorPicker.value, searchInput.value);
  });

  state.stage = "starting";
  demo = await start(stage, location.search, BigInt(seed));
  state.stage = "running";
} catch (error) {
  state.stage = "failed";
  state.error = String(error);
  console.error(error);
  showStatus(`Cannot start the demo: ${error}`);
  throw error;
}

state.backend = demo.backend();
state.adapter = demo.adapter();
state.attractorName = demo.attractor_name();

attractorPicker.addEventListener("change", () => {
  navigate({ attractor: attractorPicker.value });
});

seedInput.addEventListener("change", () => {
  if (/^\d+$/.test(seedInput.value)) {
    navigate({ seed: seedInput.value });
  } else {
    seedInput.value = seed;
  }
});

reseedButton.addEventListener("click", () => {
  navigate({ seed: randomSeed() });
});

new ResizeObserver(() => {
  const [width, height] = fitCanvas();
  demo.resize(width, height);
}).observe(stage);

// The first callback establishes the clock origin: the animation must not
// depend on the epoch offset between performance.now() and Date.now().
let previous = null;
let overlayDrawnAt = 0;

function frame(timestamp) {
  const dt = previous === null ? 0 : Math.min((timestamp - previous) / 1000, 0.1);
  previous = timestamp;
  try {
    demo.tick(dt);
    demo.render();
  } catch (error) {
    state.stage = "failed";
    state.error = String(error);
    console.error(error);
    showStatus(`The demo stopped: ${error}`);
    return;
  }

  state.frames = demo.frames();
  state.attractorId = demo.attractor_id();
  state.attractorName = demo.attractor_name();
  if (timestamp - overlayDrawnAt > 200) {
    overlay.textContent = demo.overlay();
    overlayDrawnAt = timestamp;
  }
  requestAnimationFrame(frame);
}

requestAnimationFrame(frame);
