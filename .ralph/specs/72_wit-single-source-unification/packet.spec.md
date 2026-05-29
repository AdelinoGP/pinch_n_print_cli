---
status: active
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

- **AC-1. Given** the repo tree, **when** checking for the contract, **then** top-level `wit/` is absent and `crates/slicer-schema/wit/` holds `world-{layer,prepass,postpass,finalization}.wit` plus `deps/{types,config,ir-types,common}.wit`. | `bash -c 'test ! -e wit && test -f crates/slicer-schema/wit/world-layer.wit && test -f crates/slicer-schema/wit/deps/common.wit; echo EXIT=$?'`
- **AC-2. Given** `crates/slicer-macros/src/lib.rs`, **when** grepping, **then** it carries no inline `package slicer:world-…` literal, no `extrusion-path-3d` rename, and its `include_str!` targets `slicer-schema/wit`. | `bash -c '! rg -q "package slicer:world-layer@1.0.0;" crates/slicer-macros/src/lib.rs && ! rg -q "extrusion-path-3d" crates/slicer-macros/src/lib.rs && rg -q "slicer-schema/wit" crates/slicer-macros/src/lib.rs; echo EXIT=$?'`
- **AC-3. Given** `crates/slicer-runtime/src/wit_host.rs`, **when** grepping, **then** no `bindgen!` uses an `inline: r#` literal and at least one `path:` points at the canonical wit dir. | `bash -c '! rg -q "inline: r#" crates/slicer-runtime/src/wit_host.rs && rg -q "path:.*slicer-schema/wit" crates/slicer-runtime/src/wit_host.rs; echo EXIT=$?'`
- **AC-4. Given** the canonical files, **when** grepping for the geometry path type, **then** the illegal label `extrusion-path-3d` never appears and the legal `extrusion-path3d` is defined. | `bash -c '! rg -q "extrusion-path-3d" crates/slicer-schema/wit && rg -q "extrusion-path3d" crates/slicer-schema/wit/deps/types.wit; echo EXIT=$?'`
- **AC-5. Given** the canonical files, **when** locating `record module-error`, **then** it is declared in exactly one file, `deps/common.wit`. | `bash -c 'test "$(rg -l "record module-error" crates/slicer-schema/wit)" = "crates/slicer-schema/wit/deps/common.wit"; echo EXIT=$?'`
- **AC-6. Given** the canonical files, **when** grepping, **then** the dead `gcode-output-interface` appears nowhere. | `bash -c '! rg -q "gcode-output-interface" crates/slicer-schema/wit; echo EXIT=$?'`
- **AC-7. Given** freshly built guests, **when** the all-worlds roundtrip runs, **then** every world instantiates and round-trips unchanged (ABI preserved across the relocation). | `cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd`
- **AC-8. Given** the relocated source, **when** the freshness check runs after a rebuild, **then** it reports no `STALE:` guests. | `cargo xtask build-guests --check`
- **AC-9. Given** the canonical wit dir, **when** the conformance test resolves it, **then** `wit_parser` parses it and finds worlds `layer-module`, `prepass-module`, `postpass-module`, `finalization-module`. | `cargo test -p slicer-runtime --test wit_single_source_tdd -- canonical_wit_resolves`

## Negative Test Cases

- **AC-N1. Given** a WIT fragment carrying the illegal label `extrusion-path-3d` (segment begins with a digit), **when** the conformance test feeds it to `wit_parser::Resolve`, **then** parsing is rejected — proving the canonical source is genuinely validated and the old phantom-drift class can no longer pass silently. | `cargo test -p slicer-runtime --test wit_single_source_tdd -- illegal_label_rejected`

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
