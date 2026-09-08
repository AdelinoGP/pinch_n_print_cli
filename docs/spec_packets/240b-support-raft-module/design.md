# Design: 240b-support-raft-module

## Controlling Code Paths

- Primary code path: `SupportPlanIR.raft_plan` (blackboard) →
  `paint-region-layer-view.raft-plan` (240a accessor) → new
  `com.core.raft-default` guest (`Layer::Infill`) → `SlicedRegion.raft_fill`
  (240a carrier) → ordinary ordered-entity G-code at the raft band layers
  (global indices `0 .. support_raft_layers - 1`).
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
| `paint-region-layer-view.is-raft` | `func() -> bool` | same |
| `PaintRegionLayerView::is_raft()` | SDK getter beside `raft_plan()` | `crates/slicer-sdk/src/traits.rs` |
| `PaintRegionLayerView::raft_plan()` | SDK getter beside `support_plan()` | `crates/slicer-sdk/src/traits.rs` |
| `LayerModule::run_infill` | takes `layer_index: u32` (unchanged by 240a) | `crates/slicer-sdk/src/traits.rs` |
| positive raft offset band | `GlobalLayer.index` in `0 .. N-1` with `is_raft == true`; model layers at `N ..` | `crates/slicer-wasm-host/src/marshal/{in_,native}.rs` |
| `GlobalLayer.is_raft` | `bool`, `#[serde(default)]` raft marker (host-side; a WASM guest CANNOT read this directly — it reads `paint-region-layer-view.is-raft` above) | `crates/slicer-ir/src/slice_ir.rs` |
| `CURRENT_SLICE_IR_SCHEMA_VERSION` | minor-bumped past 4.8.0 | `crates/slicer-ir/src/slice_ir.rs` |

## Absorbed Substrate Gap: Guest→raft_fill Transport + Emitter (AD-240B-1)

- **Verified at Step 3 (2026-09-05, independent source inspection):** the
  authoring-time assumption "the write leg was already complete" is FALSE.
  `slice-region-view::raft-fill` and `perimeter-region-view::raft-fill` in
  `crates/slicer-schema/wit/deps/ir-types.wit` are GETTERS ONLY — no WIT
  setter exists for raft polygons. Guests deliver fill output only via the
  `infill-output-builder` resource (`push-sparse-path` / `push-solid-path` /
  `push-ironing-path` / `set-current-origin`), and the host carrier
  `InfillOutputCollected`
  (`crates/slicer-wasm-host/src/marshal/accumulators.rs`) holds
  sparse/solid/ironing path Vecs with parallel `Vec<Option<OriginId>>` origin
  vecs — no polygon carrier.
- **Verified in the same inspection:** nothing CONSUMES
  `SlicedRegion.raft_fill` for emission — remaining hits are definition,
  partition (`split_field!`), restore, visual-debug, and tests. G-code
  emission reads `LayerCollectionIR.ordered_entities`, assembled by
  `assemble_ordered_entities_with_support_identities`
  (`crates/slicer-runtime/src/layer_executor.rs`) from
  PerimeterIR/InfillIR/SupportIR after dispatch.
- **Decision (user-approved scope amendment, 2026-09-05):** packet 240b
  absorbs the missing write transport and the missing emitter as
  **AD-240B-1** rather than routing them back to 240a or deferring them to a
  follow-up packet.

Also absorbed (Step 4): the dispatch-time claim resolution for `claim:raft-fill` (`resolve_held_claims` in crates/slicer-scheduler/src/validation.rs) and raft-only WASM commit preservation (crates/slicer-wasm-host/src/dispatch.rs) — both were missing and are now owned here.

Absorbed work (this packet now owns, and only these):

1. **(a) WIT:** one additive method on the existing `infill-output-builder`
   resource in `crates/slicer-schema/wit/deps/ir-types.wit`:
   `push-raft-fill: func(polygons: list<ex-polygon>) -> result<_, string>`,
   correlated to regions via the existing `set-current-origin` mechanism
   (host-provided resource — additive, no guest forced to call it; no new
   type introduced; no `SliceIR` schema field added; no
   `CURRENT_SLICE_IR_SCHEMA_VERSION` bump).
