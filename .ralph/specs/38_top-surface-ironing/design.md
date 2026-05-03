# Design: top-surface-ironing

## Controlling Code Paths

- Primary code path:
  - `modules/core-modules/top-surface-ironing/` (NEW directory; mirror `support-surface-ironing/` skeleton).
  - `crates/slicer-host/src/gcode_emit.rs` — confirm `ExtrusionRole::Ironing` → `;TYPE:Ironing` mapping (FACT; add line if missing).
- Neighboring tests or fixtures:
  - `modules/core-modules/support-surface-ironing/` — template reference; read structure only via SUMMARY.
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append `benchy_gcode_contains_ironing_evidence`.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing`.

## Architecture Constraints

- **Module-only packet.** No host changes (except possibly one line in `gcode_emit.rs`).
- **Transform-chain ordering** via `[ir-access].reads = ["InfillIR.regions"]` and `[ir-access].writes = ["InfillIR.regions"]`. Per `docs/04 §Composable Multi-Writer Patterns`, this establishes an A→B edge (fill module → ironing module) deterministically.
- **Topmost-layer detection** uses 12-rev1's `is_top_surface` AND 35's `top_solid_layers` (via `RegionMapIR`/config-view). The module's runtime check: this region is the *topmost* of its top-solid stack iff `is_top_surface == true` AND no further layer above this region has the region polygon overlap (or, simpler: there is no `is_top_surface == true` flag on the same region at layer N+1, which is equivalent for non-stepped objects).
- **Append-only output**: ironing paths are appended to `solid_infill`; existing `TopSolidInfill` paths are preserved unchanged. The fill module's output is the first stroke; ironing's output is the second stroke at same Z, low flow.

## Code Change Surface

- Selected approach:
  - Mirror the existing `support-surface-ironing` module's directory layout (Cargo.toml, manifest.toml, src/lib.rs, optional wit-guest).
  - In `src/lib.rs`, implement the `Layer::InfillPostProcess` callback:
    1. Read `InfillIR.regions` via the SDK view.
    2. For each region, check `is_top_surface == true` from `SliceRegionView`.
    3. Determine "is topmost layer of this region" using the SDK's accessors. Simplest reliable form: filter `solid_infill` paths to those with `role == TopSolidInfill`; if there are any, the layer is part of the top stack. Then check whether the next-higher layer also has `TopSolidInfill` for this same `(object_id, region_id)` — if not, this is the topmost layer. The implementation reads the `SliceRegionView` flag for the layer above via the SDK's per-layer view (delegate to confirm SDK provides this; otherwise treat the topmost layer as "any layer with `TopSolidInfill` paths whose `is_top_surface` is true and `top_solid_layers > 0`" for simplicity, then refine).
    4. If not topmost or `ironing == false` → continue.
    5. Compute the bounding ExPolygon of the `TopSolidInfill` paths.
    6. Generate a rectilinear zigzag at `ironing_spacing`, oriented orthogonal to the top-surface fill direction (use 0° default; refine with parity SUMMARY).
    7. Emit each stroke as an `ExtrusionPath` with `role = ExtrusionRole::Ironing`, `flow_factor = ironing_flow`, `speed_factor` derived from `ironing_speed` / region default.
    8. Append to `InfillIR.regions[i].solid_infill` via the SDK output builder.
  - Preserve existing input paths (the SDK's transform-chain pattern handles this when `writes` declares the same path the module reads — verify via SUMMARY of `docs/04 §Composable Multi-Writer Patterns`).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/top-surface-ironing/Cargo.toml` (NEW) — declare crate + SDK dependency.
  - `modules/core-modules/top-surface-ironing/manifest.toml` (NEW) — `stage = "Layer::InfillPostProcess"`, `[ir-access]`, `[claims]` (no claims declared — append-only modules do not need a claim), config keys.
  - `modules/core-modules/top-surface-ironing/src/lib.rs` (NEW) — module implementation.
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append `benchy_gcode_contains_ironing_evidence`.
  - `crates/slicer-host/src/gcode_emit.rs` — verify (FACT) and add `ExtrusionRole::Ironing` → `;TYPE:Ironing` mapping if missing.
  - Central config schema — add the 5 new config keys with defaults.
  - Workspace `Cargo.toml` — add `top-surface-ironing` to workspace members (if needed).
  - `./modules/core-modules/build-core-modules.sh` — verify the script auto-discovers the new module directory; if not, add it explicitly.
