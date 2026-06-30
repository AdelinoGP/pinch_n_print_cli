---
status: implemented
packet: 127_sdk_wit_origin_propagation
task_ids:
  - TASK-252
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: This file lives in the spec-packet-generator skill. The skill writes a copy into ./.ralph/specs/<spec-slug>/ with status set to draft or active.
---

# Packet Contract: 127_sdk_wit_origin_propagation

## Goal

Add an explicit `set-current-origin` method to the WIT `perimeter-output-builder` resource and a matching `begin_region` context method on the SDK `PerimeterOutputBuilder`, so per-region perimeter output pushes (walls, infill areas, seam candidates, reordered wall loops) carry the origin of the region the guest is currently iterating rather than the last-touched WIT view's stale LIFO origin — restoring per-tool sparse-infill distribution on the painted-cube fixture to OrcaSlicer parity.

## Scope Boundaries

This packet adds one WIT method and one SDK method, threads the new explicit origin through the macro drain, and adds one `begin_region` call at the top of the `for region in regions` loop in four guest modules (classic-perimeters, arachne-perimeters, seam-placer, fuzzy-skin). The host's `effective_perimeter_origin` gains the explicit origin as a highest-precedence fallback (additive — the existing `touch_*` fallback stays as defence-in-depth). The marshal `OriginBucket` and `convert_perimeter_output` are unchanged. The infill stage, support stage, and `resolved_seam` drain gap are explicitly out of scope.

## Prerequisites and Blockers

- Depends on: the uncommitted marshal precondition (11 files from the diagnose session: per-call `infill_areas` accumulation + `OriginBucket` per-origin drain in `marshal/out.rs`, `marshal/accumulators.rs`, `host.rs`, SDK `builders.rs`, macro `__slicer_drain_perimeter`, and 5 test files). This packet folds those changes into its Step 1 commit. The precondition is a no-op on gcode without the explicit-origin mechanism this packet adds.
- Depends on: packet 126 (TASK-250, MMU painted-cube parity) — introduced the multi-region `variant_chain` that creates the multi-region dispatch scenario this bug surfaces on.
- Depends on: packet 95 (TASK-245/246, paint-segmentation OrcaSlicer parity port) — introduced per-color region splitting.
- Unblocks: a future infill-stage origin-propagation packet (the infill `HostInfillOutputBuilder` has the same LIFO-touch bug via `current_slice_region.clone()`; this packet establishes the `begin_region` SDK pattern the infill sequel can adopt).
- Activation blockers: none. All design questions settled during the grilling session (Shape 2 + Sub-shape 2A, additive origin chain, resolved_seam deferred, 4-module scope).

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** `resources/cube_4color.3mf` sliced with classic-perimeters only (no arachne) via `pnp_cli slice --model resources/cube_4color.3mf --no-default-module-paths --module-dir tmp/repro/modules-no-arachne --output tmp/repro/pnp_out.gcode`, **when** the per-tool `;TYPE:Sparse infill` extrusion segment count is tallied, **then** `T1` count is >= 1000 (OrcaSlicer golden: 1243; PnP pre-fix: 30 — unretract priming only) and `T3` count is <= 1500 (OrcaSlicer golden: 992; PnP pre-fix: 2425 — absorbing T1/T2). | `awk 'BEGIN{layer=0;t="";tool=""} /^;LAYER_CHANGE/{layer++;t="";tool=""} /^;TYPE:/{t=$0} /^T[0-9]+$/{tool=$0} /^G1 /&&/E/{if(t==";TYPE:Sparse infill"){key=tool; if(key=="")key="(no tool)"; counts[key]++}} END{for(k in counts) print k, counts[k]}' tmp/repro/pnp_out.gcode | sort`

- **AC-2. Given** the existing regression test `cube_4color_first_layer_perimeter_colour_matches_bottom_face` in `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs`, **when** `cargo test -p slicer-runtime --test executor -- cube_4color_first_layer_perimeter_colour_matches_bottom_face` runs, **then** it passes (no regression on wall tool attribution — walls were already correctly tagged via the spatial fallback; the explicit origin makes the fallback redundant but must not break it). | `cargo test -p slicer-runtime --test executor -- cube_4color_first_layer_perimeter_colour_matches_bottom_face 2>&1 | tail -3`

- **AC-3. Given** `resources/cube_4color.3mf` sliced through the full executor test path, **when** the new test `cube_4color_sparse_infill_per_painted_region` runs, **then** it asserts: (a) all four tool indices T0, T1, T2, T3 each have at least one `;TYPE:Sparse infill` segment with a `G1 ... E` extrusion move (today T1 is effectively absent), and (b) T1 sparse-infill segment count >= 1000, and (c) T3 sparse-infill segment count <= 1500. | `cargo test -p slicer-runtime --test executor -- cube_4color_sparse_infill_per_painted_region 2>&1 | tail -3`

- **AC-4. Given** a `HostExecutionContext` with only `explicit_perimeter_origin` set (no `current_perimeter_region`, no `current_slice_region`), **when** `set_infill_areas` and `push_wall_loop` are driven through the `HostPerimeterOutputBuilder` trait impl, **then** `convert_perimeter_output` produces a `PerimeterRegion` whose `object_id` and `region_id` match the explicit origin, not empty-string. | `cargo test -p slicer-wasm-host --test contract -- set_current_origin_routes_to_correct_bucket 2>&1 | tail -3`

