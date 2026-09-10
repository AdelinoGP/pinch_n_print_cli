# Design: 305-xy-size-compensation-slice-prepass

## Controlling Code Paths

- Primary code path: `PrePass::Slice` commit → NEW `commit_xy_size_compensation_builtin` (`crates/slicer-runtime/src/builtins/xy_size_compensation_producer.rs`, new) → `Blackboard::replace_slice_ir` (`crates/slicer-runtime/src/blackboard.rs`) → `PrePass::OverhangAnnotation` → `PrePass::ShellClassification` → support analysis and geometry. Pure geometry in `slicer_core::algos::xy_size_compensation` (new module beside the ungated `bridge_over_infill` in `crates/slicer-core/src/algos/mod.rs`).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/executor/prepass_executor_tdd.rs` for the blackboard-and-`run_prepass` fixture shape; `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` for the `STAGE_ORDER` / `HOST_ONLY_STAGES` / `VALID_STAGES` triangle; `crates/slicer-ir/tests/` for the `resolved_config_*_tdd` shape.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **The kernel must be ungated.** `crates/slicer-core/src/algos/mod.rs` gates every module except `bridge_over_infill` behind `#[cfg(feature = "host-algos")]`. `xy_size_compensation` joins `bridge_over_infill` as ungated. It needs nothing from behind the gate: `crates/slicer-core/src/lib.rs` declares `pub mod polygon_ops;` with no `cfg`, so `offset`, `union_ex`, `difference_ex`, `intersection_ex` and `OffsetJoinType` are all reachable. The kernel may not use `rayon` (it is behind `host-algos`), and its test target may not carry `required-features` or a file-level `cfg` — AC-14 pins both.
- **The seam is a host prepass built-in, and the stage is host-only.** The three reasons are in `requirements.md` §Problem Statement and are not restated here. What follows from them for the implementer: the stage id is a `STAGE_ORDER` member and a `HOST_ONLY_STAGES` member, and it is **absent** from `slicer_schema::VALID_STAGES` and `slicer_schema::STAGES`, which is exactly what keeps the WIT world set, the `slicer-macros` stage-glue match, `crates/slicer-ir/src/stage_io.rs`'s commit enum and the `pnp_cli module new` scaffold arm out of scope. This mirrors packet 304's `PrePass::PolyholeTransform`.
- **`PrePass::XySizeCompensation` is host-only, permanently.** Adding it to `VALID_STAGES` later would require a WIT world, an SDK trait and a `STAGES` row; that is a different packet, not a follow-up edit.
- **Ordering against the three neighbouring draft packets is fixed by canonical and must be honoured at registration time.** Canonical's `slice_volumes` calls `apply_conical_overhang` before the compensation block, and `PrintObject::slice` calls `_transform_hole_to_polyholes` after `slice_volumes` returns. So: 297's conical-overhang built-in, then this one, then 304's `PrePass::PolyholeTransform`, then `PrePass::OverhangAnnotation`. All three are `draft` and none exists on HEAD; register relative to whichever have landed, and place this one immediately after `PrePass::Slice` if neither has. Packet 303's elephant-foot is downstream of every prepass stage under either resolution of its open seam question, which is canonical's order, so nothing here depends on it.
- **Rule 4 does not fire.** Canonical has exactly one XY-compensation implementation and no enum selecting between alternatives, so there is no claim to hold or require. Do not invent a claim ID and do not mint a `Producer`.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`. **Specific to this packet:** canonical scales the two deltas into its own units with `scaled<float>()` before calling `_shrink_contour_holes`, but `slicer_core::polygon_ops::offset` takes a **millimetre** delta. The port therefore carries the deltas in millimetres end to end and performs **no** scaling at all. Porting canonical's `scaled<>` call is the single most likely arithmetic defect here and would be off by a factor of 10 000.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. This packet adds no guest and edits no guest input, so `cargo xtask build-guests --check` is expected to exit `0` throughout. Inspect the **exit code**, never grep for `STALE:` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. A stale report here means the tree was already stale; reconcile it before attributing anything to this work.
- **ADR conformance (no amendment).** `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` and `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` govern extrusion-path ordering. Neither is contradicted: this stage runs in the prepass, before any extrusion path — locked or otherwise — exists. The packet authors no ADR, so no slot is allocated.
- Schema/version constants: **not applicable**. No `PROGRESS_EVENT_SCHEMA_VERSION` bump, no `SliceIR` schema change — the pass rewrites `SlicedRegion.polygons` in place and adds no field.

## Code Change Surface

- **Selected approach.** Mirror packet 297's staging shape exactly — pure kernel in `slicer-core`, thin `Blackboard` bridge in `slicer-runtime/builtins`, one `run_builtin_stage` registration in `prepass.rs` — and packet 304's stage-registration shape for the new host-only stage id. Config arrives per object through `RegionMapIR::config_for` rather than through the raw source, because canonical's keys are `PrintObjectConfig` members and the per-object overlay is exactly what `config_for` composes. The split between kernel and producer matters: the kernel is pure, deterministic and testable natively (AC-1 through AC-8 and AC-N1/AC-N2 are ordinary `slicer-core` tests with no blackboard at all), while the producer holds only config resolution, grouping and write-back.
- **Exact functions, traits, manifests, tests, and fixtures.**
  - `slicer_core::algos::xy_size_compensation::shrink_contour_holes(polys: &[ExPolygon], contour_delta_mm: f32, hole_delta_mm: f32) -> Vec<ExPolygon>` — canonical `_shrink_contour_holes`, one expolygon at a time, accumulated and unioned.
  - `slicer_core::algos::xy_size_compensation::XySizeCompensationParams { contour_delta_mm: f32, hole_delta_mm: f32 }`.
  - `slicer_core::algos::xy_size_compensation::compensate_object_layer(regions: &mut [(RegionId, Vec<ExPolygon>)], params: &XySizeCompensationParams) -> bool` — canonical's control flow for one object on one layer.
  - `crates/slicer-core/src/algos/mod.rs` — one ungated `pub mod xy_size_compensation;` declaration.
  - `crates/slicer-runtime/src/builtins/xy_size_compensation_producer.rs` — `pub fn commit_xy_size_compensation_builtin(blackboard: &mut Blackboard) -> Result<(), XySizeCompensationBuiltinError>` with `MissingSliceIr` / `MissingRegionMap` / `Blackboard` variants; declared in `crates/slicer-runtime/src/builtins/mod.rs`.
  - `crates/slicer-runtime/src/prepass.rs` — one `run_builtin_stage` call and one `required_slots` arm; one new `PrepassExecutionError::XySizeCompensation` variant beside its siblings.
  - `crates/slicer-scheduler/src/execution_plan.rs` — one `STAGE_ORDER` entry.
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — one `HOST_ONLY_STAGES` entry.
  - `crates/slicer-ir/src/resolved_config.rs` — two `cli` fields + two `to_config_map` emissions (macro arms only).
  - Tests as listed in `requirements.md` §In Scope.
- **The hole-offset sign is the packet's one real subtlety, and it must be written down rather than discovered.** Canonical calls `offset(hole, -hole_delta)` on a **clockwise** hole polygon and relies on ClipperLib's orientation-sensitive offset to turn that into an enlarged aperture. `ExPolygon` here stores `contour` CCW and `holes` CW (`crates/slicer-ir/src/slice_ir.rs`), and `slicer_core::polygon_ops::offset` operates on whole `ExPolygon`s, not bare paths. The port must therefore make the sign explicit rather than transcribing canonical's: wrap each hole as a hole-free `ExPolygon` with its winding reversed to CCW, offset it by `+hole_delta_mm`, and difference the result out of the offset contour. Net observable, which AC-2 and AC-3 pin in both directions: **positive `xy_hole_compensation` makes the hole bigger, negative makes it smaller.** Transcribing canonical's minus sign would silently invert the key.
- **The two-pass order is behaviour, not style.** Canonical applies all non-negative deltas in one `_shrink_contour_holes` call and all non-positive deltas in a second, with the expolygon set re-derived between them, because the union/difference in the first pass can merge or split islands that the second pass then sees differently. A single call with mixed signs is not equivalent. AC-4 fails a combined implementation.
- **Rejected alternatives and reasons.**
  - *A module on `Layer::SlicePostProcess` (the tier table's owner).* Rejected for the three reasons in `requirements.md` §Problem Statement; AC-12 is the executable form of the first, and the second is a hard scheduler failure, not a preference.
  - *A module on a new module-dispatch prepass stage.* Rejected: making a prepass stage module-targetable costs a WIT world, an SDK trait, a `STAGES` row, a `stage_io.rs` commit arm and a `module_new` scaffold arm — a much larger packet than the pass it would carry, and packet 304 already set the host-only precedent for this neighbourhood.
  - *Folding into packet 304's `PrePass::PolyholeTransform` built-in.* Rejected: canonical runs the two passes in different places with `fix_slicing_errors` between them, they share no state, and folding would make each packet's activation depend on the other's.
  - *Reading the deltas from the raw config source (packet 297's convention).* Rejected: canonical's keys are `PrintObjectConfig` members, and the raw source is global. `RegionMapIR::config_for` is the seam that already composes the per-object overlay, and packet 304 uses it for the same reason.
  - *Compensating the mesh before slicing.* Rejected: canonical operates post-slicing on layer polygons; growing the mesh would change the slice topology and recurse into region assignment.
  - *Rewriting packet 303's `elefant-foot` manifest to narrow its writes so both could share `Layer::SlicePostProcess`.* Rejected: it does not fix the shell-classification problem (reason 1), and a packet must not amend another packet.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/xy_size_compensation.rs` - role: the ported kernel; expected change: new file with the mandatory porting header.
