# Headless capture snapshot

`cli_lorenz_snapshot.json` pins one complete seeded capture without a GPU:

| Field | Meaning |
| --- | --- |
| `attractor`, `seed`, `warmup_steps`, `frames` | the flags of the recorded run |
| `parameters_bits`, `initial_state_bits` | the seeded selection, as `f64` bit patterns |
| `point_count`, `vertex_count`, `triangle_count` | ribbon topology |
| `ribbon_radius_bits` | world-space ribbon half-thickness (`f64` bits) |
| `digest` | FNV-1a 64 over the `f32` bits of every position and color |
| `first_vertex`, `last_vertex` | position and color `f32` bits of two buffer ends |

The run uses `--attractor lorenz --seed 42 --cam-azimuth 0 --cam-elevation 0
--cam-speed 0 --trail-color-speed 0 --color-mode trail --warmup-steps 5000
--frames 60`. Pinning the camera to the +x axis keeps `sin`/`cos` exactly zero
or one, so the buffers are bit-identical across libm implementations; with the
default elevation the last `f32` bits depend on the platform's trig.

The snapshot test builds that run in-process, so it needs no capture request.
A manual CLI reproduction does, because the binary always writes a file; add
e.g. `--width 320 --height 240 --screenshot out.png`. The output size and the
PNG affect neither the geometry nor the pinned digest.

Values are bit patterns rather than decimals on purpose: serde_json's default
float parser is fast but not always correctly rounded, so a decimal round trip
is not an exact contract.

Regenerate after an intentional geometry change:

```bash
cd rust
ATTRACTOR_DUMP_FIXTURES=1 cargo test -p attractor-cli --test headless_snapshot
```
