# Implementation Plan: 255-wipe-tower-geometry-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs. This packet carries no `docs/07` task IDs (queue-packet precedent: `task_ids: []`; implementation is recorded against wayfinder ticket 10).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 2".

## Steps

### Step 1: Declare the 12 P03 keys in the wipe-tower manifest

- Task IDs: none (`task_ids: []` queue precedent).
- Objective: `modules/core-modules/wipe-tower/wipe-tower.toml` `[config.schema]` grows by 12 entries with Orca-parity defaults/bounds; `wipe_tower_max_purge_speed` is not declared.
- Precondition: manifest parses; the wipe-tower guest builds; the current key count is re-derived from disk (21 if packet 254 landed, 8 + 254's exact delta otherwise).
- Postcondition: manifest parses; per-key type/default/bounds match AC-1's list; the three pre-existing `ORCA_CONFIG_PADDING` keys keep their padding entries host-side (no host edit in this step).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - full (≤ 250 lines)
  - `modules/core-modules/seam-placer/seam-placer.toml` - lines 24-40 only - purpose: enum `values = [...]` + string default shape to mirror
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
- Files explicitly out of bounds:
  - every other manifest; all `crates/**` (padding table included); all other `modules/**`
- Expected sub-agent dispatches:
  - none (declaration facts are in `requirements.md` §Per-key parity evidence)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P03 row (ranged read ~10 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate; never load): the 12 defaults/bounds quoted in the §Per-key parity evidence table
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit=0 after rebuild if stale (manifest feeds the guest fingerprint; an only-tree-support-planner-stale result is pre-existing — rebuild and proceed)
  - interim parse check only; the real gate is Step 2's AC-1 test plus the freshness command above (packet 254's Step-1 pattern)
- Exit condition: `cargo xtask build-guests` rebuilds the wipe-tower guest without error and `--check` reports exit 0.

### Step 2: Author the schema contract test (AC-1)

