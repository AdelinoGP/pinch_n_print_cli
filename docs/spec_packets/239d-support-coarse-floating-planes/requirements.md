# Requirements: 239d-support-coarse-floating-planes

## Packet Metadata

- Grouped task IDs: `TASK-523`..`TASK-530` (this packet's step mapping; re-derive the
  registration range against `docs/07_implementation_status.md` at registration time)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 239c (implemented) made support Z independent of the object grid, but only in the
FINER direction. Measured 2026-08-31 on
`crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` with the tracked
config, flag true:

- `support_layer_height_mm = 0.1` over `layer_height` 0.2 → 273 distinct `;Z:`, 123
  extruding off-grid rows (finer direction works).
- `support_layer_height_mm = 0.3` over `layer_height` 0.2 → 150 distinct `;Z:`, **0**
  off-grid rows (coarse direction degenerates to the object grid).
- `support_layer_height_mm = 0.3` over `layer_height` 0.1 → 299 distinct `;Z:`, 0 off-grid
  rows; family-labeled support rows (per the TASK-523 record): normal(auto) 85 of 299 rows,
  tree(auto) exploratory run 248 rows — in both families the grid decimation did NOT cut
  support to ~every 3rd row.

Root cause: the 239c intermediate-plane derivation (`packet239c_intermediate_planes` in both
planners) brackets consecutive support rows, which sit at object-grid spacing; when the
support pitch >= the object gap, `n_layers_extra == 1` and the stack stays grid-bound.
Canonical OrcaSlicer does not degenerate: `raft_and_intermediate_support_layers`
(`Support/SupportMaterial.cpp`) brackets the sorted `extremes` — the top/bottom contact
layers, which span many object layers — and fills between consecutive ones at
`step = dist / n_layers_extra ≈ pitch`, free-floating relative to the object grid. That is
the user-visible speed purpose of the toggle ("supports are waste material; print them
coarser"). 239c delivered free-floating planes; 239d must deliver free-floating STACKS in
the coarse direction.

The decimation question is answered by measurement: `build_emit_schedule`
(`crates/slicer-core/src/algos/support_geometry.rs`) gates the host-side `SupportGeometryIR`
only, and both planners ignore `SupportGeometryView` on the meshed-object planner path (the
tree's sole read is the mesh-less legacy contact fallback in the tree planner's
`SupportPlanner::plan_for_object` — a genuinely mesh-less object with no contacts; the
traditional planner's `_support_geometry` parameter at its `run_support_geometry_with_analysis`
entry is never read) — so the host decimation never reaches the meshed-object planner path
(family-labeled: normal(auto) support on 85/299 rows, tree(auto) exploratory run 248 rows,
despite the decimation). Only the traditional
planner decimates, via `support_step = round(pitch / gap)`, which is on-grid. The coarse
rows must therefore come from a new floating stack, not from either decimation mechanism.

## In Scope

- **Planner derivation (both planners).** For each consecutive demanded bracket pair where
  the binding coarse predicate holds (configured nonzero pitch >= `local_support_gap`, the
  maximum positive anchor-Z difference between consecutive surviving support-bearing rows
  of that same `(object_id, region_id)` contiguous run covered by the bracket; these rows
  are already available to both planner callers, compared in exact canonical units with
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the only tolerance if needed —
  no new epsilon), generate the support stack at pitch
  spacing between the brackets of each
  `(object_id, region_id)` contiguous run of demanded rows, following the
  **family-specific** canonical stepping — traditional:
  `n_layers_extra = ceil((dist - EPSILON) / pitch)`, `step = dist / n_layers_extra`, planes
  at `below_z + k * step`, last aligned to the upper bracket, per
  `raft_and_intermediate_support_layers` (`Support/SupportMaterial.cpp`) — its EPSILON bias
  is part of the rule; tree: `n_layers_extra = ceil(dist / pitch)`, same step/plane shape,
  **no** EPSILON bias, per `plan_layer_heights` (`TreeSupport.cpp`). One shared formula for
  both families is a spec defect, not a simplification. Bracket selection is binding: the
  run's interface-role rows (`TopInterface`/`BaseInterface`/`BottomInterface`) bracket when
  at least two exist, otherwise the run's first/last support-bearing rows. Synthesized
  stack planes **clone the lower bracket's geometry and rewrite roles to `SupportBody`**:
  the source `global_layer_index` is captured into the local duplicate key and clone-source
  provenance decision only, the emitted entry's final `global_layer_index` is assigned from
  the per-plane DEV-163 synthetic identity map (`BTreeMap<i64, i32>`) so all entries at one
  synthesized plane share that plane identity, and other provenance fields are preserved.
  After
  stepping, apply the canonical grouping rule of
  `generate_support_layers` (`Support/SupportCommon.cpp`): group candidate print-Z within
  `EPSILON`, take the midpoint. Canonical's group **minimum-height** rule is explicitly
  **not** reproduced: `SupportPlanEntry` has no height field, and a row's effective height
  derives from the `anchor_z` of its adjacent rows, so a per-entry group height has no
  representation in PnP — a recorded inapplicability, not an omission. The body rows
  between the brackets are replaced by the pitch-spaced stack. Entries per object are
  nondecreasing in `anchor_z` in original output order, distinct planes strictly increasing,
  and duplicate synthesized candidates are prevented at insertion by the stable identity key
  `(source global_layer_index, object_id, region_id, ordered body_ids, synthesized anchor_z)`
  — the cloned lower bracket's source-entry identity (its `global_layer_index`, owning
  `object_id`/`region_id`, and ordered `body_ids: Vec<String>`) plus the synthesized plane's
  `anchor_z`; no entry `id` field exists to key on, and because the key spans the full source
  identity it cannot collide two legitimately distinct geometries (a second entry with the
  same key is dropped, so the per-object entry map never carries two rows with one identity),
  and each synthesized plane's `anchor_layer_index` is the true-nearest object layer by
  absolute Z distance with the lower index winning ties. Coarse-vs-finer selection is
  bracket-local: it is decided per bracket pair by the binding predicate above (coarse iff
  configured nonzero pitch >= `local_support_gap`; pitch 0.2 over covered surviving-row
  gaps 0.3 stays finer even
  if the first/base layer gap is 0.2), never from the first/contact layer height
  alone.
