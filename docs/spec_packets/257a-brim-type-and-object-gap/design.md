# Design: brim-type-and-object-gap

## Selected Approach

**Derive the contour from what is printed, not from a new host input.**

Every canonical brim mode needs a per-object first-layer outline with holes. Canonical takes it from `object->layers().front()->lslices`. This tree's `skirt-brim` is a `FinalizationModule` running at `PostPass::LayerFinalization`; its `run_finalization` receives `&[LayerCollectionView]` and a `FinalizationOutputBuilder`, and the view exposes `layer_index()`, `z()` and `ordered_entities()`. Slice polygons are not among them.

The two ways to get an outline are (i) carry layer-0 slice `ExPolygon`s through to the finalization stage, or (ii) reconstruct it from the entities already in the view. Option (i) means a new field on the finalization input, which crosses the guest boundary — a WIT and marshalling change, which under this session's rules is a `[BLOCK]`. Option (ii) needs nothing new: `PrintEntity` already carries `role: ExtrusionRole` and `region_key: RegionKey { object_id, .. }`, so the module can group layer-0 entities by `region_key.object_id`, keep those whose role is `ExtrusionRole::OuterWall`, treat each `path.points` sequence as a closed loop, and union them with `slicer_sdk::host::clip_polygons(loops, &[], ClipOperation::Union)`. The union resolves nesting: an outer loop and the loops inside it come back as one `ExPolygon` with a contour and holes, which is exactly the shape `outer_inner_brim_area` consumes.

Option (ii) is selected. It keeps the whole packet inside one module, adds no cross-boundary contract, and needs no new host capability — `clip_polygons` and `offset_polygons` are already exported to guests by `slicer_sdk::host`.

**Band generation.** With an `ExPolygon` per object, the two bands are direct ports of `outer_inner_brim_area`'s two branches. The outer band offsets the contour outward by `brim_object_gap` to find its inner boundary, then walks outward one `line_width` per loop until `brim_width` is consumed. The inner band offsets each hole inward by `brim_object_gap` and walks inward the same way. The existing `make_rect_loop` helper stays, unused by the brim path, because the skirt still needs it.

**Per-object dispatch.** `brim_type` is resolved per object rather than globally, because canonical declares it on `PrintObjectConfig` and `Print::has_auto_brim` explicitly scans objects individually. This is the one place the packet does not simplify to a scalar-global key, and AC-4 pins it.

## Rule 4 Trigger Test

Authoring rule 4 routes an Orca enum whose values are *different algorithms living in different modules* to `claim:*` holders, and its trigger test says the rule does **not** fire on a module branching internally over a mode it implements itself — naming `seam_position`, `support_style`, `wall_sequence`, `retract_mode` and `wave_overhang_pattern` as the latter.

`brim_type` is the latter. Its values are not competing fill algorithms; they select *which parts of one object's outline* get a brim — outer, inner, both, or none — from a single band generator. Canonical itself expresses them as two booleans (`has_outer_brim`, `has_inner_brim`) over one code path, not as separate fillers. A holder-per-value split would create five modules that share one implementation and differ by a boolean pair, which is the coupling rule 4 exists to prevent, not the one it prescribes. Rule 4 does not fire; `brim_type` stays a manifest key on `skirt-brim`.

The `brim_ears` value is the one genuine algorithm difference, and it is a *separate pass* (`make_brim_ears_auto`) rather than a separate module — the `257b` split follows the packet boundary, not a claim boundary.

## Claims Held

- No new claim. `skirt-brim` keeps the claims it holds today; this packet changes what it reads and what it emits, never what it claims.
- Emitted entities keep `ExtrusionRole::Brim`, which remains the single source of truth for G-code labelling and host routing.

## Which Existing Mechanism Carries the New Data

| New behaviour | Carrier | Why not something else |
| --- | --- | --- |
| Per-object first-layer outline | `LayerCollectionView::ordered_entities()` + `PrintEntity.region_key.object_id` + `ExtrusionRole::OuterWall`, unioned via `slicer_sdk::host::clip_polygons` | A new finalization input field would cross the WIT boundary; the data needed is already in the view |
| `brim_type`, `brim_object_gap` reaching the module | `skirt-brim.toml` `[config.schema]` + `ConfigView::get`, the path `brim_width` already takes | Host-side special-casing would violate rule 4's "new decision points go where the architecture puts them" |
| Per-object brim identity | `RegionKey.object_id` on each emitted entity | The current literal `"brim"` object id erases the per-object distinction AC-4 needs |

No WIT type, no IR field, no `ResolvedConfig` field, no `SliceRegionView` metadata, no new `PostPass` claim.

## Recorded Divergences

