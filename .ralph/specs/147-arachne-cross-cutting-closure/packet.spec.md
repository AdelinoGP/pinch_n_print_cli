---
status: draft
packet: 147-arachne-cross-cutting-closure
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 147-arachne-cross-cutting-closure

## Goal

Close the Arachne parity N1–N13 packet chain: re-green the `cube_4color.3mf` end-to-end outer-wall closure gate, re-baseline the cross-crate `slicer-runtime` perimeter_parity fixtures (the stragglers after A1–E's per-packet re-baselines), register deviation-log supersession entries for the chain, and author ADR `0035-arachne-faithful-emission-and-transitions.md`.

## Scope Boundaries

This packet owns NO finding fixes (N1–N13 are owned by A1–E). F is the cross-cutting closure: the e2e closure gate (record-only across A1–E, blocking in F), the final cross-crate fixture batch, the deviation-log chain supersession, and ADR 0035. Full in/out-of-scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `141` (A1), `142` (A2), `143` (B), `144` (C), `145` (D), `146` (E) — ALL must be `status: implemented` before F can close. F is the closure gate for the whole chain.
- Unblocks: the Arachne parity N1–N13 chain is complete when F closes.
- Activation blockers: ALL of A1–E must be `status: implemented` (their red tests green, their per-packet fixtures re-baselined, their deviation-log entries present). F cannot close until the chain is green.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** A1–E are all `status: implemented` (N1–N13 fixes in place), **when** `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/f-cube4color.gcode` runs, **then** `cube_4color_arachne_outer_walls_close_end_to_end` passes — every outer-wall sub-loop closes end-to-end (gap ≤ 0.30 mm) across all layers. This is the cross-chain e2e closure gate that was record-only across A1–E; F blocks on green.
  | `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 | tee target/test-output-f-ac1.log`
- **AC-2. Given** A1–E's per-packet fixture re-baselines are in place, **when** `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1` runs, **then** all `perimeter_parity` Arachne fixtures pass — the cross-crate `slicer-runtime` fixtures (re-recorded via their `#[ignore]`d `record_*` functions) reflect the canonical pipeline. The fixtures: `tapered_wedge`, `narrow_strip_widening`, `max_bead_count_cap`, `complex_multi_feature`, `cube_4color_arachne`.
  | `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/test-output-f-ac2.log`

## Negative Test Cases

- **AC-N1. Given** the full Arachne parity chain (A1–F) is in place, **when** `cargo xtask test --workspace --summary 2>&1` runs, **then** the summary reports PASS (the full workspace test suite is green — the closure ceremony for the chain). This is the ONE packet-level entry where `cargo test --workspace` is permitted (per `docs/specs/arachne-parity-N1-N13-plan.md` test discipline: only at Packet F's closure ceremony).
  | `cargo xtask test --workspace --summary 2>&1 | tee target/test-output-f-neg1.log`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask test --workspace --summary 2>&1 | tee target/test-output-f-gate.log`

## Authoritative Docs

- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies (e2e record-only→block-in-F, fixture re-baseline distributed-per-packet→F-closes-stragglers, deviation-log supersession pattern, ADR 0035).
- `docs/DEVIATION_LOG.md` — all `D-11X-*` entries (A1–E's); read full; F adds the chain-closure addendum.
- `docs/adr/0034-arachne-faithful-graph-construction.md` — read full (short); ADR 0035 follows it.

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `docs/DEVIATION_LOG.md` — new entry `D-147-CHAIN-CLOSURE` documenting the chain closure (all N1–N13 fixes in place, e2e closure gate green), with addenda on each of `D-141-JUNCTION-BANDS`, `D-142-CONNECTJUNCTIONS-EMISSION`, `D-143-TRANSITION-ENDS`, `D-144-ANGLE-FUDGE-NONCENTRAL`, `D-145-LOCAL-MAXIMA-EPILOGUE`, `D-146-POSTPROCESS-ORDER` noting the chain is closed. Supersession pattern.
  - `rg -q 'D-147-CHAIN-CLOSURE' docs/DEVIATION_LOG.md`
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW) — records the architectural decision for the chain: canonical `generateJunctions`/`connectJunctions` emission + transition ends + `filterNoncentralRegions` + local maxima + post-process order, superseding the PNP "ADAPTATION" divergence. Authored alongside F's closure.
  - `rg -q '0035-arachne-faithful-emission-and-transitions' docs/adr/0035-arachne-faithful-emission-and-transitions.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- F owns NO finding fixes, so it has NO new OrcaSlicer parity refs. The chain's OrcaSlicer refs are owned by A1–E (see their `requirements.md` §OrcaSlicer Reference Obligations). F's ADR 0035 references the chain's parity surface but does not introduce new refs.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.