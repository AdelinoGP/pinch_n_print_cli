# Implementation Plan: 254-prime-tower-keys-wipe-tower

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs. This packet carries no `docs/07` task IDs (queue-packet precedent: `task_ids: []`; implementation is recorded against wayfinder ticket 09).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the 13 P02 keys in the wipe-tower manifest

- Task IDs: none (`task_ids: []` queue precedent).
- Objective: `modules/core-modules/wipe-tower/wipe-tower.toml` `[config.schema]` grows from 8 to 21 entries with Orca-parity defaults/bounds.
- Precondition: manifest parses and declares exactly 8 keys today (verified at authoring time).
- Postcondition: manifest parses; the 21-key set with per-key type/default/bounds matches AC-1's list; `toml` round-trip clean.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - full (98 lines)
  - `modules/core-modules/traditional-support/traditional-support.toml` - the `support_interface_flow` percent entry (~lines 74-80) - purpose: percent-declaration shape to mirror
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
- Files explicitly out of bounds:
  - every other manifest; all `crates/**`; all other `modules/**`
- Expected sub-agent dispatches:
  - none (all declaration facts are in `requirements.md` §Verified Grounding)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P02 row (ranged read ~10 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate; never load): the 13 defaults/bounds quoted in the §Per-key parity evidence table
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit=0 after rebuild if stale (manifest feeds the guest fingerprint)
  - the Step-3 schema test lands next; interim check is parse-only: `cargo run -p xtask -- bin-check 2>/dev/null || cargo check -p wipe-tower --target wasm32-unknown-unknown 2>&1 | tail -3 || true` — **do not over-run here**; the real gate is Step 2's AC-1 test plus the guest freshness command above.
- Exit condition: `cargo xtask build-guests` rebuilds the wipe-tower guest without error and `--check` reports exit 0.

### Step 2: Author the schema contract test (AC-1)

- Task IDs: none.
- Objective: a new per-crate test binary pinning the 21-key manifest contract.
- Precondition: Step 1 landed.
- Postcondition: `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd` passes; the test asserts the exact 21-key set (8 existing + 13 new) with per-key type/default/min/max, percent default parsed as `"150%"`, bool defaults as `true/false`, and the `printable_area` required-field still intact.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` (verified present at authoring time) - purpose: schema-parse-and-assert shape to mirror (that crate's `toml = "0.8"` dev-dep is the pattern for wipe-tower's). Re-derive on first use: if the file moved, locate by grep `config.schema` in that tests dir.
  - `docs/spec_packets/254-prime-tower-keys-wipe-tower/packet.spec.md` - AC-1 text only
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` (new)
  - `modules/core-modules/wipe-tower/Cargo.toml` (add `toml = "0.8"` to `[dev-dependencies]`, mirroring part-cooling)
- Files explicitly out of bounds:
  - `modules/core-modules/wipe-tower/src/lib.rs` (no production change in this step); all crates
- Expected sub-agent dispatches:
  - Question: which part-cooling test file asserts the manifest schema and how does it parse the TOML? scope: `modules/core-modules/part-cooling/tests/`; return: `SNIPPETS` (≤ 30 lines); purpose: fixture shape. Skip if the Step-2 implementer already knows it.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - direct read (~80 lines), the plumbing-key standard applied here
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd --test slicer_module_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (the sibling binary guards against accidental manifest-shape drift)
- Exit condition: the new binary passes and the manifest-growth contract is pinned.

### Step 3: Wire `prime_tower_infill_gap` into the scan-line pitch (AC-2) + fallout

- Task IDs: none.
- Objective: replace the hardcoded `y += line_width` purge-path advance with `(percent/100) × line_width`; read the key in `from_config`; update pitch-pinned tests.
- Precondition: key declared (Step 1) — otherwise `ConfigView::from_declared` silently hides it and the read arm would be dead code.
- Postcondition: module generates scan lines at the canonical-formula pitch with **no** config entry (schema default 150% → 0.6 mm at `line_width` 0.4), a `"200%"` config doubles the pitch, a `"110%"` config yields `1.1 × line_width`, and pitch is never below `line_width` for accepted values; the divergence comment (basis `line_width` vs canonical `m_perimeter_width`; depth-refitting out of scope) sits at the compute site.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines 30-60, 143-207, 283-425 only - purpose: wiring target
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` - full (≤ 300 lines; grep-count the pitch pins first)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`
- Files explicitly out of bounds:
  - `crates/**` (no host-side production change; pitch fallout outside the module crate escalates to the coordinator instead of a silent drive-by)
- Blast-radius discipline:
  - New struct field `infill_gap_percent: f32` on `WipeTower` (constructor + `from_config` + any test struct-literal site — grep `WipeTower {` in `modules/core-modules/wipe-tower/` **before editing**; at authoring time the struct is built only via `from_config`, but author the dispatch: Question: list every construction site of the `WipeTower` struct incl. tests? scope: `modules/core-modules/wipe-tower/`; return: `LOCATIONS` (≤ 10)).
  - Pitch fallout: any `wipe_tower_tdd.rs` assertion that hard-counts scan lines or advances (grep `line_width` / spacing math). The authoring survey found the fixture inserts `line_width` and asserts `wt.line_width()` (accessor, pitch-neutral) — the live pins, if any, are scan-line count asserts; update them to formula-derived expectations, and where the old 1.0× pitch is semantically wanted, construct an explicit `"prime_tower_infill_gap": "100%"` config.
