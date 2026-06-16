---
status: implemented
packet: workspace-aware-guest-builder
task_ids:
  - TASK-214
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: workspace-aware-guest-builder

## Goal

Replace `modules/core-modules/build-core-modules.sh` AND `test-guests/build-test-guests.sh` with a single `cargo xtask build-guests [--check]` command using a validated filesystem walk for discovery. The xtask discovers guest crates via a walk over two tree-roots (no hardcoded module lists), tracks freshness via file mtimes, runs `wasm-tools component new` post-processing per a configurable artifact-path convention, and lands as a new `xtask/` crate with no dependency on `slicer-runtime` so agentic hooks can invoke it without compiling the full runtime closure. The `docs/05_module_sdk.md` module-author build-flow section is rewritten to document the `cargo build --target wasm32-unknown-unknown --release` + `wasm-tools component new` two-step plus a sidebar pointing workspace contributors at `cargo xtask build-guests`.

## Scope Boundaries

This packet retires both guest-build bash scripts and replaces them with a filesystem-walk-driven xtask. It also rewrites the module-author build flow in `docs/05_module_sdk.md` to match the post-merge reality where `pnp_cli build` does not exist. It does NOT change WASM module ABI, manifest TOML schema, the `wasm-tools component new` invocation shape, `manifest::is_placeholder_wasm`, or host-runtime code (that work landed in Packet 1 `pnp-cli-unification`).

## Prerequisites and Blockers

- Depends on: Packet 1 `pnp-cli-unification` (must be `status: implemented`). This packet's `docs/05_module_sdk.md` rewrite assumes the `pnp_cli` binary name and the absence of the `pnp_cli build` verb.
- Unblocks: none directly — this is the second of two architecture-deepening packets identified in `C:\Users\agpen\.claude\plans\inherited-gathering-frost.md`.
- Activation blockers: Packet 1 not yet `status: implemented`.

## Acceptance Criteria

- **AC-1. Given** a clean workspace post-Packet-1, **when** the implementer runs `cargo xtask build-guests` from scratch, **then** every guest crate under `modules/core-modules/**/wit-guest/` AND every guest crate under `test-guests/*/` is built and its component-model `.wasm` artifact lands at the expected path (`modules/core-modules/<dir>/<dir>.wasm` for core-modules; `test-guests/<crate-name>.component.wasm` for test-guests, matching the existing bash output paths). | `cargo xtask build-guests && cargo xtask build-guests --check`
- **AC-2. Given** a validated filesystem walk as the source of guest enumeration, **when** the implementer queries the xtask's discovery output, **then** the count of core-module guests equals `find modules/core-modules -mindepth 2 -maxdepth 2 -type d -name wit-guest | wc -l` and the count of test-guests equals `find test-guests -mindepth 1 -maxdepth 1 -type d | wc -l`, with no hardcoded list embedded in xtask source. The verification sums both tree-root counts. | `cargo xtask build-guests --list | wc -l > /tmp/xtask_count && { find modules/core-modules -mindepth 2 -maxdepth 2 -type d -name wit-guest -exec test -e {}/Cargo.toml \; -print | wc -l; find test-guests -mindepth 1 -maxdepth 1 -type d -exec test -e {}/Cargo.toml \; -print | wc -l; } | awk '{s+=$1} END {print s}' > /tmp/fs_count && diff /tmp/xtask_count /tmp/fs_count`
- **AC-3. Given** a fresh build (`cargo xtask build-guests` just succeeded), **when** the implementer touches `wit/world-layer.wit` and then runs `cargo xtask build-guests --check`, **then** the command exits 1 and stdout contains at least one `STALE: ` line for each guest that depends on `world-layer.wit` (every core-module guest plus every test-guest). | `touch wit/world-layer.wit && ! cargo xtask build-guests --check && cargo xtask build-guests --check 2>&1 | grep -q '^STALE: '`
- **AC-4. Given** a stale-state surfaced by AC-3, **when** the implementer runs `cargo xtask build-guests` (no flag) and then `cargo xtask build-guests --check`, **then** the second invocation exits 0 with no `STALE:` lines. | `cargo xtask build-guests && cargo xtask build-guests --check`
- **AC-5. Given** the bash-script retirement, **when** the implementer checks the filesystem, **then** neither `modules/core-modules/build-core-modules.sh` nor `test-guests/build-test-guests.sh` exists. | `test ! -f modules/core-modules/build-core-modules.sh && test ! -f test-guests/build-test-guests.sh`
- **AC-6. Given** the doc updates, **when** the implementer greps the workspace, **then** `CLAUDE.md`, `docs/05_module_sdk.md`, and `.github/workflows/ci.yml` all reference `cargo xtask build-guests` and none of `CLAUDE.md`, `docs/`, `.github/`, `.claude/`, `.agents/` contain references to `build-core-modules.sh` or `build-test-guests.sh`. | `grep -q 'cargo xtask build-guests' CLAUDE.md && grep -q 'cargo xtask build-guests' docs/05_module_sdk.md && grep -q 'cargo xtask build-guests' .github/workflows/ci.yml && ! grep -rln 'build-core-modules\.sh\|build-test-guests\.sh' CLAUDE.md docs/ .github/ .claude/ .agents/ 2>/dev/null`
- **AC-7. Given** the rewritten module-author build flow, **when** the implementer reads `docs/05_module_sdk.md`'s Developer CLI section, **then** the section documents the two-step `cargo build --target wasm32-unknown-unknown --release` followed by `wasm-tools component new target/wasm32-unknown-unknown/release/<name>.wasm -o target/slicer/<name>.wasm` as the module-author build path, plus a sidebar/note pointing workspace contributors at `cargo xtask build-guests`. | `grep -q 'cargo build --target wasm32-unknown-unknown' docs/05_module_sdk.md && grep -q 'wasm-tools component new' docs/05_module_sdk.md && grep -q 'cargo xtask build-guests' docs/05_module_sdk.md`

