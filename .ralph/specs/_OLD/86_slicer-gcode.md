---
status: implemented
packet: 86
task_ids: [TASK-236]
---

# 86_slicer-gcode

## Goal

Move `gcode_emit.rs` (1 914 LOC: `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `serialize_thumbnail_block`, and the in-process `GCODE_EMIT_PRODUCER` static) out of `slicer-runtime/src/` into a new `slicer-gcode` crate, together with the `GCodeEmitter` and `GCodeSerializer` trait definitions currently in `crates/slicer-runtime/src/postpass.rs`; keep a thin `GCodeEmitProducer` wrapper (~30 LOC) in `crates/slicer-runtime/src/builtins/` that owns the `BuiltinProducer` impl and the `Blackboard` commit (per ADR-0001); `FeedrateConfig` already lives in `slicer-ir` after P84, so `slicer-gcode` imports it from there; the `classify_layers` call inside `emit_gcode` imports from `slicer-core` (where P84 moved the kernel).

## Problem Statement

`gcode_emit.rs` (1 914 LOC) is the final large concern in `slicer-runtime` that has nothing to do with orchestration: it is pure IR → text serialization. The structural problem is the same as the prior moves — three concerns braided:

1. **Pure serialization** (`DefaultGCodeEmitter`, `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `serialize_thumbnail_block`) — the depth this packet extracts.
2. **A `BuiltinProducer` impl + `Blackboard` commit** — the runtime wrapper preserved per ADR-0001.
3. **Trait definitions** (`GCodeEmitter`, `GCodeSerializer`) that today live in `crates/slicer-runtime/src/postpass.rs` and are re-exported through `slicer-runtime`'s `pub use postpass::{GCodeEmitter, GCodeSerializer};` block. They belong with the serialization, not with the orchestrator that calls them.

The fix mirrors P84's algorithm split: kernel + trait defs move to `slicer-gcode`; a ~30-LOC `GCodeEmitProducer` wrapper stays in `slicer-runtime/src/builtins/`. After P86, the runtime can be unit-tested without g-code knowledge, and g-code serialization can be golden-tested without runtime — both interfaces deepen.

`FeedrateConfig`'s move to `slicer-ir` (P84 prework) means `slicer-gcode` imports `FeedrateConfig` from `slicer-ir`, not from runtime. `overhang_classifier::classify_layers` lives in `slicer-core` (P84) — `slicer-gcode`'s emit path calls it from there.

## Architecture Constraints

- ADR-0001 preserved: the `GCODE_EMIT_PRODUCER` `BuiltinProducer` impl and the `Blackboard` commit live in `slicer-runtime/src/builtins/gcode_emit_producer.rs` (in-stage).
- ADR-0002 / 0003 (preserved); ADR-0005 / 0006 (P83 — runner traits + export_for_stage_id); ADR-0007 (P85 — CompiledModule Static/Live split with HashMap-keyed pairing) all preserved. ADR-0004 (Test support in slicer-sdk, P77) is unrelated to this packet's surface.
- `slicer-gcode` MUST NOT depend on `slicer-runtime`, `slicer-wasm-host`, `slicer-scheduler`, `slicer-schema`, `slicer-sdk`, `slicer-model-io`. Path deps in `slicer-gcode/Cargo.toml` are limited to `slicer-ir`, `slicer-core`, `slicer-helpers` plus crates.io external deps the moved code uses.
- No path in this packet's change surface feeds the guest WASM build. `wasm-staleness` snippet intentionally NOT included.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

(`gcode_emit` converts INTEGER UNITS BACK TO MILLIMETRES at the text-output boundary. The serializer's `format_xyz` / `format_coord` helpers are the conversion site; preserve their exact rounding behavior or AC-7 SHA parity breaks.)

## Data and Contract Notes

- `GCodeEmitter::emit_gcode` signature after move:
  ```rust
  pub trait GCodeEmitter {
      fn emit_gcode(&self, layers: &[LayerCollectionIR])
          -> Result<GCodeIR, GCodeEmitError>;
      fn travel_feedrate_mm_per_min(&self) -> Option<f32> { None }
  }
  ```
  The wrapper in `slicer-runtime` calls `emit_gcode(&layers)` directly; no context indirection. The `&Blackboard` parameter is dropped (Step 1 dispatch #1: it was `_blackboard`, unused, with zero method calls).
- `GCodeSerializer::serialize_gcode` signature:
  ```rust
  pub trait GCodeSerializer {
      fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError>;
  }
  ```
- `GCodeEmitError` enum mirrors the existing `PostpassError` variants that `gcode_emit` was the source of. The wrapper converts `GCodeEmitError` → `PostpassError` via a `From` impl.

## Locked Assumptions and Invariants

- ADR-0001 preserved: the `GCODE_EMIT_PRODUCER`'s commit-to-`Blackboard` happens inside the wrapper in `slicer-runtime/src/builtins/gcode_emit_producer.rs`.
- Byte-identical g-code: AC-7 SHA = P85 closure SHA = ... = P81 closure SHA. The cross-packet baseline is preserved.
- OrcaSlicer-parity constants (`;TYPE:` labels, retract opcodes, thumbnail format) are preserved verbatim — `gcode_emit.rs`'s content moves unchanged into `slicer-gcode`.
- Guest WASMs stay clean (`--check`); no guest-feeding path is edited.

## Risks and Tradeoffs

- **Risk: `GCodeEmitError`-to-`PostpassError` conversion misses a variant.** Mitigation: walk `gcode_emit.rs` `Err(_)` paths and map each to a `GCodeEmitError` variant; the `From<GCodeEmitError> for PostpassError` impl in `slicer-runtime` covers all of them.
- **Risk: SHA divergence on AC-7** because the moved code's behavior differs from before. Mitigation: the move is verbatim — dropping `_blackboard` changes nothing observable because it was already unused (line 263 of pre-move gcode_emit.rs). If SHA does diverge, the likely culprit is the `classify_layers` re-import from `slicer-core` (P84) or the `FeedrateConfig` re-import from `slicer-ir` (P84).
- **Tradeoff: introduces `GCodeEmitError`** — one new type to maintain the dep direction. Acceptable cost; the alternative is `slicer-gcode → slicer-runtime` back-edge.