- `crates/slicer-runtime/src/builtins/xy_size_compensation_producer.rs` - role: config resolution, per-object grouping, blackboard write-back; expected change: new file.
- `crates/slicer-runtime/src/prepass.rs` - role: stage registration, `required_slots`, one error variant; expected change: three small edits in one file.
- `crates/slicer-scheduler/src/execution_plan.rs` - role: `STAGE_ORDER`; expected change: one entry.
- `crates/slicer-ir/src/resolved_config.rs` - role: the two config fields; expected change: two macro rows and two `to_config_map` emissions.

Extras beyond the five primaries, each a one-line mechanical edit and justified rather than split out: `crates/slicer-core/src/algos/mod.rs` and `crates/slicer-runtime/src/builtins/mod.rs` (one `pub mod` line each), `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` (one `HOST_ONLY_STAGES` entry — it must not be separated from the `STAGE_ORDER` edit), `crates/slicer-runtime/tests/executor/main.rs` (one `mod` line), the three new test files, the doc edits, and the two wayfinder asset annotations in Step 7.

## Read-Only Context

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - the `commit_shell_classification_builtin` body only - purpose: the clone-`Vec<SliceIR>` → mutate → `replace_slice_ir` shape, and its region-lookup helpers.
- `crates/slicer-runtime/src/prepass.rs` - the `run_builtin_stage` signature and the `PrePass::Slice` / `PrePass::OverhangAnnotation` registrations only (large file) - purpose: the exact registration and guard shape.
- `crates/slicer-core/src/polygon_ops.rs` - the `offset`, `union_ex`, `difference_ex`, `intersection_ex` and `OffsetJoinType` definitions only - purpose: confirm the millimetre delta and the join/arc-tolerance parameters before writing a single offset call.
- `crates/slicer-ir/src/slice_ir.rs` - the `SliceIR`, `SlicedRegion`, `ExPolygon`, `Polygon`, `RegionKey`, `RegionMapIR::config_for` and `MODIFIER_FOOTPRINT_REGION_ID` definitions only (very large file; ranged reads) - purpose: field names, the CCW-contour / CW-hole winding contract, and the per-region config lookup.
- `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` - whole file - purpose: what a host-only stage must and must not appear in.
- `crates/slicer-core/src/algos/mod.rs` - whole file - purpose: confirm `bridge_over_infill` is the ungated precedent before adding `xy_size_compensation` beside it.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` - the two `ORCA_CONFIG_PADDING` twins are rule-2 non-evidence and ride ticket 132; do not open it
- `crates/slicer-schema/wit/**`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-ir/src/stage_io.rs`, `crates/pnp-cli/src/module_new.rs` - the stage is host-only; if any step finds itself editing one of these, the design is wrong — stop and re-scope
- `modules/core-modules/**` - this packet ships no module and changes no manifest
- Every other `docs/spec_packets/*/` directory - never modify another packet, in particular `303-elefant-foot-slice-postprocess`
- `crates/slicer-core/src/arachne/**` and the `arachne_*` tests - unrelated; the most likely accidental scope leak in this crate

## Data and Contract Notes

- **Winding and sign.** `ExPolygon.contour` is CCW and `ExPolygon.holes` are CW. The kernel's contract is stated in observable terms, not in offset signs: positive `contour_delta_mm` grows the outer boundary, positive `hole_delta_mm` grows the hole aperture. See §Code Change Surface for why canonical's literal sign must not be transcribed.
- **Mutation target.** The pass rewrites `SlicedRegion.polygons` only. `infill_areas`, `top_solid_fill`, `bottom_solid_fill`, `bridge_areas` and `sparse_infill_area` are all produced by stages that run **after** this one, which is the whole point of the seam — there is no derived-area consistency problem to solve here, unlike at `Layer::SlicePostProcess`.
- **Determinism.** The pass is a pure function of `(SliceIR, RegionMapIR)`. Iterate layers in `global_layer_index` order and, within a layer, objects and regions in the order they appear in `SliceIR.regions`, so the region-priority trim in the positive branch is reproducible. Canonical parallelises this loop with `tbb::parallel_for`; the port runs it serially — recorded as a deviation clause, same as packet 304.
- **Blackboard ordering.** `Blackboard::replace_slice_ir` `debug_assert!`s that no Tier 2 layer slot has been written. The prepass satisfies this by construction; do not move the registration below any `Layer::` dispatch.
- **Deviation row — one row, four clauses.** Re-derive the next free `DEV-###` over `docs/DEVIATION_LOG.md` **and** `docs/spec_packets/*/` at write time; packets 303 and 304 each also intend to file one, so the authoring-time value has almost certainly rotted. The clauses: (a) canonical's two painted-object suppressions and their `active_step_add_warning` warnings are not ported, with the two blockers named; (b) canonical's `PrintApply` region-assignment bbox growth by `xy_contour_compensation` has no counterpart, because region assignment happens at `PrePass::RegionMapping` before slicing; (c) `MODIFIER_FOOTPRINT_REGION_ID` regions are passed through uncompensated, so a modifier split later runs against uncompensated modifier bounds; (d) serial where canonical is parallel.

## Locked Assumptions and Invariants

- **Ungatedness of `slicer_core::algos::xy_size_compensation` is locked.** Adding `#[cfg(feature = "host-algos")]` to it, or `required-features` to `algo_xy_size_compensation_tdd`, silently empties the narrow test run — the CLAUDE.md trap that AC-14 exists to prevent.
- **`PrePass::XySizeCompensation` must remain strictly between `PrePass::Slice` and `PrePass::ShellClassification`.** AC-12 is void if it moves; the stage's entire justification is that shell classification consumes its output.
- **The stage stays host-only.** It is a `STAGE_ORDER` and `HOST_ONLY_STAGES` member and must never appear in `VALID_STAGES`.
- Everything else is reversible via config defaults: both keys at `0.0` make the built-in a no-op that returns without touching the blackboard (AC-5), and removing the registration removes the behaviour entirely.

