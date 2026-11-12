# Golden formula fixtures

`formula_evaluation.json` records the Swift oracle's evaluation of every
catalog formula (`dx`, `dy`, `dz` for all 12 attractors) at eight sample
states: 288 entries. `tests/formula_fixtures.rs` re-evaluates each one with
the Rust evaluator and requires agreement within `1e-12` (relative, with an
absolute floor of `1e-12` for values near zero).

## Provenance

- Generated from the Swift sources at commit `dfbadb8` (the last commit that
  touched `Core/Sources`) on 2026-11-12.
- Generator: `Core/Tests/AttractorCoreTests/FormulaFixtureDumpTests.swift`,
  disabled unless `ATTRACTOR_DUMP_FIXTURES=1`.
- Bindings: parameters sit at the midpoint of their declared range; the eight
  states are `x = 1.3i - 3.7`, `y = 0.7i² - 2`, `z = 0.5i + 0.25` for
  `i = 0…7`. Values are stored with `%.17g`, so they round-trip exactly.
- Do not edit the file by hand. Regenerate it only when the evaluation
  contract changes, and update the commit above.

## Regenerating

From the repository root:

```bash
cd Core
ATTRACTOR_DUMP_FIXTURES=1 swift test --filter FormulaFixtureDumpTests
```

The generator resolves the output path from its own `#filePath`, so the
working directory does not matter. Inside a restricted sandbox (the coding
agent's environment) SwiftPM cannot start its own sandbox; add
`--disable-sandbox` there. A normal `swift test` without the environment
variable skips the suite and writes nothing.

After I12 retires `Core/`, regenerate from the archived oracle tag instead;
the Rust tests never need a live Swift toolchain.
