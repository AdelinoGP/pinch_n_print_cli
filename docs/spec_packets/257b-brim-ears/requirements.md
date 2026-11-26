# Requirements: brim-ears

## Packet Metadata

- **Packet directory:** `docs/spec_packets/257b-brim-ears/`
- **Slug:** `brim-ears`
- **Status:** `draft`
- **Task IDs:** none (queue packet — `task_ids: []`, precedent packets 234a, 253–265)
- **Backlog source:** wayfinder ticket 12 (`docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md`), map `docs/specs/orca-feature-gap/map.md` packet P05
- **Tier:** **B** — new logic (two helpers and a mode arm) inside `skirt-brim`, which already owns the stage. No module, no seam, no claim. See `design.md` § Tier Derivation.
- **Authoring note:** this packet is the second half of the `257` split (`257a-brim-type-and-object-gap` + `257b-brim-ears`), taken with explicit user approval under the 210a/210b letter-suffix precedent and the map's Authoring rules 1–7. It is a net-new packet directory, not an overwrite; it supersedes nothing.

## Problem Statement

Ticket 12's key list contains two ear keys, `brim_ears_max_angle` and `brim_ears_detection_length`. Re-derived from disk at authoring time, **both have zero occurrences in Rust or TOML source under `crates/` and `modules/`** — not in a manifest, not in `ORCA_CONFIG_PADDING`, not as a `ResolvedConfig` field. (Each appears once in a generated `.gcode` artifact under `crates/slicer-runtime/target/`, which is build output, not source, and is not evidence of a read site.) There is no ear geometry in this tree at all.

The prior packet 257 dispositioned both "declared-with-gap", which Authoring rule 1 prohibits. Under rule 1 the choice is build or return, and until packet `257a` lands the honest answer would have been *return* — the pre-`257a` brim is a rectangle around the plate's global bounding box, and the only convex vertices a rectangle has are its four corners, so "detect sharp corners on the object contour" has nothing to run on. Ear detection is not a key-wiring problem; it is downstream of a geometry problem.

`257a` solves that geometry problem: it derives a per-object layer-0 contour by unioning the object's `ExtrusionRole::OuterWall` loops, and it builds the `brim_type` mode dispatch. Once that exists, ears become exactly what canonical says they are — a decimation pass and a corner test over a contour, emitting one small polygon per surviving vertex. This packet builds that.

A note on the precedent, because it is easy to get backwards: ticket 04/12 ruled the `brim_ears` **bool** dead, and it still is. The map records explicitly that the ears *feature* is live in canonical, reached through `brim_type == btBrimEars` rather than the retired bool, and that `brim_ears_max_angle` / `brim_ears_detection_length` are live with it. The dead-key ruling does not extend to the feature.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue; **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `brim_ears_max_angle` | **(b)** | `skirt-brim` | the corner-sharpness threshold, converted to canonical's `angle_threshold = (180 - max_angle) * PI / 180` and compared against contour vertex angles to select ear anchors | AC-2 |
| `brim_ears_detection_length` | **(b)** | `skirt-brim` | the Douglas-Peucker decimation tolerance applied to the contour before corner detection, with canonical's `0 = disabled` semantics and its below-four-points skip guard | AC-3, AC-4, AC-N1 |

Counts: **(a) 0 · (b) 2 · (c) 0 · (d) 0.** Zero declaration-only keys (map preflight gate (a)); both keys carry ACs asserting behaviour changes at non-default values (map preflight gate (b)).

The packet additionally ships the `brim_ears` **value** of `brim_type`, which `257a` declares and rejects. `brim_type` itself is `257a`'s key and is not counted again here.

## Returned to Queue — unimplemented

**None.** Both of this packet's keys are implemented.

Two adjacent items are deliberately not built and are recorded in § Out of Scope rather than as returned keys, because neither is a ticket-12 key:

- **`brim_ears_outer_only`** — canonical's flag that decides whether ear modes also produce an inner brim (it participates in `outer_inner_brim_area`'s `has_inner_brim` derivation). Not in ticket 12's list. This packet fixes the behaviour at canonical's default and records it.
- **`brim_type = painted`** — canonical `btPainted` dispatches `make_brim_ears` over user-painted points. `slicer_ir::PaintSemantic` attaches semantics to surface regions, not points, and has no brim variant. `257a` rejects the value by name; AC-N3 asserts this packet does not quietly enable it while shipping its sibling ear mode.

## Ruled Dead-in-Canonical

**None.** Both keys have read sites inside OrcaSlicer's slicing pipeline under `src/libslic3r/`, verified at authoring time by a delegated sweep that excluded `src/slic3r/GUI/**` (including `ConfigManipulation.cpp`), `PrintConfig.cpp` tooltip and label text, `Preset.cpp` key lists, and `IGNORE`/legacy-alias sets. Both appear additionally in `PrintObject::invalidate_state_by_config_options`, which is a key-*name* list and is excluded from the evidence below.

