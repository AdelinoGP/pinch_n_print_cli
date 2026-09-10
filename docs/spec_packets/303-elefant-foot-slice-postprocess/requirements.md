# Requirements: 303-elefant-foot-slice-postprocess

## Packet Metadata

- Grouped task IDs: none — this is a wayfinder queue packet, not a `docs/07` backlog slice. `task_ids: []` matches the queue precedent (packets 296–302).
- Backlog source: `docs/specs/orca-feature-gap/issues/93-author-packet-p86-quality-precision-new-elefant-foot.md` (wayfinder map: Close the OrcaSlicer FFF feature gap)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer shrinks the bottom layers of a print to cancel the "elephant foot" — the outward bulge a hot first layer develops when squashed onto the bed. Pinch 'n Print has no such correction anywhere: the only occurrence of either P86 key in this tree is the hardcoded `("elefant_foot_compensation", "0")` row in the `ORCA_CONFIG_PADDING` table (`crates/slicer-gcode/src/serialize.rs`), which under the map's authoring rule 2 is not evidence of anything. Any print sliced by this port carries the full elephant-foot error.

The correction is not a uniform inward offset. Canonical's `elephant_foot_compensation` computes a **per-vertex** compensation that is throttled wherever the contour is narrower than `min_contour_width`, so a thin rib or a small boss is shrunk less than a broad wall — precisely so that thin features do not vanish. Implementing this as `offset(-compensation)` would be visibly wrong on exactly the geometry the algorithm exists to protect, which is why P86 was tiered C rather than A.

This is one coherent slice because the two keys are a single mechanism: `elefant_foot_compensation` is the magnitude and `elefant_foot_compensation_layers` is the number of layers it tapers across. Neither means anything without the other, and neither means anything without the kernel.

No packet is reopened or superseded.

## In Scope

- **`crates/slicer-core/src/algos/elephant_foot.rs`** — a new, **ungated** module (no `#[cfg(feature = "host-algos")]`, matching its neighbour `bridge_over_infill`) exporting `elephant_foot_compensation(input: &ExPolygon, min_contour_width_mm: f32, compensation_mm: f32) -> ExPolygon` and an `ExPolygons` arity over it. Ungated is load-bearing: the module is a WASM guest and cannot link feature-gated host code. Carries the standard porting header from `docs/ORCASLICER_ATTRIBUTION.md`.
- **A 2D segment-distance acceleration structure private to that kernel file.** Canonical uses `EdgeGrid::Grid` at cell size `0.7 * search_radius`; this tree has no 2D equivalent (`slicer_core::AabbTree` is a 3D triangle structure and does not apply). The kernel builds its own uniform segment grid at the same cell size. It is a file-private implementation detail, not new public crate surface.
- **`modules/core-modules/elefant-foot/`** — a new core module scaffolded with `pnp_cli module new` on stage `Layer::SlicePostProcess`, comprising `Cargo.toml`, `elefant-foot.toml`, `src/lib.rs`, `wit-guest/`, and tests. It becomes the **first production module on that stage**; the seam is proven today only by `crates/slicer-wasm-host/test-guests/dispatch-layer-slice-postprocess-guest`.
- **Two owned config keys**, declared in `elefant-foot.toml` at canonical defaults: `elefant_foot_compensation` (`float`, default `0.0`, min `0.0`, unit mm) and `elefant_foot_compensation_layers` (`int`, default `1`, min `1`).
- **Five re-declared keys**, needed to reach the decision point, none of them P86 queue keys and none changing the queue count. `support_raft_layers` (the PnP name for canonical `raft_layers`; `int`, default `0`) gates the whole pass, following the identical re-declaration in `classic-perimeters.toml` and `arachne-perimeters.toml`. `outer_wall_line_width`, `initial_layer_line_width`, `line_width` and `nozzle_diameter` populate the `slicer_core::flow::RoleWidthContext` fields that `resolve_role_width(ExtrusionRole::OuterWall, ..)` consults, so `min_contour_width` is derived exactly as canonical's `Flow` overload does rather than guessed.
- **The taper and gate**, ported from `PrintObject::slice_volumes`: skip entirely when `support_raft_layers != 0` or `elefant_foot_compensation <= 0.0`; skip a layer when `layer_index >= elefant_foot_compensation_layers`; otherwise apply `efc - (efc / layers) * layer_index`.
- **Per-region write-back** through `SlicePostprocessBuilder::set_polygons`, one call per region, with a `RegionKey` carrying that region's own `object_id`, `region_id` and `variant_chain` plus the dispatched `layer_index`. The compensated result is unioned (`slicer_core::polygon_ops::union`) before write-back, mirroring canonical's `union_ex`.
- **Tests**: `crates/slicer-core/tests/algo_elephant_foot_tdd.rs` (kernel) and `modules/core-modules/elefant-foot/tests/{elefant_foot_tdd.rs,slicer_module_binding_tdd.rs}` (module). The kernel test target must **not** be given `required-features` in `crates/slicer-core/Cargo.toml` — the kernel is ungated and a `required-features` row would silently reduce a narrow `-p slicer-core` run to zero tests (see `CLAUDE.md` §"Feature-gated test files report green when they don't compile").
- **One `docs/DEVIATION_LOG.md` row** covering the seam divergences enumerated under Cross-packet impact below.

