---
status: implemented
packet: 193-overhang-distance-prepass-carrier
task_ids:
  - TASK-314
---

# 193-overhang-distance-prepass-carrier

## Goal

Give PnP a **continuous, signed, per-point overhang distance** where today it has only a four-bucket quantization: add `overhang_distance_mm: Option<f32>` to `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`) and the matching `overhang-distance-mm: option<f32>` to the `point3-with-width` WIT record (`crates/slicer-schema/wit/deps/types.wit`); carry the previous layer's slice boundary from `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`) through `SurfaceClassificationIR` and `SliceRegionView` to the two sites that already stamp `overhang_quartile` — `expolygon_to_path3d` (`crates/slicer-core/src/perimeter_utils.rs`, the classic-perimeters path) and the per-vertex band loop in `modules/core-modules/arachne-perimeters/src/lib.rs` (packet 148's AC-5) — and stamp the distance there beside the quartile; then sweep every exhaustive `Point3WithWidth` struct literal in the workspace.

This packet ships the carrier and changes no emitted G-code. It is the prerequisite the maintainer's **option (C)** ruling created for packet 190, and it is where the carrier's **signedness and `+ boundary_offset` normalisation are defined once** for packets 190 and 191 to reference identically.

## Problem Statement

### The signal packet 190 needs does not exist, and it cannot be recovered from the one that does

PnP's only per-point overhang signal is `Point3WithWidth.overhang_quartile: Option<u8>` — a four-bucket quantization stamped downstream of `annotate_overhangs`' concentric band polygons (`crates/slicer-core/src/algos/overhang_annotation.rs`). Canonical `ExtrusionQualityEstimator::estimate_extrusion_quality` interpolates speed over a **continuous** per-point distance to the previous layer's boundary. **The distance cannot be recovered from the bucket** — every point in a band shares one bucket, so interpolating over bucket-derived pseudo-distances reproduces the same step function, which is what packet 190's own §Code Change Surface rejects as "smoothing in name only".

Packet 190 as originally drafted resolved this by computing the distance **in-module**, against the previous layer's `OuterWall` centerline. That reversed ADR-0031's recorded removal of exactly that cross-layer wall-distance code, and — worse — it proposed to hand-compensate the `line_width/2` wall-proxy bias that ADR-0031's own Context gives as the *reason* classification moved off walls ("Walls are merely an inset-by-`line_width/2` proxy for the true cross-section"). That is `[BLOCK-2]` in `190/design.md` §Open Questions.

**The maintainer ruled option (C).** The continuous distance is stamped by the same prepass path that already stamps the quartile, from the previous layer's real slice boundary — which is what canonical measures against. Classification stays upstream, the module stays a consumer, the in-module wall-distance re-add is dissolved, and the whole `line_width/2` proxy-bias story goes with it. This packet is that carrier.

### What option (C) does and does not buy, stated so the ADR work is not over-claimed

It does **not** make packet 190 ADR-0031-conforming. `190/AC-6` removes `EntityMutation::SetSpeedFactor` from the module under **every** option, and applying `SetSpeedFactor` is named in ADR-0031's Decision text; ADR-0008's finalization-tier speed-factor decision is implicated the same way. Option (C) **narrows** the supersession rather than avoiding it. `ADR-0053` (being authored in a parallel workstream; not on the tree at authoring time) is the decision record, and it is written to cover packet 191's geometry mutation as well as packet 190's interpolation.

### The correction that shapes this packet's size

The queue row frames this as "add a field and sweep the literals". Grounded against the tree, that is only half of it. The prepass emits **banded polygons** (`SurfaceClassificationIR.overhang_quartile_polygons`), and the two stamping sites classify a vertex by winding-number membership against those bands. Neither site has the previous layer's boundary, and `SliceRegionView` does not expose it — its accessor list carries `overhang-areas` and `overhang-quartile-polygons` but nothing from which a signed distance to the previous layer can be derived (verified against `crates/slicer-schema/wit/deps/ir-types.wit`). Carrying the boundary from `annotate_overhangs`, which already diffs consecutive committed `SliceIR` layers to produce the bands, is therefore in scope: an additive `SurfaceClassificationIR` field, a `slice-region-view` accessor, and the host marshalling between them. `design.md` §Open Questions records the option of splitting that half into its own packet.

