# Implementation Plan: 93_region-mapping-cross-product

## Execution Rules

- One atomic step at a time.
- Each step ends with the narrowest test green before proceeding.
- Test output teed to `target/test-output.log`.
- Pre-packet baseline SHA captured in Step 0; AC-11 depends on it.

## Steps

### Step 0: Capture pre-packet baseline g-code SHA

- Task IDs: `TASK-243`
- Objective: AC-11 prerequisite.
- Precondition: P92 closed.
- Postcondition: baseline SHA recorded.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-baseline.gcode && sha256sum /tmp/p93-baseline.gcode`; return FACT (sha256)".
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: FACT returned.
- Exit condition: hash recorded.

### Step 1: Inventory `region_mapping.rs` symbols + `DEFAULT_REGION_MAP_CAP` location

- Task IDs: `TASK-243`
- Objective: pinpoint kernel functions to rewrite + constant to raise.
- Precondition: Step 0 complete.
- Postcondition: file:line for each landed in implementer's notes.
- Files allowed to read: none directly.
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Open `crates/slicer-core/src/algos/region_mapping.rs` and return LOCATIONS for `commit_region_mapping_builtin`, `overlapping_semantics_for_region`, `derive_resolved_config`, `derive_stage_modules`. Cap at 10 entries".
  - "Run `rg -nE 'DEFAULT_REGION_MAP_CAP' crates/`; return LOCATIONS".
  - "Run `rg -nE 'aggregate_region_splits|AggregatedRegionSplitEntry' crates/slicer-scheduler/src/`; return LOCATIONS (≤ 5)" — confirm the scheduler-side aggregator from P1b.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: LOCATIONS non-empty for each.
- Exit condition: inventory recorded.

### Step 2: Implement `enumerate_canonical_chains` in `slicer-ir`

- Task IDs: `TASK-243`
- Objective: AC-3.
- Precondition: Step 1 complete.
- Postcondition: helper exists with unit tests for 2×1 (6 chains), 1×4 (5 chains), 0-semantic (1 chain) cases.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — read `PaintValue` def only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/region_split_registry.rs` (NEW, ≤ 100 LOC).
  - `crates/slicer-ir/src/lib.rs` (one line: `pub mod region_split_registry;`).
  - `crates/slicer-ir/tests/region_split_registry_tdd.rs` (NEW, ≤ 80 LOC).
- Files out-of-bounds for this step: any.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-ir region_split_registry 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs: roadmap §"P1c" enumeration spec.
- OrcaSlicer refs: none.
- Verification: 3 unit tests pass.
- Exit condition: AC-3 satisfied.

### Step 3: Rewrite kernel — add `scan_paint_data`, cross-product loop, defensive Scalar guard, replace `overlapping_semantics_for_region`

- Task IDs: `TASK-243`
- Objective: AC-1, AC-2, AC-4, AC-5, AC-7, AC-N3.
- Precondition: Step 2 complete; kernel and producer signatures decided.
- Postcondition: kernel produces cross-product entries; `overlapping_semantics_for_region` removed; defensive guard rejects Scalar.
- Files allowed to read:
  - `crates/slicer-core/src/algos/region_mapping.rs` — full read OK.
  - `crates/slicer-scheduler/src/region_split.rs` — for `AggregatedRegionSplitEntry` shape.
  - `crates/slicer-ir/src/slice_ir.rs` — for `RegionPlan`, `RegionMapIR`, `ConfigId`, `intern_config` reference.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/region_mapping.rs`.
- Files out-of-bounds for this step: producer wrapper (Step 4); tests (Step 5).
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-core`; return FACT pass/fail with first error".
  - "Run `cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `M`.
- Authoritative docs: roadmap §"P1c"; OrcaSlicer PrintApply SUMMARY (delegated).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — delegate SUMMARY at start of step.
- Verification: kernel unit tests for `scan_paint_data`, `cross_product_entry_count`, and `no_scalar_in_variant_chain` all pass.
- Exit condition: AC-1, AC-2, AC-4, AC-5, AC-7, AC-N3 satisfied.

### Step 4: Update `region_mapping_producer.rs` to thread `aggregated_region_split`

- Task IDs: `TASK-243`
- Objective: producer wrapper feeds the kernel correctly.
- Precondition: Step 3 green.
- Postcondition: producer compiles; workspace check clean.
- Files allowed to read:
  - `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — full (≤ 60 LOC).
  - The scheduler-provided context type (locate via Grep).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`.
  - The producer trait or context-shape file IF the wrapper's signature needs to widen.
- Files out-of-bounds: kernel.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs: `docs/04_host_scheduler.md` §"BuiltinProducer".
- OrcaSlicer refs: none.
- Verification: workspace check clean.
- Exit condition: producer wired.

### Step 5: Raise `DEFAULT_REGION_MAP_CAP` to 750_000; update overflow diagnostic to identify top contributor

