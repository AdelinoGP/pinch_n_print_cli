# Implementation Plan: support-support-keys

## Execution Rules

- Work one atomic step at a time; map every step to the wayfinder ticket 20 (queue packet — `task_ids: []`).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the planner-manifest tables and the style-enum correction, with their guards

- Task IDs: `[]` (wayfinder ticket 20 — packet P13)
- Objective: land the nine `tree-support-planner.toml` tables, the eight `traditional-support-planner.toml` tables, the `traditional-support.toml` `support_style` string→enum correction, and the three net-new guard test files that pin them (AC-1, AC-2, AC-N2).
- Precondition: clean tree; the three manifests' current state matches `requirements.md` (verified at authoring: none of the nine keys declared; `support_style` string in `traditional-support.toml`).
- Postcondition: all nine/eight tables declared with the exact AC-1/AC-2 entries (each with `display`, `group = "Support"`, and a `description` comment); `support_style` in `traditional-support.toml` is an enum; the three guard binaries pass; `traditional-support-planner.toml` carries no `raft_first_layer_expansion`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - lines `97-127` (raft cluster, the `raft_first_layer_expansion`/percent-table form) and `160-230` (the `support_object_xy_distance` / `support_style` table forms)
  - `modules/core-modules/traditional-support/traditional-support.toml` - lines `60-95` (`support_interface_flow` percent form + `support_style` table)
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` - lines `34-95` (existing table forms + the `support_threshold_angle` table)
  - `crates/slicer-scheduler/src/manifest.rs` - ranged (`percent` default parse)
  - `modules/core-modules/part-cooling/Cargo.toml` - full (~30 lines; the `toml = "0.8"` dev-dep add-if-absent pattern)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - (subsequent commit: `modules/core-modules/{tree-support-planner,traditional-support-planner,traditional-support}/Cargo.toml` + the three net-new guard test files — 6 files total, two commit batches allowed)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (Step 2)
  - `docs/spec_packets/240-support-raft/` and all other packet directories
  - `crates/slicer-gcode/src/serialize.rs` (read-only context for the no-twins contract)
  - `OrcaSlicerDocumented/...` — delegate; never load
- Blast-radius discipline: none — no Rust struct gains a field, no schema/version constant changes; the manifest edits cannot break compilation (manifests are data), and no existing test asserts these tables' absence. The `traditional-support` `support_style` type change is a declaration-only edit; the module's `smooth_supports` read site (`src/lib.rs` `config.get("support_style")` → `String`) is unchanged — verify with `rg -q 'support_style' modules/core-modules/traditional-support/src/lib.rs` after the edit.
- Expected sub-agent dispatches:
  - Question: does `part-cooling/Cargo.toml` carry the `toml = "0.8"` dev-dependency, and are `toml` dev-deps absent from the three edited modules' `Cargo.toml` files (add-if-absent targets)?; scope: the four `Cargo.toml` files; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegated SUMMARY if the `[config.schema]` `percent`/`enum`/`int` forms need confirmation beyond the grounded tables
  - `docs/15_config_keys_reference.md` — not read; Step 5 regenerates
- OrcaSlicer refs:
  - None needed — canonical type/default/bounds for all 12 keys are already captured in `requirements.md` §Per-Key Canonical Evidence (dispatched reads, 2026-09-01); dispute only via a delegated read
- Verification:
  - `cargo test -p tree-support-planner --test support_main_keys_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p traditional-support-planner --test support_main_keys_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p traditional-support --test support_style_enum_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: all three guard binaries pass, and a `rg` sweep confirms each of the nine keys' `type`/`default`/`min`/`max`/`values` lines matches AC-1/AC-2 exactly.

### Step 2: Wire `enforce_support_layers` in the host producer

