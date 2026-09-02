# Design: brim-ears

## Selected Approach

Ears are a **pass over a contour**, not a new kind of brim. Canonical `make_brim_ears_auto` takes an `ExPolygon`, optionally simplifies it, tests each remaining vertex's turn angle against a threshold, and emits one small regular polygon per surviving vertex. Every input it needs exists in `skirt-brim` once packet `257a` lands: the per-object layer-0 contour, `brim_width`, `line_width`, and `brim_object_gap`.

The port therefore adds two private helpers and one mode arm:

1. **`decimate_contour(points, tolerance_mm)`** — Douglas-Peucker, with canonical's two guards. Tolerance `0` returns the input untouched (canonical's documented disable), and a result below four points is discarded in favour of the input. Both guards are asserted (AC-4, AC-N1) rather than left implicit, because both are silent-degradation paths: a contour decimated to two points produces no corners at all, which would read as "this shape has no ears" instead of "the tolerance was too large".
2. **`detect_ear_anchors(contour, angle_threshold_rad, convex)`** — the turn-angle test. Convex selection for the outer band, concave for the inner, mirroring canonical's `Polygon::convex_points` / `concave_points` split.
3. **The `brim_ears` arm** of the mode dispatch `257a` builds. `257a` declares all seven `BrimType` values and rejects `brim_ears` by name with a message pointing at this packet; this packet turns that rejection into a generator. `painted` stays rejected (AC-N3).

**Ear geometry is an annulus.** Canonical emits a disc of `POLY_SIDE_COUNT` sides at radius `size_ear` and the caller subtracts the gap-offset object island, leaving a ring around the corner. The port does the same with `slicer_sdk::host::clip_polygons(ears, &island, ClipOperation::Difference)`, where `island` is the contour offset outward by `brim_object_gap` — the same island `257a` already computes for its outer band. AC-5 pins it: an ear that overlapped the object would be extruded on top of the part.

**`size_ear` couples to `257a`.** Canonical computes it as `brim_width_mod - brim_offset - flow.scaled_spacing()`, i.e. the band width minus the object gap minus one extrusion spacing. In this tree that is `brim_width - brim_object_gap - line_width`. Ears are therefore *not* independent of `257a`'s keys, and a change to `brim_object_gap` moves both the band and the ear radius. This is canonical behaviour, reproduced deliberately, and named here so it is not later read as a leak.

## Rule 4 Trigger Test

Authoring rule 4 fires on **cross-module** algorithm selection, where alternatives are separate implementations resolved through the claim seam. Its trigger test explicitly excludes a module branching internally over a mode it implements itself.

`brim_ears` is a mode of `brim_type`, and `brim_type` was already adjudicated in `257a`'s `design.md` as internal branching: canonical expresses the whole enum as two booleans over one code path in `outer_inner_brim_area`, and ears are dispatched from *inside* that same function rather than from a filler-selection seam. Ears are a distinct algorithm, but they are a distinct *pass in the same module*, sharing that module's contour, band width and gap. Splitting them into a claim holder would require exporting `257a`'s contour derivation across a module boundary to serve one value of one key. Rule 4 does not fire. The `257a` / `257b` boundary is a packet-size boundary, not a claim boundary — and this file says so explicitly so a later reviewer does not mistake the split for a claim decision.

## Claims Held

- No new claim. `skirt-brim` keeps the claims it holds today.
- Emitted ear entities carry `ExtrusionRole::Brim` and the owning object's `object_id` in their `RegionKey`, exactly as `257a`'s band entities do.

## Which Existing Mechanism Carries the New Data

| New behaviour | Carrier | Why not something else |
| --- | --- | --- |
| Per-object contour to detect corners on | `257a`'s contour derivation, consumed unchanged | Re-deriving it here would duplicate `257a`'s logic and let the two drift |
| `brim_ears_max_angle`, `brim_ears_detection_length` reaching the module | `skirt-brim.toml` `[config.schema]` + `ConfigView::get`, the path `brim_width` and `257a`'s two keys already take | Host-side special-casing would violate rule 4's "new decision points go where the architecture puts them" |
| Annulus construction | `slicer_sdk::host::clip_polygons` with `ClipOperation::Difference`, already exported to guests | No new host capability is needed |

No WIT type, no IR field, no `ResolvedConfig` field, no `SliceRegionView` metadata, no new `PostPass` claim.

## Recorded Divergences

