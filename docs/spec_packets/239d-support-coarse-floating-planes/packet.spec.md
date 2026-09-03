---
status: implemented
packet: 239d-support-coarse-floating-planes
depends_on: 239c-support-layer-height-producer, 239a-anchored-host-seams, 239b-anchored-wit-contract
task_ids:
  - TASK-523
  - TASK-524
  - TASK-525
  - TASK-526
  - TASK-527
  - TASK-528
  - TASK-529
  - TASK-530
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 239d-support-coarse-floating-planes

## Goal

Deliver free-floating support **stacks** in the coarse direction: when the support pitch
(`support_layer_height_mm`) is >= the object layer pitch, both support planners generate the
support stack at pitch spacing between the brackets of each `(object_id, region_id)`
contiguous run — brackets selected by the measured Q1 rule (interface-role planes across
body-bearing interface spans only; otherwise the run's surviving support-bearing endpoint
fallback) — the **traditional** family following `raft_and_intermediate_support_layers`
(`Support/SupportMaterial.cpp`) stepping (`ceil((dist - EPSILON) / pitch)`, `step = dist / n`,
last plane aligned to the upper bracket) and the **tree** family following
`plan_layer_heights` (`TreeSupport.cpp`) stepping (`ceil(dist / pitch)` for main-body
spacing, **no** EPSILON bias, `step = dist / n`) — plus the `generate_support_layers`
(`Support/SupportCommon.cpp`) EPSILON candidate-grouping/midpoint rule, replacing the 239c
grid-bound degeneration — so a real nominal 0.45-pitch slice of `SupportTest.stl` emits off-grid
support rows that each extrude, with the disabled flag reproducing the baseline exactly and
the finer direction unregressed.

## Nominal Acceptance Override

The original 0.3 mm run is retained only as the measured pre-239d baseline and the small
planner-fixture pitch used by the unit tests. The binding real-slice acceptance is 0.45 mm:
the supplied human-generated references use a 0.5 mm nozzle with `max_layer_height = 0.45`
and produce approximately 0.447273 mm effective coarse spacing. PnP uses the explicit
`support_layer_height_mm = 0.45` override in the packet-specific `-045` configs.

## Scope Boundaries

Planner-side derivation only. In scope: the coarse-direction stack derivation in both
planners (the `packet239c_intermediate_planes` callers) selected per bracket pair by the
binding coarse predicate (configured nonzero pitch >= `local_support_gap`, the maximum
positive anchor-Z difference between consecutive surviving support-bearing rows of that
same `(object_id, region_id)` contiguous run covered by the bracket; otherwise the 239c
finer derivation is retained for that bracket), the traditional `support_step`
neutralization per the binding Q3 decision (entries inside the computed coarse
bracket-range membership bypass the `support_step` modulo/removal gate for bracket pairs
satisfying that predicate; rows outside the coarse ranges retain the computed
`support_step` decimation; no global bypass), the real-slice and planner-level tests including the
extrusion-presence assertions (the DEV-161 guardrail), the measure-first coarse-direction
`height_delta` verdict (TASK-519 pattern), and the blocking human validation gate. Out of
scope and owned elsewhere: the renderer/row path (unchanged; the tree renderer traverses
`paint.support_plan()` and the traditional renderer consumes `support_plan_entries_for`,
both obeying `anchor_z` — the DEV-159..163 seam
completion is inherited), the host `build_emit_schedule` decimation
(`crates/slicer-core/src/algos/support_geometry.rs` — read-only pre-239c surface), the finer
direction (unchanged), the `support_layer_height_mm == 0.0` sentinel (unchanged, 239c [FWD]
option b), and 239c's closed Step 2/4 semantics — do not reopen them.

## Prerequisites and Blockers

- Depends on: **all three** `239c-support-layer-height-producer` (implemented),
  `239a-anchored-host-seams` (implemented), `239b-anchored-wit-contract` (implemented).
  239c supplies the finer-direction derivation this packet extends, the `[FWD]` sentinel
  decision (option b: `support_layer_height_mm == 0.0` → object pitch) recorded in both
  planner `lib.rs` files, and the real-slice test infrastructure this packet's ACs reuse.
