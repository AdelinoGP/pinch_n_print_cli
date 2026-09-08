# Design: 246-wave-overhang-bridge-fill

## Controlling Code Paths

- Primary code path: the new `com.core.wave-overhangs` module (`modules/core-modules/wave-overhangs/`),
  a `Layer::Infill` module whose `run` reads `SliceRegionView` and writes `InfillIR` via
  `InfillOutputBuilder`, mirroring `gyroid-infill`'s `src/lib.rs`.
- Neighboring tests/fixtures: `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` (the
  scaffold precedent); `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_arbiter_e2e_tdd.rs`
  (the `pnp_cli visual-debug` typed-capture precedent for the end-to-end AC).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat
  delegation rules.

## Architecture Constraints

- The generator is an own copy inside the module (ADR-0026 single-caller rule): no extraction to
  `slicer-core`, no sharing with `rectilinear-infill`. The fallback rectilinear scanlines are an
  owned copy inside the wave module, not a call into `rectilinear-infill`.
- Holder selection forces waves: being configured as `bridge_fill_holder` is the enable (equivalent
  to canonical `use_instead_of_bridges = true` for external bridge sites). No master enable bool.
- The `internal-bridge-areas` accessor is a WIT/SDK view addition over the already-existing
  `SlicedRegion.internal_bridge_areas` field; no IR schema version bump (the `slicer:ir-handles`
  package is unversioned per ADR-0044).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: one new guest module plus a four-file view-accessor addition. The generator is
  ported as an own copy of canonical `WaveOverhangs.cpp` (not a faithful stage/site port), with the
  divergence recorded in packet prose.

### View accessor (prerequisite)

- `crates/slicer-schema/wit/deps/ir-types.wit` — add `internal-bridge-areas: func() -> list<ex-polygon>`
  to the `slice-region-view` resource (after `bridge-areas`).
- `crates/slicer-sdk/src/views.rs` — add `internal_bridge_areas: Vec<ExPolygon>` to `SliceRegionView`,
  a `pub fn internal_bridge_areas(&self) -> &[ExPolygon]` getter, and a host-only
  `pub fn set_internal_bridge_areas(&mut self, ...)` setter (mirroring `bridge_areas` at lines
  ~272/446).
- `crates/slicer-macros/src/lib.rs` — in the `SliceRegionView` projection (lines ~2402-2469), read
  `r.internal_bridge_areas()` and call `sdk_view.set_internal_bridge_areas(...)`.
- `crates/slicer-wasm-host/src/marshal/in_.rs` — in the `SliceRegionData` assembly (lines ~392-520),
  add `internal_bridge_areas: ir_to_wit_expolygons(view.internal_bridge_areas())`.

### Module scaffold (`modules/core-modules/wave-overhangs/`)

- `Cargo.toml` — mirror `gyroid-infill`'s (deps on `slicer-sdk`, `slicer-schema`, `slicer-ir`,
  `slicer-core`; dev-dep `slicer-sdk` with `test` feature; wasm32 `wit-bindgen`).
- `wave-overhangs.toml` — `id = "com.core.wave-overhangs"`, `[stage] id = "Layer::Infill"`,
  `[ir-access] reads = ["SliceIR"] writes = ["InfillIR"]`, `[claims] holds = ["claim:bridge-fill"]`,
  the config schema (table in `requirements.md`), `[config.overridable-per-region]` /
  `[config.overridable-per-layer]` listing the wave keys, and `[hints]`.
- `src/lib.rs` — the `#[slicer_module]` `LayerModule` impl: `from_config` reads the wave keys and the
  required bridge basics; `run` implements the region pipeline and delegates to `generator.rs`.
- `src/generator.rs` — the ported generator (own copy), with the standard porting header
  (`docs/ORCASLICER_ATTRIBUTION.md`) crediting Andersons, Sanchez, Vaneker, McCulloch, Klappe.
- `tests/wave_overhangs_tdd.rs` — module-level tests (generator, fallback, exclusion, determinism).
- `wit-guest/` — mirror `gyroid-infill/wit-guest/`.
- Registration: add `"modules/core-modules/wave-overhangs"` to the root `Cargo.toml` workspace
  members; add `wave-overhangs = { path = "...", optional = true }` + a feature to
  `crates/slicer-integrated-modules/Cargo.toml`; add `integrated-wave-overhangs = [...]` to
  `crates/pnp-cli/Cargo.toml`.

### Region pipeline (in `src/lib.rs`)