- **DIV-1 — the brim outline is derived from deposited outer walls, not from slice polygons.** Canonical builds the brim from `lslices`, the sliced cross-section. The port builds it from the layer-0 `OuterWall` extrusion loops. The two differ by exactly the outer wall's own half-width, since a wall's centreline sits inside the slice boundary. **The port compensates by treating the derived union as the wall centreline and offsetting outward by `line_width / 2` before applying `brim_object_gap`**, so the brim's inner boundary lands on the deposited material's outer edge — which is what a brim must adhere to. Arguably more faithful than canonical's own behaviour, which offsets from a boundary the printer never touches. Recorded, not hidden: the compensation is a named constant expression in the code and AC-5 measures the resulting stand-off.
- **DIV-2 — objects with no layer-0 outer wall get no brim, and say so.** Canonical always has `lslices`. The port's derivation can legitimately find nothing (an object whose first layer is pure infill, or a module ordering that emits no outer wall). Rather than silently emitting no brim, the module records a diagnostic. Silent absence is the failure mode the support-planner defect work was created to eliminate.
- **DIV-3 — `brim_type`'s non-brim couplings are not ported.** Canonical also reads `brim_type` in `PerimeterGenerator::_traverse_loops` (outer-wall-first ordering on layer 0 under `btOuterOnly`), `SupportCommon.cpp::generate_support_toolpaths` (support-avoid expansion), `SupportSpotsGenerator` and `PrintObject::estimate_curled_extrusions`. Each is a different feature in a different module and none is in ticket 12's scope. Porting them from a brim packet would widen it silently. Named here so a reviewer does not read their absence as an oversight; reported to the map as separate gaps.
- **DIV-4 — `auto_brim` is `outer_only`, not an auto-detection heuristic.** Canonical's `btAutoBrim` participates in `has_outer_brim` exactly as `btOuterOnly` does within `outer_inner_brim_area`; the "auto" part lives in GUI-side width heuristics the port does not have. The port maps `auto_brim` onto the outer band and records that the width heuristic is absent.

## Tier Derivation

Ticket 04's rubric: **Tier A** is plumbing into a decision point that already exists; **Tier B** is new logic in an existing owner; **Tier C** is a new module at a new seam.

Neither key has a decision point today, so Tier A is out. The work is contour derivation and band generation inside `skirt-brim`, which already owns `PostPass::LayerFinalization` and already generates brim entities — an existing owner. No module is created, no seam is added, no claim is introduced. The packet is **Tier B** (was Tier A). The map's tier table needs the correction; it is listed below and not applied from here.

## Code Change Surface

**Editable:**

- `modules/core-modules/skirt-brim/src/lib.rs` — two new `SkirtBrim` fields and their `from_config` reads; a new contour-derivation helper; `generate_brim_entities` replaced by a per-object, per-band generator; the `RegionKey.object_id` change; the `run_finalization` brim block.
- `modules/core-modules/skirt-brim/skirt-brim.toml` — two net-new `[config.schema]` tables.
- `modules/core-modules/skirt-brim/tests/brim_type_tdd.rs` — net-new (AC-1, AC-2, AC-3, AC-4, AC-6, AC-N1).
- `modules/core-modules/skirt-brim/tests/brim_object_gap_tdd.rs` — net-new (AC-5).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — one bounds arm (AC-N2).
- `docs/15_config_keys_reference.md` — regenerated.

`modules/core-modules/skirt-brim/Cargo.toml` uses Cargo's test auto-discovery — it declares no `[[test]]` entries, verified at authoring time — so the two net-new test files need no manifest entry. **Re-derive this before relying on it**; if `[[test]]` entries have since been added, both files need one and Step 4's edit list grows.

**Read-only context:** `crates/slicer-sdk/src/host.rs` (the `clip_polygons` / `offset_polygons` signatures and the `ClipOperation` variants — range-read, do not load the file), `crates/slicer-ir/src/slice_ir.rs` (the `ExtrusionRole` variant list and `RegionKey` shape — ranged).

**Out of bounds — must not be loaded or edited:**

- `crates/slicer-gcode/src/serialize.rs` (AC-N4 asserts it is untouched).
- `crates/slicer-schema/wit/**` — no WIT change is in scope. Touching it means the packet was mis-scoped: stop and report.
- `crates/slicer-sdk/src/traits.rs` beyond a ranged read of `LayerCollectionView`.
- `modules/core-modules/` other than `skirt-brim/`.
- `docs/spec_packets/257b-brim-ears/` and every other packet directory.
- `docs/specs/orca-feature-gap/map.md` and `docs/specs/orca-feature-gap/issues/**`.

## Blast-Radius Discipline

