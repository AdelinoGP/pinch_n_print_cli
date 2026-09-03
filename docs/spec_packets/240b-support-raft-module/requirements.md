# Requirements: 240b-support-raft-module

## Packet Metadata

- Grouped task IDs: `TASK-414`..`TASK-418`, `TASK-535`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` (§11 queue row 7, §12 brief "240-support-raft"); gap register row G-06
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

G-06 states the raft situation exactly: "the IR exists, the consumer does not."
`RaftPlan` is produced by the tree planner's `push_raft_plan` when
`support_raft_layers > 0` and merged into the blackboard by `raft_plan_min`
(`crates/slicer-runtime/src/blackboard.rs`), and nothing renders it. Every raft
config key is dead in the four support modules.

The role/claim plumbing half-exists: `ExtrusionRole::RaftInfill` is a real
variant (`crates/slicer-ir/src/slice_ir.rs`) and `SliceRegionView::should_emit`
(`crates/slicer-sdk/src/views.rs`) already maps it to `"claim:raft-fill"` — but
no manifest anywhere declares that claim, so raft emission is suppressed
everywhere.

**240a-support-raft-substrate** removes the four structural blockers that made
writing the consumer impossible (unsigned indices, no way to create a prefix
band, three consumers assuming `index == Vec position`, and a sign-truncating
guest bridge) and adds the two carriers the consumer needs
(`SlicedRegion.raft_fill`, `paint-region-layer-view.raft-plan`). This packet
builds the consumer on top of it and closes G-06.

It absorbs the consumer half of deleted-draft 215-raft-geometry per plan §10;
that directory was already deleted by 236 (AC-10), and the mapping is recorded
below rather than re-litigated.

### Absorption mapping from 215-raft-geometry (plan §10)

- New module `com.core.raft-default` (`Layer::Infill` synthesizer) holding
  `claim:raft-fill`; reads `SupportPlanIR.raft_plan`, `SliceIR`, `LayerPlanIR`;
  writes `SlicedRegion.raft_fill` with deterministic fill polygons.
  Extrusion-path conversion happens downstream under the claim-holder path
  (design.md §ADR-0009 Reconciliation). — **this packet.**
- ADR-0009 contract preserved: rafts stay signed negative global-layer prefix
  entries, never anchored entities. — **substrate in 240a, honored here.**
- Signed-index migration `u32`→`i32`. — **240a.**
- Issue-19/20 raft keys `raft_contact_distance`, `raft_expansion`,
  `raft_first_layer_expansion`, plus wire-or-record for the existing dead raft
  keys in the four support modules. — **this packet.**
- DEV-124 check while the raft path is open. — **filed by 240a, re-verified
  here** (see §DEV-124 Re-verification).

## In Scope

- New guest module `modules/core-modules/raft-default/`: `Cargo.toml`,
  `raft-default.toml` manifest, `wit-guest/` binding the existing
  `slicer:layer-infill` world, and guest `src/lib.rs` implementing
  `LayerModule::run_infill`, holding `claim:raft-fill`, synthesizing
  deterministic raft footprint polygons (boundaries + inflation staging only)
  into `SlicedRegion.raft_fill`.
- Raft geometry semantics ported from canonical `generate_raft_base`:
  first-layer expansion (`raft_first_layer_expansion`), contact/inflate staging
  ("inflate in multiple steps to avoid leaking" preserved as iterated offsets),
  base-raft vs interface-raft layer loops, expansion via `raft_expansion`,
  honoring `RaftPlan.raft_layers` / `.base_raft_layers` /
  `.interface_raft_layers`.
- Config keys `raft_contact_distance` / `raft_expansion` /
  `raft_first_layer_expansion` declared in the new manifest's
  `[config.schema]` with canonical defaults, each read by the geometry it
  controls.
- Wire-or-record decisions for every dead raft key in the four support-module
  manifests, written into §Wire-or-Record Decisions below.
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs` (T8).
- Claim-conflict behavior for a second `claim:raft-fill` holder (AC-N1).
- Raft-band bounds rejection (AC-N2) and the undeclared-key rejection test
  (AC-N3).
- Formal ADR-0009 amendment plus the `D-<pkt>-ADR-0009-AMENDED` deviation row.
- DEV-124 re-verification under a live raft (AC-8).
- Human Validation Gate artifacts (`packet.spec.md` §Human Validation Gate).

