# Requirements: 239c-support-layer-height-producer

## Packet Metadata

- Grouped task IDs: `TASK-515`, `TASK-516`, `TASK-517`, `TASK-518`, `TASK-519`, `TASK-520`,
  `TASK-521`, `TASK-522`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)
- Supersedes: `239-support-independent-layer-z`
- Depends on: `239a-anchored-host-seams` **and** `239b-anchored-wit-contract` (both required)

## Problem Statement

PnP has no support-layer Z independent of object-layer Z. Gap-register row `G-02` has named
this since the support-family audit, and the superseded packet `239-support-independent-layer-z`
tried to close it host-side only. A `/swarm` run on 2026-08-28 measured 239's central premise
as false and split it into three packets
(`docs/specs/support-independent-layer-z-split-plan.md`). Two of its nine findings are this
packet's problem statement:

- **F8 — support Z is structurally grid-bound.** `modules/core-modules/tree-support/src/lib.rs`
  and `modules/core-modules/traditional-support/src/lib.rs` both emit via `let z = region.z()`
  inside `run_support`. `modules/core-modules/tree-support-planner` reads
  `layer_plan.layers[layer_rev].z` in `SupportPlanner::plan_for_object` and takes heights from
  `layer.effective_layer_height` / `layer_plan.layers[0].effective_layer_height` as
  `nominal_layer_height`. `LayerPlanView` is the single Z authority and **no module has any
  concept of a support-specific layer height**.
- **The consequence for 239a and 239b.** Both dependency packets state honestly that no real
  slice exercises their paths, because nothing constructs anchored work. Their closure rests on
  hand-built plans and purpose-built test guests. Without a producer, slicing
  `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` yields structurally
  identical output to today's — vacuous evidence (E1).

The exact gap the superseded 239 missed: it put the module surfaces out of bounds, so it could
never make a real slice behave differently, and its AC-1 was unprovable. This packet moves the
boundary to the modules, where the Z authority actually lives.

Two further facts shape the slice. First, `independent_support_layer_height` does not exist
anywhere in the tree (verified: zero matches across `*.rs`, `*.toml`, `*.md`). Second, a
neighbouring key **does** exist and is not the same thing: `support_layer_height_mm` is
declared on both `*-support-planner` manifests and typed in
`declare_resolved_config!` (`crates/slicer-ir/src/resolved_config.rs`), with `0.0` meaning
"use the object's effective layer height". It **decimates the object grid** — a 0.4 mm support
height over 0.2 mm model layers emits support at object layers `{1,3,5}` — so it selects a
*subset* of grid planes and never produces an off-grid Z. `build_emit_schedule`
(`crates/slicer-core/src/algos/support_geometry.rs`) is that decimation. The new key is the
gate that lets the planner leave the grid entirely; the existing key remains the height value.

## In Scope

- **Key declaration.** `[config.schema.independent_support_layer_height]` with `type = "bool"`
  and `default = true` (canonical `PrintConfig.cpp` `init_fff_params`, `coBool`, default true)
  added to `modules/core-modules/tree-support-planner/tree-support-planner.toml` and
  `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`. The key
  string is snake_case in every Rust read site, per the repo-enforced convention.
- **Declared-read proof.** A red-first negative test that the plan-build declared-read guard
  (`ExecutionPlanError::UndeclaredConfigKey` in
  `crates/slicer-scheduler/src/execution_plan.rs`) rejects the key when a module has not
  declared it, plus a positive test that both shipped planner manifests bind it through
  `bind_module_config_view`.
- **Planner Z derivation.** `SupportPlanEntry.anchor_z` becomes the declared support print
  plane instead of a copy of the object layer Z. Enabled: free-floating planes derived from the
  support/interface pitch, following canonical `bottom_contact_layer` (enabled branch) plus the
  `generate_support_layers` stepping rule
  (`n_layers_extra = ceil((dist - EPSILON) / max_support_layer_height)`, `step = dist / n_layers_extra`,
  `print_z = bottom_z + k * step`) and its group midpoint rule
  (`zavg = 0.5 * (first.print_z + last.print_z)`, height = group minimum). Disabled: exact copy
  of the object layer's Z, matching `sync_gap_with_object_layer`.
  `anchor_layer_index` continues to name the nearest object layer and is not repurposed.
