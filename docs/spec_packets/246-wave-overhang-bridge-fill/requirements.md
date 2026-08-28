# Requirements: 246-wave-overhang-bridge-fill

## Packet Metadata

- Grouped task IDs: `TASK-356`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

PnP has no bridge-fill pattern that bonds its first fronts to solid ground. Canonical
`WaveOverhangs.cpp` (OrcaSlicer fork `dennisklappe/OrcaSlicer-WaveOverhangs`) extrudes an anchor band
into supported material beside the overhang so wave fronts bond to solid ground, then meanders
wave-shaped fronts across the overhang. This packet ports that generator as a PnP bridge-fill module
over the existing `bridge_areas` sites, with two deliberate adaptations: (1) it is a bridge-fill
adaptation, not a faithful port of the canonical stage/site semantics — the generator runs over
`bridge_areas` rather than replacing selected overhang perimeters; (2) internal bridges are excluded
and get unlocked rectilinear fallback. The waves are order-locked (packet 244) so the linker,
optimizer, and emitter (packet 245) preserve their anchor-first print order.

## In Scope

- **View accessor prerequisite.** Add `internal-bridge-areas` to the WIT `slice-region-view` resource
  (`crates/slicer-schema/wit/deps/ir-types.wit`), the SDK `SliceRegionView` field + getter +
  host-only setter (`crates/slicer-sdk/src/views.rs`), the macro adapter
  (`crates/slicer-macros/src/lib.rs`), and the marshal (`crates/slicer-wasm-host/src/marshal/in_.rs`).
  The module computes `external_bridge_areas = bridge_areas − internal_bridge_areas`.
- **Module scaffold.** `modules/core-modules/wave-overhangs/` mirroring `gyroid-infill`:
  `Cargo.toml`, `wave-overhangs.toml`, `src/lib.rs`, `src/generator.rs`, `tests/`, `wit-guest/`.
  Registration: root workspace members, optional dep + feature in
  `crates/slicer-integrated-modules/Cargo.toml` and `crates/pnp-cli/Cargo.toml`. Ships in all
  editions, opt-in; `rectilinear-infill` remains the default holder.
- **Manifest.** `holds = ["claim:bridge-fill"]` ONLY; `[ir-access] reads = ["SliceIR"] writes =
  ["InfillIR"]`. Selection via `bridge_fill_holder = "wave-overhangs"` (short-name match through
  `slicer_scheduler::validation::module_id_matches_holder`).
- **Region pipeline** (where `should_emit(BridgeInfill)` && `!bridge_areas().is_empty()`):
  1. `supported_fill = prev_object_boundary ∩ union(top_solid_fill, bottom_solid_fill, sparse_infill_area)`
  2. `anchor_band = supported_fill ∩ expand(external_bridge_areas, anchor_depth)`
  3. `wave_domain = external_bridge_areas ∪ anchor_band`
  4. Holder selection forces waves (canonical `should_generate_waves_for_region` ported but
     effectively bypassed, equivalent to `use_instead_of_bridges = true`). Fallback triggers on
     missing anchors, empty seeds, min-length-filtered components, iteration residual, or empty
     generator output. No silent component drop.
  5. Waves emitted as role `BridgeInfill`, order-locked (one tag per connected wave domain),
     anchor-first. Internal-qualified polygons get unlocked rectilinear fallback with today's role
     mapping; the host `InternalBridgeInfill` constructor is untouched.
  6. Flow: bead `width = nozzle_diameter`; `flow_factor = wave_overhang_flow_mm3_per_mm / (width ×
     effective_layer_height)` (volumetric-E precedent in `crates/slicer-gcode/src/emit.rs`).
  7. Speed: `speed_factor = wave_overhang_print_speed / bridge_speed` per region; fatal when the
     ratio falls outside the emitter clamp `[0.05, 5.0]`.
- **Generator port** (own copy of canonical `WaveOverhangs.cpp`): seed extraction along the supported
  boundary · narrow-neck split slits (`generate_narrow_split_slits`) · accumulated-region offset loop
  at line spacing · front extraction against the half-width-inset trim boundary ·
  simplify/reconnect (`reconnect_polylines`) · pattern assembly (`append_wave_fronts` smart
  support-scored start-end choice, monotonic append, `append_zig_zag_front_levels` meander) ·
  empty/short-front filtering. Not ported: Kaiser/generator shells, inert `min_angle`, inert
  `seam_mode`, progressive `spacing_mode`, corner taper (deferred), wall replacement, floor layers,
  G-code event injection.
- **Config keys** (snake_case, fork defaults; anchor depth is PnP-only): see the table below.
- **Porting header** per `docs/ORCASLICER_ATTRIBUTION.md`; credit Andersons, Sanchez, Vaneker,
  McCulloch, Klappe.

### Config keys

