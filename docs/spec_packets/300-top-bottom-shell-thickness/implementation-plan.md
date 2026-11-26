# Implementation Plan: 300-top-bottom-shell-thickness

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: ResolvedConfig thickness fields

- Task IDs: `TASK-000`
- Objective: Declare `top_shell_thickness` (default `0.6`) and `bottom_shell_thickness` (default `0.0`) as `f32` `extract_float` rows beside the `top_shell_layers` / `bottom_shell_layers` rows, with `cli` bindings of the same names.
- Precondition: `crates/slicer-ir/src/resolved_config.rs` lines 1892–1897 declare the neighbouring layer-count rows.
- Postcondition: Both fields resolve at canonical defaults, round-trip through `to_config_map`, and compose through the ticket-126 overlay arms the macro emits.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `1892-1897`
  - `crates/slicer-ir/src/resolved_config.rs` - lines `1833-1835`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`
  - `crates/slicer-runtime/...`
  - `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not a struct-field addition at the Rust level (the `declare_resolved_config!` macro emits the fields) — but the macro emits a whole-struct `PartialEq` the ticket-126 drift guard covers automatically; no hand-written struct literals name `ResolvedConfig` fields outside the macro. No separate blast-radius dispatch needed.
- Expected sub-agent dispatches:
  - Question: did `cargo test -p slicer-ir --lib resolved_config` pass; scope: `crates/slicer-ir`; return: `FACT`
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY (macro row shape; host-prepass keys need no manifest)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (canonical defaults top `0.6` / bottom `0.0`, coFloat min 0)
- Verification:
  - `rg -q 'cli "top_shell_thickness"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "bottom_shell_thickness"' crates/slicer-ir/src/resolved_config.rs 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: Both `cli` rows grep; `cargo test -p slicer-ir --lib resolved_config` passes; no other file touched.

### Step 2: Thickness arms in the Pass-2 walks + tests

- Task IDs: `TASK-000`
- Objective: Thread the thickness pair from `resolve_shell_counts` into `compute_region_updates` and extend both Pass-2 seed walks past the count cap while the print-z distance stays below thickness; land the three unit tests (`shell_thickness_extends_projection_past_count`, `shell_thickness_defaults_are_identity`, `shell_thickness_bypasses_locked_paths`).
- Precondition: Step 1 fields resolve via `region_map.config_for`; `compute_region_updates` lines 351–520 hold the count-cap walk shapes.
- Postcondition: Non-zero thickness extends the top walk past `k_top` (comparing seed `print_z` distance) and the bottom walk past `k_bot` (comparing neighbour `bottom_z` distance); `0` keeps the pre-packet count-only shape; locked paths bypass per AC-N1.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `351-520`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1081-1106`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1137-1160`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` - delegate; never load
  - `crates/slicer-ir/...`
  - `modules/...`
  - Draft packet `299-*/` files
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant is added in this step — the arms reuse the existing fill buckets and shell-index stamps. No blast-radius dispatch.
- Expected sub-agent dispatches:
  - Question: exact `discover_horizontal_shells` top/bottom loop arms (count floor + `||` thickness + `EPSILON`); scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct range read (mm-domain `EPSILON` handling; no unit conversion in the arm)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; never load (`discover_horizontal_shells` arms; `discover_vertical_shells` first/last-layer arms as the same-shape precedent; `PrintObject::infill` scatter explicitly NOT borrowed)
- Verification:
  - `cargo test -p slicer-runtime --lib shell_thickness_extends_projection_past_count 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib shell_thickness_defaults_are_identity 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib shell_thickness_bypasses_locked_paths 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: All three named tests pass; the `0`-thickness runs match the pre-packet baseline; top and bottom extensions assert separately (no print-z/bottom-z swap).

### Step 3: Ledger (DEV-191 + 04/05 + final gates)

- Task IDs: `TASK-000`
- Objective: File the DEV-191 row (scatter non-borrow, spiral-gate non-borrow, reslice-invalidation non-borrow) and annotate the 04 tier table + 05 packet list so P74 reads 2-in at packet 300; prove doc freshness.
- Precondition: Steps 1–2 green; DEV-191 verified absent from the log and all drafts (re-derive `max(DEV-*)` before writing).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-191 with (a)+(b)+(c) clauses; 04's two Strength/Top-bottom-shells rows point at packet 300; 05's P74 section reads 2 keys in; `cargo xtask gen-config-docs --check` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - tail rows only (ID-convention sample)
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`
  - `crates/...`
  - `modules/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant is added in this step. No blast-radius dispatch.
- Expected sub-agent dispatches:
  - Question: did `cargo xtask gen-config-docs --check` pass; scope: repo root; return: `FACT`
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - direct tail read (row shape + ID convention)
- OrcaSlicer refs:
  - None (no new oracle reads; all borrows landed in Step 2)
- Verification:
  - `rg -q 'DEV-191' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `rg -q 'top_shell_thickness' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: DEV-191 greps; ledger rows read 2-in at 300; all three verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two macro rows beside existing neighbours |
| Step 2 | M | Two walk arms + three tests in one file |
| Step 3 | S | Ledger row + annotations + gates |

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