- **Renderer emission.** `TreeSupport::run_support` and `TraditionalSupport::run_support` read
  `entry.anchor_z` from `PaintRegionLayerView::support_plan_entries_for` — which they already
  call — and emit at that plane instead of `region.z()`. Off-grid entries leave the module as an
  anchored collection through 239b's SDK drain; on-grid entries keep the existing
  `SupportOutputBuilder::push_support_path` route unchanged.
- **Measure-first flow protocol.** The full protocol below, including the recorded verdict and
  the conditional emitter change it authorizes or forbids.
- **Human Validation Gate.** Owned entirely here; see `packet.spec.md` §Human Validation Gate.
- **Closure.** `docs/07_implementation_status.md` registration, `docs/15_config_keys_reference.md`
  regeneration, gap-register `G-02` closure, split-plan queue row 3 update.

## Out of Scope

- `PipelineConfig.anchored_entities`, the committed-anchored executor switch at all three call
  sites, `CommittedLayerEvent::Anchored` row synthesis, and the payload-capturing `GCodeEmitter`
  test fixture — all `239a-anchored-host-seams`. Consumed here, never re-specified.
- WIT package `slicer:layer-anchored-events@1.0.0`, the `set-anchored-event-collection` method
  on `layer-collection-builder`, host lift/lower glue, the `dispatch.rs` and
  `marshal/native.rs` producer arms, and the SDK drain — all `239b-anchored-wit-contract`.
  Consumed here, never re-specified.
- Any change to `GCodeEmitter::emit_gcode`'s signature. It has many impls and many call sites
  across the test crates, with exactly one production impl (`DefaultGCodeEmitter`) and one
  production call site (`slicer_runtime::postpass::execute_postpass_with_capture`). No totals are
  quoted here on purpose — they are mutable shared state and have already rotted; re-derive before
  relying on a count (`rg -n 'impl GCodeEmitter for' crates/`,
  `rg -c '\.emit_gcode\(' crates/ -g '*.rs'`). 239a already established that off-grid rows arrive
  as ordinary `LayerCollectionIR`, so the signature stays fixed.
