# Requirements: 193-overhang-distance-prepass-carrier

## Packet Metadata

- Grouped task IDs: see `packet.spec.md` frontmatter. **The id there is an allocation, not a measurement** — queue row 9a2 was added without one, unlike every other row in that plan's central-allocation paragraph. Re-derive before writing the `docs/07_implementation_status.md` row, and reconcile **two disagreeing sources**: `rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -1` and `rg -o 'TASK-[0-9]{3}' .ralph/specs --no-filename | sort -u | tail -1`. The specs tree runs ahead of `docs/07` because several packets in this batch allocated ids they have not registered yet; take the next free number above the higher of the two.
- Backlog source: `docs/specs/deviation-backlog-remediation-plan.md` — §Packet Queue row **9a2**, created by that plan's `Queue amendment (2026-07-25d)` recording the maintainer's **option (C)** ruling on packet 190. **Do not quote the row's text or any hit count here or anywhere else.** Re-derive at the moment of use with `rg -n '^\| 9a2 ' docs/specs/deviation-backlog-remediation-plan.md`; the rows begin at column 1, so `^` must anchor directly on the leading `|` (a pattern written `^.\|` consumes that `|` and then demands a second one — measured elsewhere in this packet set at 0 hits and exit 1, a silently-empty re-derivation of exactly the kind this note exists to prevent).
- Packet status: `draft`
- Aggregate context cost: `M`

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

## In Scope

- `crates/slicer-ir/src/slice_ir.rs`:
  - `Point3WithWidth` gains `#[serde(default)] pub overhang_distance_mm: Option<f32>` with a doc-comment stating the **signed** convention and the `+ boundary_offset` normalisation verbatim from `design.md` §Data and Contract Notes.
  - `SurfaceClassificationIR` gains `#[serde(default)] pub prev_layer_boundaries: HashMap<u32, Vec<ExPolygon>>`, keyed by **global** layer index exactly as `overhang_quartile_polygons` is.
  - `CURRENT_PERIMETER_IR_SCHEMA_VERSION` takes an **additive minor** bump (the constant governing `Point3WithWidth`; see `design.md` §Architecture Constraints for why it is that constant and not `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`). `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` takes its own additive minor bump. **Both live values are ledger facts — re-derive them; no number is frozen in this packet, and `AC-2` is written as a relative assertion against a Step-0 pin file precisely so it cannot rot.**
