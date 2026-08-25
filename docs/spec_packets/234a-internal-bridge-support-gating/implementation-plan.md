# Implementation Plan: 234a-internal-bridge-support-gating (closure edition)

## Execution Rules

- Work one atomic step at a time; backlog source for every row is ISSUE-82 closure edition
  (`task_ids` N-A per task-map.md).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract; never write "see Step N".
- After ANY step touching slicer-ir/slicer-core/slicer-sdk/slicer-schema/wit: run
  `cargo xtask build-guests --check` and honor its exit code (design.md snippet rule).

## Steps

### Step 1: RC-A arithmetic fix

- Task IDs: `N-A` (backlog: ISSUE-82 filter half, closure)
- Objective: `unsupported_span_areas` initializes from pooled lower FILLS themselves;
  delete `fill_envelope`; AUTHOR net-new test fn `fills_are_the_initial_unsupported_carrier`
  in the suite and re-bless fixtures to canonical-correct outputs.
- Precondition: clean `cargo check --workspace --all-targets`.
- Postcondition: AC-1 named test exists and passes; old complement semantics absent.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` - full (≈400 lines; ranged reads by symbol)
  - decision brief - sections §0.1/§4
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/bridge_over_infill.rs`
  - `crates/slicer-core/tests/bridge_support_gating_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, IR, runtime, modules, WIT
- Blast-radius discipline: n/a (no struct/schema change).
- Expected sub-agent dispatches:
  - Question: confirm no other caller depends on `fill_envelope`'s envelope output; scope
    `crates/**`; return LOCATIONS ≤10.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/82-parity-closure-decision-brief.md` - §0.1, §4
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; gather init snippets already captured in brief
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: whole suite file green under `--features host-algos`; probe note recorded
  (does a candidate clear the opening?) without any tuning change.

### Step 2a: `internal_solid_fill` + `internal_bridge_areas` on the IR

- Task IDs: `N-A` (backlog: ISSUE-82 dense-interior taxonomy)
- Objective: add both `SlicedRegion` fields with `#[serde(default)]`; keep tree compiling.
- Precondition: Step 1 exit.
- Postcondition: fields exist host-side; all literals satisfy docs/21 gate.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - `SlicedRegion` block only
  - `docs/21_data_defaults_and_fixtures.md` - gate rules section
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-core/src/algos/prepass_slice.rs` (production exhaustive literal)
  - `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` (expected sole test-literal
    home; if the dispatch returns ANY further file, STOP and split this step rather than
    exceed the cap)
- Files explicitly out of bounds:
  - WIT, views, renderer (Step 2b); runtime passes (Step 3)
- Blast-radius discipline (MANDATORY): dispatch LOCATIONS for every `SlicedRegion` struct
  literal + every hard assertion on `internal_bridge_lines` BEFORE editing; cite results
  inline; FRU for test sites, exhaustive for production.
- Expected sub-agent dispatches:
  - Question: list all `SlicedRegion {` literal sites and `internal_bridge_lines`
    assertions; scope `crates/**`; return LOCATIONS ≤20.
- Context cost: `M`
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` - gate section
- OrcaSlicer refs: none this step
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -5` - FACT pass/fail
  - `cargo xtask check-literals` - exit 0
- Exit condition: both verifications clean; guest freshness check run.

### Step 2b: WIT mirror + view + render arms

- Task IDs: `N-A` (backlog: ISSUE-82 dense-interior taxonomy)
- Objective: `internal_solid_fill` mirrored through canonical WIT region type and
  `SliceRegionView::from_ir`; PNG render arms for both new fields.
- Precondition: Step 2a exit.
- Postcondition: AC-2 greps pass; bundles carry both fields automatically.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/views.rs` - `from_ir` only
  - `crates/slicer-runtime/src/visual_debug_render.rs` - `slice_shapes`, `geometry_points_mm`, palette
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/**` (region type file)
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-runtime/src/visual_debug_render.rs`
- Files explicitly out of bounds:
  - guest module sources; manifest TOMLs
- Blast-radius discipline: WIT-change checklist — search `wit_host.rs`/dispatch/guest
  consumers for the touched type; verify identity across the boundary; `cargo build --tests`.
- Expected sub-agent dispatches:
  - Question: which generated/host files reference the region WIT type after the edit; scope
    `crates/slicer-wasm-host/**` + `crates/slicer-sdk/**`; return LOCATIONS ≤15.
- Context cost: `M`
- Authoritative docs: `docs/19_visual_debug.md` - delegated SUMMARY
- OrcaSlicer refs: none this step
- Verification:
  - `cargo build --tests 2>&1 | tail -5` - FACT pass/fail
  - `cargo xtask build-guests --check; echo EXIT=$?` - expect EXIT=0 after rebuild
- Exit condition: AC-2 command string returns success; guests fresh.

### Step 3: Qualification rewrite

- Task IDs: `N-A` (backlog: ISSUE-82 site-selection parity)
- Objective: prepass pass qualifies `internal_solid_fill` against lower `infill_areas` fills
  and per-region solids (incl. density≥0.999 branch via per-region `RegionKey`); persists to
  `internal_bridge_areas` + extends `bridge_areas`; print_z-keyed skip logs.
- Precondition: Step 2b exit; walls still untouched (construction stays put until Step 4).
- Postcondition: AC-3 test green; qualified polygons visible in committed state.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - shell passes +
    `gate_internal_bridge_sites` region only
  - decision brief - §0.2/§0.3, Item 2 decisions
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  - `crates/slicer-runtime/tests/integration/region_partition_tdd.rs` (net-new
    `internal_bridge_qualification_writes_gated_areas`)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/layer_executor.rs` (Step 4); modules
- Blast-radius discipline: n/a beyond Step 2a sweep.
- Expected sub-agent dispatches:
  - Question: confirm `config_for` accepts per-lower-layer `RegionKey`s for every timeline
    entry pattern used here; scope `crates/slicer-runtime/src/**` + `crates/slicer-ir/src/**`
    (`config_for`/`RegionKey` live in `slice_ir.rs`, not slicer-runtime); return FACT ≤5 lines.
- Context cost: `M`
- Authoritative docs: `docs/04_host_scheduler.md` - delegated SUMMARY (commit payloads)
- OrcaSlicer refs: `PrintObject.cpp` density branch - delegate SNIPPETS if needed
- Verification:
  - `cargo test -p slicer-runtime --test integration -- internal_bridge_qualification_writes_gated_areas --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: AC-3 command green; interim calicat probe recorded (presence-in-window),
  no tuning changes.

### Step 4a: Construction-reachability probe

- Task IDs: `N-A` (backlog: ISSUE-82 venue split)
- Objective: prove perimeter-wall paths and Layer::Infill polylines are reachable in the
  InfillPostProcess commit arm.
- Precondition: Step 3 exit.
- Postcondition: named payload/accessor symbols recorded, or STOP-and-report.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - InfillPostProcess arm + stage-payload plumbing
  - `crates/slicer-ir/src/stage_io.rs` - payload types
- Files allowed to edit (at most 3): NONE (read-only step)
- Files explicitly out of bounds: everything writable
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches: design.md dispatch #3 (FACT ≤5 lines).
- Context cost: `S`
- Authoritative docs: `docs/04_host_scheduler.md` - delegated SUMMARY
- OrcaSlicer refs: none
- Verification: dispatch return recorded verbatim in this file's step notes.
- Exit condition: FACT names concrete accessors, OR packet stops for redesign approval.

### Step 4b: Constructor relocation

- Task IDs: `N-A` (backlog: ISSUE-82 venue split)
- Objective: InfillPostProcess arm constructs anchored strips from
  `internal_bridge_areas` + real anchors; `internal_bridge_lines` retired tree-wide.
- Precondition: Step 4a FACT positive.
- Postcondition: emitter-only logic gone; field absent; tree compiles.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - arm + anchor sources
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-ir/src/slice_ir.rs` (field removal)
  - `crates/slicer-core/src/algos/prepass_slice.rs` (literal follow-up)
- Files explicitly out of bounds: contract test rewrite (Step 4c); prepass pass (done Step 3)
- Blast-radius discipline: re-dispatch the Step 2a LOCATIONS worker for
  `internal_bridge_lines` before removing; fix every cited site in this step or 4c.
- Expected sub-agent dispatches: reuse Step 2a worker session if fresh.
- Context cost: `M`
- Authoritative docs: decision brief - Item 3
- OrcaSlicer refs: `generate_sparse_infill_polylines_for_anchoring` - delegate SUMMARY
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -5` - FACT pass/fail
  - `rg -q 'internal_bridge_lines' crates/ && echo STILL_PRESENT || echo RETIRED` - expect RETIRED (code sites; test rewrites land in 4c)
- Exit condition: compiles; construction path exercised by existing runtime tests except
  the rewritten contract (4c).

### Step 4c: Contract rewrite + literal completion

- Task IDs: `N-A` (backlog: ISSUE-82 venue split)
- Objective: rewrite `infill_postprocess_contract_tdd.rs` asserting anchored construction
  from committed areas + anchors (234a pure-emitter check reversed deliberately); finish any
  residual literal/assertion sites.
- Precondition: Step 4b exit.
- Postcondition: AC-4 command green end-to-end.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs` - full
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs`
  - `crates/slicer-sdk/src/test_support/fixtures.rs` (known residual literal home)
  - at most ONE further residual site from the Step 4b dispatch; any more ⇒ split the step
- Files explicitly out of bounds: production code (should need nothing)
- Blast-radius discipline: same worker as 4b; zero uncited sites may remain.
- Expected sub-agent dispatches: none beyond 4b reuse.
- Context cost: `M`
- Authoritative docs: decision brief - Item 7 AC-4
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test contract -- infill_postprocess_constructs_anchored_paths --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: AC-4 full command string (test AND absence grep) succeeds.

### Step 5a: Expansion-zone parity

- Task IDs: `N-A` (backlog: ISSUE-82 coverage half, F4)
- Objective: port expansion zones onto anchored construction: `expansion_step = scaled(0.1)`
  up to 5 steps, `expansion_bottom_bridge = shell_width*sqrt(2)`, frSolidInfill-spacing
  closing radius; attribution header on ported sections.
- Precondition: Step 4c exit.
- Postcondition: construction grows candidates canonically; unit tests pin constants.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` - full
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/bridge_over_infill.rs`
  - `crates/slicer-runtime/src/layer_executor.rs` (parameter threading)
  - `crates/slicer-core/tests/bridge_support_gating_tdd.rs` (or sibling core test file)
- Files explicitly out of bounds: modules (5c); WIT/IR
- Blast-radius discipline: constant additions are internal; no schema bump.
- Expected sub-agent dispatches: design.md dispatch #1 (SNIPPETS ≤3x30).
- Context cost: `M`
- Authoritative docs: `docs/ORCASLICER_ATTRIBUTION.md` - header obligation
- OrcaSlicer refs: expansion-zone block in `PrintObject.cpp::bridge_over_infill` - delegate
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: suite green incl. new expansion cases; attribution headers present.

### Step 5b: Depth harvesting + clustering

- Task IDs: `N-A` (backlog: ISSUE-82 coverage half, F4)
- Objective: port `gather_areas_w_depth` downward harvesting and thread clustering +
  filled-polygons-on-lower-layers removal as pure helpers consumed at the construction site.
- Precondition: Step 5a exit.
- Postcondition: thick-span anchoring behaves canonically; determinism preserved.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` - full
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/bridge_over_infill.rs`
  - `crates/slicer-runtime/src/layer_executor.rs`
  - one core/runtime test file for the new behaviour
- Files explicitly out of bounds: modules; scheduler stage set
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches: design.md dispatch #4 (SUMMARY ≤200 words).
- Context cost: `M`
- Authoritative docs: `docs/ORCASLICER_ATTRIBUTION.md` - header obligation
- OrcaSlicer refs: `gather_areas_w_depth`, cluster loops - delegate
- Verification:
  - `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd 2>&1 | tee target/test-output.log` - FACT pass/fail
  - `cargo test -p slicer-runtime --test e2e -- wedge_linked_infill_report_tdd --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: both green; any wedge failure = STOP-and-report.

### Step 5c-host: carrier-free duplicate authoring + emission coverage

- Task IDs: `N-A` (backlog: P75 key `enable_extra_bridge_layer`)
- Objective: prepass appends gated duplicates to the upper layer's existing
  `internal_bridge_areas`; the EXISTING InfillPostProcess path constructs them; net-new
  integration test pins byte-stability (key off/default) + duplication (key on).
- Precondition: Step 5b exit.
- Postcondition: AC-7 green end-to-end; default-off emitted bytes unchanged.
- Files allowed to read: `slice_postprocess_prepass.rs` shell/gate region; ONE existing
  integration test as house pattern.
- Files allowed to edit (max 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  - ONE new/existing test file under `crates/slicer-runtime/tests/integration/` (+ its
    aggregator registration line)
  - at most ONE residual site
- Expected sub-agent dispatches: none beyond blast-radius reuse.
- Context cost: `M`
- Authoritative docs: decision brief extra-layer capture.
- Orca refs: canonical second-pass placement — delegate SNIPPETS if needed.
- Verification:
  - `cargo test -p slicer-runtime --test integration -- extra_bridge_layer_emission_semantics --nocapture 2>&1 | tee target/test-output.log` FACT pass/fail
  - `cargo xtask build-guests --check; echo EXIT=$?` EXIT=0.
- Exit condition: both green.

### Step 6a: Arbitration harness (net-new e2e)

- Task IDs: `N-A` (backlog: ISSUE-82 arbitration protocol)
- Objective: bundle-primary calicat arbiter test (AC-5) + mixed-density rejection test
  (AC-N1), registered through the e2e aggregator.
- Precondition: Steps 4c and 5 exits.
- Postcondition: both net-new tests exist and assert the frozen bars.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` - as the house pattern
  - `docs/19_visual_debug.md` - delegated SUMMARY for capture invocation
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_arbiter_e2e_tdd.rs` (net-new)
  - `crates/slicer-runtime/tests/e2e/mixed_density_internal_bridge_rejection_e2e_tdd.rs` (net-new)
  - `crates/slicer-runtime/tests/e2e/main.rs` (registrations)
- Files explicitly out of bounds: production code
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - Question: minimal in-test invocation of the capture pipeline used by visual-debug;
    scope `crates/pnp-cli/src/visual_debug.rs` + `crates/slicer-runtime/src/visual_debug_render.rs`;
    return SNIPPETS ≤30 lines.
- Context cost: `M`
- Authoritative docs: decision brief - Items 5/7
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_arbiter_e2e_tdd --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail
  - `cargo test -p slicer-runtime --test e2e -- mixed_density_internal_bridge_rejection_e2e_tdd --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: AC-5 and AC-N1 commands green.

### Step 6b: Gcode bars + golden ceremony

- Task IDs: `N-A` (backlog: ISSUE-82 regression set)
- Objective: revise `calicat_internal_bridge_gating_e2e_tdd` to AC-6 bars; run the NEG-2
  re-bless ceremony with the mandated evidence table; keep wedge tripwire green (AC-N3).
- Precondition: Step 6a exit.
- Postcondition: AC-6/AC-N2/AC-N3 commands green; golden comment carries evidence.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs` - full
  - `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` - NEG-2 test only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_gating_e2e_tdd.rs`
  - `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` (BLESS only,
    with in-comment evidence)
  - `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` (doc-comment only)
- Files explicitly out of bounds: production code
- Blast-radius discipline: re-bless ONLY via `BLESS_GOLDEN=1` documented flow; diff table +
  Z-set identity + per-diff-class reasoning recorded BEFORE blessing.
- Expected sub-agent dispatches:
  - Question: run legacy precision slice twice, return section-count diff table vs current
    golden + Z-set comparison; scope `target/test-output.log`; return FACT ≤5 lines.
- Context cost: `M`
- Authoritative docs: decision brief - golden policy
- OrcaSlicer refs: none
- Verification: AC-6, AC-N2, AC-N3 command strings - FACT pass/fail each.
- Exit condition: all three green; evidence table present in golden comment.

### Step 6c: Doc impact + closure gates

- Task IDs: `N-A` (backlog: ISSUE-82 closure)
- Objective: execute Doc Impact (F3/F4 rows with measured numbers); final gates.
- Precondition: Step 6b exit.
- Postcondition: doc greps hit; all packet-level gates green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/bridge-parity-plan.md` - F3/F4 sections only
- Files allowed to edit (at most 3):
  - `docs/specs/bridge-parity-plan.md`
  - `docs/spec_packets/234a-internal-bridge-support-gating/task-map.md` (status notes)
- Files explicitly out of bounds: source code
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches: measured-number extraction from `target/test-output.log`
  (FACT ≤5 lines).
- Context cost: `S`
- Authoritative docs: `docs/specs/bridge-parity-plan.md` - F3/F4
- OrcaSlicer refs: none
- Verification:
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask check-literals` - exit 0
  - `rg -q '### F3 — HIGH' docs/specs/bridge-parity-plan.md && rg -q '### F4 — HIGH' docs/specs/bridge-parity-plan.md && echo DOCS_OK`
- Exit condition: all green; packet ready for status transition review.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 | S | RC-A init flip + fixtures |
| 2a | M | IR fields + literal sweep |
| 2b | M | WIT/view/render |
| 3 | M | qualification rewrite |
| 4a | S | feasibility probe |
| 4b | M | constructor + retirement |
| 4c | M | contract rewrite |
| 5a | M | expansion zones |
| 5b | M | depth harvest + clustering |
| 5c-host | M | carrier-free duplicates + AC-7 |
| 6a | M | net-new e2e pair |
| 6b | M | gcode bars + golden |
| 6c | S | docs + gates |

Aggregate `M`; no step is `L`.

## Packet Completion Gate

- All steps and exits complete; 4a probe outcome recorded (positive or STOP handled).
- Every pipe-suffixed AC command returns PASS (AC-1..AC-6, AC-N1..AC-N3).
- Gates: `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo xtask check-literals`, `cargo xtask build-guests --check` exit 0.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full read.
- Reconcile reopened/superseded status transitions (this packet revises itself in place).
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- THEN, and only then: `cargo xtask test --workspace` dispatched to a sub-agent returning
  FACT pass/fail (AGENTS.md acceptance rule; includes guest-freshness preflight).
- Record remaining packet-local risk (opening-radius audit outcome; 20mm-box steady-state
  classification).
- Confirm context stayed ≤150k standard, or ≤300k only with a logged swarm ESCALATION;
  otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use `--all-targets` so the test, bench, and example targets compile.