- Task IDs: `[]` (wayfinder ticket 20)
- Objective: make `resolve_contact_params` read the typed `config.enforce_support_layers` field (AC-3) — the packet's only behavioral change.
- Precondition: Step 1 green (manifests final; the wiring is independent of them but lands after to keep the tree single-purpose).
- Postcondition: `SupportContactParams.enforce_support_layers` carries the config value; the "no production config source yet" comment is corrected; two unit arms pass (explicit 3 → 3; default → 0); default-path output unchanged (identity — the decision point is off at 0).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - lines `580-670` (`resolve_contact_params` + helpers, the comment to correct) and `737-820` (tests mod fixtures + the `resolve_contact_params_uses_typed_threshold_overlap_percent_and_literal` arm template)
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - lines `168-200` (`SupportContactParams` — the field to verify) and `325-340` (the `force_support` branch)
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs` - lines `170-215` (the two existing enforce arms — context, not edited)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (the field read + comment + the `#[cfg(test)] mod tests` arms — same file)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/overhang_annotation.rs` (the decision point — read-only; its geometry arms already exist)
  - `crates/slicer-ir/src/resolved_config.rs` (the field's home — read-only; the CLI-bound `u32 = 0` default stands)
  - The three manifests (Step 1) and `docs/`
- Blast-radius discipline: `SupportContactParams` is NOT modified (no struct change) — no struct-literal sites. The edit is a value expression inside `resolve_contact_params` plus tests inside the same file's tests mod; no other compilation surface.
- Expected sub-agent dispatches: none — the surface is fully pinned by the authoring survey.
- Context cost: `S`
- Authoritative docs: none beyond the file-local doc comments.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` `detect_overhangs` / `TreeSupport.cpp` `detect_overhangs` — only if a worker disputes the forced-layer semantics already captured in `requirements.md`
- Verification:
  - `cargo test -p slicer-runtime --lib support_analysis_producer 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p slicer-core --test support_overhang_detection_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (existing decision-point arms stay green)
- Exit condition: both commands pass; `rg -n 'enforce_support_layers' crates/slicer-runtime/src/builtins/support_analysis_producer.rs` shows the typed read and the corrected comment, with no `0` hardcode remaining for the field.

### Step 3: Add the scheduler bounds/enum arms and the CONFIG_BLOCK arms

- Task IDs: `[]` (wayfinder ticket 20)
- Objective: prove global-path enforcement of the new declarations (AC-4: `support_type` `"banana"` → `TypeMismatch`; `"tree(auto)"` positive; `raft_first_layer_expansion = -1.0` → `OutOfRange`; `enforce_support_layers = 5001` → `OutOfRange`) and CONFIG_BLOCK honesty (AC-5: default-state presence/absence of the 12 keys; explicit `raft_first_layer_expansion = 3.0` exactly once).
- Precondition: Steps 1-2 green.
- Postcondition: the four AC-4 arms and the AC-5 arms pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (the `load_module_from_paths` setup + the `rejects_unknown_support_style_value` / `out_of_range_support_threshold_angle_is_rejected` arm templates)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + one existing CONFIG_BLOCK assertion to mirror
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (read-only context; AC-5 pins no edits)
  - `crates/slicer-scheduler/src/config_resolution.rs` (read-only; the enforcement machinery is pre-existing)
- Blast-radius discipline: none — test-only additions to existing integration binaries.
- Expected sub-agent dispatches:
  - Question (the driver-read dispatch): does `config_bounds_enforcement_tdd.rs` load the real `tree-support-planner.toml` via `load_module_from_paths` such that the newly declared keys are in the index, and do existing arms already prove `TypeMismatch`/`OutOfRange` shapes (quote the two arms)?; scope: the two files in "Files allowed to read"; return: `FACT`
  - Question: does the CONFIG_BLOCK driver thread explicit module-declared keys into `raw_config` for `serialize_config_block`, and does an explicit `raft_first_layer_expansion` reach the block exactly once via the sorted dump with `emit_config_kv` dedup?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` + `crates/slicer-runtime/src` (raw_config construction) + `crates/slicer-gcode/src/serialize.rs` (`serialize_config_block` / `emit_config_kv` only); return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK — delegated SUMMARY if the no-twins contract needs confirmation
- OrcaSlicer refs: none needed — enforcement semantics are host machinery, not parity reads.
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: both commands pass.

### Step 4: Non-perturbation harness for the tree planner's declared keys

- Task IDs: `[]` (wayfinder ticket 20)
- Objective: prove AC-N1 — the nine declared keys, set explicitly (including `support_type = "tree(auto)"`, the family-consistent value), produce byte-identical `SupportPlanIR` + `RaftPlan` vs absent in a real planner run.
- Precondition: Steps 1-3 green.
- Postcondition: `support_main_keys_nonperturbation_tdd.rs` passes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` - `make_planner_config` (~line 1481) and one full planner-call test (the harness pattern); the file is read-only
  - `modules/core-modules/tree-support-planner/src/lib.rs` - ranged: the `SupportPlanner` entry / `run_support_geometry_with_analysis` / `from_config` config-get regions and `canonical_support_family` (the `support_type` read that makes `"tree(auto)"` the family-consistent explicit value); never browse geometry
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tests/support_main_keys_nonperturbation_tdd.rs` (net-new)
- Files explicitly out of bounds:
  - `orca_parity_tdd.rs` itself (packet 261's planned AC-2 arms claim it; do not edit)
  - `modules/core-modules/traditional-support-planner/` (the traditional non-perturbation claim is structural — unread-ness verified at authoring; no separate run is planned)
- Blast-radius discipline: none — net-new test file.
- Expected sub-agent dispatches:
  - Question: exact fixture source + planner-call shape to replicate (see `design.md` §Expected Sub-Agent Dispatches — the harness replication dispatch); scope: `modules/core-modules/tree-support-planner/tests/` + the planner's public entry surface; return: `SNIPPETS` (≤3, ≤30 lines each)
- Context cost: `M`
- Authoritative docs: none beyond the module's own doc comments.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p tree-support-planner --test support_main_keys_nonperturbation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: command passes with all nine keys explicitly set and byte-identity asserted against the absent-key run.

### Step 5: Regenerate docs, rebuild guests, run the full AC matrix

- Task IDs: `[]` (wayfinder ticket 20)
- Objective: land AC-6 (doc regeneration: module-table rows + deviation block at 26) and close the guest-freshness gate (the three edited manifests are fingerprint inputs).
- Precondition: Steps 1-4 green.
- Postcondition: `docs/15_config_keys_reference.md` regenerated; `cargo xtask gen-config-docs --check` passes; `cargo xtask build-guests --check` returns exit 0 (after a plain rebuild if stale); every pipe-suffixed AC command passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - targeted `rg`/`sed` only (generated; ~1000 lines — never read in full)
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (generated edit — via `cargo xtask gen-config-docs`, never hand-written)
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` (hand-maintained ❌ column — ticket 07 ruling)
  - `docs/config/host-keys.toml` (no default changes)
- Blast-radius discipline: none.
- Expected sub-agent dispatches:
  - Question: after regeneration, do the nine keys appear under the planner owner columns, does the `traditional-support` `support_style` row render as enum with the 7-value list, and does the deviations block count exactly 26 data rows?; scope: `docs/15_config_keys_reference.md` (generated); return: `FACT` (quote the row-count probe)
  - Question: run `cargo xtask build-guests --check` and report the exit code; scope: xtask; return: `FACT` (exit code only; run `cargo xtask build-guests` first if stale)
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask gen-config-docs --check` - FACT exit code
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit code
  - AC-6 probe: `rg -q 'support_type' docs/15_config_keys_reference.md && rg -q 'raft_first_layer_expansion' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| \`')" = "26" ] && echo OK` - FACT OK
  - Full matrix from `requirements.md` §Verification Commands (all ACs) - FACT pass/fail each
- Exit condition: all AC commands pass; deviation-block count re-measured at 26.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Three manifests + three guard files + dev-deps; table-form reads are bounded |
| Step 2 | S | One field read + comment + unit arms; the surface is fully pinned |
| Step 3 | M | Two integration binaries; driver-read dispatches first |
| Step 4 | M | Planner-harness replication; the parity-file dispatch is the risk |
| Step 5 | S | Regeneration + gate commands only |

Aggregate: `M` — no step is L; no split required before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` — N-A for queue packets (`task_ids: []`); the crosswalk question is re-derived at completion time per the ledger-fact rule (wayfinder ticket 20 is the implementation record).
- Reconcile reopened/superseded status transitions: none.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the first-implementation guard pattern and the `support_type` global-path strictness change are the two behavior-adjacent edges; both pinned by ACs.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
