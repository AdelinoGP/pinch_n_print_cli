# Implementation Plan: 211-support-interface-bottom-layers

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Port the canonical fallback and wire the config field

- Task IDs: `TASK-327`
- Objective: add `pub fn resolve_interface_bottom_layers(bottom_layers: i32, top_layers: i32) -> u32` with its in-file test, and add `support_interface_bottom_layers: i32` to `SupportPlanner`, parsed in `from_config`. No geometry yet, no stub deletion yet.
- Precondition: working tree clean; `cargo test -p support-planner` green.
- Postcondition: `cargo test -p support-planner --lib resolve_interface_bottom_layers` passes all four fallback cases; the crate compiles; the code-1003 diagnostic still fires exactly as before (its tests are untouched in this step).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - two ranges: the `SupportPlanner` struct + `from_config` block, and the `#[cfg(test)] mod tests` header with `default_planner()`
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - every `tests/*.rs` in this module (this step touches only the in-file test module)
  - `docs/**`, `crates/**`, `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline (mandatory — this step adds a struct field):
  - `SupportPlanner` gains `support_interface_bottom_layers: i32`. The **complete, grep-verified struct-literal blast radius is two sites, both in `modules/core-modules/support-planner/src/lib.rs`**: `Ok(Self { … })` at the end of `from_config`, and `default_planner()` inside the in-file `#[cfg(test)] mod tests`. `SupportPlanner` is `pub`, but no external test constructs it by literal — all six files under `modules/core-modules/support-planner/tests/` go through `SupportPlanner::from_config(&config)`. Both sites are in this step's single edited file; that is why the edit cap is 1.
  - No schema or version constant is bumped, so there is no constant-value test fallout. `support-planner.toml`'s `default = -1` is unchanged (its comment is deleted in Step 4).
- Expected sub-agent dispatches:
  - Question: what exactly does `number_of_support_interface_bottom_layers` return for a negative input, and where is the `std::max(0, …)` clamp applied?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp`; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - the `support_interface_bottom_layers` note paragraph only, located by grep; confirms the key's live default and range before the field is added
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - delegate; never load
- Verification:
  - `cargo test -p support-planner --lib resolve_interface_bottom_layers` - FACT pass/fail (AC-1)
  - `cargo check -p support-planner --all-targets` - FACT pass/fail
- Exit condition: AC-1 PASS, both struct-literal sites compile, and `-5` resolves to the top count rather than to `0`. If the canonical dispatch shows a different clamp order, follow canonical and update AC-1's expectations in `packet.spec.md` in the same edit.

### Step 2: Extract the sub-chain walk from `smooth_branches`

- Task IDs: `TASK-327`
- Objective: lift `smooth_branches`' `sub_starts` gap walk into a private `split_column_into_chains(entries: &[SupportPlanEntry], column: &[usize]) -> Vec<(usize, usize)>` returning half-open ranges, and rewrite `smooth_branches` to consume it — with zero behaviour change.
- Precondition: Step 1 complete and green.
- Postcondition: `cargo test -p support-planner --test smooth_nodes_tdd` passes with its four assertions **unmodified**; `split_column_into_chains` has exactly one caller so far.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - the `group_branches_into_columns` / `first_point_xyw` / `smooth_branches` block only
  - `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` - lines `[39-90]` only, to confirm the guard assertions
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` — editing the guard defeats the step's purpose
  - `docs/**`, `crates/**`
- Blast-radius discipline: not applicable — this step adds no struct field and bumps no constant. It is a pure extraction with one caller.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p support-planner --test smooth_nodes_tdd` still pass, and with how many tests?; scope: `modules/core-modules/support-planner`; return: `FACT` pass/fail + the `test result:` line
- Context cost: `S`
- Authoritative docs:
  - none — this is a mechanical extraction inside one function
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo test -p support-planner --test smooth_nodes_tdd` - FACT pass/fail
  - `rg -q 'fn split_column_into_chains' modules/core-modules/support-planner/src/lib.rs` - FACT: the helper exists
- Exit condition: `smooth_nodes_tdd` green with unmodified assertions and the chain-split logic present exactly once in the file. If the extraction changes any smoothing output, it was not behaviour-preserving — revert and redo rather than adjusting the test.

### Step 3: Detect the landing and emit the bottom band (RED first)

