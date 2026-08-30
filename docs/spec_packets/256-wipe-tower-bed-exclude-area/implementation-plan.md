# Implementation Plan: 256-wipe-tower-bed-exclude-area

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs. This packet carries no `docs/07` task IDs (queue-packet precedent: `task_ids: []`; implementation is recorded against wayfinder ticket 11).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 2".

## Steps

### Step 1: Declare `bed_exclude_area` in the wipe-tower manifest

- Task IDs: none (`task_ids: []` queue precedent).
- Objective: `modules/core-modules/wipe-tower/wipe-tower.toml` `[config.schema]` grows by exactly one entry — `bed_exclude_area`, `type = "float-list"`, no `default`, no `min`/`max`, `display = "Excluded bed area"`, `group = "Printer"`, `advanced = true` — placed directly after `[config.schema.printable_area]`.
- Precondition: manifest parses; the wipe-tower guest builds; the current key count is re-derived from disk (8 if 254/255 have not landed; plus their keys if they have).
- Postcondition: manifest parses; the new entry matches AC-1's shape exactly; no other entry changed.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - full (≤ 150 lines)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
- Files explicitly out of bounds:
  - every other manifest; all `crates/**`; all other `modules/**`
- Expected sub-agent dispatches:
  - none (the declaration facts are in `requirements.md` §In Scope / §Verified Grounding)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P04 row (ranged read ~7 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate; never load): the `bed_exclude_area` definition quoted in §Per-key parity evidence
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit=0 after rebuild if stale (the manifest feeds the guest fingerprint; ticket 101: guests embed config key names)
- Exit condition: manifest parses, guest `--check` exits 0 after any needed rebuild.

### Step 2: Author the module contract test binary (AC-1, AC-2) + the AC-3 ingest arm