1. `supported_fill = prev_object_boundary ∩ union(top_solid_fill, bottom_solid_fill, sparse_infill_area)`
2. `anchor_band = supported_fill ∩ expand(external_bridge_areas, anchor_depth)` where
   `anchor_depth = wave_overhang_anchor_depth_mm` if > 0, else auto
   `anchors_size + base_spacing`, with `anchors_size = min(3 mm, bridge extrusion spacing ×
   (wall_count + 1))`. **Deviation from canonical-auto (deliberate):** bare `anchors_size` never
   exceeds the generator own `anchors_size`, so `inset_anchors` comes out empty, seed generation
   fails, and every component falls back to rectilinear — measured as ZERO waves on
   `resources/A_upsidedown.obj`. The `+ base_spacing` floor restores the packet intent that
   selecting the module as `bridge_fill_holder` IS the enable. Pinned by
   `waves_engage_with_default_anchor_depth`.
3. `wave_domain = external_bridge_areas ∪ anchor_band`.
4. Waves forced (holder selection); fallback on missing anchors / empty seeds / min-length-filtered
   components / iteration residual / empty output.
5. Emit waves as `BridgeInfill`, order-locked (one `OrderLockAllocator` tag per connected wave
   domain), anchor-first. Internal polygons → unlocked rectilinear fallback.
6. Flow: `width = nozzle_diameter`; `flow_factor = wave_overhang_flow_mm3_per_mm / (width ×
   effective_layer_height)`.
7. Speed: `speed_factor = wave_overhang_print_speed / bridge_speed`; fatal when outside `[0.05, 5.0]`.

- Rejected alternatives: host-carved fifth partition polygon (`bridge_anchor_area`) — rejected (D4);
  self-limiting waves to `bridge_areas` — rejected (removes supported-side bonding); a new
  `ExtrusionRole` — rejected (D2); sharing the fallback with `rectilinear-infill` — rejected
  (ADR-0026).

## Files in Scope (read + edit)

- `modules/core-modules/wave-overhangs/Cargo.toml` - role: new; expected change: scaffold.
- `modules/core-modules/wave-overhangs/wave-overhangs.toml` - role: new; expected change: manifest.
- `modules/core-modules/wave-overhangs/src/lib.rs` - role: new; expected change: module + pipeline.
- `modules/core-modules/wave-overhangs/src/generator.rs` - role: new; expected change: ported generator.
- `modules/core-modules/wave-overhangs/tests/wave_overhangs_tdd.rs` - role: new; expected change: tests.
- `modules/core-modules/wave-overhangs/wit-guest/` - role: new; expected change: guest glue.
- `crates/slicer-schema/wit/deps/ir-types.wit` - role: edit; expected change: `internal-bridge-areas`.
- `crates/slicer-sdk/src/views.rs` - role: edit; expected change: field + getter + setter (incl. the
  hand-written `Default` impl and the `from_ir` clone — the latter is load-bearing, without it the
  accessor always reads empty).
- `crates/slicer-sdk/src/test_support/fixtures.rs` - role: edit; expected change: `SliceRegionViewBuilder`
  field + default + builder method + `build()` setter call.
- `crates/slicer-wasm-host/src/host.rs` - role: edit; expected change: `SliceRegionData` field, its
  exhaustive literal, and the `slice-region-view` resource impl `internal_bridge_areas` (provenance
  `"SliceIR.regions.internal-bridge-areas"`). Forced by the WIT resource gaining a func.
- `crates/slicer-wasm-host/tests/contract/{slice_region_view_contract_tdd.rs,wit_boundary_tdd.rs}` -
  role: edit; expected change: the 7 exhaustive `SliceRegionData` literals gain the new field.
- `crates/slicer-macros/src/lib.rs` - role: edit; expected change: projection.
- `crates/slicer-wasm-host/src/marshal/in_.rs` - role: edit; expected change: marshal.
- `Cargo.toml`, `crates/slicer-integrated-modules/Cargo.toml`, `crates/pnp-cli/Cargo.toml` - role:
  edit; expected change: registration.
- `crates/slicer-scheduler/tests/contract/holder_matching_tdd.rs` + `tests/contract/main.rs` - role:
  new; expected change: the `module_id_matches_holder_wave_overhangs` holder-selection test (AC-3).
- `crates/slicer-runtime/tests/e2e/wave_overhang_bridge_fill_e2e_tdd.rs` + `tests/e2e/main.rs` - role:
  new; expected change: end-to-end discriminator.

## Read-Only Context

- `modules/core-modules/gyroid-infill/` - `Cargo.toml`, `gyroid-infill.toml`, `src/lib.rs`,
  `wit-guest/Cargo.toml` only - purpose: scaffold precedent.