2. **(b) Host:** `HostInfillOutputBuilder::push_raft_fill`
   (`crates/slicer-wasm-host/src/host.rs`, mirroring the path pushes),
   `InfillOutputCollected.raft_fill` polygon carrier with parallel origins
   (`crates/slicer-wasm-host/src/marshal/accumulators.rs`), and
   `convert_infill_output` (`crates/slicer-wasm-host/src/marshal/out.rs`)
   gains an additive per-region raft polygon carrier (e.g.
   `InfillIR.raft_regions`) — the exact struct shape is the implementer's
   choice, keyed like `InfillRegion`/`OriginBucket`.
3. **(c) Runtime commit:** the `LayerStageCommit::Infill` commit path writes
   the delivered polygons into the layer's `SlicedRegion.raft_fill` for
   matching regions (region partition/restore for `raft_fill` already exists
   via `split_field!`).
4. **(d) Emitter (amended after the human validation gate, 2026-09-06):**
   the original contour-only conversion was a defect: it emitted raft
   outlines but no raft fill. Canonical `generate_raft_base` areas go through
   OrcaSlicer's infill pass; this repository's equivalent is a generic
   host-side `hatch_areas` helper (parallel scanlines clipped to ExPolygons,
   including holes), exposed through the same WIT/native host-service surface
   as `offset_polygons`. `run_infill` keeps the expanded area contours as
   closed `RaftInfill` rings and also emits the hatch result as open, two-point
   contours through `push-raft-fill`. The `raft_fill` conversion in
   `assemble_ordered_entities_with_support_identities` must preserve two-point
   contours as open `ExtrusionPath3D` paths; other contours are closed only
   when needed. Both are ordinary ordered entities (RegionKey from region
   object_id / region_id + layer index), NOT anchored events. The module owns
   the pattern: `raft_line_spacing` is a snake_case float-mm config key,
   default 0.5, and the hatch angle is a documented constant of 45 degrees.
   `RaftInfill` already has G-code feedrate handling and the `;TYPE:Support`
   label — no flow/width table changes.
5. **(e) SDK native-leg mirror** of the builder method (slicer-sdk /
   slicer-macros adaptation surface) so the wasm and native legs deliver
   identically — AC-3's byte-identical parity requirement.
6. **(f) Guest:** the raft-default module's `run_infill` (Step 4) writes its
   synthesized polygons through `push-raft-fill` into
   `SlicedRegion.raft_fill` instead of the path-emission fallback used by
   the initial partial attempt.

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
3. **Keys + decisions:** declare the three canonical keys — all net-new; none
   of `raft_contact_distance`, `raft_expansion`, or
   `raft_first_layer_expansion` exists anywhere under `modules/` or `crates/`
   today, and their names/defaults come from `docs/ORCA_CONFIG_REFERENCE.md`
   plus canonical `init_fff_params` in `PrintConfig.cpp` — in the new manifest;
   fill `requirements.md` §Wire-or-Record Decisions with one row per
   raft-related key the existing core-module manifests actually declare,
   re-derived by grep at execution time rather than assumed; regenerate
   `docs/15_config_keys_reference.md` with `cargo xtask gen-config-docs`.
4. **Negatives:** claim-conflict (AC-N1, Step 2), a non-raft-layer no-write
   case (AC-N2, Step 3 — module-side, NOT a host band-bounds validator), and an
   undeclared-key rejection (AC-N3, Step 4) that actually exercises the
   rejection path rather than grepping the manifest for a key's presence.
5. **ADR + records:** formal ADR-0009 amendment plus its deviation row; DEV-124
   re-verification.

### Exact functions, traits, manifests, tests, and fixtures

- New module: `modules/core-modules/raft-default/{Cargo.toml,
  raft-default.toml, src/lib.rs, wit-guest/}`, mirroring
  `modules/core-modules/rectilinear-infill/` as the shape template.
