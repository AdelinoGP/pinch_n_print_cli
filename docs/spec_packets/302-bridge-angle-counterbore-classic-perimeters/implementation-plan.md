# Implementation Plan: 302-bridge-angle-counterbore-classic-perimeters

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: ResolvedConfig rows + map arms + lock arms + TOML rows

- Task IDs: `TASK-000`
- Objective: Declare `bridge_angle` (default `0.0`) as a float `extract_float` row and `counterbore_hole_bridging` (default `"none"`) as a string `extract_string` row with `wire_type: Some("enum")` + `values: &["none", "partiallybridge", "sacrificiallayer"]`, each with a `cli` binding of the same name; add both `to_config_map` arms (`ConfigValue::Float` / `ConfigValue::String`); extend the doc-lock's `resolved_num` with `bridge_angle` and `resolved_str` with `counterbore_hole_bridging`; add both `[resolved_config]` TOML rows so the regen picks them up.
- Precondition: `crates/slicer-ir/src/resolved_config.rs` lines 2035–2036 declare the neighbouring `bridge_no_support` bool row; `to_config_map` lines 215–218 show the bool-arm shape; the lock's `resolved_num` / `resolved_str` match the TOML table row-for-row.
- Postcondition: Both fields resolve at their canonical defaults, round-trip through `to_config_map`, compose through the ticket-126 overlay arms the macro emits, the lock passes, and the regen lists both keys.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - lines `2035-2036`
  - `crates/slicer-ir/src/resolved_config.rs` - lines `215-218`
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - lines `55-74`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`
  - `docs/config/host-keys.toml`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`
  - `crates/slicer-runtime/src/...`
  - `modules/...`
  - `crates/slicer-gcode/src/serialize.rs` (no padding twin for either key — read-only absence witness)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Not a Rust-level struct-field addition (the `declare_resolved_config!` macro emits the fields) — but the macro emits a whole-struct `PartialEq` the ticket-126 drift guard covers automatically (`overlay_onto_drift_guard` drives `host_config_keys`, so both rows are auto-covered); the manual `PartialEq for ResolvedConfig` + `Hash for ResolvedConfig` impls name every field by hand (see `bridge_no_support` at `== other.bridge_no_support` and `.hash(state)`), so both impls gain one arm each in this step. Every sampled `ResolvedConfig {` literal uses `..Default::default()`, so no literal names the new fields. No separate blast-radius dispatch needed — the two manual impls are in the edit file.
- Expected sub-agent dispatches:
  - Question: did `cargo test -p slicer-ir` pass after the rows land; scope: `crates/slicer-ir`; return: `FACT`
- Context cost: `S` (split an L step)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY (macro row shape; host-prepass keys need no manifest)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (canonical defaults `0` / `chbNone` — already verified at authoring)
- Verification:
  - `rg -q 'cli "bridge_angle"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "counterbore_hole_bridging"' crates/slicer-ir/src/resolved_config.rs && rg -q '"counterbore_hole_bridging".into()' crates/slicer-ir/src/resolved_config.rs 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test unit host_keys 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail
- Exit condition: All three greps green; the lock test passes; `--check` passes with both TOML rows present; no other file touched.

### Step 2: External-bridge direction override + lib tests

- Task IDs: `TASK-000`
- Objective: Add `resolve_bridge_angle` (same first-timeline-entry `region_map.config_for` pattern as `resolve_shell_counts`), apply the `> 0` verbatim overwrite of `region.bridge_orientation_deg` after the existing `update_external_bridge_orientation(region, lower_layer_slices)` call (0 keeps the detected value), and land the lib tests (`bridge_angle_overrides_detected_orientation`, `bridge_angle_zero_keeps_detected_orientation`, `counterbore_defaults_leave_regions_untouched`, `counterbore_unknown_spelling_falls_back_to_none`, `counterbore_hole_free_spans_unaffected`).
- Precondition: Step 1 field resolves via `region_map.config_for`; the bridge-gate neighbourhood (`object_layers`/`lower_layer_polygons` maps, `gate_bridge_areas_by_unsupported_span` + `update_external_bridge_orientation` call order) holds at lines 200–260; the `rect` / `slice_with` helpers (lines 1225–1252) exist.
- Postcondition: AC-2/AC-5/AC-N1/AC-N2 green; `0` reproduces the pre-packet detection byte-for-byte (pinned); repeated override runs are identical (per-region pure function of the resolved float).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `200-260`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1169-1194`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1225-1252`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` - delegate; never load
  - `crates/slicer-ir/...`
  - `modules/...`
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-core/src/algos/prepass_slice.rs` (consumed read-only — orientation helpers; the overwrite lands in the runtime prepass)
  - Draft packet `299-*/` files
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant is added in this step — the overwrite reuses the existing `bridge_orientation_deg` bucket. No blast-radius dispatch.
- Expected sub-agent dispatches:
  - Question: exact `process_external_surfaces` top/bottom custom-angle arms (gate shape, absolute vs relative application); scope: `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`; return: `SUMMARY`
  - Question: did `cargo test -p slicer-runtime --lib bridge_angle` pass; scope: `crates/slicer-runtime`; return: `FACT`
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering — overwrite sees BASE timelines)
  - `docs/02_ir_schemas.md` - delegated SUMMARY (bridge bucket contract the overwrite preserves)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` - delegate; never load (`process_external_surfaces` arms; `relative_bridge_angle` + `align_infill_direction_to_model` explicitly NOT borrowed)
