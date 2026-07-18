# Design: 99_paint-pipeline-doc-sync

## Controlling Code Paths

- Primary code paths: NONE — this packet edits docs only.
- Edit targets: 6 `docs/*.md` files + 1 `docs/specs/*.md` file (1-line frontmatter flip) + `CONTEXT.md` (verification only).
- OrcaSlicer comparison surface: none directly.

## Architecture Constraints

- No-production-code invariant: this packet does NOT edit any file under `crates/`, `modules/`, `wit/`, or `resources/`. AC-17's byte-identical g-code is the regression guard.
- Sync-not-redefine invariant: every doc edit reflects the state landed in packets 89-98. The packet does NOT introduce new design decisions; if a question arises about content shape, the source-of-truth is the implementation or the planning docs, not the doc text being edited.
- Deletion-content invariant: when removing references to deleted types (`PaintRegionIR` et al.), don't replace with placeholder prose. Delete the section / paragraph entirely.

## Code Change Surface

- Selected approach: edit each doc file in its own step (Steps 1-7). Each step has a narrow grep gate.
- Exact files expected to change:
  - **`docs/01_system_architecture.md`** — rewrite PrePass section, add variant-chain model, remove "new — runs first" warning.
  - **`docs/02_ir_schemas.md`** — bump versions, add new types, remove deleted types, document interner.
  - **`docs/03_wit_and_manifest.md`** — add `[[region_split]]` schema + priority registry + aggregation, remove `mesh-segmentation-output`.
  - **`docs/04_host_scheduler.md`** — update PrePass table, document host-filtered dispatch + empty-polygon guard, remove "guard-based fallback contract".
  - **`docs/07_implementation_status.md`** — TASK-239..TASK-249 implemented entries (delegate via sub-agent — never load full backlog).
  - **`docs/08_coordinate_system.md`** — add constants conversion table.
  - **`docs/specs/orca-paint-segmentation-parity.md`** — flip `Status:` (1-line frontmatter edit).
  - **`CONTEXT.md`** — verify only (entries added during planning).
- Rejected alternatives:
  - **Edit docs in-flight (each packet syncs its own doc subset)**: rejected (the roadmap explicitly defers doc sync to a single packet to avoid mid-roadmap doc churn).
  - **Defer doc sync to a future cleanup pass**: rejected — the user requested the roadmap close with synced docs.

## Files in Scope (read + edit)

- `docs/01_system_architecture.md` — read PrePass section; edit.
- `docs/02_ir_schemas.md` — read IR sections; edit.
- `docs/03_wit_and_manifest.md` — read manifest section; edit.
- `docs/04_host_scheduler.md` — read PrePass table + dispatch section; edit.
- `docs/07_implementation_status.md` — delegate edit to sub-agent (never load full file).
- `docs/08_coordinate_system.md` — read constants section; edit.
- `docs/specs/orca-paint-segmentation-parity.md` — read frontmatter; edit Status line.
- `CONTEXT.md` — verify only (no edit unless entries missing).

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` — source-of-truth for content shape.
- `crates/slicer-ir/src/slice_ir.rs` post-roadmap state — for IR shape facts.
- `crates/slicer-runtime/src/builtins/*_producer.rs` — for stage definitions.
- `crates/slicer-runtime/src/prepass.rs` post-roadmap state — for stage ordering.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate (none expected).
- `target/`, `Cargo.lock`, generated code — never load.
- Production source under `crates/*/src/` — DO NOT EDIT. (Reading specific small ranges to confirm content is acceptable; editing is not.)
- The full `docs/07_implementation_status.md` — delegate. The doc is large and monotonically growing.

## Expected Sub-Agent Dispatches

- "Open `docs/01_system_architecture.md` PrePass section; return SNIPPETS (≤ 40 lines)" — purpose: Step 1 inventory.
- "Open `docs/02_ir_schemas.md` IR type sections; return LOCATIONS of each affected type" — purpose: Step 2 inventory.
- "Open `docs/03_wit_and_manifest.md` manifest section; return SNIPPETS (≤ 60 lines)" — purpose: Step 3 inventory.
- "Open `docs/04_host_scheduler.md` PrePass table + dispatch section; return SNIPPETS (≤ 60 lines)" — purpose: Step 4 inventory.
- "Append TASK-239 through TASK-249 implemented entries to `docs/07_implementation_status.md`; return FACT pass/fail" — purpose: delegated edit (Step 5).
- "Open `docs/08_coordinate_system.md` constants section; return SNIPPETS (≤ 30 lines)" — purpose: Step 6 inventory.
- "Run per-AC `rg -q` commands; return FACT pass/fail each" — purpose: Step 7 verification gates.
- "Run wedge + cube slices + sha256sum; FACT (2 SHAs)" — purpose: AC-17 regression check.

## Data and Contract Notes

- IR contracts: documented (not changed).
- WIT boundary: documented (not changed).
- Determinism: this packet has no effect on runtime determinism.

## Locked Assumptions and Invariants

- **Behavior preservation**: AC-17 confirms.
- **Source-of-truth is the implementation**: if a doc-edit content question is ambiguous, the answer is in the code, not invented.
- **Deletions are full deletions, not redactions**: removed-type sections are erased, not replaced with placeholder prose.

## Risks and Tradeoffs

- **Risk: a doc edit inadvertently includes outdated information** about a deleted type. Mitigation: per-doc grep checks (AC-N1, AC-N2, AC-N3) catch this.
- **Risk: `docs/07_implementation_status.md` becomes corrupted** by direct loading + editing. Mitigation: delegate the edit to a sub-agent that runs a small `Edit` operation; never load the full file.
- **Tradeoff: doc-sync packet vs. doc-edits-per-packet**: deferred sync (this approach) trades transient incorrect documentation in the middle of the roadmap for a clean closing packet. The user explicitly chose this trade.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 2 — `docs/02_ir_schemas.md` is the largest edit surface).
- Highest-risk dispatch: the `docs/07` delegated edit (must be tightly scoped to avoid loading the full backlog).

## Open Questions

- `[FWD]` — Does `docs/07_implementation_status.md` already contain the TASK-239..TASK-249 placeholder entries (added at each prior packet's close)? Step 5 dispatch confirms; if yes, the work is just status-flips; if no, the entries are added wholesale.
- `[FWD]` — Are `CONTEXT.md`'s 4 paint-vocab entries actually present? Per the conversation summary they were added during planning, but a verification grep is needed before continuing.
- `[BLOCK]` — None.
