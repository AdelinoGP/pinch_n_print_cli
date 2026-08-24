# Design: 240-support-raft

## Controlling Code Paths

- Primary code path: `SupportPlanIR.raft_plan` (blackboard) → new
  `com.core.raft-default` guest (`Layer::Infill`) → `SlicedRegion.raft_fill`
  → ordinary ordered-entity G-code at negative global-layer indices.
- Neighboring tests/fixtures:
  `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs`,
  `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`,
  `modules/core-modules/tree-support-planner/tests/*` (RaftPlan producer),
  fixture `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference
  Obligations; do not repeat delegation rules.

## Architecture Constraints

- **ADR-0009 preserved:** rafts are signed negative global-layer PREFIX
  entries (`-1 .. -N`, sorting before model layer 0). No raft geometry may be
  minted as an `AnchoredEntity`, routed through `execute_per_layer_with_anchored_events`,
  or carried by any anchored-event structure (plan §15 prohibition). The
  existing `layer-idx = s32` WIT type and the signed doc comment on
  `ir-handles` already anticipate this — the Rust IR must match them.
- **Single-writer per IR is unchanged:** `com.core.raft-default` writes only
  the new `SlicedRegion.raft_fill` sub-field of `SliceIR`; it does not claim
  `SliceIR` wholesale against perimeter/infill writers. Its manifest declares
  `writes = ["SliceIR"]` with the fill-role claim narrowing actual ownership,
  mirroring how the four infill modules coexist via fill claims today.
- **Determinism:** raft polygon synthesis must be a pure function of
  (raft plan, config keys, region context) — no RNG, no
  iteration-order-dependent maps; identical inputs produce identical output
  across runs and across wasm/native legs. Downstream rectilinear path
  generation from those polygons is likewise deterministic.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Schema/version constants and event-specific locking: this packet bumps
  `CURRENT_SLICE_IR_SCHEMA_VERSION`'s minor for `SlicedRegion.raft_fill` +
  signed indices. The bump and every test asserting the old value land in the
  same step (Step 3); no locked wire format may read the live constant.

## Code Change Surface

### Selected approach

1. **Signed-index migration first** (substrate before consumer): retype the
   six u32 fields to i32, change `LayerModule::run_infill`'s parameter to
   `i32`, review the WIT boundary (`layer-idx` is already `s32`;
   `prepass-types.wit global-layer-index: u32` stays unless a raft entry
   crosses that specific view — it does not: raft entries live in
   `SupportPlanEntry.global_layer_index: i32` and layer execution), then fix
   every struct literal / assertion site from the LOCATIONS dispatch.
2. **Carrier + WIT accessor:** add `SlicedRegion.raft_fill: Vec<ExPolygon>`
   (serde default), add `raft-fill: func() -> list<ex-polygon>` to the
   `slice-region-view` resource in `crates/slicer-schema/wit/deps/ir-types.wit`,
   project in both marshal legs (`in_.rs` resource + `native.rs`
   construction), minor-bump the SliceIR schema version.
3. **New guest module** `modules/core-modules/raft-default/`: manifest TOML +
   guest src + `wit-guest/`. WIT world: **reuse the existing
   `slicer:layer-infill` world** (`world infill-module` in
   `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit`) — the module
   is a `Layer::Infill` synthesizer like `rectilinear-infill`; a new dep world
   would duplicate the identical import set for zero benefit. The new field's
   accessor rides `slice-region-view` in `ir-handles`, which the world already
   imports.
4. **Geometry port:** canonical `generate_raft_base` staged behavior with PnP
   unit discipline — synthesize object-independent raft footprint POLYGONS per
   `RaftPlan` counts (`raft_layers` total = first + base + interface bands),
   inflate by `raft_expansion` (÷100 → canonical-units), apply
   `raft_first_layer_expansion` on the first printed raft layer ("inflate in
   multiple steps" staging preserved as iterated offsets), and derive
   interface-band footprints at contact-distance spacing. Emit polygons only
   through `run_infill` into `SlicedRegion.raft_fill`; extrusion-path
   conversion happens downstream under the claim-holder emit path (see
   §ADR-0009 Reconciliation).
5. **Keys + decisions:** declare the three canonical keys in the new manifest;
   write wire-or-record rows for each dead raft key in the four support-module
   manifests; regenerate `docs/15_config_keys_reference.md`.

### Exact functions, traits, manifests, tests, and fixtures

- Retype: `GlobalLayer.index`, `ObjectLayerRef.{local_layer_index,global_layer_index}`,
  `SliceIR.global_layer_index`, `InfillIR.global_layer_index`,
  `SupportIR.global_layer_index` (`crates/slicer-ir/src/slice_ir.rs`);
  `LayerModule::run_infill` (`crates/slicer-sdk/src/traits.rs`); macro glue
  stage-method signatures (`crates/slicer-macros/src/lib.rs`).
- New field/accessor: `SlicedRegion` + `ir-types.wit` + `in_.rs` + `native.rs`
  (+ `out.rs` only if the read-back projection needs it).
- New module: `modules/core-modules/raft-default/{Cargo.toml, raft-default.toml,
  src/lib.rs, wit-guest/, tests/}`.
- Tests authored: `signed_layer_indices_tdd`, `sliced_region_raft_fill_tdd`
  (slicer-ir), integration cases `raft_prefix_orders_before_model_layers`,
  `raft_mints_no_anchored_entities`, contract cases
  `raft_keys_declared_and_wired`, `raft_index_outside_band_rejected`,
  scheduler case `raft_fill_double_holder_conflicts`.

### Rejected alternatives and reasons

- **New dep world for raft-default:** rejected — `slicer:layer-infill` world
  already imports exactly what the synthesizer needs; duplicating it adds a
  versioned surface to maintain with no capability difference.
- **Anchored entities for raft layers:** rejected — plan §15 prohibition;
  ADR-0009 contract.
- **Making `rectilinear-infill` hold `claim:raft-fill` (ADR-0009's v1 shape):**
  superseded by plan §12, which assigns the claim to `com.core.raft-default`
  itself (FORMAL AMENDMENT of ADR-0009 Decision 5, executed in Step 7 — see
  §ADR-0009 Reconciliation). The claim string and `should_emit` mapping stay,
  so a future pattern module can take the claim over without IR changes.

### ADR-0009 Reconciliation (normative for Steps 3–5; ONE position)

**The single position, stated once:** `com.core.raft-default` holds
`claim:raft-fill` (plan §12 240 brief is the governing authority; ADR-0009
Decision 5's assignment of the claim to the pattern module
(`rectilinear-infill`) is superseded by that plan and formally amended in
Step 7) AND writes `SlicedRegion.raft_fill` with deterministic fill polygons.
The module performs NO extrusion-path, flow, speed, or role-tagged rendering —
consumption/conversion of `raft_fill` polygons into printable paths happens
downstream under the claim holder's existing emit machinery.

ADR-0009's Decision point 4 ("`raft-default` … contains zero pattern
algorithms"), its Negative consequence ("synthesizer-only, zero pattern
code"), and Future-Reviewer Note "Do not re-suggest making `raft-default` a
renderer" are preserved UNCHANGED and read as prohibiting **pattern-algorithm
ownership and extrusion-path rendering** in this module. They do NOT prohibit
`com.core.raft-default` from computing region boundaries / fill-area polygons
deterministically; ADR-0009's own Decision point 4 says the synthesizer
"populates the raft polygon carriers", and plan §12 requires it to write
`SlicedRegion.raft_fill`.

Boundary adopted (and recorded in the Step 7 formal amendment):

- `com.core.raft-default` synthesizes **polygons only**:
  object-independent raft footprints per `RaftPlan` counts,
  `raft_expansion`/`raft_first_layer_expansion` inflation staging,
  interface-band footprints at contact-distance spacing — deterministic pure
  geometry into `SlicedRegion.raft_fill`. No scan-line pattern math, no
  `ExtrusionPath3D`, no flow/speed/role decisions.
- Conversion of those polygons into extrusion paths happens **downstream**
  through the existing `Layer::Infill`/emit machinery that the claim-holder
  path already drives — no new rendering code in this module, no pattern math
  duplicated anywhere.
- The claim remains reassignable: a future pattern module can take
  `claim:raft-fill` over by manifest change alone, without IR changes.

This keeps both authorities true: plan §12's "deterministic rectilinear
rendering" is satisfied by the downstream claim-holder conversion over
deterministic polygons; ADR-0009's synthesizer-only clause holds because no
pattern algorithm or renderer code lives in this module.

## Migration Table (signed indices u32→i32)

| Field/method | File | From→To | Notes |
| --- | --- | --- | --- |
| `GlobalLayer.index` | `crates/slicer-ir/src/slice_ir.rs` | u32→i32 | serde round-trip test |
| `ObjectLayerRef.local_layer_index` | same | u32→i32 | |
| `ObjectLayerRef.global_layer_index` | same | u32→i32 | |
| `SliceIR.global_layer_index` | same | u32→i32 | schema minor bump |
| `InfillIR.global_layer_index` | same | u32→i32 | consumed by run_infill callers |
| `SupportIR.global_layer_index` | same | u32→i32 | |
| `LayerModule::run_infill(_layer_index)` | `crates/slicer-sdk/src/traits.rs` | u32→i32 | all impls + macro glue |
| WIT `layer-idx` | `crates/slicer-schema/wit/deps/ir-types.wit` | already s32 | verify only; align host conversions |

Already-signed pattern: `SupportPlanEntry.global_layer_index: i32`,
`support-plan-entry-view.global-layer-index: s32`,
`prepass-support-geometry.wit global-layer-index: s32`.

### Enumerated blast radius (pre-baked; Step 2 executes, never discovers)

The implementing worker MUST dispatch this LOCATIONS sweep first and paste the
result into Step 2's edit list before editing:

> Question: enumerate every file containing a struct literal or field
> assignment of `GlobalLayer {`, `ObjectLayerRef {`, `SliceIR {`,
> `InfillIR {`, `SupportIR {` plus every `global_layer_index:` /
> `local_layer_index:` occurrence and every `fn run_infill(` impl; scope:
> `crates/ modules/`; return: LOCATIONS ≤20 entries (aggregate per file when
> larger, listing per-file counts).

Known hot files from grounding (worker verifies counts): `slice_ir.rs` itself
(Default impls + ~8 other u32 `global_layer_index` fields NOT in scope —
`SupportCandidateSource`, `AnchoredEntity.anchor_global_layer_index`, etc.
stay untouched), `crates/slicer-sdk/src/views.rs` (view builders),
`crates/slicer-wasm-host/src/marshal/{in_,out,native}.rs` (boundary `as`
conversions), `crates/slicer-runtime/src/blackboard.rs` + `layer_executor.rs`
(layer loop plumbing), `crates/slicer-macros/tests/*.rs`
(`slicer_module_tdd.rs`, `binding_surface_tdd.rs` call `run_infill(0, …)`),
`crates/slicer-gcode` consumers of `global_layer_index`, plus executor/
integration tests constructing these structs. Literal-gate rule applies: any
watched-type literal gains a `..` rest or waiver comment per
`docs/21_data_defaults_and_fixtures.md`; production literals stay exhaustive.

## Transport Reuse Diagram

```
tree-support-planner guest
  └─ push_raft_plan(RaftPlan)            [exists; prepass_support_geometry]
       ├─ crates/slicer-sdk/src/prepass_builders.rs::SupportGeometryOutput::push_raft_plan   [exists]
       ├─ crates/slicer-macros/src/lib.rs glue (output.push_raft_plan)                        [exists]
       ├─ crates/slicer-wasm-host/src/host.rs (resource fn push_raft_plan)                    [exists]
       ├─ marshal/in_.rs (host→guest projection of RaftPlan)                                  [exists]
       ├─ marshal/native.rs (native leg RaftPlan)                                             [exists]
       └─ crates/slicer-runtime/src/blackboard.rs::raft_plan_min merge                        [exists]
              ↓ blackboard slot: SupportPlanIR.raft_plan
com.core.raft-default (NEW, Layer::Infill, holds claim:raft-fill)
  reads: SupportPlanIR.raft_plan + LayerPlanIR (Z of prefix layers)
         + SliceIR (per-region context)
  writes: SlicedRegion.raft_fill  [NEW field]
       ↓ host partition/delivery (existing region flow)
G-code emission at negative-index layers (ordinary ordering; NO anchored events)
```

Nothing upstream changes except the index types it already flows through.

## DEV-124 Protocol

1. Re-run the two pinned contract tests under a raft-configured config view
   (AC-6 commands). Expected green, unchanged.
2. Record the verdict + the unported `has_bottom_shell_layers` residual in
   `requirements.md` §DEV-124 Verify-Record (already scaffolded above).
3. Any failure is a NEW finding: file a `DEV-1xx` row via the standard ledger
   procedure (re-derive next free ID at write time) and route it; never widen
   the assertions.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` - role: IR retypes + new field + schema bump; expected change: six i32 fields, `raft_fill`, version history doc.
- `crates/slicer-schema/wit/deps/ir-types.wit` - role: WIT accessor + boundary verification; expected change: `raft-fill` accessor on `slice-region-view`.
- `modules/core-modules/raft-default/**` - role: new guest module; expected change: full directory (manifest, src, wit-guest, tests).
Justified extras (blast-radius-owned, edited inside their owning steps):
`crates/slicer-sdk/src/traits.rs`, `crates/slicer-sdk/src/views.rs`,
`crates/slicer-macros/src/lib.rs` + its two test files,
`crates/slicer-wasm-host/src/marshal/in_.rs`, `.../native.rs`,
`crates/slicer-runtime/src/{blackboard.rs, layer_executor.rs}` call sites,
the four support-module manifests (wire-or-record annotations),
`docs/02_ir_schemas.md`, `docs/15_config_keys_reference.md`,
`docs/adr/0009-*.md`, new test files listed above.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` - lines `1020-1200` (`GlobalLayer`,
  `ObjectLayerRef`) and `1340-1430` (`SupportPlanEntry`, `RaftPlan`,
  `SupportPlanIR`) and `1760-1840` (`SlicedRegion`, `SliceIR`) - purpose:
  exact shapes before edit; the rest of the ~2.8k-line file is ranged-only.
- `modules/core-modules/tree-support-planner/src/lib.rs` - lines around
  `push_raft_plan` (~1540-1580) - purpose: producer contract; never load whole file.
- `crates/slicer-runtime/src/blackboard.rs` - lines around `raft_plan_min`
  (~755-790) - purpose: merge semantics.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load (T1: gitignored, glob-blind).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/tree-support-planner/src/lib.rs` beyond the cited
  range (~5.9k lines) - planner algorithms are 238b's surface; read ranges only.
- `crates/slicer-core/src/algos/support_geometry.rs` and other 238a/238b/238c/
  239/241-owned files - delegate symbol lookups; do not browse.
- `docs/spec_packets/215-raft-geometry/` - 236 deletes it; inspect via SUMMARY
  dispatch if provenance needed, never edit.

## Expected Sub-Agent Dispatches

- LOCATIONS sweep for the migration blast radius (question verbatim above);
  scope `crates/ modules/`; return LOCATIONS; purpose: Step 2 edit list.
- OrcaSlicer SUMMARY: `generate_raft_base` staging order (expand-first vs
  contact-first, multi-step inflation counts); return SUMMARY; purpose: Step 5.
- FACT check that `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit`
  world imports suffice for the new guest; return FACT; purpose: Step 4.

## Data and Contract Notes

- IR/manifest contracts: `SliceIR` schema minor-bumped; `SlicedRegion.raft_fill`
  serde-defaulted so old JSON loads; config keys snake_case (E9); manifest
  `[config.schema]` entries carry min/max/display/group like sibling modules.
- WIT boundary: canonical sources live at `crates/slicer-schema/wit/` (both
  host `bindgen!` and guest `include_str!` read them); after any WIT edit run
  `cargo build --tests`, then rebuild guests (T4).
- Determinism/scheduler constraints: single `claim:raft-fill` holder expected
  (`com.core.raft-default` per plan §12; ADR-0009 Decision 5 formally amended);
  double-holder surfaces as structured `ClaimConflict` advisory (post-G-21
  validator semantics) and per-region resolution stays deterministic.

## Locked Assumptions and Invariants

- Rafts remain signed negative global-layer prefix entries; never anchored
  entities (plan §15; ADR-0009).
- `SupportPlanEntry.global_layer_index: i32` semantics (-N..-1 raft band,
  0.. model) extend unchanged to the newly-signed fields.
- Canonical defaults: `raft_contact_distance` 0.1 mm, `raft_expansion` 1.5 mm,
  `raft_first_layer_expansion` 2.0 mm — declared as-is, converted ÷100 at the
  unit boundary.
- Invariant 16: every acceptance command names `--exact` tests or asserts a
  non-zero matched count in the same run.

## Risks and Tradeoffs

- **Migration breadth:** u32→i32 ripples wider than six fields in practice
  (call-site `as` casts, test literals). Mitigated by the pre-baked LOCATIONS
  sweep and a dedicated compile gate step; the step is bounded M, split L if
  the sweep exceeds ~20 files.
- **Schema bump fallout:** tests hard-asserting the old SliceIR schema version
  fail loudly; bump + fallout land in one step by design.
- **wasm/native leg skew (T9):** `raft-fill` must be projected in BOTH
  marshal legs; the determinism AC compares outputs so a one-leg omission
  fails visibly.
- **ADR boundary:** the polygon-synthesis vs pattern-rendering split is made
  explicit in §ADR-0009 Reconciliation and recorded as an ADR amendment
  (Step 7) rather than left to silent drift.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 migration; split trigger documented there)
- Highest-risk dispatch and required return format: LOCATIONS blast-radius
  sweep (must aggregate per-file counts, not raw hits)

## Open Questions

- [FWD] Does any non-test consumer rely on `GlobalLayer.index >= 0` in a way
  `i32` breaks beyond casts (e.g., sort keys assuming unsigned wrap)? Worker
  resolves during Step 2 via the LOCATIONS result; no activation blocker.
- [FWD] Exact interface-band spacing derivation from `raft_contact_distance`
  (canonical uses it between raft top and object bottom): worker confirms the
  precise consumption site via the delegated `generate_raft_base` SUMMARY in
  Step 5 and records the mapping in code comments; no activation blocker.
- None [BLOCK].