- Task IDs: `TASK-243`
- Objective: AC-8, AC-N2.
- Precondition: Step 4 green.
- Postcondition: cap raised; cap-overflow test added and passing.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — locate constant via Step 1's LOCATIONS.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs` (cap constant + doc-comment).
  - `crates/slicer-core/src/algos/region_mapping.rs` (top-contributor helper if not added in Step 3).
  - `crates/slicer-runtime/tests/integration/region_map_cap_overflow_tdd.rs` (NEW).
- Files out-of-bounds: any other.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime region_map_cap 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs: roadmap §"P1c" cap subsection.
- OrcaSlicer refs: none.
- Verification: cap-overflow test passes.
- Exit condition: AC-8, AC-N2 satisfied.

### Step 6: Cube_4color test review — confirm 5 GREEN / 7 RED distribution; adjust if necessary

- Task IDs: `TASK-243`
- Objective: AC-9, AC-10.
- Precondition: Step 5 green.
- Postcondition: 5 cube_4color tests asserting on `variant_chain` shape pass; 7 cube_4color tests asserting on per-variant polygons fail (expected); closure log enumerates each test's status with name.
- Files allowed to read:
  - `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` — range-read by test name; file likely > 300 lines.
- Files allowed to edit (≤ 3):
  - Only those cube tests whose assertion text needs an `RegionKey.variant_chain` shape alignment — and ONLY with closure-log justification (per AC-N1 of P0a's pattern).
- Files out-of-bounds: kernel (Step 3); producer (Step 4).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; return FACT with per-test breakdown (pass/fail per test name)".
  - "On any unexpected pass/fail: `Grep` for the test name in the test file; return SNIPPETS ≤ 30 lines of the test body" — purpose: classify the divergence.
- Context cost: `M`.
- Authoritative docs: roadmap §"P1c" cube-test retargeting subsection.
- OrcaSlicer refs: none.
- Verification: cube_4color test results match the 5/7 expectation OR closure log documents the divergence with reasoning per test.
- Exit condition: AC-9 satisfied (5 GREEN); AC-10 satisfied (7 RED with documented intent).

### Step 7: Behavior preservation — AC-11 byte-identical g-code on regression_wedge.stl

- Task IDs: `TASK-243`
- Objective: AC-11.
- Precondition: Step 6 green.
- Postcondition: post-packet SHA matches Step 0 baseline.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-post.gcode && sha256sum /tmp/p93-post.gcode`; return FACT (sha256)".
  - On mismatch: "Run `diff -u /tmp/p93-baseline.gcode /tmp/p93-post.gcode | head -200`; return SNIPPETS ≤ 100 lines" — purpose: root-cause the diff.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: sha256 matches baseline. If not, packet is BLOCKED until root-caused.
- Exit condition: AC-11 satisfied.

### Step 8: Guest WASM rebuild + `--check`

- Task IDs: `TASK-243`
- Objective: AC-12.
- Precondition: Step 7 green.
- Postcondition: guest WASMs clean.
- Files allowed to read / edit: none.
- Files out-of-bounds: any source.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs: `CLAUDE.md` §"Guest WASM Staleness".
- OrcaSlicer refs: none.
- Verification: PASS.
- Exit condition: AC-12 satisfied.

### Step 9: Final acceptance ceremony — narrow gates + clippy

- Task IDs: `TASK-243`
- Objective: final gate.
- Precondition: Step 8 green.
- Postcondition: clippy clean; targeted test buckets green.
- Files allowed to read / edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-core 2>&1 | tee target/test-output.log`; return FACT pass/fail with count".
  - "Run `cargo test -p slicer-ir 2>&1 | tee target/test-output.log`; return FACT pass/fail with count".
- Context cost: `S`.
- Verification: all PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baseline. |
| Step 1 | S | Symbol inventory. |
| Step 2 | S | `enumerate_canonical_chains` helper + tests. |
| Step 3 | M | Kernel rewrite. |
| Step 4 | S | Producer wrapper. |
| Step 5 | S | Cap + diagnostic. |
| Step 6 | M | Cube test review + minimal adjustments. |
| Step 7 | S | Behavior preservation check. |
| Step 8 | S | Guest rebuild. |
| Step 9 | S | Workspace gate. |

Aggregate: M.

## Packet Completion Gate

- All 10 steps complete; each exit condition satisfied.
- AC-1 through AC-12 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: baseline SHA, post-packet SHA (match), cube_4color GREEN/RED per-test breakdown with names.
- `docs/07_implementation_status.md` updated for `TASK-243` (delegate edit).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC command; confirm PASS (or expected-FAIL for AC-10's 7 RED tests).
- Confirm clippy + targeted tests green.
- Confirm byte-identical g-code (AC-11).
- Peak context usage under 70%.
