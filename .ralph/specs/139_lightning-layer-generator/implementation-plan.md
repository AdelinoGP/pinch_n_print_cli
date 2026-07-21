# Implementation Plan: 139_lightning-layer-generator

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- All `cargo check`, `cargo clippy`, and `cargo test` invocations must use `--all-targets`
  where applicable so the test, bench, and example targets compile.

## Steps

### Step 0: Per-region refinement bundle (RED→GREEN) — closes `D-137-LIGHTNING-PER-OBJECT-COLLAPSE`

- Task IDs: `TASK-264`
- Objective: add `region_id: RegionId` field to `LightningTreeEntry`; fix the host
  dispatch HashMap keying in `dispatch.rs:1383` from `wildcard_region = "*"` to
  the actual `region_id` (mirroring `support-plan-segments` at `:1353`); update the
  SDK accessor `lightning_tree_segments_for` at `traits.rs:195-199` to honor
  `region_id` (no longer `_region_id`); update the 137 roundtrip test in
  `lightning_tree_view_roundtrip_tdd.rs` to add a per-region assertion; add a new
  `lightning_tree_per_region_roundtrip_tdd.rs` (AC-N3) that proves two regions on
  the same `(object, layer)` get distinct segment buckets.
- Precondition: packet 137 status `implemented` (forward-dep), packets 138+139
  algorithm bodies NOT YET LANDED (this step is the per-region scaffolding that
  the algorithm body will later fill; runs before Steps 1-4 to lock the IR shape
  the algorithm will commit into).
