---
status: draft
packet: 239c-support-layer-height-producer
supersedes: 239-support-independent-layer-z
depends_on: 239a-anchored-host-seams, 239b-anchored-wit-contract
task_ids:
  - TASK-515
  - TASK-516
  - TASK-517
  - TASK-518
  - TASK-519
  - TASK-520
  - TASK-521
  - TASK-522
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 239c-support-layer-height-producer

## Goal

Declare `independent_support_layer_height`, decouple support-layer Z from the object layer
grid inside `tree-support-planner` / `traditional-support-planner` and their two renderers,
emit the resulting off-grid support work through the anchored path established by
`239a-anchored-host-seams` and `239b-anchored-wit-contract`, and settle the measure-first
`height_delta` flow verdict — so a real slice of `SupportTest.stl` produces support print
rows at Z values the object layer plan does not contain.

## Scope Boundaries

Module-side production of off-grid support Z plus the one measured emitter decision. In
scope: the `[config.schema]` declaration of `independent_support_layer_height` on both
`*-support-planner` manifests, the `anchor_z` derivation inside both planners, the anchored
emission path inside both renderers, and the conditional `crates/slicer-gcode/src/emit.rs`
change that the Step 5 measurement authorizes or forbids. Out of scope and owned elsewhere:
the host anchored-entity input seam, the `execute_per_layer*` call-site switch, and
`CommittedLayerEvent::Anchored` row synthesis (`239a-anchored-host-seams`); the WIT package,
lift/lower glue, dispatch producer arms, and SDK drain (`239b-anchored-wit-contract`); raft
geometry (240); the AGG rasterizer (241); final Orca closure (242).

## Prerequisites and Blockers

- Depends on: **both** `239a-anchored-host-seams` and `239b-anchored-wit-contract`. 239a
  supplies `PipelineConfig.anchored_entities`, the committed-anchored executor switch at all
  three call sites, the `CommittedLayerEvent::Anchored` → `LayerCollectionIR` row-synthesis
  function using the canonical `|dz| <= EPSILON` merge, and a payload-capturing
  `GCodeEmitter` test fixture. 239b supplies WIT package
  `slicer:layer-anchored-events@1.0.0` (interface `anchored-events`, world
  `anchored-events-module`), the `set-anchored-event-collection` method on the
  `layer-collection-builder` resource, host lift/lower glue, the `dispatch.rs` +
  `marshal/native.rs` producer arms constructing `LayerStageCommit::AnchoredEvents`, the
  SDK drain glue, **and — decisively for this packet — the widened two-builder `run` on the
  `layer-support` world.** 239b adds `collection: layer-collection-builder` to `run` in
  `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` (after
  `output: support-output-builder`), mirroring
  `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit`'s existing
  `output: gcode-output-builder, collection: layer-collection-builder`, and correspondingly gives
  `LayerModule::run_support` (`crates/slicer-sdk/src/traits.rs`) a
  `&mut LayerCollectionBuilder`. **This packet consumes that two-builder signature** — its
  renderers reach `set_anchored_event_collection` through the `collection` parameter their own
  `run_support` now receives — and re-specifies none of it.
