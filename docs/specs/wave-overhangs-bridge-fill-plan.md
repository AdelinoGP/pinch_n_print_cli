# WaveOverhangs Bridge Fill Plan

Planning record for porting the WaveOverhangs algorithm (OrcaSlicer fork
`dennisklappe/OrcaSlicer-WaveOverhangs`) into Pinch 'n Print as the bridge fill
pattern. Produced by an adversarial grilling session; every decision in the
table below is user-confirmed.

**Status:** plan only. Spec packets will be authored from this document in a
later session via `/spec-packet-generator`. The decided split is four packets
(see the Packet Queue below); re-derive the next free packet numbers and task
IDs from `docs/spec_packets/` and `docs/07_implementation_status.md` at
authoring time.

**Feature identity:** this is a **bridge-fill adaptation** of the canonical
overhang-perimeter algorithm, not a faithful port of its stage/site semantics.
Canonical `WaveOverhangs::generate` replaces selected overhang perimeters; this
plan runs the generator over PnP's existing `bridge_areas` sites and records
the adaptation in packet prose. The divergence is deliberate and documented,
not parity.

**Normative references for the algorithm:**

- Fork checkout: `OrcaSlicerDocumented/` on branch `review-cleanup-issues`
  (base OrcaSlicer 2.4.0-dev + WaveOverhangs).
  - Algorithm body: `WaveOverhangs.cpp::generate`
    (`src/libslic3r/WaveOverhangs/WaveOverhangs.cpp`)
  - Call site: `generate_wave_overhang_paths` (`PerimeterGenerator.cpp`)
  - Fork's own docs: `docs/ALGORITHMS.md`, `docs/WAVE_OVERHANG_SETTINGS.md`,
    `docs/LIMITATIONS.md`
- Research lineage (per fork header and docs): Andersons / Sanchez / Vaneker
  (paper), McCulloch (arc-overhang predecessor), Klappe (OrcaSlicer port).

---

## Decided (user-confirmed)

| # | Decision | Choice |
|---|----------|--------|
| D1 | Object scoping | Both `prev_layer_boundaries` AND `overhang_quartile_polygons` become object-scoped nested serializable maps (object → global layer → payload). Major `SurfaceClassificationIR` bump. Marshal reads by `(object_id, global_layer_index)`. |
| D2 | Anti-reorder mechanism | Additive field **`order_lock: Option<u64>`** on `ExtrusionPath3D` (WIT `extrusion-path3d.order-lock`) plus an `OrderedEntityView` projection and an SDK allocator type for local tags. No new `ExtrusionRole` variant. |
| D3 | Lock strength | Atomic contiguous sequence: paths sharing a tag within one `(layer, object, region)` stay adjacent, in authored order and direction; the block may move as a unit. Host-enforced at every mutation point (InfillPostProcess commit, path-optimization proposal, finalization merge, tool rotation) with fatal contract errors. Locks protect sequence and geometry (points, widths); speed/flow side mutations remain legal. G-code emission bypasses D-P and min-segment for locked paths. |
| D4 | Anchor band | Tagged-path exception: order-locked paths are self-clipping — the ENTIRE swept footprint (variable-width segment trapezoids + round disks at every vertex) must lie inside `external_bridge_areas ∪ anchor_band`. The infill-linker carves untagged fill of all roles by that swept footprint. No host fifth polygon. |
| D5 | Fallback fill | Conventional rectilinear bridge scanlines with bridge role basics: current bridge orientation, resolved bridge width/nozzle fallback, canonical bridge spacing/flow, `BridgeInfill` with speed factor 1.0. Owned copy inside the wave module. No extraction to `slicer-core`, no sharing with `rectilinear-infill` (ADR-0026). |
| D6 | Internal bridges | Expose `SlicedRegion.internal_bridge_areas` through `SliceRegionView`/WIT. Waves only over `bridge_areas − internal_bridge_areas`; internal polygons get unlocked rectilinear fallback with today's role mapping; the host `InternalBridgeInfill` constructor is untouched. |
| D7 | Site boundary | Existing `bridge_areas` sites only. **Holder selection forces waves** — being configured as `bridge_fill_holder` is equivalent to canonical `use_instead_of_bridges = true` for external bridge sites. Cantilever/sloped support-free printing deferred. |
| D8 | Speed encoding | `speed_factor = wave_overhang_print_speed / bridge_speed` resolved per region; fatal rejection when the ratio falls outside the emitter clamp `[0.05, 5.0]` for that region's resolved `bridge_speed`. |
| D9 | Config keys | Fork-mirror subset (below) plus one PnP-only key. No master enable bool — being configured as `bridge_fill_holder` IS the enable. |
| D10 | Anchor depth | PnP-only `wave_overhang_anchor_depth_mm`, manifest default `0.0` = canonical-auto: `min(3 mm, bridge extrusion spacing × (wall_count + 1))` where spacing comes from the current bridge flow helper. Positive override accepted up to 20 mm. Band = expand(bridge_areas, depth) ∩ supported_fill. |
| D11 | Tag allocation | SDK allocator type hands out invocation-local tags from 1 (deterministic discovery order); `Some(0)` is rejected. Host remaps local tags to layer-unique global tags (bit 63 set) at every output boundary. One tag per connected wave domain. |
| D12 | Rollout | Four packets (queue below). Workspace-wide test ceremony runs once, at Packet 4 closure. |

