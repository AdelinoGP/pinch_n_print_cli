# Design: lightning kernel portability (Gate B)

## Selected Approach

Gate B is one bounded experiment: the module-owned kernel **source** receives the canonical planning
domains published by packet 07 and runs through the real native entry and a real WASM guest. The
driver decodes both outputs, compares each independently to the canonical replay graph, and records a
truthful verdict.

The kernel source is included, not reimplemented:

```text
feasibility-kernel/src/kernel/mod.rs
    #[path = "../../../src/kernel/distance_field.rs"] mod distance_field;
    #[path = "../../../src/kernel/tree_node.rs"]     mod tree_node;
    #[path = "../../../src/kernel/layer.rs"]         mod layer;
    #[path = "../../../src/kernel/generator.rs"]     mod generator;
```

`super::` links inside the four files resolve against this module tree, so the files compile
byte-identically in both crates. The include module is the only new file inside the included tree;
the four included files are never edited. `slicer-core` (default features) is added to the
feasibility crate because the included files call `slicer_core::{difference, offset, OffsetJoinType}`,
`slicer_core::geometry::closest_point_on_segment`, and `slicer_core::polygon_ops::clip_polylines`.
No `host-algos`, `rayon`, `boostvoronoi`, `slicer-runtime`, or production-module dependency enters
the crate.

`feasibility-kernel/src/lib.rs` gains `pub mod kernel;` and its crate-level comment is amended to
name the `slicer-core` exception: the crate remains independent of `slicer-runtime` and every host
algorithm, but it is no longer dependency-free of `slicer-core` because the included module source
links against it. Leaving the old "intentionally independent of `slicer-core`" sentence in place
would be a false claim about the compiled crate.

The module's `mod.rs` is deliberately **not** included: it derives inputs from `SliceIR` and hardcodes
production constants. Gate B supplies canonical inputs through its own adapter, so the experiment
tests the algorithm on canonical data rather than the production adapter's derivation.

## Input Adapter

`plan_lightning(input: &KernelInput) -> Result<KernelPlan, KernelError>` reads only
`planning_layers`, `canonical_region_order`, `scalar_source_region`, and `parameters`. `KernelInput.layers`
is unused by planning and is empty on the planned arm; a planned record that depends on `layers` is a
defect and the driver asserts the empty-`layers` case still produces the full graph.

- **Layer vector.** The outline vector spans global layers `0..=max`, where `max` is the highest
  global layer index in the canonical replay record. A layer with a `PlanningLayer` contributes one
  outline per `internal_domains` contour, in recorded order; a layer without one contributes an empty
  outline. This mirrors the producer's per-layer vector and the recording corroborates it: replay
  nodes exist only on the layer that has a planning record.
- **Region grouping.** The producer groups by `(region, polygon ordinal)`. Each internal domain is
  attributed to a region by matching its geometry against `PlanningLayer.source_catalog` surfaces
  (each surface carries a `SourceSurfaceId.identity`). Exactly one region per domain is required;
  zero or several is `KernelError::SourceProvenanceUnavailable` naming the layer and domain index.
  Domains are ordered by `canonical_region_order` then by recorded domain order, giving the polygon
  ordinal. One `Generator` is built per `(region, polygon ordinal)`.

  **Superseded — 2026-10-25 fidelity pass.** Canonical `generateTrees` accumulates every eligible
  expolygon of a layer into one flat `Polygons` set; it has no polygon-ordinal identity. The adapter
  now builds one `Generator` per region only, flattening every internal domain of that region and
  layer into contour-plus-hole rings. Positional island pairing across layers is gone. See the Gate B
  fidelity re-run in `docs/specs/layer-module-preparation-plan.md`.
- **Parameters.** `LightningParameters` is transcribed field-for-field from
  `reference.parameters`; every field is required (`deny_unknown_fields`). The constructor receives
  density, extrusion width, multiline, layer height, overhang angle, prune angle, and straightening
  angle in the recorded units. `supporting_radius_mm` is `None` exactly when density is zero and
  `Some` otherwise; a disagreement between the option and the density is a named error. A
  zero-density set plans a zero supporting radius, which the kernel derives itself; no nonzero radius
  is ever defaulted.
