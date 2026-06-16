# Implementation Plan: 108_perimeter-special-modes-and-seam

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: T-070/T-071 — `extra_perimeters` bonus consumer

- Task IDs:
  - `T-070` — Register `extra_perimeters` config key
  - `T-071` — Honour `extra_perimeters` config bonus
- Objective: register the config key in both manifests; read via `_config.get("extra_perimeters")` in `run_perimeters`; compute `loop_number = wall_count + extra_perimeters - 1` per OrcaSlicer.
- Precondition: P102 + P104 + P105 landed; workspace builds clean.
- Postcondition: AC-1 passes.
- Files allowed to read:
  - Both perimeter modules' `lib.rs` (range-read `run_perimeters` head + wall loop count).
- Files allowed to edit (≤ 3 per sub-step):
  - 1a (manifests + reference): `modules/core-modules/classic-perimeters/classic-perimeters.toml`, `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `docs/15_config_keys_reference.md`.
  - 1b (consumers + test): both perimeter `lib.rs` + `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs` (NEW).
- Files explicitly out-of-bounds: all other source files.
- Expected sub-agent dispatches:
  - "FACT: confirm OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1569 carries `loop_number = wall_loops + surface.extra_perimeters - 1`; single-line FACT."
  - "Run `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd`; FACT pass/fail."
- Context cost: `S`
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-070/T-071 rows.
- OrcaSlicer refs: `PerimeterGenerator.cpp:1569` (delegate FACT).
- Verification: `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1 green.

### Step 2: T-072/T-073 — Narrow-island smaller_perimeter handling + T-074b/c/d non-planar emission

- Task IDs:
  - `T-072` — Register narrow-island keys
  - `T-073` — Narrow-island handling
  - `T-074b` — Detect non-planar; emit `LoopType::NonPlanarShell`
  - `T-074c` — `SurfaceGroup.shell_count` override
  - `T-074d` — Skip thin-wall/gap-fill/`infill_areas` for non-planar regions
- Objective: register 3 narrow-island keys; implement narrow-island detection + smaller-width emission; implement non-planar branch at the head of `run_perimeters` that emits `shell_count` NonPlanarShell walls and skips thin-wall/gap-fill/infill.
- Precondition: Step 1 exit condition met.
- Postcondition: AC-2 + AC-3 + AC-N1 pass.
- Files allowed to read:
  - Both perimeter modules' `lib.rs` (range-read `run_perimeters`).
  - `crates/slicer-sdk/src/views.rs` — confirm `surface_group()` accessor signature (added by P104).
- Files allowed to edit (≤ 3 per sub-step):
  - 2a (manifests): both perimeter `.toml`, `docs/15_config_keys_reference.md`.
  - 2b (narrow-island consumer + test): both perimeter `lib.rs` + `crates/slicer-runtime/tests/integration/narrow_island_smaller_perimeter_tdd.rs` (NEW).
  - 2c (non-planar consumer + test): both perimeter `lib.rs` (re-edit) + `crates/slicer-runtime/tests/integration/nonplanar_shell_emission_tdd.rs` (NEW).
- Files explicitly out-of-bounds: all other source files; P104's `surface_group()` accessor already exists (do not re-edit).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1611-1628 for narrow-island `smaller_ext_perimeter_flow`; return SUMMARY ≤ 150 words."
  - "Run `cargo test -p slicer-runtime --test integration narrow_island_smaller_perimeter_tdd nonplanar_shell_emission_tdd`; FACT pass/fail per test."
- Context cost: `M`
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-072..T-074d rows; `docs/specs/overhang-pipeline-restructuring.md` (for D-11 context).
- OrcaSlicer refs: `PerimeterGenerator.cpp:1611-1628` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-runtime --test integration narrow_island_smaller_perimeter_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test integration nonplanar_shell_emission_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-2 + AC-3 + AC-N1 green.

### Step 3: T-080/T-081 — Sharp-corner seam threshold

- Task IDs:
  - `T-080` — Replace every-vertex candidate with sharp-corner threshold
  - `T-081` — Register `seam_candidate_angle_threshold_deg`
- Objective: add `generate_sharp_corner_seam_candidates` helper to `slicer-core::perimeter_utils`; register the config key; migrate both perimeter modules to call the new helper.
- Precondition: Step 2 exit condition met.
- Postcondition: AC-4 passes.
- Files allowed to read:
  - `crates/slicer-core/src/perimeter_utils.rs` (range-read existing `generate_seam_candidates`).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/perimeter_utils.rs`
  - `crates/slicer-core/tests/sharp_corner_seam_threshold_tdd.rs` (NEW)
  - Both perimeter `lib.rs` migration (counts as one file conceptually; if it must be two, do classic first as 3a and arachne as 3b).
- Files explicitly out-of-bounds: `seam-placer` (Step 4); manifests (handled with Step 2's reference doc).
- Expected sub-agent dispatches:
  - "Find call sites of `generate_seam_candidates` across the workspace; LOCATIONS ≤ 10 entries."
  - "Run `cargo test -p slicer-core --test sharp_corner_seam_threshold_tdd`; FACT pass/fail."
- Context cost: `S`
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-080/T-081 rows.
- OrcaSlicer refs: optional `OrcaSlicerDocumented/src/libslic3r/Feature/SeamPlacer/SeamPlacer.cpp` SUMMARY ≤ 100 words for angle-threshold default; default to 30° if SUMMARY doesn't specify.
- Verification: `cargo test -p slicer-core --test sharp_corner_seam_threshold_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-4 green; both perimeter modules call the new helper.

