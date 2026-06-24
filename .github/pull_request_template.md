<!--
PR title must be a Conventional Commit (e.g. `feat(epg): ...`, `fix(player): ...`).
It becomes the squash-merge commit and drives the next release version.
-->

## What & why

<!-- Briefly describe the change and the motivation. -->

## Checklist

- [ ] A test was written **before** the implementation and now passes (TDD).
- [ ] `cargo fmt --all -- --check` is clean.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean.
- [ ] `cargo test --all` passes.
- [ ] If the frontend changed, `dx build` succeeds (UI still compiles to WASM).
- [ ] New domain logic landed in `cathode-core`, not in a command handler or component.
- [ ] `cathode-core` still builds for `wasm32-unknown-unknown` (no native-only dep leaked).
- [ ] Any new backend call from the UI goes through a typed wrapper in `src/bindings.rs`.
