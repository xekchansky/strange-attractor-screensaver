# Rust port — status

Design and iteration plan:
[`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](../docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md).

Protocol for a fresh session: read the design, re-run the previous
iteration's verification command, implement exactly one iteration, update this
file, and commit atomically.

## Current state

- Branch: `rust-port`
- Last completed iteration: **I0 — workspace skeleton and CI** (2026-11-12)
- Next iteration: **I0.5 — renderer and web spike**

## Verification

From `rust/`:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same three commands in the `rust` job of
`.github/workflows/ci.yml`.

## Iterations

- [x] I0 — workspace skeleton and CI
- [ ] I0.5 — renderer and web spike, timeboxed
- [ ] I1 — data model and catalog
- [ ] I2 — formula parser
- [ ] I3 — numerical core
- [ ] I4 — engine, selection and health monitor
- [ ] I5 — view layer, no GPU
- [ ] I6 — wgpu renderer and CLI capture
- [ ] I7a — player and C ABI
- [ ] I7b — macOS host migration
- [ ] I8 — Windows `.scr`
- [ ] I9 — web demo on GitHub Pages
- [ ] I10 — Linux
- [ ] I11 — Android live wallpaper
- [ ] I12 — distribution, docs and cleanup

## Swift oracle

The oracle commit is not pinned yet. It is recorded here when I2 starts, so
the golden fixtures stay reproducible after `Core/` is retired.

## Notes and risks

- I0 created the Cargo workspace with eight empty crates and no external
  dependencies. Every crate has one smoke test asserting its package name,
  which proves the member list and manifests are wired correctly.
- The `rust` CI job runs on `ubuntu-latest`. It is independent of the Swift
  jobs, which still run on `macos-26` unchanged.
- The workflow's `push` trigger now includes `rust-port`; without it the Rust
  job would only run on pull requests and the iteration's "CI green" could
  not be observed on the working branch.
- The toolchain is plain stable Rust (edition 2024); no version is pinned
  yet. I0 was verified locally with rustc 1.99.1. Pin a version once the
  renderer spike fixes the dependency set (I0.5).