## Out of Scope

- Everything 240a owns: the signed-index migration, the raft prefix band and
  its WIT marking, the positional-consumer repair, the `SlicedRegion.raft_fill`
  carrier and its WIT accessors, the `paint-region-layer-view.raft-plan`
  accessor, and the SliceIR schema bump. If any of it needs changing, that is a
  240a defect to route back, not work to absorb here.
- Extrusion-path, flow, speed, or role-tagged rendering inside
  `com.core.raft-default` — ADR-0009 Decision 4's zero-pattern-algorithm clause
  and the "Do not re-suggest making `raft-default` a renderer" Future-Reviewer
  Note are preserved unchanged.
- Editing `rectilinear-infill` or any other pattern module. The claim rides the
  existing machinery; any holder-side wiring discovered to be missing is a
  follow-up recorded in `design.md` §ADR-0009 Reconciliation, not silent scope
  growth.
- Pattern variety beyond v1 (grid / honeycomb / lightning raft via alternative
  `claim:raft-fill` holders) — future work; the claim stays reassignable by
  manifest change alone.
- Tree/traditional planner algorithm fidelity (238b) and renderer
  flow/density/interface semantics (238c) — raft reuses their outputs, fixes
  none.
- DEV-124's deliberately-unported `has_bottom_shell_layers` residual — record,
  do not port.
- Ironing / filament feature-gap keys — separate track.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - ~750 lines; direct range reads of §7, §8, §9, §10, §12 only.
- `docs/adr/0009-raft-as-layer-infill-role.md` - 93 lines; direct read.
- `docs/specs/support-parity-gap-register.md` - G-06 row only; direct range read.
- `docs/spec_packets/240a-support-raft-substrate/design.md` - §Migration Table and §`raft_plan` Read-Path Footprint only.
- `docs/15_config_keys_reference.md` - regenerated, not read in bulk.
- `docs/19_visual_debug.md` / `docs/17_agent_debugging.md` - human-gate bundle only; delegated SUMMARY.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_raft_base`: object-independent raft construction; first-layer expansion (`inflate_factor_1st_layer` = `raft_first_layer_expansion`), contact/inflate staging, base/interface loops, multi-step inflation, classic vs organic branches.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `init_fff_params`: defaults `raft_contact_distance = 0.1`, `raft_expansion = 1.5`, `raft_first_layer_expansion = 2.0`.

## Wire-or-Record Decisions

AC-7 is satisfied by this table, not by a test. Step 5 fills the Verdict and
Reason columns; the table below is the scaffold and MUST list one row per
(key, manifest) pair the step inspects. Each row must begin with a pipe followed by the backticked key — the literal
shape ``| `raft_<key>` | `<manifest>` | <verdict> | <reason> |`` — so AC-7's
count grep matches it; and no row may still read `_pending Step 5_` at closure.
AC-7 asserts both. A row whose verdict is `stays dead` must name the owner of
that decision.

| Key | Manifest | Verdict | Reason / decision owner |
| --- | --- | --- | --- |
| `raft_contact_distance` | `tree-support-planner` | _pending Step 5_ | |
| `raft_contact_distance` | `traditional-support-planner` | _pending Step 5_ | |
| `raft_expansion` | `tree-support` | _pending Step 5_ | |
| `raft_first_layer_expansion` | `traditional-support` | _pending Step 5_ | |

Step 5 expands this table to the actual key set each manifest declares — the
four rows above are the minimum AC-7 enforces, not the expected total. Enumerate
the real set with a dispatched grep before filling it in; do not assume this
scaffold is complete.

## DEV-124 Re-verification

DEV-124 (`docs/DEVIATION_LOG.md`, Status: "Closed — 2026-08-07: fixed the same
day") makes both perimeter generators gate `only_one_wall_first_layer` on
`layer_index == support_raft_layers`, pinned by
`classic_clamp_follows_raft_layers_not_layer_zero` and
`classic_clamp_unchanged_when_no_raft_configured` in
`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`.

240a's `requirements.md` §DEV-124 Reopen establishes that this predicate is
**index-convention-dependent**: it is correct under a positive prefix band, and
wrong under the signed negative band this family adopted, where the first
printed model layer is index `0`. 240a files the reopen row; this packet is
where the raft path actually goes live, so this is where the predicate is
re-verified against real behaviour.

Step 7 records here, with evidence:

- The AC-8 outcome for each of the two pinned tests (pass, or the failing
  assertion).
- If either fails: the corrected predicate, the fix applied to the perimeter
  generators, and the deviation row updated. Do NOT widen or weaken the
  assertions to make them pass — the tests encode canonical behaviour.
- Residual recorded, unchanged and deliberately unported: canonical's
  `has_bottom_shell_layers` conjunct is unconditionally true under PnP's
  `ResolvedConfig` range [1, 10]; revisit only if that range ever admits 0.

_(Outcome pending Step 7.)_

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest + fresh artifact), `AC-2` (single claim holder +
  `should_emit`), `AC-3` (deterministic `raft_fill` across two runs and both
  legs), `AC-4` (expansions and interface spacing honored), `AC-5`
  (negative-prefix ordering, zero anchored entities), `AC-6` (keys declared and
  wired), `AC-7` (four-manifest wire-or-record table), `AC-8` (DEV-124
  re-verification).
- Negative: `AC-N1` (double-holder `SchedulerError::ClaimConflict`), `AC-N2`
  (out-of-band negative index rejected), `AC-N3` (undeclared key rejected, not
  silently defaulted).
- Cross-packet impact: hard-depends on 240a; feeds 242's closure evidence;
  leaves 239/241 untouched.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure-gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'id = "com.core.raft-default"' modules/core-modules/raft-default/raft-default.toml && rg -q 'claim:raft-fill' modules/core-modules/raft-default/raft-default.toml && rg -q 'Layer::Infill' modules/core-modules/raft-default/raft-default.toml && cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` | AC-1 manifest + freshness | FACT exit code |
