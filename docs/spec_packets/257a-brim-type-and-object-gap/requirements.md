# Requirements: brim-type-and-object-gap

## Packet Metadata

- **Packet directory:** `docs/spec_packets/257a-brim-type-and-object-gap/`
- **Slug:** `brim-type-and-object-gap`
- **Status:** `draft`
- **Task IDs:** none (queue packet — `task_ids: []`, precedent packets 234a, 253–265)
- **Backlog source:** wayfinder ticket 12 (`docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md`), map `docs/specs/orca-feature-gap/map.md` packet P05
- **Tier:** **B** — re-derived. The prior revision was Tier A on the "declare + wire the one cheap gate" reading. Under map Authoring rule 1 a packet that *builds* a decision point is B or C; this packet builds contour brim and a per-object mode dispatch as new logic inside `skirt-brim`, which already owns the stage, and adds no module and no seam. See `design.md` § Tier Derivation.
- **Re-authoring note:** the prior `257-brim-type-and-brim-keys` directory is **split** into `257a` (this packet) and `257b-brim-ears` with explicit user approval, under the 210a/210b letter-suffix precedent and map Authoring rules 1–7. The prior draft's `preflight-report.md` described a packet that no longer exists and was removed with the split.

## Problem Statement

The prior packet 257 declared all five of ticket 12's brim keys and dispositioned four of them "declared-with-gap", claiming `brim_type`'s `no_brim` arm as the one live wire. Two things are wrong with that, re-derived from disk at authoring time.

First, the disposition is prohibited outright by Authoring rule 1, and rule 5 removes the plumbing exemption it leaned on. Second — and this is the part the prior draft did not surface — **`brim_type` is not wired at all.** It appears in `crates/slicer-model-io/src/loader.rs`'s sidecar key classification, in `ORCA_CONFIG_PADDING`, and in one 3MF test fixture. It is read nowhere in `modules/core-modules/skirt-brim/`. Of ticket 12's five keys, **zero** are live; the only live brim key in the module is `brim_width`, which is not a ticket-12 key.

The reason none of them can be wired as-is is structural. Every canonical brim mode is defined over **object contours**: `Brim.cpp::outer_inner_brim_area` derives `has_outer_brim` / `has_inner_brim` and then offsets `ex_poly.contour` outward or the reversed holes inward. This tree's brim is one rectangle: `SkirtBrim::generate_brim_entities` computes `num_loops` from `brim_width / line_width` and calls `make_rect_loop` on the **global** bounding box of every entity across all skirt-height layers — one shape for the whole plate, with no object identity and no holes. Over a rectangle, `outer_only` is indistinguishable from `auto_brim`, `inner_only` has nothing to offset into, `brim_object_gap` has no contour to stand off from, and per-object dispatch is unrepresentable. Declaring the keys against that generator is exactly the "add a consumer that does nothing" rule 5 forbids.

This packet therefore builds the missing feature — per-object contour brim — and makes two keys drive it.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue (no decision point, not built here); **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point this packet builds | Non-default AC |
| --- | --- | --- | --- | --- |
| `brim_type` | **(b)** | `skirt-brim` | per-object mode dispatch over contour brim: `no_brim` emits nothing, `outer_only` offsets the contour outward, `inner_only` offsets the reversed holes inward, `outer_and_inner` does both, `auto_brim` behaves as `outer_only` | AC-1, AC-2, AC-3, AC-4, AC-6 |
| `brim_object_gap` | **(b)** | `skirt-brim` | the stand-off distance between the object contour and the innermost brim loop, applied outward on the contour and inward on the holes | AC-5 |

Counts: **(a) 0 · (b) 2 · (c) 1 · (d) 0**, with 2 keys deferred to `257b-brim-ears` — five of ticket 12's keys accounted for. Zero declaration-only keys (map preflight gate (a)); both in-packet keys carry ACs asserting behaviour changes at non-default values (map preflight gate (b)).

## Returned to Queue — unimplemented, needs elephant-foot compensation

### `brim_use_efc_outline`

`coBool`, canonical default `false`. Its sole canonical predicate is `Brim.cpp::use_brim_efc_outline`, which requires **all** of: the flag true, `elefant_foot_compensation > 0`, `elefant_foot_compensation_layers > 0`, and `raft_layers == 0`. When it holds, `outer_inner_brim_area` builds the brim base from `get_print_object_bottom_layer_expolygons` — the post-EFC first-layer footprint — instead of the pre-compensation `lslices`, and `make_brim_ears` projects painted ear points onto the same EFC outline.