- Unblocks: `242-support-family-orca-closure`.
- Activation blockers: both dependencies must reach `status: implemented` first — that is the
  only remaining activation gate. The former `[BLOCK]`-tagged interface question in `design.md`
  §Open Questions (whether 239b's drain is reachable from a `Layer::Support` guest context) is
  **resolved**: it is reachable, via the two-builder `run` above, and 239b records the same
  decision. The dispatch that ran that question is now a **confirmation** check against 239b's
  landed surface, not an open design choice; the two fallbacks previously documented here (a thin
  `Layer::AnchoredEvents` re-emitter module, and host-side lowering from committed `SupportIR`)
  are withdrawn. `depends_on` remains **both** `239a-anchored-host-seams` and
  `239b-anchored-wit-contract`. Two `[FWD]` questions stay open in `design.md` §Open Questions
  (the `support_layer_height_mm == 0.0` sentinel, and raft prefix layers with negative
  `global_layer_index`); both are implementer-resolvable and neither blocks activation.

This is the packet that makes the anchored substrate non-vacuous. 239a and 239b both state,
honestly, that no real slice exercises their paths until a producer exists. This packet is
that producer, and AC-1 is the acceptance criterion the superseded packet 239 could not
honestly write.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1 (real slice emits an off-grid support row).** **Given** a real
  `slicer_runtime::run::run_slice` of
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` through the tracked
  `orca-matched-config.json` with `independent_support_layer_height = true` and a support
  pitch finer than the object layer height, **when** the resulting G-code is parsed, **then**
  the set of distinct `;Z:` values is a strict superset of the same slice's
  `independent_support_layer_height = false` baseline, and at least one Z present only in the
  enabled run is followed by a `;TYPE:Support` block before the next `;LAYER_CHANGE` — i.e.
  at least one support row prints at a Z the object layer plan does not contain. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- independent_support_layer_height_emits_support_row_off_object_grid --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2 (canonical enabled semantics: free-floating `anchor_z`).** **Given** a
  `LayerPlanView` whose layers sit at 0.2 mm pitch and a support pitch demanding a finer
  contact plane, **when** `SupportPlanner::plan_for_object`
  (`modules/core-modules/tree-support-planner/src/lib.rs`) runs with the key enabled,
  **then** at least one emitted `SupportPlanEntry.anchor_z` differs from
  `mm_to_units(layer_plan.layers[entry.anchor_layer_index].z)` by more than
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` (10 units = 1e-3 mm), every
  `anchor_z` is strictly increasing within an object, and the intermediate planes follow the
  canonical stepping of `generate_support_layers` (`Support/SupportCommon.cpp`):
  `n_layers_extra = ceil((dist - EPSILON) / max_support_layer_height)`,
  `step = dist / n_layers_extra`, `print_z = bottom_z + k * step`. |
  `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- enabled_independent_height_produces_free_floating_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3 (canonical disabled semantics: gap synced to the object layer).** **Given** the same
  inputs with the key disabled, **when** the planner runs, **then** every emitted
  `SupportPlanEntry.anchor_z` equals `mm_to_units(layer_plan.layers[entry.anchor_layer_index].z)`
  exactly (integer equality on `i64`, no tolerance window), matching canonical
  `PrintObjectSupportMaterial::bottom_contact_layer` (`Support/SupportMaterial.cpp`) calling
  `sync_gap_with_object_layer` and copying the upper layer's `print_z`/`height`. |
  `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- disabled_independent_height_copies_object_layer_print_z_exactly --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4 (renderer emits at the declared plane, not `region.z()`).** **Given** a
  `SupportPlanEntry` whose `anchor_z` is off-grid by more than
  `COORDINATE_TOLERANCE_UNITS`, delivered to `TreeSupport::run_support`
  (`modules/core-modules/tree-support/src/lib.rs`) through
  `PaintRegionLayerView::support_plan_entries_for`, **when** the renderer emits, **then**
  every emitted point's Z equals `entry.anchor_z` (in canonical units, converted once via
  `mm_to_units`) rather than `region.z()`, and the paths leave the module as an anchored
  collection whose declared plane equals `entry.anchor_z`. |
  `mkdir -p target && cargo test -p tree-support --test tree_family_tdd -- offgrid_plan_entry_renders_at_declared_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5 (measure-first `height_delta` verdict).** **Given** the Step 5 measurement recorded
  under `TASK-519` in `docs/07_implementation_status.md` — the height term
  `DefaultGCodeEmitter::emit_gcode` actually applies to the off-grid pass, that pass's
  declared plane delta (its own Z minus the previous extrusion Z), and the resulting E —
  **when** the verdict test runs, **then** it asserts exactly the recorded branch and names
  it in its own assertion message: `MISSCALE_FIXED` (applied height term differed from the
  declared plane delta by more than `1e-6` absolute) asserts
  `e == distance * point.width * declared_plane_delta * point.flow_factor / filament_area`
  within `1e-6`; `CONSISTENT` asserts the current per-row formula equal within `1e-6` on the
  measured constants and asserts no emitter behaviour changed. The verdict must already be
  recorded before the test is authored. |
  `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd -- offgrid_pass_height_delta_matches_recorded_verdict --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6 (the key is declared and reaches both planners).** **Given** the shipped manifests
  `modules/core-modules/tree-support-planner/tree-support-planner.toml` and
  `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`,
  **when** `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) binds a
  global config containing `independent_support_layer_height = true`, **then** each planner's
  `ConfigView::get_bool("independent_support_layer_height")` returns `Some(true)`, the
  manifest entry declares `type = "bool"` with `default = true` (matching canonical
  `PrintConfig.cpp` `init_fff_params`, `coBool`, default true), and the key string is
  snake_case in every Rust read site. |
  `mkdir -p target && cargo test -p slicer-runtime --test executor -- support_config_surface_tdd::independent_support_layer_height_is_declared_and_bound_on_both_planners --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-7 (guest artifacts fresh before any slice-level evidence).** **Given** this packet
  edits `modules/core-modules/*/src/**` and `modules/core-modules/*/*.toml`, **when** any
  slice-level evidence run is produced (AC-1, the human-gate artifacts, the visual-debug
  bundle), **then** `cargo xtask build-guests --check` exited `0` immediately beforehand —
  decided by exit code only (`0` fresh, `1` stale, `3` `wasm-tools` infrastructure error),
  never by grepping for `STALE:`. |
  `cargo xtask build-guests --check && echo FRESH`

