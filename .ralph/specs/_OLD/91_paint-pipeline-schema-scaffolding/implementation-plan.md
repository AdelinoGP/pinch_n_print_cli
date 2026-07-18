# Implementation Plan: 91_paint-pipeline-schema-scaffolding

## Execution Rules

- One atomic step at a time. Each step ends with `cargo check --workspace --all-targets` green.
- Step 0 is a baseline capture; without it AC-10 cannot be verified.
- TDD-first where applicable (Step 2's PaintValue Hash test before the impl, etc.); mechanical schema renames don't need a test-first approach.
- Test output teed to `target/test-output.log` per `CLAUDE.md` §Test Discipline.

## Steps

### Step 0: Capture pre-packet baseline g-code SHA on `regression_wedge.stl`

- Task IDs:
  - `TASK-241`
- Objective: record the byte-identical baseline g-code hash before any edit lands; without this, AC-10 cannot be validated.
- Precondition: working tree at packet's parent commit; `resources/regression_wedge.stl` exists (depends on packet 90 if applicable, OR fallback to `resources/cube_4color.3mf` if wedge is not yet authored — document the chosen fixture in the closure log).
- Postcondition: baseline SHA recorded in closure-log scaffold.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p91-baseline.gcode && sha256sum /tmp/p91-baseline.gcode`; return FACT with the sha256 hash" — purpose: capture baseline.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: dispatch returns a single SHA.
- Exit condition: baseline SHA stored in implementer's closure-log notes.

### Step 1: Inventory `boundary_paint` references, `plan.config` call sites, `HashablePaintValue` call sites

- Task IDs:
  - `TASK-241`
- Objective: produce LOCATIONS lists for the three rename / migration sweeps to follow.
- Precondition: Step 0 complete.
- Postcondition: three LOCATIONS lists in implementer's notes.
- Files allowed to read: none directly.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds for this step: any source file (this is pure dispatch).
- Expected sub-agent dispatches:
  - "Run `rg -nE '\bboundary_paint\b' crates/ modules/ docs/ .ralph/`; return LOCATIONS (≤ 30 entries)".
  - "Run `rg -nE '\bplan\.config\b' crates/slicer-runtime/src/`; return LOCATIONS (≤ 20 entries)".
  - "Run `rg -nE 'HashablePaintValue' crates/`; return LOCATIONS (≤ 10 entries)".
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: three non-empty LOCATIONS lists.
- Exit condition: inventory recorded.

### Step 2: Migrate `ResolvedConfig.extensions: HashMap → BTreeMap`; derive Hash; derive/impl Hash on PaintValue

- Task IDs:
  - `TASK-241`
- Objective: enable Hash on the two types the interner needs.
- Precondition: Step 1 complete.
- Postcondition: `ResolvedConfig` is `Hash + Eq`; `PaintValue` is `Hash + Eq`; unit test `paint_value_hash` (in `crates/slicer-ir/tests/` or as `#[test]` in `slice_ir.rs`) asserts `PaintValue::Scalar(1.5).hash` equals itself; workspace compiles.
- Files allowed to read:
  - `crates/slicer-ir/src/resolved_config.rs` — range-read the struct + impls (≤ 400 lines).
  - `crates/slicer-ir/src/slice_ir.rs` — range-read `PaintValue` def (use `Grep` to locate).
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/resolved_config.rs`.
  - `crates/slicer-ir/src/slice_ir.rs` (PaintValue + a test).
  - `crates/slicer-ir/tests/paint_value_hash_tdd.rs` (CREATE, small) — alternative location for the unit test.
- Files explicitly out-of-bounds for this step:
  - Production source under `crates/slicer-runtime/src/` (Step 7 territory).
- Expected sub-agent dispatches:
  - "Locate `pub struct ResolvedConfig` and its `#[derive]` line in `crates/slicer-ir/src/resolved_config.rs`; return LOCATIONS (≤ 5 entries)" — purpose: confirm current derives before edit.
  - "Run `cargo test -p slicer-ir paint_value_hash 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: verify Hash impl.
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail with first compile error" — purpose: gate.
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — read sections for `ResolvedConfig` and `PaintValue` only.
- OrcaSlicer refs: none.
- Verification:
  - Hash test passes; `cargo check` clean.
- Exit condition: AC-6, AC-7 satisfied.

### Step 3: Add `variant_chain` to `RegionKey` and `SlicedRegion`; rename `boundary_paint` → `segment_annotations`; add `ConfigId` newtype; add `RegionMapIR.configs` Vec; change `RegionPlan.config` to ConfigId; add `config_for` / `intern_config` accessors

- Task IDs:
  - `TASK-241`
- Objective: the IR shape changes that the rest of the packet depends on.
- Precondition: Step 2 green.
- Postcondition: the new fields exist; the rename is complete in `slicer-ir/src/`; the workspace compiles BUT tests that touch `boundary_paint` may fail until Step 6's rename sweep lands. That's expected — `cargo test -p slicer-ir` is the narrow gate here.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — full read acceptable (this is the file being edited).
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`.
- Files explicitly out-of-bounds for this step:
  - Producer files (Step 4); production call sites (Step 7); tests outside `slicer-ir` (Step 6).
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-ir`; return FACT pass/fail with first error" — purpose: validate type changes compile in isolation.
  - "Run `cargo test -p slicer-ir 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: verify IR tests still pass.
- Context cost: `M`.
- Authoritative docs:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1a" (IR-change subsection).
  - `docs/02_ir_schemas.md` — relevant sections.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — delegate SUMMARY confirming variant-chain shape parity.
- Verification:
  - `cargo check -p slicer-ir` passes.
  - `cargo test -p slicer-ir` passes.
- Exit condition: AC-1, AC-2, AC-3, AC-4 satisfied (in `slicer-ir` isolation; full workspace still fails until Steps 6-7).

### Step 4: Bump `SliceIR` + `RegionMapIR` schema constants to 2.0.0; update every `BuiltinProducer.max_ir_schema` if needed

- Task IDs:
  - `TASK-241`
- Objective: schema version reflects the breaking change.
- Precondition: Step 3 green.
- Postcondition: `SLICE_IR_SCHEMA` and `REGION_MAP_IR_SCHEMA` (or equivalent constants) report `{major: 2, minor: 0, patch: 0}`. Producer constants admit 2.x.
- Files allowed to read:
  - `crates/slicer-runtime/src/builtins/*.rs` (each ≤ 60 lines; read in full as needed).
- Files allowed to edit (≤ 3 per atomic commit; multiple commits acceptable):
  - `crates/slicer-ir/src/slice_ir.rs` (schema constants only).
  - `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`.
  - `crates/slicer-runtime/src/builtins/paint_segmentation_producer.rs`.
  - Other producer files as required by the inventory.
- Files explicitly out-of-bounds for this step:
  - Production call sites (Step 7); tests (Step 6).
- Expected sub-agent dispatches:
  - "Run `rg -nE 'min_ir_schema|max_ir_schema' crates/slicer-runtime/src/builtins/`; return LOCATIONS (≤ 30 entries) so we know which files to edit" — purpose: enumerate producers.
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail" — purpose: gate.
- Context cost: `S` (most files are 1-line edits).
- Authoritative docs:
  - `docs/05_module_sdk.md` §"BuiltinProducer".
- OrcaSlicer refs: none.
- Verification: workspace check clean; `cargo test -p slicer-ir` and `cargo test -p slicer-scheduler` (or whichever crate hosts the schema-version validator) both pass.
- Exit condition: AC-5 satisfied.

### Step 5: Delete `HashablePaintValue` wrapper; rewrite the call site to use `PaintValue` directly

- Task IDs:
  - `TASK-241`
- Objective: AC-9.
- Precondition: Step 2 complete (PaintValue is Hash).
- Postcondition: zero references to `HashablePaintValue`; the previous call site (around `paint_segmentation.rs:117`) compiles using `PaintValue`.
- Files allowed to read:
  - `crates/slicer-core/src/algos/paint_segmentation.rs` — range-read around line 117 (40 lines window).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation.rs`.
- Files explicitly out-of-bounds for this step:
  - Other files under `crates/slicer-core/src/` unless the wrapper definition lives elsewhere (Step 1's LOCATIONS confirms).
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-core`; return FACT pass/fail" — purpose: validate the deletion compiles.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: workspace check clean.
- Exit condition: AC-9 satisfied.

### Step 6: Mechanical rename — `boundary_paint` → `segment_annotations` across ~20 test files

- Task IDs:
  - `TASK-241`
- Objective: complete the implementation-tree rename (doc-tree rename deferred to packet 99).
- Precondition: Step 3 complete (the IR field is renamed; tests will now name the new field).
- Postcondition: zero `boundary_paint` substrings under `crates/` and `modules/` (doc tree exempt).
- Files allowed to read:
  - Each test file flagged by Step 1's LOCATIONS, one at a time. Range-read around the matching line if file is large.
- Files allowed to edit (≤ 3 per commit; batch into multiple commits, one per crate, for `git log --follow`):
  - All flagged test files. Use `Edit` with `replace_all: true` for the literal string `boundary_paint` → `segment_annotations`.
- Files explicitly out-of-bounds for this step:
  - Production source (Step 7 territory).
  - Doc files (packet 99 territory).
  - The roadmap doc (`docs/specs/paint-pipeline-orca-parity-roadmap.md`) — explicitly exempted because it records the historical name.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail" — purpose: gate (any remaining unrenamed reference is a compile error).
  - "Run `! rg -n --glob '!.ralph/specs/91_paint-pipeline-schema-scaffolding/**' --glob '!docs/specs/paint-pipeline-orca-parity-roadmap.md' 'boundary_paint' crates/ modules/`; return FACT pass/fail" — purpose: AC-N3 verification.
- Context cost: `S` (mechanical; the dispatch is the actual work).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: workspace check clean; AC-N3 dispatch returns PASS.
- Exit condition: AC-3 fully satisfied (was partially in Step 3); AC-N3 satisfied.

### Step 7: Migrate 4 production call sites to `region_map.config_for(&key)`

- Task IDs:
  - `TASK-241`
- Objective: AC-8.
- Precondition: Step 6 complete (workspace compiles cleanly; only the `plan.config` references remain as type errors after the `RegionPlan.config: ConfigId` change).
- Postcondition: every read of `plan.config` in `crates/slicer-runtime/src/` is replaced with `region_map.config_for(&key)`; workspace compiles; tests pass.
- Files allowed to read (ranged):
  - `crates/slicer-runtime/src/prepass_slice.rs` — lines 250-300.
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — lines 330-410.
  - `crates/slicer-runtime/src/layer_executor.rs` — lines 770-800.
  - `crates/slicer-runtime/src/dispatch.rs` — lines 1960-2020 (only the two call sites).
- Files allowed to edit (≤ 3 per commit; batch one commit per file for clean history):
  - The four files above.
- Files explicitly out-of-bounds for this step:
  - Any other file under `crates/slicer-runtime/src/`. If Step 1's LOCATIONS surfaced a 5th call site, escalate before editing — the roadmap audit may have missed one.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail with the first compile error" — purpose: gate.
  - "Run `cargo test -p slicer-runtime 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall count" — purpose: validate slicer-runtime tests.
- Context cost: `S` (≤ 6 line changes total).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: workspace check + slicer-runtime tests pass.
- Exit condition: AC-8, AC-N1 satisfied.

### Step 8: Guest WASM rebuild + `--check`

- Task IDs:
  - `TASK-241`
- Objective: AC-N2.
- Precondition: Steps 0-7 complete; workspace check clean.
- Postcondition: every guest `.wasm` reflects the 2.x IR shape.
- Files allowed to read: none.
- Files allowed to edit: none (xtask rebuilds artifacts under `modules/core-modules/*/*.wasm`).
- Files explicitly out-of-bounds for this step: any source file.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-N2.
- Context cost: `S`.
- Authoritative docs: `CLAUDE.md` §"Guest WASM Staleness".
- OrcaSlicer refs: none.
- Verification: dispatch returns PASS.
- Exit condition: AC-N2 satisfied.

### Step 9: AC-10 byte-identical g-code check vs. Step 0 baseline

- Task IDs:
  - `TASK-241`
- Objective: confirm behavior preservation.
- Precondition: Step 8 green.
- Postcondition: post-packet SHA matches the Step 0 baseline SHA. If not, root-cause the diff before continuing.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds for this step: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p91-post.gcode && sha256sum /tmp/p91-post.gcode`; return FACT with the sha256 hash" — purpose: AC-10.
  - On hash mismatch: "Run `diff -u /tmp/p91-baseline.gcode /tmp/p91-post.gcode | head -200`; return SNIPPETS ≤ 100 lines" — purpose: surface the diff for root-causing.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: hash matches Step 0 baseline. If it doesn't, the packet is BLOCKED; do not proceed to Step 10 until either the diff is closed or the mismatch is documented and approved.
- Exit condition: AC-10 satisfied (or escalation if mismatch).

### Step 10: Final acceptance ceremony — `cargo test --workspace`

- Task IDs:
  - `TASK-241`
- Objective: AC-11. Workspace-wide gate per `CLAUDE.md` §Test Discipline rule 2 (this packet's schema bump justifies the full suite).
- Precondition: Steps 0-9 complete.
- Postcondition: workspace test suite green; closure log records final pass count.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds for this step: any.
- Expected sub-agent dispatches:
  - "Run `cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; return FACT pass/fail with per-bucket counts" — purpose: final gate.
- Context cost: `S` (dispatch-only; the dispatched command is long but the implementer's context doesn't absorb the full output).
- Authoritative docs: `CLAUDE.md` §Test Discipline.
- OrcaSlicer refs: none.
- Verification: every bucket reports `test result: ok`.
- Exit condition: AC-11 satisfied; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baseline capture (dispatch). |
| Step 1 | S | Three LOCATIONS dispatches. |
| Step 2 | M | BTreeMap + Hash work in `slicer-ir`. |
| Step 3 | M | The bulk of the IR-shape change. |
| Step 4 | S | Constant updates. |
| Step 5 | S | Wrapper deletion. |
| Step 6 | S | Mechanical sweep (dispatch-driven). |
| Step 7 | S | 4 call-site migrations. |
| Step 8 | S | Guest rebuild dispatch. |
| Step 9 | S | Byte-identical check. |
| Step 10 | S | Workspace test dispatch. |

Aggregate: M (no L step; largest is M).

## Packet Completion Gate

- All 11 steps complete; each exit condition satisfied.
- AC-1 through AC-11 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: pre-packet baseline SHA, post-packet SHA (must match), workspace test pass count, any Hash-impl detail decisions for `ResolvedConfig`.
- `docs/07_implementation_status.md` updated to record `TASK-241` as implemented (delegate the edit).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`; confirm each PASS.
- Confirm `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask build-guests --check`, `cargo test --workspace` all green via sub-agent FACT.
- Verify SHA equality with Step 0 baseline (AC-10).
- Record any remaining risk in the closure log (expect: none — schema scaffolding is behavior-preserving by design).
- Confirm peak context usage stayed under 70%.