- Task IDs: none.
- Objective: a per-crate test binary pinning the manifest contract including the union base.
- Precondition: Step 1 landed.
- Postcondition: `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd` passes; the test asserts the re-derived key union (8 pre-existing + P02-from-254 if landed + the 12 declared here) with per-key type/default/min/max, percent defaults parsed as `"100%"`, bool defaults as `true/false`, the enum domain `["rectangle", "cone", "rib"]` with default `"rib"`, and `wipe_tower_max_purge_speed` asserted **absent**.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - purpose: schema-parse-and-assert shape to mirror (that crate's `toml = "0.8"` dev-dep is the pattern). Re-derive on first use: if the file moved, locate by grep `config.schema` in that tests dir.
  - `docs/spec_packets/255-wipe-tower-geometry-keys/packet.spec.md` - AC-1 text only
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` (new, or extended if packet 254 created it first)
  - `modules/core-modules/wipe-tower/Cargo.toml` (add `toml = "0.8"` to `[dev-dependencies]`, only if 254 has not)
- Files explicitly out of bounds:
  - `modules/core-modules/wipe-tower/src/lib.rs` (no production change in this step); all crates
- Expected sub-agent dispatches:
  - none beyond the listed reads; the schema-test shape is verified in-tree
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - direct read (~80 lines), the plumbing-key standard applied here
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd --test slicer_module_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (the sibling binary guards against accidental manifest-shape drift)
- Exit condition: the new binary passes and the manifest-growth contract is pinned.

### Step 3: Wire `wipe_tower_extra_flow` into the scan-line flow factor (AC-2) + fallout

- Task IDs: none.
- Objective: replace the hardcoded scan-line `flow_factor: 1.0` with `(percent/100)`; read the key in `from_config`; update any flow-pinned tests.
- Precondition: key declared (Step 1) — otherwise `ConfigView::from_declared` silently hides the read (packet 254 invariant).
- Postcondition: module generates scan lines with `flow_factor == extra_flow_percent/100` — identity 1.0 with no config entry, 1.5 for `"150%"`, 2.0 for `"200%"` — on **both** `process()` and `run_finalization()` outputs; travel entity stays 0.0 and prime keeps 0.0/1.0 in all cases; the percent-semantics comment sits at the compute site.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines 30-60, 143-207, 283-425, 540-568 only - purpose: wiring target
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` - full (≤ 300 lines; grep-count the flow pins first)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_extra_flow_tdd.rs` (new) or `wipe_tower_tdd.rs` (if fallout demands)
- Files explicitly out of bounds:
  - `crates/**` (no host-side production change; fallout outside the module crate escalates to the coordinator instead of a silent drive-by)
- Blast-radius discipline:
  - New struct field `extra_flow_factor: f32` on `WipeTower` (constructor + `from_config` + any test struct-literal site — grep `WipeTower {` in `modules/core-modules/wipe-tower/` **before editing**; at authoring time the struct is built only via `from_config`, but author the dispatch).
  - Expected fallout: **none at defaults** (identity factor). The authoring survey found no `flow_factor` pin in module or emitter tests; the Step-3 dispatch re-derives it. If a pin appears outside `crates/slicer-gcode`+module tests, STOP and report (falsifies a locked assumption).
- Expected sub-agent dispatches:
  - Question: does any test pin scan-line `flow_factor == 1.0` (or copy the literal into expectations)? scope: `modules/core-modules/wipe-tower/` + `crates/slicer-gcode/tests/`; return: `LOCATIONS` (≤ 10); purpose: fallout list, run before editing.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md` - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` - ctor (`m_extra_flow` init), `toolchange_Wipe` (the flow multiplier this wiring mirrors), `set_toolchange`/`save_on_last_wipe` (delegate; never load)
- Verification:
  - `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail across all module binaries
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit=0 (src feeds the guest fingerprint)
- Exit condition: every `wipe-tower` test binary green; flow-factor semantics comment present at the compute site; guest `--check` exit 0.

### Step 4: Scheduler bounds/threading/enum test + runtime leakage arm (AC-3, AC-N1, AC-N2)

- Task IDs: none.
- Objective: pin the scheduler-side behavior (bounds acceptance/rejection, percent-default threading for both percent keys, non-threading of non-percent defaults, enum membership) and the cross-module non-leakage of the new keys.
- Precondition: Steps 1-3 landed.
- Postcondition: the two test binaries pass; specifically `extensions["wipe_tower_extra_flow"] == Percent(100.0)` and `extensions["wipe_tower_extra_spacing"] == Percent(100.0)` under empty source, `wipe_tower_cone_angle`/`wipe_tower_wall_type` absent from `extensions`, `99%`/`301%` rejected (error names the key), `100%`/`300%` accepted, `"hexagon"` rejected and `"rib"` accepted for `wipe_tower_wall_type`, and a non-wipe-tower module's `ConfigView::from_declared` hides `wipe_tower_extra_flow`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` - the three percent tests only - purpose: fixture shape
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - the percent bounds arms only - purpose: rejection-arm shape
  - `crates/slicer-runtime/tests/integration/` - locate the existing module-config leakage file by grep (`from_declared`), read only that file
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/wipe_tower_p03_config_bounds_tdd.rs` (new; flat file — auto-discoverable test binary)
  - `crates/slicer-runtime/tests/integration/<the leakage file found by grep>` (one appended test)
- Files explicitly out of bounds:
  - all production scheduler/IR files (`config_resolution.rs`, `manifest.rs`, `resolved_config.rs`, `feedrate.rs`) — the machinery already behaves as AC-3 asserts; if it does not, STOP and report (that falsifies a locked assumption; do not patch scheduler code in this packet)
- Expected sub-agent dispatches:
  - Question: locate the registered integration file asserting cross-module config hiding (the `from_declared` leakage pattern)? scope: `crates/slicer-runtime/tests/integration/`; return: `LOCATIONS` (≤ 5)
  - Question: quote the `LoadedModuleBuilder` + `ConfigFieldEntry` fixture used by `percent_schema_bounds`? scope: `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`; return: `SNIPPETS` (≤ 30 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - direct read; plumbing-key evidence rows in §Per-key parity evidence
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate)
- Verification:
  - `cargo test -p slicer-scheduler --test wipe_tower_p03_config_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration -- undeclared_p03_wipe_tower_keys_stay_hidden_from_other_modules 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: both pass, with the leakage arm naming at least one non-owner module receiving resolved config with `wipe_tower_extra_flow` present in the source map and absent from its view.

### Step 5: Docs regeneration + workspace gates

- Task IDs: none.
- Objective: `docs/15_config_keys_reference.md` regenerated to list the wipe-tower keys including the 12 new ones; workspace gates green; AC-4 verified by grep.
- Precondition: Steps 1-4 complete.
- Postcondition: gen-config-docs `--check` clean; AC-4's greps pass; check-literals clean; no doc prose claims the port's purge flow is a fixed 1.0.
- Files allowed to read, with ranges when over 300 lines:
  - none directly; run the generator and grep
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (regenerated, never hand-edited)
  - `docs/01_project_overview.md` (only if its grep finds stale flow prose)
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
  - `rg -q 'wipe_tower_wall_type' docs/15_config_keys_reference.md && rg -q 'wipe_tower_extra_flow' docs/15_config_keys_reference.md && echo AC4-PASS` - FACT AC4-PASS
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT pass/fail
  - `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: all gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest-only |
| Step 2 | S | new schema test (+ dev-dep if 254 hasn't landed) |
| Step 3 | M | wiring + fallout dispatch |
| Step 4 | M | two test binaries, two dispatches |
| Step 5 | S | regen + gates |

Aggregate `M`; no row is `L`. Split before activation if aggregate exceeds `M`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**, not from this frozen note (ledger-fact rule). The feature-gap queue's packets carry no TASK row (survey precedent at 234a/253/254 authoring time); implementation is recorded against wayfinder ticket 10.
- Reconcile reopened/superseded status transitions: none.
- `packet.spec.md` is ready for `status: implemented` once packet 254's schema test settles the union base.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm `cargo xtask build-guests --check` exit 0 with the wipe-tower guest rebuilt; record any still-stale unrelated guest in the ceremony notes as pre-existing (authoring-time baseline: `tree-support-planner-guest`).
- Record remaining packet-local risk: the +2 CONFIG_BLOCK lines at defaults must be reconciled against any self-captured baseline suite discovered during implementation (updated in the owning step, listed in its blast-radius result).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.