### Why not a new ExtrusionRole (D2 rationale, condensed)

Keeping role `BridgeInfill` costs nothing we need: `resolve_feedrate`
(`crates/slicer-gcode/src/emit.rs`) maps `BridgeInfill` to `bridge_speed` whose
clamp floor accommodates the fork's 2 mm/s default; `part-cooling`
(`modules/core-modules/part-cooling/src/lib.rs::is_overhang_role`) already
treats `BridgeInfill` as its overhang trigger. A new variant would hardcode one
module's need into every consumer's match arms; a `Custom("…")` string
convention would scatter semantics across per-consumer string matches.
Surprise finding worth keeping: `Custom` roles already pass through the infill
linker verbatim today because `RoleBoundaries::for_role`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) returns `None` for
them — but their feedrate/fan/optimizer handling would still need string
special-casing, so the typed field wins.

---

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | object-scoped-overhang-annotation | Make both overhang annotation maps object-scoped with a major `SurfaceClassificationIR` bump. | TASK-353 | - | generated | docs/spec_packets/243-object-scoped-overhang-annotation/ |
| 2 | order-locked-extrusion-sequences | Land the generic order-lock carrier, namespace, SDK allocator, and host enforcement. | TASK-354 | #1 | generated | docs/spec_packets/244-order-locked-extrusion-sequences/ |
| 3 | lock-aware-infill-consumers | Make the infill linker, path optimizer, and G-code emission honor locked sequences. | TASK-355 | #2 | generated | docs/spec_packets/245-lock-aware-infill-consumers/ |
| 4 | wave-overhang-bridge-fill | Ship the wave bridge-fill module with internal-bridge exclusion and rectilinear fallback. | TASK-356 | #3 | generated | docs/spec_packets/246-wave-overhang-bridge-fill/ |

---

## Packet 1 — Object-scoped overhang annotation

Objective: stop cross-object contamination in the two host-only overhang
annotation maps, with a breaking-but-host-only schema change.