Every AC names exact fields, symbols, values, or output fragments and ends with its own
runnable command. Each command names one test with `--exact`, tees to
`target/test-output.log`, and asserts a non-zero matched count so a zero-match run can never
read green. Feature-gated blindness (`CLAUDE.md` §"Feature-gated test files report green when
they don't compile") does not apply to this packet's suite: verified at authoring time that
`crates/slicer-gcode/Cargo.toml` has no `[features]` and no `[[test]]`/`required-features`
entries, and that none of `tree-support-planner`, `traditional-support-planner`,
`tree-support`, `traditional-support` declares a `[features]` section. No AC command targets
`slicer-core`. Re-confirm before relying on this.

**Test-naming convention for the `mod`-aggregated binaries (binding, not advisory).** All three
`slicer-runtime` integration-test binaries named above are `mod` aggregators, so libtest names a
test after the path from the binary root. Two conventions are therefore in play and this packet
commits to one per binary:

- `--test executor` and `--test contract`: the test carries `#[test]` **in its own module file**,
  so its libtest name is module-prefixed. AC-6 and AC-N3 filters accordingly read
  `support_config_surface_tdd::…` and `config_view_binding_tdd::…`.
- `--test integration`: this packet uses the **wrapper convention (option b)** that
  `crates/slicer-runtime/tests/integration/support_family_closure.rs` already uses for the large
  majority of its checks — the check is a `pub fn` returning `Result<(), String>` in
  `support_family_closure.rs`, and a `#[test]` wrapper declared in
  `crates/slicer-runtime/tests/integration/main.rs` calls it and unwraps. That wrapper sits at the
  binary root, so its libtest name is **bare**, which is why AC-1, AC-N1, and AC-N2 filters carry
  no module prefix. Evidence: `cargo test -p slicer-runtime --test integration -- --list` reports
  exactly one `support_family_closure::`-prefixed name
  (`tree_branch_a_merge_keeps_drawable_nodes_on_merge_layer`, the file's single in-file `#[test]`);
  every other `support_family_closure` check — `fixture_invariants`, `final_gcode_roles`,
  `support_never_intersects_model_at_exact_z`, and the rest — is listed bare. New checks added by
  this packet MUST follow the wrapper convention, and `crates/slicer-runtime/tests/integration/main.rs`
  is therefore an edit site wherever such a check is added. Do **not** add a bare `#[test]` inside
  `support_family_closure.rs` for AC-1/AC-N1/AC-N2: that would make the three filters match zero
  tests, and the non-zero matched-count guard on each command is what would catch it.

## Negative Test Cases

- **AC-N1 (disabled reproduces the pre-change Z sequence exactly).** **Given**
  `independent_support_layer_height = false` on the same fixture and config, **when** a real
  slice runs, **then** the emitted sequence of distinct `;Z:` values is element-wise
  identical in length and value to the recorded pre-change baseline captured before Step 2,
  **zero** synthesized off-grid rows exist, and every `SupportPlanEntry.anchor_z` equals its
  object layer's Z under integer equality. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- disabled_independent_support_layer_height_reproduces_baseline_z_sequence --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2 (support disabled emits nothing).** **Given** `enable_support = false` with
  `independent_support_layer_height = true`, **when** a real slice runs, **then** the G-code
  contains no `;TYPE:Support` and no `;TYPE:Support interface` line, both planners emit zero
  `SupportPlanEntry` values, and no anchored collection is proposed. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- support_disabled_emits_no_support_rows_even_with_independent_height --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3 (undeclared key is rejected, not silently dropped).** **Given** a module binding
  whose `ConfigView` carries `independent_support_layer_height` but whose manifest
  `[config.schema]` does not declare it, **when** the execution plan is built, **then**
  plan construction returns
  `Err(ExecutionPlanError::UndeclaredConfigKey { module_id, key })` with
  `key == "independent_support_layer_height"` — proving the declared-read guard in
  `crates/slicer-scheduler/src/execution_plan.rs` covers this key rather than
  `ConfigView::from_declared` dropping it silently. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- config_view_binding_tdd::undeclared_independent_support_layer_height_fails_plan_build --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask build-guests --check && echo FRESH` (before every slice-level evidence run)
- Primary targeted proof: AC-1's command (real slice, off-grid support row).

## Human Validation Gate

Blocking. Carried over from the superseded `239-support-independent-layer-z`; this packet is
the only place in the split where it is meaningful, because it is the only packet that
produces a behaviourally different slice. Evidence standard is **E2 — inspection only**;
exact Orca toolpath identity is explicitly out of scope, and behavioural parity with
measured deltas is the bar.

**Precondition — fresh references, HUMAN-generated with `independent_support_layer_height`
ENABLED. This packet never generates them:**

- `tmp/p239-orca-ref-tree-independent.gcode`
- `tmp/p239-orca-ref-normal-independent.gcode`

Existence gate, recorded verbatim in the gate document as `REFS-PRESENT` or
`REFS-ABSENT-GATE-OPEN`:

```bash
test -f tmp/p239-orca-ref-tree-independent.gcode && test -f tmp/p239-orca-ref-normal-independent.gcode && echo REFS-PRESENT
```

**Verified at authoring time: neither file exists (`REFS-ABSENT-GATE-OPEN`).** The gate
cannot be signed until a human produces them.

**TRAP T11 (mandatory).** The pre-existing references under `tmp/` were sliced with the
feature **DISABLED**, so they cannot measure this gap at all. The "Orca 205 vs PnP 150
print-Z" figure derived from them is **VOID** and must never be requoted anywhere in this
packet, its handoffs, or its gap-register edits.

**Packet artifacts.** Regenerate each immediately after `cargo xtask build-guests --check`
returns exit `0` (AC-7); a stale guest silently invalidates every artifact below.

- `tmp/p239c-support-indep-tree.gcode` —
  `cargo run --bin pnp_cli --release -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-tree-matched.json --output tmp/p239c-support-indep-tree.gcode --module-dir modules/core-modules`
- `tmp/p239c-support-indep-normal.gcode` — same fixture and invocation with
  `tmp/support-family-config-normal-matched.json` and output
  `tmp/p239c-support-indep-normal.gcode`
- `tmp/vd-p239c/` — a `pnp_cli visual-debug --request <request.json> --output tmp/vd-p239c`
  bundle showing intermediate support print rows beside ordinary object rows

Both matched configs are **VERIFIED present** at authoring time. Both must be edited to set
`independent_support_layer_height = true` for the enabled artifacts; keep a disabled-copy
slice alongside for the AC-N1 comparison.

**Checklist.** Each item is answered in writing with **layer, tap, verdict** in
`tmp/239c-human-validation.md`:

- [ ] **Termination** — support reaches the plate or the model beneath its overhangs on both
      families, including on the new intermediate rows.
- [ ] **Coverage** — every demanded overhang region on the fixture carries support.
- [ ] **Collision freedom** — no support intersects model walls at any print row, including
      every newly synthesized off-grid row.
- [ ] **Interfaces** — roofs and floors sit carved out of the support body at interface pitch
      on their own rows.
- [ ] **Matched-height comparison (the item this packet exists for)** — distinct print-Z rows
      exceed the object-layer count wherever a finer support pitch is demanded; the object and
      support Z sequences interleave monotonically with no duplicate or inverted plane; every
      placement difference against the fresh references is recorded as a **measurement**, not
      a characterization.
- [ ] **Block counts** — `;TYPE:Support` and `;TYPE:Support interface` counts recorded for
      both families and compared against `tmp/p239-orca-ref-tree-independent.gcode` and
      `tmp/p239-orca-ref-normal-independent.gcode`.

Sign-off: `_date_ _verdict_`. The packet may not reach `status: implemented` without a
completed sign-off line.

Note for implementers: `assert_no_test_reads_orca_gcode`
(`crates/slicer-runtime/tests/integration/support_family_closure.rs`) forbids any test from
reading Orca reference G-code. The reference comparison above is **human inspection only** and
must never be encoded as a test.

## Authoritative Docs

- `docs/specs/support-independent-layer-z-split-plan.md` - the plan of record; findings F1–F9
  and the canonical reference block. Short file; direct ranged read.
- `docs/specs/support-parity-gap-register.md` - row `G-02` only; direct range read around
  that row. Never full-read (the file is long).
- `docs/specs/support-families-anchored-entities-plan.md` - §6 invariants, §7 evidence
  standards (E2), §8 human gate, §13 trap T11. Bounded ranged reads only.
- `docs/08_coordinate_system.md` - consulted through the coord-system constraint in
  `design.md`; do not full-read.
- `docs/15_config_keys_reference.md` - regenerated, not hand-edited; see Doc Impact.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` - regenerated by `cargo xtask gen-config-docs` after the
  Step 1 manifest declarations land, so the new module key appears in the module-key
  reference - `rg -q 'independent_support_layer_height' docs/15_config_keys_reference.md`
- `docs/07_implementation_status.md` - `TASK-515`..`TASK-522` registered at packet-owned
  closure (Step 8), plus the Step 5 measurement record (verdict plus the three numbers) filed
  under `TASK-519` -
  `rg -q 'TASK-515' docs/07_implementation_status.md && rg -q 'TASK-522' docs/07_implementation_status.md && rg -q 'TASK-519' docs/07_implementation_status.md`
- `docs/specs/support-parity-gap-register.md` - row `G-02` closed with this packet as its
  destination; the two off-grid blockers it names are retired by 239a and this packet -
  `rg -q '239c-support-layer-height-producer' docs/specs/support-parity-gap-register.md`
- `docs/specs/support-independent-layer-z-split-plan.md` - queue row 3's `status` and
  `packet dir` columns updated -
  `rg -q 'docs/spec_packets/239c-support-layer-height-producer' docs/specs/support-independent-layer-z-split-plan.md`
- **No IR schema version bump.** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`
  (`crates/slicer-ir/src/slice_ir.rs`) **is not bumped by this packet** and must not be disturbed;
  whatever value the live constant and `docs/02_ir_schemas.md` carry at activation is the value it
  keeps. Do not copy a literal from this packet — the version is mutable shared state; re-derive it
  from the constant (`rg -A3 'CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION: SemVer' crates/slicer-ir/src/slice_ir.rs`)
  if you need it, and confirm `docs/02_ir_schemas.md` agrees. Off-grid rows
  arrive as ordinary `LayerCollectionIR` per 239a. No field is added to `SupportPlanEntry`:
  the existing `anchor_z: i64` ("anchor height in canonical units") is the declared support
  print plane, and support layer height is derived from consecutive `anchor_z` deltas rather
  than transported as a new field. That decision is recorded in `design.md` §Code Change
  Surface with its rejected alternative, and it is what keeps this packet clear of a WIT and
  struct-literal blast radius.
- **No host config key.** `independent_support_layer_height` is a **module** key declared in
  `[config.schema]`, not a `declare_resolved_config!` host key, so `docs/config/host-keys.toml`
  and its lock test `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` are
  untouched.
- The `crates/slicer-gcode/src/emit.rs` edit is **conditional on the Step 5 verdict**. On the
  `CONSISTENT` branch there is no source edit and no additional doc impact. On the
  `MISSCALE_FIXED` branch, Step 6 owns its own test-fallout inventory in the same step.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `init_fff_params`: confirm
  `independent_support_layer_height` is `coBool` with default **true**. This is the ground
  truth for the manifest `type`/`default` asserted by AC-6.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` —
  `PrintObjectSupportMaterial::bottom_contact_layer`: the flag's real effect. Enabled →
  `print_z` free-floating, derived from the interface flow height. Disabled → calls
  `sync_gap_with_object_layer` and copies the upper layer's `print_z` and `height`. This is
  the AC-2/AC-3 semantic pair.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — the rounding of `gap_raft_object`,
  `gap_object_support`, and `gap_support_object` to multiples of the object `layer_height`
  **only when the flag is FALSE**. Confirms the disabled branch is the grid-snapping one.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`:
  note that it does **not** reference the flag. It groups already-Z-assigned layers by
  `print_z <= first.print_z + EPSILON`, sets each group's Z to the midpoint
  `zavg = 0.5 * (first.print_z + last.print_z)` and its height to the group minimum;
  intermediate rows come from `n_layers_extra = ceil((dist - EPSILON) / max_suport_layer_height)`,
  `step = dist / n_layers_extra`, `print_z = bottom_z + step`. This is the AC-2 stepping rule.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_extrude`: never recomputes geometry, it
  reads the precomputed `path.mm3_per_mm`. Comparison target for the AC-5 verdict only.
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::mm3_per_mm`:
  `m_height * (m_width - m_height * (1 - PI/4))`, or `w^2 * PI/4` when bridging; the height
  term is baked **per extrusion entity** at generation time, and supports use
  `support_material_flow(object, layer_height)` with the support layer's own height. This is
  the comparison target for the flow verdict. **It does not pre-decide the verdict** — the
  verdict comes only from the Step 5 measurement of this tree's emitter.

Citation policy (E7): canonical behaviour is cited by file + function only, never line number,
and only what a delegated dispatch actually returned. The paragraphs above record what the
plan-of-record's delegated dispatch returned on 2026-08-28; re-verify by dispatch before
implementing Steps 2 and 6 rather than treating them as a substitute for inspection.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
