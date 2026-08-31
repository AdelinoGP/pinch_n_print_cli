# Implementation Plan: raft-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs. This packet carries no `docs/07` task IDs (queue precedent, `task_ids: []`); implementation is recorded against wayfinder ticket 19.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the two raft keys in the manifest + create the schema guard

- Task IDs: none (wayfinder ticket 19)
- Objective: in `tree-support-planner.toml`, add the two net-new `[config.schema]` tables exactly as AC-1 pins them — `raft_contact_distance` (`type = "float"`, `default = 0.1`, `min = 0.0`, no `max`, `display = "Raft Contact Distance"`, `group = "Support"`) and `raft_expansion` (`type = "float"`, `default = 1.5`, `min = 0.0`, no `max`, `display = "Raft Expansion"`, `group = "Support"`) — each with a `description` comment recording the decision-point gap and canonical consumer (the no-max float form mirrors the existing `[config.schema.max_bridge_length]` table in the same manifest). Author the net-new guard test `raft_config_schema_tdd.rs` in the module's `tests/` asserting AC-1's exact tables (including drift-fail naming, AC-N1) and the AC-N2 omission pin (parses `traditional-support-planner.toml` via a relative path and asserts it does NOT declare either key), using part-cooling's `cooling_config_schema_tdd.rs` pattern and adding the `toml = "0.8"` dev-dependency (add-if-absent) to the module's `Cargo.toml` (verified absent at authoring; test-target auto-discovery is on, so no `[[test]]` entry is needed — verified at authoring).
- Precondition: tree green; `cargo xtask build-guests --check` exit 0 before starting (manifest edits are guest-fingerprint inputs).
- Postcondition: the manifest parses; the guard passes; no `src/lib.rs` change — module behavior unchanged (the keys are unread).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - full (~235 lines)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (guard pattern)
  - `modules/core-modules/tree-support-planner/Cargo.toml` - full (dev-dep check)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  - `modules/core-modules/tree-support-planner/tests/raft_config_schema_tdd.rs` (net-new)
  - `modules/core-modules/tree-support-planner/Cargo.toml` (add `toml = "0.8"` to `[dev-dependencies]` — add-if-absent; skip the edit if present, verify don't assume)
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support-planner/src/lib.rs` (Step 2's read-only context; no edits in this packet)
  - `crates/slicer-gcode/src/serialize.rs` (read-only, AC-4)
  - everything outside `modules/core-modules/tree-support-planner/` except the AC-N2 omission-pin read of `traditional-support-planner.toml`
- Blast-radius discipline: TOML manifest tables are additive; no struct literals or constants change, so no blast radius beyond the dev-dep add.
- Expected sub-agent dispatches:
  - Question: after the manifest edit, is the module still loadable with its schema intact (load-or-none check over the real manifest)?; scope: `modules/core-modules/tree-support-planner/`; return: `FACT`; purpose: guard against TOML/table-form mistakes.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - delegated; verify with `cargo xtask gen-config-docs --check` at packet close only (doc impact lands in Step 4).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (evidence captured in `requirements.md` §Per-Key Canonical Evidence).
- Verification:
  - `cargo test -p tree-support-planner --test raft_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0 (manifest fingerprint changed; rebuild if stale, then re-run)
- Exit condition: guard green + guests fresh.

### Step 2: Non-perturbation arms in the module suite

- Task IDs: none (wayfinder ticket 19)
- Objective: author the AC-2 arms in `orca_parity_tdd.rs` (the suite that already drives the planner with config maps and inspects `output.entries()` + `output.raft_plan()` — the `raft_and_interface_layers_emit_expected_entry_count` test is the pattern): run the planner over the overhang fixture with `support_raft_layers = 3` twice — once with the two keys absent, once with `raft_contact_distance = 0.5` and `raft_expansion = 3.0` explicit in the module config — and assert the emitted `Vec<SupportPlanEntry>` and `Option<RaftPlan>` are equal (`SupportPlanEntry` and `RaftPlan` both derive `PartialEq`). The keys are unread in `from_config`, so the comparison must hold byte-for-byte.
- Precondition: Step 1 exit met (manifest declares the two tables, guard green, guests fresh).
- Postcondition: AC-2 green; no `src/lib.rs` change.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` - lines `1-260` (harness helpers + the raft-plan test to mirror; ~1300 lines total — ranged reads only beyond)
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines `1571-1590` (raft `from_config` reads — the keys are unread) and `1723-1731` (`push_raft_plan`)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` (AC-2 arms)
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` (Step 1's surface)
  - `modules/core-modules/tree-support-planner/src/lib.rs` (read-only — the keys stay unread)
  - `OrcaSlicerDocumented/` (delegate any further canonical reads)
- Blast-radius discipline: test-only additions to an existing suite; no production surface.
- Expected sub-agent dispatches: none — the harness is in-suite and the comparison targets are verified (`PartialEq` on both types, confirmed at authoring).
- Context cost: `S`
- Authoritative docs: none beyond the packet's own files.
- OrcaSlicer refs: none — the arms are port-side only (canonical consumers recorded in `requirements.md`).
- Verification:
  - `cargo test -p tree-support-planner --test orca_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-2)
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0
- Exit condition: AC-2 command-green; guests fresh.