Today `commit_overhang_annotation_builtin`
(`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) merges
every object's previous-layer boundaries into one
`HashMap<u32 /*global_layer_index*/, Vec<ExPolygon>>` and every object's
quartile bands into one `HashMap<u32, Vec<QuartileBand>>`; the marshal
(`crates/slicer-wasm-host/src/marshal/in_.rs`, `SliceRegionData` assembly)
hands each region whichever polygons share its layer index regardless of
object identity. The host's own bridge gate is already object-scoped:
`slice_postprocess_prepass.rs` builds `lower_layer_polygons` keyed
`(ObjectId, u32)` before calling `gate_bridge_areas_by_unsupported_span`
(`crates/slicer-core/src/algos/prepass_slice.rs`), so prepass computes exactly
the right data and discards the object dimension for the view.

Change:

1. Replace both maps with nested serializable shapes keyed by object first
   (e.g. `HashMap<ObjectId, HashMap<u32, Vec<…>>>`). A literal
   `HashMap<(ObjectId, u32), …>` key is NOT acceptable: tuple keys do not
   serialize as a normal JSON map.
2. Marshal reads with `(view.object_id(), global_layer_index)` — the object id
   is already in scope at that lookup. WIT accessor signatures unchanged.
3. Desired side effect: `classic-perimeters` and `arachne-perimeters` consume
   `prev_layer_boundary()` for `overhang_distance_mm`
   (`crates/slicer-core/src/perimeter_utils.rs::signed_distance_to_boundary`
   call path) and stop measuring against other objects' boundaries in
   multi-object scenes; the quartile gate stops being supplied by another
   overlapping object. Update affected fixtures (sanctioned by test
   discipline).
4. This is a field-type replacement: per the IR versioning contract table in
   `docs/02_ir_schemas.md`, take the **major** `SurfaceClassificationIR` bump
   (1.3.0 → 2.0.0). The maps are host-only (never cross WIT), so the blast
   radius is the marshal, the two perimeter consumers, and struct-literal
   fixtures (`crates/slicer-ir/tests/ir_tests.rs`,
   `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`,
   `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`).
5. Update the packet-193 provenance rows in `docs/02_ir_schemas.md`.

## Packet 2 — Order-lock carrier and host enforcement

Objective: land the generic `order_lock` contract — carrier, namespace, SDK
allocator, and host-side enforcement — provably changing nothing for existing
slices (all-`None` paths take the old-equivalent branches).

1. `ExtrusionPath3D` (`crates/slicer-ir/src/slice_ir.rs`): additive
   `#[serde(default)] pub order_lock: Option<u64>`.
2. WIT: `record extrusion-path3d` gains `order-lock: option<u64>`
   (`crates/slicer-schema/wit/deps/types.wit`). Canonical source edit — both
   host bindgen and guest macro read these files directly. The
   `slicer:types/geometry` package is unversioned (ADR-0044): no WIT version
   tax.
3. `OrderedEntityView` projection gains the field end-to-end: host projection
   (`crates/slicer-runtime/src/layer_executor.rs::project_ordered_entities`
   and `crates/slicer-wasm-host/src/dispatch.rs::project_ordered_entities_from`),
   WIT `ordered-entity-view` record (`crates/slicer-schema/wit/deps/ir-types.wit`),
   SDK view (`crates/slicer-sdk/src/views.rs`), macro adapter
   (`crates/slicer-macros/src/lib.rs`), and wasm-host marshal out.
4. Schema bump: follow the `ExtrusionPath3D.tool_index` precedent (packet 226)
   — bump `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` additively in the same
   step, and sweep the test that hard-asserts the old constant value.
5. Production struct literals gain the field (compiler-enforced; exhaustive
   literals in `src/`, FRU rest in tests per `cargo xtask check-literals`).
6. Guest rebuild obligation: `cargo xtask build-guests` (drop `--check`)
   before any test run; `--check` exit 0 before attributing failures.

### Namespace and allocation

- **Local tags** are `1..2^63−1`, allocated by a new SDK allocator type
  (invocation-local, deterministic discovery order, fails on exhaustion).
  `Some(0)` is rejected at the output boundary.
- **Global tags** have bit 63 set. The host deterministically remaps local
  tags to layer-unique global tags at every output boundary: `Layer::Infill`
  commit, `Layer::InfillPostProcess` commit, and finalization merge. Unknown
  global tags in module output are a contract error.
- Producers: any layer producer, any InfillPostProcess module, and any
  finalization module may mint local locks. Consumers honor the field without
  knowing the producer.

### Enforcement (host-enforced invariant)

The host validates locked sequences at every mutation point and rejects
violations with a fatal, atomic contract error (prior IR preserved):

- InfillPostProcess commit: the replacement `InfillIR` must preserve every
  locked block from the prior `InfillIR` exactly (same paths, same order,
  same direction, same widths) and may only add new blocks.
- Path-optimization proposal application
  (`apply_entity_order_proposal`): no locked block may be split, interleaved,
  reversed, or reordered internally.
- Finalization merge (`apply_to`): `modify_entity` may not change locked
  geometry; `sort_layer_by` may not split or internally reorder locked
  blocks.
- Cross-layer tool-cluster rotation (`apply_cross_layer_tool_rotation` in
  `crates/slicer-gcode/src/emit.rs`): rotation must not split a locked block.

Semantics contract (goes verbatim into the ADR): paths sharing a tag within
one `(layer, object, region)` form an atomic contiguous sequence — they stay
adjacent, in authored order and point direction; the block may move as a unit
relative to unrelated entities. Locks protect sequence and geometry (points,
widths); speed/flow side mutations remain legal.

## Packet 3 — Lock-aware consumers

### C1. infill-linker honors locks

In `process_bucket_role`
(`modules/core-modules/infill-linker/src/orchestrate.rs`):

- Locked paths bypass boundary lookup, linking, overlap-offset trimming, and
  clipping — appended verbatim per region in emission order.
- Carve pass (module-local implementation per ADR-0026 single-caller rule):
  the swept footprint of locked paths — one endpoint-width trapezoid per
  segment plus a round disk at every vertex (covers end caps and corner
  joins) — is differenced out of every untagged role bucket of the same
  region. Host-side precedent for the segment-quad shape exists in
  `crates/slicer-runtime/src/visual_debug_render.rs::swept_fill_shape`; the
  linker needs its own guest-side equivalent, and the host precedent has no
  round caps — the linker's version adds them.

Locked paths may extend beyond `bridge_areas` into neighboring fill domains
(anchor band, D4) — this amends the four-canonical-fill-polygons invariant in
`docs/02_ir_schemas.md` § "Post-`Layer::Perimeters` invariant" and the
`CONTEXT.md` Infill entry.

### C2. path-optimization-default honors locks

`group_then_nearest_neighbor`
(`modules/core-modules/path-optimization-default/src/lib.rs`): each locked
block is one nearest-neighbor candidate (authored first start, last end),
never reversed, never split; blocks keep internal order. Mirrors the
wall-subsequence precedent of ADR-0011.

### C3. G-code emission honors locks

`DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`): locked
paths bypass both Douglas-Peucker (`infill_resolution`) and
`min_segment_length` pruning — every authored point is emitted. Coordinate
formatting at serialization still applies.

