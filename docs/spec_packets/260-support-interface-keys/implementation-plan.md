# Implementation Plan: support-interface-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs. This packet carries no `docs/07` task IDs (queue precedent, `task_ids: []`); implementation is recorded against wayfinder ticket 18.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the four keys in both manifests + create both schema guards

- Task IDs: none (wayfinder ticket 18)
- Objective: in `traditional-support.toml` and `tree-support.toml`, update the existing `[config.schema.support_interface_spacing]` default to `0.5` (leaving `min = 0.0`, `max = 2.0`, `display`, `group` unchanged), add a divergence comment above `[config.schema.support_bottom_interface_spacing]` (kept `min = -1.0`, PnP `-1` mirror — canonical has no sentinel; user ruling 2026-08-31) and add the two net-new tables exactly as AC-1 pins them (`support_interface_pattern` enum with values `["auto", "rectilinear", "concentric", "rectilinear_interlaced", "grid"]`, default `"auto"`; `support_interface_loop_pattern` bool default `false`; both with `display` + `group = "Support"`), mirroring each other table-for-table so no family asymmetry is introduced. Author the net-new guard tests `support_config_schema_tdd.rs` in both modules' `tests/` asserting AC-1's exact tables (including drift-fail naming, AC-N2), using part-cooling's `cooling_config_schema_tdd.rs` pattern and adding the `toml = "0.8"` dev-dependency (add-if-absent) to both modules' `Cargo.toml`.
- Precondition: tree green; `cargo xtask build-guests --check` exit 0 before starting (manifest edits are guest-fingerprint inputs).
- Postcondition: both manifests parse; both guards pass; no `src/lib.rs` change yet — module behavior unchanged (the toml default change alone is inert until Step 2's const change, and tests that read the toml are the guards only).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/traditional-support.toml` - full (~103 lines)
  - `modules/core-modules/tree-support/tree-support.toml` - full (~92 lines)
  - `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (guard pattern)
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - lines `216-226` (enum-table form)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/tree-support/tree-support.toml` plus one new guard file: `modules/core-modules/tree-support/tests/support_config_schema_tdd.rs` (split the traditional guard file + both Cargo.toml dev-dep edits into this step's sub-steps if the 3-edit cap binds — the cap counts files edited per step; run a first pass on both tomls, a second pass on the two new guard files + the two Cargo.tomls)
  - `modules/core-modules/traditional-support/tests/support_config_schema_tdd.rs` (net-new)
  - `modules/core-modules/traditional-support/Cargo.toml`, `modules/core-modules/tree-support/Cargo.toml` (add `toml = "0.8"` to `[dev-dependencies]` — add-if-absent; skip the edit if present, verify don't assume)
- Files explicitly out of bounds:
  - `modules/core-modules/traditional-support/src/lib.rs` and `modules/core-modules/tree-support/src/lib.rs` (Step 2's surface)
  - `crates/slicer-gcode/src/serialize.rs` (read-only, AC-5)
  - everything outside `modules/core-modules/{traditional-support,tree-support}/`
- Blast-radius discipline: TOML manifest tables are additive (one existing default changes but no test parses the toml's default yet — the guards are net-new); no struct literals or constants change, so no blast radius beyond the dev-dep adds.
- Expected sub-agent dispatches:
  - Question: after the manifest edits, is each module still loadable with its schema intact (load-or-none check over the real manifests)?; scope: `modules/core-modules/{traditional-support,tree-support}/`; return: `FACT`; purpose: guard against TOML/table-form mistakes.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - delegated; verify with `cargo xtask gen-config-docs --check` at packet close only (doc impact lands in Step 4).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (evidence captured in `requirements.md` §Per-Key Canonical Evidence).
- Verification:
  - `cargo test -p traditional-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p tree-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0 (manifest fingerprint changed; rebuild if stale, then re-run)
- Exit condition: both guards green + guests fresh.

### Step 2: Align the fallback const + fixture + lock the divergence with test arms

- Task IDs: none (wayfinder ticket 18)
- Objective: change `DEFAULT_INTERFACE_SPACING_MM` from `0.4` to `0.5` and correct both "matching OrcaSlicer's `support_interface_spacing` default of 0.4 mm" comments (→ 0.5) in `traditional-support/src/lib.rs` and `tree-support/src/lib.rs` (the fallback const is the default used when the key is absent from the module view — AC-2's "absent == explicit 0.5" arm proves the aligned default reaches `pitches_mm`). Update the `orca-matched-config.json` fixture value 0.4 → 0.5 and re-measure any `support_family_closure.rs` interface-count expectation that pinned the 0.4 pitch (justify from the pitch math — the interface scan-line count scales with `(0.4 + flow_spacing) / (0.5 + flow_spacing)` on the counted layers). Author three test arm groups in both module suites: AC-2 (absent key path count == explicit-0.5 count, strictly sparser than explicit-0.4), AC-3 (bottom `-1` == bottom absent == bottom set-to-top-value — the retained mirror witness), AC-N1 (explicit `support_interface_pattern` = concentric/grid/rectilinear_interlaced and `support_interface_loop_pattern` = true produce byte-identical interface paths to absent keys). Inventory AND fix every other site that pinned the old 0.4 default in the same pass (e2e goldens or other fixtures — the design's `[FWD]` inventory decides; packet 257/258 re-baseline precedent applies with measured justification).
- Precondition: Step 1 exit met (manifests declare the four tables, guards green, guests fresh).
- Postcondition: AC-2, AC-3, AC-N1 green in both suites; `support_family_closure.rs` green with re-measured expectations; no other site references the 0.4 top-interface default.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/src/lib.rs` - lines `46-56` (const + comment) and `330-430` (`pitches_mm`, mirror branch, density calls)
  - `modules/core-modules/tree-support/src/lib.rs` - lines `50-60` (const + comment), `260-360` (from_config reads), and the `pitches_mm` region near line `742`
  - `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` - full (~415 lines, the change surface)
  - `modules/core-modules/tree-support/tests/tree_support_tdd.rs` - full (~340 lines)
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - lines `150-200` + the interface-count assertion sites (ranged reads only; ~800 lines total)
  - `crates/slicer-runtime/tests/fixtures/support-family/orca-matched-config.json` - full (small)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/src/lib.rs` + `modules/core-modules/tree-support/src/lib.rs` (const + comment only)
  - `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` + `modules/core-modules/tree-support/tests/tree_support_tdd.rs` (AC-2/3/N1 arms)
  - `crates/slicer-runtime/tests/fixtures/support-family/orca-matched-config.json` + `crates/slicer-runtime/tests/integration/support_family_closure.rs` (fixture + re-measured expectations; plus any e2e golden the inventory surfaces — re-baseline with measured justification)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (read-only, AC-5)
  - `modules/core-modules/{traditional-support,tree-support}/Cargo.toml` (Step 1's surface)
  - `OrcaSlicerDocumented/` (delegate any further canonical reads)
- Blast-radius discipline: the const change is a private module constant consumed only in `from_config` fallback — no struct literals, no schema. The fixture value change propagates into `support_family_closure.rs` expectations only (inventory confirms); both are in the edit list. The retained mirror branch is NOT edited (AC-3 witnesses it).
- Expected sub-agent dispatches:
  - Question: which assertions in `support_family_closure.rs` (and any e2e goldens under `crates/slicer-runtime/tests/e2e/`) depend on the top-interface default pitch, and what are their current expected values?; scope: `crates/slicer-runtime/tests/`; return: `LOCATIONS` (≤20) + `FACT`; purpose: pre-baked fallout list for this step.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK contract - delegated SUMMARY only if a worker needs the padding rule restated (pinned in AC-5 anyway).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` `SupportParameters::SupportParameters` - delegate (the density formula + absence of the bottom sentinel this step's divergence comment records).
- Verification:
  - `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-2/3/N1)
  - `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-2/3/N1)
  - `cargo test -p slicer-runtime --test integration support_family_closure 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (fixture fallout)
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0 (src edits are fingerprint inputs)
- Exit condition: AC-2 + AC-3 + AC-N1 command-green; fixture consumer green; guests fresh.

### Step 3: Integration arms — bounds/enum + CONFIG_BLOCK

- Task IDs: none (wayfinder ticket 18)
- Objective: add the scheduler bounds tests (AC-4: `support_interface_pattern = "bogus"` → enum `TypeMismatch`; `support_interface_loop_pattern = "yes"` → bool `TypeMismatch`; `support_interface_spacing = -0.5` → `OutOfRange`; `support_bottom_interface_spacing = -2.0` → `OutOfRange`; and the positive arm `support_bottom_interface_spacing = -1.0` resolves — the retained sentinel stays legal) against the real `traditional-support.toml` manifest, mirroring the packet 259 arm pattern. Add the CONFIG_BLOCK tests (AC-5: at defaults zero `support_interface_*` lines; explicit `support_interface_spacing = 0.8` → exactly one `; support_interface_spacing = 0.8`; explicit `support_interface_pattern = "rectilinear"` → exactly one `; support_interface_pattern = rectilinear`) using the runtime binary's per-test config injection (proven at packet 258/259 authoring).
- Precondition: Steps 1–2 exit met.
- Postcondition: AC-4 and AC-5 pass against the real manifest and the real pipeline driver; `serialize.rs` untouched (verified — no padding twins).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - full (460-ish lines — bounded read)
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror (do not read all ~1040 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (AC-5 pins: no entries gained or lost, no edits)
  - `docs/15_config_keys_reference.md` (generated; Step 4 only)
- Blast-radius discipline: test-only additions to existing binaries; no production surface. If the runtime binary's driver needs a config key it asserts on, use the same per-test config mechanism its current tests use (verified present at authoring for packet 258/259's keys).
- Expected sub-agent dispatches:
  - Question: does the CONFIG_BLOCK driver thread explicit module-declared keys into `raw_config`, and do existing arm tests set keys via sidecar/CLI or direct `raw_config` injection?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`; return: `FACT`; purpose: AC-5 arm form.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §CONFIG_BLOCK - delegated SUMMARY (padding rule), already applied.
- OrcaSlicer refs: none — integration arms are port-side only.
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-4)
  - `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (AC-5)
- Exit condition: AC-4 + AC-5 green.

### Step 4: Regenerate docs + close gates

- Task IDs: none (wayfinder ticket 18)
- Objective: run `cargo xtask gen-config-docs` to regenerate `docs/15_config_keys_reference.md` (the four module-key rows ×2 owner columns, the spacing default cells 0.5, the deviations block minus the two `support_interface_spacing` rows); verify `--check` passes, the pattern keys appear, and the deviations block holds exactly 25 data rows (AC-6); then the packet completion gate (AC suite re-run, workspace gates, guests-fresh check).
- Precondition: Steps 1–3 exit met.
- Postcondition: AC-6 exit 0 and keys present; all AC commands green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - NEVER load in full; verify via `rg`/`sed` only (key-presence + deviation-block row count; the doc has no per-module subheadings — rows carry the owner column)
- Files allowed to edit (at only via the generator):
  - `docs/15_config_keys_reference.md` - through `cargo xtask gen-config-docs` only
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (read-only)
  - any hand-edit of the generated doc
- Blast-radius discipline: none (generated doc).
- Expected sub-agent dispatches:
  - Question: does `gen-config-docs --check` pass, do the two pattern keys appear in the module-key table under both owner columns, do the spacing rows show 0.5, and does the deviations block count 25 with no `support_interface_spacing` row?; scope: `docs/15_config_keys_reference.md`; return: `FACT`; purpose: AC-6.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated; key-presence rg-verified.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask gen-config-docs --check && rg -q 'support_interface_pattern' docs/15_config_keys_reference.md && rg -q 'support_interface_loop_pattern' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| `')" = "25" ]; echo "exit=$?"` - FACT (AC-6)
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit 0
- Exit condition: AC-6 green; workspace gates green; guests fresh.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | two manifests + two guard files + dev-dep adds |
| Step 2 | M | const/comment ×2, fixture + consumer fallout, three arm groups ×2 suites; blast-radius inventory pre-baked |
| Step 3 | M | two test-file additions |
| Step 4 | S | generator + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read — **re-derive the crosswalk question at completion time**, not from this frozen note (ledger-fact rule). The feature-gap queue's packets carry no TASK row (survey precedent at 234a/253–259 authoring time); implementation is recorded against wayfinder ticket 18.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
