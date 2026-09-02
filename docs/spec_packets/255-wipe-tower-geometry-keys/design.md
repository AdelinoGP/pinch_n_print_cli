# Design: wipe-tower-geometry-keys

## Selected Approach

The `wipe-tower` core module keeps sole ownership of tower geometry. Seven keys are declared on its manifest, read in `WipeTower::from_config`, and consumed at two sites inside the module:

1. **A new wall generator.** `WipeTower::wall_loop(z, layer_depth, tower_top_z) -> Vec<(f32, f32)>` returns the layer's closed wall ring in tower-local mm coordinates, branching on the parsed `wipe_tower_wall_type`:
   - `rectangle` — the four footprint corners.
   - `cone` — the footprint corners plus a 40-segment arc at each corner of radius `r = tan(cone_angle / 2) × (tower_top_z − z)`, emitted only while `r > 0.5 × layer_depth + 0.01`.
   - `rib` — the footprint ring unioned with two diagonal bars of width `rib_width` (clamped to `min(layer_depth, tower_width) / 2`), each extended `(rib_length − diagonal) / 2` past the corners where `rib_length = max(diagonal, diagonal + extra_rib_length)`, the extension tapered by `|tower_top_z − z| / tower_top_z`; then, when `fillet_wall` is set, every junction whose turn exceeds `30°` is replaced by a tangent arc of radius `min(2.0, ab_len / 2.1, bc_len / 2.1)`.

   `generate_purge_paths` pushes the ring first, as one closed `ExtrusionRole::WipeTower` `ExtrusionPath3D` at the layer's `z`, ahead of the existing travel / scan-line / prime entities.

2. **A rotation transform.** `WipeTower::place(p) -> (f32, f32)` maps a tower-local point through `origin + R(rotation_angle)·(p − origin)` with `origin = (tower_x, tower_y)`. Every point `generate_purge_paths` emits — wall, travel, scan line, prime — goes through it, and `run_finalization`'s bed check validates the **placed wall ring's vertices** instead of the current axis-aligned `tower_width` square. At `0.0` the transform is the identity and output is bit-identical.

3. **An effective purge width.** `effective_width = line_width × (extra_flow / 100)` replaces `line_width` in the scan lines' point `width`, in the scan-line pitch, and in the purge cross-section, while the scan-line points' `flow_factor` becomes `extra_flow / 100`. Fattening the line therefore reduces the number of lines a fixed `prime_volume` needs, exactly as canonical's `x_to_wipe /= m_extra_flow` does. The travel entity keeps `flow_factor = 0.0` and the prime entity keeps its `0.0 → 1.0` pair; neither is a wipe extrusion in canonical.

## Mechanism Check (Authoring rule 4)

- **No claim, no holder, no new module.** Map Authoring rule 4's holder-only rule fires on *cross-module* algorithm selection — alternatives that must live in separate modules and resolve through the claim seam. Tower wall shape is not that: all three shapes are variations of one module's own footprint, computed from the same tower rectangle it already owns, and no other module can produce a wipe-tower wall (the module holds `writes = ["LayerCollectionIR.wipe-tower"]` exclusively). This is the Q8 in-module mode-branching case — the same shape as `seam_position`, `support_style`, `wall_sequence`, `retract_mode` and `wave_overhang_pattern`, all of which the grilling ruled stay in-module. **This is a deliberate departure from the Q3(a) row of `docs/specs/orca-feature-gap/issues/key-correction-inventory.md`, which lists `wipe_tower_wall_type` among the holder-only enums; the departure was ruled by the human at authoring time and requires that row to be amended (see `implementation-plan.md` § Reporting Obligations).** The mechanical argument for the amendment: a holder for tower walls would need a new `ResolvedConfig` field (every existing holder — `sparse_fill_holder`, `top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder` — is declared in the `declare_resolved_config!` block in `crates/slicer-ir/src/resolved_config.rs`, and `module_overrides` exists only as prose in `docs/01_system_architecture.md`, with no implementation), plus a new claim ID and two new guest modules, to select between three variants of one rectangle.
- **No WIT interface, no IR schema bump, no `ResolvedConfig` field.** The wall is an ordinary `ExtrusionPath3D` with the existing `ExtrusionRole::WipeTower`; the seven keys are ordinary module config declarations delivered through `bind_module_config_view` → `ConfigView::from_declared`. Nothing crosses a host boundary that does not already carry module config. **No `[BLOCK]` is open in this packet.**
- **New decision points go where the architecture puts them.** All three sit inside the owning module at `PostPass::LayerFinalization`, not as host special cases and not as hardcoded module constants.

## Tier Derivation

