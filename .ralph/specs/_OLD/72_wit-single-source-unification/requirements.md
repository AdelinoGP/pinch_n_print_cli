# Requirements: 72_wit-single-source-unification

## Packet Metadata

- Grouped task IDs:
  - `TASK-144` — consolidate host, macro, and guest codegen onto one canonical shared WIT source.
  - `TASK-145` — normalize WIT package/version identifiers and restore missing members across the canonical surface.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`

## Problem Statement

`TASK-144`/`TASK-145` are checked `[x]` in `docs/07`, but the consolidation they describe is incomplete in the current tree, and the gap is actively harmful to the project's goal of accepting outside collaborators. The WIT contract exists in **three** copies: (1) top-level `wit/*.wit`, which **nothing parses** — `xtask/src/build_guests.rs` only stat()s it for mtime and `manifest.rs` validates the `wit-world` string against an allowlist without loading the file; (2) inline string literals for the four world bodies inside `crates/slicer-macros/src/lib.rs` (the real guest contract, reached via `include_str!` of `wit/deps/*` plus textual mangling); (3) four inline `wasmtime::component::bindgen!{ inline: … }` blocks in `crates/slicer-runtime/src/wit_host.rs` (the real host contract, ~700 lines). The two compiled copies are kept in agreement only by the existing instantiation tests; the phantom `wit/` has already drifted (it carries the illegal label `extrusion-path-3d`, a doc-only `extrusion-mode` gcode command, and a dead `gcode-output-interface`). A contributor who edits the obvious-looking `wit/world-layer.wit` changes nothing and gets no error — the central trap this packet removes. This packet finishes TASK-144/145: one canonical source, consumed by both sides, with the phantom deleted.

## In Scope

- Create `crates/slicer-schema/wit/` as standard, statement-form WIT packages: `world-{layer,prepass,postpass,finalization}.wit` + `deps/{types,config,ir-types,common}.wit`.
- Reconcile content to the two **compiled** copies (macro + host), not the phantom docs: keep what both agree on; use the legal `extrusion-path3d` spelling everywhere; drop the dead `gcode-output-interface` (Candidate 3); drop the doc-only `extrusion-mode`/`gcode-extrusion-mode-cmd` (absent from both compiled sides); reconcile prepass `global-layer-index` typing to the compiled form.
- Hoist `module-error` into `deps/common.wit` (`package slicer:common@1.0.0`) and have the four worlds `import`/`use` it; collapse the host's four generated `ModuleError` types to the shared one (Candidate 2).
- Repoint `slicer-macros` `include_str!` consts + replace the four inline world literals with `include_str!` of the canonical world files; build each `wit_bindgen::generate!{ inline: … }` blob by nested-package concatenation and delete `strip_package_decl`, the `geometry`-rename, the `extrusion-path-3d` rename, and textual `include` substitution (or fall back to option B per Step 0).
- Migrate the four host `bindgen!` calls to `path: "../slicer-schema/wit"` + remapped `with:` keys; sweep generated type-name churn in `wit_host.rs`/`dispatch.rs`.
- Delete top-level `wit/`; repoint `xtask/src/build_guests.rs` staleness mtime walk to the new dir.
- Add `crates/slicer-runtime/tests/wit_single_source_tdd.rs` (parse + negative illegal-label + no-inline assertions).
- Doc edits per `packet.spec.md` §Doc Impact Statement.

## Out of Scope

- Any change to `run-support-geometry`'s contract, config plumbing, or error surfacing (packet 73).
- Candidate 4 (table-driven host dispatch) — deferred.
- Adding the phantom-doc `extrusion-mode` gcode command as a real feature (flag for a future packet if wanted).
- Any module behavior change, IR field change, or config-key change.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — primary. Likely > 300 lines: **delegate a SUMMARY** scoped to the world/interface inventory + the `wit-world` allowlist contract.
- `docs/01_system_architecture.md` — module search path / codegen ownership. Delegate; consult only if Step 1 reconciliation is ambiguous.
- `CLAUDE.md` §"Guest WASM Staleness", §"WIT/Type Changes Checklist" — load directly (short, already in working context); both are edited by this packet.

## Acceptance Summary

- Positive cases: `AC-1` (single source + phantom deleted), `AC-2` (guest sources canonical, hacks gone), `AC-3` (host on `path:`), `AC-4` (legal `extrusion-path3d`), `AC-5` (one `module-error`), `AC-6` (orphan gone), `AC-7` (ABI-preserving roundtrip), `AC-8` (guest freshness), `AC-9` (canonical wit resolves) — all in `packet.spec.md`. Refinement: AC-7 is the load-bearing proof that reconciliation preserved the component ABI; it must pass against guests rebuilt from the relocated source (run AC-8 first).
- Negative cases: `AC-N1` (a WIT fragment whose label's FIRST segment begins with a digit, e.g. `3d-extrusion-path`, is rejected by `wit_parser::Resolve`, proving the canonical source is genuinely parser-validated so malformed labels cannot pass silently).
- Cross-packet impact: unblocks `73_support-geometry-normalization`.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -c 'test ! -e wit && test -f crates/slicer-schema/wit/deps/world-layer/world-layer.wit && test -f crates/slicer-schema/wit/deps/common.wit; echo EXIT=$?'` | AC-1 layout | FACT `EXIT=0` |
| `bash -c '! rg -q "package slicer:world-layer@1.0.0;" crates/slicer-macros/src/lib.rs && ! rg -q "extrusion-path-3d" crates/slicer-macros/src/lib.rs && rg -q "slicer-schema/wit" crates/slicer-macros/src/lib.rs; echo EXIT=$?'` | AC-2 guest source | FACT `EXIT=0` |
| `bash -c '! rg -q "inline: r#" crates/slicer-runtime/src/wit_host.rs && rg -q "path:.*slicer-schema/wit" crates/slicer-runtime/src/wit_host.rs; echo EXIT=$?'` | AC-3 host on path: | FACT `EXIT=0` |
| `bash -c '! rg -q "extrusion-path-3d" crates/slicer-schema/wit && rg -q "extrusion-path3d" crates/slicer-schema/wit/deps/types.wit; echo EXIT=$?'` | AC-4 legal label | FACT `EXIT=0` |
| `bash -c 'test "$(rg -l "record module-error" crates/slicer-schema/wit)" = "crates/slicer-schema/wit/deps/common.wit"; echo EXIT=$?'` | AC-5 one module-error | FACT `EXIT=0` — **portability note:** the canonical gate is `wit_single_source_tdd::shared_interface_defined_once`; the `rg -l` string-eq emits EXIT=1 on Windows due to backslash path separators — a Windows EXIT=1 here is NOT a real failure. |
| `bash -c '! rg -q "gcode-output-interface" crates/slicer-schema/wit; echo EXIT=$?'` | AC-6 orphan gone | FACT `EXIT=0` |
| `cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd` | AC-7 ABI preserved | FACT pass/fail; SNIPPETS ≤20 lines on fail |
| `cargo xtask build-guests --check` | AC-8 guest freshness | FACT clean / `STALE:` list |
| `cargo test -p slicer-runtime --test wit_single_source_tdd` | AC-9 + AC-N1 conformance | FACT pass/fail; SNIPPETS on fail |
| `cargo test -p slicer-runtime --test core_module_ir_access_contract_tdd` | regression guard (IR-access boundary unchanged) | FACT pass/fail |
| `cargo test -p slicer-runtime --test benchy_end_to_end_tdd` | regression guard (e2e gcode unchanged) | FACT pass/fail |
| `cargo check --workspace` | gate | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | gate | FACT pass/fail |

No command above is `cargo test --workspace`; the workspace suite is not a gate for this packet.

## Step Completion Expectations

- Ordering: Step 0 (spike) must precede Step 2/4 — it selects the guest codegen mechanism (nested-package inline vs flatten) and confirms host `path:` viability. Step 1 (author canonical) precedes Step 2/4 (consumers point at it). The top-level `wit/` deletion (Step 3) must not precede Step 2's guest repoint, or the guest build breaks.
- Cross-step invariant: `macro_all_worlds_roundtrip_tdd`, `core_module_ir_access_contract_tdd`, and `benchy_end_to_end_tdd` must stay green at the end of **every** step that touches a consumer (Steps 2 and 4), each preceded by a guest rebuild. A red here means reconciliation changed the ABI — fix before advancing.

## Context Discipline Notes

- `crates/slicer-runtime/src/wit_host.rs` is ~6000 lines — **never load in full**. Range-read only the four `bindgen!` sites (≈241, 493, 890, 1066), the `with:` maps just below each, and the `pub use layer::ModuleError;` line (≈455). The four host trait-impl name-churn sites surface as compiler errors — fix by error, do not browse.
- `crates/slicer-macros/src/lib.rs` is ~3000 lines — range-read only `emit_world_preamble`/`expand_inline_wit` (≈478–558) and the four inline world literals (≈567, 892, 1283, 2941).
- Heaviest dispatch return-format hint: build/test runs return FACT pass/fail or SNIPPETS ≤20 lines on failure — never the full cargo log.