### C4. Parity gate

Structural parity tests prove neutrality while no module emits `order_lock`:
representative linker/optimizer/emitter fixtures with all-`None` locks produce
identical path/entity/G-code structures through the new branches. No new
golden files.

### Docs landed with Packets 2–3

- New ADR: *Order lock for print-order-sensitive extrusion sequences* — draft
  in Appendix A. Re-derive the next free number at landing.
- New ADR: *Sequence-locked paths may occupy neighboring fill domains* — draft
  in Appendix A; amends the invariant above.
- `CONTEXT.md` glossary terms — drafts in Appendix B.

## Packet 4 — `com.core.wave-overhangs` module

Scaffold mirrors `gyroid-infill`: `modules/core-modules/wave-overhangs/` with
`Cargo.toml`, `wave-overhangs.toml`, `src/lib.rs`, `src/generator.rs`,
`tests/`, `wit-guest/`. Registration: root workspace members, optional dep +
feature in `crates/slicer-integrated-modules/` and `crates/pnp-cli/Cargo.toml`.
Manifest claims: `holds = ["claim:bridge-fill"]` ONLY; `[ir-access] reads =
["SliceIR"] writes = ["InfillIR"]`. Selection via
`"bridge_fill_holder": "wave-overhangs"` (short-name match through
`slicer_scheduler::validation::module_id_matches_holder`). Ships in all
editions, opt-in; `rectilinear-infill` remains the default holder.