| key | type | default | notes |
| --- | --- | --- | --- |
| `wave_overhang_pattern` | enum | `smart` | `smart` \| `monotonic` \| `zigzag` |
| `wave_overhang_line_spacing` | float | `0.35` | mm |
| `wave_overhang_perimeter_overlap` | float | `0.1` | mm |
| `wave_overhang_minimum_width` | float | `0.7` | mm |
| `wave_overhang_min_new_area` | float | `0.01` | mm² |
| `wave_overhang_min_length` | float | `0.0` | mm |
| `wave_overhang_max_iterations` | int | `0` | 0 = unlimited |
| `wave_overhang_flow_mm3_per_mm` | float | `0.15` | mm³/mm |
| `wave_overhang_print_speed` | float | `2.0` | mm/s |
| `wave_overhang_anchor_depth_mm` | float | `0.0` | PnP-only; 0.0 = auto `anchors_size + base_spacing` where `anchors_size = min(3 mm, bridge extrusion spacing × (wall_count + 1))`; positive override up to 20.0. **Deviation from canonical-auto:** the bare canonical `anchors_size` never exceeds the generator internal `anchors_size`, leaving `inset_anchors` empty so ZERO waves generate and every component silently falls back to rectilinear (measured on `resources/A_upsidedown.obj`: 48 BridgeInfill paths, all speed_factor 1.0). The `+ base_spacing` floor makes waves engage out-of-the-box, which holder selection is meant to guarantee. Pinned by `waves_engage_with_default_anchor_depth`. |

Required reads (declared in the manifest so the config view exposes them): `bridge_speed`,
`bridge_line_width`, `bridge_flow`, `bridge_density`, `nozzle_diameter`, `wall_count`,
`layer_height`. All wave settings and required bridge basics resolve per region (modifier/layer
overrides honored).

## Out of Scope

- Support-free cantilevers / angled overhangs (D7 deferred).
- Solid floor layers above waves, Hilbert floor fill, per-wave fan/temp/travel/retract/dwell events,
  support suppression under covered areas, inner-wall replacement in the overhang zone, corner taper
  reinforcement, custom visual-debug overlays for module-internal seed/front/residual state.
- Changes to the `InternalBridgeInfill` host constructor or the host partition (no fifth polygon).
- A new `ExtrusionRole` variant (D2 rejected).

## Authoritative Docs

- `docs/specs/wave-overhangs-bridge-fill-plan.md` - 473 lines; §"Packet 4" and the config-keys list
  read directly; the rest is context.
- `docs/03_wit_and_manifest.md` - over 300 lines; §"Holder identifier matching" (lines ~746-763) and
  the manifest field reference (lines ~641-744) read directly.
- `docs/08_coordinate_system.md` - coordinate-system hazard.
- `docs/ORCASLICER_ATTRIBUTION.md` - porting header.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/WaveOverhangs/WaveOverhangs.cpp` — `generate` (the algorithm body), `generate_narrow_split_slits`, `reconnect_polylines`, `append_wave_fronts`, `append_zig_zag_front_levels`, `generate_wave_overhang_seeds`, `should_generate_waves_for_region` (the bridgeability gate this packet ports but effectively bypasses).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `generate_wave_overhang_paths` (the call site; this packet runs the generator over PnP's `bridge_areas` sites instead).
- `OrcaSlicerDocumented/docs/ALGORITHMS.md`, `OrcaSlicerDocumented/docs/WAVE_OVERHANG_SETTINGS.md`, `OrcaSlicerDocumented/docs/LIMITATIONS.md` — the fork's own algorithm/settings/limitations docs.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (view accessor), `AC-2` (scaffold + manifest), `AC-3` (holder selection), `AC-4`
  (wave emission), `AC-5` (internal exclusion), `AC-6` (fallback), `AC-7` (speed/flow), `AC-8`
  (end-to-end), `AC-9` (determinism).
- Negative: `AC-N1` (speed-factor clamp rejection), `AC-N2` (internal disjointness), `AC-N3`
  (missing/narrow anchor no holes).
- Cross-packet impact: this is the final packet in the wave-overhangs queue; it consumes the
  object-scoped `prev_layer_boundaries` (243), the `order_lock` carrier + `OrderLockAllocator` (244),
  and the lock-honoring consumers (245).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wave-overhangs --test wave_overhangs_tdd` | generator, fallback, exclusion, determinism | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-sdk --test test_support_slice_region_view_builder_setters_tdd` | view accessor setter | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_contract` | holder matching + manifest validation | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e -- wave_overhang_bridge_fill_e2e_tdd::wave_overhang_bridge_fill_e2e --exact` | end-to-end discriminator | FACT pass/fail |
| `cargo check --workspace --all-targets` | all targets compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT pass/fail |
| `cargo xtask build-guests` then `cargo xtask build-guests --check` | guest freshness (exit 0) | FACT exit code |

## Step Completion Expectations

- The WIT accessor addition and the module scaffold both feed the guest build; `cargo xtask
  build-guests` must run after each, and `--check` must exit 0 before any failure is attributed.
- The generator is an own copy inside the module (ADR-0026 single-caller rule); no extraction to
  `slicer-core`, no sharing with `rectilinear-infill`.
- All mm constants (spacing, overlap, widths, seed expansion) convert through `mm_to_units` at entry;
  never raw Orca constants (docs/08).

## Context Discipline Notes

- `OrcaSlicerDocumented/` is never loaded directly; all reads are delegated (orca-delegation snippet).
- `crates/slicer-sdk/src/views.rs` is large; read only the `SliceRegionView` struct + getter/setter
  ranges, never the whole file.
- `crates/slicer-macros/src/lib.rs` is large; read only the `SliceRegionView` projection range
  (lines ~2400-2470), never the whole file.
- `crates/slicer-wasm-host/src/marshal/in_.rs` is large; read only the `SliceRegionData` assembly
  range (lines ~371-520).