Re-derived from disk at authoring time: **`elefant_foot_compensation` occurs exactly once in this repository outside documentation — as a literal in `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs`.** There is no elephant-foot-compensation geometry, no `elefant_foot_compensation_layers` key, and no `ResolvedConfig` field. The key's gate therefore can never be true, and there is no second outline for it to select. The missing feature is **elephant-foot compensation of the first-layer footprint**, which has no packet and no queue row today; the closing agent must report it (see § Map and Ticket Updates Required in `design.md`). AC-N3 asserts the key is not declared, so a future worker cannot re-add it as a stub.

## Deferred to `257b-brim-ears` — not returned, not declared here

`brim_ears_max_angle` (`coFloat`, default 125, `[0, 180]`) and `brim_ears_detection_length` (`coFloat`, default 1, min 0) are live in canonical (`Brim.cpp::make_brim_ears_auto`) and are genuinely buildable **once a per-object contour exists** — which is precisely what this packet creates. They are carried by `257b-brim-ears`, which depends on this packet. They are not declared here (AC-N3), and `brim_type`'s `brim_ears` value is not shipped by this packet: under Authoring rule 1 an unshipped enum value is unimplemented, not declared.

## Unshipped `brim_type` value — `painted`

Canonical's `btPainted` dispatches `Brim.cpp::make_brim_ears`, which consumes **user-painted points on the model**. The port's paint carrier is `slicer_ir::PaintSemantic`, whose variants are `Material`, `FuzzySkin`, `SupportEnforcer`, `SupportBlocker` and `Custom(String)`, and it attaches semantics to **surface regions**, not to points. There is no brim paint semantic and no point-valued paint carrier. `painted` is therefore an unshipped value of a shipped key — the same disposition Authoring rule 4 assigns to unshipped values of a claim-holder enum. AC-N1 asserts it is rejected by name rather than silently treated as `auto_brim`; the missing feature (a point-valued brim paint carrier) is reported to the map.

## Ruled Dead-in-Canonical

**None.** All five of ticket 12's keys have read sites inside OrcaSlicer's slicing pipeline under `src/libslic3r/`, verified per key at authoring time by a delegated sweep that excluded `src/slic3r/GUI/**` (including `ConfigManipulation.cpp`), `PrintConfig.cpp` tooltip and label text, `Preset.cpp` key lists, and `IGNORE`/legacy-alias sets. Authoring rule 3 rules none of them out of scope.

Two caveats, both of them exactly the trap rule 3 warns about:

- **All five keys also appear in `PrintObject::invalidate_state_by_config_options`**, which is a key-*name* list. It is excluded from the evidence below and must not be cited as a read site — with one genuine exception noted there for `brim_type`.
- **The ticket-04/12 `brim_ears` precedent is narrower than it reads, and the map says so explicitly.** Ticket 12 ruled the `brim_ears` *bool* dead, and it still is. The ears *feature* is live, reached through `brim_type == btBrimEars` rather than the retired bool, and `brim_ears_max_angle` / `brim_ears_detection_length` are live with it. This packet does not extend the dead-bool ruling to the ear keys; `257b` builds them.

## Per-Key Canonical Evidence

Cited by file and function, never by line number (repo rule).

| Key | Canonical read sites under `libslic3r/` |
| --- | --- |
| `brim_type` | `Brim.cpp` `outer_inner_brim_area` (sets `has_outer_brim` / `has_inner_brim` / the ear-mode flags); `Print.hpp` `PrintObject::has_brim` (gates whether brim generation runs) and `Print::has_auto_brim`; `PerimeterGenerator.cpp` `_traverse_loops` (forces outer-wall-first on layer 0 when `btOuterOnly` and `brim_width > 0`); `Support/SupportCommon.cpp` `generate_support_toolpaths` (expands support-avoid areas); `Support/SupportSpotsGenerator.cpp` `estimate_supports_malformations`; `PrintObject.cpp` `estimate_curled_extrusions`. It is additionally the one genuine *value* read in `invalidate_state_by_config_options`, comparing old against new for the `btOuterOnly` perimeter-order invalidation |
| `brim_object_gap` | `Brim.cpp` `outer_inner_brim_area` (scaled into `brim_offset`; offsets the contour outward for the outer band, shrinks the reversed holes by `brim_offset` and by `brim_width + brim_offset`, and inflates the island appended to `no_brim_area_object`); `Support/SupportCommon.cpp` `generate_support_toolpaths` (offsets first-layer slices to keep support out of the brim gap) |
| `brim_ears_max_angle` | `Brim.cpp` `make_brim_ears_auto` (converted to `angle_threshold`) and `outer_inner_brim_area` (forwards it) — carried by `257b` |
| `brim_ears_detection_length` | `Brim.cpp` `make_brim_ears_auto` (Douglas-Peucker tolerance) and `outer_inner_brim_area` (scales and forwards it) — carried by `257b` |
| `brim_use_efc_outline` | `Brim.cpp` `use_brim_efc_outline` (the sole predicate) and `outer_inner_brim_area` / `make_brim_ears` (consume it to switch the base outline) — returned to the queue |

