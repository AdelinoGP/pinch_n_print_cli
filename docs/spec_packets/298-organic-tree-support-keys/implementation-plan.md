# Implementation Plan: 298-organic-tree-support-keys

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare all eight keys plus the renderer gate row

- Task IDs: `TASK-000`
- Objective: Both manifests expose the P72 surface at canonical defaults/types so the declared-view whitelist passes every key to its reader.
- Precondition: Packet number 298 and DEV-189 verified free (done at authoring; re-derive with the two commands in Verification before editing).
- Postcondition: Planner manifest holds 6 organic rows; renderer manifest holds 2 brim rows + the `support_style` gate row; canonical defaults re-verified against the oracle.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - lines 30–100 (classic key stanza shape + style row to mirror)
  - `modules/core-modules/tree-support/tree-support.toml` - `[config.schema]` region (stanza shape)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  - `modules/core-modules/tree-support/tree-support.toml`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no padding edits, rule 2)
  - Any `src/lib.rs` (no code until Step 2)
  - `OrcaSlicerDocumented/...` (delegate only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant): no struct field added in this step (manifest rows only) — no blast radius.
- Expected sub-agent dispatches:
  - Question: re-verify canonical type/default/min/max for all eight keys; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` (8 rows: key, type, default, min, max or "none")
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest schema stanza shape, percent string spelling)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load
- Verification:
  - `ls -d docs/spec_packets/298*/ && rg -c 'DEV-189' docs/DEVIATION_LOG.md docs/spec_packets/ 2>/dev/null; echo "expect: only this packet's own files match"` - FACT: number still 298 and DEV-189 still free outside this packet
  - AC-1 manifest grep chain from `packet.spec.md` - FACT pass/fail
- Exit condition: AC-1 pipe command passes; defaults match the delegated FACT rows exactly.

### Step 2: Planner style-gated organic selection + tests + DEV-189

- Task IDs: `TASK-000`
- Objective: Explicit-organic runs resolve branch geometry from the organic set with canonical value semantics; classic styles are provably unaffected; DEV-189 row lands.
- Precondition: Step 1 exit holds (manifest rows present, defaults verified).
- Postcondition: `SupportPlanner::from_config` selects the organic set behind `organic_substitution_requested`, saturates sub-canonical inputs, updates all 3 struct literals; `organic_params_tdd` covers AC-2–AC-5 + AC-N1–AC-N2; DEV-189 row in the log.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines 202–258 (style helpers) and 1618–1800 (`from_config` + literal)
  - `crates/slicer-ir/src/slice_ir.rs` - lines 875–975 (accessor semantics)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs` (selection arm + saturation + 3 literal updates, all in-file)
  - `modules/core-modules/tree-support-planner/tests/organic_params_tdd.rs` (new binary, autodiscovered — no aggregator edit)
  - `docs/DEVIATION_LOG.md` (append DEV-189 row only)
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support/**` (Step 3)
  - `docs/15_config_keys_reference.md` (Step 4 regen only)
  - `OrcaSlicerDocumented/...` (delegate only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant): effective organic values resolve into existing fields where shape allows; any genuinely new field lists its blast radius here — `SupportPlanner` literals are fully inventoried (authoring LOCATIONS: `from_config` `Ok(Self {`, `default_planner`, test helper — all in `src/lib.rs`, zero test-dir sites) and all three are in this step's edit file. No deferred fallout.
- Expected sub-agent dispatches:
  - Question: organic settings-constructor formulas (radian points, clamp order, tip≤diameter, top-rate application, slow-cap application); scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp`; return: `SNIPPETS` (≤3 × 30 lines) + `SUMMARY` (≤200 words)
  - Question: `run_support_geometry` code-1005 Warn site shape; scope: `modules/core-modules/tree-support-planner/src/lib.rs`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct range read (mm↔unit helpers)
  - `docs/DEVIATION_LOG.md` - DEV-156 row (contract preserved)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` - delegate; never load
- Verification:
  - `cargo test -p tree-support-planner --test organic_params_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (AC-2–AC-5, AC-N1–AC-N2)
  - `cargo test -p tree-support-planner --test tree_style_styles_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (no style-routing regression)
  - `cargo xtask check-literals 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: all three commands pass; `rg -q 'DEV-189' docs/DEVIATION_LOG.md` passes.

### Step 3: Renderer brim stage + tests

- Task IDs: `TASK-000`
- Objective: First-layer brim loops around build-plate-contact tree bases on the explicit-organic path, auto-derived or fixed width, zero-width silent.
- Precondition: Step 2 exit holds; auto-brim derived-width formula + `draw_circles` base conditions borrowed via delegation.
- Postcondition: `TreeSupport` carries the brim block, `TreeSupport::from_config` reads the three renderer rows, first-layer emission adds brim loops only on the gated path; `tree_brim_tdd` covers AC-6–AC-7.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support/src/lib.rs` - lines 242–310 (`from_config` + literal; re-derive exact emission-site window via in-step grep before editing)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support/src/lib.rs` (fields + reads + emission stage)
  - `modules/core-modules/tree-support/tests/tree_brim_tdd.rs` (new binary, autodiscovered — no aggregator edit)
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support-planner/**` (Step 2 landed)
  - `docs/15_config_keys_reference.md` (Step 4 regen only)
  - `OrcaSlicerDocumented/...` (delegate only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant): `TreeSupport` literals fully inventoried (authoring LOCATIONS: exactly 1 site, `from_config` `Ok(Self {`; `from_config_defaults` calls `from_config`, no literal) and the site is in this step's edit file. No deferred fallout.
- Expected sub-agent dispatches:
  - Question: `draw_circles` brim path (auto-derived vs fixed selection, first-layer/base conditions, width units); scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SNIPPETS` (≤3 × 30 lines) + `SUMMARY` (≤200 words)
  - Question: plate-contact base polygon site in the renderer emission path; scope: `modules/core-modules/tree-support/src/lib.rs`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct range read (brim-width mm→units at the loop boundary)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate; never load
- Verification:
  - `cargo test -p tree-support --test tree_brim_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (AC-6–AC-7)
  - `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | tail -5` - FACT pass/fail (no renderer regression)
- Exit condition: both commands pass; classic-style run with brim keys set emits zero brim loops (AC-7 gate arm).

### Step 4: Regen docs, freshness, and gates

- Task IDs: `TASK-000`
- Objective: Generated tables current, guests fresh, workspace gates green.
- Precondition: Steps 1–3 exits hold.
- Postcondition: `docs/15` tables contain the 8 keys; `gen-config-docs --check` passes, `build-guests --check` exits 0 (fresh), check, clippy, and both new test binaries pass.
- Files allowed to read, with ranges when over 300 lines:
  - None beyond command output.
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` (via `cargo xtask gen-config-docs` tool only — never hand-edited)
- Files explicitly out of bounds:
  - All module manifests and `src/` (Steps 1–3 landed)
  - `docs/DEVIATION_LOG.md` (Step 2 landed)
  - `OrcaSlicerDocumented/...` (delegate only)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant): no struct/schema change in this step — no blast radius.
- Expected sub-agent dispatches:
  - None. All commands run directly; output is small and parseable.
- Context cost: `S`
- Authoritative docs:
  - None. Tool-driven step.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo xtask gen-config-docs && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail (AC-8 doc arm)
  - `cargo xtask build-guests --check 2>&1 | tee target/test-output.log | tail -3; test $? -eq 0 || cargo xtask build-guests 2>&1 | tee target/test-output.log | tail -3` - FACT: guests fresh (rebuild without `--check` only if stale)
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
  - `cargo test -p tree-support-planner --test organic_params_tdd 2>&1 | tee target/test-output.log | tail -3 && cargo test -p tree-support --test tree_brim_tdd 2>&1 | tee target/test-output.log | tail -3` - FACT pass/fail
- Exit condition: every command above passes; AC-8 full chain passes. Known pre-existing red excluded from this gate: `cargo xtask check-deviations --check` fails on the clean tree too (ticket-42 finding) — do not gate on it, do not fix it here.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Manifests + oracle default FACT |
| Step 2 | M | Planner wiring + 6 tests + DEV row |
| Step 3 | M | Renderer stage + 2 tests |
| Step 4 | S | Regen + freshness + gates |

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