## Per-Key Canonical Evidence

Cited by file and function, never by line number (repo rule).

| Key | Canonical read sites under `libslic3r/` |
| --- | --- |
| `brim_ears_max_angle` | `Brim.cpp` `make_brim_ears_auto` — converted to `angle_threshold` for convex/concave vertex detection; `Brim.cpp` `outer_inner_brim_area` — read from the object config and forwarded |
| `brim_ears_detection_length` | `Brim.cpp` `make_brim_ears_auto` — Douglas-Peucker decimation tolerance applied before angle detection; `Brim.cpp` `outer_inner_brim_area` — scaled into `ear_detection_length` and forwarded |

### Canonical semantics this packet borrows exactly

`make_brim_ears_auto` is the function (the painted variant is `make_brim_ears`). For each contour `ExPolygon`:

1. **Decimate, conditionally.** If `ear_detection_length > 0`, the contour is decimated by `MultiPoint::_douglas_peucker` with that scaled length as tolerance. The decimation is **skipped if the result would drop below 4 points**. The parameter is therefore a simplification radius that suppresses ears on noisy near-straight geometry, and `0` disables it.
2. **Convert the angle.** `angle_threshold = (180 - brim_ears_max_angle) * PI / 180`.
3. **Select anchors.** The threshold is passed to `Polygon::convex_points` for outer brims, or `Polygon::concave_points` for inner brims.
4. **Emit per anchor.** One regular polygon of `POLY_SIDE_COUNT` sides at radius `size_ear`, translated onto the vertex. `size_ear` is computed by the caller as `brim_width_mod - brim_offset - flow.scaled_spacing()`.
5. **Subtract the island.** The caller subtracts the gap-offset object island, so only the annular part of each ear remains — an ear is a ring around a corner, never a disc overlapping the object.

Note the coupling this creates with `257a`: `size_ear` depends on `brim_object_gap` (`brim_offset`), which is `257a`'s key. Ears are not independent of the band geometry.

### Canonical parameters this packet must re-derive, not assume

`POLY_SIDE_COUNT` and the exact `flow.scaled_spacing()` term in `size_ear` are canonical constants that must be fetched by delegated read at implementation time, not guessed. `size_ear`'s spacing term maps onto this tree's `line_width`, and the mapping must be stated in the code comment.

## In Scope

1. **Decimation helper** in `skirt-brim`: a Douglas-Peucker contour simplifier parameterised by `brim_ears_detection_length` in millimetres, with canonical's two guards — skip entirely when the tolerance is `0`, and skip when the result would fall below four points (AC-4, AC-N1).
2. **Corner-detection helper**: compute each vertex's interior angle on the decimated contour and select vertices whose turn exceeds `angle_threshold = (180 - brim_ears_max_angle) * PI / 180`. Convex selection for the outer band, concave for the inner band, matching canonical's `convex_points` / `concave_points` split.
3. **Ear emission**: one regular polygon of the canonical side count at radius `size_ear`, centred on each anchor, then the gap-offset object island subtracted via `slicer_sdk::host::clip_polygons` with `ClipOperation::Difference` so each ear is an annulus (AC-5).
4. **`brim_type = brim_ears` dispatch**: the value `257a` declares and rejects becomes a live arm of the mode dispatch. `257a`'s four shipped modes are untouched (AC-N5); `painted` stays rejected (AC-N3).
5. **Manifest declarations**: `brim_ears_max_angle` (float, default `125`, `[0.0, 180.0]`) and `brim_ears_detection_length` (float, default `1.0`, min `0`, no max) on `skirt-brim.toml`, each with a `description` naming `make_brim_ears_auto`.
6. **Tests**: one net-new test file in `modules/core-modules/skirt-brim/tests/` (the crate uses Cargo test auto-discovery — re-derive this before relying on it), plus a bounds arm in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`.
7. **Docs**: regenerate `docs/15_config_keys_reference.md`.

## Out of Scope

- **`ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin.** Authoring rule 2: padding is not parity, is never a deliverable, and is not evidence. AC-N4 asserts the file is untouched.
- **`257a`'s change surface**: the contour derivation, the four non-ear modes, `brim_object_gap`, and the skirt path. This packet consumes them and must not modify them; AC-N5 guards the modes.
- **`brim_ears_outer_only`** — not a ticket-12 key; fixed at canonical's default and reported.
- **`brim_type = painted`** and a point-valued paint carrier — reported, not built (AC-N3).
- **`brim_use_efc_outline`** — returned to the queue by `257a`; elephant-foot compensation does not exist in this tree.
- **Canonical's `closest_point_on_matching_island` ear projection**, which only runs on the painted path under `use_brim_efc_outline`. Both of its preconditions are absent here.

## Authoritative Docs