- **AC-5. Given** a `HostExecutionContext` with no explicit origin set and only `current_slice_region` set (the pre-fix fallback path), **when** `set_infill_areas` is driven through the trait impl, **then** `convert_perimeter_output` produces a `PerimeterRegion` whose `object_id` matches the slice region's UUID (the existing fallback behaviour is preserved — additive, not replacement). | `cargo test -p slicer-wasm-host --test contract -- layer_perimeters_origin_falls_back_to_slice_region_through_host_trait 2>&1 | tail -3`

- **AC-6. Given** the workspace after all packet edits, **when** `cargo clippy --workspace --all-targets -- -D warnings` runs, **then** it exits 0 (no warnings — the `--all-targets` flag compiles test and bench targets per the project's acceptance gate). | `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`

## Negative Test Cases

- **AC-N1. Given** a `HostExecutionContext` with no origin set at all (no `explicit_perimeter_origin`, no `current_perimeter_region`, no `current_slice_region`), **when** `set_infill_areas` is driven through the trait impl, **then** `convert_perimeter_output` produces a single anonymous `PerimeterRegion` with `object_id = ""` and `region_id = 0` (the existing `OriginBucket` anonymous-mode behaviour is preserved — a guest that never calls `begin_region` and never touches a WIT view gets the same fallback as today, not a hard error). | `cargo test -p slicer-wasm-host --test contract -- effective_perimeter_origin_is_none_when_neither_set 2>&1 | tail -3`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test executor -- cube_4color 2>&1 | tail -3`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — the WIT `perimeter-output-builder` resource definition and the host-boundary enforcement contract. Delegate a SUMMARY of the section on `perimeter-output-builder` (the doc is large; only the resource-method table and the origin-tagging paragraph are needed).
- `docs/02_ir_schemas.md` — `PerimeterIR` / `PerimeterRegion` struct definitions (§IR 2). Delegate a SUMMARY of the `PerimeterRegion` field list (object_id, region_id, walls, infill_areas, seam_candidates, resolved_seam).
- `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` — the `OriginBucket` design ADR. Read directly (it's a single ADR file, likely < 200 lines) for the all-or-none origin-attribution rule this packet preserves.
- `CONTEXT.md` §"Marshalling boundary" — the glossary term for the marshal's re-attribution responsibility. Read directly (small glossary file).

For each doc, the implementer should delegate unless the file is small (< 300 lines) and only one section is needed.

## Doc Impact Statement

- `docs/07_implementation_status.md` §<task entry for TASK-252> — `rg -q 'TASK-252' docs/07_implementation_status.md` (new task entry recording the packet closing the SDK→WIT origin propagation gap for perimeters infill; cross-references packet 126 and packet 95).
- `CONTEXT.md` §Terms — `rg -q 'Per-region output origin' CONTEXT.md` (new glossary term for the explicit `begin_region` / `set-current-origin` mechanism, distinguishing it from the `touch_slice_region` fallback).
- `docs/adr/` — `rg -q 'ADR-0022' docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md` (new ADR documenting the Shape 2 vs Shape 1 trade-off and the additive origin-chain decision).

The doc edits must land in the same packet (not deferred to a follow-up); the verification greps are appended to the Acceptance Criteria above and gate packet close.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644` — per-region `set_infill_areas` call site; OrcaSlicer uses a per-output-builder-instance vector (one `SurfaceCollection*` per `LayerRegion`), not a last-write-wins accumulator. This packet's `begin_region` mechanism is the SDK/WIT analogue of OrcaSlicer's per-container association, adapted to PNP's single-builder WIT contract.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:1541-1892` — `prepare_infill` where the host-side partition hook splits infill_areas by region priority (the OrcaSlicer equivalent of `sync_perimeter_infill_areas_into_slice`).
- `OrcaSlicerDocumented/src/libslic3r/Layer.cpp:296-332` — the multi-region path where a temporary `SurfaceCollection` is split back to individual regions via `intersection_ex` against each region's slice geometry. Confirms that region identity in OrcaSlicer is implicit in container ownership, not a tag on each surface.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- [AC-1, §Acceptance Criteria] — Specified: T1 >= 1000 AND T3 <= 1500 per-tool sparse-infill segment count | Implemented: NOT verified — absolute thresholds blocked on a separate pre-existing infill-generation bug (multiple infill patterns running concurrently, ~9-12x move-count inflation); origin mechanism verified working (T1: 30→14906) | Reason: user confirmed infill-generation bug is out of scope, addressing in another session.
- [AC-3, §Acceptance Criteria] — Specified: test asserts (a) all four tools in sparse infill, (b) T1 >= 1000, (c) T3 <= 1500 | Implemented: test asserts only (a); (b) and (c) omitted | Reason: absolute thresholds blocked on the infill-generation bug (same as AC-1).
- [Scope, §Scope Boundaries + requirements.md §Out of Scope] — Specified: infill stage deferred to a sequel packet | Implemented: infill stage origin propagation implemented in this session (WIT `set-current-origin` on `infill-output-builder`, SDK `InfillOutputBuilder.begin_region`, host `set_current_origin` on `HostInfillOutputBuilder`, macro `__slicer_drain_infill` forwarding, `begin_region` in top-surface-ironing + rectilinear-infill + gyroid-infill + lightning-infill) | Reason: user asked to fold it into the same session.
- [Docs, CONTEXT.md §Per-region output origin] — Specified: "infill and support stages (deferred to sequel packets)" | Implemented: term updated to mention infill output pushes; only support stage now deferred | Reason: term was written for perimeter-only scope, corrected after the infill extension was folded in.