- Unblocks: `242-support-family-orca-closure`.
- Activation blockers: none. The former `[FWD]` questions Q1-Q3 are recorded as **binding
  decisions** in `design.md` §Recorded Decisions (no longer open); the only remaining open
  question (`[FWD]` Q4) is human-reference only, blocks no code, and does not block
  activation.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1 (real slice, coarse direction, free-floating extruding rows).** **Given** a real
  `slicer_runtime::run::run_slice` of
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` through the tracked
  `orca-matched-config.json` with `independent_support_layer_height = true` and
  `support_layer_height_mm = 0.45` (coarser than `layer_height` 0.2), **when** the resulting
  G-code is parsed, **then** the set of distinct `;Z:` values is a strict superset of the
  same slice's `independent_support_layer_height = false` baseline, at least one Z present
  only in the enabled run is followed by a `;TYPE:Support` block, and **every** enabled-only
  `;Z:` row followed by a `;TYPE:Support` block carries at least one G1 move with `E > 0` —
  no E=0 off-grid support row (the DEV-161 defect class). **The measured adjacent interface
  cluster must take the Q1 endpoint fallback, not block the stack**: on this fixture the
  tree run's interface roles sit only at the adjacent top rows `anchor_z` 246000/248000 and
  the traditional run's only at 244000/246000/248000 (measured 2026-09-02, the
  `[DEBUG-239D-BRACKETS]` records in `target/test-output.log`), every consecutive interface
  pair enclosing zero strictly interior non-interface rows — no body-bearing interface span —
  so each run brackets its first and last surviving support-bearing rows, emits off-grid
  rows, and the `;TYPE:Support interface` blocks of both families survive (the test asserts
  interface blocks remain present in both families' enabled G-code). |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- coarse_support_pitch_emits_free_floating_extruding_rows --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2 (tree planner: coarse direction produces free-floating `anchor_z`).** **Given** a
  `LayerPlanView` whose layers sit at 0.2 mm pitch and a support demand spanning multiple
  layers with `support_layer_height_mm = 0.3` (pitch >= object gap), **when**
  `SupportPlanner::plan_for_object`
  (`modules/core-modules/tree-support-planner/src/lib.rs`) runs with the key enabled,
  **then** at least one emitted `SupportPlanEntry.anchor_z` differs from
  `mm_to_units(layer_plan.layers[entry.anchor_layer_index].z)` by more than
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` (10 units = 1e-3 mm), the sequence
  of entries is nondecreasing in `anchor_z` within each object in the planner's original
  output order, the distinct `anchor_z` planes are strictly increasing, and the stack
  between two consecutive bracket planes follows the **tree-family** canonical stepping of
  `plan_layer_heights` (`TreeSupport.cpp`): `n_layers_extra = ceil(dist / pitch)`,
  `step = dist / n_layers_extra`, planes at `below_z + k * step` with **no** EPSILON bias and
  the last plane aligned to the upper bracket. The test asserts the **exact expected
  bracket planes** (computed from the fixture's bracket-to-bracket Z distance — the
  `dist` in that formula — by that formula), each entry's
  role is `SupportBody` **on synthesized stack planes only** (genuine interface entries
  survive with their interface roles; body replacement removes only non-interface rows
  strictly inside each bracket pair), and every
  synthesized plane's `anchor_layer_index` is the layer whose Z is nearest by absolute
  distance with the lower index winning ties. **Bracket selection is the measured Q1 rule,
  not a plane count:** for each `(object_id, region_id)` contiguous run, collect the exact
  distinct interface-role planes and use them as bracket planes only when consecutive
  interface bracket planes enclose a **body-bearing interface span** — at least one
  strictly interior surviving support-bearing row with no interface role; process those
  interface brackets. If no such span exists (including zero/one interface plane or an
  adjacent interface cluster confined to a run boundary), the run's first and last
  surviving support-bearing rows are the fallback brackets. All genuine interface entries
  are protected real entries — never removed or demoted — under either bracket source.** |
  `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- coarse_pitch_produces_free_floating_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3 (traditional planner: same, with `support_step` neutralized).** **Given** the same
  inputs, **when** the traditional planner runs, **then** at least one emitted
  `SupportPlanEntry.anchor_z` is off-grid by more than `COORDINATE_TOLERANCE_UNITS`, and the
  `support_step` decimation (`modules/core-modules/traditional-support-planner/src/lib.rs`)
  is neutralized for the coarse direction — support rows come from the pitch-spaced stack,
  not the every-Nth-layer grid subset: entries inside the computed coarse bracket-range
  membership bypass the `support_step` modulo/removal gate, while rows outside the coarse
  ranges retain the computed `support_step` decimation (no global bypass). The stack follows the **traditional-family** canonical
  stepping of `raft_and_intermediate_support_layers` (`Support/SupportMaterial.cpp`):
  `n_layers_extra = ceil((dist - EPSILON) / max_support_layer_height)`, `step = dist /
  n_layers_extra`, planes at `below_z + k * step` with the last aligned to the upper
  bracket. The test asserts the **exact expected bracket planes** from that formula in the
  planner's original output order, that entries are nondecreasing in `anchor_z` within each
  object with strictly increasing distinct planes, that every **synthesized** stack-plane
  entry
  carries the `SupportBody` role (the nearest surviving at-or-below source row's other fields
  cloned, with non-interface geometry preferred per `body_ids` membership and otherwise-
  unmatched memberships retained, roles rewritten — the Q2 decision), that genuine interface entries survive with their interface
  roles (body replacement removes only non-interface rows strictly inside each bracket
  pair; the Q1 bracket rule is the measured body-bearing-span rule, not a plane count —
  interface planes bracket only when they enclose a body-bearing interface span, otherwise
  the run's surviving support-bearing endpoints are the fallback brackets, and interface
  entries stay protected real entries either way —
  asserted where the fixture naturally expresses the adjacent-cluster fallback case), and
  that each
  synthesized plane's `anchor_layer_index` is the
  true-nearest layer by absolute Z distance (lower index on ties). |
  `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- coarse_pitch_produces_free_floating_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4 (measure-first coarse `height_delta` verdict).** **Given** the Step 5 measurement
   recorded under `TASK-527` in `docs/07_implementation_status.md` — the height term
   `DefaultGCodeEmitter::emit_gcode` actually applies to a coarse 0.45-pitch off-grid pass,
  that pass's declared plane delta (its own Z minus the previous extrusion Z), and the
  resulting E — **when** the verdict test runs, **then** it asserts exactly the recorded
  branch and names it in its own assertion message: `MISSCALE_FIXED` (applied height term
  differed from the declared plane delta by more than `1e-6` absolute) asserts
  `e == distance * point.width * declared_plane_delta * point.flow_factor / filament_area`
  within `1e-6`; `CONSISTENT` asserts the current per-row formula equal within `1e-6` **on
  the recorded applied-height constants** (the height term the emitter actually applied,
  not a re-derivation) and the declared plane delta as measured, and asserts no emitter
  behaviour changed. The verdict must already be recorded before the test is authored. |
  `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd -- coarse_pass_height_delta_matches_recorded_verdict --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5 (guest artifacts fresh before any slice-level evidence).** **Given** this packet
  edits `modules/core-modules/*/src/**`, **when** any slice-level evidence run is produced
  (AC-1, AC-N1, the human-gate artifacts, the visual-debug bundle), **then** `cargo xtask
  build-guests --check` exited `0` immediately beforehand — decided by exit code only (`0`
  fresh, `1` stale, `3` `wasm-tools` infrastructure error), never by grepping for `STALE:`. |
  `cargo xtask build-guests --check && echo FRESH`

Every AC names exact fields, symbols, values, or output fragments and ends with its own
runnable command. Each command names one test with `--exact`, tees to
`target/test-output.log`, and asserts a non-zero matched count so a zero-match run can never
read green. Feature-gated blindness (`CLAUDE.md` §"Feature-gated test files report green
when they don't compile") does not apply to this packet's suite: verified at authoring time
that `crates/slicer-gcode/Cargo.toml` has no `[features]` and no `[[test]]`/`required-features`
entries, and that none of `tree-support-planner`, `traditional-support-planner`,
`tree-support`, `traditional-support` declares a `[features]` section. No AC command targets
`slicer-core`. Re-confirm before relying on this.

**Test-naming convention for the `mod`-aggregated binaries (binding, not advisory).** The
`slicer-runtime` integration-test binary is a `mod` aggregator, so libtest names a test after
the path from the binary root. This packet uses the **wrapper convention (option b)** that
`crates/slicer-runtime/tests/integration/support_family_closure.rs` already uses for the
large majority of its checks — the check is a `pub fn` returning `Result<(), String>` in
`support_family_closure.rs`, and a `#[test]` wrapper declared in
`crates/slicer-runtime/tests/integration/main.rs` calls it and unwraps. That wrapper sits at
the binary root, so its libtest name is **bare**, which is why AC-1 and AC-N1 filters carry
no module prefix. New checks added by this packet MUST follow the wrapper convention, and
`crates/slicer-runtime/tests/integration/main.rs` is therefore an edit site wherever such a
check is added. Do **not** add a bare `#[test]` inside `support_family_closure.rs` for
AC-1/AC-N1: that would make the two filters match zero tests, and the non-zero matched-count
guard on each command is what would catch it.

## Negative Test Cases

- **AC-N1 (disabled reproduces the pre-change Z sequence exactly).** **Given**
   `independent_support_layer_height = false` with `support_layer_height_mm = 0.45` on the
  same fixture and config, **when** a real slice runs, **then** the emitted sequence of
  distinct `;Z:` values is element-wise identical in length and value to the recorded
  pre-239d baseline captured before Step 2 (the `P239D_DISABLED_COARSE_PITCH_BASELINE_Z`
  const in `crates/slicer-runtime/tests/integration/support_family_closure.rs`), and **zero**
  synthesized off-grid rows exist. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- disabled_coarse_pitch_reproduces_baseline_z_sequence --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2 (finer direction unregressed).** **Given** the 239c AC-1 inputs
  (`support_layer_height_mm = 0.1` over `layer_height` 0.2, flag enabled), **when** the 239c
  real-slice test `independent_support_layer_height_emits_support_row_off_object_grid` runs,
  **then** it still passes — the finer direction's off-grid rows (the 273-row behavior) are
  unchanged by the coarse-direction derivation. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- independent_support_layer_height_emits_support_row_off_object_grid --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3 (sentinel 0.0 stays object pitch).** **Given** `support_layer_height_mm = 0.0`
  with the flag enabled, **when** the tree planner runs, **then** no emitted
  `SupportPlanEntry.anchor_z` differs from
  `mm_to_units(layer_plan.layers[entry.anchor_layer_index].z)` by more than
  `COORDINATE_TOLERANCE_UNITS` — the 239c [FWD] option (b) decision ("object pitch → no
  off-grid planes") is preserved by the coarse derivation. |
  `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- zero_pitch_sentinel_stays_object_grid --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N4 (finer-direction adaptive local gap, bracket-local selection).** **NET-NEW
  Step-2 test authored by this packet**, present in the existing `tree_family_tdd` target
  (`modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`) and runnable —
  the test is implemented and passing. No pre-existing test is claimed. **Given** a
  `LayerPlanView` with an `(object_id, region_id)` contiguous run whose bracket pair's
  `local_support_gap` (the maximum positive anchor-Z difference between consecutive
  surviving support-bearing rows of that same run covered by the bracket; these rows are
  already available to both planner callers) exceeds the configured pitch (an adaptive
  local gap) while the
  global pitch >= the
  object's base layer pitch, **when** the tree planner runs, **then** that bracket keeps the
  239c finer derivation (its rows stay at the finer spacing; no pitch-spaced coarse stack is
  synthesized for it), and consecutive surviving-row pairs outside the outer
  brackets retain that same derivation because brackets constrain only genuinely coarse
  replacement — the binding predicate (coarse iff configured nonzero pitch >=
  `local_support_gap`, in exact canonical units with
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` as the only tolerance if needed,
  no new epsilon) is evaluated per bracket pair and never decided from
  the first/contact layer height alone; concretely, configured pitch 0.2 over covered
  surviving-row gaps 0.3 stays finer even if the object's first/base layer gap is 0.2. |
  `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- adaptive_local_gap_stays_finer --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask build-guests --check && echo FRESH` (before every slice-level evidence run)
- Primary targeted proof: AC-1's command (real slice, coarse off-grid extruding rows).

## Human Validation Gate

Blocking. Carried over from `239c-support-layer-height-producer`; this packet changes
behaviourally visible output (the coarse support stack), so the gate is meaningful here.
Evidence standard is **E2 — inspection only**; exact Orca toolpath identity is explicitly
out of scope, and behavioural parity with measured deltas is the bar.

**Precondition — fresh references, HUMAN-generated with `independent_support_layer_height`
ENABLED and the nominal 0.45 mm support pitch.** This packet never generates them:

- `tmp/p239-orca-ref-tree-independent.gcode`
- `tmp/p239-orca-ref-normal-independent.gcode`

Existence gate, recorded verbatim in the gate document as `REFS-PRESENT` or
`REFS-ABSENT-GATE-OPEN`:

```bash
test -f tmp/p239-orca-ref-tree-independent.gcode && test -f tmp/p239-orca-ref-normal-independent.gcode && echo REFS-PRESENT
```

**Verified against the current workspace: both files exist and the command prints
`REFS-PRESENT`.** They are the supplied human-generated nominal references. Note: canonical OrcaSlicer has no
`support_layer_height_mm` key — its support pitch is nozzle-derived
(`max_layer_height_from_nozzle`, `Slicing.cpp`); the human must slice with a support-extruder
nozzle whose `max_layer_height` yields the nominal 0.45 mm pitch represented by the PnP run.

**Packet artifacts.** Regenerate each immediately after `cargo xtask build-guests --check`
returns exit `0` (AC-5); a stale guest silently invalidates every artifact below.

The coarse artifacts use the packet-specific profiles below so the shared 239c matched
profiles retain their finer-direction `0.1` mm setting.

- `tmp/p239d-support-coarse-tree.gcode` —
  `cargo run --bin pnp_cli --release -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/p239d-config-tree-045.json --output tmp/p239d-support-coarse-tree.gcode --module-dir modules/core-modules`
  with `support_layer_height_mm` set to `0.45` in the config
- `tmp/p239d-support-coarse-normal.gcode` — same fixture and invocation with
  `tmp/p239d-config-normal-045.json` and output
  `tmp/p239d-support-coarse-normal.gcode`
- `tmp/vd-p239d/` — a `pnp_cli visual-debug --request <request.json> --output tmp/vd-p239d`
  bundle showing the coarse support stack rows beside ordinary object rows

**Checklist.** Each item is answered in writing with **layer, tap, verdict** in
`tmp/239d-human-validation.md`:

- [x] **Termination** — support reaches the plate or the model beneath its overhangs on both
      families, including on the new coarse stack rows.
- [x] **Coverage** — every demanded overhang region on the fixture carries support at the
      coarse pitch.
- [x] **Collision freedom** — no support intersects model walls at any print row, including
      every newly synthesized off-grid coarse row.
- [x] **Interfaces** — roofs and floors sit carved out of the support body at interface pitch
      on their own rows; the coarse stack does not disturb them.
- [x] **Coarse-stack comparison (the item this packet exists for)** — support rows are spaced
      at the support pitch between the interface planes, free-floating relative to the object
      grid; every placement difference against the fresh references is recorded as a
      **measurement**, not a characterization.
- [x] **Block counts** — `;TYPE:Support` and `;TYPE:Support interface` counts recorded for
      both families and compared against `tmp/p239-orca-ref-tree-independent.gcode` and
      `tmp/p239-orca-ref-normal-independent.gcode`.

Sign-off: **2026-09-02 — APPROVED**. The human gate owner approved all six checklist items;
the packet is implemented.

Note for implementers: `assert_no_test_reads_orca_gcode`
(`crates/slicer-runtime/tests/integration/support_family_closure.rs`) forbids any test from
reading Orca reference G-code. The reference comparison above is **human inspection only** and
must never be encoded as a test.

## Authoritative Docs

- `docs/spec_packets/239c-support-layer-height-producer/packet.spec.md` - the ACs this packet
  extends, the test-naming convention, the gate structure. Direct ranged read.
- `docs/spec_packets/239c-support-layer-height-producer/design.md` - the derivation rules,
  the `[FWD]` sentinel decision, the locked invariants. Direct ranged read.
- `docs/specs/support-independent-layer-z-split-plan.md` - the canonical block and the packet
  queue. Short file; direct ranged read.
- `docs/DEVIATION_LOG.md` - rows `DEV-159`..`DEV-163` only (the seam completion 239d
  inherits). Direct range read around those rows.
- `docs/specs/support-parity-gap-register.md` - row `G-02` (closed by 239c) and the new
  coarse-direction row only. Direct range read around those rows. Never full-read.

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` - `TASK-523`..`TASK-530` registered at packet-owned
  closure (Step 8), plus the Step 1 measurement record (decimation reconciliation + coarse
  baseline) under `TASK-523` and the Step 5 measurement record (height-delta verdict) under
  `TASK-527` -
  `rg -q 'TASK-523' docs/07_implementation_status.md && rg -q 'TASK-530' docs/07_implementation_status.md && rg -q 'TASK-527' docs/07_implementation_status.md`
- `docs/specs/support-independent-layer-z-split-plan.md` - queue row 4 (`239d`) added with
  its task range and dependency on row 3 -
  `rg -q '239d-support-coarse-floating-planes' docs/specs/support-independent-layer-z-split-plan.md`
- `docs/specs/support-parity-gap-register.md` - a new row (next free `G-` id, re-derived at
  Step 8, never quoted from this packet) recording the coarse-direction gap, closed by this
  packet -
  `rg -q '239d-support-coarse-floating-planes' docs/specs/support-parity-gap-register.md`
- **No IR schema version bump.** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`
  (`crates/slicer-ir/src/slice_ir.rs`) **is not bumped by this packet** and must not be
  disturbed; whatever value the live constant and `docs/02_ir_schemas.md` carry at activation
  is the value it keeps. No field is added to `SupportPlanEntry` (live shape: body membership
  is the `body_ids: Vec<String>` field; there is no entry `id` field — duplicate prevention
  keys on live fields only, see `design.md` §Code Change Surface): the existing
  `anchor_z: i64` remains the declared support print plane, and the coarse stack is expressed
  entirely through `anchor_z` values. That decision is what keeps this packet clear of a WIT
  and struct-literal blast radius.
- **No WIT change, no host config key, no manifest change.** The keys
  (`independent_support_layer_height`, `support_layer_height_mm`) are already declared on both
  planner manifests; `docs/15_config_keys_reference.md` is not regenerated.
- `tmp/239d-human-validation.md` - the human-gate document (Step 7).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `raft_and_intermediate_support_layers`: the non-synchronized branch (flag enabled) brackets the sorted `extremes` (top/bottom contact layers) and fills between consecutive ones at `n_layers_extra = ceil((dist - EPSILON) / max_suport_layer_height)`, `step = dist / n_layers_extra`, `print_z = extr1z + i * step`, last layer aligned to `extr2z`; the synchronized branch (flag disabled) snaps to object layers. This is the **AC-3 (traditional-family)** stepping rule. Canonical's `extremes` span many object layers by construction; PnP's measured fixture does not (its interface roles sit in adjacent top-row clusters), so the bracket-**selection** ground truth is the measured Q1 rule (body-bearing interface spans, endpoint fallback) — canonical's `extremes` bracketing motivates it but does not transfer as a plane-count rule. The AC-2 (tree-family) rule is `plan_layer_heights` (`TreeSupport.cpp`) with no EPSILON bias.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`: the grouping predicate (`print_z <= first.print_z + EPSILON`), the midpoint Z rule (`zavg = 0.5 * (first + last)`), and the group-height rule (minimum). PnP reproduces the grouping predicate and midpoint only; the group-height rule is representation-inapplicable (`SupportPlanEntry` has no height field; effective row height derives from adjacent `anchor_z`). This is the grouping/midpoint step the coarse stack applies after stepping.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `max_suport_layer_height = max_layer_height` (nozzle-derived via `max_layer_height_from_nozzle`, clamped >= object layer height). Confirms canonical has no `support_layer_height_mm` key; PnP's key is the pitch knob this packet uses.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — the parallel support-layer
  generation loop in `plan_layer_heights` (`n_layers_extra = ceil(dist / max_layer_height)`,
  `step = dist / n_layers_extra`, `print_z = z1 + step`): the tree-family stepping —
  **no** EPSILON bias, unlike `raft_and_intermediate_support_layers`. This is the AC-2
  stepping rule; the AC-3 stepping rule comes from `raft_and_intermediate_support_layers`
  with its EPSILON bias. One shared formula for both families would be wrong.

Citation policy (E7): canonical behaviour is cited by file + function only, never line number,
and only what a delegated dispatch actually returned. The paragraphs above record what the
delegated dispatches returned on 2026-08-31; re-verify by dispatch before implementing Steps 2
and 3 rather than treating them as a substitute for inspection.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
