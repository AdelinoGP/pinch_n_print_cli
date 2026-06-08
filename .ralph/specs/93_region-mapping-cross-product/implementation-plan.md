# Implementation Plan: 93_region-mapping-cross-product

## Execution Rules

- One atomic step at a time.
- Each step ends with the narrowest test green before proceeding.
- Test output teed to `target/test-output.log`.
- Pre-packet baseline SHA captured in Step 0; AC-10 depends on it.

## Steps

### Step 0: Capture pre-packet baseline g-code SHA into closure-log.md

- Task IDs: `TASK-243`
- Objective: AC-10 prerequisite. The SHA is written to `.ralph/specs/93_region-mapping-cross-product/closure-log.md` as the line `P92_BASELINE_SHA=<hex>` so AC-10's shell command can read it back.
- Precondition: P92 closed.
- Postcondition: baseline SHA recorded in `closure-log.md`.
- Files allowed to read: none.
- Files allowed to edit:
  - `.ralph/specs/93_region-mapping-cross-product/closure-log.md` (CREATE or append).
- Files out-of-bounds: any other file.
- Expected sub-agent dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p93-baseline.gcode && sha256sum target/p93-baseline.gcode | awk '{print $1}'`; return FACT (single sha256 hash, hex only)".
  - Then write `P92_BASELINE_SHA=<hash>` as a line in `closure-log.md` (delegated edit).
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `grep -q 'P92_BASELINE_SHA=[a-f0-9]\{64\}' .ralph/specs/93_region-mapping-cross-product/closure-log.md` exits 0.
- Exit condition: closure-log.md carries the baseline SHA.

### Step 1: Inventory `region_mapping.rs` symbols + `DEFAULT_REGION_MAP_CAP` location + producer-wrapper context shape + AC-7b fixture

- Task IDs: `TASK-243`
- Objective: confirm the symbol locations pre-baked into `design.md` (`execute_region_mapping_inner` line 384, `overlay_resolved` line 110, `stamp_modifier_config_deltas` line 217, `overlapping_semantics_for_region` line 286, `DEFAULT_REGION_MAP_CAP` at `crates/slicer-ir/src/slice_ir.rs:1196`) are still accurate; record the producer-wrapper context-shape so Step 4 can land cleanly; capture a pre-packet fixture of `overlapping_semantics_for_region`'s output for AC-7b's equivalence test.
- Precondition: Step 0 complete.
- Postcondition: file:line for each landed in implementer's notes; producer-wrapper context shape recorded; AC-7b fixture captured.
- Files allowed to read: none directly.
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Re-verify LOCATIONS in `crates/slicer-core/src/algos/region_mapping.rs` for `execute_region_mapping_inner`, `overlay_resolved`, `stamp_modifier_config_deltas`, `overlapping_semantics_for_region`. Cap at 8 entries".
  - "Run `rg -nE 'DEFAULT_REGION_MAP_CAP' crates/`; return LOCATIONS".
  - "Run `rg -nE 'aggregate_region_splits|AggregatedRegionSplitEntry' crates/slicer-scheduler/src/`; return LOCATIONS (≤ 5)" — confirm the scheduler-side aggregator from P92.
  - "Open `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` (full read OK; ≤ 60 LOC) and return SNIPPETS (≤ 30 lines) showing the wrapper's commit signature and access to scheduler context".
  - "Run a one-off harness against `resources/regression_wedge.stl` capturing `overlapping_semantics_for_region(layer_idx=0, paint_regions=&pr)` and the resulting `effective_config` for region_id=0; serialize to a JSON fixture under `crates/slicer-core/tests/fixtures/p93_overlay_baseline.json`. Return FACT path".
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: LOCATIONS non-empty for each; fixture file exists.
- Exit condition: inventory + fixture recorded.

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

### Step 3: Extend kernel — add `scan_paint_data`, cross-product loop, chain-derived overlay fold, DELETE `overlapping_semantics_for_region` + line-494 caller

- Task IDs: `TASK-243`
- Objective: AC-1, AC-2, AC-4, AC-5, AC-7, AC-N3.
- Precondition: Step 2 complete; kernel and producer signatures decided.
- Postcondition: `execute_region_mapping_inner` produces cross-product entries; `overlapping_semantics_for_region` AND its call site at line 494 are deleted; defensive guard rejects Scalar; `stamp_modifier_config_deltas` (line 217) called before the chain fold as the base; `overlay_resolved` (line 110) called once per `(semantic, value)` in each chain.
- Files allowed to read:
  - `crates/slicer-core/src/algos/region_mapping.rs` — range-read by symbol.
  - `crates/slicer-scheduler/src/region_split.rs` — for `AggregatedRegionSplitEntry` shape.
  - `crates/slicer-ir/src/slice_ir.rs` — for `RegionPlan`, `RegionMapIR`, `ConfigId`, `intern_config` reference.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/region_mapping.rs`.
