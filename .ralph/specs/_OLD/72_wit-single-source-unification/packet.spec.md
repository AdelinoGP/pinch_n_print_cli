---
status: implemented
packet: 72_wit-single-source-unification
task_ids:
  - TASK-144
  - TASK-145
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 72_wit-single-source-unification

## Goal

Make `crates/slicer-schema/wit/` the one canonical, build-consumed WIT contract — read by both the guest macro and the host `bindgen!` — and delete the phantom top-level `wit/`, with zero change to slicer output.

## Scope Boundaries

This packet relocates the WIT contract into the existing `slicer-schema` crate as standard, tool-parseable WIT, repoints the guest proc-macro (`slicer-macros`) and the host (`slicer-runtime::wit_host`) at it, folds the four per-world `module-error` declarations into one shared interface, and drops the dead `gcode-output-interface`. It is a structural deduplication only: the component ABI and every slicer artifact stay byte-identical, proven by the existing instantiation/roundtrip guard. It does **not** change any module's behavior, signatures, or the `run-support-geometry` contract (that is packet 73).

## Prerequisites and Blockers

- Depends on: none (Step 0 spike selects the codegen mechanism before any authoring).
- Unblocks: `73_support-geometry-normalization` (which edits the relocated `world-prepass.wit`).
- Activation blockers: none open. Step 0 must resolve the A/B mechanism fork (see `design.md` §Risks) before Step 2; this is a forward-looking spike, not an activation blocker.

## Acceptance Criteria

- **AC-1. Given** the repo tree, **when** checking for the contract, **then** top-level `wit/` is absent and `crates/slicer-schema/wit/` holds world packages under `deps/world-X/` plus `deps/{types,config,ir-types,common}.wit`. | `bash -c 'test ! -e wit && test -f crates/slicer-schema/wit/deps/world-layer/world-layer.wit && test -f crates/slicer-schema/wit/deps/common.wit; echo EXIT=$?'`
- **AC-2. Given** `crates/slicer-macros/src/lib.rs`, **when** grepping, **then** it carries no inline `package slicer:world-…` literal, no `extrusion-path-3d` rename, and its `include_str!` targets `slicer-schema/wit`. | `bash -c '! rg -q "package slicer:world-layer@1.0.0;" crates/slicer-macros/src/lib.rs && ! rg -q "extrusion-path-3d" crates/slicer-macros/src/lib.rs && rg -q "slicer-schema/wit" crates/slicer-macros/src/lib.rs; echo EXIT=$?'`
- **AC-3. Given** `crates/slicer-runtime/src/wit_host.rs`, **when** grepping, **then** no `bindgen!` uses an `inline: r#` literal and at least one `path:` points at the canonical wit dir. | `bash -c '! rg -q "inline: r#" crates/slicer-runtime/src/wit_host.rs && rg -q "path:.*slicer-schema/wit" crates/slicer-runtime/src/wit_host.rs; echo EXIT=$?'`
- **AC-4. Given** the canonical files, **when** grepping for the geometry path type, **then** the illegal label `extrusion-path-3d` never appears and the legal `extrusion-path3d` is defined. | `bash -c '! rg -q "extrusion-path-3d" crates/slicer-schema/wit && rg -q "extrusion-path3d" crates/slicer-schema/wit/deps/types.wit; echo EXIT=$?'`
- **AC-5. Given** the canonical files, **when** locating `record module-error`, **then** it is declared in exactly one file, `deps/common.wit`. | `bash -c 'test "$(rg -l "record module-error" crates/slicer-schema/wit)" = "crates/slicer-schema/wit/deps/common.wit"; echo EXIT=$?'`
  > **Portability note:** The canonical gate for "one `record module-error`" is the platform-independent `wit_single_source_tdd::shared_interface_defined_once`. The `rg -l` string-eq command above is a convenience check; it emits EXIT=1 on Windows purely due to ripgrep returning backslash path separators — a future Windows EXIT=1 here is NOT a real failure.
- **AC-6. Given** the canonical files, **when** grepping, **then** the dead `gcode-output-interface` appears nowhere. | `bash -c '! rg -q "gcode-output-interface" crates/slicer-schema/wit; echo EXIT=$?'`
- **AC-7. Given** freshly built guests, **when** the all-worlds roundtrip runs, **then** every world instantiates and round-trips unchanged (ABI preserved across the relocation). | `cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd`
- **AC-8. Given** the relocated source, **when** the freshness check runs after a rebuild, **then** it reports no `STALE:` guests. | `cargo xtask build-guests --check`
- **AC-9. Given** the canonical wit dir, **when** the conformance test resolves it, **then** `wit_parser` parses it and finds worlds `layer-module`, `prepass-module`, `postpass-module`, `finalization-module`. | `cargo test -p slicer-runtime --test wit_single_source_tdd -- canonical_wit_resolves`

