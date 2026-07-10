---
status: implemented
packet: 03_wit-canonical-source-and-validation
task_ids:
  - TASK-144
  - TASK-145
  - TASK-146
---

# 03_wit-canonical-source-and-validation

## Goal

Consolidate WIT compatibility onto one canonical shared source rooted in `wit/` by replacing the three diverged inline WIT copies in `slicer-macros` (macro inline WIT in `lib.rs`), `slicer-host` (host inline WIT in `wit_host.rs`), and test guests, with `include_str!`-backed references to the on-disk canonical WIT files. Normalize all package/version identifiers to match the on-disk canonical. Add host-side `wit_world` allowlist validation using the canonical identifiers. Add drift-detection regression coverage.

## Problem Statement

WIT compatibility is split across three sources: (1) on-disk `wit/` directory, (2) macro inline WIT in `crates/slicer-macros/src/lib.rs` (the `build_*_world_glue` functions), and (3) host inline WIT in `crates/slicer-host/src/wit_host.rs`. This duplication has caused real drift:

1. **Package name drift**: Host inline WIT uses `slicer:layer-world@1.0.0` and `slicer:prepass-world@1.0.0`; canonical on-disk files use `slicer:world-layer@1.0.0` and `slicer:world-prepass@1.0.0`. The macro uses the canonical names. DAG construction code (`dag.rs:158`, `execution_plan.rs:858`) hardcodes the canonical names.
2. **ir-types version drift**: On-disk canonical is `slicer:ir-types@1.1.0`; some inline copies may reference `1.0.0`.
3. **Missing members**: `needs-support` is absent from inline WIT copies; `push-z-hop` is absent from postpass inline `gcode-output-builder`.
4. **No drift detection**: No test verifies the three copies stay in sync.

This packet consolidates onto one canonical source (`wit/`) and adds startup `wit_world` validation.

If this packet reopens or narrows a prior packet: this is the first WIT consolidation packet for Workstream 1. There is no prior WIT consolidation packet.

## Architecture Constraints

- `slicer-macros` is a proc-macro crate — it compiles in its own compilation context before the main crate. `include_str!` is resolved at compile time in that context. The relative path from `crates/slicer-macros/src/lib.rs` to `wit/` must be validated at the start of the packet.
- `wit_bindgen::generate!` accepts inline WIT as a `&str`. The `include_str!` result (`&'static str`) satisfies this.
- The four canonical world identifiers are defined by the on-disk `wit/` files. The allowlist in the host must be derived from those same files (hardcoded constants matching the canonical names, updated together with any WIT changes).
- Version (`@1.0.0` vs `@1.1.0`) is part of the identifier for allowlist purposes — `slicer:ir-types@1.1.0` ≠ `slicer:ir-types@1.0.0`.
- The drift detection test must not require runtime I/O that could make it non-deterministic. It should compare `include_str!` results at compile/test time.

## Data and Contract Notes

- WIT boundary considerations: Consolidation does NOT change WIT types, only their source. The `wit_bindgen!` output types remain identical.
- Package name normalization: `slicer:layer-world@1.0.0` (host inline, wrong) → `slicer:world-layer@1.0.0` (canonical, correct). The macro already uses the canonical name. This fix aligns the host with the macro and disk.
- ir-types version: On-disk canonical is `@1.1.0`. If any inline copy uses `@1.0.0`, it must be updated. Check macro `lib.rs` for the `ir-types` version reference.
- Determinism: The `include_str!` macro produces identical `&str` content across builds. No runtime I/O is introduced in hot paths.
- Scheduler constraints: The allowlist check is at module-load time (startup), not per-invocation. No per-layer overhead.

## Locked Assumptions and Invariants

- The four canonical world identifiers (`slicer:world-layer@1.0.0`, `slicer:world-prepass@1.0.0`, `slicer:world-postpass@1.0.0`, `slicer:world-finalization@1.0.0`) are stable for the lifetime of this consolidation packet. They will not change.
- The `wit/` directory is the single source of truth for macro WIT glue generation (macro uses WIT-level `include` directives referencing `wit/deps/`). The host `wit_host.rs` inline WIT blocks are retained because wasmtime's `bindgen!` requires fully-expanded inline WIT — external WIT package imports cannot be resolved at `bindgen!` compile time without the packages being present. This is a known deviation from the original "eliminate all inline copies" goal.
- The allowlist contains exactly four entries — one per WIT world. No wildcard or regex matching.
- `needs-support` and `push-z-hop` are additive missing members — adding them to the disk canonical does not break any existing bindings because they are optional interfaces.

## Risks and Tradeoffs

- **Proc-macro `include_str!` path resolution**: The relative path `../../wit/` from `crates/slicer-macros/src/lib.rs` must be verified to work at macro compile time. If it does not, an alternative is to place a copy of the key WIT files in the `slicer-macros` crate directory.
- **Package name changes in host**: Changing `slicer:layer-world@1.0.0` → `slicer:world-layer@1.0.0` in `wit_host.rs` requires the host's `bindgen!` output types to be regenerated. This is a breaking change for any code that imports from the old package name — but since this is host-only code that doesn't expose these types publicly, it should be a contained refactor.
- **ir-types version bump**: If the canonical version is `@1.1.0`, any manifest that declares `min-ir-schema = "1.1.0"` must now pass. Existing manifests that only declare `1.0.0` range may need updating (but the IR schema version is independent of the WIT version per docs/03 architecture rules).
- **Drift detection test fragility**: Extracting `include_str!` paths by string search is fragile. A more robust approach is to use a const that holds the string directly and compare that. Consider defining a const in each crate: `const CANONICAL_WIT_TYPES: &str = include_str!("../../wit/deps/types.wit");` and comparing those consts across crates.