- **Scalars.** `Generator::new` derives `supporting_radius`, `wall_supporting_radius`,
  `prune_length`, and `straightening_max_distance`. The adapter asserts each derived value equals the
  recorded derived scalar converted from mm to PnP units, so a scalar regression fails before
  planning.
- **Provenance.** Each `GraphLayer.provenance` is the `PlanningLayer`'s `source_catalog`,
  `boundary_provenance`, and `operations` carried unchanged. `origin_region_index` is the owning
  region's index in `canonical_region_order`; it is always populated and always resolvable, even
  though the canonical capture cannot confirm it.

## Graph Extraction (`tree.rs`)

After `Generator::generate_trees`, walk `Generator::get_trees_for_layer(layer_id)` and each
`Layer::tree_roots` `NodeRef`:

- assign ids deterministically in traversal order with a visited set; ids start at 1 and stay below
  `1 << 24`;
- `parent_id` is the node that owns the child edge (`None` for a root), `root_id` is the root
  ancestor's id;
- `grounding` is `Some` only when `Node::get_last_grounding_location()` is `Some`. The point is
  resolved against `GraphLayer.provenance.boundary_provenance`: a boundary whose recorded endpoints
  contain the point yields `GroundKind::Boundary` when the point is outside every temporary anchor,
  otherwise `GroundKind::TemporaryAnchor`. No containing boundary is
  `KernelError::SourceProvenanceUnavailable` naming the layer and node — never a fabricated address;
- edges are emitted once per parent→child occurrence with their own ids; no geometry dedup;
- `sampled_segments` is `Generator::committed_segments_for_layer` converted to two-point segments.

The canonical recording puts `last_grounding` exactly on root nodes (non-null count equals
`root_count` for every case), so the adapter's `Some`/`None` split is directly comparable: a kernel
witness on a canonical-null node is a `fail`.

## Clip Surface

`clip_lightning(plan, input) -> Result<Vec<ClippedLayer>, KernelError>` is a pure function over
`KernelInput.layers[].regions[].final_fills`. It clips the planned tree polylines to each
`FinalFillInput.geometry` with the production `clip_polylines` open-subject clip, emits oriented
two-point segments in polyline order, and carries `dispatch_token`, `caller_identity`, and
`contributors` unchanged. A duplicate, missing, or ambiguous dispatch token is
`KernelError::InvalidInput` naming the token. It never clips to a planning domain and never welds or
reorders segments.

The canonical `reference.final_fills` is `null`, so the canonical clipped comparison cannot execute.
AC-3 proves the function on constructed fills; AC-5 records the canonical clipped population as
`inconclusive` with the manifest reason and the recorded fill counts. This is a measured limitation
and a migration blocker, not a skipped test.

## Transport

Use only `ExtrusionPath3D.points` and each `Point3WithWidth`'s x/y/z/width/flow fields. Every planned
record is one six-point sparse path with finite coordinates, actual layer Z, positive width, flow 1,
speed 1. Metadata integers are below `1 << 24`; zero encodes absent optional fields. Three tags,
ordered `(x,y)` pairs:

- Node tag 1: `[(1,node_id), (x,y), (parent_id_or_0,root_id), (origin_region_index,ground_code),
  (surface_id,ring_id), (segment_id,domain_code)]`. `ground_code` is `0` absent, `1` boundary, `2`
  temporary anchor; `domain_code` is `0` absent, `1` internal, `2` void, `3` temporary anchor.
  Every node is emitted, including isolated and coincident-but-distinct nodes.
- Edge tag 2: `[(2,edge_id), (parent_id,child_id), (parent_x,parent_y), (child_x,child_y),
  (root_id,origin_region_index), (0,0)]`. Every occurrence, no geometry dedup.