## Negative Test Cases

- **AC-N1. Given** a WIT fragment carrying a label whose FIRST segment begins with a digit (e.g. `3d-extrusion-path`), **when** the conformance test feeds it to `wit_parser::Resolve`, **then** parsing is rejected — proving the canonical source is genuinely parser-validated so malformed labels cannot pass silently. | `cargo test -p slicer-runtime --test wit_single_source_tdd -- illegal_label_rejected`

## Verification

Gate commands only (full matrix in `requirements.md` §Verification Commands):

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT worlds, host-boundary enforcement, manifest schema. **Delegate a SUMMARY** of the "WIT worlds / package layout" section (file may exceed 300 lines); the implementer needs only the canonical world/interface inventory and the manifest `wit-world` allowlist contract.
- `docs/01_system_architecture.md` — module search path / codegen ownership. Delegate; load only if Step 1 reconciliation is ambiguous.

## Doc Impact Statement (Required)

This packet relocates the WIT contract and removes the dual-codegen maintenance burden, so docs that describe the old layout must change. Edits land in this packet; greps gate close:

- `docs/03_wit_and_manifest.md` §"WIT package layout / source location" — `rg -q 'crates/slicer-schema/wit' docs/03_wit_and_manifest.md`
- `CLAUDE.md` §"WIT/Type Changes Checklist" (drop "update both inline WIT and external package references" — there is no inline copy after this packet) — `rg -q 'crates/slicer-schema/wit' CLAUDE.md`
- `CLAUDE.md` §"Guest WASM Staleness" input list (repoint `wit/**/*.wit` → `crates/slicer-schema/wit/**/*.wit`) — `bash -c '! rg -q "^- \`wit/\*\*/\*.wit\`" CLAUDE.md; echo EXIT=$?'`
- New `crates/slicer-schema/wit/README.md` (single-source contract, how guest + host consume it, the `extrusion-path3d` naming rule) — `test -f crates/slicer-schema/wit/README.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- **[AC-1 / world path]** Specified top-level `crates/slicer-schema/wit/world-layer.wit`; implemented under the nested-package umbrella `deps/world-layer/world-layer.wit` (selected via fully-qualified `world:` names). Reason: `wasmtime 43` `push_path` requires one main package per dir. AC-1 command updated; intent preserved and tightened by `wit_single_source_tdd`.
- **[deps versioning]** Specified dep packages at `@1.0.0`; implemented **unversioned** (`slicer:types`, `slicer:config`, `slicer:ir-handles`, `slicer:common`) — only world packages keep `@1.0.0`. Reason: wit-parser cross-package resolution constraint; `manifest.rs` allowlist (world packages) unaffected.
- **[host path layout]** Specified a single flat `path: "../slicer-schema/wit"`; implemented umbrella `path:` + qualified `world:` + canonical `with:` keys, guest Option A nested-inline. Reason: one-main-package-per-dir; both consumers read the same `deps/*.wit`, so identity agreement is structural.
- **[AC-N1 / "illegal" label]** Specified that `extrusion-path-3d` is an *illegal* WIT label; this is **factually wrong** — wit-parser 0.247 permits a non-first kebab segment to start with a digit (`lex.rs:validate_id`). The real defect was naming **drift** (host `extrusion-path3d` vs phantom `extrusion-path-3d` → different Rust idents). Canonical standardizes on `extrusion-path3d` for ABI-name consistency; the `illegal_label_rejected` test now uses a genuinely-illegal first-segment-digit label (`3d-extrusion-path`). `design.md` Architecture Constraints corrected.
- **[AC-3 / grep strength]** Grep-only acceptance admitted a per-world flat-copy `path:`; strengthened with `wit_single_source_tdd` structural guards (`no_flat_copies`, `worlds_are_not_self_contained`, `shared_interface_defined_once`, `host_bindgen_paths_target_shared_root`). Agreement ≠ single-source (the flat copies passed the roundtrip).
- **[host ModuleError]** Specified collapsing four generated `ModuleError` to one; implemented as four per-world generated Rust types (separate `bindgen!` expansions), unified only at the WIT level (single `record module-error` in `deps/common.wit`). Reason: separate `bindgen!` macros each generate their own Rust type; AC-5 is WIT-level and holds.
- **[test-target fallout / gate hole]** Three `slicer-runtime` test targets (`live_seam_path_tdd`, `pipeline_tdd`, `z_envelope_contract_tdd`) referenced the *moved* generated paths (`world_layer::geometry`→`types::geometry`, `world_layer::ir_handles`→`ir_handles::ir_handles`) and **did not compile at commit `a4587d4`**. The name-churn sweep missed test targets, and `cargo check/clippy --workspace` (without `--all-targets`) did not catch it. Path-only fixes were applied during packet 73 and are re-attributed here. Gate hardened to `--all-targets` (CLAUDE.md).