## Out of Scope

- **`brim_use_efc_outline`** (P05, currently `shed-to-queue` in `docs/specs/orca-feature-gap/issues/key-correction-inventory.md`). Canonical stores the pre-compensation footprints in `lslices_elfoot_uncompensated` and lets the brim choose between them; this packet stores no uncompensated copy, and this tree's `skirt-brim` module derives brim loops from a **bounding box**, not from the object contour at all (the bbox-vs-contour divergence ticket 12 recorded). The key therefore has no meaning to restore here, and no observable behaviour changes because of the omission. Its owner stays `skirt-brim` per ticket 04; it needs its own ticket once brim follows the real contour.
- **`elefant_foot_layers_density`** (canonical coPercent, min 50, max 100, default 100; read by `Fill.cpp` to densify solid infill across the same layer band). It is **absent from `docs/ORCA_CONFIG_REFERENCE.md` entirely**, so it is not in the 409-key queue and cannot be scoped by this packet. Surfaced to wayfinder ticket 123 (gap-source completeness audit).
- **`xy_contour_compensation` / `xy_hole_compensation`** (P88, wayfinder ticket 95). Canonical applies them to the same expolygons immediately **before** EFC inside `slice_volumes`. They stay with P88; this packet only fixes the ordering contract they must honour (see Cross-packet impact).
- Any change to `ORCA_CONFIG_PADDING` or its `elefant_foot_compensation` twin — rides ticket 132.
- Any per-tool (`tool_config:<n>:`) arm — both keys are canonical `PrintObjectConfig` scalars carried by the existing per-object overlay, not ticket 125's axis.
- Any change to `PrePass::Slice`, the layer executor's stage ordering, or the WIT surface.

## Key Disposition

Zero declaration-only keys: 0. Both P86 keys end this packet driving a behaviour-changing decision point with a test asserting the change at a non-default value.

| Key | Canonical type / default | Owner after this packet | Disposition |
| --- | --- | --- | --- |
| `elefant_foot_compensation` | coFloat, `0.`, min `0` | `elefant-foot` module manifest | **wired** — magnitude fed into `elephant_foot_compensation`'s `compensation_mm`; asserted at `0.2` by AC-2, AC-4, AC-6 |
| `elefant_foot_compensation_layers` | coInt, `1`, min `1` | `elefant-foot` module manifest | **wired** — taper divisor and layer-band guard in the per-layer formula; asserted at `3` by AC-6 |

## Authoritative Docs