## Risks and Tradeoffs

- **The multi-region branch is where this packet will overrun, not the kernel.** `shrink_contour_holes` itself is about thirty lines; canonical's surrounding control flow — two growth extrema, a merge, a per-region intersect-and-trim with a running `processed` accumulator, and a separate negative-trimming path — is the larger and subtler half. It is Step 2, budgeted `M`. If it cannot land within an `M` budget, split it at the positive/negative branch boundary rather than collapsing the two branches into one.
- **Getting the hole sign backwards is a silent, plausible-looking failure.** A part with a reversed hole compensation still slices, still prints, and is wrong by twice the requested amount. AC-2 and AC-3 must be written before the implementation, not after it.
- **This tree's offset is Clipper2 with an arc tolerance; canonical's is ClipperLib.** On a circular hole the two produce different vertex counts. Assert bounding boxes and areas, never point equality, and do not author a golden fixture (`requirements.md` §Parity Evidence Standard).
- **The new stage's blast radius is bounded only because it is host-only.** The two neighbouring assertions to watch, both inherited from packet 304's analysis, are the DAG-CLI containment check (`crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`) and the built-in producer count (`crates/slicer-runtime/tests/unit/builtin_producers_tdd.rs` — note the different crate), and they hold only while this built-in mints no `Producer`. Verify, do not assume.
- **Three draft packets want positions in the same short stretch of `prepass.rs`.** None has landed. Whichever merges second must read the registrations that exist rather than the order written in its own packet.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the multi-region control flow)
- Highest-risk dispatch and required return format: the `slice_volumes` compensation block — `SNIPPETS`, at most 2 snippets of 30 lines, covering the single-region branch and the multi-region branch. Reject any reply that paraphrases the branch structure instead of quoting it, and redispatch narrower.

## Open Questions

- `[FWD]` Does any consumer between `PrePass::Slice` and this stage cache a footprint derived from the uncompensated slices? `PrePass::RegionMapping` runs before `PrePass::Slice` and holds configs rather than geometry, and `PrePass::Slice` is the producer, so the answer is expected to be "no". Resolve in Step 5 with a `LOCATIONS` query for readers of `Blackboard::slice_ir` registered before this stage; if one exists, record it as a fifth deviation clause rather than widening the packet.
- `[FWD]` Should the built-in skip the `replace_slice_ir` call entirely when every object resolves both deltas to `0.0`? Expected yes, on the AC-5 identity argument and to avoid a needless full `Vec<SliceIR>` clone on the default path. Confirm in Step 5 that skipping does not break the prepass instrumentation's expectation that a registered stage emits an event.