- Any bump to `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (`crates/slicer-ir/src/slice_ir.rs`).
  It **is not bumped by this packet**; leave it at whatever the live constant carries. No literal
  is quoted here deliberately — the version is mutable shared state, so re-derive it from the
  constant at the moment you need it rather than trusting a packet-frozen number.
- Any new field on `SupportPlanEntry` or any WIT change. See `design.md` §Code Change Surface
  for the rejected alternative and why the existing `anchor_z` suffices.
- Raft geometry (240), the AGG rasterizer (241), final Orca closure (242), planner geometric
  fidelity (238b), renderer flow/interface fidelity (238c).
- Generating the Orca reference G-code. The `tmp/p239-orca-ref-*-independent.gcode` files are
  HUMAN-generated preconditions; this packet gates on their existence and never produces them.

## Measure-First Flow Protocol (a gate, not a premise)

Carried over verbatim in force from the superseded 239. Skipping the measurement is the
falsifying exit for this packet.

1. **Measure.** Construct the minimal off-grid case through the **real** emitter
   (`DefaultGCodeEmitter::emit_gcode`, `crates/slicer-gcode/src/emit.rs`) and record **three
   numbers**: (a) the height term the code path actually applies to the off-grid pass,
   (b) that pass's declared plane delta — its own Z minus the previous extrusion Z, and
   (c) the resulting E.
2. **Verdict rule.** `MISSCALE_FIXED` is required **iff** the applied height term differs from
   the declared plane delta by more than `1e-6` absolute. Otherwise the verdict is
   `CONSISTENT`.
3. **Record before deciding.** Write the verdict **and** all three numbers into
   `docs/07_implementation_status.md` under `TASK-519` **before** any fix/no-fix decision is
   made or any line of `emit.rs` is edited.
4. **Conditional fix.** On `MISSCALE_FIXED`: carry per-entity plane-Z context so an off-grid
   pass uses its declared plane delta, keeping grid passes **bit-identical**. On `CONSISTENT`:
   make **no** emitter change and lock current behaviour with the verdict test. Either way the
   verdict test exists and names the recorded branch in its assertion message.
5. **Blast radius on the fix branch only.** Before editing `emit.rs`, enumerate via a
   `LOCATIONS` dispatch every test that hard-asserts emitted E values, and **never** widen a
   tolerance to make one pass. Authoring-time inventory, to be re-derived rather than trusted:
   `emit_e_uses_volumetric_flow_formula` (`crates/slicer-gcode/tests/gcode_emit_tdd.rs`) and
   `first_layer_volumetric_e_uses_configured_first_layer_height`
   (`crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`) are the two strongest binders;
   `purge_volume_within_tolerance` (`crates/slicer-gcode/tests/gcode_toolchange_wrapping.rs`),
   `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`, and
   `crates/slicer-runtime/tests/e2e/wave_overhang_bridge_fill_e2e_tdd.rs` are volume/flow
   derived and next most fragile.
6. **Additional recorded observation O-1 (does not alter the verdict rule).** The measurement
   must **also** record the same three numbers for the object pass immediately following the
   off-grid pass. Grounding: in `DefaultGCodeEmitter::emit_gcode` the height term is derived
   from the gap to the previous **emitted** row, and move-less layers are skipped before the
   delta is computed, so inserting a row changes the term applied to its successor. Whether
   that is a mis-scale is **unmeasured**; O-1 exists so the measurement cannot miss it. The
   `MISSCALE_FIXED`/`CONSISTENT` decision remains governed solely by rule 2 applied to the
   off-grid pass.

No flow figure is stated anywhere in this packet, because none has been measured.

## Authoritative Docs

- `docs/specs/support-independent-layer-z-split-plan.md` - short (about 150 lines); direct
  ranged read. Findings F1–F9 and the canonical reference block.
- `docs/specs/support-parity-gap-register.md` - long; **ranged read around row `G-02` only**,
  never full-read. `G-02`'s "Unverified risk" sentence about `height_delta` is the origin of
  the measure-first protocol above.
- `docs/specs/support-families-anchored-entities-plan.md` - long; bounded ranged reads of §6
  invariants, §7 evidence standards (E1, E2, E4), §8 human gate, §13 trap T11 only.
- `docs/08_coordinate_system.md` - consulted through the coord-system constraint in
  `design.md`; do not full-read.
- `docs/02_ir_schemas.md` - long; ranged read of the `LayerCollectionIR` and `SupportPlanIR`
  sections only, and only to confirm that nothing needs to change.
- `docs/15_config_keys_reference.md` - **generated**; read only to verify the Step 8 grep.
  Never hand-edit; regenerate with `cargo xtask gen-config-docs`.
- `CLAUDE.md` §"Guest WASM Staleness" and §"Config Key Naming Convention" - apply verbatim to
  every step.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `init_fff_params`: the
  `independent_support_layer_height` declaration, `coBool`, default **true**. Ground truth for
  the manifest `type`/`default` asserted by AC-6.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` —
  `PrintObjectSupportMaterial::bottom_contact_layer`: enabled → free-floating `print_z` from the
  interface flow height; disabled → `sync_gap_with_object_layer`, copying the upper layer's
  `print_z`/`height`. The AC-2/AC-3 semantic pair.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `gap_raft_object` / `gap_object_support` /
  `gap_support_object` rounded to multiples of the object `layer_height` **only when the flag is
  FALSE**.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`:
  does not reference the flag; groups by `print_z <= first.print_z + EPSILON`, group Z is the
  midpoint `zavg = 0.5 * (first.print_z + last.print_z)`, group height is the minimum;
  intermediate rows step by
  `n_layers_extra = ceil((dist - EPSILON) / max_suport_layer_height)`, `step = dist / n_layers_extra`,
  `print_z = bottom_z + step`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_extrude`: reads the precomputed
  `path.mm3_per_mm`, never recomputes geometry.
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::mm3_per_mm` =
  `m_height * (m_width - m_height * (1 - PI/4))`, or `w^2 * PI/4` when bridging; the height term
  is baked per extrusion entity, and supports use `support_material_flow(object, layer_height)`
  with the support layer's own height. Comparison target for the AC-5 verdict; it does **not**
  pre-decide the verdict.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

Applied to `independent_support_layer_height`: the canonical consumer is
`PrintObjectSupportMaterial::bottom_contact_layer` (`Support/SupportMaterial.cpp`), with the
grid-snapping side effect in `Slicing.cpp`; the declaration is `PrintConfig.cpp`
`init_fff_params`. It is **not** a plumbing key — it changes emitted geometry — so AC-2, AC-3,
and AC-N1 are behavioural invariant tests, not default-propagation tests. Should any part of
the enabled-branch `print_z` derivation prove unverifiable against the canonical source, raise
it with the human **before** filing anything, then file a `docs/DEVIATION_LOG.md` row only with
their sign-off. Re-derive the next free `DEV-###` at edit time
(`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1` gives the high-water mark;
take the next one). **No high-water value is quoted here on purpose** — it is mutable shared
state, it has already moved more than once during this packet's authoring, and a frozen value is
how two packets end up filing the same row.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7` (seven). Measurable refinements not already in their
  Given/When/Then text:
  - `AC-1` is the criterion the superseded 239 could not honestly write. Its home is
    `crates/slicer-runtime/tests/integration/support_family_closure.rs`, verified at authoring
    time to have a working real-slice driver: `run_slice_for_family` →
    `slicer_runtime::run::run_slice` over the tracked fixture and `orca-matched-config.json`,
    already asserting `;TYPE:Support` in `final_gcode_roles`. Test binary: `integration`.
  - `AC-2`/`AC-3` compare `anchor_z` as `i64` canonical units. `AC-3` uses **integer equality**
    with no tolerance; `AC-2` uses a strict `>` against
    `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` (10 units = 1e-3 mm).
  - `AC-5` may only be authored after the `TASK-519` record exists in
    `docs/07_implementation_status.md`; the test's assertion message must contain the literal
    recorded branch name (`MISSCALE_FIXED` or `CONSISTENT`).
  - `AC-6` asserts the manifest `type`/`default` pair and binding through
    `bind_module_config_view`, not merely that the string appears in a TOML file.
- Negative: `AC-N1`, `AC-N2`, `AC-N3` (three).
  - `AC-N1` requires a baseline `;Z:` sequence captured **before** Step 2 edits any planner and
    stored as a test fixture constant, not re-derived after the change.
- Cross-packet impact: 239a and 239b both become non-vacuous once this packet lands — their
  paths gain a real production producer. Neither packet's contract changes. `242-support-family-orca-closure`
  is unblocked. Gap-register row `G-02` is closed here.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure gates.

**Filter shapes follow `packet.spec.md` §"Test-naming convention for the `mod`-aggregated
binaries".** In short: `--test executor` and `--test contract` filters are module-prefixed because
those tests carry `#[test]` in their own module file; `--test integration` filters (AC-1, AC-N1,
AC-N2) are **bare** because this packet uses the wrapper convention — the check is a `pub fn` in
`crates/slicer-runtime/tests/integration/support_family_closure.rs` and a `#[test]` wrapper in
`crates/slicer-runtime/tests/integration/main.rs` calls it, putting the libtest name at the binary
root. Every row below keeps `--exact` and a non-zero matched-count guard, so a filter that matches
nothing fails rather than reading green.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check && echo FRESH` | AC-7; MUST precede every slice-level evidence run. Exit 0 fresh / 1 stale / 3 `wasm-tools` infra error. Never grep for `STALE:`. | FACT exit code |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- independent_support_layer_height_emits_support_row_off_object_grid --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1, the real-slice proof | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- enabled_independent_height_produces_free_floating_anchor_z --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2, canonical enabled semantics | FACT pass/fail |
| `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- disabled_independent_height_copies_object_layer_print_z_exactly --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3, canonical disabled semantics | FACT pass/fail |
| `mkdir -p target && cargo test -p tree-support --test tree_family_tdd -- offgrid_plan_entry_renders_at_declared_anchor_z --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4, renderer emits at declared plane | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd -- offgrid_pass_height_delta_matches_recorded_verdict --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5, measure-first verdict | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor -- support_config_surface_tdd::independent_support_layer_height_is_declared_and_bound_on_both_planners --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-6, key declared and bound | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- disabled_independent_support_layer_height_reproduces_baseline_z_sequence --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1, disabled equals baseline | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- support_disabled_emits_no_support_rows_even_with_independent_height --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2, support-disabled silence | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- config_view_binding_tdd::undeclared_independent_support_layer_height_fails_plan_build --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3, declared-read guard | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 \| tee target/test-output.log && grep -q '^test result: ok' target/test-output.log` | Emitter blast radius on the `MISSCALE_FIXED` branch only | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd 2>&1 \| tee target/test-output.log && grep -q '^test result: ok' target/test-output.log` | Emitter blast radius on the `MISSCALE_FIXED` branch only | FACT pass/fail |
| `cargo check --workspace --all-targets` | Type gate across all targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate, required before commit | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate, required before commit | FACT pass/fail |
| `cargo xtask gen-config-docs` then `rg -q 'independent_support_layer_height' docs/15_config_keys_reference.md` | Step 8 doc regeneration + its grep | FACT pass/fail |
| `test -f tmp/p239-orca-ref-tree-independent.gcode && test -f tmp/p239-orca-ref-normal-independent.gcode && echo REFS-PRESENT` | Human-gate reference existence gate; record `REFS-PRESENT` or `REFS-ABSENT-GATE-OPEN` | FACT literal |
| `cargo xtask test --summary --workspace --no-fail-fast` | Acceptance ceremony only, per `CLAUDE.md` §Test Discipline. Dispatch to a sub-agent for a FACT pass/fail; never absorb the output. | FACT pass/fail |

Commands must have small, parseable output suitable for delegation. **Never run
`cargo test --workspace` directly** — the gated entry point is `cargo xtask test`, and only at
the acceptance ceremony.

## Step Completion Expectations

- **Baseline before behaviour.** The AC-N1 baseline `;Z:` sequence is captured in Step 1,
  before Step 2 changes any planner. Capturing it afterwards makes AC-N1 tautological.
- **Measurement before source.** Step 5 records the verdict and its three numbers in
  `docs/07_implementation_status.md`. Step 6 may not open `crates/slicer-gcode/src/emit.rs`
  for editing until that record exists. This ordering is the packet's falsifiability contract.
- **Freshness before evidence.** Every step that produces slice-level evidence (Steps 3, 4, 7
  and the AC-1/AC-N1/AC-N2 runs) begins with `cargo xtask build-guests --check` returning
  exit `0`. This packet edits `modules/core-modules/*/src/**` and `modules/core-modules/*/*.toml`,
  so a stale guest will present as an unrelated-looking failure.
- **Ledger facts are re-derived, not quoted.** Before Step 8 appends to
  `docs/07_implementation_status.md`, re-derive the tail of that file, the next free `G-` row
  in `docs/specs/support-parity-gap-register.md`, and the next free `DEV-###` in
  `docs/DEVIATION_LOG.md`. **No values are quoted here on purpose.** The task high-water mark and
  the `DEV-###` high-water mark are both mutable shared state and both moved during this packet's
  authoring; derive each from its file at the moment you append
  (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, then take the next).
  **The next-free `G-` row is additionally CONTESTED** — reviewers have reported the highest
  existing row inconsistently, so no `G-` figure in this packet is settled and none is quoted.
  Derive it from
  `docs/specs/support-parity-gap-register.md` at the moment you append, never from this packet.
