# Golden view fixtures from the Swift oracle

`view_fixtures.json` pins the Rust view layer to the Swift `CameraController`
and `TraceRenderer`:

| Section | Contents | Rust test | Agreement |
| --- | --- | --- | --- |
| `camera` | fourteen `SCNNode` positions of the orbit camera around a fixed nine-point trace (azimuth `0.7`, elevation `π/5`, `dt = 1/60`, rotation speed `0.35`) | `tests/view_fixtures.rs` | bit-identical |
| `trail` | the exact `SCNGeometry` position/color/index buffers for five cases: `trail` at offset `0.0` and `0.3`, `age`, `depth` and `spatial` at `0.3` | `tests/view_fixtures.rs` | bit-identical |

Numbers are stored with `%.17g`, so they round-trip exactly. Positions and
colors are `Float32` values read back from `SCNGeometrySource`; the camera
positions are the `SCNNode` getter values, which SceneKit stores as `Float32`
even though its API is `CGFloat`-typed. The Rust port rounds through `f32` at
the same points, which is why the comparison is exact rather than approximate.

Do not edit the file by hand.

## Provenance

- Generated on 2026-11-12 from the Swift sources on the `rust-port` branch
  (the camera and renderer are unchanged from the pre-port `main`).
- Generator: `Render/Tests/AttractorRenderTests/ViewFixtureDumpTests.swift`,
  a Swift Testing suite disabled unless `ATTRACTOR_DUMP_FIXTURES=1`.

## Regenerating

From the repository root:

```bash
cd Render
ATTRACTOR_DUMP_FIXTURES=1 swift test --filter ViewFixtureDumpTests
```

The generator resolves its output path from `#filePath`, so the working
directory does not matter. Inside a restricted sandbox SwiftPM cannot start
its own nested sandbox; the file committed here was produced by compiling the
same source bodies with `swiftc` directly:

```bash
# Copies of Core/Sources/AttractorCore/{State,TraceBuffer,TraceColorMode}.swift
# and Render/Sources/AttractorRender/{CameraController,TraceRenderer}.swift
# with the `import AttractorCore` lines removed, plus a `main.swift` holding
# the same body as ViewFixtureDumpTests.dumpViewFixtures.
swiftc -O *.swift -o view-dump
./view-dump rust/crates/attractor-view/tests/fixtures/view_fixtures.json
```

Both paths call the same oracle functions, so the output is identical.
