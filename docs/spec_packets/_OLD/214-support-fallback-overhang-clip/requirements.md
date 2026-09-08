# Requirements: support-fallback-overhang-clip

## Packet Metadata

- Grouped task IDs: `TASK-323`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Fallback support modules currently iterate every `region.polygons()` expolygon, so they fill model interiors. The host already computes region-clipped `overhang_areas`, but `needs_support` is hardcoded true. Reusing those existing values is one coherent fallback eligibility and fill-boundary fix.

## In Scope

- In traditional-support and tree-support `run_support`, use `region.overhang_areas()` for the DefaultEligible fill path.
- Preserve `region.polygons()` for SupportPaintPolicy::Enforced and preserve Blocked precedence.
- Set `SliceRegionData.needs_support` to `!overhang_areas.is_empty()` in `sliced_region_to_data`.
- Add targeted regression coverage for fallback clipping, enforcement, and host marshalling, then rebuild guests and run visual-debug.

## Out of Scope

- Planner lone-node propagation and tip radius (TASK-322).
- Adding or removing IR/WIT fields; `needs_support` and `overhang_areas` already exist at the host and SDK boundary.
- Overhang detection, quartile computation, paint-policy semantics, raft, interfaces, support variants, and G-code roles.

## Authoritative Docs

- `docs/specs/support-generation-defect-verified-findings.md` - direct ranges listed in `packet.spec.md`.
- `docs/specs/support-generation-remediation-plan.md` - direct read, approved decisions and queue.

## Acceptance Summary

- Positive: `AC-1` through `AC-3` in `packet.spec.md`.
- Negative: `AC-N1` in `packet.spec.md`.
- Cross-packet impact: exports no new symbol; later packets consume the corrected existing `needs_support` and `overhang_areas` behavior.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Detect stale guest artifacts | FACT pass/fail |
| `cargo test -p slicer-wasm-host --all-targets` | Exercise boundary contract fixtures | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p traditional-support --all-targets` | Exercise fallback fill behavior | FACT pass/fail |
| `cargo test -p tree-support --all-targets` | Exercise tree fallback behavior | FACT pass/fail |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support.json --output target/vd-support-fixed --overwrite` | Render clipped fallback at layers 10/24/30 | FACT manifest; PNG inspection delegated |
| `cargo check --workspace --all-targets` | Workspace compile gate | FACT pass/fail |

## Step Completion Expectations

Host derivation must land before module fallback assertions are treated as meaningful; guest freshness must be checked before visual-debug. Enforced behavior must remain full-polygon behavior.

## Context Discipline Notes

Use bounded ranges around `run_support` and `sliced_region_to_data`; do not browse generated WIT or full test output. Delegate cargo commands with FACT returns.