- `crates/slicer-schema/wit/deps/types.wit`: `record point3-with-width` gains `overhang-distance-mm: option<f32>`. **`record seam-point3-with-width` in the same file does NOT** — see `design.md` §Data and Contract Notes.
- `crates/slicer-schema/wit/deps/ir-types.wit`: `resource slice-region-view` gains `prev-layer-boundary: func() -> list<ex-polygon>`, beside the existing `overhang-quartile-polygons`.
- `crates/slicer-core/src/algos/overhang_annotation.rs`: `annotate_overhangs` additionally returns the previous-layer boundary contours it already computes for the diff. **Its `BAND_BOUNDARY_MULTIPLIERS` and all banding geometry are untouched.** `AC-N2` enforces this by comparing the `const BAND_BOUNDARY_MULTIPLIERS` **declaration** against its `HEAD` version, deliberately *not* by a whole-file `git diff --quiet`: the function's return shape changing while the constants do not is the intended — and only permitted — edit to that file, and a whole-file conjunct would forbid it.
- `crates/slicer-core/src/perimeter_utils.rs`: `expolygon_to_path3d` gains a previous-layer-boundary parameter and stamps `overhang_distance_mm` beside the quartile it already stamps; a new `signed_distance_to_boundary(x, y, &[ExPolygon]) -> Option<f32>` helper.
- `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`: `commit_overhang_annotation_builtin` populates the new map.
- `crates/slicer-wasm-host/src/marshal/**`: the new accessor and the new `point3-with-width` field.
- `modules/core-modules/classic-perimeters/src/lib.rs` and `modules/core-modules/arachne-perimeters/src/lib.rs`: pass the boundary through / stamp the distance. **The arachne site's assignment must sit outside its `if !overhang_bands.is_empty()` guard** (`AC-6`) — the two signals have different availability and a region with a boundary but no bands is exactly the fast-printing population packet 190 must interpolate for.
- **The exhaustive `Point3WithWidth` struct-literal sweep.** Re-derive with `rg -c 'dist_to_top_mm:' --glob '*.rs' crates modules xtask` and sum — every exhaustive literal names that field once, so it is an exact proxy for the sites needing an inserted initialiser. **Treat both the sum and the file count as ledger facts; `implementation-plan.md` Step 0 re-derives them and Steps 3-9 split the result by crate group.** The count over-reports slightly for a known reason: the struct *definition* itself and the WIT marshal conversions in `crates/slicer-wasm-host/src/marshal/{in_,out,leaf}.rs` also name the field without being literals needing this edit. `cargo check --workspace --all-targets`'s `E0063` set is the authority (`AC-8`).
- New tests: `crates/slicer-core/tests/overhang_distance_carrier_tdd.rs` (`overhang_distance_is_signed_and_boundary_offset_normalised`, `expolygon_to_path3d_stamps_signed_distance_and_none_on_empty_boundary`, `no_previous_layer_stamps_none_not_zero`, `quartile_stamping_is_unchanged_by_the_distance_carrier`); `crates/slicer-ir/tests/point3_overhang_distance_roundtrip.rs` (`absent_overhang_distance_deserializes_as_none`), mirroring the existing `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs`; `modules/core-modules/arachne-perimeters/tests/overhang_distance_tdd.rs` (`arachne_stamps_distance_for_regions_with_no_overhang_bands`).
- Doc edits enumerated in `packet.spec.md` §Doc Impact Statement, including one new `DEV-###` row for the per-point-vs-per-path `boundary_offset` divergence.

## Out of Scope

- **Any consumer of `overhang_distance_mm`.** `modules/core-modules/overhang-classifier-default/src/lib.rs` is untouched; it keeps emitting one `SetSpeedFactor` per entity. Packet 190 (TASK-309) changes that.
- **Mid-segment vertex insertion and any path-geometry mutation channel.** Packet 191 (TASK-310).
- **`annotate_overhangs`' four concentric quartile bands and `BAND_BOUNDARY_MULTIPLIERS`.** Recorded in the `DEV-009` row as an accepted permanent deviation; untouched here, in 190 and in 191. `AC-N2`'s declaration-vs-`HEAD` conjunct is the enforcement.
- **`CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`.** Packet 189 is separately specified to move it. `Point3WithWidth` reaches `LayerCollectionIR` transitively through `ExtrusionPath3D::points`, which is a real reason someone might reach for that constant — do not. Two packets bumping one line is a merge conflict dressed as a design decision.
- **Any new config key.** `enable_overhang_speed` and `slowdown_for_curled_perimeters` belong to packet 190.
- **`record seam-point3-with-width`.** A separate WIT record for a separate purpose; giving it the field would widen the seam-planning wire format for no consumer.
- **Whole-output G-code byte comparison as a verification technique.** `DEV-093` records that two runs of the same unmodified release binary already differ by ~100-160 lines, so no criterion here uses it.

## Authoritative Docs