- **Decimation reconciliation.** The floating stack is the source of coarse rows. The
  traditional `support_step` decimation is neutralized for bracket pairs satisfying the
  binding coarse predicate (configured nonzero pitch >= `local_support_gap`)
  (binding form: set `support_step = 1` exactly for those coarse brackets; no global
  gate bypass; the stack replaces it; `support_step` stays for the finer direction where it
  is already 1).
  `build_emit_schedule` stays read-only (out of scope; documented as ineffective for the
  planner path).
- **Extrusion-presence ACs.** AC-1 asserts every off-grid support row carries at least one
  G1 move with `E > 0` — the DEV-161 defect class (off-grid rows whose moves carry no E) is
  a FAIL, not a human-gate-only finding.
- **Measure-first coarse `height_delta` verdict.** A TASK-519-pattern measurement of the
  height term `DefaultGCodeEmitter::emit_gcode` applies to a coarse 0.3-pitch off-grid pass
  (applied height, declared plane delta, resulting E), recorded under `TASK-527`, with a
  verdict AC (AC-4) asserting the recorded branch.
- **Blocking human validation gate.** Visual-debug bundle + inspection checklist, signed
  before `status: implemented` (see `packet.spec.md` §Human Validation Gate).

## Out of Scope

- The renderer/row path: the tree renderer traverses `paint.support_plan()` and the
  traditional renderer consumes `support_plan_entries_for`
  (`PaintRegionLayerView`) — both emit at the plan-declared `anchor_z` already (239c); the
  anchored transport is complete
  (DEV-159..163). No transport changes are expected; if a renderer edit is needed, it is a
  scope change.
- `build_emit_schedule` and `execute_support_geometry`
  (`crates/slicer-core/src/algos/support_geometry.rs`,
  `crates/slicer-runtime/src/builtins/support_geometry_producer.rs`) — read-only pre-239c
  surface; the host decimation is not the coarse-row mechanism.
- The finer direction (pitch < gap): the 239c derivation is unchanged (AC-N2 pins it).
- The `support_layer_height_mm == 0.0` sentinel: stays "object pitch" (239c [FWD] option b,
  recorded in both planner `lib.rs` files; AC-N3 pins it).
- 239c's closed Step 2/4 semantics (the enabled/disabled derivation rules and the renderer
  emission): do not reopen; if the coarse direction needs a planner change beyond the
  derivation site, `design.md` states the blast radius explicitly.
- WIT, IR schemas, manifests, host config keys: none change.

## Authoritative Docs

- `docs/spec_packets/239c-support-layer-height-producer/packet.spec.md` - direct ranged read
  of §Acceptance Criteria, the test-naming convention, and §Human Validation Gate.
- `docs/spec_packets/239c-support-layer-height-producer/design.md` - direct ranged read of
  §Code Change Surface, §Locked Assumptions and Invariants, §Open Questions (the `[FWD]`
  sentinel decision).
- `docs/specs/support-independent-layer-z-split-plan.md` - direct read of the canonical block
  and the packet queue.
