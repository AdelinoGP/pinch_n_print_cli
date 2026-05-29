# Design: 72_wit-single-source-unification

## Controlling Code Paths

- New canonical source: `crates/slicer-schema/wit/` (statement-form WIT packages + `deps/`).
- Guest codegen: `crates/slicer-macros/src/lib.rs` — `emit_world_preamble`/`expand_inline_wit` (≈478–558) and the four inline world literals (`build_postpass_world_glue` ≈566, `build_finalization_world_glue` ≈891, `build_prepass_world_glue` ≈1282, layer world ≈2940). Today: `include_str!("../../../wit/deps/{types,config,ir-types}.wit")` (480–482) + `strip_package_decl` + `.replace("slicer:types/geometry","geometry")` (505) + `.replace("extrusion-path-3d","extrusion-path3d")` (511) + textual `include "../../wit/deps/…"` substitution (508–510).
- Host codegen: `crates/slicer-runtime/src/wit_host.rs` — four `wasmtime::component::bindgen!{ inline: r#"…"# , with: { … } }` (241, 493, 890, 1066) and `pub use layer::ModuleError;` (≈455).
- Staleness walk: `xtask/src/build_guests.rs` (≈481) — `WalkDir::new(ws_root.join("wit"))`.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/macro_all_worlds_roundtrip_tdd.rs`, `core_module_ir_access_contract_tdd.rs`, `benchy_end_to_end_tdd.rs`, `guest_fixture_freshness_tdd.rs` (the existing host↔guest agreement guard).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The `#[slicer_module]` proc-macro expands in arbitrary downstream module crates: it cannot use `bindgen! path:` and cannot read a dependency's consts. It must reach the canonical files via a stable relative `include_str!` path from `crates/slicer-macros/src/lib.rs` → `../../slicer-schema/wit/…`.
- WIT labels must start with a letter: `extrusion-path-3d` (segment `3d`) is invalid WIT. The canonical files must use `extrusion-path3d`. This is why the old macro string-replaced it and the host hand-wrote it; with real `path:`/parse consumption there is no string-replace escape hatch.
- Component ABI is structural, not nominal: generated type **names** may differ across the host/guest boundary, but record fields, variant case count/order, and function shapes must match. Reconciliation in Step 1 must preserve the ABI both compiled copies currently produce.

## Code Change Surface

- Selected approach (option A, pending Step 0 confirmation): canonical files are standard WIT consumed by **both** sides. Host uses `bindgen! path:`; guest macro reads the same files via `include_str!`, wraps each `package x;` in nested-package braces `package x { … }`, concatenates, and lets `wit_bindgen::generate!{ inline: … }` resolve cross-package `use` natively — deleting the flatten/rename machinery.
- Exact surfaces expected to change:
  - **New** `crates/slicer-schema/wit/**` (7 wit files + README).
  - `crates/slicer-macros/src/lib.rs`: `include_str!` consts → `../../slicer-schema/wit/…`; four inline world literals → `include_str!`; `emit_world_preamble` blob assembly rewritten to nested-package concat; `expand_inline_wit` deleted/gutted.
  - `crates/slicer-runtime/src/wit_host.rs`: four `bindgen!` `inline:`→`path:` + `with:` key remap (`slicer:world-…/config-types` → `slicer:config/config-types`, etc.); name-churn sweep; `ModuleError` re-export points at the shared world type.
  - `xtask/src/build_guests.rs`: staleness walk path.
  - **New** `crates/slicer-runtime/tests/wit_single_source_tdd.rs`.
  - Docs: `docs/03_wit_and_manifest.md`, `CLAUDE.md`, new wit `README.md`.
- Rejected alternatives: **(B)** keep the guest flatten machinery, make files standard only for the host — retained as the Step 0 fallback if nested-package `inline:` is unsupported; rejected as default because it keeps the brittle textual rewrites. **build.rs codegen** for the host — retained as the R3 fallback if `path:` cannot resolve the package set; rejected as default because `path:` against a standard package dir is simpler. **A standalone `slicer-wit` crate** — rejected (chosen during planning) in favor of co-locating with the `STAGES` table in `slicer-schema`.

## Files in Scope (read + edit)

Primary edit surface is 3 code files + the new wit dir; the new test and doc edits are mechanical.

- `crates/slicer-schema/wit/**` — role: the canonical contract; expected change: created from reconciled compiled copies.
- `crates/slicer-macros/src/lib.rs` — role: guest codegen; expected change: source from canonical files, delete mangling.
- `crates/slicer-runtime/src/wit_host.rs` — role: host codegen; expected change: `inline:`→`path:`, shared `ModuleError`, name churn.
- `xtask/src/build_guests.rs` — role: staleness walk; expected change: one path string.
- `crates/slicer-runtime/tests/wit_single_source_tdd.rs` — role: conformance + negative; expected change: new file.

