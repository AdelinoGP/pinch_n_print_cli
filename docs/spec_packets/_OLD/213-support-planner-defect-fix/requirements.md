# Requirements: support-planner-defect-fix

## Packet Metadata

- Grouped task IDs: `TASK-329`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After propagated nodes merge, the surviving lone node has no MST edge and `dist_to_top > 0`, so the planner emits no segment and the support column ends in mid-air. Independently, `tapered_radius` returns zero at the contact tip. This coherent planner slice restores plate-reaching geometry and a printable minimum tip width.

## In Scope

- Emit a degenerate per-layer segment for every surviving, non-dropped lone propagated node with `dist_to_top > 0` and no surviving MST edge.
- Add `MIN_BRANCH_RADIUS: f32 = 0.4` and clamp `tapered_radius` to `[MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS_MM]`.
- Preserve MST edge emission, fresh contact emission, collision exclusion, merge dropping, and `MAX_BRANCH_RADIUS_MM = 6.0`.
- Rebuild stale guest WASM and run the tree visual-debug requests at `PrePass::SupportGeometry` and `Layer::Support`.

## Out of Scope

- Fallback clipping and `needs_support` derivation (TASK-323).
- Raft, interface layers, support-type mode selection, and G-code roles.
- Numerical Orca radius/position parity.

## Authoritative Docs

- `docs/specs/support-generation-defect-verified-findings.md` - direct ranges listed in `packet.spec.md`.
- `docs/specs/support-generation-remediation-plan.md` - direct read, approved decisions and queue.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - `drop_nodes` and `draw_circles` behavior used only to adjudicate lone-node structural parity

## Acceptance Summary

- Positive: `AC-1` through `AC-3` in `packet.spec.md`.
- Negative: `AC-N1` in `packet.spec.md`.
- Cross-packet impact: exports the new `MIN_BRANCH_RADIUS` constant and plate-reaching planner geometry; TASK-323 must not alter planner output.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Detect stale guest artifacts before visual evidence | FACT pass/fail |
| `cargo test -p support-planner --all-targets` | Exercise planner helper and existing planner tests | FACT pass/fail; bounded failure SNIPPETS |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-tree-fixed.json --output target/vd-tree-fixed --overwrite` | Produce planner and consumer captures at layers 0, 50, 100, and 125 | FACT manifest exists; PNG inspection delegated |
| `cargo check --workspace --all-targets` | Workspace compile gate | FACT pass/fail |

## Step Completion Expectations

The guest rebuild must precede visual-debug. Planner geometry at lower layers must be checked before treating `Layer::Support` output as evidence.

## Context Discipline Notes

Do not load full `lib.rs`; use the verified emission and helper ranges. Delegate Orca reads and all cargo commands with bounded returns.
