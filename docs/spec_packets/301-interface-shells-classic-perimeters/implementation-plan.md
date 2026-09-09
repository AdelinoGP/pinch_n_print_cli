# Implementation Plan: 301-interface-shells-classic-perimeters

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: ResolvedConfig flag + map arm + lock row + TOML row

- Task IDs: `TASK-000`
- Objective: Declare `interface_shells` (default `false`) as a bool `extract_bool` row beside the `bridge_no_support` row, with a `cli` binding of the same name; add the `to_config_map` `ConfigValue::Bool` arm; extend the doc-lock's `resolved_bool` with the key; add the `[resolved_config]` TOML row so the regen picks it up.
- Precondition: `crates/slicer-ir/src/resolved_config.rs` lines 2035–2040 declare the neighbouring bool rows; `to_config_map` lines 215–218 show the bool-arm shape; the lock's `resolved_bool` matches the TOML table row-for-row.
- Postcondition: The field resolves at the canonical default, round-trips through `to_config_map`, composes through the ticket-126 overlay arms the macro emits, the lock passes, and the regen lists the key.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `2035-2040`
  - `crates/slicer-ir/src/resolved_config.rs` - lines `215-218`
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - lines `55-63`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`
  - `docs/config/host-keys.toml`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`
  - `crates/slicer-runtime/src/...`
  - `modules/...`
  - `crates/slicer-gcode/src/serialize.rs` (padding twin untouched — read-only spelling witness)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not a Rust-level struct-field addition (the `declare_resolved_config!` macro emits the field) — but the macro emits a whole-struct `PartialEq` the ticket-126 drift guard covers automatically; every sampled `ResolvedConfig {` literal uses `..Default::default()`, so no literal names the new field. No separate blast-radius dispatch needed.
- Expected sub-agent dispatches:
  - Question: did `cargo test -p slicer-ir` pass after the row lands; scope: `crates/slicer-ir`; return: `FACT`
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY (macro row shape; host-prepass keys need no manifest)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (canonical default `false`, coBool — already verified at authoring)
- Verification:
  - `rg -q 'cli "interface_shells"' crates/slicer-ir/src/resolved_config.rs && rg -q '"interface_shells".into()' crates/slicer-ir/src/resolved_config.rs 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test unit host_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: Both greps green; the lock test passes; `--check` passes with the TOML row present; no other file touched.

### Step 2: Neighbour-source gate in the prepass + tests

- Task IDs: `TASK-000`
- Objective: Add `resolve_interface_shells` (same first-timeline-entry `region_map.config_for` pattern as `resolve_shell_counts`), precompute one collective union per slice index over all timelines' same-slice polys, switch Pass 1's `upper_polys` / `lower_polys` reads to the union when the timeline's flag resolves `false` (keeping the same-timeline shape at `true`), and land the four unit tests (`interface_shells_false_uses_collective_upper_cover`, `interface_shells_true_marks_bottom_on_other_material`, `interface_shells_defaults_are_identity_single_region`, `interface_shells_disjoint_objects_unaffected`).
- Precondition: Step 1 field resolves via `region_map.config_for`; `compute_region_updates` lines 351–520 hold the Pass-1 read shapes; the `rect` / `slice_with` helpers (lines 1137–1164) exist.
- Postcondition: AC-2/AC-3/AC-4/AC-N1 green; `true` reproduces the pre-packet baseline byte-for-byte (pinned); repeated `false` runs are identical (per-slice union is order-independent).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `351-520`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1081-1106`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1137-1164`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` - delegate; never load
  - `crates/slicer-ir/...`
  - `modules/...`
  - `crates/slicer-gcode/src/serialize.rs`
  - Draft packet `299-*/` files
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant is added in this step — the gate reuses the existing fill buckets and shell-index stamps. No blast-radius dispatch.
- Expected sub-agent dispatches:
  - Question: exact `detect_surfaces_type` same-region-vs-collective arms (upper source, lower overhang + extra-bottom construction); scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`
  - Question: did `cargo test -p slicer-runtime --lib interface_shells` pass; scope: `crates/slicer-runtime`; return: `FACT`
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering — gate sees BASE timelines)
  - `docs/02_ir_schemas.md` - delegated SUMMARY (five-way partition invariant the bypass preserves)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; never load (`detect_surfaces_type` arms; `discover_vertical_shells` merge explicitly NOT borrowed; `PrintObject::infill` scatter explicitly NOT borrowed)
- Verification:
  - `cargo xtask build-guests --check` - FACT exit code 0 (the `resolved_config.rs` edit feeds the guest build; never `rg 'STALE:'`)
  - `cargo test -p slicer-runtime --lib interface_shells_false_uses_collective_upper_cover 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib interface_shells_true_marks_bottom_on_other_material 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib interface_shells_defaults_are_identity_single_region 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib interface_shells_disjoint_objects_unaffected 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test executor prepass_slice_and_shell 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (no existing-shell churn)
- Exit condition: All four named tests pass; the existing executor shell tests stay green (or every red carries a measured-justification fixture update, map test discipline — never a weakened assertion); top and bottom directions assert separately (no upper/lower swap).

### Step 3: Ledger (DEV-192 + 04/05 + final gates)

- Task IDs: `TASK-000`
- Objective: File the DEV-192 row (spiral-conjunct, vertical-merge, perimeter-mask, and reslice non-borrows per design.md Locked Assumptions) and annotate the 04 tier table + 05 packet list so P76 reads 1-in at packet 301; prove doc freshness.
- Precondition: Steps 1–2 green; DEV-192 verified absent from the log and all drafts (re-derive `max(DEV-*)` before writing).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-192 with (a)+(b)+(c)+(d) clauses; 04's Multimaterial-advanced row points at packet 301; 05's P76 section reads 1 key in; `cargo xtask gen-config-docs --check` is green.
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
  - `rg -q 'DEV-192' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `rg -q 'interface_shells' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: DEV-192 greps; ledger rows read 1-in at 301; all three verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One macro row + map arm + lock arm + TOML row |
| Step 2 | M | Gate + four tests in one file |
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
