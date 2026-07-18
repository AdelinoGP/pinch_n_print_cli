---
status: implemented
packet: 143-arachne-transition-ends-and-extra-ribs
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 143-arachne-transition-ends-and-extra-ribs

## Goal

Port canonical transition-end machinery (N3 — `filterTransitionMids` + `generateAllTransitionEnds` + `applyTransitions` at ends + fractional `transition_ratio` + beading interpolation at emission) and `generateExtraRibs` (N8), so bead-count changes ramp over the configured `wall_transition_length` instead of snapping at a single point, and long spine edges get intermediate width-sampling points at nonlinear-strategy breakpoints.

## Scope Boundaries

Extend the `BeadingStrategy` trait with `get_transitioning_length` / `get_transition_anchor_pos` / `get_nonlinear_thicknesses`, add the `generate_all_transition_ends` pipeline stage (filter mids → generate ends → apply at ends), port `generateExtraRibs`, and interpolate beadings at emission for nonzero `transition_ratio`. Update the N3 red-test call sites to invoke the new stage before `apply_transitions` (assertions untouched). Full in/out-of-scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `142-arachne-canonical-connectjunctions-emission` (A2 — B's beading interpolation reads A2's canonical junction fans; B's `generate_all_transition_ends` walks the graph A1/A2 produced).
- Unblocks: `144-arachne-angle-fudge-and-noncentral-regions` (C — independent code path once A2 lands, but C's `filterNoncentralRegions` interacts with B's transition regions).
- Activation blockers: none (audit encoded in committed red tests at `b2ea52b7`).

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** a single central twin-pair edge (v0 R=1mm bc=1 → v1 R=3mm bc=2) with a `TransitionMiddle` at pos 0.5 (the fixture from `arachne_parity_red_transition_ends.rs`), **when** `generate_all_transition_ends` + `apply_transitions` run, **then** `apply_transitions` does NOT produce exactly one new spine vertex exactly at the mid position carrying only `Some(1)` — canonical spawns a lower end and an upper end straddling the mid by the configured transition length, inserting nodes at END positions with bead counts `{1, 2}` (`SkeletalTrapezoidation.cpp:1247-1403`, `:1525-1526`).
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends -- n3_apply_transitions_creates_lower_and_upper_end_splits --nocapture 2>&1 | tee target/test-output-b-ac1.log`
- **AC-2. Given** a two-edge central chain where edge A is 0.2 mm long with the transition mid at its middle (so the upper end necessarily travels past shared vertex v1 onto edge B for any plausible transition length — default 0.4 mm), **when** `generate_all_transition_ends` + `apply_transitions` run, **then** shared vertex v1 has a fractional `transition_ratio` strictly between 0 and 1 — canonical `generateTransitionEnd`'s recursion (`SkeletalTrapezoidation.cpp:1331-1371`) assigns traversed nodes a fractional ratio that `generateSegments` (`:1712-1721`) uses to interpolate the beading between 1 and 2 beads.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends -- n3_transition_spilling_past_vertex_sets_fractional_ratio --nocapture 2>&1 | tee target/test-output-b-ac2.log`

## Negative Test Cases

- **AC-N1. Given** the `BeadingStrategy` trait was extended with `get_transitioning_length` / `get_transition_anchor_pos` / `get_nonlinear_thicknesses` (default implementations delegating to `self.parent` for the 4 decorators, `DistributedBeadingStrategy` returning its stored `default_transition_length` for `get_transitioning_length`), **when** `cargo check -p slicer-core --all-targets` runs, **then** all 5 concrete strategies compile without adding new required methods (the defaults absorb the extension) — no caller-side breakage.
  | `cargo check -p slicer-core --all-targets 2>&1 | tee target/test-output-b-neg1.log`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-b-gate.log`

## Authoritative Docs

- `docs/08_coordinate_system.md` — §"Constant Conversion Table" (~30 lines); 0.4 mm = 4000 units, 0.1 mm = 1000 units. Delegate if > 300 lines.
- `docs/15_config_keys_reference.md` — §"Arachne beading strategy stack" (lines ~479-521); `wall_transition_length` (4000 units = 0.4 mm), `wall_transition_filter_deviation` (1000 units = 0.1 mm). Read directly.
- `docs/DEVIATION_LOG.md` `D-112-PROPAGATION-ADAPT` + `D-141-JUNCTION-BANDS` + `D-142-CONNECTJUNCTIONS-EMISSION` entries — read full; substrate + A1/A2 addenda.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies.

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `docs/DEVIATION_LOG.md` — new entry `D-143-TRANSITION-ENDS` documenting the N3+N8 fix (canonical `filterTransitionMids` + `generateAllTransitionEnds` + `applyTransitions` at ends + fractional `transition_ratio` + `generateExtraRibs` + `BeadingStrategy` trait extension), with an addendum on `D-112-PROPAGATION-ADAPT` noting B supersedes the single-mid-split scheme. Supersession pattern.
  - `rg -q 'D-143-TRANSITION-ENDS' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:881-915` — `generateTransitioningRibs` (the full transition pipeline: mids → filter → ends → apply).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1007-1076` — `filterTransitionMids` (recursive dissolve of nearby same-`lower_bead_count` transitions within `transition_filter_dist`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1247-1403` — `generateAllTransitionEnds` (lower end backward on `edge.twin`, upper end forward, recursive travel onto successor edges, fractional `transition_ratio`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1487-1543` — `applyTransitions` at ends (insert nodes at END positions with `bead_count = lower` or `lower + 1` per `is_lower_end`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1579-1633` — `generateExtraRibs` (upward central edges ≥ `discretization_step_size`, insert rib nodes at every `getNonlinearThicknesses()` radius).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1712-1721` — `generateSegments` beading interpolation (`compute(thickness, bead_count)` ↔ `compute(thickness, bead_count + 1)` for nonzero `transition_ratio`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h` — `getTransitioningLength` / `getTransitionAnchorPos` / `getNonlinearThicknesses` trait surface.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.