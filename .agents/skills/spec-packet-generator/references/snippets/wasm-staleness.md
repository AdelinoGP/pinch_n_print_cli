# Snippet: wasm-staleness

**When to include**: packets that edit any of the following paths, because their edits will not propagate to guest `.wasm` artifacts until rebuilt:

- `wit/**/*.wit`
- `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**`
- `modules/core-modules/*/src/**`, `modules/core-modules/*/Cargo.toml`, `modules/core-modules/*/wit-guest/**`
- `test-guests/*/src/**`, `test-guests/*/Cargo.toml`

**Skip** when the packet edits only host crates that are not in this list (e.g., `slicer-runtime` internal refactor with no IR/WIT/SDK change), or when the packet touches only docs/tests outside the WASM build path.

**Where to include**: as a bullet in `design.md` §`Architecture Constraints` (NOT as its own section — it's one constraint among several). Add `<!-- snippet: wasm-staleness -->` on the line above the bullet.

**Verbatim bullet**:

```
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
```

**Do not paraphrase.** The exact `--check` invocation matters because the self-review and `spec-review` skill grep for it.