## Negative Test Cases

- **AC-N1. Given** the workspace-metadata-driven enumeration, **when** the implementer greps the xtask source for hardcoded module lists, **then** there is no `MODULES = …` or `GUESTS = …` array literal in any `.rs` file under `xtask/`. | `! grep -rn 'MODULES\s*[:=]\s*\[\|GUESTS\s*[:=]\s*\[\|const.*MODULES.*\[' xtask/`
- **AC-N2. Given** the absence of `pnp_cli build`, **when** the implementer reads `docs/05_module_sdk.md`'s Developer CLI section, **then** the doc does NOT recommend `pnp_cli build` as a module-author build verb. | `! grep -E 'pnp_cli\s+build\b' docs/05_module_sdk.md`

## Verification

- `cargo build --workspace --release`
- `cargo clippy --workspace -- -D warnings`
- `cargo xtask build-guests && cargo xtask build-guests --check`

## Authoritative Docs

- `CLAUDE.md` — load directly; §"Guest WASM Staleness (MUST follow)" is the rewrite target (currently mentions both bash scripts; collapses to one xtask command).
- `docs/05_module_sdk.md` — delegate SUMMARY of the "Developer CLI" section before editing; the rest of the file is unchanged.
- `modules/core-modules/build-core-modules.sh` — load directly (~220 lines); used as the reference for guest-build mechanics and `wasm-tools` invocation shape, then deleted.
- `test-guests/build-test-guests.sh` — load directly; structurally similar to core-modules script; used as second reference, then deleted.
- `.github/workflows/ci.yml` — load directly (~60 lines).

## Doc Impact Statement (Required)

A list of specific doc sections this packet modifies, with verification greps:

- `docs/05_module_sdk.md` §"Developer CLI" / build flow — rewritten to two-step + xtask sidebar — `rg -q 'cargo xtask build-guests' docs/05_module_sdk.md && rg -q 'wasm-tools component new' docs/05_module_sdk.md && ! rg -E 'pnp_cli\s+build\b' docs/05_module_sdk.md`
- `CLAUDE.md` §"Guest WASM Staleness (MUST follow)" — collapsed from two scripts to one xtask invocation — `rg -q 'cargo xtask build-guests --check' CLAUDE.md && ! rg -q 'build-core-modules\.sh\|build-test-guests\.sh' CLAUDE.md`
- `docs/07_implementation_status.md` — TASK-214 entry added with closure note — `rg -q 'TASK-214' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Changelog

- 2026-05-29: Corrected discovery mechanism from `cargo_metadata` to a validated filesystem walk over the two tree-roots. Guest Cargo.tomls declare `[workspace]` sentinels and are invisible to the parent workspace's metadata, so `cargo_metadata` returns zero guests. Per-tree validation: core-module wit-guests require `crate-type=["cdylib"]` + a `[workspace]` sentinel + a parent path dep (`{ path = ".." }`); test-guests require `crate-type=["cdylib"]` + a `[workspace]` sentinel + a direct `wit-bindgen` dependency. The "no hardcoded MODULES list" invariant (AC-N1) is satisfied identically. Also fixed AC-2's verification command, which had a `find -o` precedence bug that prevented it from counting the `test-guests/` tree.
- 2026-05-29: Tightened AC-2's `find` halves with `-exec test -e {}/Cargo.toml \; -print` so the verification agrees with the validated walk on stray directories (e.g. `test-guests/sdk-layer-plan-guest/`, which has only `Cargo.lock`+`target/` and no manifest).
- 2026-05-29: Implementation detail — `lib_name` derivation reads `[lib].name` from the candidate manifest first and falls back to `package.name.replace('-', '_')` only when `[lib].name` is absent. This is Cargo's own rule for cdylib artifact names; one guest (`test-guests/path-optimization-multi-read`) declares an explicit `[lib].name = path_optimization_multi_read_guest` that differs from its `package.name = path-optimization-multi-read`, so the strict "always underscore the package name" rule would have looked for the wrong intermediate `.wasm`.