- Tests authored: integration cases
  `raft_writes_nothing_on_non_raft_layer` (AC-N2),
  `raft_fill_is_deterministic_across_two_runs`,
  `raft_first_layer_expansion_exceeds_upper_layers`,
  `raft_geometry_orders_before_model_layers`,
  `raft_mints_no_anchored_entities`; contract cases
  `raft_keys_declared_and_wired`,
  `undeclared_raft_key_is_rejected_not_defaulted`; scheduler case
  `raft_fill_double_holder_conflicts` in a new
  `crates/slicer-scheduler/tests/raft_claim_conflict_tdd.rs`.
- Reused as-is: `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs`
  case `ac4_raft_fill_claim_emits_raft_infill` (verified to exist under that
  exact name; the file's other cases are `ac_n1_sparse_fill_claim_does_not_emit_raft_infill`
  and `ac_n3_empty_held_claims_suppress_raft_infill`).
- Absorbed transport (AD-240B-1):
  - `crates/slicer-schema/wit/deps/ir-types.wit` — one additive resource
    method `push-raft-fill: func(polygons: list<ex-polygon>) -> result<_, string>`
    on `infill-output-builder` (item (a)).
  - `crates/slicer-wasm-host/src/host.rs` —
    `HostInfillOutputBuilder::push_raft_fill` mirroring the path pushes
    (item (b)).
  - `crates/slicer-wasm-host/src/marshal/accumulators.rs` —
    `InfillOutputCollected.raft_fill` polygon carrier with parallel origins
    (item (b)).
  - `crates/slicer-wasm-host/src/marshal/out.rs` — `convert_infill_output`
    gains an additive per-region raft polygon carrier, e.g.
    `InfillIR.raft_regions`, keyed like `InfillRegion`/`OriginBucket`
    (item (b)).
  - `crates/slicer-ir/src/slice_ir.rs` — additive `InfillIR` raft carrier
    only (never `SlicedRegion`) (item (b)).
  - `crates/slicer-wasm-host/src/dispatch.rs` — only if the commit path
    needs it.
  - `crates/slicer-runtime/src/layer_executor.rs` — emitter:
    `assemble_ordered_entities_with_support_identities` converts raft_fill
    ex-polygons to `ExtrusionRole::RaftInfill` ordered entities at raft
    band layers (item (d)).
  - `crates/slicer-sdk` (+ `slicer-macros` adaptation surface) — native-leg
    mirror of the builder method so wasm and native legs deliver
    identically (item (e)).

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
dropping the parenthetical: `docs/specs/raft-default-module.md` does not exist
at that path — an archived predecessor does exist at
`docs/specs/_OLD/raft-default-module.md`, so the ADR's pointer is dangling
because the doc was archived, not because it was never written. The pointer
appears THREE times in the ADR (the `## Status` line, the Decision-3 carrier
parenthetical, and the References list); all three must be replaced, not just
the Status line. The archived doc is historical context only and is NOT the
contract: it names `raft_expansion_mm`, `raft_z_gap_mm`, `raft_layer_height_mm`,
and `raft_pattern`, whereas 240b deliberately adopts the canonical Orca names
(`raft_contact_distance`, `raft_expansion`, `raft_first_layer_expansion`) per
`docs/ORCA_CONFIG_REFERENCE.md`. And
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
G-code emission at raft band layers 0..N-1 (ordinary ordering; NO anchored events)
```

The write leg was assumed complete at authoring; verification at Step 3
found it missing (no WIT setter, no host carrier, no emitter — AD-240B-1).
This packet now supplies the consumer AND the absorbed transport+emitter:
guest `push-raft-fill` → `InfillOutputCollected.raft_fill` → `InfillIR` raft
carrier → `SlicedRegion.raft_fill` →
`assemble_ordered_entities_with_support_identities` (RaftInfill-role ordered
entities).

## Files in Scope (read + edit)

- `modules/core-modules/raft-default/**` - role: the new guest module; expected change: full directory (Cargo.toml, manifest, src, wit-guest).
- `modules/core-modules/*/*.toml` for whichever manifests the Step 5 re-derivation grep shows declare a raft-related key (at authoring: `arachne-perimeters`, `classic-perimeters`, `tree-support-planner`; re-derive, do not assume) - role: wire-or-record annotations; expected change: comment or `[config.schema]` rows only, no logic.
- `crates/slicer-runtime/tests/integration/raft_geometry.rs` + `main.rs` registration - role: AC-3/AC-4/AC-5 cases.
- Absorbed transport files (AD-240B-1):
  `crates/slicer-schema/wit/deps/ir-types.wit` (one additive resource
  method), `crates/slicer-wasm-host/src/host.rs` +
  `crates/slicer-wasm-host/src/marshal/accumulators.rs` +
  `crates/slicer-wasm-host/src/marshal/out.rs` (+
  `crates/slicer-wasm-host/src/dispatch.rs` only if the commit path needs
  it), `crates/slicer-ir/src/slice_ir.rs` (additive `InfillIR` raft carrier
  only — never `SlicedRegion`), the `crates/slicer-sdk` native builder
  mirror surface (+ `slicer-macros` adaptation), and
  `crates/slicer-runtime/src/layer_executor.rs` (emitter) - role: the
  guest→`SlicedRegion.raft_fill` write transport and the `raft_fill`
  ordered-entity emitter per §Absorbed Substrate Gap.
- `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs` + `main.rs` registration - role: AC-6/AC-N3 cases. (AC-N2 is module-side and lives in `crates/slicer-runtime/tests/integration/raft_geometry.rs`, Step 3.)
- `crates/slicer-scheduler/tests/raft_claim_conflict_tdd.rs` - role: AC-N1.
- `docs/adr/0009-raft-as-layer-infill-role.md`, `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md`, `docs/03_wit_and_manifest.md` - role: records.
- `docs/spec_packets/240b-support-raft-module/requirements.md` - role: the wire-or-record table and the DEV-124 outcome.

## Read-Only Context

- `modules/core-modules/rectilinear-infill/{Cargo.toml, rectilinear-infill.toml}` - full read (small) - purpose: the shape template for the new module.
- `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` - full read (20 lines) - purpose: confirm the world's import set covers the new accessors.
- `crates/slicer-sdk/src/views.rs` - the `should_emit` range, located at read time with `rg -n 'fn should_emit'` - purpose: the claim-string mapping.
- `crates/slicer-scheduler/src/validation.rs` - the `ClaimConflict` variant definition only (`rg -n 'ClaimConflict'`) - purpose: the exact error shape AC-N1 asserts. It has FOUR fields: `claim: String`, `module_a: ModuleId`, `module_b: ModuleId`, and `scope: ConflictScope` ("scope in which the conflict was observed"). Any match or construction must bind or `..`-elide all four.
- `modules/core-modules/tree-support-planner/src/lib.rs` - the range around `push_raft_plan` only - purpose: producer contract; the file is very long - ranged reads only, never load it in full.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load (T1: gitignored, glob-blind).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- Everything in 240a's change surface — `crates/slicer-ir/src/slice_ir.rs`,
  `crates/slicer-schema/wit/deps/{ir-types.wit, prepass-layer-planning/}`,
  `crates/slicer-wasm-host/src/marshal/**`, `crates/slicer-runtime/src/**` —
  except the AD-240B-1 absorbed items listed in Files in Scope. Read them if
  a FORWARD-DEP needs verifying; never edit them here beyond the absorbed
  scope. A needed change OUTSIDE the absorbed list is a 240a defect to route
  back.
- `modules/core-modules/rectilinear-infill/src/**` and other pattern modules -
  untouched this packet.
- `crates/slicer-scheduler/src/validation.rs` - the validator shape is
  236-owned; the `resolve_held_claims` claim-resolution table is absorbed by
  AD-240B-1, and everything else in this file remains 236-owned. This packet
  only tests the observable contract outside that carve-out.
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
- LOCATIONS: every raft-related key declared in any core-module manifest;
  scope `modules/core-modules/`; return LOCATIONS; purpose: Step 5's
  wire-or-record table (the row set is whatever the grep returns; there is no
  fixed expected set, and the three net-new raft-default keys are excluded).

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
  extrusion paths, or is a holder-side wiring change needed?
  **RESOLVED at Step 3 (2026-09-05): the emit path did NOT exist and neither
  did the guest→`raft_fill` write transport; both absorbed as AD-240B-1.**
- None [BLOCK].