### Why this packet changes no emitted G-code

Nothing reads `overhang_distance_mm`. The field lands `None`-by-default on every construction site the sweep touches, is `Some(_)` only where the two perimeter stamping sites now write it, and no consumer exists until packet 190. That is deliberate: it lets the carrier land against the full existing golden and regression wall with `AC-N2` (quartile stamping bit-identical) and `AC-N3` (absent field deserializes as `None`) as the safety net, instead of landing a carrier and a behaviour change together.

## Architecture Constraints

- **The governing schema constant is `CURRENT_PERIMETER_IR_SCHEMA_VERSION`, and the wrong answer is plausible enough to name.** `Point3WithWidth` is declared inside the `// Perimeter IR Types` banner section of `crates/slicer-ir/src/slice_ir.rs`, and `docs/02_ir_schemas.md` documents its struct block under `## IR 7 — PerimeterIR`. The plausible wrong answer is `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`, because the type reaches `LayerCollectionIR` transitively via `ExtrusionPath3D::points` — but packet 189 is separately specified to move that constant, and two packets editing one line is a merge conflict dressed as a design decision. `SurfaceClassificationIR` has its own constant, `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`, whose doc-comment already records an additive bump for the `overhang_quartile_polygons` map itself — the exact shape of this packet's second addition. **Both live values are ledger facts. Re-derive them; `AC-2` is written as a relative assertion against a Step-0 pin file so that no literal version can rot into an acceptance criterion** (packet 189's `AC-2`, which hardcodes `1.2.0`, is the failure mode being avoided).
- **`None` is a value, not an absence.** `overhang_distance_mm` is `Option<f32>`, not `f32`, and `#[serde(default)]` on an `Option` yields `None` — which is why the bump is additive (`AC-N3`). A bare `f32` would default to `0.0`, and `0.0` is a *meaningful* distance under this carrier's contract ("exactly on the offset boundary"), so a bare `f32` would make "not measured" indistinguishable from "on the boundary" at every one of the sites the sweep touches. That is not a stylistic preference: packet 191's normative unwrap rule and this packet's `AC-N1` both rest on `None` being representable.
- **Additive beside the quartile, never instead of it.** `overhang_quartile` keeps its four bands, its `BAND_BOUNDARY_MULTIPLIERS`, and its job. Packet 190 keeps `overhang_quartile.is_some()` as the "is this an overhang at all" gate under option (C) exactly as it planned to under option (B); the new field supplies only *how far*. `AC-N2` locks this with a test **and** a probe comparing the `const BAND_BOUNDARY_MULTIPLIERS` **declaration** against its `HEAD` version, because a test that does not read that constant cannot see an edit to it. The probe is scoped to the declaration rather than the whole file on purpose — §Code Change Surface requires work *in* that same file (`annotate_overhangs` returns the previous-layer contours it already computes), so a whole-file conjunct would forbid an edit this design mandates. What is out of bounds is the band **geometry**, not the file.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. **This packet edits two files under `crates/slicer-schema/wit/`, which invalidates every guest's bindgen — not just the two perimeter modules.**
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **…and here the snippet's concrete instruction is "convert exactly once, at the polygon boundary, and never again."** `Point3WithWidth.x/.y/.z/.width` are documented as **millimetres**, and `expolygon_to_path3d` already performs the conversion with `slicer_ir::units_to_mm(p.x)` before constructing each point. `ExPolygon` contours, by contrast, are in scaled 100 nm integer units. `signed_distance_to_boundary` therefore takes its query point in **millimetres** and converts the boundary, or takes both in units and converts the result — it must not mix, and its unit convention must be stated in its doc-comment. The failure mode this constraint guards is a distance out by 10⁴, which would place every point far outside every section threshold and look like "the feature is off" rather than like a bug.

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
