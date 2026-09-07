# Implementation Plan: 280-bed-mesh-adaptive-placeholders

## Execution Rules

- Work one atomic step at a time; every step maps to wayfinder ticket 53 and `task_ids: []`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every cargo command tees combined output to `target/test-output.log`; inspect that file rather than rerunning truncated output.
- Delegate canonical reads, authoritative-doc checks, cargo runs, and broad inventories with bounded return formats.

## Steps

### Step 1: Declare typed machine-mesh inputs

- Task IDs: `[]` (wayfinder ticket 53)
- Objective: add the four exact manifest declarations and test malformed point lists, probe minimums, defaults, and absence of host-key changes.
- Precondition: re-derive the current `[config.schema]` form and `config.keys()` sweep; confirm no bed-mesh declaration exists.
- Postcondition: all four keys resolve with exact defaults/types; malformed lists and negative values reject with `TypeMismatch`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - schema only.
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - config sweep and test helpers only.
  - `docs/config/host-keys.toml` - grep-only proof that no host edit is needed.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs`
- Files explicitly out of bounds:
  - `docs/config/host-keys.toml`, `serialize.rs`, WIT/IR, Orca source.
- Expected sub-agent dispatches:
  - Question: locate manifest schema fixtures and config resolution helper; scope: machine-gcode-emit; return `LOCATIONS` <=20.
  - Question: confirm canonical declarations; scope: delegated `PrintConfig.cpp`; return `SUMMARY` <=100 words.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated float-list/bounds summary.
  - `docs/specs/orca-feature-gap/map.md` - targeted authoring rules.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegated only.
- Verification:
  - `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd malformed_point_type_rejected 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo xtask gen-config-docs --check` - FACT exit code after generated docs are updated in Step 4.
- Exit condition: schema tests prove four inputs and exact rejection; host-key grep remains empty.

### Step 2: Implement bbox and derived placeholder math

- Task IDs: `[]` (wayfinder ticket 53)
- Objective: collect all XY-complete Move coordinates, apply margin/clamp, compute counts and algorithm, and add scalar site variables.
- Precondition: Step 1's schemas and fixtures pass; locate the existing site-variable map and substitution call.
- Postcondition: AC-1 through AC-5 pass, including small/large defaults, margin, clamps, distance floor, and the product boundary.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - post-pass, site map, substitution symbols only.
  - `crates/slicer-ir/src/slice_ir.rs` - `GCodeCommand::Move` definition only.
  - `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs` - fixtures/tests only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs`
- Files explicitly out of bounds:
  - WIT/IR definitions, host emitter, serializer/padding, Orca source.
- Expected sub-agent dispatches:
  - Question: verify the existing `GCodeCommand::Move` coordinate convention and site-variable insertion point; scope: named files; return `FACT` <=10 lines.
  - Question: summarize canonical formulas and hull inputs; scope: delegated `GCode.cpp::apply_print_config`; return `SUMMARY` <=200 words.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - targeted conversion checklist.
  - `docs/01_system_architecture.md` - targeted post-pass ownership.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegated only.
- Verification:
  - `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd defaults_small_large_bbox 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd margin_expands_bounds 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd algorithm_boundary 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: derived scalar values are deterministic and no vector-index parser or raw-input placeholder exists.

### Step 3: Add flavor fallback and end-to-end substitution proof

- Task IDs: `[]` (wayfinder ticket 53)
- Objective: apply optional Klipper minimums and prove machine-start-G-code resolves every scalar token without warnings.
- Precondition: Step 2's math is green and the existing substitution path is identified.
- Postcondition: AC-6 and AC-7 pass for explicit Klipper, absent flavor, and a complete template.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - optional config lookup and substitution only.
  - `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs` - end-to-end fixture only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/flavor.rs`, host config carriers, WIT/IR, serializer, Orca source.
- Expected sub-agent dispatches:
  - Question: run the focused test binary; scope: exact test command; return `FACT` pass/fail with <=20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - targeted module-site-variable contract.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegated only.
