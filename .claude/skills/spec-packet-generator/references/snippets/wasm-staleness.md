---
when: Include in `design.md` when an edited path feeds guest WASM artifacts.
keywords: WASM, stale guest, build-guests, architecture constraint
---

# Guest WASM Staleness Snippet

Applies to:

- `crates/slicer-schema/wit/**/*.wit`
- `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`
- `modules/core-modules/*/src/**`, `modules/core-modules/*/Cargo.toml`, `modules/core-modules/*/wit-guest/**`
- `crates/slicer-wasm-host/test-guests/*/src/**`, `crates/slicer-wasm-host/test-guests/*/Cargo.toml`

Skip host-only changes outside this list and docs/tests outside the WASM build path. Copy exactly as one `design.md` Architecture Constraints bullet:

```markdown
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
```

Do not paraphrase; self-review and `spec-review` depend on the exact `--check` invocation.