- Provenance tag 3: the complete `GraphProvenance` DTO as canonical UTF-8 JSON, split into eight-byte
  chunks: `[(3,chunk_index), (byte_len,chunk_count), (b0,b1), (b2,b3), (b4,b5), (b6,b7)]`.
  Zero-pad only the last chunk; reject nonzero padding, invalid UTF-8, unknown/omitted fields, and
  missing/extra/duplicate chunks.

The guest decodes test-only config strings `probe_input_json`, `probe_output` (`planned` or
`clipped`), and `probe_final_dispatch`. For `clipped`, it executes the whole plan from constructed
fills and emits only the selected dispatch's two-point segment multiset with its actual caller origin.
`PaintValue::Custom` inputs are rejected before dispatch; every emitted path has an explicit `Some`
origin because the current boundary has no clear-origin call.

## Adapters and Driver

- **Guest** (`crates/slicer-wasm-host/test-guests/lightning-portability-guest/`): `#[slicer_module]
  impl LayerModule` with the existing `run_infill` trait method, `cdylib` + `rlib`, `wit-bindgen`,
  the owned kernel path dependency, and an isolated `[workspace]` sentinel. Model it on
  `test-guests/sdk-layer-infill-guest` and add the sentinel pattern from
  `test-guests/infill-postprocess-echo-guest`. Its macro-generated `__slicer_native_entry()` is the
  native adapter. No `test` feature is required.
- **Native**: construct `slicer_sdk::native::NativeLayerRequest` with empty `regions`, a `ConfigView`
  carrying the probe keys, and call the generated `NativeStageEntry::Layer`; drain
  `NativeLayerResponse.infill` via `InfillOutputBuilder::sparse_paths()` and
  `sparse_path_origins()`, assert equal lengths before zipping, require every origin `Some`, and
  reject `None`/Custom.
- **WASM**: use `host::layer_infill::LayerModule::add_to_linker`/`instantiate`,
  `HostExecutionContextBuilder`, config/region/paint/output handles, the generated
  `slicer_layer_infill_infill().call_run(...)`, and `HostExecutionContext::infill_output`; fresh
  stores per call, via `tests/common/wasm_cache.rs::compiled_guest("lightning-portability-guest")`.
  Check both the trap/result layer and the collected records.
- Both feeds enter the same decoder. Compare each to `reference.replay.production.graph`
  independently; never to each other.

## Comparison

The comparator is multiset-based and id-agnostic:

- **Nodes**: multiset keyed by `(point, parent point or none, root point, is_root)` with multiplicity.
  Points are compared after the recorded transport conversion
  (`round-half-away-from-zero(canonical / 100)`) with `tolerance_pnp_units = 1`. Distinct nodes may
  not merge; coincident-but-distinct nodes survive as multiset multiplicity.
- **Edges**: complete directed edge-occurrence multiset keyed by `(parent point, child point)`.
  Missing/extra/reversed/disconnected records fail with the first difference named.
- **Roots**: isolated-root multiset keyed by root point; a root with no outgoing edge is isolated.
- **Grounding**: for every canonical node with `last_grounding = null` the kernel node carries no
  witness; for every non-null point the point lies on the addressed boundary segment (tolerance as
  above). Addresses resolve in the returned `GraphLayer.provenance`, not only in the input table.
- **Non-comparable, always named**: `origin_region_index`, the per-node provenance table,
  `sampled_segments`, and grounding `kind`. They are recorded in the outcome as
  `non_comparable_populations` with the manifest's absent reason where one exists; they never count
  toward a pass and are never silently omitted.
- **Clipped**: `inconclusive` for the canonical comparison, with the exact
  `reference.final_fills` absent reason and the recorded fill counts `13/2/2/4/6`. Constructed-fill
  semantics are AC-3's evidence.

Case status: `pass` when every comparable group matches; `fail` when any comparable group differs
(first difference named); `inconclusive` when a comparable group could not be produced (named). Gate
status: `pass` when all five cases pass, `fail` when any fails, otherwise `inconclusive`. A case
`pass` is a statement about the recorded comparable populations only; the outcome says so in the same
record.