- Task IDs: none.
- Objective: one test binary pinning the declaration shape, the four wiring cases, and the Orca point-string ingest.
- Precondition: Step 1 landed.
- Postcondition: `cargo test -p wipe-tower --test wipe_tower_bed_exclude_area_tdd` passes (schema shape per AC-1 + the four AC-2 wiring cases: absence-identity; corner-inside → `Err` naming `bed_exclude_area` and the corner; tower-outside → `Ok`; empty/odd/<6 → no exclusion, no error), and `cargo test -p wipe-tower --test bed_bounds_tdd` passes with the new AC-3 arm: Orca point-string exclusion (`["0x0","20x0","20x20","0x20"]`) + tower corner at (10,10) → rejected, placed next to `orca_point_string_bed_is_parsed_not_silently_defaulted` whose shape it mirrors. The behaviour asserts may be written TDD-red here and go green in Step 3.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` - full - purpose: `config_from_pairs` helper + point-string fixture shape to reuse; this file GAINS the AC-3 arm
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full - purpose: schema-parse-and-assert shape (that crate's `toml = "0.8"` dev-dep is the pattern)
  - `docs/spec_packets/256-wipe-tower-bed-exclude-area/packet.spec.md` - AC-1/AC-2/AC-3 text only
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/tests/wipe_tower_bed_exclude_area_tdd.rs` (new)
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` (one appended AC-3 test)
  - `modules/core-modules/wipe-tower/Cargo.toml` (add `toml = "0.8"` to `[dev-dependencies]`)
- Files explicitly out of bounds:
  - `modules/core-modules/wipe-tower/src/lib.rs` (no production change in this step); all crates
- Expected sub-agent dispatches:
  - none beyond the listed reads; both fixture shapes are verified in-tree
- Context cost: `S` (test authored here may reference behaviour landed in Step 3 — write the behaviour-facing asserts in this step and expect them red until Step 3, per TDD; the schema-shape asserts go green immediately)
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - direct read (~80 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_bed_exclude_area_tdd --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (schema asserts green; behaviour asserts may be red until Step 3; the existing point-string regressions must stay green)
- Exit condition: the schema-shape asserts pass; the AC-3 arm compiles in `bed_bounds_tdd.rs`; the behaviour asserts compile and fail for the *expected* reason (no wiring yet), or pass if Step 3 already ran.

### Step 3: Wire the exclusion check into `run_finalization` (AC-2, AC-3) + fallout

- Task IDs: none.
- Objective: read `bed_exclude_area` in `from_config` into a new `WipeTower` field; after the existing bed-polygon corner loop in `run_finalization`, add the exclusion corner check with the fatal message naming the key; keep everything else identical.
- Precondition: key declared (Step 1) — otherwise `ConfigView::from_declared` silently hides the read.
- Postcondition: the module's behaviour matches AC-2's four cases, on the shared live path; the degenerate-value contract (empty/odd/<6 → no exclusion, no error) holds; on-edge counts as inside (shared `point_in_polygon`); the canonical-asymmetry comment (tower rectangle vs object hulls) sits at the check site.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines 30-60, 82-108, 141-208, 470-560 only
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` - full (< 300 lines) - purpose: do not break the two ticket-100 regression tests
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_bed_exclude_area_tdd.rs` (behaviour asserts from Step 2 go green here)
- Files explicitly out of bounds:
  - `crates/**` production (fallout outside the module crate escalates to the coordinator instead of a silent drive-by)
- Blast-radius discipline:
  - New struct field `bed_exclude_area: Vec<(f32, f32)>` on `WipeTower` (constructor + `from_config` + any test struct-literal site — grep `WipeTower {` in `modules/core-modules/wipe-tower/` **before editing**; at authoring time the struct is built only via `from_config`, but author the dispatch).
  - Struct-literal churn gate: any `WipeTower {` literal in new test code must carry a `..` rest or an `// exhaustive: <reason>` waiver (`docs/21_data_defaults_and_fixtures.md`; `cargo xtask check-literals` enforces).
  - Expected fallout: **none at defaults** (absence-identity). In-tree tests never supply `bed_exclude_area`, so no existing fixture changes.
- Expected sub-agent dispatches:
  - Question: does any test pin the absence of extra fatal returns from `run_finalization` or count its error paths? scope: `modules/core-modules/wipe-tower/` + `crates/slicer-runtime/tests/`; return: `LOCATIONS` (≤ 10); purpose: fallout list, run before editing.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md` - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Print.cpp` - `Print::validate` / `layered_print_cleareance_valid` (the fatal collision-risk semantics mirrored in the message) - delegate; never load
- Verification:
  - `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail across all module binaries (bed_bounds_tdd, finalization_live_tdd, slicer_module_binding_tdd, wipe_tower_tdd, the new binary)
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit=0 (`src/lib.rs` feeds the guest fingerprint)
- Exit condition: every `wipe-tower` test binary green; guest `--check` exit 0; degenerate-value contract pinned by test.

### Step 4: Non-leakage test (AC-N1)

- Task IDs: none.
- Objective: pin that the new declaration leaks no config into non-owner modules.
- Precondition: Step 1 landed (the declaration is what binds delivery).
- Postcondition: `cargo test -p slicer-scheduler --test wipe_tower_p04_binding_tdd` passes: a `bed_exclude_area` value present in the resolved source map is absent from a non-wipe-tower module's `ConfigView` and present in the wipe-tower module's own view.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` - the module-config / `LoadedModuleBuilder` fixture tests only - purpose: fixture shape
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/wipe_tower_p04_binding_tdd.rs` (new; flat file — auto-discoverable test binary)
- Files explicitly out of bounds:
  - all production scheduler/IR files (`config_resolution.rs`, `manifest.rs`, `execution_plan.rs`, `slice_ir.rs`, `resolved_config.rs`) — the machinery already behaves as AC-N1 asserts; if it does not, STOP and report (that falsifies a locked assumption; do not patch scheduler code in this packet)
- Expected sub-agent dispatches:
  - Question: quote the `LoadedModuleBuilder` + `ConfigFieldEntry` fixture used by a from_declared/hiding test? scope: `crates/slicer-scheduler/tests/integration/`; return: `SNIPPETS` (≤ 30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - direct read; the plumbing-key evidence row in §Per-key parity evidence
- OrcaSlicer refs:
  - none (scheduler-side only)
- Verification:
  - `cargo test -p slicer-scheduler --test wipe_tower_p04_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: passes, with the test naming a non-owner module receiving resolved config where `bed_exclude_area` is in the source map and absent from its view.

### Step 5: Docs regeneration + workspace gates

- Task IDs: none.
- Objective: `docs/15_config_keys_reference.md` regenerated to list `bed_exclude_area` under owner `wipe-tower`; workspace gates green; AC-4 verified by grep.
- Precondition: Steps 1-4 complete.
- Postcondition: gen-config-docs `--check` clean; AC-4's grep passes; the Orca-deviations table gains **no** row for the key (no default → no comparand); check-literals clean.
- Files allowed to read, with ranges when over 300 lines:
  - none directly; run the generator and grep
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (regenerated, never hand-edited)
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` — no row is expected; filing one requires human sign-off surfaced first (ticket 02 standard)
- Expected sub-agent dispatches:
  - cargo/xtask runs and greps delegated per context discipline
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - regeneration target; grep-verify only
- OrcaSlicer refs:
  - none (docs-only step)
- Verification:
  - `cargo xtask gen-config-docs --check 2>&1 | tail -3` - FACT pass/fail
  - `rg -q 'bed_exclude_area' docs/15_config_keys_reference.md && echo AC4-PASS` - FACT AC4-PASS
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT pass/fail
  - `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: all gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest-only |
| Step 2 | S | new test binary (+ dev-dep) |
| Step 3 | M | wiring + fallout dispatch |
| Step 4 | S | one scheduler test binary, one dispatch |
| Step 5 | S | regen + gates |

Aggregate `M`; no row is `L`. Split before activation if aggregate exceeds `M`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**, not from this frozen note (ledger-fact rule). The feature-gap queue's packets carry no TASK row (survey precedent at 234a/253/254/255 authoring time); implementation is recorded against wayfinder ticket 11.
- Reconcile reopened/superseded status transitions: none.
- `packet.spec.md` is ready for `status: implemented` once the schema test settles the union base (Step 2 re-derives it from disk).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm `cargo xtask build-guests --check` exit 0 with the wipe-tower guest rebuilt; record any still-stale unrelated guest in the ceremony notes as pre-existing (authoring-time baseline: `tree-support-planner-guest`, per ticket 99's note and packets 254/255's ceremony rows).
- Record remaining packet-local risk: none expected — no CONFIG_BLOCK change at defaults (no schema default); user-supplied values newly appearing in CONFIG_BLOCK are intended round-trip behaviour.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.
- Record the reduced-semantics gap (object hulls vs tower rectangle) as the packet's standing follow-up note in the completion report — it is not closed by this packet.