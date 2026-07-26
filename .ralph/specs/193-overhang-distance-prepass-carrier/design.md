# Design: 193-overhang-distance-prepass-carrier

## Controlling Code Paths

- Producer path: `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`) → `commit_overhang_annotation_builtin` (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) → `SurfaceClassificationIR` (`crates/slicer-ir/src/slice_ir.rs`) → the host marshaller (`crates/slicer-wasm-host/src/marshal/in_.rs`) → `resource slice-region-view` (`crates/slicer-schema/wit/deps/ir-types.wit`).
- Stamping sites — the two places that already write `overhang_quartile`, and the two this packet extends:
  - `expolygon_to_path3d` (`crates/slicer-core/src/perimeter_utils.rs`), whose `overhang_bands: &[QuartileBand]` parameter classifies each vertex by winding-number membership; called by `modules/core-modules/classic-perimeters/src/lib.rs`.
  - the per-vertex `overhang_bands` loop in `modules/core-modules/arachne-perimeters/src/lib.rs` (packet 148's AC-5), currently nested inside an `if !overhang_bands.is_empty()` guard.
- Carrier: `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`) and `record point3-with-width` (`crates/slicer-schema/wit/deps/types.wit`).
- Consumers (**none in this packet**): packet 190's per-point interpolation and packet 191's crossing/segmentation predicates. Both read the field; neither is written here.
- Neighboring tests/fixtures: `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs` (the shape the new roundtrip test mirrors), `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` (the only existing test that drives real quartile bands through the real `expolygon_to_path3d`), `crates/slicer-gcode/tests/golden_emit_tdd.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **The governing schema constant is `CURRENT_PERIMETER_IR_SCHEMA_VERSION`, and the wrong answer is plausible enough to name.** `Point3WithWidth` is declared inside the `// Perimeter IR Types` banner section of `crates/slicer-ir/src/slice_ir.rs`, and `docs/02_ir_schemas.md` documents its struct block under `## IR 7 — PerimeterIR`. The plausible wrong answer is `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`, because the type reaches `LayerCollectionIR` transitively via `ExtrusionPath3D::points` — but packet 189 is separately specified to move that constant, and two packets editing one line is a merge conflict dressed as a design decision. `SurfaceClassificationIR` has its own constant, `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`, whose doc-comment already records an additive bump for the `overhang_quartile_polygons` map itself — the exact shape of this packet's second addition. **Both live values are ledger facts. Re-derive them; `AC-2` is written as a relative assertion against a Step-0 pin file so that no literal version can rot into an acceptance criterion** (packet 189's `AC-2`, which hardcodes `1.2.0`, is the failure mode being avoided).
- **`None` is a value, not an absence.** `overhang_distance_mm` is `Option<f32>`, not `f32`, and `#[serde(default)]` on an `Option` yields `None` — which is why the bump is additive (`AC-N3`). A bare `f32` would default to `0.0`, and `0.0` is a *meaningful* distance under this carrier's contract ("exactly on the offset boundary"), so a bare `f32` would make "not measured" indistinguishable from "on the boundary" at every one of the sites the sweep touches. That is not a stylistic preference: packet 191's normative unwrap rule and this packet's `AC-N1` both rest on `None` being representable.
- **Additive beside the quartile, never instead of it.** `overhang_quartile` keeps its four bands, its `BAND_BOUNDARY_MULTIPLIERS`, and its job. Packet 190 keeps `overhang_quartile.is_some()` as the "is this an overhang at all" gate under option (C) exactly as it planned to under option (B); the new field supplies only *how far*. `AC-N2` locks this with a test **and** a probe comparing the `const BAND_BOUNDARY_MULTIPLIERS` **declaration** against its `HEAD` version, because a test that does not read that constant cannot see an edit to it. The probe is scoped to the declaration rather than the whole file on purpose — §Code Change Surface requires work *in* that same file (`annotate_overhangs` returns the previous-layer contours it already computes), so a whole-file conjunct would forbid an edit this design mandates. What is out of bounds is the band **geometry**, not the file.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. **This packet edits two files under `crates/slicer-schema/wit/`, which invalidates every guest's bindgen — not just the two perimeter modules.**
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **…and here the snippet's concrete instruction is "convert exactly once, at the polygon boundary, and never again."** `Point3WithWidth.x/.y/.z/.width` are documented as **millimetres**, and `expolygon_to_path3d` already performs the conversion with `slicer_ir::units_to_mm(p.x)` before constructing each point. `ExPolygon` contours, by contrast, are in scaled 100 nm integer units. `signed_distance_to_boundary` therefore takes its query point in **millimetres** and converts the boundary, or takes both in units and converts the result — it must not mix, and its unit convention must be stated in its doc-comment. The failure mode this constraint guards is a distance out by 10⁴, which would place every point far outside every section threshold and look like "the feature is off" rather than like a bug.