- Task IDs: `TASK-327`
- Objective: author `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs` with its six cases, watch them fail, then implement `densify_bottom_interface` and its guarded call after `smooth_branches` in `plan_for_object` until they pass.
- Precondition: Steps 1 and 2 complete and green.
- Postcondition: all six cases pass; `to_buildplate_tdd` and `smooth_nodes_tdd` remain green; the code-1003 stub is still present (Step 4 retires it).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` - lines `[136-236]` (the `unreachable_buildplate_node_pruned` fixture) and `[482-570]` (the helper block) only — read-only template
  - `modules/core-modules/support-planner/src/lib.rs` - two ranges: the top-interface densification block inside `plan_for_object` (the mirror to follow, including its `bbox_half` and `layer_parity` derivation) and the `push_interface_scan_lines` helper
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs`
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` — template only, never edited
  - `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` — Step 4 owns it
  - `crates/**`, `docs/**`
- Blast-radius discipline: not applicable — no struct field is added here and no constant's value changes. `densify_bottom_interface` and its call site are additive.
- Expected sub-agent dispatches:
  - Question: in `TreeSupport::draw_circles`' floor-area block, (a) what condition triggers a floor band, (b) what happens when no model contact is found below a component, and (c) what disables the block entirely?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY` (≤200 words, no code)
  - Question: do `support_bottom_enable` and `support_floor_layers` both derive from `number_of_support_interface_bottom_layers`?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp`; return: `FACT` (≤5 lines)
  - Question: does `cargo test -p support-planner --test interface_bottom_layers_tdd` pass, and which cases fail?; scope: `modules/core-modules/support-planner`; return: `FACT` pass/fail + failing case names
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - only if the implementer needs the mm↔unit boundary for the band's half-extent after packet 210 lands; otherwise skip
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` - delegate; never load
- Verification:
  - `cargo test -p support-planner --test interface_bottom_layers_tdd` - FACT pass/fail + case names (AC-2, AC-3, AC-4, AC-5, AC-6, AC-N2)
  - `cargo test -p support-planner --test to_buildplate_tdd` - FACT pass/fail (the fixture's own semantics unchanged)
  - `cargo test -p support-planner --test smooth_nodes_tdd` - FACT pass/fail (ordering did not disturb smoothing)
- Exit condition: all six cases green **and** the RED run before implementation was recorded with every case failing. If a case passes before the implementation exists, the fixture is not exercising a model landing — fix the fixture (the layer below `L_end` must carry a `SupportGeometryViewEntry` whose outline covers the chain's XY) rather than accepting the pass.

### Step 4: Retire the code-1003 stub and the manifest comment

- Task IDs: `TASK-327`
- Objective: delete the code-1003 `push_diagnostic` block from `run_support_geometry`, delete the `# Not yet implemented` comment from the manifest, and rewrite the two `diagnostics_tdd.rs` cases to the new contract.
- Precondition: Step 3 green — the geometry exists, so the warning is now false.
- Postcondition: `rg 'code: 1003'` and `rg 'is not yet implemented'` return nothing in `src/lib.rs`; `diagnostics_tdd` passes with the two rewritten cases asserting zero code-1003 records at value 3 and at `-1`/absent, and with the code-1001/1002 cases untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` - the `//!` header, the two bottom-layers cases, and the helper block only; the code-1001/1002 cases are out of bounds
  - `modules/core-modules/support-planner/support-planner.toml` - the `[config.schema.support_interface_top_layers]` … `[config.schema.tree_support_interface_spacing_mm]` span only
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/tests/diagnostics_tdd.rs`
  - `modules/core-modules/support-planner/support-planner.toml`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs` — Step 3 owns it
  - every other module's manifest
  - `docs/**` — Step 5 owns the doc edits
- Blast-radius discipline: not applicable — nothing is added. The deletion's fallout is exactly the two `diagnostics_tdd.rs` cases, both edited in this step, which is why the edit cap is used in full.
- Expected sub-agent dispatches:
  - Question: does `cargo test -p support-planner` pass, and do any binaries other than `diagnostics_tdd` change their result?; scope: `modules/core-modules/support-planner`; return: `FACT` pass/fail + failing test names
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` - §Status only, read here to draft the retirement sentence applied in Step 5
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo test -p support-planner --test diagnostics_tdd` - FACT pass/fail (AC-N1)
  - `! rg -q 'code: 1003' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'is not yet implemented' modules/core-modules/support-planner/src/lib.rs` - FACT: AC-9
  - `! rg -q 'Not yet implemented' modules/core-modules/support-planner/support-planner.toml` - FACT: AC-7
- Exit condition: AC-7, AC-9 and AC-N1 PASS. The two rewritten tests must keep their "exactly N code-1003 records" assertion shape with N = 0 and keep dumping the observed code list on failure — deleting either test, or replacing the count assertion with a weaker predicate, fails this step.

### Step 5: Rebuild the guest, run the invariant gate, and close the ledger

- Task IDs: `TASK-327`
- Objective: rebuild `support-planner.wasm`, prove the wedge invariants hold with bands enabled by default, clear the workspace gates, and land the four doc edits.
- Precondition: Steps 3 and 4 complete; `cargo test -p support-planner` green.
- Postcondition: `cargo xtask build-guests --check` reports no `STALE:`; `support_invariants_wedge_tdd` passes; `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are clean; all four Doc Impact greps in `packet.spec.md` return PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - the `support_interface_bottom_layers` note paragraph only, located by grep
  - `docs/adr/0010-typed-diagnostic-channel.md` - §Status only
  - `docs/DEVIATION_LOG.md` - the `DEV-129` row only, located by grep
  - `docs/07_implementation_status.md` - the Workstream 3 heading and the `TASK-163b-diagnostic` row only, located by grep
- Files allowed to edit (at most 3 per sub-step; this step is split into 5a and 5b below because it carries four doc files):
  - **5a (verification, then lint fixes only):** `modules/core-modules/support-planner/src/lib.rs`
  - **5b (ledger):** `docs/15_config_keys_reference.md`, `docs/adr/0010-typed-diagnostic-channel.md`, `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` — four single-paragraph edits, no design content; treat as one ledger transaction and land them in one commit.
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/**` — the invariant suite is the oracle; adjusting it to accept the new bands is prohibited
  - `target/**`, `modules/core-modules/support-planner/support-planner.wasm` (regenerated, never hand-edited)
  - `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` — no contract changed
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests --check` report any `STALE:` line, and after a rebuild does it come back clean?; scope: workspace; return: `FACT` (≤5 lines)
  - Question: do the four named tests in `support_invariants_wedge_tdd` pass?; scope: `cargo test -p slicer-runtime --test integration support_invariants_wedge_tdd`; return: `FACT` pass/fail + ≤20 lines of the first failure
  - Question: does `cargo clippy --workspace --all-targets -- -D warnings` pass, and which lints fire in `support-planner`?; scope: workspace; return: `FACT` pass/fail + lint names
  - Question: what is the current status cell of `DEV-129`, and is `TASK-327` still absent from `docs/07_implementation_status.md`?; scope: both files; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` §Status; `docs/15_config_keys_reference.md` note paragraph; `docs/DEVIATION_LOG.md` `DEV-129`; `docs/07_implementation_status.md` Workstream 3 — all ranged or delegated
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo xtask build-guests --check` - FACT: no `STALE:` (AC-N3)
  - `cargo test -p slicer-runtime --test integration support_invariants_wedge_tdd` - FACT pass/fail (AC-8)
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `rg -q 'resolves to .support_interface_top_layers' docs/15_config_keys_reference.md && ! rg -q 'code-.1003' docs/15_config_keys_reference.md` - FACT pass/fail
  - `rg -q '1003.*retired' docs/adr/0010-typed-diagnostic-channel.md` - FACT pass/fail
  - `rg -q '^\| DEV-129 .*Closed' docs/DEVIATION_LOG.md` - FACT pass/fail
  - `rg -q 'TASK-327' docs/07_implementation_status.md` - FACT pass/fail
- Exit condition: AC-8, AC-N3 and every Doc Impact grep PASS. A wedge-invariant failure is this packet's bug until `build-guests --check` has been shown clean; the deflections listed in `CLAUDE.md` §"Guest WASM Staleness" are prohibited. If a support G-code baseline shifts because bands are now on by default, re-record it to the canonical-correct output and say so — never revert the behaviour to keep a baseline green. If `TASK-327` turns out to be taken, take the next free ID and update `packet.spec.md` frontmatter and `task-map.md` in the same edit.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One `pub fn`, one field, two struct-literal sites, one in-file test |
| Step 2 | S | Pure extraction guarded by an untouched existing suite |
| Step 3 | M | The packet's core: six-case RED suite plus landing detection and band emission |
| Step 4 | S | Three deletions and two test rewrites |
| Step 5 | S | Verification plus four single-paragraph ledger edits, split 5a/5b |

Aggregate `M`; no step is L. Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read; the update includes both the new `TASK-327` row and the amendment note on the existing `TASK-163b-diagnostic` row that code 1003 is retired.
- Reconcile reopened/superseded status transitions: none — this packet reopens and supersedes nothing. Packet 118's code-1003 work is *retired by implementation*, which is recorded on `TASK-163b-diagnostic` and in ADR-0010 §Status rather than by a status flip on packet 118.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (`AC-1` … `AC-9`, `AC-N1` … `AC-N3`) and the three packet-level gate commands.
- Record remaining packet-local risk: the cap-truncation false positive (a chain truncated by `max_branches_per_layer` above model geometry gets a band canonical would not draw), and the coarseness of segment-count assertions. Both are stated on the `DEV-129` closure row.
- Confirm the packet-210 forward dependency: if 210 merged during implementation, re-run `cargo test -p support-planner --test interface_bottom_layers_tdd` and AC-8 against the migrated signatures before declaring closure.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.
- `cargo test --workspace` is **not** required for this packet's closure. The wedge invariant suite plus `cargo check/clippy --workspace --all-targets` is the closure bar; run the full suite only if the user asks or if a support G-code baseline is re-recorded.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
