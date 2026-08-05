---
status: implemented
packet: 72_wit-single-source-unification
task_ids:
  - TASK-144
  - TASK-145
---

# 72_wit-single-source-unification

## Goal

Make `crates/slicer-schema/wit/` the one canonical, build-consumed WIT contract — read by both the guest macro and the host `bindgen!` — and delete the phantom top-level `wit/`, with zero change to slicer output.

## Problem Statement

`TASK-144`/`TASK-145` are checked `[x]` in `docs/07`, but the consolidation they describe is incomplete in the current tree, and the gap is actively harmful to the project's goal of accepting outside collaborators. The WIT contract exists in **three** copies: (1) top-level `wit/*.wit`, which **nothing parses** — `xtask/src/build_guests.rs` only stat()s it for mtime and `manifest.rs` validates the `wit-world` string against an allowlist without loading the file; (2) inline string literals for the four world bodies inside `crates/slicer-macros/src/lib.rs` (the real guest contract, reached via `include_str!` of `wit/deps/*` plus textual mangling); (3) four inline `wasmtime::component::bindgen!{ inline: … }` blocks in `crates/slicer-runtime/src/wit_host.rs` (the real host contract, ~700 lines). The two compiled copies are kept in agreement only by the existing instantiation tests; the phantom `wit/` has already drifted (it carries the illegal label `extrusion-path-3d`, a doc-only `extrusion-mode` gcode command, and a dead `gcode-output-interface`). A contributor who edits the obvious-looking `wit/world-layer.wit` changes nothing and gets no error — the central trap this packet removes. This packet finishes TASK-144/145: one canonical source, consumed by both sides, with the phantom deleted.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The `#[slicer_module]` proc-macro expands in arbitrary downstream module crates: it cannot use `bindgen! path:` and cannot read a dependency's consts. It must reach the canonical files via a stable relative `include_str!` path from `crates/slicer-macros/src/lib.rs` → `../../slicer-schema/wit/…`.
- Canonical type names must be consistent across host and guest. `extrusion-path-3d` and `extrusion-path3d` are **both legal** WIT (wit-parser 0.247 allows a non-first kebab segment to start with a digit — `lex.rs:validate_id`), but they generate **different** Rust idents. The original drift was the host hand-writing `extrusion-path3d` while the phantom `wit/` used `extrusion-path-3d`. The canonical files standardize on `extrusion-path3d` so both consumers generate the same type — a naming-consistency requirement, **not** a legality one. (Earlier drafts of this packet wrongly called `extrusion-path-3d` illegal; see Deviations.)
- Component ABI is structural, not nominal: generated type **names** may differ across the host/guest boundary, but record fields, variant case count/order, and function shapes must match. Reconciliation in Step 1 must preserve the ABI both compiled copies currently produce.

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

## Deviations (recorded during implementation)

1. **AC-1 world path updated** to `deps/world-layer/world-layer.wit` — worlds moved under `deps/` subdirectories so `wasmtime` `push_path`/`push_dir` (one-main-package-per-dir constraint) can load each world package independently; selected via fully-qualified `world:` names in `bindgen!`. Intent (canonical files at known paths, phantom gone) preserved; `wit_single_source_tdd` adds stricter anti-flat-copy checks to compensate.
2. **Dep packages are UNVERSIONED** (`slicer:types`, not `slicer:types@1.0.0`) — required for `wit-parser` cross-package `use` resolution; world packages keep `@1.0.0` so `manifest.rs` allowlist is unaffected. Packet prose saying `@1.0.0` deps is superseded by this implementation outcome.
3. **Host uses umbrella `path: "../slicer-schema/wit"` + qualified `world:` names + canonical `with:` keys** (`slicer:config/config-types.config-view`, `slicer:ir-handles/ir-handles.<resource>`), NOT a single flat `path:` pointing at one file — required by the one-main-package-per-dir constraint of `wasmtime` 43's `push_path`. Guest uses Option A nested-package inline. Both read the same `deps/*.wit` bytes → identity agreement is structural.
4. **Original AC-3 grep was too weak** (a flat-copy subdir would still have passed); strengthened via `wit_single_source_tdd` conformance tests (`no_flat_copies`, `worlds_are_not_self_contained`, `shared_interface_defined_once`) which guard against the entire flat-copy drift class, not just the `inline: r#` pattern.
5. **Host keeps four per-world generated `ModuleError` Rust types** (four separate `bindgen!` expansions) all originating from the single `slicer:common/module-errors.module-error`; each converts to `DispatchError`. AC-5 (one `record module-error` defined in `common.wit`) holds at the WIT level; the Rust-level multiplicity is a wasmtime codegen artifact, not a WIT duplication.
6. **AC-1 originally under-enumerated canonical files** (omitted `root.wit` anchor and host-services inline interfaces); absorbed without behavioral impact — worlds resolve correctly, roundtrip and benchy tests green.