- **DIV-1 — the decimation guards are asserted, not implicit.** Canonical skips decimation below four points as an inline condition. The port makes both guards (`tolerance == 0`, `result < 4 points`) named, separately tested behaviours (AC-4, AC-N1). Same semantics; the difference is that a future edit cannot silently drop a guard, because a test names it.
- **DIV-2 — ears inherit `257a`'s deposited-contour divergence.** `257a` derives the contour from layer-0 `OuterWall` extrusion loops rather than slice polygons and compensates by half a line width (its DIV-1). Ear anchors are vertices of that compensated contour, so ears sit on corners of the *deposited* outline. For a corner this is arguably more correct than canonical's, which anchors on a boundary the printer never touches — but it is a divergence, it is inherited rather than introduced here, and it must be re-read from `257a`'s `design.md` rather than re-litigated.
- **DIV-3 — `brim_ears_outer_only` is fixed at canonical's default.** Canonical lets ear modes also produce an inner brim when this flag is false. It is not a ticket-12 key, so this packet does not declare it and fixes the behaviour at the canonical default. Recorded rather than silently assumed, and reported to the map as a gap.

## Tier Derivation

Ticket 04's rubric: **Tier A** is plumbing into an existing decision point; **Tier B** is new logic in an existing owner; **Tier C** is a new module at a new seam.

Neither key has a decision point today — both have zero occurrences in the tree — so Tier A is out. The work is two helpers and one mode arm inside `skirt-brim`, which already owns `PostPass::LayerFinalization` and, after `257a`, already generates contour brim. No module, no seam, no claim. **Tier B.**

## Code Change Surface

**Editable:**

- `modules/core-modules/skirt-brim/src/lib.rs` — two new `SkirtBrim` fields and their `from_config` reads; the `decimate_contour` and `detect_ear_anchors` helpers; the ear emitter; the `brim_ears` arm of the mode dispatch.
- `modules/core-modules/skirt-brim/skirt-brim.toml` — two net-new `[config.schema]` tables.
- `modules/core-modules/skirt-brim/tests/brim_ears_tdd.rs` — net-new (AC-1 … AC-5, AC-N1, AC-N3).
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — one bounds arm (AC-N2).
- `docs/15_config_keys_reference.md` — regenerated.

`modules/core-modules/skirt-brim/Cargo.toml` used Cargo test auto-discovery at authoring time, so the net-new test file needs no manifest entry. **Re-derive this before relying on it** — `257a` lands between this packet's authoring and its implementation and may have added `[[test]]` entries. If it has, the file needs one and Step 3's edit list grows.

**Read-only context:** `257a`'s contour helper and band generator in `modules/core-modules/skirt-brim/src/lib.rs` (ranged); `crates/slicer-sdk/src/host.rs` for the `clip_polygons` signature and `ClipOperation` variants (ranged); `docs/spec_packets/257a-brim-type-and-object-gap/design.md` § Recorded Divergences, for DIV-2's inherited compensation — read that section only, not the whole packet.

**Out of bounds — must not be loaded or edited:**

- `crates/slicer-gcode/src/serialize.rs` (AC-N4 asserts it is untouched).
- `crates/slicer-schema/wit/**` — no WIT change is in scope. Touching it means the packet was mis-scoped: stop and report.
- `257a`'s contour derivation, its four shipped mode arms, `brim_object_gap`'s application, and `generate_skirt_entities`. Consume, do not modify (AC-N5).
- `modules/core-modules/` other than `skirt-brim/`.
- `docs/specs/orca-feature-gap/map.md` and `docs/specs/orca-feature-gap/issues/**`.

## Blast-Radius Discipline

`SkirtBrim` gains two more fields on top of `257a`'s two. Re-derive its field count at implementation time to determine whether it is a watched type under the struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`) — `257a` may already have pushed it over the five-field threshold. If it is watched, the step adding the fields owns every literal site in the crate's tests, and its exit condition is `cargo check --workspace --all-targets` **plus** `cargo xtask check-literals` in the same step, not a later step's discovery.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **The decimation tolerance and the ear radius are millimetres at this seam.** `slicer_sdk::host` offsets take millimetres; canonical's `scale_()` wrappers are deliberately not ported. Canonical's `ear_detection_length` is scaled before use, so the port's value is the unscaled millimetre figure — do not double-convert.
- **`angle_threshold` is radians.** Canonical converts `(180 - max_angle) * PI / 180`. The manifest key is degrees; the conversion happens once, in `from_config`, and the helper takes radians. Naming the helper's parameter `angle_threshold_rad` is required, not stylistic — an unmarked angle unit is the classic source of a silently wrong corner set.
- **`POLY_SIDE_COUNT` and `size_ear`'s spacing term must be fetched from canonical**, not guessed. `size_ear` is `brim_width - brim_object_gap - line_width` in this tree's terms; confirm the spacing term maps onto `line_width` before relying on it.
- **The AGPLv3 porting header at the top of `skirt-brim/src/lib.rs` must be retained**; it already names `src/libslic3r/Brim.cpp`, which is the file this packet ports further from (`docs/ORCASLICER_ATTRIBUTION.md`).