- Verification:
  - `cargo xtask build-guests --check` - FACT exit code 0 (the `resolved_config.rs` edit feeds the guest build; never `rg 'STALE:'`)
  - `cargo test -p slicer-runtime --lib bridge_angle 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib counterbore_defaults_leave_regions_untouched 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib counterbore_unknown_spelling_falls_back_to_none 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --lib counterbore_hole_free_spans_unaffected 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test executor prepass_slice_and_shell 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (no existing-shell churn)
- Exit condition: All five named tests pass; the existing executor shell tests stay green (or every red carries a measured-justification fixture update, map test discipline — never a weakened assertion); the overwrite wins over detection on a non-degenerate span (no order swap).

### Step 3: Counterbore hole-bearing span stage + executor tests

- Task IDs: `TASK-000`
- Objective: Add `author_counterbore_bridge_spans` after `gate_internal_bridge_sites` (unsupported computed against the committed lower layer via the existing `object_layers` + `lower_layer_polygons` maps; holes from the region's own polys; already-bridged spans subtracted; `filled` authors whole uncovered spans, `partial` authors rim spans only, `none`/unknown authors nothing; authored spans get a `detect_bridging_direction_deg` orientation against the raw lower contours), resolve the enum per timeline through the `resolve_shell_counts` pattern, and land the two stepped-hole executor tests (`counterbore_filled_authors_hole_interior_as_bridge`, `counterbore_partial_leaves_hole_interior_unbridged`).
- Precondition: Step 2 overwrite green; the stepped-hole fixture shape (lower full slab, upper holed slab over air) builds through the `cuboid_mesh` / `make_plan` / `make_region_map` helpers; hole contours survive slicing into `SlicedRegion.polygons` as `holes`.
- Postcondition: AC-3/AC-4 green; authored spans carry non-default orientations; `filled` and `partial` disagree exactly on the hole interior (mode-swap pinned).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `200-260`
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `611-704`
  - `crates/slicer-runtime/tests/executor/prepass_slice_and_shell_tdd.rs` - lines `1-120`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs`
  - `crates/slicer-runtime/tests/executor/prepass_slice_and_shell_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` - delegate; never load
  - `crates/slicer-ir/...`
  - `modules/...`
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-core/src/algos/prepass_slice.rs` (consumed read-only — `detect_bridging_direction_deg` + span-gate helpers)
  - Draft packet `299-*/` files
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or schema constant is added in this step — the stage reuses the existing `bridge_areas` bucket plus the `is_bridge` flag. No blast-radius dispatch.
- Expected sub-agent dispatches:
  - Question: exact `process_no_bridge` island separation and filled-vs-partial handling (mode distinction, coverage role); scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SUMMARY`
  - Question: did `cargo test -p slicer-runtime --test executor counterbore` pass; scope: `crates/slicer-runtime`; return: `FACT`
- Context cost: `M` (split an L step)
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated SUMMARY (five-way partition invariant the authored spans must preserve)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate; never load (`process_no_bridge` mode distinction; detector math + anchor-band shaping explicitly NOT borrowed)
  - `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` + `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; never load (extra-fill recovery + slice-union explicitly NOT borrowed)
- Verification:
  - `cargo test -p slicer-runtime --test executor counterbore_filled_authors_hole_interior_as_bridge 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test executor counterbore_partial_leaves_hole_interior_unbridged 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test executor prepass_slice_and_shell 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (no existing-shell churn)
- Exit condition: Both named tests pass; hole interior is covered in `filled` and uncovered in `partial` on the same fixture (no mode swap); the existing executor shell tests stay green (or every red carries a measured-justification fixture update).

### Step 4: Ledger (DEV-193 + 04/05 + final gates)

- Task IDs: `TASK-000`
- Objective: File the DEV-193 row (relative-angle, align-offset, slice-union, and reslice non-borrows per design.md Locked Assumptions) and annotate the 04 tier table + 05 packet list so P77 reads 2-in at packet 302; prove doc freshness.
- Precondition: Steps 1–3 green; DEV-193 verified absent from the log and all drafts (re-derive `max(DEV-*)` before writing).
- Postcondition: `docs/DEVIATION_LOG.md` carries DEV-193 with (a)+(b)+(c)+(d) clauses; 04's two Quality/Bridging rows point at packet 302; 05's P77 section reads 2 keys in; `cargo xtask gen-config-docs --check` is green.
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
  - None (no new oracle reads; all borrows landed in Steps 2–3)
- Verification:
  - `rg -q 'DEV-193' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `rg -q 'counterbore_hole_bridging' docs/15_config_keys_reference.md && rg -q 'bridge_angle' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: DEV-193 greps; ledger rows read 2-in at 302; all three verification commands pass; no queue-count change; no new fog graduated, nothing ruled out of scope.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two macro rows + map arms + lock arms + TOML rows |
| Step 2 | M | Override + five lib tests in one file |
| Step 3 | M | Stage + orientations + two executor tests |
| Step 4 | S | Ledger row + annotations + gates |

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