Porting header per `docs/ORCASLICER_ATTRIBUTION.md`; credit Andersons, Sanchez,
Vaneker, McCulloch, Klappe.

### View accessor prerequisite

Add `internal-bridge-areas` to the `slice-region-view` WIT resource
(`crates/slicer-schema/wit/deps/ir-types.wit`), the SDK `SliceRegionView`
field + getter + host-only setter (`crates/slicer-sdk/src/views.rs`), the
macro adapter (`crates/slicer-macros/src/lib.rs`), and the marshal
(`crates/slicer-wasm-host/src/marshal/in_.rs`). The module computes
`external_bridge_areas = bridge_areas − internal_bridge_areas` and waves only
the external subset.

### Region pipeline (where `should_emit(BridgeInfill)` && !bridge_areas().is_empty())

1. `supported_fill = prev_object_boundary ∩ union(top_solid_fill,
   bottom_solid_fill, sparse_infill_area)`
2. `anchor_band = supported_fill ∩ expand(external_bridge_areas, anchor_depth)`
3. `wave_domain = external_bridge_areas ∪ anchor_band`
4. **Holder selection forces waves** — the canonical
   `should_generate_waves_for_region` bridgeability gate is ported but
   effectively bypassed (equivalent to `use_instead_of_bridges = true`).
   Fallback triggers instead: missing anchors, empty seeds, min-length-filtered
   components, iteration residual, or empty generator output. No silent
   component drop: every nonempty external bridge component emits at least
   one wave or fallback path.
5. Waves emitted as role `BridgeInfill`, order-locked (one tag per connected
   wave domain), anchor-first. Internal-qualified polygons
   (`internal_bridge_areas`) get unlocked rectilinear fallback with the same
   role mapping rectilinear-infill uses today; the host `InternalBridgeInfill`
   constructor is untouched.
6. Flow: bead `width = nozzle_diameter`;
   `flow_factor = wave_overhang_flow_mm3_per_mm / (width × effective_layer_height)`.
   Volumetric-E precedent: `crates/slicer-gcode/src/emit.rs` computes E as
   distance × width × height × flow_factor / filament_area, so this encodes
   the paper's layer-height-independent bead area without new path fields.
7. Speed: `speed_factor` ratio per D8, resolved per region; fatal when
   unrepresentable.

Coordinate hazard: polygon offsets/intersections run in scaled integers
(1 unit = 100 nm) via `slicer_core::polygon_ops`; all mm constants
(spacing, overlap, widths, seed expansion) convert through `mm_to_units` at
entry — never raw Orca constants (docs/08_coordinate_system.md).

### Generator port (own copy, canonical `WaveOverhangs.cpp`)

Seed extraction along the supported boundary · narrow-neck split slits
(`generate_narrow_split_slits`) · accumulated-region offset loop at line
spacing · front extraction against the half-width-inset trim boundary ·
simplify/reconnect (`reconnect_polylines`) · pattern assembly:
`append_wave_fronts` (smart support-scored start-end choice),
monotonic append, `append_zig_zag_front_levels` meander · empty/short-front
filtering. Not ported: Kaiser/generator shells, inert `min_angle`,
inert `seam_mode`, progressive `spacing_mode`, corner taper (deferred), wall
replacement, floor layers, G-code event injection.

### Config keys (snake_case, fork defaults; anchor depth is PnP-only)