## Expected Dispatches

| Question | Scope | Return format |
| --- | --- | --- |
| The full `make_brim_ears_auto` algorithm: decimation guards, angle conversion, convex/concave split, per-ear polygon | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` | `SUMMARY` ≤ 200 words |
| `POLY_SIDE_COUNT`'s value and `size_ear`'s exact expression | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` | `FACT` ≤ 5 lines |
| The angle convention `convex_points` / `concave_points` compare against | `OrcaSlicerDocumented/src/libslic3r/Polygon.cpp` | `SUMMARY` ≤ 200 words |
| `_douglas_peucker`'s tolerance semantics | `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp` | `SUMMARY` ≤ 200 words |
| Re-derive `SkirtBrim`'s field count and whether the crate still uses test auto-discovery | `modules/core-modules/skirt-brim/**` | `LOCATIONS` ≤ 20 entries |
| Run each verification command | `requirements.md` § Verification Matrix | `FACT` pass/fail |

## Invariants

- No ear overlaps the object: every ear is the polygon minus the gap-offset island (AC-5).
- `brim_ears_detection_length = 0` means no decimation, never "decimate with tolerance zero" (AC-4).
- Decimation never yields a contour below four points; it degrades to the input instead (AC-N1).
- The ear keys are inert outside `brim_type = brim_ears`; `257a`'s four modes produce byte-identical output at any ear-key value (AC-N5).
- `painted` remains rejected by name (AC-N3). Shipping one ear mode must not quietly enable the other.

## Risks

- **The angle convention is the likeliest silent defect.** Canonical's threshold is a *turn* angle derived as `180 - max_angle`, so a larger `brim_ears_max_angle` means *more* corners qualify, not fewer. Getting the sense backwards produces a plausible-looking ear set that is exactly wrong. AC-2 is built to catch it: it names two specific corner angles and asserts which one survives at which setting.
- **Inherited contour divergence.** Ears anchor on `257a`'s deposited-outline contour (DIV-2). If `257a`'s half-line-width compensation is wrong, ears are displaced by that amount and nothing in this packet would show it. The dependency is stated so a reviewer checks `257a` first.
- **Coupling through `size_ear`.** A change to `brim_object_gap` moves the ear radius as well as the band. That is canonical behaviour, but it means AC-5 and `257a`'s AC-5 constrain each other; run both after either changes.

## Open Questions

- `[FWD]` Whether `257a` resolves `brim_type` per object or globally — its `design.md` carries that as its own `[FWD]`. If it degraded to global, ear modes are also global and this packet's ACs are unaffected in substance; only the fixture setup changes. Recorded so the implementer checks `257a`'s resolution before writing fixtures rather than discovering it mid-step.

**No `[BLOCK]` is open.** The packet needs no new WIT interface, no IR schema bump, and no new host `ResolvedConfig` field. Its one hard dependency is `257a`, which is a packet-ordering constraint and is declared as a FORWARD-DEP in `packet.spec.md` § Prerequisites, not as a satisfied prerequisite — `257a` is `status: draft` at authoring time.

## Map and Ticket Updates Required

Listed only; **not applied by this packet** (the map and tickets are out of bounds). Items 1–3 duplicate `257a`'s list deliberately, so whichever packet closes first carries them.

1. **Tier correction.** The map's P05 entry and ticket 04's tier table carry the packet as Tier A. `257a` and `257b` are both **Tier B**.
2. **Packet split.** P05's packet row must point at `257a-brim-type-and-object-gap` and `257b-brim-ears` instead of `257-brim-type-and-brim-keys`.
3. **Coverage-count correction.** `257a` covers 2 keys, `257b` covers 2. `brim_use_efc_outline` leaves the P05 count entirely.
4. **A gap with no owner: `brim_ears_outer_only`** (DIV-3). Canonical lets ear modes also produce an inner brim; the port fixes the flag at its default. Not a ticket-12 key, so it needs a queue row.
5. **Reinforce the `brim_ears` precedent note in ticket 12.** The map already records that ticket 12 ruled the `brim_ears` *bool* dead while the ears *feature* is live through `brim_type == btBrimEars`. This packet builds the feature; the ticket must not be read as having ruled it out.