- `docs/01_system_architecture.md` - over 300 lines; delegate a SUMMARY of layer-stage ownership and the claim-system rule-4 trigger test.
- `docs/03_wit_and_manifest.md` - over 300 lines; delegate a SUMMARY of the module manifest TOML schema (`[stage]`, `[claims]`, `[ir-access]`, `[config.schema]`).
- `docs/08_coordinate_system.md` - direct ranged read of the mm-to-unit helper section only.
- `docs/13_slicer_helpers_crate.md` - direct read; establishes which polygon primitives exist before the kernel adds any.
- `docs/ORCASLICER_ATTRIBUTION.md` - direct read; the porting header is mandatory on `elephant_foot.rs`.
- `docs/21_data_defaults_and_fixtures.md` - direct read; the `check-literals` rule governs the `RoleWidthContext` literals in the new tests.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - ranged read of the `elefant_foot_*` rows only (large asset).
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - ranged read of the P86 entry only (large asset).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp` — `elephant_foot_compensation` (both `ExPolygon` arities plus the `Flow` overload that derives `min_contour_width` as `width + spacing`): borrow the whole pass shape — the tiny-contour early-out, the `SCALED_EPSILON` simplify, the contour resample, the per-point distance and delta computation, `smooth_compensation_banded`, and the variable inward offset. This is the packet's primary borrow.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `PrintObject::slice_volumes`: borrow the `raft_layers == 0` gate, the `layer_id < elefant_foot_compensation_layers` guard, the `elfoot = efc - (efc / layers) * layer_id` taper, and the union of the kernel result. The `lslices_elfoot_uncompensated` store is a **named non-borrow** — see the deviation row.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: borrow the two defaults exactly (`elefant_foot_compensation` coFloat `0.`, min `0`; `elefant_foot_compensation_layers` coInt `1`, min `1`).
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `use_brim_efc_outline` (named non-borrow: `brim_use_efc_outline` is out of scope here, and this tree's brim is bbox-derived).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — the `elefant_foot_layers_density` solid-infill density arm (named non-borrow: that key is absent from this repo's gap source entirely — surfaced to wayfinder ticket 123).
- `OrcaSlicerDocumented/src/slic3r/GUI/ConfigManipulation.cpp` — the `> 1` mm clamp (named non-borrow: GUI hint, not slicing validation, per the ticket-113 rule).

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

- Positive: `AC-1` through `AC-11`. Refinements not stated in their Given/When/Then text: AC-2's `min_contour_width = 0.757` is `0.4 + line_width_to_spacing(0.4, 0.2)` — a 0.4 mm bead at 0.2 mm layer height, canonical's own sanity case, already pinned by `slicer_core::flow`'s `classic_0p4mm_bead_0p2mm_layer` test. AC-4's rib is 2 mm wide so that it sits above the tiny-contour early-out of AC-3 but below `min_contour_width + 2 * compensation`-driven full compensation, which is the only band where the width limiter is observable.
- Negative: `AC-N1` (non-positive flow spacing is fatal, never silently substituted), `AC-N2` (manifest `min = 1` rejects `elefant_foot_compensation_layers = 0`).
- Cross-packet impact:
  - **P88 / ticket 95 ordering obligation.** Canonical applies `_shrink_contour_holes` (the `xy_*_compensation` pair) to the same expolygons **before** `elephant_foot_compensation`. When P88 lands on this same `Layer::SlicePostProcess` stage, it must run first. This packet does not create that ordering mechanism and does not claim it; it records the obligation so P88 is authored against a known constraint rather than discovering it.
  - **Deviation row (re-derive the next free `DEV-###` at write time; `DEV-198` was next-free at authoring).** Three divergences: (a) **seam** — canonical compensates inside `slice_volumes` before region merge and `make_slices`, this port compensates at `Layer::SlicePostProcess`, after `PrePass::Slice` has committed and after `Layer::PaintRegionAnnotation`; the geometry is the same but the mutation point is later, so any future consumer of pre-compensation footprints must be authored against this seam, not canonical's; (b) **uncompensated store non-borrow** — `lslices_elfoot_uncompensated` has no port counterpart, which is what leaves `brim_use_efc_outline` unimplementable (no behaviour differs today because this tree's brim is bbox-derived); (c) **acceleration structure** — canonical's `EdgeGrid::Grid` is replaced by a file-private uniform segment grid at the same `0.7 * search_radius` cell size; results are expected to be equal within polygon tolerance, not bit-identical.
  - **No queue-count change.** The scoped target stays at 409. The five re-declared keys are existing keys reached at a new decision point, not queue keys claimed by this packet.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | New crate member and test targets compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Lint gate, required before committing | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate on the new `RoleWidthContext` test literals | FACT exit code |
