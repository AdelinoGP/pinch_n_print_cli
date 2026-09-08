---
status: implemented
packet: 240b-support-raft-module
task_ids:
  - TASK-414
  - TASK-415
  - TASK-416
  - TASK-417
  - TASK-418
  - TASK-537
---

# 240b-support-raft-module

## Goal

Close G-06 by building the raft consumer on 240a's substrate: a new
`com.core.raft-default` `Layer::Infill` synthesizer holding `claim:raft-fill`
that reads `SupportPlanIR.raft_plan` (through 240a's
  `paint-region-layer-view.raft-plan` accessor), and `SliceIR` and
writes deterministic raft footprint polygons into `SlicedRegion.raft_fill`;
plus the three net-new canonical raft config keys (none of them exists
anywhere under `modules/` or `crates/` today - they are introduced here for the
first time), the raft-key wire-or-record sweep across the existing support
manifests, the formal ADR-0009 Decision-5 amendment, and the Human Validation
Gate.

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
  `claim:raft-fill`; declares `reads = ["SliceIR"]` and
  `writes = ["SliceIR", "InfillIR"]`; its `SupportPlanIR.raft_plan` access rides
  the host-provisioned paint view per the `Layer::Infill` stage contract
  (docs/01 §Module Access Contract), and it writes `SlicedRegion.raft_fill`
  with deterministic fill polygons.
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

### AD-240B-1: absorbed transport gap (scope amendment)

Verified at Step 3 (2026-09-05, independent source inspection):

- The authoring-time assumption "the write leg was already complete" is FALSE:
  `slice-region-view::raft-fill` / `perimeter-region-view::raft-fill` in
  `crates/slicer-schema/wit/deps/ir-types.wit` are GETTERS ONLY, guests
  deliver fill output only via `infill-output-builder` path pushes, and
  `InfillOutputCollected`
  (`crates/slicer-wasm-host/src/marshal/accumulators.rs`) has no polygon
  carrier — no WIT setter or host carrier exists for raft polygons.
- Nothing CONSUMES `SlicedRegion.raft_fill` for emission: remaining hits are
  definition, partition (`split_field!`), restore, visual-debug, tests. G-code
  emission reads `LayerCollectionIR.ordered_entities`, assembled by
  `assemble_ordered_entities_with_support_identities`
  (`crates/slicer-runtime/src/layer_executor.rs`).
- `convert_infill_output` (`crates/slicer-wasm-host/src/marshal/out.rs`)
  groups paths by (object_id, region_id) into `InfillIR.regions`; the runtime
  commits `LayerStageCommit::Infill` into a per-layer arena slot — nothing
  maps guest output to `SlicedRegion.raft_fill`.

**Decision (user-approved scope amendment, 2026-09-05):** packet 240b absorbs
the missing write transport and emitter rather than routing them to 240a or a
follow-up packet. Absorbed items (this packet now owns, and only these):

- **(a)** WIT: additive `infill-output-builder::push-raft-fill:
  func(polygons: list<ex-polygon>) -> result<_, string>` on the existing
  host-provided resource, correlated via `set-current-origin`; no new type,
  no `SliceIR` schema field, no schema-version bump.
- **(b)** Host: `HostInfillOutputBuilder::push_raft_fill`,
  `InfillOutputCollected.raft_fill` carrier with parallel origins, and an
  additive per-region raft carrier in `convert_infill_output`
  (e.g. `InfillIR.raft_regions`).
- **(c)** Runtime commit: the `LayerStageCommit::Infill` commit path writes
  delivered polygons into the layer's `SlicedRegion.raft_fill` for matching
  regions (partition/restore already exist via `split_field!`).
- **(d)** Emitter at `assemble_ordered_entities_with_support_identities`:
  raft_fill ex-polygons → `ExtrusionPath3D` with `ExtrusionRole::RaftInfill`
  as ordinary ordered entities at raft band layers — no anchored events, no
  flow/width table changes.
- **(e)** SDK native-leg mirror of the builder method so wasm and native
  legs deliver identically (AC-3 byte-identical parity).
- **(f)** The raft-default module's `run_infill` (Step 4) writes its
  synthesized polygons through `push-raft-fill` into
  `SlicedRegion.raft_fill` — no path-emission fallback.

## Architecture Constraints

- **Positive raft offset band (plan §12/§15 authority, matching canonical):**
  rafts occupy global layer indices `0 .. N-1` where `N = support_raft_layers`,
  identified by `GlobalLayer.is_raft`, with model layers at `N ..`. No raft
  geometry may be minted as an `AnchoredEntity`, routed through
  `execute_per_layer_with_anchored_events`, or carried by any anchored-event
  structure (plan §15 prohibition). Note ADR-0009 is NOT a layer-index
  authority — it decides where raft pattern algorithms live and its Status is
  `Proposed`; this packet owns its Decision-5 amendment.