## Read-Only Context

- `crates/slicer-runtime/src/wit_host.rs` — range-read the four `bindgen!` sites (≈241, 493, 890, 1066) and `with:` maps + line ≈455 only. > 600 lines: never load whole.
- `crates/slicer-macros/src/lib.rs` — range-read ≈478–558 + the four world literals only. > 600 lines: never load whole.
- `wit/deps/{types,config,ir-types}.wit` + `wit/world-*.wit` (current top-level) — the reconciliation inputs; small, load directly while authoring Step 1, then delete.
- `docs/03_wit_and_manifest.md` — delegate SUMMARY (likely > 300 lines).

## Out-of-Bounds Files

- `crates/slicer-runtime/src/wit_host.rs` / `crates/slicer-macros/src/lib.rs` **in full** — range-read only.
- `target/`, `Cargo.lock`, generated `.wasm` under `modules/core-modules/*/` and `test-guests/*` — never load; rebuild via `cargo xtask build-guests`.
- `OrcaSlicerDocumented/**` — not relevant (no parity surface); never load.
- Crates outside the change surface — delegate trait/impl lookups.

## Expected Sub-Agent Dispatches

- "Run `cargo build -p slicer-runtime` (host `path:` spike on one world); return FACT (pass) or SNIPPETS (≤20 lines) on the first error" — purpose: Step 0 host viability.
- "Run `cargo xtask build-guests` then `--check`; return FACT clean or the `STALE:` list" — purpose: Steps 2/4 guest refresh.
- "Run `cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd`; FACT pass/fail + failing assertion ≤20 lines" — purpose: ABI-preservation gate after each consumer edit.
- "Summarize `docs/03_wit_and_manifest.md` world/interface inventory + `wit-world` allowlist; return SUMMARY ≤200 words" — purpose: Step 1 reconciliation + Step 5 doc edit.
- "List the wit-bindgen-generated type names in `wit_host.rs` that reference the layer world's `module-error`/`extrusion-path` types; return LOCATIONS" — purpose: scope the Step 4 name-churn sweep without browsing.

## Data and Contract Notes

- IR/manifest contracts touched: none semantically — the same WIT types, relocated. `manifest.rs` `wit-world` allowlist strings (`slicer:world-layer@1.0.0`, …) are unchanged, so manifest validation is unaffected.
- WIT boundary: package identifiers and version (`@1.0.0`) preserved; only the file *location* and the `module-error` *owner interface* change. `config-view` is already shared via `import slicer:config/config-types` — `module-error` joins it in `slicer:common`.
- Determinism/scheduler: unaffected.

## Locked Assumptions and Invariants

- The component ABI produced for all four worlds is byte-compatible before and after — locked by AC-7 (`macro_all_worlds_roundtrip_tdd`) and AC-8 (guest freshness). Any divergence is a packet failure, not an accepted change.
- No slicer output (gcode, IR) changes — locked by `benchy_end_to_end_tdd`.
- The canonical wit dir is a valid, resolvable WIT package set — locked by AC-9 / AC-N1.

## Risks and Tradeoffs

- **R1 (mechanism):** `wit_bindgen 0.57` `inline:` may reject nested-package form. Mitigation: Step 0 spike; fallback = option B (keep guest flatten), no other plan change.
- **R3 (host path:):** `wasmtime 43` `bindgen! path:` may not resolve the multi-package `deps/` tree or the remapped `with:` keys. Mitigation: Step 0 spike; fallback = build.rs codegen emitting the inline string from canonical files (still single-source), or defer the host migration (Step 4) — the phantom trap is already gone after Step 2.
- **R4 (name churn):** switching the host source may rename generated types across `wit_host.rs`. Mitigation: compiler-guided; scoped by a LOCATIONS dispatch, not by browsing.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 4 — host `path:` migration + name-churn sweep in the 6000-line `wit_host.rs`, mitigated by range-reads + LOCATIONS dispatch).
- Highest-risk dispatch: the host build spike (Step 0) — must return FACT/SNIPPETS only, never the full compiler log, or it blows budget.

## Open Questions

- `[FWD]` Option A vs B for the guest, and `path:` vs build.rs for the host, are resolved by the Step 0 spike. Either resolution keeps every AC intact — not activation-blocking.
- `None [BLOCK].`