### Canonical semantics this packet borrows exactly

- **Mode → band derivation.** `has_outer_brim` is true for `btOuterOnly`, `btOuterAndInner`, `btAutoBrim`, `btEar` and `btPainted`. `has_inner_brim` is true for `btInnerOnly` and `btOuterAndInner` (and for the ear modes when `brim_ears_outer_only` is false, which is `257b`'s concern). `btOuterAndInner` is the only non-ear mode producing both. `btNoBrim` produces neither and instead contributes to `no_brim_area_object`.
- **`BrimType` value order and default.** `PrintConfig.hpp` declares `btAutoBrim, btEar, btPainted, btOuterOnly, btInnerOnly, btOuterAndInner, btNoBrim`, serialized as `auto_brim, brim_ears, painted, outer_only, inner_only, outer_and_inner, no_brim`, default `btAutoBrim`. The manifest's `values` list preserves that order.
- **Gap application.** `brim_object_gap` offsets the contour outward to form the inner boundary of the outer band, and shrinks the reversed holes by `brim_offset` for the outer edge and `brim_width + brim_offset` for the inner edge.

### Canonical behaviour this packet deliberately does not borrow

- `brim_type`'s couplings **outside** brim generation: the `btOuterOnly` outer-wall-first ordering in `PerimeterGenerator::_traverse_loops`, the support-avoid expansion in `generate_support_toolpaths`, and the curling estimate in `SupportSpotsGenerator`. Each is a separate feature in a different module, none is in ticket 12's scope, and porting them here would silently widen the packet. Recorded as **DIV-3**.

## In Scope

1. **Per-object layer-0 contour derivation** inside `skirt-brim`: group the first layer's `ordered_entities()` by `PrintEntity.region_key.object_id`, keep those whose `role` is `ExtrusionRole::OuterWall`, treat each entity's `path.points` as a closed loop, and union them through `slicer_sdk::host::clip_polygons(loops, &[], ClipOperation::Union)` so contours and holes resolve. Objects contributing no outer-wall loop yield no brim and are logged, not silently dropped.
2. **`brim_type` manifest declaration and read** — `enum`, seven values in the canonical declared order, default `auto_brim`; `from_config` rejects unknown values by name (AC-N1). The value is resolved **per object** through the module's per-object config view, with the global value as fallback (AC-4).
3. **`brim_object_gap` manifest declaration and read** — float, default `0.0`, `[0.0, 2.0]` canonical bounds.
4. **Band generation replacing `generate_brim_entities`' rectangle:** for the outer band, offset the object contour outward by `brim_object_gap` to get the inner boundary, then emit successive outward loops one `line_width` apart until `brim_width` is consumed. For the inner band, offset each hole inward by `brim_object_gap` and emit successive inward loops the same way. `make_rect_loop` is retained for the skirt and is no longer used by the brim path.
5. **Region keying**: each emitted brim loop keeps `ExtrusionRole::Brim` but carries the owning object's `object_id` in its `RegionKey` instead of the current literal `"brim"`, so AC-4 can assert per-object emission. The role remains the single source of truth for G-code labelling (the packet that dropped the `__brim__` marker established this).
6. **Tests**: two net-new test files in `modules/core-modules/skirt-brim/tests/` (the crate uses Cargo test auto-discovery, so no `[[test]]` entry is needed — verified at authoring time), plus a bounds arm in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`.
7. **Docs**: regenerate `docs/15_config_keys_reference.md`.

## Out of Scope

- **`ORCA_CONFIG_PADDING` and every CONFIG_BLOCK twin.** Authoring rule 2: padding is not parity, is never a deliverable, and is not evidence. AC-N4 asserts the file is untouched.
- **Ear geometry** — `257b-brim-ears`.
- **Elephant-foot compensation** and therefore `brim_use_efc_outline` — returned to the queue, no owner yet.
- **A point-valued brim paint carrier** and therefore `brim_type = painted`.
- **The skirt path.** `generate_skirt_entities` keeps the global bounding box; a skirt is a plate-level loop by definition. AC-N5 guards it.
- **`brim_type`'s non-brim couplings** — see DIV-3.
- **`brim_width`.** Live already and not a ticket-12 key.

## Authoritative Docs

- `docs/00_project_overview.md` — project goals the design must satisfy.
- `docs/03_wit_and_manifest.md` — manifest `[config.schema]` enum declaration shape.
- `docs/08_coordinate_system.md` — every offset here is millimetres at the module boundary; the 1 unit = 100 nm hazard applies through the host geometry ops.
- `docs/21_data_defaults_and_fixtures.md` — the struct-literal churn gate governs the new `SkirtBrim` fields.
- `docs/15_config_keys_reference.md` (generated) — regenerate at close.
- `docs/ORCASLICER_ATTRIBUTION.md` — `skirt-brim/src/lib.rs` already carries the porting header; keep it.

## Parity Evidence Standard

Under Authoring rule 5, "default matches and the value reaches the consumer" is **not** sufficient evidence for either key. Each key's evidence is a behaviour difference measured between two runs differing only in that key's value, with the non-default value named in the AC. AC-N5 exists solely as a regression guard on the skirt path and is not evidence for any key.

## Acceptance Summary

| AC | Key | Class | Asserts |
| --- | --- | --- | --- |
| AC-1 | `brim_type` | b | `no_brim` emits zero `Brim` entities, skirt count unchanged |
| AC-2 | `brim_type` | b | `inner_only` emits only inside holes; `outer_only` only outside the contour |
| AC-3 | `brim_type` | b | `outer_and_inner` is the union of both |
| AC-4 | `brim_type` | b | resolved per object, not globally |
| AC-5 | `brim_object_gap` | b | innermost loop stands off `1.0` mm further at `1.0` than at `0.0` |
| AC-6 | `brim_type` | b | brim follows a concave contour, which the bbox rectangle could not |
| AC-N1 | `brim_type` | — | unknown value rejected by name, not defaulted |
| AC-N2 | `brim_object_gap` | — | out-of-bounds value rejected, not clamped |
| AC-N3 | returned/deferred | — | three keys not declared in the manifest |
| AC-N4 | padding | — | `serialize.rs` untouched |
| AC-N5 | skirt | — | regression guard only |

## Verification Matrix

| Surface | Command |
| --- | --- |
| `brim_type` modes (AC-1, AC-2, AC-3, AC-4, AC-6, AC-N1) | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd 2>&1 \| tee target/test-output.log` |
| `brim_object_gap` (AC-5) | `mkdir -p target && cargo test -p skirt-brim --test brim_object_gap_tdd 2>&1 \| tee target/test-output.log` |
| skirt regression (AC-N5) | `mkdir -p target && cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 \| tee target/test-output.log` |
| module binding | `mkdir -p target && cargo test -p skirt-brim --test slicer_module_binding_tdd 2>&1 \| tee target/test-output.log` |
| live finalization path | `mkdir -p target && cargo test -p skirt-brim --test finalization_live_tdd 2>&1 \| tee target/test-output.log` |
| scheduler bounds (AC-N2) | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 \| tee target/test-output.log` |
| guest freshness | `cargo xtask build-guests --check` (inspect the exit code; never grep for `STALE:`) |
| type gate | `cargo check --workspace --all-targets` |
| lint gate | `cargo clippy --workspace --all-targets -- -D warnings` |
| literal gate | `cargo xtask check-literals` |

## Step Completion Expectations

- The manifest declaration and the module read for a given key must land in the same step, or the manifest declares a key the module does not read — the exact disposition rule 1 prohibits.
- The contour-derivation step must land before either key's behaviour step; both keys are meaningless over the bounding box.
- No step may edit `crates/slicer-gcode/src/serialize.rs`.
- `skirt-brim/src/**` feeds the guest build, so any step whose verification is a module test must run `cargo xtask build-guests --check` first and judge by its exit code.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `outer_inner_brim_area`, for the `has_outer_brim` / `has_inner_brim` derivation per `BrimType` value and for how `brim_object_gap` becomes `brim_offset` on the contour and on the reversed holes.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `PrintConfig.hpp` — the `BrimType` enum's declared value order and default, for the manifest's `values` list.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintObject::has_brim`, the gate this packet's `no_brim` arm reproduces.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths`'s first-layer brim-avoidance block, cited as deliberately **not** ported (DIV-3).

## Context Discipline Notes

- `modules/core-modules/skirt-brim/src/lib.rs` is short enough to read in full once; do it once, then work from ranged edits.
- Do **not** read `crates/slicer-sdk/src/traits.rs` in full. `LayerCollectionView` wraps a `LayerCollectionIR` and exposes `layer_index()`, `z()` and `ordered_entities()`; range-read only if more is needed.
- Every ledger fact here (key counts, the next free packet number, the `docs/07` inventory) must be re-derived at point of use.