`SkirtBrim` is a `pub` struct under `modules/*/src`; re-derive its field count before editing to determine whether it is a watched type under the struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`). If it is, the step adding the two fields owns every literal site in the crate's tests, and its exit condition is `cargo check --workspace --all-targets` **plus** `cargo xtask check-literals` in the same step — not a later step's discovery.

The `RegionKey.object_id` change from the literal `"brim"` to the owning object's id has its own blast radius: any test asserting `object_id == "brim"`. Re-derive those sites with a grep before Step 3 and include them in that step's edit list.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **`slicer_sdk::host` offsets take millimetres.** The planners' `host::offset_polygons(&polys, distance_mm, join, miter)` calls are the precedent; canonical's `scale_()` wrappers are deliberately not ported.
- **Hole offsets are negative, not reversed-polygon positive.** Canonical shrinks *reversed* holes by a positive amount. The port offsets the hole polygon by a negative distance, which is the same geometry in this tree's orientation convention. Assert the resulting loop lies inside the hole (AC-2) rather than trusting the sign.
- **The AGPLv3 porting header already at the top of `skirt-brim/src/lib.rs` must be retained**; it already names `src/libslic3r/Brim.cpp`, which is the file this packet ports further from (`docs/ORCASLICER_ATTRIBUTION.md`).

## Expected Dispatches

| Question | Scope | Return format |
| --- | --- | --- |
| Confirm the `has_outer_brim` / `has_inner_brim` derivation per `BrimType` value | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` `outer_inner_brim_area` | `SUMMARY` ≤ 200 words |
| Confirm the `BrimType` declared value order and default | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`, `PrintConfig.hpp` | `FACT` ≤ 5 lines |
| Confirm how `brim_object_gap` offsets the contour and the holes | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` `outer_inner_brim_area` | `SUMMARY` ≤ 200 words |
| Re-derive `SkirtBrim`'s field count and every `object_id == "brim"` assertion | `modules/core-modules/skirt-brim/**` | `LOCATIONS` ≤ 20 entries |
| Run each verification command | `requirements.md` § Verification Matrix | `FACT` pass/fail |

## Invariants

- `no_brim` emits zero `Brim`-role entities for that object and changes nothing else — in particular it does not suppress the skirt.
- The outer band never overlaps the object contour, and the inner band never leaves its hole. AC-2 pins both directions.
- Loop spacing is exactly one `line_width`; `brim_object_gap` shifts the band's start, it does not change the spacing.
- The skirt path is byte-identical before and after (AC-N5).
- An unknown `brim_type` value is an error, never a silent fallback (AC-N1). This mirrors the map's rule-4 ruling that a holder naming no module must fail validation rather than yield a silently hollow part.

## Risks

- **The derived outline is only as good as the outer-wall entities.** If a module ordering places `skirt-brim` before outer walls exist, the derivation finds nothing and every object silently loses its brim. DIV-2's diagnostic is the tripwire; Step 1's exit condition confirms the ordering empirically rather than assuming it.
- **Union of open polylines.** `make_rect_loop` closes its loops by repeating the first point; outer-wall entities produced by the perimeter modules may or may not. The derivation must close each loop before unioning, or `clip_polygons` will return degenerate results.
- **Per-object config resolution** is the one mechanism this packet leans on that `skirt-brim` does not use today. Step 2's falsifying exit is precisely that the module has no per-object config view — in which case `brim_type` degrades to a global key, AC-4 is dropped with a written reason, and the gap is reported.

## Open Questions

- `[FWD]` Whether `skirt-brim` can read a *per-object* `brim_type` at `PostPass::LayerFinalization`, or only a global one. Canonical declares the key on `PrintObjectConfig`, and AC-4 asserts the per-object behaviour. If the finalization stage exposes only a global `ConfigView`, the implementer must degrade `brim_type` to global, drop AC-4 with the reason written into this section, and report the per-object gap — **never** fake per-object behaviour by keying off geometry. This is a forward question, not a blocker: it changes one AC, not the packet's shape.

**No `[BLOCK]` is open.** The packet needs no new WIT interface, no IR schema bump, and no new host `ResolvedConfig` field. That is a direct consequence of choosing the entity-derived contour over a new finalization input — option (i) in § Selected Approach *would* have been a `[BLOCK]`, and was rejected for that reason among others.

## Map and Ticket Updates Required

Listed only; **not applied by this packet** (the map and tickets are out of bounds).

1. **Tier correction.** The map's P05 entry and ticket 04's tier table carry this packet as Tier A. `257a` and `257b` are both **Tier B**.
2. **Packet split.** P05's packet row must point at `257a-brim-type-and-object-gap` and `257b-brim-ears` instead of `257-brim-type-and-brim-keys`.
3. **Coverage-count correction.** `257a` covers **2** keys, `257b` covers **2**. `brim_use_efc_outline` leaves the P05 count entirely.
4. **A gap with no owner: elephant-foot compensation.** `elefant_foot_compensation` exists in this repository only as an `ORCA_CONFIG_PADDING` literal. Until first-layer EFC geometry exists, `brim_use_efc_outline` is unimplementable. It needs a queue row.
5. **A second gap with no owner: a point-valued brim paint carrier.** `PaintSemantic` attaches semantics to surface regions; canonical's `btPainted` brim consumes painted *points*. `brim_type = painted` needs that carrier.
6. **`brim_type`'s non-brim couplings** (DIV-3) are four separate gaps in `PerimeterGenerator`, support toolpath generation, and curling estimation. None has a queue row.
7. **The `brim_ears` precedent note is worth reinforcing in ticket 12.** The map already records that ticket 12 ruled the `brim_ears` *bool* dead while the ears *feature* is live through `brim_type == btBrimEars`. `257b` builds it; the ticket should not be read as having ruled the feature out.
