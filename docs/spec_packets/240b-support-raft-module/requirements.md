# Requirements: 240b-support-raft-module

## Packet Metadata

- Grouped task IDs: `TASK-414`..`TASK-418`, `TASK-537`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` (§11 queue row 7, §12 brief "240-support-raft"); gap register row G-06
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

G-06 states the raft situation exactly: "the IR exists, the consumer does not."
`RaftPlan` is produced by the tree planner's `push_raft_plan` when
`support_raft_layers > 0` and merged into the blackboard by `raft_plan_min`
(`crates/slicer-runtime/src/blackboard.rs`), and nothing renders it. Every
raft-related config key any existing manifest declares is unread (re-derive
the set with a grep over `modules/core-modules/*/*.toml`, per
§Wire-or-Record Decisions), and the three canonical Orca raft keys
(`raft_contact_distance`, `raft_expansion`, `raft_first_layer_expansion`) do
not exist anywhere under `modules/` or `crates/` at all.

The role/claim plumbing half-exists: `ExtrusionRole::RaftInfill` is a real
variant (`crates/slicer-ir/src/slice_ir.rs`) and `SliceRegionView::should_emit`
(`crates/slicer-sdk/src/views.rs`) already maps it to `"claim:raft-fill"` — but
no manifest anywhere declares that claim, so raft emission is suppressed
everywhere.

**240a-support-raft-substrate** removes the four structural blockers that made
writing the consumer impossible (no way to mark a layer as raft, no raft flag
on the IR, object-bottom predicates hardcoding layer zero, and no read-side
raft transport) and adds the two carriers the consumer needs
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
- Rafts occupy a positive global-layer offset band (`0 .. N-1`, model layers at
  `N ..`), never anchored entities (plan §15). — **substrate in 240a, honored here.**
- `GlobalLayer.is_raft` marker + WIT `layer-proposal.is-raft-prefix`. — **240a.**
- Issue-19/20 raft keys `raft_contact_distance`, `raft_expansion`,
  `raft_first_layer_expansion` — all three net-new, introduced only in the new
  raft-default manifest — plus a wire-or-record sweep over whatever
  raft-related keys the existing core-module manifests actually declare. —
  **this packet.**
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
  `raft_first_layer_expansion` — net-new; none of the three exists anywhere
  under `modules/` or `crates/` today — declared in
  `modules/core-modules/raft-default/raft-default.toml`'s `[config.schema]`
  with canonical defaults, each read by the geometry it controls. The canonical
  name and default source is `docs/ORCA_CONFIG_REFERENCE.md` together with
  canonical `init_fff_params` in `PrintConfig.cpp`, never a pre-existing
  manifest.
- Wire-or-record decisions for every raft-related key the existing core-module
  manifests actually declare (re-derived by grep at execution time), written
  into §Wire-or-Record Decisions below.
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs` (T8).
- Claim-conflict behavior for a second `claim:raft-fill` holder (AC-N1).
- Module writes nothing on a non-raft layer (AC-N2) and the undeclared-key rejection test
  (AC-N3).
- Formal ADR-0009 amendment plus the `D-<pkt>-ADR-0009-AMENDED` deviation row.
- DEV-124 re-verification under a live raft (AC-8).
- Human Validation Gate artifacts (`packet.spec.md` §Human Validation Gate).

## Out of Scope

- Everything 240a owns: the `GlobalLayer.is_raft` marker and its WIT marking,
  the raft band emission, the object-bottom predicate audit, the `SlicedRegion.raft_fill`
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
- `docs/adr/0009-raft-as-layer-infill-role.md` - short; full read allowed.
- `docs/specs/support-parity-gap-register.md` - G-06 row only; direct range read.
- `docs/spec_packets/240a-support-raft-substrate/design.md` - §`raft_plan` Read-Path Footprint and §Architecture Constraints only. (There is no §Migration Table; the u32->i32 migration was withdrawn in the re-spec.)
- `docs/15_config_keys_reference.md` - regenerated, not read in bulk.
- `docs/ORCA_CONFIG_REFERENCE.md` - canonical name/default source for the three net-new raft keys; targeted grep only, never a bulk read.
- `docs/19_visual_debug.md` / `docs/17_agent_debugging.md` - human-gate bundle only; delegated SUMMARY.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_raft_base`: object-independent raft construction; first-layer expansion (`inflate_factor_1st_layer` = `raft_first_layer_expansion`), contact/inflate staging, base/interface loops, multi-step inflation, classic vs organic branches.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `init_fff_params`: defaults `raft_contact_distance = 0.1`, `raft_expansion = 1.5`, `raft_first_layer_expansion = 2.0`.

## Wire-or-Record Decisions

AC-7 is satisfied by this table, not by a test. **The row set is not fixed by
this packet.** Step 5 must first re-derive the real set of raft-related keys
declared by the existing core-module manifests:

```
rg --no-filename -o '^\[config\.schema\.[a-z_]*raft[a-z_]*\]' modules/core-modules -g '*.toml' -g '!raft-default.toml'
```

(drop `--no-filename` to get the manifest per hit) and write exactly one row per
declaration site the grep returns. At authoring time that grep returns
`support_raft_layers` in
`modules/core-modules/arachne-perimeters/arachne-perimeters.toml`,
`modules/core-modules/classic-perimeters/classic-perimeters.toml`, and
`modules/core-modules/tree-support-planner/tree-support-planner.toml`, plus
`raft_first_layer_density`, `base_raft_layers`, and `interface_raft_layers` in
`tree-support-planner`. **That is an observation, not a contract** — re-run the
grep and follow whatever it returns. Note in particular that
`raft_contact_distance`, `raft_expansion`, and `raft_first_layer_expansion` are
NOT among them: those three are net-new in this packet and belong to
`raft-default.toml` only, so they get no row here.

Each row must have the literal shape
``| `<key>` | `<manifest>` | <verdict> | <reason> |`` with the verdict starting
in lowercase (`wired` / `stays dead`) — AC-7 counts rows of that shape and
compares the count against the grep above, and additionally fails while a TABLE
ROW still carries the placeholder sentinel. A row whose verdict
is `stays dead` must name the owner of that decision.

AC-7's greps are SECTION-SCOPED (`sed` from this heading to
`## DEV-124 Re-verification`) on purpose: `requirements.md` quotes AC-7's own
command verbatim in §Verification Commands, so an unscoped
`rg` for the sentinel over the whole file would match the command text itself
and be unsatisfiable forever. The grep is ALSO row-anchored (`^\|`) so it cannot
match this explanatory prose, which sits inside the scoped range. Both the
scoping and the row anchor are load-bearing — do not "simplify" either away.

| Key | Manifest | Verdict | Reason / decision owner |
| --- | --- | --- | --- |
| _rows added in Step 5, one per grep hit_ | | PENDING-STEP5-ROW | |

## DEV-124 Re-verification

DEV-124 (`docs/DEVIATION_LOG.md`, Status: "Closed — 2026-08-07: fixed the same
day") makes both perimeter generators gate `only_one_wall_first_layer` on
`layer_index == support_raft_layers`, pinned by
`classic_clamp_follows_raft_layers_not_layer_zero` and
`classic_clamp_unchanged_when_no_raft_configured` in
`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`.

240a's `requirements.md` §DEV-124 Upheld establishes that this predicate is
**correct as shipped** under the positive offset band this family adopted: the
first printed model layer is index `support_raft_layers`, exactly what the
clamp tests. 240a files no reopen row, and its AC-N4 asserts the pinning file
is unmodified. This packet is where the raft path actually goes live, so this
is where the predicate is re-verified against real behaviour rather than a
synthesized config view.

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
  (raft band ordering before the first model layer, zero anchored entities), `AC-6` (keys declared and
  wired), `AC-7` (raft-key wire-or-record table), `AC-8` (DEV-124
  re-verification).
- Negative: `AC-N1` (double-holder `SchedulerError::ClaimConflict`, the
  four-field variant `claim` / `module_a` / `module_b` / `scope`), `AC-N2`
  (module writes nothing on a non-raft layer), `AC-N3` (undeclared key rejected, not
  silently defaulted).
- Cross-packet impact: hard-depends on 240a; feeds 242's closure evidence;
  leaves 239/241 untouched.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure-gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -q 'id\s*=\s*"com\.core\.raft-default"' modules/core-modules/raft-default/raft-default.toml && rg -q 'claim:raft-fill' modules/core-modules/raft-default/raft-default.toml && rg -q 'Layer::Infill' modules/core-modules/raft-default/raft-default.toml && cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` | AC-1 manifest + freshness | FACT exit code |
| `mkdir -p target && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- ac4_raft_fill_claim_emits_raft_infill --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 claim dispatch | FACT pass/fail |
| `test "$(rg -l 'claim:raft-fill' modules/core-modules/*/[a-z-]*.toml \| wc -l)" -eq 1` | AC-2 exactly one declared holder | FACT exit code |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_fill_is_deterministic_across_two_runs --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 determinism | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_first_layer_expansion_exceeds_upper_layers --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 expansions | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_geometry_orders_before_model_layers --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_mints_no_anchored_entities --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 ordering / no anchored (each command tees and is guarded separately) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_bounds_tdd::raft_keys_declared_and_wired --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-6 keys wired | FACT pass/fail |
| `DECL="$(rg --no-filename -o '^\[config\.schema\.[a-z_]*raft[a-z_]*\]' modules/core-modules -g '*.toml' -g '!raft-default.toml' \| wc -l)"; ROWS="$(sed -n '/^## Wire-or-Record Decisions$/,/^## DEV-124 Re-verification$/p' docs/spec_packets/240b-support-raft-module/requirements.md \| rg -c '^\| `[a-z_]*raft[a-z_]*` \| `[^`]+` \| (wired\|stays dead)')"; test "$DECL" -ge 1 && test "${ROWS:-0}" -eq "$DECL" && ! sed -n '/^## Wire-or-Record Decisions$/,/^## DEV-124 Re-verification$/p' docs/spec_packets/240b-support-raft-module/requirements.md \| rg -q '^\|.*PENDING-STEP5-ROW'` | AC-7 wire-or-record table filled: one row per raft-key declaration site the grep returns | FACT exit code |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- only_one_wall_first_layer_tdd::classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test contract -- only_one_wall_first_layer_tdd::classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-8 DEV-124 re-verification (each command tees and is guarded separately) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-scheduler --test raft_claim_conflict_tdd -- raft_fill_double_holder_conflicts --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1 conflict advisory | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_writes_nothing_on_non_raft_layer --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2 module writes nothing on a non-raft layer (owned by Step 3) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_bounds_tdd::undeclared_raft_key_is_rejected_not_defaulted --exact --nocapture 2>&1 \| tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3 undeclared key | FACT pass/fail |
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
- `modules/core-modules/tree-support-planner/src/lib.rs` is very long: ranged
  reads only; never load in full.
- `crates/slicer-ir/src/slice_ir.rs` is >3k lines: locate symbols with
  `rg -n 'pub struct <Name>'` at read time; never store a line pin.
- Read 240a's `design.md` only for §Architecture Constraints and §`raft_plan` Read-Path
  Footprint; the rest is substrate detail this packet does not need.
