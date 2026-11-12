# Golden fixtures from the Swift oracle

Four files pin the Rust port to the Swift implementation:

| File | Contents | Rust test | Agreement |
| --- | --- | --- | --- |
| `formula_evaluation.json` | every catalog formula (`dx`, `dy`, `dz` for all 12 attractors) at eight sample states — 288 entries | `tests/formula_fixtures.rs` | `1e-12` relative, `1e-12` absolute floor |
| `trajectories.json` | 1000 RK4 steps for Lorenz, Chua and Aizawa, plus 500 Euler steps for Lorenz | `tests/trajectory_fixtures.rs` | `1e-9` relative, `1e-9` absolute floor |
| `randomizer_selections.json` | `Randomizer.select` for seeds 0…15 against the bundled catalog | `tests/randomizer_fixtures.rs` | bit-identical |
| `engine_traces.json` | the seeded selection and a 200-step `AttractorEngine` trajectory (trail capacity 128) for the same sixteen seeds | `tests/engine_fixtures.rs` | selection bit-identical, trace `1e-9` relative |

Values are stored with `%.17g`, so they round-trip exactly. Do not edit the
files by hand.

## Provenance

- Generated from the Swift sources at commit `dfbadb8` (the last commit that
  touched `Core/Sources`) on 2026-11-12.
- Formula generator: `Core/Tests/AttractorCoreTests/FormulaFixtureDumpTests.swift`.
- Numerical generators: `Core/Tests/AttractorCoreTests/NumericalFixtureDumpTests.swift`.
  Both suites are disabled unless `ATTRACTOR_DUMP_FIXTURES=1`.
- Formula bindings: parameters sit at the midpoint of their declared range;
  the eight states are `x = 1.3i - 3.7`, `y = 0.7i² - 2`, `z = 0.5i + 0.25`
  for `i = 0…7`.
- Trajectories use `DebugConfig.selection`: midpoint parameters, midpoint
  hyperparameters (so the step size is the midpoint of its range: `0.007` for
  Lorenz and Aizawa, `0.0006` for Chua) and the initial state offset
  `(+0.01, -0.01, +0.01)` from the range midpoint, clamped to the range.
  `states[0]` is the initial state; every following entry is one integration
  step.
- Randomizer entries record the descriptor id, every sampled parameter, all
  five hyperparameter values and the initial state. The Rust port reproduces
  the Swift standard library's `next(upperBound:)` (Lemire), `Int.random(in:)`
  and `Double.random(in:using:)` algorithms, so the comparison is exact rather
  than a range check (`tests/splitmix64.rs` pins those primitives separately).
- Engine entries reuse the same sixteen seeded selections and record 200 RK4
  steps each, starting with the initial state. The trail capacity is 128, so
  the Rust test also checks that the ring keeps the newest 128 states in order.
  This is the end-to-end gate: the same seed yields the same attractor,
  parameters, initial state and trace as the Swift engine.

Short horizons are deliberate: a chaotic flow amplifies the last bit of a
`Double`, so long runs are covered by invariants (finite, bounded, still
moving) instead of point-by-point equality.

## Regenerating

From the repository root:

```bash
cd Core
ATTRACTOR_DUMP_FIXTURES=1 swift test --filter FormulaFixtureDumpTests
ATTRACTOR_DUMP_FIXTURES=1 swift test --filter NumericalFixtureDumpTests
```

`NumericalFixtureDumpTests` writes `trajectories.json`,
`randomizer_selections.json` and `engine_traces.json`; the three tests can
also be filtered individually (`--filter dumpEngineTraces`, and so on).

```bash
```

The generators resolve their output paths from `#filePath`, so the working
directory does not matter. Inside a restricted sandbox (the coding agent's
environment) SwiftPM cannot start its own sandbox; add `--disable-sandbox`
there, or drive the same `Core/Sources` functions from a standalone `swiftc`
program. A normal `swift test` without the environment variable skips the
suites and writes nothing.

After I12 retires `Core/`, regenerate from the archived oracle tag instead;
the Rust tests never need a live Swift toolchain.
