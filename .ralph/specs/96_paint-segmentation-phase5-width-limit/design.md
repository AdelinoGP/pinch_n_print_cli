# Design: 96_paint-segmentation-phase5-width-limit

## Controlling Code Paths

- Primary code paths: `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (NEW), `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (integration point post Phase 7), the config-schema landing location (host config OR `paint-segmentation-default` manifest per P95 structural choice).
- Neighboring tests or fixtures: `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` unit tests, three new integration tests in the cube_4color suite (or a sibling `cube_4color_phase5_tdd.rs`), potentially a small `resources/cube_4color_tall.3mf` (≤ 100 KB) authored if the existing fixture is too short to expose layer-alternation visibility.
- OrcaSlicer comparison surface: see `requirements.md`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Short-circuit invariant: when both `mmu_segmented_region_max_width = 0.0` and `mmu_segmented_region_interlocking_depth = 0.0` (defaults), Phase 5 is a no-op. Default-config slices produce byte-identical g-code (AC-8). This is the regression-guard contract.
- Beam-flag invariant: `mmu_segmented_region_interlocking_beam = true` produces constant-depth interlocking; `false` produces alternating-depth. When `interlocking_depth = 0`, the beam flag is meaningless (AC-N3 documents this).
- Negative-value invariant: any of the depth/width keys with negative value triggers `PaintSegmentationError::InvalidPhase5Config { key, value }` at runtime (AC-N1). The config-schema declares both as non-negative; the runtime guards against schema validator bypass.
- Empty-output invariant: width larger than the smallest variant footprint correctly produces empty per-variant polygons; entries persist in `SliceIR` (D15 compatible).

## Code Change Surface

- Selected approach: implement the kernel as a single function in a new file, write unit tests against synthetic polygon inputs (no full slice required), then add integration tests that exercise the full pipeline on the cube_4color fixture with non-default config.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`** (NEW, ≤ 200 LOC):
    ```rust
    pub fn cut_segmented_layers(
        variants_per_layer: &mut [HashMap<Vec<(String, PaintValue)>, ExPolygons>],
        input_expolygons_per_layer: &[ExPolygons],
        region_width_units: i64,
        interlocking_depth_units: i64,
        interlocking_beam: bool,
    ) -> Result<(), PaintSegmentationError>
    ```
    Algorithm:
    ```
    if region_width_units == 0 && interlocking_depth_units == 0 {
        return Ok(()); // short-circuit
    }
    if region_width_units < 0 || interlocking_depth_units < 0 {
        return Err(PaintSegmentationError::InvalidPhase5Config { ... });
    }
    for (layer_idx, variants) in variants_per_layer.iter_mut().enumerate() {
        let depth = if interlocking_depth_units == 0 {
            region_width_units
        } else if interlocking_beam {
            interlocking_depth_units
        } else if layer_idx % 2 == 0 {
            interlocking_depth_units + region_width_units
        } else {
            region_width_units
        };
        let layer_input = &input_expolygons_per_layer[layer_idx];
        let inner = offset(layer_input, -depth_to_mm(depth), ...);
        for (chain, expolys) in variants.iter_mut() {
            if chain.is_empty() { continue; } // base region unchanged
            *expolys = difference_ex(expolys.clone(), &inner);
        }
    }
    Ok(())
    ```
    (Adjust to exact polygon-ops signatures; the helpers `offset` + `difference_ex` exist post-P95.)
  - **`crates/slicer-core/src/algos/paint_segmentation/mod.rs`** (integration):
    After Phase 7's `compose_variants` call, add:
    ```rust
    let width_units = mm_to_units(config.mmu_segmented_region_max_width);
    let interlock_units = mm_to_units(config.mmu_segmented_region_interlocking_depth);
    let beam = config.mmu_segmented_region_interlocking_beam;
    cut_segmented_layers(&mut variants_per_layer, &input_expolygons_per_layer, width_units, interlock_units, beam)?;
    ```
    Then the existing `replace_slice_ir` commit at the end.
  - **Config-schema entries**: location depends on P95's structural decision. If host-effective config: `crates/slicer-runtime/src/builtins/<paint_segmentation_producer>.rs` or sibling config-schema file. If module manifest: `modules/core-modules/paint-segmentation-default/paint-segmentation-default.toml`. Each entry:
    ```toml
    [config.schema.mmu_segmented_region_max_width]
    type = "f32"
    default = 0.0
    units = "mm"
    description = "Maximum width of a paint-segmented region; 0 disables width limiting."
    minimum = 0.0
    ```
    (Mirror existing config-schema syntax in the workspace.)
  - **Three kernel unit tests** in `width_limit.rs` `#[cfg(test)] mod tests`:
    - `width_limit_only_no_interlocking_erodes_to_band`.
    - `interlocking_alternates_when_beam_false`.
    - `interlocking_constant_when_beam_true`.
    Synthetic input: 2-3 layers, simple ExPolygons (squares), known expected outputs.
  - **Three integration tests** under `crates/slicer-runtime/tests/executor/`:
    - `cube_4color_phase5_width_limit_bands_tdd.rs` (NEW).
    - `cube_4color_phase5_interlocking_alternates_tdd.rs` (NEW).
    - `cube_4color_phase5_interlocking_beam_constant_tdd.rs` (NEW).
    Each loads cube_4color.3mf (or cube_4color_tall.3mf), applies a non-default config, asserts on specific layer's variant polygons.
  - **Three negative-case kernel tests**:
    - `width_limit_negative_rejected`.
    - `width_limit_oversize_yields_empty`.
    - `interlocking_depth_zero_ignores_beam`.
  - **Optional `resources/cube_4color_tall.3mf`** (only if existing cube is too short for layer-alternation visibility — likely 30 mm + tall).