- Rejected alternatives that were considered and why they were not chosen:
  - **Inline ironing inside `rectilinear-infill`** — rejected: ironing is a distinct print operation with separate config and timing; mixing pollutes the fill module's responsibility.
  - **Ironing as a host built-in** — rejected: would require host-side path generation, breaking the module-extensibility model.
  - **Ironing in PostPass** — rejected: ironing happens between fill and travel within a single layer; PostPass is too late.

## Files in Scope (read + edit)

Primary edit targets (≤ 3 per step; aggregate ≤ 5 across the packet):

- Step "Module skeleton": new `Cargo.toml` + `manifest.toml` + `src/lib.rs` (3 files; one step).
- Step "Tests": new `tests/top_surface_ironing_emission_tdd.rs` + Benchy E2E append (2 files; one step).
- Step "Verify gcode_emit mapping": at most 1 file (`gcode_emit.rs`); FACT-conditional.
- Step "Workspace + build script": `Cargo.toml` + maybe build script (1-2 files).

## Read-Only Context

- `modules/core-modules/support-surface-ironing/` — read directory structure via SUMMARY; identify the manifest pattern and SDK call shape.
- `docs/05_module_sdk.md` — read directly (delegate SUMMARY for sections > 100 lines).
- `docs/04_host_scheduler.md` — § "Composable Multi-Writer Patterns" via SUMMARY.
- `crates/slicer-host/src/gcode_emit.rs` — only the `ExtrusionRole` → marker map (FACT-narrowed range).
- `crates/slicer-sdk/src/views.rs` — public API only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate only.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` — out of scope.
- `wit/` — no WIT changes.
- All other crates not listed in the change surface.

## Expected Sub-Agent Dispatches

- "Summarize `modules/core-modules/support-surface-ironing/` directory structure and module-skeleton pattern in ≤ 200 words. Return SUMMARY." — purpose: validate Step 1 module skeleton.
- "Does `crates/slicer-host/src/gcode_emit.rs` map `ExtrusionRole::Ironing` to `;TYPE:Ironing`? Return FACT yes/no with file:line." — validate Step 2.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing` algorithm + default values for `ironing_spacing`, `ironing_flow`, `ironing_speed` in ≤ 200 words. Return SUMMARY." — validate Step 0.
- "Run `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd`; return FACT pass/fail per test." — validate Step 4.
- "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail with failing module name on fail." — validate Step 3.
- "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence`; return FACT pass/fail." — validate Step 5.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `InfillIR.regions[*].solid_infill` — transform-chain consumer + producer.
  - No schema-version bump (this packet adds a module that uses existing IRs and an existing `ExtrusionRole::Ironing` enum variant).
- WIT boundary considerations: none (the module uses existing WIT/SDK types).
- Determinism or scheduler constraints:
  - Transform-chain edge fill→ironing established by `reads ∩ writes`.
  - Output order deterministic: ironing paths appended after fill paths within each region.

## Locked Assumptions and Invariants

- `ExtrusionRole::Ironing` enum variant already exists (`crates/slicer-host/src/wit_host.rs:2572` confirms it does).
- `support-surface-ironing` module exists as a working template.
- The `Layer::InfillPostProcess` stage already runs on the live path (it's used by other modules; verify via SUMMARY of the dispatch list in `docs/04`).

## Risks and Tradeoffs

- **Topmost-layer detection is the trickiest part.** Without packet 35, every layer in a multi-layer top-solid stack would get ironed, which is wrong. Mitigation: this packet declares packet 35 as a hard prerequisite. The module reads the `top_solid_layers` config and only emits ironing on the highest layer of the stack.
- **Path ordering inside a region.** The append model assumes the host preserves the fill paths and runs ironing strokes after them. Verify via the transform-chain SUMMARY.
- **Default values may differ from Orca.** Delegate FACT for Orca defaults; document any chosen-different-from-Orca values in `docs/DEVIATION_LOG.md` if applicable.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 1: module skeleton + first implementation).
- Highest-risk dispatch: WASM rebuild — FACT-only return.

## Open Questions

- Step 0 dispatch resolves: Orca defaults for `ironing_spacing`, `ironing_flow`, `ironing_speed`. If different from our chosen defaults (`0.2`, `0.15`, `15.0`), reconcile.
- Step 2 dispatch resolves: is `ExtrusionRole::Ironing` already mapped to `;TYPE:Ironing` in `gcode_emit.rs`? If yes, no change; if no, one-line addition.
- The "topmost layer" detection mechanism — depending on what the SDK exposes via `SliceRegionView` for adjacent-layer lookups, the implementation may be simpler or require a small SDK accessor addition. Step 1 dispatches a FACT to clarify.