**Tier B.** Authoring rule 1 forces B or C for a packet that builds a decision point. C is "new granular module at a new seam" (ticket 04's rubric) — this packet adds no module, no seam, no claim and no host field; it adds new logic to an existing owner, which is B by definition. The key count (7) is under the B ceiling of 12.

## Code Change Surface (authoritative files-in-scope)

| File | Change |
| --- | --- |
| `modules/core-modules/wipe-tower/wipe-tower.toml` | seven new `[config.schema.*]` tables (AC-1) |
| `modules/core-modules/wipe-tower/Cargo.toml` | add `toml = "0.8"` to `[dev-dependencies]` **only if** `254a` has not already added it |
| `modules/core-modules/wipe-tower/src/lib.rs` | seven new `WipeTower` fields + their `from_config` reads; new `wall_loop`, `rib_polygon`, `round_corners`, `place` helpers; `generate_purge_paths` emits the wall and uses `effective_width` and `place`; `run_finalization` validates placed wall vertices |
| `modules/core-modules/wipe-tower/tests/wipe_tower_wall_tdd.rs` | **new** test binary (AC-2 … AC-7); no `mod` registration needed — the crate's tests are standalone binaries |
| `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` | extend with the seven tables and the six forbidden keys; **author the file** if `254a` has not landed |
| `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` | AC-9 arms; re-express any scan-line-count assertion against the formula |
| `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` | AC-8 arms (rotated vertex outside bed) |
| `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` | AC-N2 arm (enable gate with all seven keys non-default) |
| `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` | AC-10 arms (numeric bounds, enum membership, percent threading) |
| `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` | AC-N1 arm |
| `docs/15_config_keys_reference.md` | regenerated by `cargo xtask gen-config-docs` — never hand-edited |

## Read-Only Context

`crates/slicer-scheduler/src/config_resolution.rs` (`ConfigBoundsIndex::from_modules` / `::check` / `schema_defaults` shapes), `crates/slicer-ir/src/slice_ir.rs` (`ExtrusionPath3D`, `Point3WithWidth` field lists), `modules/core-modules/path-optimization-default/path-optimization-default.toml` (enum table shape), `docs/03_wit_and_manifest.md`, `docs/08_coordinate_system.md`.

## Out of Bounds (must not be loaded or edited)

- `crates/slicer-gcode/src/serialize.rs` — the `ORCA_CONFIG_PADDING` table, including its `wipe_tower_rotation_angle` twin (Authoring rule 2).
- `crates/slicer-ir/src/feedrate.rs` — `wipe_tower_speed` belongs to wayfinder ticket 108 (ruling Q6(a)).
- `crates/slicer-ir/src/resolved_config.rs` and `crates/slicer-gcode/src/emit.rs` as browsing targets — long files; the needed facts are in `requirements.md` § In-Tree Grounding.
- Every other packet directory under `docs/spec_packets/`, including `254a` and `254b` (reconcile through their `packet.spec.md` only, via a SUMMARY dispatch).
- `OrcaSlicerDocumented/` — delegated reads only.

## Expected Dispatches

| Question | Scope | Return |
| --- | --- | --- |
| Exact per-layer pitch and depth expressions in `generate_purge_paths` as they stand now | `modules/core-modules/wipe-tower/src/lib.rs` | `SNIPPETS` ≤ 1 × 30 lines |
| Has `254a` landed, and does the manifest already carry `wipe_tower_config_schema_tdd.rs` / `toml` dev-dep? | `docs/spec_packets/254a-prime-tower-geometry-keys/packet.spec.md`, `modules/core-modules/wipe-tower/**` | `FACT` ≤ 5 lines |
| Canonical arc/rounding parameters if any formula above needs re-checking | sibling `OrcaSlicerDocumented` | `SUMMARY` ≤ 200 words |
| Each verification command | workspace | `FACT` pass/fail |

## Divergences (recorded, with rationale)

- **DIV-1 — no rib-mode square-tower re-planning.** Canonical's rib mode forces a square tower (`plan_tower_new`: `width = align_ceil(sqrt(max_depth × width), perimeter_width)`) and re-plans every toolchange through `set_toolchange`. This port derives tower depth per layer from purge volume (`254a`'s depth model); forcing a square would fight that model and silently move the user's `prime_tower_width`. The port keeps the user's width and derives `rib_length` from the *actual* rect diagonal instead. Consequence accepted: on a very shallow tower the arms are shorter than canonical's.
- **DIV-2 — cone/rib taper reference height.** Canonical uses `m_wipe_tower_height`, known up front from tower planning. The port has no planning stage, so `tower_top_z` is the maximum `z` over the layers that receive tower entities, computed once in `run_finalization` before the emit loop. Same geometry, derived rather than declared.
- **DIV-3 — rotation applied at generation, not at G-code integration.** Canonical rotates in `WipeTowerIntegration::transform_wt_pt` because the tower class emits in tower-local coordinates and a separate integration stage places it. This port has no integration stage, and the module owns its own world-space points, so the same `R(θ)` about the tower origin is applied at emit. Identical result, one fewer host special case — the "better seam" case Authoring rule 4 asks packets to take.
- **DIV-4 — `extra_flow` as one effective width.** Canonical multiplies extrusion flow and analyzer line width, and divides `x_to_wipe`, in three places. The port folds all three into one `effective_width` feeding point width, pitch and cross-section, with `flow_factor` carrying the multiplier to the emitter's E computation. Same invariant (purge volume preserved), expressed once.
- **DIV-5 — fillet is rib-only.** Canonical calls `rounding_polygon` in the rib branch and unions the result back with the plain box, so convex footprint corners are never eaten; the rectangle and cone branches never round. The port reproduces the *effect* directly — rounding is applied to the rib junctions only — rather than materialising a union. Asserted both ways in AC-2 and AC-6.