## Outcome

`target/lightning-portability/outcome.json` records `gate: "B"`, status, executed flag, the
`#[path]` include list with a sha256 per included kernel source file, kernel/guest artifact hashes,
`build-guests --check` exit, native/WASM measured node/edge/root counts, per-case planned statuses,
per-case clipped status `inconclusive` with the manifest reason and fill counts, the named
non-comparable populations, commands/exits/log hashes, and named blockers. `migration_eligible` is
the conjunction of A `pass`, B `pass`, and independently recorded generic framework acceptance. This
packet does not verify framework acceptance or Gate A, so those are recorded as named blockers rather
than inferred from packet status.

## Measured Facts

Verified against the checked-in fixtures and the module source on 2026-10-23:

- `reference.replay.production.graph` is populated for all five cases with node/edge/root populations
  `27/16/11`, `46/32/14`, `41/27/14`, `41/25/16`, `48/30/18`. Node ids are globally unique, every
  edge resolves inside its own layer, roots have `parent_id = -1`, `is_root = true`, and
  `root_id == node_id`.
- `last_grounding` is non-null exactly on roots (counts equal `root_count`); every other node is
  null.
- `reference.planning_layers` records global layer 1 for every case, plus global layer 2 for
  `anchor_bridge_classic` (zero internal domains, two temporary anchors). Replay nodes exist only on
  layer 1. Internal-domain counts are `2/2/1/2/3`; region counts are `1/2/1/1/3`, so
  `anchor_bridge_classic` and `island_split_merge_classic` have two domains in one region and the
  others map one domain per region.
- `reference.parameters` is identical across the five cases and matches `LightningParameters`
  field-for-field. `Generator::new` derives the same four values in PnP units — `30,000`, `2,000`,
  `175`, and `175` — from those raw values; the recorded `3.0`, `0.2`, `0.017497`, and `0.017497` mm
  convert to those same units under the recorded transport conversion.
- `reference.clipped` holds `13/2/2/4/6` fills per case on layers 1 (and 2 for
  `anchor_bridge_classic`); every dispatch token resolves in `b_input.contributor_evidence`. Anchor
  statuses are 6 `Complete`/7 `Unavailable`; the other cases are all `Complete`.