| `cargo test -p slicer-core --test algo_elephant_foot_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Kernel: AC-1 through AC-4 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p elefant-foot --test elefant_foot_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Module behaviour: AC-5 through AC-8, AC-N1 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p elefant-foot --test slicer_module_binding_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Manifest and binding: AC-9, AC-N2 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo xtask build-guests --check; echo "exit=$?"` | AC-10 guest freshness; `0` fresh, `1` stale, `3` `wasm-tools` missing | FACT exit code — never grep for `STALE:` |
| `cargo test -p slicer-core --features host-algos --no-fail-fast 2>&1 \| tee target/test-output.log \| tail -20` | No regression in the feature-gated `slicer-core` suite the new ungated module shares a crate with | FACT binary count + pass/fail |
| `rg -q 'elefant_foot_compensation_layers.*wired' docs/spec_packets/303-elefant-foot-slice-postprocess/requirements.md && rg -q 'declaration-only keys: 0' docs/spec_packets/303-elefant-foot-slice-postprocess/requirements.md; echo "exit=$?"` | AC-11 disposition table | FACT exit code |

## Step Completion Expectations

- **The kernel lands before the module.** Steps 1–2 leave `slicer-core` compiling and tested with no module in the tree; the module in Steps 3–5 only ever calls an already-proven kernel. Do not interleave.
- **Ungatedness is a cross-step invariant.** Any step that touches `crates/slicer-core/src/algos/mod.rs` or `crates/slicer-core/Cargo.toml` must leave `elephant_foot` free of `#[cfg(feature = "host-algos")]` and its test target free of `required-features`. Re-check after every edit to either file; a regression here is silent (a clean-looking green run with zero tests compiled).
- **Guest freshness is a cross-step invariant from Step 3 onward.** Every step after the module directory exists must end with `cargo xtask build-guests --check` reading exit `0`, since `crates/slicer-core/**` and `modules/core-modules/*/src/**` both feed the guest build.
- **`min_contour_width` is derived once and shared.** Steps 4 and 5 both need `resolve_role_width(OuterWall, ..) + line_width_to_spacing(..)`; it is computed in one helper in `src/lib.rs`, not twice.

## Context Discipline Notes

- `crates/slicer-sdk/src/views.rs` and `crates/slicer-sdk/src/builders.rs` are both large. Read only the `SliceRegionView` accessor block and the `SlicePostprocessBuilder` impl respectively; never open either in full.
- `crates/slicer-wasm-host/test-guests/dispatch-layer-slice-postprocess-guest/src/lib.rs` is ~50 lines and is the single best read in the tree for the `RegionKey` construction shape. Read it in full; it is cheap and it is the precedent AC-8 pins.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` and `05-asset-packet-list.md` are both large assets. Locate the `elefant_foot` / `P86` rows with `rg -n` first, then open a +/-10-line window. Never read either in full.
- The `ElephantFootCompensation.cpp` dispatch is the packet's heaviest. Split it: one `SUMMARY` for the overall pass shape, then at most two `SNIPPETS` dispatches (30 lines each) for the resample-and-delta loop and for `smooth_compensation_banded`. Do not request the file.