- `docs/00_project_overview.md` — project goals the design must satisfy.
- `docs/03_wit_and_manifest.md` — manifest `[config.schema]` shape.
- `docs/08_coordinate_system.md` — the decimation tolerance and the ear radius are millimetres at the module boundary; the 1 unit = 100 nm hazard applies through the host geometry ops.
- `docs/21_data_defaults_and_fixtures.md` — the struct-literal churn gate governs the new `SkirtBrim` fields.
- `docs/ORCASLICER_ATTRIBUTION.md` — `skirt-brim/src/lib.rs` already carries the porting header naming `Brim.cpp`; keep it.
- `docs/15_config_keys_reference.md` (generated) — regenerate at close.

## Parity Evidence Standard

Under Authoring rule 5, "default matches and the value reaches the consumer" is **not** sufficient evidence for either key. Each key's evidence is a behaviour difference measured between two runs differing only in that key's value, with the non-default value named in the AC. AC-N5 exists solely as a regression guard on `257a`'s modes and is not evidence for any key.

## Acceptance Summary

| AC | Key | Class | Asserts |
| --- | --- | --- | --- |
| AC-1 | `brim_type` (`brim_ears` value) | — | ears emit only near convex corners; total path shorter than `outer_only` |
| AC-2 | `brim_ears_max_angle` | b | at `100` only the 90-degree corner qualifies; at `125` both do |
| AC-3 | `brim_ears_detection_length` | b | at `3.0` the zig-zag is decimated away and emits no ears; at `0.0` it does |
| AC-4 | `brim_ears_detection_length` | b | `0.0` disables decimation entirely |
| AC-5 | ear geometry | — | each ear is an annulus, excluding the gap-offset island |
| AC-N1 | `brim_ears_detection_length` | — | decimation skipped below four points |
| AC-N2 | both | — | out-of-bounds values rejected, not clamped |
| AC-N3 | `brim_type = painted` | — | still rejected by name |
| AC-N4 | padding | — | `serialize.rs` untouched |
| AC-N5 | `257a` modes | — | regression guard only |

## Verification Matrix

| Surface | Command |
| --- | --- |
| ear generator and both keys (AC-1 … AC-5, AC-N1, AC-N3) | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd 2>&1 \| tee target/test-output.log` |
| `257a` modes regression (AC-N5) | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd 2>&1 \| tee target/test-output.log` |
| `brim_object_gap` regression | `mkdir -p target && cargo test -p skirt-brim --test brim_object_gap_tdd 2>&1 \| tee target/test-output.log` |
| skirt regression | `mkdir -p target && cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 \| tee target/test-output.log` |
| scheduler bounds (AC-N2) | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log` |
| guest freshness | `cargo xtask build-guests --check` (inspect the exit code; never grep for `STALE:`) |
| type gate | `cargo check --workspace --all-targets` |
| lint gate | `cargo clippy --workspace --all-targets -- -D warnings` |
| literal gate | `cargo xtask check-literals` |

## Step Completion Expectations

- The manifest declaration and the module read for a given key land in the same step, or the manifest declares a key the module does not read — the disposition rule 1 prohibits.
- The decimation helper lands before the detection helper: `brim_ears_detection_length` is defined as "applied *before* detection", and building detection first would make AC-3 untestable.
- No step may edit `crates/slicer-gcode/src/serialize.rs`, and no step may modify `257a`'s contour derivation or its four shipped mode arms.
- `skirt-brim/src/**` feeds the guest build, so any step whose verification is a module test runs `cargo xtask build-guests --check` first and judges by its exit code.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `make_brim_ears_auto`, the whole algorithm: the `_douglas_peucker` decimation gated on `ear_detection_length > 0` and skipped below four points, the `angle_threshold = (180 - max_angle) * PI / 180` conversion, the `convex_points` / `concave_points` selection, and the per-ear regular polygon of `POLY_SIDE_COUNT` sides at radius `size_ear`.
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `outer_inner_brim_area`, for how `size_ear` is computed by the caller (`brim_width_mod - brim_offset - flow.scaled_spacing()`) and how the gap-offset object island is subtracted to leave the annulus.
- `OrcaSlicerDocumented/src/libslic3r/Polygon.cpp` — `Polygon::convex_points` and `Polygon::concave_points`, for the exact angle convention the threshold is compared against.
- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp` — `MultiPoint::_douglas_peucker`, for the decimation semantics `brim_ears_detection_length` parameterises.

## Context Discipline Notes

- `modules/core-modules/skirt-brim/src/lib.rs` grows in `257a`; read it once after `257a` lands, then work from ranged edits.
- Do **not** re-derive `257a`'s contour helper — consume it. If it is absent, `257a` has not landed and this packet cannot start.
- Every ledger fact here (key counts, `POLY_SIDE_COUNT`, whether the crate still uses test auto-discovery, the next free packet number) must be re-derived at point of use.
