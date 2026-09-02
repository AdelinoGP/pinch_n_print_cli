# Design: wipe-tower-bed-exclude-area

## Selected Approach

Pre-slice print validation becomes a **module**, not a host branch.

`modules/core-modules/print-validator/` declares `[stage] id = "PrePass::MeshAnalysis"` — the earliest module-hostable stage, with no required prepass slots — and implements `PrepassModule::run_mesh_analysis`. It reads `bed_exclude_area` and `printable_area` from its `ConfigView`, and for each object id it is handed:

1. **Cheap reject.** `slicer_sdk::host::object_bounds(object_id) -> Result<BoundingBox3, HostUnavailable>` gives the object's bounds; project to XY. If that rectangle does not overlap the exclusion polygon's bounding rectangle, the object cannot collide — skip it, zero raycasts. A `HostUnavailable` return is a host-services failure, not a collision: propagate it as a fatal module error naming the service, never as a silent pass.
2. **Probe.** Otherwise walk a `1.0` mm grid over the intersection of the two rectangles. Keep only grid points **strictly inside** the exclusion polygon (even-odd ray cast, the same predicate `wipe-tower`'s `point_in_polygon` uses). Submit the kept points as one `slicer_sdk::host_batch::raycast_z_down_batch(&[RaycastRequest])` call — the batched form exists precisely to avoid N guest↔host crossings; `slicer_sdk::host::raycast_z_down` is the single-point fallback. `start_z` is just above the object's bounds max Z.
3. **Reject.** The first `Some(_)` in the batch result means the object has material above an excluded point. Return `ModuleError::fatal` naming the object id, the key, and the point. The prepass executor turns that into `PrepassExecutionError::FatalModule`, and the pipeline into `PipelineError::Prepass` — the slice fails, as canonical's `Print::validate` does.

The module commits nothing to the blackboard and holds no claim, so the host built-in `host:mesh_analysis` still produces `SurfaceClassification` exactly as today.

Separately, `wipe-tower` declares the same key and extends its existing code-3 bed-bounds site: any tower footprint corner inside the exclusion polygon is a fatal rejection naming `bed_exclude_area`. This is not redundant — the tower is generated at `PostPass::LayerFinalization`, hundreds of pipeline steps after the validator, and no pre-slice pass can see it.

## Why a module, and why this stage

The alternative — a `validate_bed_exclusion` function beside `validate_support_layer_heights`, called from `run_slice_with_collector` — is shorter and would use the host-side `compute_xy_footprint` helper that already exists in `crates/slicer-core/src/algos/mesh_analysis.rs`, giving an exact footprint. It was rejected: `docs/00_project_overview.md`'s goals are a modular pipeline and community extensibility, and map Authoring rule 4 says new decision points go where the architecture puts them rather than becoming host-side special cases. Print validation is exactly the kind of policy a user or a printer vendor should be able to replace, tighten, or switch off by swapping a module — a hardcoded host check can be none of those things. It also gives the P18/P19 print-volume keys (`printable_height`, `extruder_printable_area`, `extruder_clearance_*`) a home that already exists instead of accreting more host branches.

`PrePass::MeshAnalysis` is the right stage because it is the earliest one a module can occupy, it requires no prepass slots, and failing there costs the user nothing — no slicing work has happened yet.

## Mechanism Check (Authoring rule 4)

- **No WIT change.** `mesh-analysis`'s `run(objects: list<object-id>, output, config)` and the `mesh-analysis-module` world's import of `slicer:common/host-services` are used exactly as they stand. `object-bounds` and `raycast-z-down` are already exported.
- **No IR schema bump, no new `ResolvedConfig` field, no new error type.** The key rides `ResolvedConfig.extensions` → `to_config_map` → `bind_module_config_view`; the rejection rides `ModuleError::fatal`, whose fatal path is already wired end to end.
- **No claim.** `RECOGNIZED_CLAIMS` includes `mesh-analyzer`, but holding it would put the validator in conflict with the stage's host built-in, which is the actual mesh analyzer. Validation is not an interchangeable-implementation role with an output slot to own; it is an inert observer that can veto. `holds = []` (the `wipe-tower` precedent). A future packet adding a *second* validator can revisit whether a `claim:print-validation` is warranted; this packet does not mint a claim id for one module.
- **No host-side special case.** The only host-crate edits are registration (workspace member, integrated registry, CLI features) and tests.
- **No `[BLOCK]` is open in this packet.**

## Tier Derivation

**Tier C** — new granular module at a seam this tree does not use yet (ticket 04's rubric). Authoring rule 1 forces B or C for a packet that builds a decision point; the new module, its guest, and its registration surface put it above the single-module-diff shape of Tier B. The prior revision's Tier A rested on the tower-corner reading of the key and does not survive the ⚠ correction.

## Code Change Surface (authoritative files-in-scope)

| File | Change |
| --- | --- |
| `modules/core-modules/print-validator/Cargo.toml` | **new** crate manifest, modelled on `modules/core-modules/layer-planner-default/Cargo.toml` |
| `modules/core-modules/print-validator/print-validator.toml` | **new** module manifest (AC-1) |
| `modules/core-modules/print-validator/wit-guest/**` | **new** guest WIT wiring for the `mesh-analysis-module` world |
| `modules/core-modules/print-validator/src/lib.rs` | **new** — `PrepassModule::run_mesh_analysis`, polygon parse, bounds pre-filter, probe grid, fatal rejection |
| `modules/core-modules/print-validator/tests/bed_exclusion_tdd.rs` | **new** — AC-2, AC-4, AC-5, AC-6, AC-N3 |
| root `Cargo.toml` | add the crate as a workspace member |
| `crates/slicer-integrated-modules/{Cargo.toml, src/lib.rs}` | register the module for the integrated edition |
| `crates/pnp-cli/Cargo.toml` | passthrough feature entry |
| `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` | core-module count +1 (re-derived from disk) |
| `crates/slicer-runtime/tests/integration/bed_exclusion_abort_tdd.rs` | **new** — AC-2 abort path, AC-3, AC-N2 |
| `crates/slicer-runtime/tests/integration/main.rs` | `mod bed_exclusion_abort_tdd;` registration — without it the file compiles to zero tests and reports green |
| `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` | AC-N1 arm |
| `modules/core-modules/wipe-tower/wipe-tower.toml` | one new `[config.schema.bed_exclude_area]` table |
| `modules/core-modules/wipe-tower/src/lib.rs` | parse the polygon in `from_config`; extend the code-3 corner check |
| `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` | AC-7 arms |
| `docs/04_host_scheduler.md` | one sentence: `PrePass::MeshAnalysis` hosts a guest validator beside its built-in; a fatal error there aborts the slice |
| `docs/15_config_keys_reference.md` | regenerated by `cargo xtask gen-config-docs` — never hand-edited |

## Read-Only Context

`modules/core-modules/layer-planner-default/**` (the prepass-module shape to copy), `crates/slicer-schema/wit/deps/prepass-mesh-analysis/prepass-mesh-analysis.wit` and `crates/slicer-schema/wit/deps/common.wit` (the `run` signature and host services), `crates/slicer-sdk/src/traits.rs` (`PrepassModule`), `crates/slicer-runtime/src/prepass.rs` (fatal handling — ranged read only), `modules/core-modules/wipe-tower/src/lib.rs` (`point_in_polygon`, `parse_printable_area`, `float_list_from_config`).

## Out of Bounds (must not be loaded or edited)

- `crates/slicer-schema/wit/**` — no WIT edit is in scope; if the implementation believes it needs one, it stops and raises a `[BLOCK]` rather than editing.
- `crates/slicer-runtime/src/run.rs` and `crates/slicer-scheduler/src/config_resolution.rs` — the host-side validator route was considered and rejected; do not add one.
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`).
- `crates/slicer-core/src/algos/mesh_analysis.rs` — `compute_xy_footprint` is host-only and stays host-only.
- Other packet directories, including `254a` / `254b` / `255` (reconcile via their `packet.spec.md` through a SUMMARY dispatch).
- `OrcaSlicerDocumented/` — delegated reads only.

## Expected Dispatches

| Question | Scope | Return |
| --- | --- | --- |
| Current core-module count and the exact assertion text | `modules/core-modules/`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` | `FACT` ≤ 5 lines |
| Exact `PrepassModule` trait shape and the `MeshAnalysisOutput` methods | `crates/slicer-sdk/src/traits.rs` | `SNIPPETS` ≤ 1 × 30 lines |
| The registration points a new core module must touch (workspace, integrated registry, CLI features) as they stand now | root `Cargo.toml`, `crates/slicer-integrated-modules/**`, `crates/pnp-cli/Cargo.toml` | `LOCATIONS` ≤ 10 |
| Whether `254b` has landed and already changed the core-module count | `docs/spec_packets/254b-prime-tower-interface-and-ramming/packet.spec.md` | `FACT` ≤ 3 lines |
| Each verification command | workspace | `FACT` pass/fail |

## Divergences (recorded, with rationale)

- **DIV-1 — sampled probe instead of a convex hull.** Canonical intersects each model volume's 2D convex hull with the exclusion polygon. At `PrePass::MeshAnalysis` a guest cannot see triangles (`mesh-object-view` is passed only to the seam-planning and support-geometry worlds), and extending the WIT is out of scope, so the port probes the excluded region on a `1.0` mm grid with `raycast-z-down` instead. Two consequences, both stated rather than hidden: (a) the port can miss a collision narrower than the grid pitch — a **false negative**, which is the safe direction for a fatal check, unlike the false positives an axis-aligned bounding-box test would produce; (b) the port tests the *actual mesh*, not its hull, so a C-shaped object whose hull covers the exclusion zone but whose material does not is **accepted** here and rejected by canonical. (b) is arguably the better answer — the hull is canonical's approximation, not its intent — but it is a difference, and it is recorded as one. Tightening this needs a host service exposing a per-object footprint polygon over WIT; that is a WIT change and therefore a separate packet, named here and not built.
- **DIV-2 — the wipe tower is validated too.** Canonical never tests the tower against `bed_exclude_area`. This port does, because the tower is a real printed structure the pre-slice validator cannot see, and letting it print into a cutter zone would be a defect the user cannot diagnose. Recorded as a deliberate improvement, per Authoring rule 4's "where the port can give a better answer, take it".
- **Probe pitch.** `1.0` mm is a fixed module constant, not a config key: it is a numerical tolerance, not a decision point, and inventing a PnP-specific key for it would add a key the queue does not track. If a future packet needs it tunable, the `_mm`-suffixed naming convention (grilling ruling Q15(a)) applies.

## Invariants

- With `bed_exclude_area` absent, empty, or degenerate, the module performs **zero** raycasts and returns `Ok(())`, and the emitted G-code is byte-identical to the pre-packet baseline. Adding a validator costs nothing when nothing is excluded.
- A malformed value never fails a slice — canonical's own default is degenerate.
- The rejection is fatal, never degraded: a non-fatal `ModuleError` is logged and execution continues, which would leave AC-2 unenforced.
- The module writes no IR and commits no blackboard slot, so `host:mesh_analysis` still produces `SurfaceClassification` unchanged.
- Only points strictly inside the exclusion polygon are probed; a point outside it can never trigger a rejection.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

  *Packet-specific note:* the exclusion polygon, `object-bounds`, `raycast-z-down` and the probe grid are all plain mm floats at this boundary — no scaled units appear. The `1.0` mm pitch is a millimetre literal, not a unit literal.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Config keys are **snake_case** in every Rust and TOML string (`CLAUDE.md` § Config Key Naming Convention).
- A module sees only its declared keys; an undeclared read returns `None` silently (`docs/03_wit_and_manifest.md` § Host-Boundary Access Enforcement). Both modules must declare `bed_exclude_area` to read it.

## Risks

- **The core-module count is a shared ledger fact.** `254b` also adds a module. Re-derive the count in the step that edits the assertion; a frozen number here would be wrong the moment either packet lands.
- **Aggregator registration.** `crates/slicer-runtime/tests/integration/` is a `mod`-aggregated bucket. An unregistered new file reports "0 tests" and reads as a pass — the exact false-green this repo's test discipline calls out. The registration is in the same step's edit list for that reason.
- **Probe cost.** The grid is bounded by the exclusion rectangle ∩ the object rectangle, not by the bed, so the worst case is proportional to the excluded area, which is small by construction. If a user configures an exclusion polygon covering most of the bed, the probe cost grows — acceptable, since that print is about to be rejected anyway.
- **Fatal-path blast radius.** Any test fixture that happens to configure `bed_exclude_area` and place an object in it will now fail the slice. At authoring the key has zero occurrences in the tree, so the blast radius is empty; re-derive that with `rg -n 'bed_exclude_area' crates modules resources` before Step 4.
- **Guest-registration churn.** Adding a core module touches the workspace manifest, the integrated registry, and the CLI feature list. The `254b` packet plans the same surface; the second lander reconciles rather than duplicates.

## Context Cost

`L` in aggregate (one M module step, one M registration step, two M/S wiring steps, one S docs step). No single step is L. If Step 2's registration sprawls, split the integrated-registry edit from the workspace-member edit rather than escalating the band.

## Open Questions

- **`[FWD]` — a per-object footprint host service.** Tightening DIV-1 from a sampled probe to a true footprint polygon needs a new `slicer:common/host-services` function (e.g. returning the object's XY outline) and therefore a WIT change. Named here, out of scope, and the natural companion to the P18/P19 print-volume packets that will share this module.
- **`[FWD]` — `claim:print-validation`.** If a second validator ever ships, the two will need a conflict rule. One module needs no claim; two might.
- No `[BLOCK]`.