## Invariants

- At `wipe_tower_rotation_angle = 0.0` and `wipe_tower_extra_flow = "100%"`, every non-wall point is bit-identical to the pre-packet output. The wall entity is new at defaults by design (canonical's default wall type is `rib`), and that is a behaviour change this packet owns.
- The wall ring is always closed (first vertex repeated) and always precedes the layer's scan lines in emission order.
- Purge volume is invariant under `wipe_tower_extra_flow`: `scan_line_count × effective_width` is constant.
- The bed check never passes a tower whose emitted geometry leaves the bed: it validates the exact placed wall vertices, a superset of the old four corners for every wall type.
- No key in `requirements.md` § Returned to Queue appears in the manifest — asserted by AC-N3, not left to review.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

  *Packet-specific note:* this module is an exception in the other direction — it works entirely in plain mm `f32` (`tower_x`, `line_width`, `Point3WithWidth`). Canonical's `scaled(m_rib_width)` and `rounding = scale_(2.)` arithmetic must therefore be **de-scaled to mm**, not transcribed. Any scaled literal copied from `WipeTower2.cpp` is a bug.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Config keys are **snake_case** in every Rust and TOML string (`CLAUDE.md` § Config Key Naming Convention).
- Module config is boundary-enforced: a module sees only its declared keys, and an undeclared read returns `None` silently (`docs/03_wit_and_manifest.md` § Host-Boundary Access Enforcement). Declaration must precede the read, hence the step order.

## Risks

- **Canonical-revision risk.** The delegated reader flagged that the sibling checkout is a documented / modified OrcaSlicer fork whose `WipeTower2` rib support carries locally added parity work. The rib formulas here follow that checkout — which is this effort's canonical source per ticket 02 — but a reviewer comparing against stock upstream may see a difference. Recorded here so it is not misdiagnosed as a port defect.
- **Merge churn with `254a` / `254b`.** All three packets edit the same manifest and the same `generate_purge_paths`. Landing order is `254a` → `254b` → `255`; the formulas here are written relative to whatever pitch and depth the function computes at implementation time, not to a frozen expression.
- **Default-path output changes.** `wipe_tower_wall_type` defaults to `rib`, so a default slice gains a rib wall it did not have. Any test that pins the wipe tower's entity count is expected to move; re-express such assertions against the formula rather than re-fitting a captured number (`CLAUDE.md` § Test Discipline).
- **Polygon union.** The rib shape is a union of a rectangle and two rotated bars. The module has no polygon-boolean dependency today; the shape is convex-per-piece and the union ring can be built analytically from the eight arm corners plus the four footprint corners in angular order. If the implementer reaches for a boolean library instead, that is a scope change and must be raised before Step 2 proceeds.

## Context Cost

`L` in aggregate (four M steps plus one S). No single step is rated L; if Step 2 grows past M in practice, split the rib shape from the cone shape at the step boundary rather than escalating the band.

## Open Questions

- **`[FWD]` — `254a` / `254b` landing order.** Both are `draft`. This packet composes with their pitch, depth and bed-check changes but does not require them: if it lands first, the formulas resolve against today's `line_width` pitch and `purge_volume / cross_section` depth, and `254a` must then re-compose. Prefer the stated order.
- **`[FWD]` — `wipe_tower_extra_spacing` after `254b`.** Once `254b`'s ramming pass exists, that key has a decision point (the ramming `y_step`). It is returned to the queue here rather than forward-stacked; a follow-up packet should pick it up together with the rest of `254b`'s fallout.
- No `[BLOCK]`.