## Code Change Surface

- **Selected approach — carry the previous layer's slice boundary through the existing prepass→perimeter channel, and stamp a signed offset distance at the two sites that already stamp the quartile.** This is the shape option (C) was ruled for: classification and its inputs stay upstream, the finalization module stays a consumer, and the measured quantity is canonical's — a distance to the previous layer's **slice boundary**, which is exactly what `ExtrusionQualityEstimator` builds `unscaled_prev_layer` from.
- **Exact functions, types, records and tests:**
  - `crates/slicer-ir/src/slice_ir.rs`: `Point3WithWidth` (field), `SurfaceClassificationIR` (field), `CURRENT_PERIMETER_IR_SCHEMA_VERSION`, `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`.
  - `crates/slicer-schema/wit/deps/types.wit`: `record point3-with-width`.
  - `crates/slicer-schema/wit/deps/ir-types.wit`: `resource slice-region-view`.
  - `crates/slicer-core/src/algos/overhang_annotation.rs`: `annotate_overhangs` returns the previous-layer contours alongside the bands. **Constants and banding geometry untouched** — see `requirements.md` §In Scope for the escape hatch if the return change cannot be made without touching tracked content.
  - `crates/slicer-core/src/perimeter_utils.rs`: new `signed_distance_to_boundary`; `expolygon_to_path3d` gains a boundary parameter and one stamped field.
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`: `commit_overhang_annotation_builtin` populates the new map.
  - `crates/slicer-wasm-host/src/marshal/{in_,out,leaf}.rs`: the new record field and the new accessor. **These three files name `dist_to_top_mm` today and will therefore appear in the sweep's proxy count — they are conversions, not literals, and they need a real edit rather than a blind inserted line.**
  - `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`.
  - Tests named in `requirements.md` §In Scope.
- **Rejected alternative 1 — compute the distance in the finalization module against the previous layer's `OuterWall` centerline.** This is packet 190's original option (B) design. Rejected by maintainer ruling: it re-adds the cross-layer wall-distance code ADR-0031 records as deliberately deleted, and it requires hand-compensating the `line_width/2` wall-proxy bias that ADR-0031's own Context gives as the reason classification moved off walls in the first place. It also measures against the wrong geometry — extrusion centerlines rather than the slice boundary canonical uses — which is a systematic, one-directional error on every section threshold.
- **Rejected alternative 2 — give the finalization guest access to `SliceRegionView`.** This was packet 190's `[BLOCK-3]`, and it is **settled by measurement, not by preference**: `world-finalization.wit` imports only `slicer:common/host-services` and `slicer:config/config-types`; `run-finalization`'s signature is `(layers, output, config)`; `layer-collection-view` exposes exactly six methods (`layer-index`, `z`, `entity-count`, `ordered-entities`, `tool-changes`, `z-hops`); and `host-services` exposes fifteen functions, none region-, surface- or quartile-related. Reaching `slice-region-view` from a finalization guest requires a `world-finalization` **world** change plus a rebuild of every guest — a larger change than this packet, for a worse answer.
- **Rejected alternative 3 — derive a pseudo-distance from the quartile band index.** Free, and worthless: every point in a band shares one distance, so the "continuous" signal is the same step function, and packet 190's `AC-16` (`interpolated_factor_is_not_a_quartile_value`) is written specifically to fail against it.
- **Rejected alternative 4 — a side table keyed by entity id, as packet 189 uses for speed profiles.** `LayerCollectionIR.speed_profiles` is the right shape for a value **produced during finalization**, because finalization is where entity ids are stable and where the producer runs. This value is produced at *perimeter-generation* time, on the same `Point3WithWidth` the quartile already rides, and it must survive every downstream transform that already carries the quartile — `interpolate_point` (`crates/slicer-core/src/lib.rs`), the arachne `simplify` pass, fuzzy-skin, seam placement. A side table would have to be re-keyed by each of those; a field is carried for free. The cost is the struct-literal sweep, which is exactly the trade packet 189 declined for *its* value and this packet accepts for *this* one — the difference is that the quartile precedent proves the field survives the transforms.
- **The sweep's size, re-derived rather than quoted.** `rg -c 'dist_to_top_mm:' --glob '*.rs' crates modules xtask`, summed. `implementation-plan.md` Step 0 re-derives it and prints the per-crate-group breakdown that Steps 3-9 are sized against. **Do not quote a figure from this document.** The proxy over-reports for a known reason: the struct definition itself and the three `crates/slicer-wasm-host/src/marshal/` conversion files also name the field. `cargo check --workspace --all-targets`'s `E0063` set is the authority (`AC-8`), and a listed file with no `E0063` was a proxy false positive.

## Files in Scope (read + edit)

Target at most 3 primary files; this packet exceeds that because a prepass→guest data path is by construction a producer, an IR type, a WIT record, a marshaller and two consumers, plus a workspace-wide struct-literal sweep. `implementation-plan.md` splits them so no single step edits more than one segment of the chain, and splits the sweep by crate group so no step rates `L`.

- `crates/slicer-ir/src/slice_ir.rs` — role: owns both carrier types and both schema constants; expected change: two fields, two additive minor bumps.
- `crates/slicer-schema/wit/deps/types.wit` — role: canonical WIT source read by both host `bindgen!` and the guest macro's `include_str!`; expected change: one record field.
- `crates/slicer-schema/wit/deps/ir-types.wit` — role: same; expected change: one `slice-region-view` accessor.
- `crates/slicer-core/src/perimeter_utils.rs` — role: the classic stamping site and the home of the new distance helper; expected change: one new function, one parameter, one stamped field.
- `crates/slicer-core/src/algos/overhang_annotation.rs` — role: the producer that already diffs consecutive layers; expected change: return the contours it already computes. **Constants untouched.**
- `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` — role: commits the producer's output into `SurfaceClassificationIR`; expected change: populate one map.
- `crates/slicer-wasm-host/src/marshal/{in_,out,leaf}.rs` — role: the WIT↔IR conversions; expected change: one field and one accessor each, as applicable.
- `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs` — role: the two guest-side consumers of the band data; expected change: pass the boundary through, stamp the distance. The arachne assignment must sit **outside** the `if !overhang_bands.is_empty()` guard.
- The struct-literal sweep files enumerated by `implementation-plan.md` Step 0's re-derivation — role: exhaustive `Point3WithWidth` literal sites; expected change: one inserted `overhang_distance_mm: None,` line each.

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-ir/src/slice_ir.rs` (very long) — locate `pub struct Point3WithWidth`, `pub struct SurfaceClassificationIR`, `pub struct QuartileBand`, `CURRENT_PERIMETER_IR_SCHEMA_VERSION`, `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`; open ±30 lines around each. **Never load whole.**
- `docs/02_ir_schemas.md` (long; a line count is a ledger fact — do not pin one) — §"IR 7 — PerimeterIR" and the `SurfaceClassificationIR` block only.
- `crates/slicer-core/src/perimeter_utils.rs` — locate `expolygon_to_path3d` and open ±60 lines; its doc-comment already states the winding-number classification rule the distance stamp sits beside.
- `modules/core-modules/arachne-perimeters/src/lib.rs` (long) — locate `region.overhang_quartile_polygons()` and the `if !overhang_bands.is_empty()` guard; open a window around each. Do not read whole.
- `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` (long) — its own module doc-comment records that it drives the **real** `expolygon_to_path3d` rather than a re-implementation, which is why its mirror helper must move with the signature.
- `docs/adr/0031-overhang-classification-in-prepass.md` — read whole; short, and its in-body `### Amendment (overhang-after-Slice inversion)` explicitly preserves "the `SurfaceClassificationIR` extension shape", which is the clause that makes this packet's additive field a continuation rather than a departure.
- `CLAUDE.md` §"Guest WASM Staleness" and §"WIT/Type Changes Checklist".

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `modules/core-modules/overhang-classifier-default/**` — **packet 190's exclusive surface.** This packet adds no consumer. Reading it to understand what will consume the field is acceptable once; editing it is not.
- `crates/slicer-core/src/algos/overhang_annotation.rs`'s `BAND_BOUNDARY_MULTIPLIERS` and every band-geometry expression — editing them reopens a closed user decision recorded in `DEV-009`. `AC-N2` enforces this with a `git diff --quiet` conjunct.
- `crates/slicer-ir/src/slice_ir.rs`'s `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` — packet 189's line.
- `record seam-point3-with-width` (`crates/slicer-schema/wit/deps/types.wit`) — a different record for a different purpose; see §Data and Contract Notes.
- `crates/slicer-gcode/**` — no emitter change; this packet has no consumer and must move no G-code.
- Unrelated crates — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: "Run `rg -c 'dist_to_top_mm:' --glob '*.rs' crates modules xtask`, return the total occurrence count, the file count, and the per-crate-group breakdown (`crates/<crate>` and `modules/core-modules` as groups)."; scope: workspace; return: `LOCATIONS` (≤ 20 group rows, not the file list); purpose: Step 0's sweep sizing.
- Question: "Does `cargo check --workspace --all-targets` pass? If not, return only the distinct file paths carrying `E0063` (missing field) errors."; scope: workspace; return: `FACT` plus at most 20 paths; purpose: every sweep step's exit.
- Question: "In `crates/slicer-core/src/algos/overhang_annotation.rs`, does `annotate_overhangs` already hold the previous layer's region polygons at the point it computes the diff, and in what coordinate units?"; scope: that file; return: `FACT` ≤ 5 lines; purpose: Step 4's producer change.
- Question: "In `crates/slicer-wasm-host/src/marshal/`, which file converts `point3-with-width` in each direction, and which converts `slice-region-view`'s accessors?"; scope: `crates/slicer-wasm-host/src/marshal/**`; return: `LOCATIONS` ≤ 10 entries; purpose: Step 5.
- Question: "In `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`, quote the two lines of `estimate_points_properties` that assign `distance + boundary_offset`, and state the four template arguments used by the G-code speed path's instantiation."; scope: that file; return: `SNIPPETS` ≤ 1 of ≤ 10 lines plus ≤ 40 words; purpose: the §Data and Contract Notes signedness contract.
- Question: "In `docs/02_ir_schemas.md`, quote the `pub struct Point3WithWidth` code block and the `SurfaceClassificationIR` struct block verbatim."; scope: that file; return: `SNIPPETS` (≤ 2, ≤ 30 lines each); purpose: Step 10's doc edit without loading the file.