- **Single-writer per IR is unchanged:** `com.core.raft-default` writes only
  the `SlicedRegion.raft_fill` sub-field of `SliceIR` and its `InfillIR` output
  carrier; it does not claim `SliceIR` wholesale against perimeter/infill
  writers. Its manifest declares `reads = ["SliceIR"]`,
  `writes = ["SliceIR", "InfillIR"]` with the fill-role claim narrowing
  actual ownership, mirroring how the existing infill modules coexist via fill
  claims today. The raft-plan accessor rides the host-provisioned paint view
  per the `Layer::Infill` stage contract (docs/01 §Module Access Contract), so
  `LayerPlanIR` and `SupportPlanIR` are not declared reads.
  Note that scheduler validation (`validate_unfulfilled_reads` /
  `read_is_declared` in `crates/slicer-scheduler/src/validation.rs`) checks only
  that a declared read has *some* upstream writer — it does NOT check that a
  WIT accessor exists for that stage. A declared read with no accessor
  validates clean and fails at runtime, which is precisely why 240a's AC-7 had
  to exist; do not treat a green scheduler validation as evidence the read path
  works.
- **Determinism:** raft polygon synthesis must be a pure function of
  (raft plan, config keys, region context) — no RNG, no
  iteration-order-dependent maps; identical inputs produce identical output
  across runs and across the wasm and native legs.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Schema/version constants: this packet adds no `SliceIR` schema field and
  bumps no schema version. Per AD-240B-1 it DOES amend the WIT
  `infill-output-builder` resource with one additive method
  (`push-raft-fill`, host-provided) and an additive transient `InfillIR`
  per-region raft carrier; everything else in 240a's change surface stays
  out of bounds.

## Data and Contract Notes

- Manifest contracts: config keys snake_case (E9); `[config.schema]` entries
  carry min/max/display/group like sibling modules; every key the guest reads
  must be declared or the filtered config view resolves an invisible in-code
  default.
- WIT boundary: canonical sources live at `crates/slicer-schema/wit/` (both host
  `bindgen!` and guest `include_str!` read them). This packet edits no WIT — if
  it needs to, that is 240a scope.
- Determinism/scheduler constraints: exactly one `claim:raft-fill` holder
  expected; a double holder surfaces as `SchedulerError::ClaimConflict` with
  both module ids plus the `claim` string and a `scope: ConflictScope`
  discriminator (four fields total), and per-region resolution stays
  deterministic.

## Locked Assumptions and Invariants

- Rafts remain a positive `0..N-1` global-layer offset band marked by
  `GlobalLayer.is_raft`; never anchored entities (plan §15 — the sole
  authority; ADR-0009 says nothing about indices).
- The first printed MODEL layer is index `support_raft_layers`, not `0`.
- Canonical defaults: `raft_contact_distance` 0.1 mm, `raft_expansion` 1.5 mm,
  `raft_first_layer_expansion` 2.0 mm, sourced from
  `docs/ORCA_CONFIG_REFERENCE.md` and canonical `init_fff_params`
  (`PrintConfig.cpp`) — declared as-is in mm, converted ÷100 at the unit
  boundary. All three are net-new keys owned solely by `raft-default.toml`.
- ADR-0009 Decision 4 and the Future-Reviewer Note are preserved verbatim; only
  Decision 5's claim assignment is amended, additively.
- Invariant 16: every acceptance command names `--exact` tests or asserts a
  non-zero matched count in the same run.

## Risks and Tradeoffs

- **Substrate drift:** every FORWARD-DEP in §Substrate Consumed From 240a is a
  name this packet does not control. Mitigated by verifying all of them as a
  Step 1 precondition rather than on first use.
- **Scheduler validation gives false comfort:** a declared read with no WIT
  accessor validates clean. Never treat a green DAG validation as evidence the
  read path works — AC-3 exercising real dispatch is the only proof.
- **wasm/native leg skew (T9):** AC-3 compares outputs across both legs, so a
  one-leg omission fails visibly.
- **ADR boundary:** the polygon-synthesis vs pattern-rendering split is made
  explicit in §ADR-0009 Reconciliation and recorded as an ADR amendment plus a
  deviation row (Step 6) rather than left to silent drift.
- **Downstream conversion WAS missing (and the write transport too)** —
  verified at Step 3 and absorbed as AD-240B-1; the packet's scope amendment
  is recorded in §Absorbed Substrate Gap. Substrate drift risk reduced
  accordingly but the absorbed surface is now owned here: a defect inside it
  is this packet's bug.
