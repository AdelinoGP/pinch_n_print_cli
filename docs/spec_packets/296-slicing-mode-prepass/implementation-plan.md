# Implementation Plan: 296-slicing-mode-prepass

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Config surface (`SlicingMode` + `ResolvedConfig`)

- Task IDs: `TASK-000`
- Objective: declare the canonical `slicing_mode` spelling with default `regular` and prove an explicit value plus per-object override survive resolution while unknown values reject.
- Precondition: no `slicing_mode` spelling exists under `crates/`/`modules/`/`xtask/` (verified at authoring).
- Postcondition: `ResolvedConfig` carries `slicing_mode` (default `regular`); `close_holes` round-trips via `apply_cli_key`; per-object overlay preserves a non-default; `diagonal` rejects with a `TypeMismatch`-family error naming the key.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - lines `2067-2160` (`SupportType` precedent only)
  - `crates/slicer-ir/src/resolved_config.rs` - lines `1960-1990` (neighbouring `slice_closing_radius` row only)
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs` (new `SlicingMode` enum + `as_canonical_str`/`from_str`)
  - `crates/slicer-ir/src/resolved_config.rs` (one `plain` field + macro arms)
  - `crates/slicer-ir/tests/resolved_config_slicing_mode_tdd.rs` (new: `schema_declares_slicing_mode`, `unknown_slicing_mode_rejected`, per-object override case)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - When the step adds a field to a struct or a new entry to a schema/version constant, list the **struct-literal blast radius** in "Files allowed to edit" — every test/non-test struct-literal site that compiles against the struct today, plus the test files that hard-assert on the constant's old value. Budget this in the step's context cost; do not let a follow-up "cargo check" discover it.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the result inline below.
  - `LOCATIONS` (authoring-time re-derivation, 2026-09-09): `ResolvedConfig` is built only through `declare_resolved_config!` (field + `Default` + `PartialEq` + `Hash` arms generated) and `..Default::default()` / `..ResolvedConfig::default()` rest patterns in tests — no exhaustive literal enumerates every field, so the blast radius is the macro arms plus the whole-struct `PartialEq` drift guard (`explicit_overrides_reach_the_composed_config_for_every_declared_field`, `crates/slicer-ir/src/resolved_config.rs`), which the new tests extend rather than break.
- Expected sub-agent dispatches:
  - Question: `SupportType` plain-field + `from_str` shape to mirror; scope: `crates/slicer-ir/src`; return: `SNIPPETS` (≤3 snippets, ≤30 lines each)
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY (`from_declared` inapplicability only)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (borrow default + domain)
- Verification:
  - `cargo test -p slicer-ir --test resolved_config_slicing_mode_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-1 and AC-N1 pass; `cargo check -p slicer-ir --all-targets` green.

### Step 2: Fill-rule kernel (Positive branch + tests)

- Task IDs: `TASK-000`
- Objective: add the mode-aware slice entry whose `regular`/`even_odd` arms share the current EvenOdd union (byte-identical) and whose `close_holes` arm closes holes via a CCW + `FillRule::Positive` union.
- Precondition: Step 1 green (`SlicingMode` importable from `slicer-ir`).
- Postcondition: `slice_mesh_ex` output unchanged for all existing callers; new entry slices a cube identically under `regular`/`even_odd` and emits zero-hole `ExPolygon`s under `close_holes` on an annulus with outer-area fill within 1%.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/triangle_mesh_slicer.rs` - lines `778-834` (`polygons_to_expolygons` union shape only)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/triangle_mesh_slicer.rs` (additive `slice_mesh_ex_with_mode` + CloseHoles arm; `slice_mesh_ex` delegates with the default mode)
  - `crates/slicer-core/tests/slicing_mode_fill_rule_tdd.rs` (new: `regular_and_even_odd_are_identity_on_valid_mesh`, `close_holes_fills_annulus_hole`)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/prepass_slice.rs` (Step 3 owns the call site)
  - All other `slice_mesh_ex` callers (mesh cross-section, overhang, region mapping, paint — behaviour pinned unchanged)
- Expected sub-agent dispatches:
  - Question: confirm `FillRule::Positive` + `boolean_op_tree_64` import path in this crate's `clipper2-rust` version; scope: `crates/slicer-core/src/triangle_mesh_slicer.rs` + workspace `clipper2-rust`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY (mm↔unit boundary only; test meshes built with `Point2::from_mm`/`Point3` mm constructors)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/TriangleMeshSlicer.hpp` - delegate; never load (borrow Positive domain shape; spiral arm not borrowed)