- Verification:
  - `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd klipper_and_missing_flavor 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo test -p machine-gcode-emit --test bed_mesh_adaptive_tdd template_substitution_end_to_end 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT exit code; rebuild if stale before trusting failures.
- Exit condition: explicit Klipper floors counts, absent flavor is graceful, and no unresolved-key warning remains.

### Step 4: Regenerate docs, annotate dispositions, and close gates

- Task IDs: `[]` (wayfinder ticket 53)
- Objective: regenerate config documentation, record owner correction and four retained keys in 04/05, then run packet gates.
- Precondition: Steps 1-3 pass and guest artifacts are fresh.
- Postcondition: generated docs and feature-gap annotations match packet 280; all ACs except AC-8 and closure gates pass; serializer diff is empty.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P46 row only.
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P46 row only.
  - `docs/15_config_keys_reference.md` - grep probes only.
- Files allowed to edit (at most 3 per atomic sub-change):
  - `docs/15_config_keys_reference.md` - generated only.
  - `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md`
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`
- Files explicitly out of bounds:
  - ticket 53 header, `docs/config/host-keys.toml`, `serialize.rs`, production code.
- Expected sub-agent dispatches:
  - Question: re-derive current P46 rows immediately before editing; scope: 04/05 targeted sections; return `SNIPPETS` <=3.
  - Question: run every gate below; scope: exact commands; return `FACT` only.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/map.md` - targeted rules 1-6.
- OrcaSlicer refs: none; prior steps own delegated evidence.
- Verification:
  - `cargo xtask gen-config-docs --check` - FACT exit code.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT exit code.
  - `git diff --stat -- crates/slicer-gcode/src/serialize.rs` - FACT empty.
- Exit condition: every pipe-suffixed AC command except AC-8 passes, docs identify machine-gcode-emit ownership, and no host key/padding/WIT/IR change exists.

### Step 5: Record the ADR-0050 site-variable amendment

- Task IDs: `[]` (wayfinder ticket 53)
- Objective: append the packet-280 amendment to ADR-0050 and add deviation row `D-280-ADR-0050-AMENDED`, proving AC-8.
- Precondition: Steps 1-4 green; re-derive the free `D-` number and the exact §2 sentence at write time (ledger facts, never frozen).
- Postcondition: AC-8 passes via the two greps.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0050-custom-gcode-architecture.md` - section 2 only.
  - `docs/DEVIATION_LOG.md` - head + tail ID sample only.
- Files allowed to edit (at most 3):
  - `docs/adr/0050-custom-gcode-architecture.md`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - all production code, manifests, tests, generated docs, other ADRs.
- Expected sub-agent dispatches:
  - Question: run the AC-8 grep pair; scope: exact command; return `FACT` pass/fail.
- Context cost: `S`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - no new translated source, no header needed.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'ADR-0050-AMENDED' docs/DEVIATION_LOG.md && rg -q 'packet 280' docs/adr/0050-custom-gcode-architecture.md && echo PASS || echo FAIL` - FACT pass.
- Exit condition: AC-8 greps both pass; no other ADR or log row touched.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Typed schemas and rejection tests |
| Step 2 | M | Move bbox, canonical math, scalar site variables |
| Step 3 | S | Flavor fallback and substitution proof |
| Step 4 | S | Generated docs, annotations, closure gates (all ACs except AC-8) |
| Step 5 | S | ADR-0050 amendment + deviation row (AC-8) |

Aggregate: `M`; no step is `L`, so no split is required before activation.

## Packet Completion Gate

- All steps and falsifying exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is not edited by this packet; re-derive any task mapping before closure.
- All four keys remain retained/live; none is returned or shed.
- `packet.spec.md` remains draft until explicit activation and becomes implemented only after acceptance ceremony.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm guest freshness by exit code and inspect `target/test-output.log` for failures rather than rerunning.
- Record remaining risks: moves-bbox rather than first-layer convex hull, custom-move over-probing, and scalar-template migration.
- Confirm context stayed within the standard band; no extended-band escalation is permitted.

All cargo check/clippy/test invocations use `--all-targets` where applicable; each test invocation tees combined output to `target/test-output.log`.
