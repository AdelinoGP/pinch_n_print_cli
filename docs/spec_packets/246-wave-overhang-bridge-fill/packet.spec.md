---
status: draft
packet: 246-wave-overhang-bridge-fill
task_ids:
  - TASK-356
depends_on:
  - 245-lock-aware-infill-consumers
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 246-wave-overhang-bridge-fill

## Goal

Ship the `com.core.wave-overhangs` bridge-fill module — a PnP bridge-fill adaptation of the
canonical `WaveOverhangs.cpp` generator that emits order-locked, anchor-first `BridgeInfill` waves
over external bridge areas, excludes internal bridges (unlocked rectilinear fallback), and falls back
to conventional rectilinear scanlines when waves cannot be generated.

## Scope Boundaries

This packet adds one new guest module (`modules/core-modules/wave-overhangs/`, scaffolded on
`gyroid-infill`) plus the `internal-bridge-areas` view accessor it needs. It does NOT change the
`InternalBridgeInfill` host constructor, does NOT add a new `ExtrusionRole`, and does NOT change the
host partition (no fifth polygon). Holder selection forces waves: being configured as
`bridge_fill_holder` is the enable (equivalent to canonical `use_instead_of_bridges = true` for
external bridge sites). `rectilinear-infill` remains the default holder. The generator is an own
copy of canonical `WaveOverhangs.cpp` (not a faithful stage/site port), with the divergence recorded
in packet prose.

## Prerequisites and Blockers

- Depends on: 245-lock-aware-infill-consumers (the linker/optimizer/emitter honor `order_lock`, so
  the waves' anchor-first order survives); 243-object-scoped-overhang-annotation (object-scoped
  `prev_layer_boundaries` is the `prev_object_boundary` source); 244-order-locked-extrusion-sequences
  (the `order_lock` carrier + SDK `OrderLockAllocator` + host remap).
- Unblocks: none (final packet in the wave-overhangs queue).
- Activation blockers: none known; the module is opt-in via `bridge_fill_holder`.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the tree, **when** the internal-bridge view accessor is inspected, **then** WIT
  `slice-region-view` carries `internal-bridge-areas: func() -> list<ex-polygon>` in
  `crates/slicer-schema/wit/deps/ir-types.wit`, the SDK `SliceRegionView`
  (`crates/slicer-sdk/src/views.rs`) carries an `internal_bridge_areas` field with a getter and a
  host-only setter, the macro adapter (`crates/slicer-macros/src/lib.rs`) sets it, and the marshal
  (`crates/slicer-wasm-host/src/marshal/in_.rs`) reads it. |
  `rg -q 'internal-bridge-areas: func\(\) -> list<ex-polygon>' crates/slicer-schema/wit/deps/ir-types.wit && rg -q 'internal_bridge_areas' crates/slicer-sdk/src/views.rs && rg -q 'internal_bridge_areas' crates/slicer-macros/src/lib.rs && rg -q 'internal_bridge_areas' crates/slicer-wasm-host/src/marshal/in_.rs && cargo test -p slicer-sdk --test test_support_slice_region_view_builder_setters_tdd 2>&1 | tee target/test-output.log | grep -qE "^test result: ok" && echo P246_VIEW_ACCESSOR`
- **AC-2. Given** the tree, **when** the module scaffold is inspected, **then**
  `modules/core-modules/wave-overhangs/` exists with `Cargo.toml`, `wave-overhangs.toml`,
  `src/lib.rs`, `src/generator.rs`, `tests/`, and `wit-guest/`; the manifest declares
  `id = "com.core.wave-overhangs"`, `holds = ["claim:bridge-fill"]` (only), and
  `[ir-access] reads = ["SliceIR"] writes = ["InfillIR"]`; the crate is a root workspace member and
  an optional dep + feature in `crates/slicer-integrated-modules/Cargo.toml` and
  `crates/pnp-cli/Cargo.toml`. |
  `rg -q 'id\s*=\s*"com.core.wave-overhangs"' modules/core-modules/wave-overhangs/wave-overhangs.toml && rg -q 'holds\s*=\s*\["claim:bridge-fill"\]' modules/core-modules/wave-overhangs/wave-overhangs.toml && rg -q 'reads\s*=\s*\["SliceIR"\]' modules/core-modules/wave-overhangs/wave-overhangs.toml && rg -q 'writes\s*=\s*\["InfillIR"\]' modules/core-modules/wave-overhangs/wave-overhangs.toml && rg -q 'modules/core-modules/wave-overhangs' Cargo.toml && rg -q 'wave-overhangs' crates/slicer-integrated-modules/Cargo.toml && rg -q 'wave-overhangs' crates/pnp-cli/Cargo.toml && echo P246_SCAFFOLD`
- **AC-3. Given** config `bridge_fill_holder = "wave-overhangs"`, **when** the scheduler resolves
  holders, **then** `module_id_matches_holder("com.core.wave-overhangs", "wave-overhangs")` is true
  and the module holds `claim:bridge-fill`; with no override, `rectilinear-infill` remains the
  default bridge-fill holder. |
  `cargo test -p slicer-scheduler --test contract -- module_id_matches_holder_wave_overhangs --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_HOLDER_SELECTION`