- Verification:
  - `cargo test -p slicer-core --test slicing_mode_fill_rule_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-core --test triangle_mesh_slicer_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (no baseline drift)
- Exit condition: AC-2 and AC-3 pass; kernel baseline suite green.

### Step 3: Prepass wiring (region-map value reaches the slice)

- Task IDs: `TASK-000`
- Objective: read `cfg.slicing_mode` beside `cfg.slice_closing_radius` in the prepass slice impl and route it into the Step-2 entry, so a region-map `close_holes` changes `SliceIR` while `regular` reproduces the baseline.
- Precondition: Steps 1–2 green (field + kernel landed).
- Postcondition: same annulus object slices with one hole under `regular` and zero holes under `close_holes` through `execute_prepass_slice_single_layer_impl`; default config output byte-identical to pre-packet.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/prepass_slice.rs` - lines `995-1045` (region-map read + slice call only)
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - filtered by `slicing_mode` (harness lookup only; delegate LOCATIONS first)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/prepass_slice.rs` (mode read + mode-entry call; closing round-trip sequencing unchanged)
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` (add `slicing_mode_close_holes_changes_slice_ir` + default-identity case)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/triangle_mesh_slicer.rs` (Step 2 owns the kernel)
  - `crates/slicer-gcode/src/serialize.rs` (Step 4 owns the honest-absence pin)
- Expected sub-agent dispatches:
  - Question: which harness in `algo_prepass_slice_tdd.rs` builds `SliceIR` from a caller-supplied mesh + region-map config; scope: `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`; return: `LOCATIONS` (≤20 entries, one context line each)
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - delegated SUMMARY (prepass-seam ownership only)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` - delegate; never load (borrow the switch-to-fill-rule shape the wiring ports)
- Verification:
  - `cargo test -p slicer-core --test algo_prepass_slice_tdd slicing_mode 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-4 passes; default-identity case passes.

### Step 4: Ledger (DEV-188 + 04/05 + final gates)

- Task IDs: `TASK-000`
- Objective: file the recorded divergences and annotate the queue assets with the fold + owner correction, then run the packet closure gates.
- Precondition: Steps 1–3 green (behaviour proven).
- Postcondition: DEV-188 row live with (a)+(b); 04 P69/`print_sequence` rows point at slicer-core prepass / ticket 124; 05 P69 reads 1 key in at packet 296; padding table untouched; check + clippy + targeted suites green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail 10 rows only (next-ID re-derivation)
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P69 + `print_sequence` rows only
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P69 section only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md` (append DEV-188: (a) Regular-as-EvenOdd simplification for valid meshes, (b) strict unknown-value rejection)
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (P69 owner `layer-planner` → slicer-core prepass; `print_sequence` folded to 124)
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (P69 2→1 at 296; `print_sequence` folded to 124)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (read-only verify for AC-N2; no edit)
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - direct tail read only (ledger re-derivation)
- OrcaSlicer refs: none (no new canonical fact; DEV-188 cites Steps 1–3 evidence).
- Verification:
  - `rg -q 'DEV-188' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `rg -q 'slicing_mode.*slicer-core' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - FACT pass/fail
  - `rg -q '296-slicing-mode' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - FACT pass/fail
  - `rg -q 'slicing_mode' crates/slicer-gcode/src/serialize.rs && exit 1 || exit 0` - FACT pass/fail (AC-N2)
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: every pipe-suffixed AC (AC-1–AC-4, AC-N1–AC-N2) re-dispatched green; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | config surface + new IR test binary |
| Step 2 | M | kernel + new core test binary (largest) |
| Step 3 | S | prepass wiring in existing test binary |
| Step 4 | S | ledger + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