- Files out-of-bounds for this step: producer wrapper (Step 4); tests (Step 6).
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-core`; return FACT pass/fail with first error".
  - "Run `! rg -q 'overlapping_semantics_for_region' crates/slicer-core/src/algos/region_mapping.rs`; return FACT pass/fail" — purpose: confirm AC-7 deletion.
- Context cost: `M`.
- Authoritative docs: roadmap §"P1c"; OrcaSlicer PrintApply SUMMARY (delegated).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — delegate SUMMARY at start of step.
- Verification: `cargo check -p slicer-core` clean; AC-7 grep returns exit 0 (function absent).
- Exit condition: AC-1, AC-7 satisfied (other ACs gated by Step 6 tests).

### Step 4: Update `region_mapping_producer.rs` to thread `aggregated_region_split`

- Task IDs: `TASK-243`
- Objective: producer wrapper feeds the kernel correctly.
- Precondition: Step 3 green.
- Postcondition: producer compiles; workspace check clean.
- Files allowed to read:
  - `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — full (≤ 60 LOC).
  - The scheduler-provided context type (locate via Step 1 inventory).
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

### Step 5: Raise `DEFAULT_REGION_MAP_CAP` from 1_000 to 750_000; update overflow diagnostic to identify top contributor

- Task IDs: `TASK-243`
- Objective: AC-8, AC-N2.
- Precondition: Step 4 green.
- Postcondition: constant at `crates/slicer-ir/src/slice_ir.rs:1196` is `750_000` (raised from `1_000` baseline); cap-overflow test added and passing; doc-comment records the 750× jump rationale.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — locate constant at line 1196 (confirmed by Step 1).
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

### Step 6: Add six net-new kernel unit tests (AC-9) + AC-7b overlay-equivalence test + AC-N1 + AC-N3

- Task IDs: `TASK-243`
- Objective: AC-2, AC-4, AC-5, AC-7b, AC-9, AC-N1, AC-N3.
- Precondition: Step 5 green; Step 1 fixture captured.
- Postcondition: all kernel-level unit tests pass with exact assertions on `variant_chain` keysets and exact `ResolvedConfig` equality for AC-7b.
- Files allowed to read:
  - `crates/slicer-core/tests/algo_region_mapping_tdd.rs` — range-read by test name.
  - `crates/slicer-core/tests/fixtures/p93_overlay_baseline.json` (created in Step 1) — load for AC-7b comparison.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/tests/algo_region_mapping_tdd.rs`.
- Files out-of-bounds: kernel (Step 3); producer (Step 4); cube_4color test files.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log`; return FACT with per-test breakdown — must include the six AC-9 test names, the AC-7b test name, AC-N1, AC-N3, and the scan + cap tests".
- Context cost: `M`.
- Authoritative docs: roadmap §"P1c" net-new test list.
- OrcaSlicer refs: none.
- Verification: all enumerated tests pass; failure of AC-7b's overlay-equivalence test BLOCKS the packet until root-caused (it indicates the chain fold and the deleted layer-wide path diverge).
- Exit condition: AC-2, AC-4, AC-5, AC-7b, AC-9, AC-N1, AC-N3 satisfied.

### Step 7: Behavior preservation — AC-10 byte-identical g-code on regression_wedge.stl

- Task IDs: `TASK-243`
- Objective: AC-10.
- Precondition: Step 6 green.
- Postcondition: post-packet SHA byte-identically matches Step 0 baseline (read back from closure-log.md).
- Files allowed to read:
  - `.ralph/specs/93_region-mapping-cross-product/closure-log.md` — to retrieve `P92_BASELINE_SHA=<hex>` for the comparison.
- Files allowed to edit: none.
- Files out-of-bounds: any other.
- Expected sub-agent dispatches:
  - "Run the AC-10 baseline-compare shell command (see `packet.spec.md` AC-10): `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p93-wedge-post.gcode && test \"$(sha256sum target/p93-wedge-post.gcode | awk '{print $1}')\" = \"$(grep -oE 'P92_BASELINE_SHA=[a-f0-9]+' .ralph/specs/93_region-mapping-cross-product/closure-log.md | head -1 | cut -d= -f2)\"`; return FACT exit code".
  - On mismatch: "Run `diff -u target/p93-baseline.gcode target/p93-wedge-post.gcode | head -200`; return SNIPPETS ≤ 100 lines" — purpose: root-cause the diff. A diff here means AC-7b's unit-level check passed but integration-level overlay equivalence failed — escalate.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-10 command exits 0. If not, packet is BLOCKED until root-caused.
- Exit condition: AC-10 satisfied.

### Step 8: Guest WASM rebuild + `--check`

- Task IDs: `TASK-243`
- Objective: AC-11.
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
- Exit condition: AC-11 satisfied.

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
| Step 0 | S | Baseline g-code SHA. |
| Step 1 | S | Symbol inventory + AC-7b fixture capture. |
| Step 2 | S | `enumerate_canonical_chains` helper + tests. |
| Step 3 | M | Kernel extension + deletion of `overlapping_semantics_for_region` + line-494 caller. |
| Step 4 | S | Producer wrapper. |
| Step 5 | S | Cap + diagnostic. |
| Step 6 | M | Six AC-9 net-new tests + AC-7b overlay-equivalence + AC-N1/N3. |
| Step 7 | S | Byte-identical g-code check. |
| Step 8 | S | Guest rebuild. |
| Step 9 | S | Workspace gate. |

Aggregate: M.

## Packet Completion Gate

- All 10 steps complete; each exit condition satisfied.
- AC-1 through AC-11 + AC-7b + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: baseline SHA, post-packet SHA (match), per-test breakdown for the six AC-9 tests + AC-7b.
- `docs/07_implementation_status.md` updated for `TASK-243` (delegate edit).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC command; confirm PASS.
- Confirm clippy + targeted tests green.
- Confirm byte-identical g-code (AC-10).
- Peak context usage under 70%.