## Data and Contract Notes

### The signedness contract — **normative for packets 190 and 191; they cite this section and must not restate it**

`Point3WithWidth.overhang_distance_mm: Option<f32>` is:

> the **signed** perpendicular distance, in millimetres, from the point to the **previous layer's slice boundary**, **already normalised by adding `boundary_offset`**, where `boundary_offset = 0.5 × width` and `width` is the point's own stamped extrusion width.
>
> - **Negative** ⇒ the point lies inside (over) the previous layer by more than `boundary_offset`.
> - **Zero** ⇒ the point lies exactly on the offset boundary.
> - **Positive** ⇒ the point overhangs beyond the offset boundary.
> - **`None`** ⇒ no distance was measured: there is no previous layer, or the previous-layer boundary for this region is empty. `None` is **never** to be substituted with `0.0`, `-1.0` or `f32::MAX` by any producer or consumer.

This is exactly the quantity canonical's `estimate_points_properties` writes into `ExtendedPoint::distance`: `distance = distance_from_lines_extra<SIGNED_DISTANCE>(…) + boundary_offset`, with `boundary_offset = PREV_LAYER_BOUNDARY_OFFSET ? 0.5 * flow_width : 0.0f`. The G-code speed path — the one packets 190 and 191 port — instantiates the function `<true, true, true, true>`, so `SIGNED_DISTANCE` and `PREV_LAYER_BOUNDARY_OFFSET` are **both true**. Verified against the checkout.

