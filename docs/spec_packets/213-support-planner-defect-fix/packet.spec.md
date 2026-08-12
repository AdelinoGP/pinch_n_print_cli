---
status: superseded
packet: 213-support-planner-defect-fix
task_ids:
  - TASK-329
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
superseded_by: support-families-and-anchored-entities sequence (packets 219-224); lone-node/radius-floor work retained only where it survives inside the new tree family (221-tree-support-family)
superseded_on: 2026-08-12
---

# Packet Contract: 213-support-planner-defect-fix

> **SUPERSEDED 2026-08-12 by the support-families and anchored-entities sequence** (`docs/specs/support-families-anchored-entities-plan.md`). The tree planner is split and renamed into `tree-support-planner` (packet 221); this packet's lone-node vertical-column and radius-floor work is retained only where it remains valid inside that new tree family, and its degenerate-disk renderer result is not closure evidence. Directory retained intact for provenance; do not implement as-is.

## Goal

Make support-planner propagation emit a printable vertical segment for every surviving lone node and keep every tapered branch radius at or above `MIN_BRANCH_RADIUS = 0.4` while retaining the `MAX_BRANCH_RADIUS_MM = 6.0` ceiling.

## Scope Boundaries

This packet changes only the support-planner emission loop and radius helper. It validates the planner at `PrePass::SupportGeometry` and its consumer at `Layer::Support` using the verified tree reproduction; it does not change fallback fillers, IR/WIT schemas, raft, interface layers, support variants, or G-code emission.

## Prerequisites and Blockers

- Depends on: approved `docs/specs/support-generation-remediation-plan.md`; verified `docs/specs/support-generation-defect-verified-findings.md`
- Unblocks: packets 3-6 in the approved support-generation queue
- Activation blockers: none

## Acceptance Criteria

- **AC-1. Given** a surviving `active_nodes` entry with `dist_to_top > 0`, no surviving MST edge, and `drop[i] == false`, **when** `plan_for_object` emits the current layer, **then** `branch_segments` contains one degenerate two-point segment for that node at `z_current`, with equal `x`, `y`, and non-zero endpoint widths, and the source contains this guard in the emission loop. | `python3 -c "from pathlib import Path; p=Path('modules/core-modules/support-planner/src/lib.rs').read_text(); assert 'dist_to_top > 0' in p and 'branch_segments.push(vec![point, point])' in p"`
- **AC-2. Given** `tmp/visual-debug-tree-fixed.json` requests both planner and consumer taps at layers 0, 50, 100, and 125, **when** `visual-debug` renders it, **then** the output bundle contains `manifest.json`, includes both `PrePass::SupportGeometry` and `Layer::Support` captures for every requested layer, and the planner geometry is present at layers 100, 50, and 0 rather than stopping around layer 100. The `Layer::Support` typed capture at layers 0 and 50 must each contain at least one degenerate two-point branch segment with equal endpoint `x`, `y`, and `z`, and non-zero endpoint widths, proving the lone-node emission reached the consumer. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-tree-fixed.json --output target/vd-tree-fixed --overwrite >/dev/null && test -f target/vd-tree-fixed/manifest.json && for tap in PrePass::SupportGeometry Layer::Support; do for layer in 125 100 50 0; do rg -q -U "\"tap\": \"$tap\",\n[[:space:]]+\"layer_index\": $layer" target/vd-tree-fixed/manifest.json || exit 1; done; done && rg -q -U '"tap": "PrePass::SupportGeometry",\n[[:space:]]+"layer_index": 100' target/vd-tree-fixed/manifest.json && rg -q -U '"tap": "PrePass::SupportGeometry",\n[[:space:]]+"layer_index": 50' target/vd-tree-fixed/manifest.json && rg -q -U '"tap": "PrePass::SupportGeometry",\n[[:space:]]+"layer_index": 0' target/vd-tree-fixed/manifest.json && python3 -c "import json; m=json.load(open('target/vd-tree-fixed/manifest.json')); c=[e for e in m['images'] if e['tap']=='Layer::Support' and e['layer_index'] in (0,50)]; assert len(c)==2 and all(any(len(p['points'])==2 and all(a[k]==b[k] for k in ('x','y','z')) and a['width']>0 and b['width']>0 for p in e['typed_capture']['value']['support_paths'] for a,b in [p['points']]) for e in c)"`
- **AC-3. Given** `dist_to_top == 0` and `dist_to_top > 0` inputs to `tapered_radius`, **when** the helper is evaluated with the normal branch parameters, **then** its result is clamped with lower bound `MIN_BRANCH_RADIUS` and upper bound `MAX_BRANCH_RADIUS_MM`, and `MIN_BRANCH_RADIUS` is exactly `0.4`. | `rg -q 'const MIN_BRANCH_RADIUS: f32 = 0\.4' modules/core-modules/support-planner/src/lib.rs && rg -q 'raw\.clamp\(MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS_MM\)' modules/core-modules/support-planner/src/lib.rs`

## Negative Test Cases

- **AC-N1. Given** a node marked dropped by the merge pass or a node whose propagated point is rejected by collision checks, **when** the emission loop runs, **then** no lone-node segment is emitted for that node, and no existing `drop[*a.max(b)] = true` or collision guard is removed. | `rg -q 'drop\[\*a\.max\(b\)\] = true' modules/core-modules/support-planner/src/lib.rs && rg -q 'point_in_any_expoly\(collision_polys' modules/core-modules/support-planner/src/lib.rs`

## Verification

- `cargo xtask build-guests --check`
- `cargo check --workspace --all-targets`
- `cargo test -p support-planner --all-targets`

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct read, lines 14-53 and 55-64
- `docs/specs/support-generation-defect-verified-findings.md` - direct read, lines 56-86, 128-136, 138-177, 178-231, and 255-284
- `docs/01_system_architecture.md` - `PrePass::SupportGeometry` and `Layer::Support` contracts
- `docs/08_coordinate_system.md` - coordinate units and mm boundary

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - `drop_nodes` and `draw_circles` lone-node continuation behavior

## Doc Impact Statement (Required)

**`none`** - no IR, WIT, scheduler, claim, manifest, host-service, SDK contract, or documentation contract changes.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

 Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

## Closure Summary

Closed 2026-08-11: planner lone-node emission + filament_lines renderer fix (Shape::Disk + Canvas::fill_disk) so degenerate branch_segments render as width-derived disks at propagated layers; visual-debug PNGs at l0/l50 now distinct and >6KB.

## Reopen Notes

Reopened 2026-08-11: renderer fix is incomplete at l100 (still renders single pixel), and the disk position overlaps the model wall instead of sitting below an overhang.
