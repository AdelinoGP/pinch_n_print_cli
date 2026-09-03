# Design: 240b-support-raft-module

## Controlling Code Paths

- Primary code path: `SupportPlanIR.raft_plan` (blackboard) →
  `paint-region-layer-view.raft-plan` (240a accessor) → new
  `com.core.raft-default` guest (`Layer::Infill`) → `SlicedRegion.raft_fill`
  (240a carrier) → ordinary ordered-entity G-code at negative global-layer
  indices.
- Neighboring tests/fixtures:
  `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` (case
  `ac4_raft_fill_claim_emits_raft_infill`),
  `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`,
  `modules/core-modules/tree-support-planner/tests/*` (RaftPlan producer),
  fixture `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference
  Obligations; do not repeat delegation rules.

## Substrate Consumed From 240a (FORWARD-DEP; verify before Step 1)

Every symbol below is created by **240a-support-raft-substrate** and is a
FORWARD-DEP at authoring time, reconciled name-for-name against 240a's
`design.md`. Verify each exists before Step 1 rather than discovering a rename
mid-implementation.

| Symbol | Shape 240a promises | Where |
| --- | --- | --- |
| `SlicedRegion.raft_fill` | `Vec<ExPolygon>`, `#[serde(default)]` | `crates/slicer-ir/src/slice_ir.rs` |
| `raft-fill` accessor | `func() -> list<ex-polygon>`, on BOTH region resources | `crates/slicer-schema/wit/deps/ir-types.wit` |
| `raft-plan-view` | record mirroring the prepass `raft-plan` | `crates/slicer-schema/wit/deps/ir-types.wit` |
| `paint-region-layer-view.raft-plan` | `func() -> option<raft-plan-view>` | same |
| `PaintRegionLayerView::raft_plan()` | SDK getter beside `support_plan()` | `crates/slicer-sdk/src/traits.rs` |
| `LayerModule::run_infill` | takes `layer_index: i32` | `crates/slicer-sdk/src/traits.rs` |
| negative prefix band | `GlobalLayer.index` in `-N .. -1`, sorting before 0 | `crates/slicer-wasm-host/src/marshal/{in_,native}.rs` |
| `CURRENT_SLICE_IR_SCHEMA_VERSION` | minor-bumped past 4.8.0 | `crates/slicer-ir/src/slice_ir.rs` |

## Architecture Constraints

- **ADR-0009 preserved:** rafts are signed negative global-layer PREFIX
  entries. No raft geometry may be minted as an `AnchoredEntity`, routed
  through `execute_per_layer_with_anchored_events`, or carried by any
  anchored-event structure (plan §15 prohibition).
- **Single-writer per IR is unchanged:** `com.core.raft-default` writes only
  the `SlicedRegion.raft_fill` sub-field of `SliceIR`; it does not claim
  `SliceIR` wholesale against perimeter/infill writers. Its manifest declares
  `writes = ["SliceIR"]` with the fill-role claim narrowing actual ownership,
  mirroring how the existing infill modules coexist via fill claims today.
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
- Schema/version constants: this packet does **not** bump any schema version.
  It adds no IR field. If it finds itself needing one, that is 240a scope to
  route back.

## Code Change Surface

### Selected approach

1. **New guest module** `modules/core-modules/raft-default/`: manifest TOML +
   guest src + `wit-guest/`. WIT world: **reuse the existing
   `slicer:layer-infill` world** (`world infill-module` in
   `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit`, 20 lines,
   importing `slicer:common/host-services`, `slicer:common/profiling`,
   `slicer:config/config-types`, and `slicer:ir-handles/ir-handles`). The
   module is a `Layer::Infill` synthesizer like `rectilinear-infill`; a new dep
   world would duplicate the identical import set for zero benefit. Both new
   accessors it needs (`raft-fill` and `raft-plan`) ride resources in
   `ir-handles`, which the world already imports — this is what makes reuse
   viable, and it is only true because 240a put them there.
