---
status: implemented
packet: 170-seam-livepath-audit
task_ids:
  - TASK-120c
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
depends_on:
  - 178-seam-region-aware-planning
  - 179-seam-canonical-algorithm-fidelity
  - 180-seam-final-placement-default
---

# Packet Contract: 170-seam-livepath-audit

## Goal

Prove (or fix) that seam rotation in `seam-placer` never erases sibling wall loops — every region entering `run_wall_postprocess` with N wall loops leaves with exactly N — via multi-wall and multi-region regression fixtures, then close or re-scope TASK-120c in `docs/07_implementation_status.md`.

## Scope Boundaries

Correctness-audit packet over `modules/core-modules/seam-placer` only: new regression test file plus a fix in `run_wall_postprocess` if any fixture falsifies the wall-preservation invariant, plus the `docs/07` TASK-120c reconciliation. No planner, host, WIT, manifest, or config changes; the candidates-vs-`resolved_seam` preference order is already correct (verified against `select_seam_candidate` and `aligned_seam_target` in `modules/core-modules/seam-placer/src/lib.rs`) and is out of scope.

## Prerequisites and Blockers

- Depends on: `.ralph/specs/178-seam-region-aware-planning` (supersedes `168-seam-aligned-modes`), `.ralph/specs/179-seam-canonical-algorithm-fidelity`, and `.ralph/specs/180-seam-final-placement-default` — all three are `status: implemented`. The audit must cover the aligned consumption branch as it now exists after the continuous-projection work in packet 180. AC-3 specifically pins the non-empty-candidate `aligned_seam_target` branch; `project_onto_wall_segment` remains the empty-candidate fallback and is not reached by that fixture.
- Unblocks: TASK-120c closure.
- Activation blockers: none — all dependencies are `status: implemented`. AC-3's "host-injected `resolved_seam`" fixture no longer requires a separate packet-168 setup; the `aligned` mode and host-injection are in place via packet 178.

## Acceptance Criteria

- **AC-1. Given** a single region with exactly 3 wall loops whose seam candidate position coincides (within the module's `0.001` mm tolerance) with a vertex of wall index 1, **when** `run_wall_postprocess` runs in `nearest` mode, **then** the output region contains exactly 3 wall loops; loops at wall indices 0 and 2 are point-for-point identical to their inputs (including `feature_flags` and `width_profile.widths` lengths); loop 1's `path.points[0]` equals the seam vertex; and `resolved_seam()` reports `wall_index == 1`. | `cargo test -p seam-placer --test seam_sibling_walls_tdd -- siblings_survive_rotation 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** two regions in one call — region A (`region_id` "0") with 3 wall loops and region B (`region_id` "1") with 2 wall loops, distinct `object_id`s allowed — each with a valid seam candidate, **when** `run_wall_postprocess` runs, **then** the output contains both regions with exactly 3 and exactly 2 wall loops respectively, and each emitted loop's `(object_id, region_id)` pairing matches its input region. | `cargo test -p seam-placer --test seam_sibling_walls_tdd -- multi_region_wall_counts_preserved 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** a 3-wall region in `aligned` mode with a host-injected `resolved_seam` 0.3 mm off the nearest wall vertex and a non-empty `seam_candidates` list, **when** `run_wall_postprocess` runs, **then** the output region contains exactly 3 wall loops and the two non-target loops are point-for-point identical to their inputs. AC-3 exercises the `aligned_seam_target` production branch: the non-empty candidates and injected `resolved_seam` enter the aligned `else` arm, which calls `aligned_seam_target`; the `project_onto_wall_segment` path is not reached. The fixture decouples this wall-preservation audit from future changes to continuous-projection behavior. | `cargo test -p seam-placer --test seam_sibling_walls_tdd -- aligned_snap_preserves_siblings 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** the audit outcome, **when** the packet closes, **then** the existing reopened `- [~] TASK-120c` row in `docs/07_implementation_status.md` is reconciled: its status flips to `[x]` (closed, audit finding recorded) or `[ ]` (re-scoped, exact residual defect named), the stale reopened-gap text about candidate preference and sibling erasure is replaced with the current facts, and the row references packet `170-seam-livepath-audit`. | `grep -E '^- \[[x ]\] TASK-120c ' docs/07_implementation_status.md | grep -q '170-seam-livepath-audit' && ! grep -qE '^- \[~\] TASK-120c ' docs/07_implementation_status.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a region with 4 wall loops whose seam candidates and `resolved_seam` all miss every wall vertex by more than the `0.001` mm tolerance (the known planner mesh-corner vs inset-boundary gap), **when** `run_wall_postprocess` runs in `nearest` mode, **then** the output still contains exactly 4 wall loops, all point-for-point identical to their inputs (pristine emission), and no resolved seam is committed for that region. (In `aligned` mode with empty `seam_candidates` and an off-vertex `resolved_seam`, the continuous-projection step in packet 180 inserts an interior point; the subsequent `find_seam_location` vertex-match fails, and the emission falls back to pristine. Wall preservation is preserved in both modes; this AC pins the `nearest`-mode path which is unaffected by 180.) | `cargo test -p seam-placer --test seam_sibling_walls_tdd -- tolerance_miss_emits_all_walls_pristine 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p seam-placer 2>&1 | grep '^test result'`

## Authoritative Docs

- `docs/07_implementation_status.md` — delegate (large); only the reopened TASK-120c row and its referencing TASK-120/151/159 rows, plus the reconciliation edit. Anchor text for the disposition edit is the literal `- [~] TASK-120c Restore seam placement on real wall-loop seam candidates` row (no line-number pin — the file has been edited since this packet was drafted).
- `crates/slicer-sdk/src/builders.rs` — delegate a FACT check of `push_reordered_wall_loop` / `begin_region` semantics if the builder behavior is in doubt (signatures at `begin_region:266` and `push_reordered_wall_loop:337` are stable).

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` — reconcile the existing reopened TASK-120c row (close or re-scope) - `grep -E '^- \[[x ]\] TASK-120c ' docs/07_implementation_status.md | grep -q '170-seam-livepath-audit' && ! grep -qE '^- \[~\] TASK-120c ' docs/07_implementation_status.md && echo PASS` (same falsifying check as AC-4; fails today because the row is still `[~]` with no packet-170 reference).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