- `docs/02_ir_schemas.md` — long; **ranged reads only** (§"IR 7 — PerimeterIR" and the `SurfaceClassificationIR` section). A line count is a ledger fact; do not pin one. Delegate anything wider.
- `docs/03_wit_and_manifest.md` — delegated grep only; the `point3-with-width` record entry and the `slice-region-view` accessor list.
- `docs/05_module_sdk.md` — delegated grep only; the "SliceRegionView accessors" section.
- `docs/adr/0031-overhang-classification-in-prepass.md` — read for the decision this packet **honours**. Note it already carries one in-body amendment (`### Amendment (overhang-after-Slice inversion)`), whose "stands unchanged" list names the multi-consumer motivation, the `SurfaceClassificationIR` extension shape, the quartile-polygon output, and keeping `overhang-classifier-default` as a finalization consumer. The `SurfaceClassificationIR` extension shape being explicitly preserved is what makes this packet's additive field a *continuation* of that ADR rather than a departure from it.
- `docs/adr/0053-*` — **forward reference; not on the tree at authoring time.** The decision record for the option (C) ruling. Cite it; do not restate it.
- `docs/DEVIATION_LOG.md` — delegated grep only. The `DEV-009` row is read for scope and is **not** edited by this packet; a new row is filed for the `boundary_offset` divergence with its id re-derived at the moment of writing.
- `CLAUDE.md` §"Guest WASM Staleness" and §"WIT/Type Changes Checklist" — both WIT edits invalidate every guest.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` — `estimate_points_properties`, consulted **only** for the definition of the quantity this carrier reproduces: `boundary_offset = PREV_LAYER_BOUNDARY_OFFSET ? 0.5 * flow_width : 0.0f`, and the assignments `start_point.distance = distance + boundary_offset` / `next_point.distance = distance + boundary_offset` where `distance` comes from `unscaled_prev_layer.distance_from_lines_extra<SIGNED_DISTANCE>(…)`. Template parameters in order: `SCALED_INPUT`, `ADD_INTERSECTIONS`, `PREV_LAYER_BOUNDARY_OFFSET`, `SIGNED_DISTANCE`; the G-code speed path instantiates `<true, true, true, true>`. The insertion branches are packet 191's and are **not** borrowed here.
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` — `ExtrusionQualityEstimator::estimate_extrusion_quality`, for the single fact that `unscaled_prev_layer` is built from the previous layer's **slice boundary**, not its extrusion paths. That is what `prev_layer_boundaries` reproduces and why option (C) does not inherit option (B)'s centerline proxy bias.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-11`.
  - Change-proving (measured FAIL on the unfixed tree, each verified by running the probe): `AC-1`, `AC-3`, `AC-7`, `AC-11`, and the structural half of `AC-6`. `AC-4`, `AC-5` and the test half of `AC-6` are change-proving by construction (their tests and their files do not exist yet). `AC-2` is change-proving relative to the Step-0 pin file and prints its own "pin file missing" FAIL until Step 0 has run — verified.
  - **`AC-4` is the packet's primary criterion.** It is the one that binds the carrier's meaning for packets 190 and 191, and it is the only place signedness and the `+ boundary_offset` normalisation are pinned in this repo.
  - Do-not-regress (must be PASS both before and after): `AC-9` (`build-guests --check` clean), `AC-10` (`slicer-core` + `slicer-ir` + the three largest runtime buckets). Baselines will have moved by the time this runs — **re-derive, do not pin.**
  - `AC-8` is neither: it is red by construction from the moment the field lands until the last sweep step exits, and its green is the sweep's completion signal.
- Negative: `AC-N1` (no boundary ⇒ `None`, never `Some(0.0)`/`Some(-1.0)`/`Some(f32::MAX)` — all three are live downstream sentinels), `AC-N2` (quartile stamping and `overhang_annotation.rs` bit-identical), `AC-N3` (absent field deserializes as `None`, which is what makes the bump additive).
- Cross-packet impact: packets 190 and 191 both read `overhang_distance_mm` and both cite `design.md` §Data and Contract Notes §"The signedness contract" **without restating it**. `AC-N1`'s forbidden-substitute list is normative for packet 191's unwrap rule.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Whole struct-literal blast radius compiles; the E0063 list is the authoritative sweep exit | FACT pass/fail; on failure, SNIPPETS of the E0063 file list only |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate over all targets including tests and benches | FACT pass/fail |
| `bash -c 'cargo test -p slicer-core --test overhang_distance_carrier_tdd 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | Home binary for AC-4, AC-5, AC-N1, AC-N2 | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-ir --test point3_overhang_distance_roundtrip 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | AC-N3, the additive-bump proof | FACT PASS/FAIL |
| `bash -c 'cargo test -p arachne-perimeters --test overhang_distance_tdd 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | AC-6's test half | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-runtime --test integration -- overhang_pipeline_e2e_tdd:: 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | The end-to-end overhang propagation pair — the closest existing test to this packet's data path, and it exercises the real `expolygon_to_path3d` whose signature changes | FACT PASS/FAIL |
| `bash -c 'cargo xtask build-guests --check 2>&1 \| rg -q "STALE:" && echo "FAIL: stale guests" \|\| echo PASS'` | Both WIT edits invalidate every guest | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo FAIL'` | Golden must not move: this packet has no consumer, so nothing may change in emitted G-code | FACT PASS/FAIL |