- `manifest.cases[*].absent` records `b_input.input` ("RegionInput.outlines and per-region
  shell/classified surfaces are not recorded"), `reference.graph` ("planning.json graph nodes record
  no origin_region_index and no sampled_segments"), `reference.final_fills` ("FinalFillInput.policy
  needs per-region resolved angles the capture does not record"), and `reference.policies`.
- The four included files are large (ranged reads only). Their external-crate imports are only
  `slicer-ir` and `slicer-core`; every other import is `std` or a `super::` sibling.
- The production adapter in `modules/core-modules/lightning-infill/src/kernel/mod.rs` calls
  `Generator::new` with `config.infill_angle` in the `lightning_overhang_angle` slot and with
  `prune_angle = 5.0`, `straightening_angle = 0.0`; the canonical capture records
  `overhang_angle_deg = 45.0`, `prune_angle_deg = 5.0`, `straightening_angle_deg = 5.0`. Gate B's
  adapter passes the recorded values in the named slots; the production adapter's argument choices
  are out of scope and recorded as observations, not repaired.

## Files in Scope

| Area | Files |
| --- | --- |
| Kernel include | `modules/core-modules/lightning-infill/feasibility-kernel/src/kernel/mod.rs` (new), `feasibility-kernel/src/lib.rs` (module declaration and crate-comment amendment), `feasibility-kernel/Cargo.toml`, `feasibility-kernel/Cargo.lock` (generated) |
| Kernel adapters | `feasibility-kernel/src/{tree,planner}.rs` |
| Guest | `crates/slicer-wasm-host/test-guests/lightning-portability-guest/{Cargo.toml,src/lib.rs,Cargo.lock}` |
| Host dev wiring | `crates/slicer-wasm-host/Cargo.toml` dev-dependencies, root `Cargo.lock` (generated) |
| Driver/tests | `crates/slicer-wasm-host/tests/integration/{main,lightning_portability_tdd,lightning_portability_support}.rs` |
| Outcome | `docs/specs/layer-module-preparation-plan.md` (append-only), `docs/07_implementation_status.md` (`TASK-576` B line) |

Packet 07's fixtures, the four included kernel files, `model.rs`, the module's `mod.rs`, and the
recorder are read-only here. `lib.rs` is editable only for the `pub mod kernel;` declaration and
the crate-comment exception named above.

## Read-Only Context

Bounded windows only: kernel `model.rs` type definitions, the four included kernel files (for
signatures and import checks), the module's `mod.rs` input derivation, the `slicer:layer-infill` WIT
world, native request/response and output-builder definitions, packet 08's `prepare_lightning`
helper, and packet 07's fixture manifest. Do not load guest WASM, full JSON graphs, lockfile
contents, or generated bindings.

## Out-of-Bounds Files

Production lightning/producer/driver/paint, the production module's `src/` **except** the four
included kernel files and `src/kernel/mod.rs` (bounded read-only context), WIT/SDK/macros/runtime,
packet 07 fixture files and recorder, packet 08, other packets, ADRs/deviations, source-plan approved
body, backlog entries other than `TASK-576`.

## Dispatches

- Canonical grounding/attachment/sampling/clipping fact checks: `SUMMARY ≤200 words`, file+function.
- Guest build/freshness and native/WASM execution: `FACT` exits and counts, ≤20 failure lines.
- Comparator/difference audit: `SUMMARY ≤200 words`.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build, run `cargo xtask build-guests --check` and inspect its exit code: exit 0 fresh, 1 stale or lock divergence, 3 infrastructure (stop). Never use `rg -q 'STALE:'` to decide freshness. Guest execution uses `cargo xtask test`.

## Invariants

- The four included kernel files are byte-identical to the module's; any edit to them is a packet
  defect. The include list and per-file sha256 are recorded in the outcome.
- No host lightning producer, host fallback, or callable host-algos path; the owned kernel and wasm32
  guest closures exclude `host-algos`. Legitimate native SDK dependencies may contain it.
- No new WIT/SDK/public schema. The guest uses the existing `slicer:layer-infill` world.
- No `None`/`Custom` origin; equal path/origin lengths before zip.
- Every emitted id, origin index, and boundary address resolves; an unresolvable value is a named
  error, never a fabricated record.
- Scientific mismatch is recorded as a named `fail`/`inconclusive`; never repaired from expectations.
- Each decision test re-runs the driver in its own body, so AC-2/AC-5 do not depend on test
  execution order.

## Risks and Tradeoffs

- The algorithm may differ numerically or topologically from the canonical graph. That is a valid B
  `fail` and the packet's product.
- The `#[path]` include makes the module's source an input to the feasibility crate; a module-side
  edit changes Gate B's subject. The outcome's file hashes make the tested revision auditable.
- Canonical origin/provenance/sampling and the clipped comparison stay unverifiable until the
  recorder captures them. They are named blockers, and the packet does not pretend otherwise.
- The production adapter's hardcoded straightening angle is a real canonical-parity deviation; this
  packet records it rather than changing production behavior.
- Guest instantiation/freshness can fail for infrastructure reasons; exit 3 is a stop, not a
  scientific result.

## Context Cost Estimate

Aggregate M across six bounded steps; largest step M. The kernel port is an inclusion plus adapters,
not a rewrite; the driver reuses packet 08's preparation harness patterns.

## Open Questions

- [FWD] Does the module kernel reproduce the canonical graph on any family, and where is the first
  difference? Recorded at execution.
- [FWD] Which canonical-parity population should a recorder extension capture first: final-fill
  domains/policy for clipping, or per-node origin/provenance? Not this packet's decision.
- Limitation: final-fill contributor provenance is incomplete; affected dispatches stay a named
  migration blocker.
- Limitation: Arachne/translated/density companions are not gate coverage.
