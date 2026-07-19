# Implementation Plan: support-modules-doc-honesty-cleanup

## Execution Rules

- Work one atomic step at a time; each step maps to source-plan B1, B2, or B3 because no canonical backlog ID is currently mapped.
- Do not activate or close the packet while the `[BLOCK]` backlog crosswalk remains unresolved.
- Keep the three lead-comment edits sequential with the later `SupportPlanner` dead-state edit; never interleave overlapping edits in `support-planner/src/lib.rs`.

## Steps

### Step 1: Reconcile current symbols and backlog ownership

- Task IDs: none mapped; source-plan items B1, B2, B3.
- Objective: verify the current contiguous leading comment blocks, the three actual `BASE_SPEED` consumers, `SupportPlanner` dead state, TOML key, test fixture, and the absence/collision of source-plan task IDs in `docs/07_implementation_status.md`.
- Precondition: the canonical support spec, batch anchor, and current tree are available.
- Postcondition: a bounded inventory identifies exact symbols and a maintainer-visible ownership blocker; no old line number or stale API name is carried forward.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/support-modules-orca-port.md` - §B1-B3 and §D8-D9 only.
  - `docs/specs/support-modules-orca-port-plan.md` - packet 116 queue row only.
  - `docs/07_implementation_status.md` - targeted searches for support rows and `TASK-250`, `TASK-251`, `TASK-252` only.
  - The four module `src/lib.rs` files - bounded contiguous leading documentation and `BASE_SPEED` symbols only.
  - `support-planner/src/lib.rs` - `SupportPlanner`, `PrepassModule::on_print_start`, and `default_planner` only.
  - `support-planner/support-planner.toml` - `[config.schema.support_interface_bottom_layers]` only.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds:
  - Every implementation file for this read-only step.
  - Other packet directories and `OrcaSlicerDocumented/**`.
- Expected sub-agent dispatches:
  - Question: Resolve source-plan B1/B2/B3 against current backlog ownership; scope: `docs/07_implementation_status.md`; return: `LOCATIONS`; purpose: activation blocker evidence.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B1-B3 and §D8-D9 direct read.
- OrcaSlicer refs: none.
- Verification:
  - Bounded `rg` inventory and backlog survey return the exact current symbols and collision evidence as `LOCATIONS`.
- Exit condition: inventory complete and the mapping blocker remains explicit; proceed only as draft.

### Step 2: Rewrite tree and traditional support documentation

- Task IDs: none mapped; source-plan B1 and B3.
- Objective: replace the contiguous leading comment claims in `tree-support` and `traditional-support` with the current grid-MST/per-layer descriptions and add the exact speed-normalization explanation to both.
- Precondition: Step 1 confirms both files still carry the stale or incomplete wording.
- Postcondition: AC-1 and AC-2 pass through bounded contiguous-leading-block extraction; both files explain `speed_factor = configured_speed / BASE_SPEED` with `BASE_SPEED = 50.0` in that block; code below the `//!` blocks is unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support/src/lib.rs` - leading `//!` block and `BASE_SPEED` declaration/use.
  - `modules/core-modules/traditional-support/src/lib.rs` - leading `//!` block and `BASE_SPEED` declaration/use.
  - `docs/specs/support-modules-orca-port.md` - §B1 and §B3 only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/traditional-support/src/lib.rs`
- Files explicitly out of bounds:
  - `support-planner/src/lib.rs` - handled in Step 3 and Step 4.
  - `rectilinear-infill/src/lib.rs` - handled in Step 3.
  - All manifests, WIT, IR, and tests.
- Expected sub-agent dispatches:
  - Question: Run AC-1, AC-2, and the two-module portion of AC-6; scope: the two edited files; return: `FACT`; purpose: verify documentation anchors.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B1 and §B3 direct range read.
- OrcaSlicer refs: none.
- Verification:
   - AC-1's bounded command in `packet.spec.md` - FACT pass/fail; it reads at most the first 80 lines, extracts the contiguous `//!` prefix, and checks the opening line plus non-parity wording.
   - AC-2's bounded command in `packet.spec.md` - FACT pass/fail; it reads at most the first 80 lines, extracts the contiguous `//!` prefix, and checks the opening line plus upstream eligibility wording.
- Exit condition: both honesty anchors and both speed sections pass; no implementation lines changed.

### Step 3: Rewrite planner documentation and rectilinear speed documentation

- Task IDs: none mapped; source-plan B1 and B3.
- Objective: replace the planner's Orca-port claim in its contiguous leading block with the algorithmic-shape/non-parity wording and add the speed-normalization section to the contiguous leading block of `rectilinear-infill`; do not add one to `support-planner` because it has no `BASE_SPEED` consumer.
- Precondition: Step 2 completed without changing planner behavior.
- Postcondition: AC-3 and AC-6 pass through bounded leading-block checks; `SupportPlanner`, `on_print_start`, `tapered_radius`, and all non-comment code remain unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - leading `//!` block only.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` - leading `//!` block and `BASE_SPEED` declaration/use.
  - `docs/specs/support-modules-orca-port.md` - §B1 and §B3 only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
- Files explicitly out of bounds:
  - `support-planner` field/parser and TOML - handled in Step 4.
  - Other `BASE_SPEED` consumers and all Orca source.
- Expected sub-agent dispatches:
  - Question: Run AC-3 and AC-6; scope: the two edited files; return: `FACT`; purpose: verify the corrected claim and actual consumer set.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B1 and §B3 direct range read.
- OrcaSlicer refs: none.
- Verification:
   - AC-3's bounded command in `packet.spec.md` - FACT pass/fail; it checks the required opening line and algorithmic-shape wording only inside the contiguous leading `//!` block.
   - AC-6's bounded command in `packet.spec.md` - FACT pass/fail; it checks `# Speed normalization`, the formula, and `BASE_SPEED = 50.0` in each current consumer's leading block.
- Exit condition: planner no longer claims numerical parity and all three actual packet consumers document the normalization.

### Step 4: Remove dead state and mark the deferred diagnostic

- Task IDs: none mapped; source-plan B2/D8.
- Objective: remove every field/struct-literal assignment for the unused bottom-interface field, its parse-and-store lookup, and private fixture assignment, while preserving the snake_case TOML key with its immediately adjacent explicit deferred-status comment. Do not add a string warning or warning test; packet 118 owns D11 typed emission.
- Precondition: Step 1 confirmed the dead field/state and the schema key; packet 118's typed channel ownership and current dependency mismatch are recorded as blockers.
- Postcondition: AC-4, AC-5, AC-N1, and AC-7 pass; AC-4 separately rejects all field/struct-literal or assignment forms and the parse lookup, AC-7 proves the snake_case key/comment adjacency, the TOML key's existing schema values are unchanged, and no packet-116 warning path is claimed.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - `SupportPlanner`, `PrepassModule::on_print_start`, and `default_planner` only.
  - `modules/core-modules/support-planner/support-planner.toml` - the named schema entry only.
  - `docs/specs/support-modules-orca-port.md` - §B2, §D8, and §D11 only.
  - `docs/adr/0010-typed-diagnostic-channel.md` - typed channel contract only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/support-planner.toml`
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` and typed diagnostic code - packet 118.
  - Any warning test binary or existing planner test.
- Expected sub-agent dispatches:
  - Question: Run the static AC-4/AC-5/AC-7 checks and planner compile/test; scope: the packet's exact paths; return: `FACT` pass/fail with at most 20 failure lines; purpose: dead-state and no-string-warning gate.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B2, §D8, and §D11 direct range read.
- OrcaSlicer refs: none.
- Verification:
   - `! rg -q 'support_interface_bottom_layers\s*[:=]' modules/core-modules/support-planner/src/lib.rs` - FACT pass; no field, struct-literal, or assignment form remains.
   - `! rg -q 'config\.get\("support_interface_bottom_layers"\)' modules/core-modules/support-planner/src/lib.rs` - FACT pass; no parse-and-store lookup remains.
   - `! rg -q 'support_interface_bottom_layers is not yet implemented' modules/core-modules/support-planner/src/lib.rs` - FACT pass.
   - AC-7's bounded 200-line schema command in `packet.spec.md` - FACT pass; it proves the snake_case section and an immediately adjacent deferred-status comment.
  - `cargo test -p support-planner --all-targets 2>&1 | tee target/test-output.log` - FACT pass.
- Exit condition: dead state is absent, the key remains schema-visible, and packet 116 makes no untyped warning claim.

### Step 5: Run packet gates and freshness check

- Task IDs: none mapped; source-plan B1, B2, B3.
- Objective: run the narrow compile/lint/test matrix and the guest freshness check, then leave the packet draft until backlog ownership is resolved.
- Precondition: Steps 2-4 pass their local exits.
- Postcondition: all packet AC commands and gate commands report pass; `cargo xtask build-guests --check` is clean or any stale artifact is rebuilt and rechecked by the delegated worker; status remains `draft` because the mapping blocker is not implementation evidence.
- Files allowed to read: none beyond prior step outputs.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds:
  - `target/**`, generated artifacts, `docs/07_implementation_status.md`, and every other packet directory.
- Expected sub-agent dispatches:
  - Question: Run the full matrix in `requirements.md`, including `cargo xtask build-guests --check`; scope: packet commands only; return: `FACT` PASS/FAIL list, with bounded failure snippets; purpose: closure evidence.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets` - FACT pass/fail.
  - `cargo clippy -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT `up to date` or stale/rebuild/recheck result.
- Exit condition: implementation verification is green; the packet stays draft and cannot be marked implemented until the exact backlog crosswalk is supplied.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Bounded symbol and ledger survey. |
| Step 2 | S | Two uniform comment blocks. |
| Step 3 | S | Planner honesty and one actual speed consumer. |
| Step 4 | S | Dead-state cleanup and deferred diagnostic boundary. |
| Step 5 | S | Narrow gates and guest freshness. |

Aggregate: `S`. No step is L; no step is M.

## Packet Completion Gate

- All implementation steps and exits pass.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` is clean after any required rebuild.
- A maintainer supplies a non-colliding `docs/07_implementation_status.md` mapping for B1, B2, and B3; until then, do not change `packet.spec.md` from `draft`.
- No backlog row or task ID is closed by this packet.

## Acceptance Ceremony

- Re-dispatch every AC command and the three packet verification gates.
- Re-derive the backlog crosswalk at ceremony time; do not rely on this packet's ledger snapshot.
- Confirm the TOML key remains present and packet 118 has the exact warning message as its typed-diagnostic migration input.