`wave_overhang_pattern` (smart|monotonic|zigzag) ·
`wave_overhang_line_spacing` 0.35 · `wave_overhang_perimeter_overlap` 0.1 ·
`wave_overhang_minimum_width` 0.7 · `wave_overhang_min_new_area` 0.01 ·
`wave_overhang_min_length` 0.0 · `wave_overhang_max_iterations` 0 ·
`wave_overhang_flow_mm3_per_mm` 0.15 · `wave_overhang_print_speed` 2.0 ·
`wave_overhang_anchor_depth_mm` 0.0 (canonical-auto sentinel, max 20.0, D10).

Required reads (declared in the module manifest so the config view exposes
them): `bridge_speed`, `bridge_line_width`, `bridge_flow`, `bridge_density`,
`nozzle_diameter`, `wall_count`, `layer_height`. All wave settings and
required bridge basics resolve per region (modifier/layer overrides honored).

---

## Tests

- Ordering survival through linker + optimizer (locked block byte-stable,
  directions kept; block-level nearest-neighbor placement).
- Carve correctness: no untagged fill overlaps swept wave area; round caps
  honored.
- First front intersects supported material; each subsequent front within one
  wavelength of a predecessor.
- Smart traversal determinism; double-run identical output; native ≡ wasm.
- Missing-anchor, narrow-anchor-band (fork issue #84 analog), and
  internal-bridge fallback cases leave no holes.
- Multi-object overlapping-footprint fixture proves object-scoped anchors
  (Packet 1).
- Layer-height invariance of extruded volume at fixed flow setting.
- `resources/A_upsidedown.obj` end-to-end: at least one contiguous locked
  `BridgeInfill` block in typed capture, plus emitted wave speed and volume
  matching configured values (typed lock + G-code physics — the discriminator
  against rectilinear fallback, which shares the role).
- Internal-area disjointness: no locked footprint overlaps internal areas;
  internal bridge paths still emitted by the host constructor.
- Structural parity: all-`None` identity through linker/optimizer/emitter
  fixtures (Packet 3).
- Visual-debug evidence: standard `Layer::Infill` / `Layer::InfillPostProcess`
  taps only. Custom seed/front/residual overlays are deferred — existing
  visual-debug cannot expose module-internal state.

## Validation commands

```bash
mkdir -p target && cargo test -p <crate> 2>&1 | tee target/test-output.log  # narrow runs
cargo check --workspace --all-targets
cargo xtask check-literals
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask build-guests            # after any WIT/IR field change
cargo xtask build-guests --check    # freshness gate before attributing failures
cargo xtask test --summary          # WASM-touching contract suites
```

Workspace-wide runs only at Packet 4 closure ceremony, after all narrow gates
pass.

## Explicitly deferred (later packets)

Support-free cantilevers/angled overhangs · solid floor layers above waves ·
Hilbert floor fill · per-wave fan/temp/travel/retract/dwell events · support
suppression under covered areas · inner-wall replacement in the overhang zone ·
corner taper reinforcement · custom visual-debug overlays for module-internal
seed/front/residual state.

---

## Appendix A — ADR drafts

Land in `docs/adr/` when Packet 2 starts. Re-derive numbering at landing.

### Draft: Order lock for print-order-sensitive extrusion sequences

**Status:** proposed (land with Packet 2).

**Context.** Wave-overhang bridge fill produces paths whose print order and
direction are physically load-bearing: fronts must be deposited anchored-first,
and chained zigzag runs break if reversed. Two downstream stages destroy such
sequences today: the infill linker re-clips, chains, and reverses bridge-role
paths; path optimization nearest-neighbor permutes role groups and may reverse
entities. Adding a dedicated role variant was rejected as one module's need
hardcoded into every consumer's match arms.

**Decision.** Add additive `order_lock: Option<u64>` to `ExtrusionPath3D`
(WIT `extrusion-path3d.order-lock`) and project onto `OrderedEntityView`.
None/absent preserves today's behavior exactly. Paths sharing a tag within one
`(layer, object, region)` form an **atomic contiguous sequence**: they stay
adjacent, in authored order and point direction; the block may move as a unit
relative to unrelated entities. Locks protect sequence and geometry (points,
widths); speed/flow side mutations remain legal. The host enforces the
invariant at every mutation point — InfillPostProcess commit, path-optimization
proposal application, finalization merge, and cross-layer tool-cluster rotation
— rejecting violations with a fatal, atomic contract error. G-code emission
bypasses D-P and min-segment pruning for locked paths.

Tags are allocated by the producing module through an SDK allocator type
(invocation-local, from 1, deterministic discovery order; `Some(0)` rejected).
The host remaps local tags to layer-unique global tags (bit 63 set) at every
output boundary; unknown global tags in module output are a contract error.
Any layer producer, InfillPostProcess module, or finalization module may mint
locks; consumers honor the field without knowing the producer.

**Considered options.** New `ExtrusionRole` variant (rejected — role
proliferation, scattered match arms). `Custom("…")` string convention
(rejected — invisible typing, per-consumer string matching). Entity-group
wrapper type in InfillIR/LayerCollectionIR (rejected — restructures every
downstream iteration for a guarantee a field carries). Trusting core modules
to comply without host enforcement (rejected — third-party optimizers and
future finalization sorters could silently violate the invariant).

**Consequences.** IR/WIT additive change → one-time guest rebuild; host gains
tag remapping and mutation-point validation; linker gains locked-passthrough +
swept carve branch (ADR-0026-consistent module-local code); optimizer treats
locked blocks as single non-reversible candidates; emitter bypasses
simplification for locked paths; production literals gain one field.

### Draft: Sequence-locked paths may occupy neighboring fill domains

**Status:** proposed (amends the four-canonical-fill-polygons invariant in
`docs/02_ir_schemas.md` and the `CONTEXT.md` Infill entry; land with Packet 3).

**Context.** The fill-partition contract says each fill-role holder emits over
exactly one pre-partitioned polygon. Faithful wave bridge fill cannot satisfy
this: canonical `WaveOverhangs.cpp` deliberately extrudes an anchor band INTO
supported material adjacent to the overhang (`generate_wave_overhang_seeds` +
seed expansion), because first fronts must bond to solid ground. Without an
exception, the linker clips those anchor sections away against the partitioned
polygon.

**Decision.** Order-locked paths are self-clipping: the producer guarantees
the ENTIRE swept footprint (variable-width segment trapezoids + round disks at
every vertex) lies inside its legal domain. The infill-linker neither clips nor
links them and differences untagged fill of every role in the same region by
that swept footprint. Band geometry is producer-owned: depth comes from module
config (`wave_overhang_anchor_depth_mm`, default = canonical-auto
`min(3 mm, bridge extrusion spacing × (wall_count + 1))`), never from the host
partition.

**Considered options.** Host-carved fifth partition polygon
(`bridge_anchor_area`) — rejected: encodes producer-config-derived geometry
into the generic partition; adds SlicedRegion/WIT/builder/marshal surface for
one consumer. Self-limiting waves to `bridge_areas` — rejected: removes
supported-side bonding; materially weaker waves; unfaithful port.

---

## Appendix B — CONTEXT.md glossary additions

Land in repo-root `CONTEXT.md` (infill/fill cluster) when Packet 2 starts.

### Order lock

A per-path marker grouping extrusion paths whose print order and direction are
physically load-bearing. Locked paths form an atomic contiguous sequence:
stages that reorder, reverse, merge, clip, or trim infill must leave locked
blocks untouched, ordinary fill must not overlap a locked path's swept area,
and G-code emission preserves every authored point. Host-enforced at mutation
points. Not a role and not tied to any one module — any fill holder may lock
its output.
_Avoid_: sequence tag, no-sort, pinned path, chain id.

### Anchor band

The strip of supported material beside a bridge area that wave bridge fill
deliberately extrudes into so its first fronts bond to solid ground. Owned by
the emitting fill holder under the order-lock exception; never a partition
polygon of its own.
_Avoid_: anchor area, seed zone, bridge margin.