**Why this must be defined once, here, rather than per consumer.** Packet 190's original draft declared its in-module `distance_to_prev_boundary` as an **unsigned point-to-segment minimum** with no offset. Every predicate packet 191 ports reads this same field, and against an unsigned un-offset value:

- the XOR crossing test `(d_prev > boundary_offset + EPSILON) != (d_next > boundary_offset + EPSILON)` degenerates — for two endpoints on the same side of the wall but straddling the offset, an unsigned magnitude cannot flip the predicate the way canonical's signed value does;
- the outer proximity test `curr.distance > -boundary_offset && curr.distance < boundary_offset + 2.0` has an **unreachable negative half** — no unsigned value is ever `< 0`, so the guard collapses to a one-sided range and admits points canonical excludes;
- `a0 = clamp((d_curr + 3 × boundary_offset) / line_len, 0, 1)` and `a1 = clamp(1 − (d_next + 3 × boundary_offset) / line_len, 0, 1)` shift by a full `boundary_offset` on every evaluation, moving both inserted vertices along the segment.

None of those is a rounding difference; each is a different algorithm. The three artifacts (193, 190, 191) previously disagreed about this, which is why the queue amendment recording the ruling makes the single definition a named consequence.

**Consumers read the field as already-normalised.** Neither packet 190 nor packet 191 may re-derive `boundary_offset` in order to *interpret* the value — it is baked in. Both, however, need `boundary_offset` as a literal in their **own** predicates (`boundary_offset + EPSILON`, `-boundary_offset`, `3 × boundary_offset`, and packet 191's `min_spacing = flow_width × 0.25`). For those uses the rule is the same one this packet stamps with: `boundary_offset = 0.5 × width` of the point in question, and `flow_width` = that same `width`. Packet 190's open `[FWD]` on "which width feeds `path_width`" is answered by this: it is the point's own `width`, because that is what the carrier was normalised against, and a second independently-derived width would silently decouple the section thresholds from the distances they threshold.

### Other contract notes

- **`boundary_offset` is per point here and per path in canonical — a recorded divergence, not an oversight.** Canonical's `flow_width` is `path.width`, constant for an `ExtrusionPath`. PnP stamps per point. For classic perimeters every point of a loop shares the one `width` argument passed to `expolygon_to_path3d`, so the two are identical. For **arachne variable-width** loops the offset varies along the loop, bounded by the loop's own width variation. `packet.spec.md` §Doc Impact Statement requires one new `DEV-###` row recording this; the id is re-derived at the moment of writing.
- **`record seam-point3-with-width` deliberately does NOT get the field.** It is a separate WIT record serving seam planning, declared two lines from `point3-with-width` in the same file. Adding the field there would widen a wire format for no consumer, and — because the two names share a suffix — it is also the reason `AC-3`'s probe is windowed with a `(?<!seam-)` lookbehind rather than being a bare substring search. A loose anchor here matches the wrong record; the same class of defect is recorded in packet 191's `AC-1`.
- **Unit boundary.** `ExPolygon` contours are scaled 100 nm integer units; `Point3WithWidth` coordinates are millimetres. `signed_distance_to_boundary` must state its convention in its doc-comment and convert exactly once. A mismatch is a 10⁴ error that presents as "the feature does nothing".
- **`prev_layer_boundaries` keying.** Keyed by **global** layer index, identically to `overhang_quartile_polygons`, whose own consumer note in `docs/02_ir_schemas.md` pins that keying explicitly. A per-object or per-region keying would not match the map it sits beside and would break the same way `overhang_quartile_polygons` did before that note existed.
- **WIT boundary.** Two additive record/resource changes on `slicer:types/geometry` and `slicer:ir-handles`. Both are read by every guest's bindgen, so **every** guest artifact is invalidated — not only the two perimeter modules. `AC-9` is the gate, and `CLAUDE.md` forbids attributing any later component failure to anything else until `--check` is clean.
- **Determinism/scheduler constraints.** No `[ir-access]`, `[claims]` or `[stage]` manifest entry changes, so no DAG edge moves. `signed_distance_to_boundary` must break ties deterministically (`total_cmp`, matching the existing `nearest_reference_point` convention) — `DEV-093` records a pre-existing whole-pipeline non-determinism and this packet must not add to it.

## Locked Assumptions and Invariants

- **The carrier is signed and `boundary_offset`-normalised.** Locked by §Data and Contract Notes; `AC-4` defends it. Reversible only by a contract change that must be propagated to packets 190 and 191 in the same edit.
- **`None` means "not measured" and has no numeric substitute.** Locked; `AC-N1` defends it, and packet 191's unwrap rule depends on it. `0.0` reads as "on the boundary", `-1.0` collides with packet 191's `min_distance` not-found sentinel, and `f32::MAX` satisfies `|d| > min_distance`. All three are live sentinels downstream, which is why the AC names all three rather than just asserting `None`.
- **`overhang_quartile` is bit-identical after this packet.** Locked; `AC-N2` defends it with a test **and** a `git diff --quiet` conjunct on `overhang_annotation.rs`.
- **The schema bumps are additive.** `#[serde(default)]` on both new fields keeps existing fixtures deserializable; `AC-N3` defends it and `AC-2` asserts the *relative* bump rather than a literal version.
- **No emitted G-code moves.** No consumer exists in this packet. Defended by `golden_emit_tdd` in §Verification Commands, and *not* by whole-output byte comparison, which `DEV-093` makes unusable.

## Risks and Tradeoffs

- **The struct-literal sweep is the largest single risk of a botched merge in this three-packet set** — several times packet 189's `LayerCollectionIR` sweep, and spread across nine crate groups. Mitigations: it is compiler-driven (every miss is an `E0063`); it is split across seven steps by crate group so no step rates `L`; the exit is `cargo check --workspace --all-targets` rather than a reviewer's eye; and the worklist is re-derived at Step 0 rather than transcribed here, so it cannot rot between authoring and execution.
- **Three files in the sweep's proxy count are not blind edits.** `crates/slicer-wasm-host/src/marshal/{in_,out,leaf}.rs` name `dist_to_top_mm` as part of real WIT↔IR conversions. Editing them mechanically produces a compiling file that silently drops the field across the guest boundary. They are called out in Step 5 and are deliberately **not** in any blind-sweep step.
- **`expolygon_to_path3d`'s signature change reaches live callers and a test mirror.** `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs`'s own doc-comment says it drives the real function rather than a re-implementation; if its helper lags the signature, the E2E pair either fails to compile or — worse, if someone re-stubs it — silently stops testing the real path. Land signature and callers in one step.
- **The packet is `M` at the top of its band.** Adding a prepass data path *and* sweeping a workspace-wide struct is two packets' worth of surface joined by one field. §Open Questions records the split option explicitly rather than leaving it to be discovered at the context ceiling.
- **Two guest-invalidating WIT edits** mean a full guest rebuild in the middle of the packet, which is slow and is the most likely source of a confusing unrelated-looking failure.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` — Step 9 (the `modules/core-modules` sweep tail) and Step 5 (the marshaller, which is the only sweep-adjacent step whose edits are not mechanical). Neither exceeds `M`; both are the reason the sweep is seven steps rather than two.
- Highest-risk dispatch and required return format: the `cargo check --workspace --all-targets` sweep exit — must return `FACT` pass/fail plus at most 20 distinct `E0063` file paths, never the raw compiler output.

## Open Questions

- `[FWD]` **Should this packet be split into 193a (carrier field + WIT record + struct-literal sweep) and 193b (the prepass boundary path + the two stamping sites)?** The seam is clean: 193a is entirely mechanical and lands a `None`-everywhere field with zero behaviour; 193b lights it up. Recommended: **keep it whole** — a landed-but-never-stamped field is a carrier no test can prove works end to end, and the split would double the guest-rebuild cost. But the aggregate is at the top of `M`, and if a swarm run hits the context band before Step 9, splitting at that seam is the correct escalation rather than compressing scope. Recorded here so the decision is made deliberately rather than under pressure.
- `[FWD]` **Should `prev_layer_boundaries` carry `Vec<ExPolygon>` or a flattened line list?** Canonical builds an `AABBTreeLines::LinesDistancer` over lines. `ExPolygon` matches every neighbouring field in `SurfaceClassificationIR` and every other `slice-region-view` accessor, and the distance helper can flatten internally. Recommended: `ExPolygon`, for consistency with the map it sits beside. If the implementer picks lines, the accessor name and `AC-7`'s WIT conjunct must move with it.
- `[FWD]` **Should `signed_distance_to_boundary` use a linear scan or an acceleration structure?** Canonical uses `AABBTreeLines`. The existing overhang and curl paths in this repo use linear scans, justified in `modules/core-modules/overhang-classifier-default/src/lib.rs`'s doc-comment as "reasonable at this codebase's per-layer vertex counts". Recommended: linear scan for parity of engineering with the surrounding code; measure once before closing, and if it dominates, that is a separate packet, not a scope expansion here.
- `[FWD]` **Does `annotate_overhangs` need to return the boundary, or should `commit_overhang_annotation_builtin` re-read it from the committed `SliceIR`?** The producer already holds it for the diff, so returning it is free; re-reading is a second source of truth that can drift from the one the bands were derived from. Recommended: **return it from the producer**, and if that cannot be done without touching `overhang_annotation.rs`'s tracked content in a way `AC-N2`'s `git diff --quiet` conjunct would flag, move the new return to a sibling function in the same module's directory rather than weakening the conjunct.