## Step Completion Expectations

- The field addition (Step 2) and the struct-literal sweep (Steps 3-9) must all land **before** any narrow test run is trusted. Until every `Point3WithWidth` literal compiles, every test binary in the workspace is a build failure, and a "test failure" attributed to the carrier design would in fact be an unswept literal.
- The two WIT edits and the guest rebuild are a single unit: `cargo xtask build-guests --check` must be run — and, if it reports `STALE:`, the rebuild performed — before any component, dispatch, or module-dispatch result from a later step is interpreted. `CLAUDE.md` forbids attributing such a failure to anything else until `--check` returns clean.
- `expolygon_to_path3d`'s signature change is the one edit in this packet that reaches beyond the sweep into live callers (`modules/core-modules/classic-perimeters/src/lib.rs`, plus test mirrors in `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs`). Land the signature and every caller in one step; a half-landed signature makes the E0063 sweep unreadable because two unrelated error classes interleave.
- `target/pin-perimeter-schema-before.txt` and `target/pin-surface-schema-before.txt` are written by Step 0 and read by `AC-2`. They are the mechanism that keeps a version assertion from being a frozen ledger fact. Do not delete `target/` between Step 0 and the acceptance ceremony; if it is cleaned, re-pin from `git show HEAD:crates/slicer-ir/src/slice_ir.rs` rather than from the working tree, or the assertion becomes vacuous.
- `target/guard-ac9-guests.txt` and `target/guard-ac10-193.txt` are the only shared scratch files; each key is unique to its criterion.

## Context Discipline Notes

- `crates/slicer-ir/src/slice_ir.rs` is very long — **ranged reads only.** Locate `pub struct Point3WithWidth`, `pub struct SurfaceClassificationIR`, `CURRENT_PERIMETER_IR_SCHEMA_VERSION` and `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` by symbol and open ±30 lines around each. Never load it whole.
- `docs/02_ir_schemas.md` — read only §"IR 7 — PerimeterIR" and the `SurfaceClassificationIR` block. Never load it whole.
- `docs/DEVIATION_LOG.md`'s `DEV-009` row is a single multi-thousand-word table cell. Delegate any question about it; do not read the row into the implementer's context.
- **The struct-literal sweep must not be done by reading the files.** Run the Step 0 re-derivation command to get the `file:count` list, edit each literal blind (one inserted `overhang_distance_mm: None,` line), and let `cargo check --workspace --all-targets` be the oracle. A file listed with a count but no `E0063` was a proxy false positive (the definition, or a marshal conversion) and needs no edit.
- `modules/core-modules/arachne-perimeters/src/lib.rs` and `modules/core-modules/classic-perimeters/src/lib.rs` are long. Only the `overhang_bands` regions are in scope; locate `region.overhang_quartile_polygons()` by symbol and open a window around it.