| `mkdir -p target && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- ac4_raft_fill_claim_emits_raft_infill --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 claim dispatch | FACT pass/fail |
| `test "$(rg -l 'claim:raft-fill' modules/core-modules/*/[a-z-]*.toml \| wc -l)" -eq 1` | AC-2 exactly one declared holder | FACT exit code |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_fill_is_deterministic_across_two_runs --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 determinism | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_first_layer_expansion_exceeds_upper_layers --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 expansions | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry_orders_before_model_layers --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- raft_mints_no_anchored_entities --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 ordering / no anchored (each command tees and is guarded separately) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_keys_declared_and_wired --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-6 keys wired | FACT pass/fail |
| `test "$(rg -c '^\| .raft_[a-z_]+. \|' docs/spec_packets/240b-support-raft-module/requirements.md)" -ge 4 && ! rg -q 'pending Step 5' docs/spec_packets/240b-support-raft-module/requirements.md` | AC-7 wire-or-record table filled | FACT exit code |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test contract -- classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-8 DEV-124 re-verification (each command tees and is guarded separately) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-scheduler --test raft_claim_conflict_tdd -- raft_fill_double_holder_conflicts --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1 conflict advisory | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_index_outside_band_rejected --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2 bounds rejection | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- undeclared_raft_key_is_rejected_not_defaulted --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3 undeclared key | FACT pass/fail |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

All commands name `--exact` tests plus a non-zero matched-count guard, or are
pure exit-code checks; none invokes `cargo test --workspace`.

## Step Completion Expectations

- 240a's AC-1..AC-7 must be green before Step 1. Verify, do not assume.
- Steps land in order Step 1 → Step 7.
- Guest-facing edits require `cargo xtask build-guests --check` before
  attributing any test result (T4/E4); the new guest requires an actual rebuild
  (drop `--check`) inside the creating step.
- Every new test file under an aggregated `slicer-runtime` binary carries its
  `mod` registration in the same step (T2 blindness).

## Context Discipline Notes

- Never open `OrcaSlicerDocumented/` directly (E7/T1): it is gitignored, so
  glob tools miss it — verify by direct listing before claiming absence.
- `modules/core-modules/tree-support-planner/src/lib.rs` is ~5.9k lines: ranged
  reads only; never load in full.
- `crates/slicer-ir/src/slice_ir.rs` is >3k lines: locate symbols with
  `rg -n 'pub struct <Name>'` at read time; never store a line pin.
- Read 240a's `design.md` only for §Migration Table and §`raft_plan` Read-Path
  Footprint; the rest is substrate detail this packet does not need.
