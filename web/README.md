# Web demo

The GitHub Pages build of the screensaver: the same Rust player
(`attractor-player`) compiled to WebAssembly, drawing into a `<canvas>`.
The overlay is DOM text, not engine text (design decision D8).

## Layout

| Path | Contents |
| --- | --- |
| `index.html`, `styles.css` | The page and its dark, full-screen layout. |
| `app.js` | The `requestAnimationFrame` loop, the DOM overlay and the URL-backed controls. |
| `build.sh` | Builds `rust/crates/attractor-wasm` for `wasm32-unknown-unknown` and runs `wasm-bindgen` into `pkg/`. |
| `tests/` | Playwright checks: non-blank canvas, animation, WebGL2 fallback, clean console. |

`pkg/` is generated and git-ignored. The page imports it directly, so open it
over HTTP (ES modules and WebAssembly do not load from `file://`).

## Build and run

```bash
# once: the wasm target and a matching wasm-bindgen CLI
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130   # must match the crate

./web/build.sh
cd web && python3 -m http.server 4173
# open http://127.0.0.1:4173/
```

`build.sh` also runs `wasm-opt -Oz` when the binary is on `PATH`; the bundle
works without it.

## URL parameters

The address bar is the whole state, so a link reproduces the frame it shows:

| Parameter | Meaning |
| --- | --- |
| `attractor` | Catalog id (`lorenz`, `chua`, `sprott_a`, …). Missing or unknown means a random attractor. |
| `seed` | Unsigned 64-bit seed for the parameter sampling and the camera. The page invents one and writes it into the URL when the link has none. |
| `backend` | `webgl2` forces the WebGL2 fallback; anything else lets wgpu prefer WebGPU. |

## Browser tests

```bash
cd web
npm install
npx playwright install chromium
npm test
```

The suite starts the static server itself. It runs the WebGL2 path explicitly
(`?backend=webgl2`) and asserts the backend actually used, so a browser
without WebGPU still covers the fallback the ADR requires.