- Expected sub-agent dispatches:
  - the blast-radius `LOCATIONS` dispatch above, before editing
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md` - direct read (23 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` - ctor (`m_extra_spacing` init), `align_perimeter`, wipe-path `dy` sites (delegate; never load)
- Verification:
  - `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail across all module binaries
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit=0 (src feeds the guest fingerprint)
- Exit condition: every `wipe-tower` test binary green under the canonical-formula pitch; divergence comment present at the compute site; guest `--check` exit 0.

### Step 4: Scheduler bounds/threading test + runtime leakage arm (AC-3, AC-N1, AC-N2)

- Task IDs: none.
- Objective: pin the scheduler-side behavior (bounds acceptance/rejection, percent-default threading, non-threading of non-percent defaults) and the cross-module non-leakage of the new keys.
- Precondition: Steps 1-3 landed (the manifest must carry the percent key for the bounds index to see it; fixtures may also build the index in-memory — the in-memory shape works even before Step 1, but the file-set ordering keeps verification honest).
- Postcondition: the two new/extended test binaries pass; specifically `extensions["prime_tower_infill_gap"] == Percent(150.0)` under empty source, `prime_tower_brim_width` absent from `extensions`, `99%`/`-2.0` rejected, `110%`/`-1.0`/`3.0` accepted, and a non-wipe-tower module's `ConfigView::from_declared` hides `prime_tower_infill_gap`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` - the three percent tests only - purpose: fixture shape
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - the percent bounds arms only - purpose: rejection-arm shape
  - `crates/slicer-runtime/tests/integration/` - locate the existing module-config leakage file by grep (`from_declared`), read only that file
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/wipe_tower_config_bounds_tdd.rs` (new; flat file — auto-discoverable test binary)
  - `crates/slicer-runtime/tests/integration/<the leakage file found by grep>` (one appended test)
- Files explicitly out of bounds:
  - all production scheduler/IR files (`config_resolution.rs`, `manifest.rs`, `resolved_config.rs`) — the machinery already behaves as AC-3 asserts; if it does not, STOP and report (that falsifies a locked assumption; do not patch scheduler code in this packet)
- Expected sub-agent dispatches:
  - Question: locate the registered integration file asserting cross-module config hiding (the `from_declared` leakage pattern)? scope: `crates/slicer-runtime/tests/integration/`; return: `LOCATIONS` (≤ 5)
  - Question: quote the `LoadedModuleBuilder` + `ConfigFieldEntry` fixture used by `percent_schema_bounds`? scope: `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`; return: `SNIPPETS` (≤ 30 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - direct read; plumbing-key evidence rows in §Per-key parity evidence
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef` (delegate)
- Verification:
  - `cargo test -p slicer-scheduler --test wipe_tower_config_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration -- undeclared_prime_tower_keys_stay_hidden_from_other_modules 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: both pass, with the leakage arm naming at least one non-owner module receiving resolved config with `prime_tower_infill_gap` present in the source map and absent from its view.

### Step 5: Docs regeneration + workspace gates

- Task IDs: none.
- Objective: `docs/15_config_keys_reference.md` regenerated to list the 21 wipe-tower keys; workspace gates green; AC-4 verified by grep.
- Precondition: Steps 1-4 complete.
- Postcondition: gen-config-docs `--check` clean; AC-4's greps pass; check-literals clean; no `docs/01_project_overview.md` prose claims the wipe-tower scan line advances by exactly `line_width` (amend if found).
- Files allowed to read, with ranges when over 300 lines:
  - none directly; run the generator and grep
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (regenerated, never hand-edited)
  - `docs/01_project_overview.md` (only if its grep finds stale prose)
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
  - `rg -q 'prime_tower_infill_gap' docs/15_config_keys_reference.md && rg -q 'filament_tower_ironing_area' docs/15_config_keys_reference.md && echo AC4-PASS` - FACT AC4-PASS
  - `cargo xtask check-literals 2>&1 | tail -3` - FACT pass/fail
  - `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: all gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest-only |
| Step 2 | S | new schema test + dev-dep |
| Step 3 | M | wiring + blast radius + fallout |
| Step 4 | M | two test binaries, two dispatches |
| Step 5 | S | regen + gates |

Aggregate `M`; no row is `L`. Split before activation if aggregate exceeds `M`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**, not from this frozen note (ledger-fact rule). The feature-gap queue's packets carry no TASK row (survey precedent at 234a/253 authoring time); implementation is recorded against wayfinder ticket 09.
- Reconcile reopened/superseded status transitions: none.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the pitch-at-defaults output change must be reconciled against any self-captured baseline suite discovered during implementation (updated in Step 3, listed in its blast-radius result).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.