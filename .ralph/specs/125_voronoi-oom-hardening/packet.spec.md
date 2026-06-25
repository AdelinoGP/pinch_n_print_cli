---
status: active
packet: 125_voronoi-oom-hardening
task_ids: []   # none — bug-fix from the 2026-06-24 diagnose session.
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 125_voronoi-oom-hardening

> **Scope pivot (rescoped in place).** WI-1 (diagnose-first) **falsified** this packet's original
> hypothesis (a `boostvoronoi::discretize` OOM). The confirmed root cause is a `region_id`↔tool-index
> **convention violation** in the G-code emitter — unrelated to Voronoi. The slug is retained for
> continuity (the directory is already committed); the contract below targets the real bug. The
> original discretize-cap / paint-seg-robustification / sort-pass work items are **dropped** (the
> `boostvoronoi::discretize` unbounded loop is a real *latent* bug per `OOM_FINDINGS.md` but is **not**
> this crash — tracked as a separate optional hardening follow-up).

## Goal

Eliminate the painted-model OOM by fixing the `region_id`↔tool-index conflation: restore correct
paint→tool resolution so painted entities carry a real tool index, make the resolver fallback never
leak a `region_id` identity into the tool slot, and bound-check the emitter's per-tool allocation.

## Scope Boundaries

This packet fixes the confirmed chain — paint-variant `region_id` (a 64-bit identity) leaking through
`layer_executor.rs`'s `.unwrap_or(region.region_id)` tool fallback into `RegionKey.region_id` (the tool
slot), which `slicer-gcode/src/emit.rs` reads as a tool index and uses to size a dense `vec![0.0f32;
max_tool + 1]` → ~9.9 GiB OOM. It keeps the WI-1 OOM tripwire and the non-vacuous fuzzy test. It does
NOT rename the overloaded `region_id`/tool field (latent design smell, out of scope), fix the separate
catchable `fpv.is_finite()` boostvoronoi panics on the painted path, or touch Voronoi.

## Prerequisites and Blockers

- **Depends on:** the WI-1 capture (region_id `0x3E8281949ECA9508`; `as u32` = 2,664,076,552 = the
  emitter's `max_tool`; `vec![0.0f32; 2,664,076,553]` = 9.924 GiB) and the guarded allocator already in
  the working tree.
- **Unblocks:** painted/MMU models slicing at all; correct per-colour tool assignment.
- **Activation blockers:** none. WI-1 is complete; the chain is code-confirmed end-to-end.

## Acceptance Criteria

- **AC-1. Given** a wall-loop / path whose four tool resolvers all return `None`, **when**
  `layer_executor.rs` computes `resolved_tool`, **then** the stored `RegionKey.region_id` (tool slot) is
  the bounded default `0`, **never** `region.region_id` (a `paint_variant_region_id` identity). | `cargo test -p slicer-runtime --test integration -- tool_fallback_never_leaks_region_identity`
- **AC-2. Given** a painted entity (e.g. cube_fuzzyPainted's painted face), **when** its tool is
  resolved, **then** `paint_tool = dominant_tool_index(&wl.feature_flags)` is `Some(t)` with `t` a valid
  small tool index, so the `.unwrap_or` fallback does not fire (parity: the painted region gets its real
  tool). | `cargo test -p slicer-runtime --test executor -- painted_entity_resolves_real_tool`
- **AC-3. Given** `cube_4color.3mf`, **when** sliced, **then** the emitted G-code's set of `T<n>` tool
  indices equals the model's painted tool set (each `< extruder count`); no garbage/identity tool id
  appears. | `cargo test -p slicer-runtime --test executor cube_4color_paint`
- **AC-4. Given** `cube_fuzzyPainted.3mf`, **when** sliced through the executor bucket, **then**
  `cube_fuzzy_painted_face_jitter` runs to completion (no OOM) AND executes its `painted_face_pts as f32 >
  unpainted_face_pts as f32 * 2.0` assertion (the `pts.is_empty()` / `painted==0 || unpainted==0` paths
  are hard failures, not `return` skips) AND asserts ≥ 2 distinct `PaintValue` colour regions on the
  painted layer. | `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter`
- **AC-5. Given** the guarded >1 GiB allocator active in the executor bucket, **when** the bucket runs
  10× in a loop, **then** no single allocation exceeds 1 GiB on the painted path (the tripwire never
  fires). | `cargo test -p slicer-runtime --test executor -- mmu_no_oversized_alloc_repeat`
- **AC-6. Given** `cube_4color.3mf`, **when** sliced, **then** all `cube_4color_paint` executor tests
  stay green (no parity regression from the tool-resolution change). | `cargo test -p slicer-runtime --test executor cube_4color_paint`

## Negative Test Cases

- **AC-N1. Given** a synthetic entity whose `region_key.region_id` is an out-of-range value
  (e.g. `2_664_076_552`), **when** `slicer-gcode/src/emit.rs` sizes the per-tool buffer, **then** it
  rejects/clamps the id with a typed error instead of allocating `vec![0.0f32; id + 1]` (no >1 GiB
  allocation; tripwire not fired). | `cargo test -p slicer-gcode -- emit_rejects_out_of_range_tool_id`

## Verification

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`   (only if a guest module is touched; this packet is host-side)
- `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter`

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PaintValue` / region structures for AC-2/AC-3 (delegate a field FACT).
- `CLAUDE.md` §"Test Discipline", §"Coordinate System Hazard" — small, load directly.
- `OOM_FINDINGS.md` + the WI-1 capture — the authoritative evidence for the confirmed chain.

## Doc Impact Statement (Required)

**`none`.** The fix restores the **existing, already-assumed** host convention (`emit.rs` treats
`region_key.region_id` as a tool index via `as u32` at many sites); it changes no IR/WIT/manifest
contract and adds no public surface. The overloaded `region_id`/tool field is a pre-existing latent
design smell, not introduced here (noted as an out-of-scope follow-up in `design.md`).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
