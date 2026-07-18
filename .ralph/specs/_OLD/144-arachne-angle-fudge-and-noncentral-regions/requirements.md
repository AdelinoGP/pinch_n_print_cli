# Requirements: 144-arachne-angle-fudge-and-noncentral-regions

## Packet Metadata

- Grouped task IDs: **none** (provenanced by the second-pass Arachne parity
  audit `target/arachne_parity_audit_20260706_020657.md` findings N5 and N6,
  encoded as committed red tests at `b2ea52b7` — N5 indirectly via N1's red
  tests, N6 via a new dumbbell test written by this packet; no `docs/07`
  `TASK-###` exists for N1–N13, matching 113c's `none` precedent).
- Backlog source: `docs/07_implementation_status.md` (no `TASK-###` for N1–N13).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 141 (A1) left the π-cap workaround (`pipeline.rs:334`,
`effective_transitioning_angle_rad = std::f64::consts::PI`, self-described
"TEMPORARY") and the 0.1× filter-dist fudge (`pipeline.rs:272-277`, scaling
`transition_filter_dist` by 0.1) in place because they were load-bearing for
the centrality-gated junction scheme — A1's rewrite replaced junction
generation but kept the centrality pipeline that the π hack sustains. The audit
(N5) flags this: canonical `updateIsCentral` uses
`beading_strategy.getTransitioningAngle()` (defaults π/4 per
`BeadingStrategyFactory.hpp:49` / 60° per `BeadingStrategy.hpp:78`, ultimately
sourced from `wall_transition_angle` ~10°); with the canonical angle, a
square's diagonal spokes (`dR/dD = sin 45° ≈ 0.707`) are **non-central**; PNP's
`cap = sin(π/2) = 1` marks every non-degenerate spine edge central. This is
required to keep N1's central-gated junction scheme producing output at all;
once A1 fixed junction generation (junctions no longer centrality-gated), the
hack must be removed or centrality/transition placement will be wrong in the
opposite direction. Separately, `filterNoncentralRegions` (N6) is absent
(`SkeletalTrapezoidation.cpp:811-862`, called unconditionally at `:633` after
`updateBeadCount`): it promotes non-central gaps between same/±1-bead-count
central regions (within a hardcoded 0.4 mm) back to central and copies bead
counts across. Without it, central regions fragment across shallow pinch
points, producing separate domains (extra seams / short lines) where canonical
produces one continuous region. This packet removes both fudges (N5) and ports
`filterNoncentralRegions` (N6), strictly after A2 lands (the fudges are
load-bearing until A1/A2's canonical scheme is in place).

This packet supersedes `D-141-JUNCTION-BANDS` for the centrality-parameter
layer only; A1's junction geometry and A2's emission remain canonical and
untouched. C does not change the `BeadingStrategy` trait (B owns the trait
extension) — C only threads the already-existing `wall_transition_angle` (on
the trait at `beading/mod.rs:93`) through `filter_central`.

## In Scope

- **Delete the π workaround** in `crates/slicer-core/src/arachne/pipeline.rs:334`:
  remove `let effective_transitioning_angle_rad = std::f64::consts::PI;` and
  its associated "TEMPORARY" doc comment (`:325-333`). Replace with the
  configured `wall_transition_angle` already threaded through
  `BeadingFactoryParams` (`factory.rs:92,157,192`) and already exposed on the
  `BeadingStrategy` trait at `beading/mod.rs:93` (`wall_transition_angle() ->
  f64`). The strategy's `wall_transition_angle()` is the source of truth, not
  a hardcoded π.
- **Delete the 0.1× filter-dist fudge** in
  `crates/slicer-core/src/arachne/pipeline.rs:272-277` (`to_centrality_params`):
  remove the `* 0.1` scaling on `params.transition_filter_dist`. The user-facing
  `transition_filter_dist` maps directly to `CentralityParams::transition_filter_dist` — the 0.1× scaling was an ad-hoc workaround for the
  0.15 mm thin-wall test strip, now handled by A1's rib-based topology.
- **Thread the configured `wall_transition_angle`** through `filter_central`
  in `pipeline.rs:335-339`: call `strategy.wall_transition_angle()` (or read
  from `BeadingFactoryParams::wall_transition_angle`) and pass it as the
  `transitioning_angle_rad` argument to `filter_central`, replacing the
  hardcoded `effective_transitioning_angle_rad`.
- **Port `filterNoncentralRegions`** in
  `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`: a new
  `filter_noncentral_regions` function mirroring
  `SkeletalTrapezoidation.cpp:811-862`. Promotes non-central gaps between
  same/±1-bead-count central regions (within a hardcoded 0.4 mm = 4000 slicer
  units) back to central and copies bead counts across. Called
  unconditionally after `assign_bead_counts` in `pipeline.rs` (mirroring
  `:633`'s "after `updateBeadCount`" ordering).
- **DO NOT wire the whisker-dissolve** `filterCentral` (`SkeletalTrapezoidation.cpp:716-730`):
  it is DEAD CODE upstream (self-contradictory condition). PNP's un-wired
  whisker-dissolve helpers (`centrality.rs:263-389`) mirror this dead code;
  leave them as-is. The audit explicitly flags this as a gotcha — do not "fix"
  PNP by wiring the dissolve in.
- **New dumbbell test** `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs`
  (NEW): a dumbbell-shaped polygon (two 3 mm-wide pads joined by a 0.35 mm neck)
  — canonical keeps one central region; assert single stitched inset-0 ring
  pair rather than four fragments. This test is NOT a pre-committed red test
  (N6 needs the N1/N2 rewrite to be observable); C writes it as a positive
  regression test.
- **Fixture re-baseline (this packet's own stage only)**:
  `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` — re-record
  via self-capture (C changes the centrality angle parameter + adds
  `filter_noncentral_regions`, so the centrality fixtures drift).
- **Deviation-log entry**: `D-144-ANGLE-FUDGE-NONCENTRAL` (new ID, addendum
  on `D-141-JUNCTION-BANDS` noting C removes the π hack A1 left in place,
  supersession pattern — no in-place edits to A1's narrative).
- **Scope decision (NOT a silent absorb)**: C's removal of the π hack changes
  the runtime behavior of `filter_central` for every polygon — the configured
  `wall_transition_angle` (default 10°) is now the actual gate, not π. This is
  the intended behavior change (canonical parity), but it will shift
  centrality classification for many fixtures. Surface this in the commit
  message; the centrality fixture re-baseline records the drift.

## Out of Scope

- **N1/N7 (junction geometry + BeadingPropagation)** — A1 (`141`). C does not
  change junction generation.
- **N2/N4 (`perimeter_index` + `is_odd`)** — A2 (`142`).
- **N3/N8 (transition ends + extra ribs)** — B (`143`). C does not touch
  `apply_transitions` or the `BeadingStrategy` trait extension.
- **N9–N13** — Packets D (`145`), E (`146`), F (`147`).
- **`cube_4color.3mf` e2e closure gate** — record-only across C (per
  `docs/specs/arachne-parity-N1-N13-plan.md` cross-cutting policy); Packet F
  blocks on green. C records the failure delta in its commit message.
- **`cargo test --workspace`** — only at Packet F's closure ceremony.
- **New WIT/IR schema changes** — C's surface is `slicer-core`-internal; no
  WIT/IR change. C does NOT extend the `BeadingStrategy` trait (B owns the
  trait extension) — C only threads the already-existing
  `wall_transition_angle`.
- **`OrcaSlicerDocumented/` C++ oracle build** — declined.
- **Wiring the whisker-dissolve `filterCentral`** — explicitly out of scope
  (dead code upstream; do not wire).

## Authoritative Docs

- `docs/15_config_keys_reference.md` — §"Arachne beading strategy stack" (lines
  ~479-521); `wall_transition_angle` (10.0°), `wall_transition_filter_deviation`
  (1000 units = 0.1 mm). Read directly.
- `docs/08_coordinate_system.md` — §"Constant Conversion Table" (~30 lines);
  0.4 mm = 4000 units (the hardcoded `filterNoncentralRegions` distance).
  Delegate if > 300 lines.
- `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS` entry — read full;
  purpose: A1's addendum noting C removes the π hack.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies.
- `.ralph/specs/113c-arachne-faithful-graph-construction/requirements.md`
  §"OrcaSlicer Reference Obligations" (the `orca-delegation` snippet) — C
  carries this contract forward verbatim.

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:811-862` — `filterNoncentralRegions` (promote non-central gaps within 0.4 mm back to central, copy bead counts across same/±1-bead-count regions).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:633` — call site (unconditional, after `updateBeadCount`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:716-730` — dead `filterCentral` whisker-dissolve (self-contradictory condition — DO NOT wire; confirms PNP's `centrality.rs:263-389` helpers correctly mirror dead code).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h:78` — canonical `getTransitioningAngle` default (60°).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategyFactory.hpp:49` — `getTransitioningAngle` factory default (π/4).

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (N1 red tests still green with configured angle
  threaded), `AC-2` (dumbbell polygon produces one central region, not four
  fragments) from `packet.spec.md`.
- Negative cases: `AC-N1` (π hack and 0.1× fudge gone from `pipeline.rs`).
- Cross-packet impact: C normalizes centrality for D's
  `generateLocalMaximaSingleBeads` (which reads the normalized centrality).
  C does not unblock any packet directly (D/E/F don't depend on C's specifics
  for their own acceptance, but D benefits from the normalized centrality).
- Refinements not captured in Given/When/Then:
  - C threads the **already-existing** `wall_transition_angle` (on the trait
    at `beading/mod.rs:93`, threaded via `BeadingFactoryParams` at
    `factory.rs:92,157,192`). C does NOT extend the `BeadingStrategy` trait
    (B owns the trait extension); C only changes the call site in
    `pipeline.rs:335-339` from a hardcoded π to `strategy.wall_transition_angle()`
    (or the `BeadingFactoryParams` field).
  - The 0.1× filter-dist fudge is deleted entirely; `to_centrality_params`
    passes the user's `transition_filter_dist` directly.
  - `filterNoncentralRegions`'s 0.4 mm distance is in slicer units (4000 units;
    1 unit = 100 nm per `docs/08_coordinate_system.md`).
  - The dumbbell test is a NEW positive regression test, not a pre-committed
    red test (N6 needs the N1/N2 rewrite to be observable; C is the first
    packet that can write it).
  - C does NOT wire the whisker-dissolve `filterCentral` — it is dead code
    upstream.
  - C's removal of the π hack changes runtime behavior for every polygon (the
    configured 10° is now the actual gate, not π); the centrality fixture
    re-baseline records the drift.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate
subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 \| tee target/test-output-c-ac1.log` | AC-1: N1 red tests stay green with configured angle | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_filter_noncentral_regions --nocapture 2>&1 \| tee target/test-output-c-ac2.log` | AC-2: dumbbell single central region | FACT pass/fail |
| `rg -q 'std::f64::consts::PI' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1` | AC-N1: π hack gone | FACT pass (exit 1 from rg = no match) |
| `rg -q '\* 0\.1' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1` | AC-N1: 0.1× fudge gone (in `to_centrality_params`) | FACT pass (exit 1) |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-c-stays-green.log` | N2/N4/N3 stay green (C doesn't regress A2/B) | FACT pass (expected) |
| `cargo test -p slicer-core --features host-algos --test centrality 2>&1 \| tee target/test-output-c-regression.log` | centrality regression (fixtures re-baselined) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/c-cube4color.gcode 2>&1 \| tail -5` then `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 \| tee target/test-output-c-e2e.log` | e2e closure delta (record-only per cross-cutting policy; C records the failure count in its commit msg, does NOT block on green) | FACT pass/fail + summary line (record-only) |
| `rg -q 'D-144-ANGLE-FUDGE-NONCENTRAL' docs/DEVIATION_LOG.md` | Deviation log entry present | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (C's surface is `slicer-core`-internal; no guest feed) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **C must keep N1, N2, N3, N4 red tests GREEN.** C builds on A1/A2/B;
  regressing any means backing out. The "stays green" verification command
  gates this.
- **C must NOT wire the whisker-dissolve `filterCentral`.** It is dead code
  upstream (`SkeletalTrapezoidation.cpp:716-730`, self-contradictory
  condition). PNP's un-wired helpers (`centrality.rs:263-389`) correctly mirror
  this dead code; leave them. The audit explicitly flags this as a gotcha.
- **C must NOT extend the `BeadingStrategy` trait.** B (`143`) owns the trait
  extension (`get_transitioning_length` / `get_transition_anchor_pos` /
  `get_nonlinear_thicknesses`). C only threads the **already-existing**
  `wall_transition_angle` (`beading/mod.rs:93`) through `filter_central`.
- **`filterNoncentralRegions`'s 0.4 mm distance is in slicer units** (4000
  units; 1 unit = 100 nm per `docs/08_coordinate_system.md`). Divide OrcaSlicer's
  0.4 mm by the unit factor.
- **C's removal of the π hack changes runtime behavior for every polygon.**
  The configured 10° is now the actual gate, not π. The centrality fixture
  re-baseline records the drift; the commit message surfaces this as a scope
  decision.
- **Fixture re-baseline is atomic per fixture and records rationale.**
  `centrality_*.json` drift because the angle parameter changes + the new
  `filter_noncentral_regions` runs. Never read the JSONs directly — re-record
  via the self-capture pattern.
- **Deviation-log correction uses the supersession pattern** — new
  `D-144-ANGLE-FUDGE-NONCENTRAL` + addendum on `D-141-JUNCTION-BANDS`. No
  in-place edits to A1's narrative.

## Context Discipline Notes

Packet-specific context-budget hazards:

- `crates/slicer-core/src/arachne/pipeline.rs` (~446 LOC) is the primary edit
  target for Step 1 — range-read `:260-340` (the `to_centrality_params` fudge +
  the π hack + `filter_central` call); do NOT full-read (the file's
  `assign_perimeter_indices` at `:384-390` is A2's scope, already deleted by
  A2; the stage wiring at `:340-360` is B's scope).
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (~389 LOC) is
  the primary edit target for Step 2 — range-read `:100-200` (the
  `updateIsCentral` predicate + `filter_central`) and `:260-390` (the un-wired
  whisker-dissolve helpers — read-only confirmation they mirror dead code, do
  NOT wire).
- `crates/slicer-core/src/beading/mod.rs` — read-only for C (the
  `wall_transition_angle` trait method at `:93` is already there; C only calls
  it from `pipeline.rs`). Do NOT edit `beading/` — B owns the trait extension.
- Likely temptation reads to skip: `OrcaSlicerDocumented/` (delegate via the
  contract), `modules/core-modules/arachne-perimeters/` (C's surface is
  `slicer-core`-internal), `slicer-sdk`/`slicer-wasm-host` (no WIT change).
- Sub-agent return-format hints for the heaviest dispatches: the
  `filterNoncentralRegions` SUMMARY dispatch (`SkeletalTrapezoidation.cpp:811-862`)
  should request the promote-back condition (same/±1-bead-count within 0.4 mm)
  + the bead-count copy rule explicitly. The dead `filterCentral` SUMMARY
  (`:716-730`) should request the self-contradictory condition explicitly (to
  confirm PNP's helpers correctly mirror dead code, not to wire them).