### Step 4: T-082/T-083/T-P98-SEAM — Painted seam consumption + seam-placer audit + integration

- Task IDs:
  - `T-082` — Audit seam-placer for dense-candidate dependency
  - `T-083` — Document seam-planner-default interaction
  - `T-P98-SEAM` — Consume painted seam_enforcer/blocker
- Objective: add `apply_seam_paint_bias` helper in `slicer-core::perimeter_utils`; call it from `seam-placer/src/lib.rs` (or from perimeter modules before commit — implementer's choice per design); audit seam-placer for empty-list robustness; document seam-planner interaction; supersede `D-98-SEAM-NO-CONSUMER`.
- Precondition: Step 3 exit condition met.
- Postcondition: AC-5 + AC-N2 pass; deviation supersession landed.
- Files allowed to read:
  - `modules/core-modules/seam-placer/src/lib.rs` — full (audit target).
  - `crates/slicer-core/src/perimeter_utils.rs` — confirm where to land the helper.
  - `docs/DEVIATION_LOG.md` — `D-98-SEAM-NO-CONSUMER` entry.
- Files allowed to edit (≤ 3 per sub-step):
  - 4a (helper): `crates/slicer-core/src/perimeter_utils.rs`.
  - 4b (consumer + test): `modules/core-modules/seam-placer/src/lib.rs` + `crates/slicer-runtime/tests/integration/painted_seam_enforcer_blocker_tdd.rs` (NEW).
  - 4c (docs): `docs/05_module_sdk.md` (audit + interaction notes) + `docs/DEVIATION_LOG.md`.
- Files explicitly out-of-bounds: perimeter modules' `lib.rs` (no further edits in this step); `seam-planner-default/src/lib.rs` (T-083 deliverable is doc-based, not source).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/Feature/SeamPlacer/SeamPlacer.cpp for sharp-corner candidate selection + painted seam consumption; return SUMMARY ≤ 200 words, no code."
  - "Run `cargo test -p slicer-runtime --test integration painted_seam_enforcer_blocker_tdd`; FACT pass/fail per case (positive enforcer + blocker exclusion + AC-N2 NoCandidates)."
- Context cost: `M`
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-082/T-083/T-P98-SEAM rows; `docs/DEVIATION_LOG.md`.
- OrcaSlicer refs: `Feature/SeamPlacer/SeamPlacer.cpp` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-runtime --test integration painted_seam_enforcer_blocker_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `rg -q 'D-.*-SEAM-CONSUMED' docs/DEVIATION_LOG.md` — exit 0.
- Exit condition: AC-5 + AC-N2 green; D-98-SEAM-NO-CONSUMER superseded; T-082/T-083 doc paragraphs present.

### Step 5: T-077 — extra_perimeters_on_overhangs real consumer

- Task IDs:
  - `T-077` — Register config + wire real consumer (consumes data from P106+P107)
- Objective: register `extra_perimeters_on_overhangs`; wire the consumer code path in both perimeter modules that reads `region.overhang_areas()` (returning non-empty post-P106+P107) and adds one extra perimeter inside those areas; AC fixture exercises both non-empty (overhang ramp) and empty (flat region) paths on the same layer.
- Precondition: Step 4 exit condition met; P106 + P107 are `status: implemented` (data flow available).
- Postcondition: AC-6 passes.
- Files allowed to read:
  - Both perimeter modules' `lib.rs` — range-read the extra-perimeter loop.
- Files allowed to edit (≤ 3 per sub-step):
  - 5a (manifests): `modules/core-modules/classic-perimeters/classic-perimeters.toml`, `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`.
  - 5b (consumers + test): both perimeter `lib.rs` + `crates/slicer-runtime/tests/integration/extra_perimeters_on_overhangs_tdd.rs` (NEW; fixture asserts N+1 walls in overhang region + N walls in flat region on the same layer).
- Files explicitly out-of-bounds: P104's `overhang_areas()` accessor (do not re-edit); P106/P107 source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test integration extra_perimeters_on_overhangs_tdd`; FACT pass/fail per case."
- Context cost: `S`
- Authoritative docs: `docs/specs/perimeter-modules-orca-parity-roadmap.md` T-077 row; `docs/specs/overhang-pipeline-restructuring.md` (predecessor data flow).
- OrcaSlicer refs: none directly — the OrcaSlicer behavior `extra_perimeters_on_overhangs` is the implementation target; the SUMMARY for it is captured by P105's investigation if needed.
- Verification:
  - `cargo test -p slicer-runtime --test integration extra_perimeters_on_overhangs_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-6 green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Manifest register + small consumer. |
| Step 2 | M | Two overrides + two new integration tests. |
| Step 3 | S | Helper + migration + one helper test. |
| Step 4 | M | Helper + seam-placer audit + new integration test + doc edits. |
| Step 5 | S | Deferred consumer + small test + deviation registration. |

Aggregate context cost: `M`. No step `L`. Per-step file edit count ≤ 3 (sub-steps where needed).

## Packet Completion Gate

- All five steps complete; each exit condition met.
- AC-1 through AC-6 + AC-N1 + AC-N2 all PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each T-070..T-P98-SEAM entry — via worker dispatch.
- `packet.spec.md` ready to move `draft` → `implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands green.
- Record T-082 audit findings in the closure log (was seam-placer robust to empty input, or did it need a fix? what was the fix?).
- Record T-077 fixture verification: confirm AC-6 fixture produces N+1 walls inside `region.overhang_areas()` and N walls outside on the same layer.
- Confirm implementer's peak context usage < 70%.