- Rejected alternatives that were considered and why they were not chosen:
  - **Make Phase 5 a separate sub-driver invoked from prepass directly**: rejected — the algorithm is internal to paint-segmentation; surfacing it as a stage would over-expose the implementation.
  - **Read config keys via direct `plan.config` (pre-P1a shape)**: rejected — that shape no longer exists after P1a. Use `region_map.config_for(&region_key)`.
  - **Make `interlocking_beam` an integer enum (depth-per-layer offset)**: rejected — OrcaSlicer's API is a bool; the user-facing semantics are `false = alternating`, `true = constant`. Don't over-engineer.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (NEW).
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (integration point).
- Config-schema file (location TBD per Step 1 dispatch — either host or module manifest).
- 3 new integration test files under `crates/slicer-runtime/tests/executor/`.
- Optional `resources/cube_4color_tall.3mf` (≤ 100 KB) only if needed.

≤ 3 files per step in implementation-plan.

## Read-Only Context

- `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 (50-80 lines; range-read).
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4".
- `docs/08_coordinate_system.md` — coordinate conversion table.
- `crates/slicer-core/src/polygon_ops.rs` — `offset` + `difference_ex` signatures (post-P95).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated code — never load.
- The other paint_segmentation sub-modules (`phase3.rs`, `colorize.rs`, `compose_variants.rs`) — P95 territory; not edited.
- `crates/slicer-runtime/src/prepass.rs` — not edited (Phase 5 is internal to paint-segmentation).
- Binary fixtures — never `Read`.

## Expected Sub-Agent Dispatches

- "Locate the config-schema file for paint-segmentation (either host or `paint-segmentation-default` module manifest); return FILE:LINE for the schema declaration block" — purpose: Step 1.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` Phase 5; return SUMMARY ≤ 150 words" — purpose: kernel design reference.
- "Open `crates/slicer-core/src/algos/paint_segmentation/mod.rs` and locate the post-Phase-7 / pre-replace_slice_ir region; return SNIPPETS (≤ 30 lines)" — purpose: integration point.
- "Run `cargo test -p slicer-core paint_segmentation::width_limit 2>&1 | tee target/test-output.log`; FACT" — kernel tests.
- "Run `cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log`; FACT" — integration tests.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-wedge.gcode && sha256sum /tmp/p96-wedge.gcode`; FACT" — AC-8 wedge byte-identical.
- "Run the same on cube_4color.3mf with default config; FACT" — AC-8 cube byte-identical.

## Data and Contract Notes

- IR contracts touched: none new. `SlicedRegion.polygons` shape unchanged.
- WIT boundary considerations: config-schema additions, if landed in module manifest, propagate to guest manifest TOML — `cargo xtask build-guests --check` confirms.
- Determinism or scheduler constraints: Phase 5's even/odd layer alternation is deterministic by layer index; same input → same output.

## Locked Assumptions and Invariants

- **Default config (0/0) → no-op**: regression-guard contract.
- **Negative values rejected**: schema-level + runtime defense.
- **Beam-only-with-depth invariant**: `beam = true` with `depth = 0` is silently a no-op; documented in the schema description.

## Risks and Tradeoffs

- **Risk: Phase 5 interacts badly with downstream perimeter generation** (the variant polygons it produces have eroded inner boundaries; perimeter generator might produce duplicate / overlapping perimeters). Mitigation: AC-9's visual report check; if visual confirms banding without perimeter artifacts, packet ships.
- **Risk: integration test for "alternating bands across adjacent layers" requires careful Z-layer pair selection**. Mitigation: pick layers far from top/bottom (avoid edge effects); document the chosen layer indices in the closure log.
- **Tradeoff: 0.0 default vs. some non-zero default** (e.g., match OrcaSlicer's default). 0.0 default is conservative and preserves byte-identical regression; users can opt in via config.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 2 — kernel + 3 unit tests + 3 negative tests + 3 integration tests in close succession).
- Highest-risk dispatch: the config-schema location dispatch in Step 1 (drives where the three keys land).

## Open Questions

- `[FWD]` — Where does the config-schema for paint-segmentation live after P95? Host or module manifest? Step 1 dispatch confirms.
- `[FWD]` — Is `cube_4color.3mf` tall enough for layer-alternation visibility? Step 4 dispatch confirms; if not, author `cube_4color_tall.3mf` (≤ 100 KB).
- `[FWD]` — What's the canonical Polygon-ops `offset(expolys, delta_mm, ...)` signature (negative delta for inward offset)? Step 2 dispatch confirms.
- `[BLOCK]` — None.