- `crates/slicer-runtime/tests/e2e/calicat_internal_bridge_arbiter_e2e_tdd.rs` - lines `1-120` only -
  purpose: the `pnp_cli visual-debug` typed-capture driver precedent.
- `crates/slicer-gcode/src/emit.rs` - `resolve_feedrate` (lines ~144-187) and the volumetric-E loop
  (lines ~554-570) only - purpose: speed clamp `[0.05, 5.0]` and E formula precedent.
- `crates/slicer-core/src/polygon_ops.rs` - `offset`/`intersection`/`difference`/`union` signatures
  only - purpose: the geometry primitives.
- `crates/slicer-ir/src/slice_ir.rs` - `mm_to_units` (line ~62) only - purpose: mm→unit conversion.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-runtime/src/layer_executor.rs` - the `InternalBridgeInfill` constructor is untouched;
  do not edit.
- `crates/slicer-runtime/src/region_partition.rs` - the host partition is untouched; do not edit.
- `modules/core-modules/rectilinear-infill/` - no sharing (ADR-0026); do not edit.

## Expected Sub-Agent Dispatches

- Question: what is the exact `gyroid-infill` `src/lib.rs` `run`/`from_config` shape and the
  `InfillOutputBuilder` API for emitting `ExtrusionPath3D` with a role and width? scope:
  `modules/core-modules/gyroid-infill/src/lib.rs`; return: `SNIPPETS` (≤30 lines); purpose: Step 2.
- Question: how does `rectilinear-infill` resolve bridge orientation, bridge width/nozzle fallback,
  and bridge spacing/flow today (the fallback must mirror it)? scope:
  `modules/core-modules/rectilinear-infill/src/`; return: `SUMMARY` (≤200 words); purpose: Step 3.
- Question: does the `pnp_cli visual-debug` typed-capture for a `Layer::InfillPostProcess` tap expose
  `ExtrusionPath3D.order_lock` in its JSON? scope: `crates/slicer-runtime/src/visual_debug_render.rs`
  + the visual-debug request schema; return: `FACT`; purpose: Step 5 (end-to-end AC driver).
- Question: canonical `WaveOverhangs.cpp` — the exact seed-extraction, offset-loop, front-extraction,
  and pattern-assembly steps and their constants (spacing, overlap, min-width, min-new-area,
  min-length, max-iterations, flow, speed). scope: `OrcaSlicerDocumented/src/libslic3r/WaveOverhangs/WaveOverhangs.cpp`; return: `SUMMARY` (≤200 words, no code); purpose: Step 3.

## Data and Contract Notes

- IR/manifest contracts: `holds = ["claim:bridge-fill"]` only; `[ir-access] reads = ["SliceIR"] writes
  = ["InfillIR"]`. No new claim id; no IR schema bump.
- WIT boundary: `slice-region-view` gains `internal-bridge-areas` (additive, unversioned package).
- Determinism/scheduler constraints: smart traversal must be deterministic (double-run identical);
  native and wasm dispatch must agree.

## Locked Assumptions and Invariants

- Waves are order-locked with one tag per connected wave domain; the linker/optimizer/emitter (packet
  245) preserve them verbatim and carve untagged fill around their swept footprint.
- Internal-qualified polygons never receive waves; they get unlocked rectilinear fallback and the host
  `InternalBridgeInfill` constructor still emits them.
- `speed_factor` outside `[0.05, 5.0]` is a fatal rejection, never a silent clamp.

## Risks and Tradeoffs

- The generator is a substantial port; the own-copy rule (ADR-0026) means no shared geometry helper,
  so the port must be self-contained and tested at the module boundary.
- The end-to-end AC depends on the visual-debug typed-capture exposing `order_lock`; if it does not,
  the AC driver must use a `run_pipeline`-based capturing runner instead (see `[FWD]` below).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3, the generator port)
- Highest-risk dispatch and required return format: the canonical `WaveOverhangs.cpp` `SUMMARY`
  (≤200 words) — the port's fidelity hinges on it.

## Open Questions

- `[FWD]` Does the `pnp_cli visual-debug` typed-capture expose `ExtrusionPath3D.order_lock` for a
  `Layer::InfillPostProcess` tap? If not, the end-to-end AC (AC-8) must use a `run_pipeline`-based
  capturing runner (the `dispatch_infill_output_tdd.rs` `full_pipeline_with_typed_layer_dispatch`
  pattern) instead of the visual-debug driver. Resolve at Step 5 before authoring the e2e test.
- None `[BLOCK]`.