- **AC-4. Given** a region with nonempty external bridge areas and a resolvable anchor band, **when**
  the module runs, **then** waves are emitted as role `BridgeInfill`, order-locked (one tag per
  connected wave domain via `OrderLockAllocator`), anchor-first, with the first front intersecting
  supported material and each subsequent front within one wavelength of a predecessor. |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- waves_emitted_anchor_first_order_locked --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_WAVE_EMISSION`
- **AC-5. Given** a region with both external and internal bridge areas, **when** the module runs,
  **then** waves cover only `bridge_areas − internal_bridge_areas`; internal-qualified polygons get
  unlocked rectilinear fallback with today's role mapping; no locked footprint overlaps an internal
  area. |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- internal_bridge_areas_excluded_from_waves --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_INTERNAL_EXCLUSION`
- **AC-6. Given** a region where waves cannot be generated (missing anchors, empty seeds,
  min-length-filtered components, iteration residual, or empty generator output), **when** the module
  runs, **then** conventional rectilinear bridge scanlines are emitted (bridge orientation, resolved
  bridge width/nozzle fallback, canonical bridge spacing/flow, `BridgeInfill` with speed factor 1.0)
  and every nonempty external bridge component emits at least one wave or fallback path. |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- fallback_rectilinear_no_silent_drop --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_FALLBACK`
- **AC-7. Given** `wave_overhang_print_speed` and a region's resolved `bridge_speed`, **when** the
  module emits, **then** `speed_factor = wave_overhang_print_speed / bridge_speed` is set per region,
  and the flow factor is `wave_overhang_flow_mm3_per_mm / (nozzle_diameter × effective_layer_height)`
  (layer-height-independent bead area via the existing volumetric-E path). |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- speed_and_flow_factors_resolved --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_SPEED_FLOW`
- **AC-8. Given** `resources/A_upsidedown.obj` sliced with `bridge_fill_holder = "wave-overhangs"`,
  **when** the real pipeline runs, **then** the typed capture contains at least one contiguous
  order-locked `BridgeInfill` block, and the emitted G-code wave speed and extruded volume match the
  configured `wave_overhang_print_speed` and `wave_overhang_flow_mm3_per_mm` (the discriminator
  against rectilinear fallback, which shares the role). |
  `cargo test -p slicer-runtime --test e2e -- wave_overhang_bridge_fill_e2e --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_E2E`
- **AC-9. Given** the module, **when** the generator runs twice on identical input (and native vs
  wasm dispatch), **then** the output is byte-identical (deterministic smart traversal). |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- deterministic_double_run --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_DETERMINISM`

## Negative Test Cases

- **AC-N1. Given** a region whose `wave_overhang_print_speed / bridge_speed` ratio falls outside the
  emitter clamp `[0.05, 5.0]`, **when** the module emits, **then** the slice is rejected with a fatal
  error naming the unrepresentable speed factor (no silent clamp). |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- speed_factor_out_of_clamp_rejected --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_SPEED_CLAMP_REJECT`
- **AC-N2. Given** a region with internal bridge areas, **when** the module runs, **then** no locked
  wave footprint overlaps an internal area, and the host `InternalBridgeInfill` constructor still
  emits the internal bridge paths (untouched). |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- locked_footprint_disjoint_from_internal --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_INTERNAL_DISJOINT`
- **AC-N3. Given** missing-anchor and narrow-anchor-band inputs (fork issue #84 analog), **when** the
  module runs, **then** the fallback leaves no holes in the external bridge coverage. |
  `cargo test -p wave-overhangs --test wave_overhangs_tdd -- missing_and_narrow_anchor_no_holes --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P246_NO_HOLES`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask build-guests` (after the WIT/manifest changes; then `cargo xtask build-guests --check` must exit 0 before attributing any failure)

## Authoritative Docs

- `docs/specs/wave-overhangs-bridge-fill-plan.md` - normative plan; §"Packet 4 — com.core.wave-overhangs
  module", the config-keys list, and the Tests section are the governing brief.
- `docs/03_wit_and_manifest.md` - §"Holder identifier matching" (lines ~746-763) and the manifest
  field reference (lines ~641-744); the doc is over 300 lines so only these ranges are read directly.
- `docs/08_coordinate_system.md` - coordinate-system hazard (1 unit = 100 nm).
- `docs/ORCASLICER_ATTRIBUTION.md` - porting header requirement.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/WaveOverhangs/WaveOverhangs.cpp` — `generate` (the algorithm body), `generate_narrow_split_slits`, `reconnect_polylines`, `append_wave_fronts`, `append_zig_zag_front_levels`, `generate_wave_overhang_seeds`, `should_generate_waves_for_region` (the bridgeability gate this packet ports but effectively bypasses).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `generate_wave_overhang_paths` (the call site; this packet runs the generator over PnP's `bridge_areas` sites instead).
- `OrcaSlicerDocumented/docs/ALGORITHMS.md`, `OrcaSlicerDocumented/docs/WAVE_OVERHANG_SETTINGS.md`, `OrcaSlicerDocumented/docs/LIMITATIONS.md` — the fork's own algorithm/settings/limitations docs.

## Doc Impact Statement (Required)

- `docs/03_wit_and_manifest.md` §"Holder identifier matching" - no edit required (the short-name
  matching already covers `com.core.wave-overhangs` → `wave-overhangs`); this packet adds no new
  claim id. |
  `rg -q 'com\.core\.' docs/03_wit_and_manifest.md && echo P246_DOCS_UNCHANGED`
- No IR schema version bump: the `internal-bridge-areas` accessor is a WIT/SDK view addition over the
  already-existing `SlicedRegion.internal_bridge_areas` field (`crates/slicer-ir/src/slice_ir.rs`),
  and the `slicer:ir-handles` package is unversioned (ADR-0044). |
  `rg -q 'pub internal_bridge_areas: Vec<ExPolygon>' crates/slicer-ir/src/slice_ir.rs && echo P246_NO_SCHEMA_BUMP`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