- `docs/DEVIATION_LOG.md` - rows `DEV-159`..`DEV-163` only; direct range read.
- `docs/specs/support-parity-gap-register.md` - rows `G-02` and the new coarse row only;
  direct range read. Never full-read (the file is long).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `raft_and_intermediate_support_layers`: the non-synchronized branch brackets the sorted `extremes` (top/bottom contact layers) and fills between consecutive ones at `n_layers_extra = ceil((dist - EPSILON) / max_suport_layer_height)`, `step = dist / n_layers_extra`, `print_z = extr1z + i * step`, last layer aligned to `extr2z`; the synchronized branch (flag disabled) snaps to object layers. This is the **AC-3 (traditional-family)** stepping rule and the bracket-selection ground truth; the AC-2 (tree-family) rule is `plan_layer_heights` (`TreeSupport.cpp`) with no EPSILON bias.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`: the grouping predicate (`print_z <= first.print_z + EPSILON`), the midpoint Z rule (`zavg = 0.5 * (first + last)`), and the group-height rule (minimum). This is the grouping/midpoint step the coarse stack applies after stepping.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `max_suport_layer_height = max_layer_height` (nozzle-derived via `max_layer_height_from_nozzle`, clamped >= object layer height). Confirms canonical has no `support_layer_height_mm` key; PnP's key is the pitch knob this packet uses.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — the parallel support-layer generation loop (`n_layers_extra = ceil(dist / max_layer_height)`, `step = dist / n_layers_extra`, `print_z = z1 + step`): the tree-family stepping, **no** EPSILON bias — the AC-2 rule; `raft_and_intermediate_support_layers` (with its EPSILON bias) is the AC-3 rule. One shared formula for both families would be wrong.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (real-slice coarse proof: off-grid rows, strict superset, every off-grid
  support row extrudes), `AC-2` (tree planner free-floating `anchor_z` + exact expected
  bracket planes per the `plan_layer_heights` (`TreeSupport.cpp`) formula, original output
  order, `SupportBody` role replacement, nearest anchoring),
  `AC-3` (traditional planner same + `support_step` neutralized, exact planes per the
  `raft_and_intermediate_support_layers` (`Support/SupportMaterial.cpp`) formula), `AC-4`
  (measure-first coarse `height_delta` verdict asserting the recorded applied-height
  constants), `AC-5` (guest freshness gate).
- Negative: `AC-N1` (disabled reproduces the pre-change Z sequence exactly), `AC-N2` (finer
  direction unregressed — the 239c AC-1 test stays green), `AC-N3` (sentinel 0.0 stays
  object pitch), `AC-N4` (**NET-NEW Step-2 test planned by this packet**, registered in the
  existing `tree_family_tdd` target; adaptive finer-direction local gap (a bracket pair
  whose `local_support_gap` exceeds the configured pitch) stays finer —
  bracket-local coarse/finer selection by the binding predicate).
- Cross-packet impact: `242-support-family-orca-closure` consumes the coarse-direction
  behavior; the 239c AC-1/AC-N1/AC-N2 tests must stay green (AC-N2 pins AC-1 explicitly).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- coarse_support_pitch_emits_free_floating_extruding_rows --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1: real slice, coarse off-grid extruding rows | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- coarse_pitch_produces_free_floating_anchor_z --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2: tree planner coarse derivation | FACT pass/fail |
| `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- coarse_pitch_produces_free_floating_anchor_z --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3: traditional planner coarse derivation + `support_step` neutralized | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd -- coarse_pass_height_delta_matches_recorded_verdict --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4: measure-first coarse verdict | FACT pass/fail |
| `cargo xtask build-guests --check && echo FRESH` | AC-5: guest freshness before every slice-level evidence run | FACT exit code |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- disabled_coarse_pitch_reproduces_baseline_z_sequence --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1: disabled reproduces baseline | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- independent_support_layer_height_emits_support_row_off_object_grid --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2: finer direction unregressed (239c AC-1) | FACT pass/fail |
| `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- zero_pitch_sentinel_stays_object_grid --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3: sentinel 0.0 stays object pitch | FACT pass/fail |
| `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- adaptive_local_gap_stays_finer --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N4: adaptive local gap stays finer (bracket-local selection) — NET-NEW test planned by this packet, registered in the existing `tree_family_tdd` target; post-implementation runnable | FACT pass/fail |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT pass/fail |
| `cargo xtask test --summary --workspace --no-fail-fast` | closure ceremony only (per Test Discipline) | FACT pass/fail |

## Step Completion Expectations

- The AC-N1 baseline (the disabled `;Z:` sequence with `support_layer_height_mm = 0.3`) is
  captured in Step 1, before any planner edit, and hard-coded in the AC-N1 test (the new
  `P239D_DISABLED_COARSE_PITCH_BASELINE_Z` const, following the 239c
  `DISABLED_INDEPENDENT_HEIGHT_BASELINE_Z` pattern) in Step 4.
- The AC-4 verdict is recorded under `TASK-527` in Step 5 before the verdict test is
  authored in Step 6.
- Ledger facts (task high-water in `docs/07`, next free `DEV-###`, next free `G-` row) are
  re-derived at Step 8, never quoted from this packet.
- The human gate's `REFS-PRESENT` precondition is verified at Step 7; the packet may reach
  "all steps complete, sign-off pending" and stop there.

## Context Discipline Notes

- Both planner `lib.rs` files are very large — ranged reads only, per the ranges in
  `design.md` §Read-Only Context and `implementation-plan.md` steps.
- `OrcaSlicerDocumented/` reads are delegated per the orca-delegation snippet; never load.
- The `docs/07_implementation_status.md` registration is a worker dispatch, never a full
  backlog read.
- The AC-1 E-assertion helper parses G1 `E` tokens; the existing precedent is
  `extract_e_values` (`crates/slicer-gcode/tests/gcode_relative_extrusion_tdd.rs`), which is
  in a different crate's test binary and cannot be imported — the helper is authored in
  `support_family_closure.rs`.