2. **Geometry port:** canonical `generate_raft_base` staged behavior with PnP
   unit discipline — synthesize object-independent raft footprint POLYGONS per
   `RaftPlan` counts (`raft_layers` total = first + `base_raft_layers` +
   `interface_raft_layers`), inflate by `raft_expansion` (÷100 → canonical
   units), apply `raft_first_layer_expansion` on the first printed raft layer
   ("inflate in multiple steps" staging preserved as iterated offsets), and
   derive interface-band footprints at contact-distance spacing. Emit polygons
   only through `run_infill` into `SlicedRegion.raft_fill`.
3. **Keys + decisions:** declare the three canonical keys in the new manifest;
   fill `requirements.md` §Wire-or-Record Decisions for every dead raft key in
   the four support-module manifests; regenerate
   `docs/15_config_keys_reference.md` with `cargo xtask gen-config-docs`.
4. **Negatives:** claim-conflict, band-bounds rejection, and an undeclared-key
   rejection that actually exercises the rejection path rather than grepping
   the manifest for a key's presence.
5. **ADR + records:** formal ADR-0009 amendment plus its deviation row; DEV-124
   re-verification.

### Exact functions, traits, manifests, tests, and fixtures

- New module: `modules/core-modules/raft-default/{Cargo.toml,
  raft-default.toml, src/lib.rs, wit-guest/}`, mirroring
  `modules/core-modules/rectilinear-infill/` as the shape template.
- Tests authored: integration cases
  `raft_fill_is_deterministic_across_two_runs`,
  `raft_first_layer_expansion_exceeds_upper_layers`,
  `raft_geometry_orders_before_model_layers`,
  `raft_mints_no_anchored_entities`; contract cases
  `raft_keys_declared_and_wired`, `raft_index_outside_band_rejected`,
  `undeclared_raft_key_is_rejected_not_defaulted`; scheduler case
  `raft_fill_double_holder_conflicts` in a new
  `crates/slicer-scheduler/tests/raft_claim_conflict_tdd.rs`.
- Reused as-is: `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs`
  case `ac4_raft_fill_claim_emits_raft_infill` (verified to exist under that
  exact name; the file's other cases are `ac_n1_sparse_fill_claim_does_not_emit_raft_infill`
  and `ac_n3_empty_held_claims_suppress_raft_infill`).

### Rejected alternatives and reasons

- **New dep world for raft-default:** rejected — `slicer:layer-infill` already
  imports exactly what the synthesizer needs once 240a's accessors land;
  duplicating it adds a versioned surface to maintain with no capability
  difference.
- **Anchored entities for raft layers:** rejected — plan §15 prohibition;
  ADR-0009 contract.