- Postcondition: AC-3 per-region wiring green; AC-N3 per-region SDK isolation
  green; `cargo check --workspace --all-targets` clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` (lines 1100-1200 — `SupportPlanIR` +
    `SupportPlanEntry` precedent at `:1129`; lines 1215-1247 for
    `LightningTreeEntry` + `LightningTreeIR` shape)
  - `crates/slicer-wasm-host/src/dispatch.rs` (lines 1330-1410 — the
    `build_paint_layer_data_with_plan` function and its `support-plan-segments`
    keying at `:1353`)
  - `crates/slicer-sdk/src/traits.rs` (lines 50-200 — `PaintRegionLayerView` +
    accessor methods)
  - `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs` (137;
    full — small file)
- Files allowed to edit (at most 4):
  - `crates/slicer-ir/src/slice_ir.rs` (one field addition on
    `LightningTreeEntry`; one `pub use` re-export if needed)
  - `crates/slicer-wasm-host/src/dispatch.rs` (~5 lines around `:1383` —
    `wildcard_region` → `entry.region_id.to_string()`)
  - `crates/slicer-sdk/src/traits.rs` (~5 lines in
    `lightning_tree_segments_for` — add `region_id` to the filter)
  - `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs`
    (add a per-region assertion; preserve the existing single-region case as
    `region_id = 0` default)
  - `crates/slicer-runtime/tests/contract/lightning_tree_per_region_roundtrip_tdd.rs`
    (new file — the AC-N3 per-region isolation test)
  - `crates/slicer-runtime/tests/contract/main.rs` (one `mod` line to register the
    new test file)
- Blast-radius discipline (mandatory for the new `region_id` field):
  - **Struct-literal blast radius for `LightningTreeEntry`:** the 137 test
    `lightning_tree_view_roundtrip_tdd.rs` constructs `LightningTreeEntry` via a
    helper `fixture_entry(...)` that takes `(object_id, layer_index, segments)` —
    must be extended to take `region_id` (default `0` for backward compat) AND
    all three existing call sites in that file must be updated. Any other
    `LightningTreeEntry` construction sites (e.g. in 138's test home, if 138
    lands first) must be discovered via a `rg 'LightningTreeEntry \{'` FACT
    BEFORE this step edits. If 138 has not yet landed, the only construction
    site is the 137 test file.
  - **HashMap keying blast radius:** the wildcard `*` key is replaced by
    `region_id.to_string()`. The corresponding test assertion in the 137 test
    file (which uses `wildcard_region = "*"`) must be replaced by the per-region
    assertion. No other site in the tree references the wildcard.
- Files explicitly out-of-bounds for this step: `crates/slicer-core/src/algos/lightning/{layer,generator}.rs`
  (Steps 1-2's new files; this step runs first and is purely a host/IR plumbing
  change); `modules/core-modules/lightning-infill/**` (140's surface).
- Expected sub-agent dispatches:
  - "FACT: every `LightningTreeEntry` construction site in the tree (rg
    `'LightningTreeEntry \{'`)" — Step 0 driver.
  - "FACT: every reference to `wildcard_region` in the tree (rg)" — Step 0
    driver.
  - "Run `cargo test -p slicer-ir --lib lightning_tree_ir` + `cargo test -p
    slicer-runtime --test contract -- lightning_tree_per_region_roundtrip` +
    `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip`;
    FACT each" — Step 0 verification.
- Context cost: `L` (justified: IR field + dispatch keying + SDK projection +
  137-test update + new test file are all coupled — partial state breaks the
  `LightningTreeEntry` construction sites and leaves the workspace un-compiling
  at the seam. The 139 design's `§Context Cost Estimate` carries the
  justification.)
- Authoritative docs: ADR-0029 (per-region discipline), `docs/DEVIATION_LOG.md`
  `D-137-LIGHTNING-PER-OBJECT-COLLAPSE` (the deviation being closed).
- OrcaSlicer refs: none.
- Verification:
  - AC-3 pipe command — FACT
  - AC-N3 pipe command — FACT
- Exit condition: per-region refinement green; per-region construction-site
  survey done; the 137 test file's per-region assertion added.

### Step 1: Overhang pass — `generate_initial_internal_overhangs` (RED→GREEN)

- Task IDs: `TASK-264`
- Objective: constants FACT first (dilation constant, per-layer move distance, density-
  coupled inputs from `FillLightning.cpp`); author the AC-1 two-layer synthetic test
  (RED); port the overhang pass into `generator.rs` (attribution header) to GREEN.
- Precondition: Step 0 exit condition (per-region IR + dispatch + SDK projection
  in place).
- Postcondition: AC-1 green; `[FWD]` density-coupling recorded resolved.
- Files allowed to read: own lightning module + `FillLightning.cpp` (delegated SUMMARY).
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/generator.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (one `pub use` export line)
  - the lightning test home (decided in 138; add the generator test beside the
    primitive tests)
- Blast-radius discipline: none — both files are net-new; the 137 skeleton's
  `generate_lightning_trees` signature is unchanged at this step. The 137-era
  `LightningTreeEntry` construction site is in the test file (Step 0 already
  extended it for the `region_id` field).
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly;
  `layer.rs` (Step 2).
- Expected sub-agent dispatches:
  - the constants FACT + the `Generator.cpp` overhang-pass section dispatch
  - "Run `cargo test -p slicer-core -- lightning_generator_overhangs …`; FACT"
- Context cost: `M`
- Authoritative docs: `docs/specs/lightning-infill-parity.md` §L3.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp`
  (sectioned), `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` (delegate).
- Verification:
  - AC-1 pipe command — FACT
- Exit condition: AC-1 green; `[FWD]` density-coupling recorded.

### Step 2: `Lightning::Layer` port — seeding, reconnect, convert

- Task IDs: `TASK-264`
- Objective: port `generateNewTrees`, `reconnectRoots`, `convertToLines` into `layer.rs`
  (attribution header), TDD'd on single-layer synthetics (seed inside overhang; roots
  reconnect to outline; conversion yields 2-point segments).
- Precondition: Step 1 exit condition.
- Postcondition: layer-level tests green.
- Files allowed to read: own lightning module.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/layer.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (one `pub use` export line)
  - the lightning test home (add the layer tests)
- Blast-radius discipline: none — `layer.rs` is net-new; the `// 139 wiring point` is
  still in `mod.rs` until Step 4.
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the `Layer.cpp` section dispatches (≥ 4 sections)
  - "Run `cargo test -p slicer-core -- lightning_layer …`; FACT + counts; SNIPPETS
    ≤ 20 on failure"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` —
  delegate, sectioned.
- Verification:
  - `cargo test -p slicer-core -- lightning 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
- Exit condition: layer tests green. **Split tripwire:** if the port exceeds M
  mid-flight, STOP and split (convertToLines + producer wiring become the successor) —
  never rate L and continue.

### Step 3: `generate_trees` two-pass + continuity + determinism

- Task IDs: `TASK-264`
- Objective: port the two-pass `generate_trees` loop (top-down outlines pass, then
  top-down growth with `propagate_to_next_layer`); AC-2 continuity on the single-
  overhang prism; AC-4 determinism; AC-N1 no-overhang case.
- Precondition: Step 2 exit condition.
- Postcondition: AC-2, AC-4, AC-N1 green.
- Files allowed to read: own lightning module.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/generator.rs` (extend)
  - the lightning test home (add the two-pass / continuity / determinism tests)
- Blast-radius discipline: none — the 138 APIs are frozen; this step is in
  `generator.rs` only.
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - the `Generator.cpp` growth-pass section dispatch
  - "Run `cargo test -p slicer-core -- lightning_generator …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: ADR-0029 (two-pass structure, delegate).
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp`
  growth-pass region — delegate.
- Verification:
  - AC-2, AC-4, AC-N1 pipe commands — FACT each
- Exit condition: generation semantics green.

### Step 4: Producer wiring + guards + gates

- Task IDs: `TASK-264`
- Objective: replace the 137 skeleton's `// 139 wiring point` body with the real
  driver — per object, construct the generator over the committed sparse outlines
  (inputs per the Step-1 FACT), store per-layer `convert_to_lines` output into
  `LightningTreeIR`; extend the 137 executor test with the commits-real-trees case
  (AC-3); run the wedge guard (AC-N2); workspace gates + guest freshness.
- Precondition: Step 3 exit condition.
- Postcondition: all packet ACs green.
- Files allowed to read, with ranges when over 300 lines:
  - the support-producer input-access LOCATIONS results (ranged)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/mod.rs` (replace skeleton body; delete
    `// 139 wiring point` comment)
  - `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (extend with
    the commits-real-trees case)
  - (if the test home is the separate `algo_lightning_tdd.rs` file from 138, this step
    adds the AC-3 fixture inputs there; otherwise no third file edit)
- Blast-radius discipline:
  - **The `generate_lightning_trees` signature change in `mod.rs`** (now wires the
    real generator instead of returning empty IR) — verify the 137 builtin wrapper
    call site still compiles against the new signature (FACT before edit; adjust the
    wrapper call if the signature changes shape). If the wrapper needs to change,
    the wrapper edit is budgeted into this step (≤ 3-file edit cap).
  - Dispatch a `LOCATIONS` FACT for the `generate_lightning_trees` call sites
    (expected: 1 — the 137 builtin wrapper at
    `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs`) before this step
    edits.
- Files explicitly out-of-bounds for this step: WIT/SDK/module files.
- Expected sub-agent dispatches:
  - "LOCATIONS ≤ 10: support-geometry producer whole-print input access"
  - "LOCATIONS ≤ 5: call sites of `generate_lightning_trees` (the 137 builtin wrapper)"
  - "Run `cargo test -p slicer-runtime --test executor -- lightning …`; FACT" — AC-3.
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT" — AC-N2.
  - "Run `cargo clippy --workspace --all-targets -- -D warnings` + `cargo check
    --workspace --all-targets` + `cargo xtask build-guests --check`; FACT each"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-3, AC-N2 pipe commands + §Verification gates — FACT each
- Exit condition: all ACs green; gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | L (justified) | per-region IR + dispatch + SDK projection + 137 test update + new test file — atomic coupled bundle that closes D-137-LIGHTNING-PER-OBJECT-COLLAPSE |
| Step 1 | M | overhang pass + constants |
| Step 2 | M | `Layer.cpp` port (448 lines; tripwire armed) |
| Step 3 | M | two-pass growth + determinism |
| Step 4 | M | wiring + guards + gates |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (TASK-264 flip),
  never a full backlog read.
- Reconcile reopened/superseded status transitions (138 API deviations, if any,
  recorded; 138 tests co-updated in the same step).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged
  swarm ESCALATION; otherwise record a packet-authoring lesson.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