- **Shared scratch state.** Steps 3, 4, and 7 all write under `tmp/`. Use the `p239c-` prefix
  for every artifact this packet creates, so nothing collides with the human-owned
  `tmp/p239-orca-ref-*` references or with 237/238 artifacts already present.
- **Both matched configs are edited, not replaced.** `tmp/support-family-config-tree-matched.json`
  and `tmp/support-family-config-normal-matched.json` are tracked and shared with earlier
  packets' measurement notes; add the new key rather than rewriting the files.

## Context Discipline Notes

- `docs/specs/support-parity-gap-register.md` is long and its rows are single very long table
  lines. Read only the range around row `G-02`; a full read will consume a large slice of the
  budget for one sentence.
- `modules/core-modules/tree-support-planner/src/lib.rs` is very large and
  `SupportPlanner::plan_for_object` is a single very long function. Never full-read it. Locate
  the `layer_plan.layers[...].z` and `effective_layer_height` read sites by symbol-scoped grep,
  then read bounded ranges around them.
- `crates/slicer-gcode/src/emit.rs` is long and `DefaultGCodeEmitter::emit_gcode` is a single
  long function containing both the `height_delta` derivation and the volumetric-E application
  inline (there is no helper function to read in isolation). Read only the two bounded ranges
  the Step 5 dispatch returns.
- The emitter blast-radius enumeration (protocol rule 5) must come back as `LOCATIONS`
  (file + test-fn name, at most 25 entries), never as snippets.
- The measurement in Step 5 must be returned as a `FACT`: three numbers plus the verdict word.
  Do not let the dispatch return emitter source.
- `OrcaSlicerDocumented/` is never loaded directly; every canonical read is delegated per the
  orca-delegation snippet above.
- Do not read `docs/spec_packets/239a-anchored-host-seams/design.md` or
  `.../239b-anchored-wit-contract/design.md`. Their `packet.spec.md` files are the contract
  surface this packet consumes.