- **Making `rectilinear-infill` hold `claim:raft-fill` (ADR-0009's v1 shape):**
  superseded by plan §12, which assigns the claim to `com.core.raft-default`
  itself. Executed as a formal amendment in Step 6, not a silent contradiction.
  The claim string and the `should_emit` mapping stay, so a future pattern
  module can take the claim over without IR changes.
- **Grepping the manifest as AC-N3's rejection test:** rejected — that asserts
  a key is present, which is the positive case. The negative must exercise the
  path where a consumed key is undeclared and confirm it is rejected rather
  than silently defaulted (E9).

### ADR-0009 Reconciliation (normative for Steps 1-3; ONE position)

**The single position, stated once:** `com.core.raft-default` holds
`claim:raft-fill` (plan §12's 240 brief is the governing authority; ADR-0009
Decision 5's assignment of the claim to the pattern module
(`rectilinear-infill`) is superseded by that plan and formally amended in
Step 6) AND writes `SlicedRegion.raft_fill` with deterministic fill polygons.
The module performs NO extrusion-path, flow, speed, or role-tagged rendering —
conversion of `raft_fill` polygons into printable paths happens downstream
under the claim holder's existing emit machinery.

ADR-0009's Decision point 4 ("`raft-default` is a synthesizer module — it reads
`SupportPlanIR.raft_plan` … and populates the raft polygon carriers. It
contains zero pattern algorithms.") and its Future-Reviewer Note ("Do not
re-suggest making `raft-default` a renderer") are preserved UNCHANGED and read
as prohibiting **pattern-algorithm ownership and extrusion-path rendering** in
this module. They do NOT prohibit `com.core.raft-default` from computing region
boundaries and fill-area polygons deterministically — Decision 4 explicitly
says the synthesizer "populates the raft polygon carriers", and plan §12
requires it to write `SlicedRegion.raft_fill`.

Boundary adopted (and recorded in the Step 6 formal amendment):

- `com.core.raft-default` synthesizes **polygons only**: object-independent
  raft footprints per `RaftPlan` counts, `raft_expansion` /
  `raft_first_layer_expansion` inflation staging, interface-band footprints at
  contact-distance spacing — deterministic pure geometry into
  `SlicedRegion.raft_fill`. No scan-line pattern math, no `ExtrusionPath3D`, no
  flow/speed/role decisions.
- Conversion of those polygons into extrusion paths happens **downstream**
  through the existing `Layer::Infill` emit machinery the claim-holder path
  already drives — no new rendering code here, no pattern math duplicated
  anywhere.
- The claim remains reassignable: a future pattern module can take
  `claim:raft-fill` over by manifest change alone.

**Amendment mechanics (Step 6).** The amendment is additive per ADR
immutability convention: the ADR's inline Decision-5 text stays verbatim, and a
new `## Amendment — <date> (packet 240b)` section quotes the contested clause
and records the reassignment. The Status line also flips from
`Proposed (lands with docs/specs/raft-default-module.md)` to `Accepted`,
dropping the parenthetical — `docs/specs/raft-default-module.md` does not exist
and never landed, so leaving the reference would be a dangling citation. And
because the packet supersedes an ADR's normative clause, it MUST also file a
deviation row: the live convention is `D-<pkt>-ADR-<NNNN>-AMENDED`
(`D-285-ADR-0051-AMENDED`, `D-286-ADR-0005-AMENDED` are the shipped
precedents). Re-derive the free ID space at write time; do not trust an ID
written in this packet.

## Transport Reuse Diagram

```
tree-support-planner guest
  └─ push_raft_plan(RaftPlan)                                    [exists]
       ├─ crates/slicer-sdk/src/prepass_builders.rs::SupportGeometryOutput::push_raft_plan  [exists]
       ├─ crates/slicer-macros/src/lib.rs glue (output.push_raft_plan)                      [exists]
       ├─ crates/slicer-wasm-host/src/host.rs (resource fn push_raft_plan)                  [exists]
       ├─ crates/slicer-wasm-host/src/marshal/native.rs (native leg RaftPlan)               [exists]
       └─ crates/slicer-runtime/src/blackboard.rs::raft_plan_min merge                      [exists]
              ↓ blackboard slot: SupportPlanIR.raft_plan
       crates/slicer-wasm-host/src/dispatch.rs::build_paint_layer_data_with_plan
       → paint-region-layer-view.raft-plan                       [240a - FORWARD-DEP]
              ↓
com.core.raft-default (NEW, Layer::Infill, holds claim:raft-fill)
  reads: raft_plan via the paint view + SliceIR per-region context
  writes: SlicedRegion.raft_fill                                 [240a carrier - FORWARD-DEP]
       ↓ host partition/delivery (existing region flow;
         raft_fill survives modifier splits via 240a's split_field!)
G-code emission at negative-index layers (ordinary ordering; NO anchored events)
```

The write leg was already complete before this family started; 240a supplied
the read leg and the carrier; this packet supplies the only missing piece, the
consumer.

## Files in Scope (read + edit)

- `modules/core-modules/raft-default/**` - role: the new guest module; expected change: full directory (Cargo.toml, manifest, src, wit-guest).
- `modules/core-modules/{tree-support-planner,traditional-support-planner,tree-support,traditional-support}/*.toml` - role: wire-or-record annotations; expected change: comment or `[config.schema]` rows only, no logic.
- `crates/slicer-runtime/tests/integration/raft_geometry.rs` + `main.rs` registration - role: AC-3/AC-4/AC-5 cases.
- `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs` + `main.rs` registration - role: AC-6/AC-N2/AC-N3 cases.
- `crates/slicer-scheduler/tests/raft_claim_conflict_tdd.rs` - role: AC-N1.
- `docs/adr/0009-raft-as-layer-infill-role.md`, `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md`, `docs/03_wit_and_manifest.md` - role: records.
- `docs/spec_packets/240b-support-raft-module/requirements.md` - role: the wire-or-record table and the DEV-124 outcome.

## Read-Only Context

- `modules/core-modules/rectilinear-infill/{Cargo.toml, rectilinear-infill.toml}` - full read (small) - purpose: the shape template for the new module.
- `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` - full read (20 lines) - purpose: confirm the world's import set covers the new accessors.
- `crates/slicer-sdk/src/views.rs` - the `should_emit` range, located at read time with `rg -n 'fn should_emit'` - purpose: the claim-string mapping.
- `crates/slicer-scheduler/src/validation.rs` - the `ClaimConflict` variant definition only (`rg -n 'ClaimConflict'`) - purpose: the exact error shape AC-N1 asserts (`claim: String, module_a: ModuleId, module_b: ModuleId`).
- `modules/core-modules/tree-support-planner/src/lib.rs` - the range around `push_raft_plan` only - purpose: producer contract; never load the ~5.9k-line file.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load (T1: gitignored, glob-blind).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- Everything in 240a's change surface — `crates/slicer-ir/src/slice_ir.rs`,
  `crates/slicer-schema/wit/deps/{ir-types.wit, prepass-layer-planning/}`,
  `crates/slicer-wasm-host/src/marshal/**`, `crates/slicer-runtime/src/**`.
  Read them if a FORWARD-DEP needs verifying; never edit them here. A needed
  change is a 240a defect to route back.
- `modules/core-modules/rectilinear-infill/src/**` and other pattern modules -
  untouched this packet.
- `crates/slicer-scheduler/src/validation.rs` - the validator shape is
  236-owned; this packet only tests its observable contract.
- `modules/core-modules/tree-support-planner/src/lib.rs` beyond the cited
  range - planner algorithms are 238b's surface.

## Expected Sub-Agent Dispatches

- FACT: confirm each row of §Substrate Consumed From 240a exists with the
  promised shape; scope `crates/`; return FACT per row; purpose: Step 1
  precondition.
- OrcaSlicer SUMMARY: `generate_raft_base` staging order (expand-first vs
  contact-first, multi-step inflation counts, base-vs-interface loop
  structure); return SUMMARY; purpose: Step 3.
- OrcaSlicer FACT: `PrintConfig.cpp::init_fff_params` defaults for the three
  raft keys; return FACT; purpose: Step 4.
- LOCATIONS: every raft key declared in the four support-module manifests;
  scope `modules/core-modules/`; return LOCATIONS; purpose: Step 5's
  wire-or-record table (the four scaffold rows are a minimum, not the total).

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
  both module ids, and per-region resolution stays deterministic.

## Locked Assumptions and Invariants

- Rafts remain signed negative global-layer prefix entries; never anchored
  entities (plan §15; ADR-0009).
- Canonical defaults: `raft_contact_distance` 0.1 mm, `raft_expansion` 1.5 mm,
  `raft_first_layer_expansion` 2.0 mm — declared as-is in mm, converted ÷100 at
  the unit boundary.
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
- **Downstream conversion may be missing:** the design assumes the claim-holder
  emit path already converts `raft_fill` polygons to paths. If Step 3 finds it
  does not, that is a real gap — record it as a follow-up packet, do not absorb
  a renderer into this module (ADR-0009).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 geometry port; Step 5 keys + doc regen)
- Highest-risk dispatch and required return format: OrcaSlicer SUMMARY of
  `generate_raft_base` staging (must describe order and loop structure, not
  paste code)

## Open Questions

- [FWD] Exact interface-band spacing derivation from `raft_contact_distance`
  (canonical uses it between raft top and object bottom): worker confirms the
  precise consumption site via the delegated `generate_raft_base` SUMMARY in
  Step 3 and records the mapping in code comments; no activation blocker.
- [FWD] Does the claim-holder emit path already convert `raft_fill` polygons to
  extrusion paths, or is a holder-side wiring change needed? Worker resolves in
  Step 3; if a change is needed it is recorded as a follow-up, not absorbed.
  No activation blocker.
- None [BLOCK].
