---
status: implemented
packet: 205c-native-dispatch-seam
task_ids:
  - TASK-329
---

# 205c-native-dispatch-seam

## Goal

Deepen the native dispatch seam so native and WASM legs use one authoritative view translation, one held-claim resolver, lossless stage commits (including per-region support origins), and an explicit load-time dispatch mode.

## Problem Statement

The integrated-modules effort added a second native IR-to-view translation beside the WASM translation, duplicated held-claim resolution, and left native response commits lossy in several families: prepass stages silently return `PrepassStageOutput::None` where the WASM leg materializes output, seam-plan candidate reasons are hardcoded, `region_id` defaults to `0` on parse failure, `PaintSegmentation` and `SlicePostProcess` commits fatal despite declared envelope outputs, and support origins are hardcoded empty (the WIT `support-output-builder` lacks `set-current-origin`, the SDK builder lacks origin tracking, and `SupportIR` is flat). The live binding also represents two mutually exclusive dispatch modes with optional fields and a placeholder pool, allowing missing native entries to fail late. This packet restores locality at the native dispatch seam without changing module semantics.

## Architecture Constraints

- ADR-0021 remains the marshalling-boundary authority; do not create a third translation or move origin reconstruction into module code.
- ADR-0056's one module-loading model and external override rule remain unchanged.
- `NativeStageEntry` remains the SDK seam; the dispatch mode becomes explicit through validated provenance plus entry presence, not a replacement field.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- IR/manifest contracts: region origin and held-claim identity must survive native dispatch exactly as they do through WASM. `SupportIR` gains `regions: Vec<SupportRegion>` with `object_id`/`region_id` (mirroring `InfillIR`/`InfillRegion` at `slice_ir.rs:2149/2133`); the flat `support_paths`/`interface_paths`/`raft_paths`/`ironing_paths` fields move into `SupportRegion`.
- WIT boundary: adapters preserve existing WIT-facing semantics; the `support-output-builder` resource gains one additive method; no package/version change.
- Determinism/scheduler constraints: claim resolution remains per `(layer, object, region)` and dispatch mode does not affect scheduling.

## Locked Assumptions and Invariants

- Integrated modules use native dispatch; external overrides use WASM dispatch.
- A missing integrated native entry is a load-time error.
- Empty perimeter input is a valid no-output postprocess case.
- The WASM leg's support output shape changes with `SupportIR` (both legs emit per-region); parity comparators compare the new shape on both sides.

## Risks and Tradeoffs

- A shared view authority may require changing bindgen resource storage; keep resource accessors as adapters and preserve ownership/lifetime rules.
- The per-region `SupportIR` change ripples into gcode emission, debug render, and test fixtures; the blast radius is bounded (7 construction sites, 7 consumers, no module crates) and lands in one step.
- The WIT/SDK/macros change feeds guest WASM; stale guests must be rebuilt via `cargo xtask build-guests`.