### Step 3: Integration arms — bounds/type + CONFIG_BLOCK

- Task IDs: none (wayfinder ticket 19)
- Objective: add the scheduler bounds tests (AC-3: `raft_contact_distance = -0.5` → `OutOfRange`; `raft_expansion = -1.0` → `OutOfRange`; `raft_contact_distance = "abc"` → `TypeMismatch`) against the real `tree-support-planner.toml` manifest, mirroring the existing `rejects_max_bridge_length_below_min` / `rejects_unknown_support_style_value` arm pattern in `config_bounds_enforcement_tdd.rs`. Add the CONFIG_BLOCK tests (AC-4: at defaults zero `raft_contact_distance` / `raft_expansion` lines; explicit `raft_contact_distance = 0.5` → exactly one `; raft_contact_distance = 0.5`; explicit `raft_expansion = 3.0` → exactly one `; raft_expansion = 3.0`) using the runtime binary's per-test config injection (proven at packet 258/259/260 authoring).
- Precondition: Steps 1–2 exit met.
- Postcondition: AC-3 and AC-4 pass against the real manifest and the real pipeline driver; `serialize.rs` untouched (verified — no padding twins).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (~460 lines — bounded read)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror (do not read all ~1040 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (AC-4 pins: no entries gained or lost, no edits)
  - `docs/15_config_keys_reference.md` (generated; Step 4 only)
- Blast-radius discipline: test-only additions to existing binaries; no production surface. If the runtime binary's driver needs a config key it asserts on, use the same per-test config mechanism its current tests use (verified present at authoring for packet 258/259/260's keys).
- Expected sub-agent dispatches:
  - Question: does the CONFIG_BLOCK driver thread explicit module-declared keys into `raw_config`, and do existing arm tests set keys via sidecar/CLI or direct `raw_config` injection?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`; return: `FACT`; purpose: AC-4 arm form.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK - delegated SUMMARY (padding rule), already applied.
- OrcaSlicer refs: none — integration arms are port-side only.
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-3)
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-4)
- Exit condition: AC-3 + AC-4 green.

### Step 4: Regenerate docs + close gates

- Task IDs: none (wayfinder ticket 19)
- Objective: run `cargo xtask gen-config-docs` to regenerate `docs/15_config_keys_reference.md` (the two module-key rows under the `tree-support-planner` owner column); verify `--check` passes, both keys appear, and the deviations block holds exactly 27 data rows (AC-5 — unchanged from the pre-packet count, measured at authoring); then the packet completion gate (AC suite re-run, workspace gates, guests-fresh check).
- Precondition: Steps 1–3 exit met.
- Postcondition: AC-5 exit 0 and keys present; all AC commands green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - NEVER load in full; verify via `rg`/`sed` only (key-presence + deviation-block row count; the doc has no per-module subheadings — rows carry the owner column)
- Files allowed to edit (at only via the generator):
  - `docs/15_config_keys_reference.md` - through `cargo xtask gen-config-docs` only
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (read-only)
  - any hand-edit of the generated doc
- Blast-radius discipline: none (generated doc).
- Expected sub-agent dispatches:
  - Question: does `gen-config-docs --check` pass, do the two raft keys appear in the module-key table under the `tree-support-planner` owner column, and does the deviations block count 27?; scope: `docs/15_config_keys_reference.md`; return: `FACT`; purpose: AC-5.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; key-presence rg-verified.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask gen-config-docs --check && rg -q 'raft_contact_distance' docs/15_config_keys_reference.md && rg -q 'raft_expansion' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| `')" = "27" ]; echo "exit=$?"` - FACT (AC-5)
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0
- Exit condition: AC-5 green; workspace gates green; guests fresh.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | one manifest + one guard file + dev-dep add |
| Step 2 | S | non-perturbation arms in the existing suite |
| Step 3 | M | two test-file additions |
| Step 4 | S | generator + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**, not from this frozen note (ledger-fact rule). The feature-gap queue's packets carry no TASK row (survey precedent at 234a/253–260 authoring time); implementation is recorded against wayfinder ticket 19.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
