# Design: 267-support-ironing-gate

## Controlling Code Paths

- Primary module path: `SupportSurfaceIroning::from_config` in `modules/core-modules/support-surface-ironing/src/lib.rs` reads the gate key; `SupportSurfaceIroning::run_support_postprocess` returns early on `!self.enabled`. The scan-line generator `SupportSurfaceIroning::fill_expolygon` is untouched.
- Config path: the module manifest's `[config.schema]` is the pre-filtering input for the per-module `ConfigView` the host hands the module; `ConfigView::from_declared` drops undeclared keys, so the manifest table is what makes the key reachable at all. Per-module filtering is `crates/slicer-scheduler/src/execution_plan.rs`.
- Host boundary path: `crates/slicer-integrated-modules/src/lib.rs` embeds this manifest by path for the integrated edition; `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` drives both the native and wasm legs with a real config map.
- Generated docs path: `xtask/src/gen_config_docs.rs` reads module manifests to produce `docs/15_config_keys_reference.md`, including the deviation comparison that has covered booleans since ticket 100.
- OrcaSlicer comparison: see `requirements.md` section "OrcaSlicer Reference Obligations"; do not repeat delegation rules here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` section "Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its **exit code**: 0 means fresh, 1 means stale, 3 means `wasm-tools` is unavailable (an infrastructure error, not cleanliness). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing run prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Ticket 101 proved by byte-search that guest WASMs embed config key names, so a gate-key change is a guest input.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10^-4 mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm/unit boundary. Full porting checklist in `docs/08_coordinate_system.md`. This packet changes no geometry, so the rule binds only if a test fixture is extended with new coordinates.

- All runtime config key strings remain snake_case (`support_ironing`, never `support-ironing`).
- No WIT, IR, or schema-version change. The manifest's `min-ir-schema` / `max-ir-schema` window is unchanged, and no struct-literal blast radius is created (`cargo xtask check-literals` should stay green without waivers).
- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) is **out of bounds**: not read, not edited, not asserted. Map Authoring rule 2.
- The top and support `ironing_enabled` entries are independent declarations in separately filtered module views. This packet must not rewrite top-side behaviour, and must not assert `ironing_enabled`'s tree-wide absence — packet `266-top-surface-ironing-keys` removes the other half.

## Tier, Claims, and Carriers (map rules 1 and 4)

- **Tier: A, confirmed.** The decision point already exists — the module's `enabled` gate short-circuits `run_support_postprocess` — so this is plumbing a canonical key into a live decision point, not new logic (B) and not a new module at a new seam (C). The tier table's `support_ironing` row (Tier A, owner `support-surface-ironing`) survives re-derivation. The tier table's second P15 row, `support_ironing_pattern` (also Tier A), does **not**: it is holder work at a seam this port does not have, and the tier-table correction rides this packet's closure (see "Queue and tier-table effects").
- **Claims: unchanged.** `support-surface-ironing.toml` declares `[claims] holds = []` and `requires = []`; this packet neither adds nor removes a claim. Rule 4's holder-per-value shape is exactly why `support_ironing_pattern` is excluded rather than declared — and standing up `claim:support-ironing` plus a concentric filler is the work that exclusion names, not work this packet does.
- **`support_ironing` is not itself an algorithm enum.** It is a bool that turns one implemented behaviour on and off. Rule 4's trigger test (cross-module algorithm selection, not in-module branching) does not fire on it; it sits with `seam_position` and `support_style` in the in-module category, behind the module's existing single implementation.
- **Which existing mechanism carries the new data:** the module manifest `[config.schema]` table, into the per-module `ConfigView`. No typed `ResolvedConfig` field, no WIT accessor, no new IR field, no new module, no host special case.

## Recorded Divergences

`DIV-267-A` and `DIV-267-B` are design-local labels for this packet only. They are **not** `docs/DEVIATION_LOG.md` IDs and must not be searched for there; per ticket 02 a log row is filed only after the human has been asked and has signed off. Neither divergence is created by this packet — both are pre-existing, and both are named here because making the canonical key live is what makes them user-visible.

- **DIV-267-A — the port irons a different subject than canonical does.** Canonical captures `top_contact_layer.polygons_to_extrude()` — the support's **top contact (interface) layer** — and fills exactly those polygons at `erIroning`. This port's module runs at `Layer::SupportPostProcess` and its trait entry receives only `_layer_index`, `&[SliceRegionView]`, a `SupportOutputBuilder`, and a `ConfigView`; it scan-fills every slice-region polygon it is handed and pushes the result as support paths. There is no support-interface geometry at this seam to select instead — closing the gap means carrying support contact/interface polygons across the WIT boundary to a `SupportPostProcess` module, which is an IR plus WIT contract change. **Rationale for accepting it here:** P15 is the key's packet, not the seam's; changing the ironing subject would change output for every user who already sets `ironing_enabled`, under a packet whose whole claim is default-path identity. The divergence is bounded, named, and graduated to the map's fog so a geometry packet can pick it up.
- **DIV-267-B — canonical's interface precondition is not expressible here.** Canonical reaches the ironing capture only inside the `top_interfaces` arm of `generate_support_toolpaths`, i.e. only when top interface layers were requested; with no interface layers the contact layer is merged into the base and never ironed. The port's module cannot see the interface-layer count (it would have to co-declare an interface key and, worse, would still be applying it to the wrong subject per `DIV-267-A`). Accepted as a consequence of `DIV-267-A`, not as an independent decision: it closes when that one does.

## Code Change Surface

- Selected approach: a one-key gate migration. The manifest's `[config.schema.ironing_enabled]` table becomes `[config.schema.support_ironing]` with canonical type and default; `from_config`'s single `config.get("ironing_enabled")` becomes `config.get("support_ironing")` with the same absent-means-`false` fallback; the struct field and its getter follow the key name. Everything downstream of the gate is unchanged.
- Because the canonical default (`false`) equals the current default and the current absent-key behaviour, **the default path is byte-identical**: a slice that does not set the key produces the same G-code before and after. The behaviour change is at `true`, and the reachability change is that an OrcaSlicer configuration can now express it.
- Exact functions, manifests, tests, and fixtures:
  - `SupportSurfaceIroning::from_config` and the `enabled` field/getter in `modules/core-modules/support-surface-ironing/src/lib.rs` (the module's own `#[cfg(test)] from_config_defaults` unit test travels with it).
  - `[config.schema]` in `modules/core-modules/support-surface-ironing/support-surface-ironing.toml`.
  - `modules/core-modules/support-surface-ironing/Cargo.toml` dev-dependencies (add `toml` if absent).
  - New `modules/core-modules/support-surface-ironing/tests/support_ironing_config_schema_tdd.rs`.
  - `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` and `tests/ironing_scanline_parity_tdd.rs`.
  - `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs`.
  - Generated `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons:
  - **Keep `ironing_enabled` as an OR-ed fallback alongside `support_ironing`.** Rejected by Authoring rule 5 and grilling ruling Q10(b): two gates for one decision is precisely the outcome the rule forbids, and it would leave the canonical key non-authoritative. The deliberate back-compat break is the same class as ticket 107's collapses and Q14(b)'s `skirt_brim_enabled` retirement.
  - **Add `support_ironing` to `CONFIG_KEY_ALIASES` so old profiles keep working.** Rejected: the alias table is host-side and holds two deliberate, documented entries; the map's standardise-to-Orca ruling eliminates alias maps rather than growing them (ticket 07), and Q3(c) settled that the port has no opinion on keys it does not implement.
  - **Declare `support_ironing_pattern` with a recorded gap so P15 covers both scoped keys.** Rejected by Authoring rule 1 and rule 4 — that is the exact "declared-with-gap" disposition the map's retroactive re-authoring pass removed from packets 253-266. The key returns to the queue instead.
  - **Also fix the ironing subject to canonical's top contact layer in this packet.** Rejected: it needs support-interface geometry at the `SupportPostProcess` seam, which is an IR plus WIT change and Tier B/C geometry work. Recorded as `DIV-267-A` and graduated to the map's fog.
  - **Also rename `ironing_speed` to `support_ironing_speed` while in this manifest.** Rejected as out of scope: it is a PnP naming fix with no canonical key, it carries an unresolved membership question for `SPEED_KEYS` (`crates/slicer-ir/src/feedrate.rs`; the grilling ruling Q11(a) calls it `FEEDRATE_KEYS`, a name that does not exist in this tree), and folding it in would put an unadjudicated decision inside a packet whose acceptance is about a different key.

## Files in Scope (read + edit)

- `modules/core-modules/support-surface-ironing/support-surface-ironing.toml` - owner manifest; replace the gate table.
- `modules/core-modules/support-surface-ironing/src/lib.rs` - the gate key read, the field name, and the module's own defaults unit test.
- `modules/core-modules/support-surface-ironing/Cargo.toml` - add the TOML parser dev-dependency if absent.
- `modules/core-modules/support-surface-ironing/tests/support_ironing_config_schema_tdd.rs` - new manifest guard; auto-discovered by Cargo.
- `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` - key migration plus the AC-2 off/absent arms and AC-N1.
- `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs` - key migration in its config fixture.
- `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` - config-map key migration (AC-3).
- `docs/15_config_keys_reference.md` - generated output only; changed through `cargo xtask gen-config-docs`.

## Read-Only Context

- `modules/core-modules/support-surface-ironing/src/lib.rs` - `from_config`, the struct fields and getters, and `run_support_postprocess`'s early return only; the scan-line generator is not needed.
- `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` - only to confirm the two `ironing_enabled` declarations are independent; never edited.
- `crates/slicer-sdk/src/traits.rs` - the `run_support_postprocess` signature only, to confirm no support-interface geometry is available at this seam (`DIV-267-A`).
- `crates/slicer-integrated-modules/src/lib.rs` - the embedded-manifest path entry for this module only.
- `docs/03_wit_and_manifest.md` - targeted ranges or delegated summaries only.
- `OrcaSlicerDocumented/...` - delegated canonical inspection only.

## Out-of-Bounds Files

- `modules/core-modules/top-surface-ironing/**` - packet 266's surface.
- `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs`, `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`, `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs`, `resources/test_config/benchy_combined_feature_evidence.json` - top-owned fixtures; packet 266 migrates them.
- `crates/slicer-gcode/src/serialize.rs` - `ORCA_CONFIG_PADDING` and CONFIG_BLOCK twins; map rule 2.
- `crates/slicer-ir/src/resolved_config.rs`, `docs/config/host-keys.toml`, `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - the key is module-owned; no host row.
- `crates/slicer-schema/wit/**` and generated bindings - no boundary change.
- `docs/ORCA_CONFIG_REFERENCE.md` - hand-maintained upstream snapshot, untouched.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, unrelated crates - never load directly.

## Expected Sub-Agent Dispatches

- Question: is `toml` absent from `support-surface-ironing`'s dev-dependencies, and does the module use Cargo test auto-discovery with no explicit `[[test]]` entries?; scope: that module's `Cargo.toml` and `tests/`; return: `FACT`; purpose: Step 1.
- Question: quote the exact gate read and the surrounding field initialisation in `SupportSurfaceIroning::from_config`, plus every `ironing_enabled` occurrence in the module's two test binaries; scope: `modules/core-modules/support-surface-ironing/src/lib.rs` and `tests/`; return: `LOCATIONS`; purpose: Step 2.
- Question: quote the config-map construction in the support integrated-parity contract test; scope: `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs`; return: `SNIPPETS` (1, <=20 lines); purpose: Step 3.
- Question: confirm canonical `support_ironing`'s type/default and the `generate_support_toolpaths` gate and fill blocks; scope: sibling `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented\src\libslic3r\PrintConfig.cpp`, `Support\SupportParameters.hpp`, `Support\SupportCommon.cpp`; return: `LOCATIONS`; purpose: parity evidence.

## Data and Contract Notes

- Manifest contract: a `bool` schema table carries `type`, `default`, `display`, and `group`; it carries no `min`/`max`, and `ConfigBoundsIndex` applies no numeric bound to it (`crates/slicer-scheduler/src/config_resolution.rs` returns `Ok` for `ConfigValue::Bool`). That is why this packet has **no** scheduler bounds AC: inventing one for a bool would be a check that cannot fail, which the map's rule 6(b) would not accept as evidence anyway.
- Reachability contract: `ConfigView::from_declared` drops undeclared keys, so the manifest edit is what makes `support_ironing` visible to the module; the module edit is what makes it load-bearing. AC-1 and AC-2 pin the halves separately, and AC-3 pins the join through the real host path.
- Deviation contract: `cargo xtask gen-config-docs` compares manifest defaults against the upstream snapshot's `Default` column, and has compared booleans since ticket 100's `num_of` fix. Canonical `false` equals the declared `false`, so the deviation block's row count must be unchanged — **re-measure it, do not quote a number from this packet** (map rule: ledger facts are re-derived at point of use).
- Determinism: nothing in this packet touches emission order or geometry; the existing scan-line parity suite is the guard that the migration did not perturb output.

## Locked Assumptions and Invariants

- `support_ironing` default `false` is canonical and is the module's only gate after this packet.
- Absent key means `false`, matching the current `ironing_enabled` behaviour, so unset configurations are byte-identical before and after.
- No support-module code path reads `ironing_enabled` after this packet; top-side reads remain untouched until packet 266 lands.
- The scan-line generator, flow, spacing, speed, and line width are unchanged; only the gate's key changes.
- No WIT/IR/schema-version change occurs, so there is no struct-literal blast radius.
- `support_ironing_pattern` is declared nowhere, in this packet or the tree (AC-N2).

## Risks and Tradeoffs

- **Deliberate back-compat break.** A profile that enabled support ironing via `ironing_enabled` silently stops ironing supports after this packet (the key falls into `extensions`). That is the intended consequence of the standardise-to-Orca ruling and of Q10(b) — and it is also the fix for the inverse surprise, where a user enabling top-surface ironing got support ironing too. Same class as Q14(b)'s `skirt_brim_enabled` retirement. Called out here so a reviewer does not read it as an accident.
- **Making the key live makes `DIV-267-A` reachable.** Users who set `support_ironing = true` will get the port's existing (non-canonical) ironing subject. The packet does not claim canonical geometry parity for support ironing — only key parity — and says so in its evidence table.
- **Half-migrated `ironing_enabled`.** Between this packet and packet 266, the tree carries the key on one module only. Any check that asserts its tree-wide absence will fail; both packets scope their assertions to their own side.
- Manifest and module source feed the guest artifact; a stale guest makes otherwise correct contract tests fail in ways that look unrelated.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S` (Step 2, the gate read and its invariants)
- Highest-risk dispatch and required return format: canonical `support_ironing` gate evidence, `LOCATIONS`; the module's existing test-fixture occurrences, `LOCATIONS`.

## Open Questions

- `[FWD]` `DIV-267-A` closes only when support contact/interface geometry reaches a `SupportPostProcess` module — an IR field plus a WIT accessor. Named here so a future geometry packet does not re-derive the cost. Graduated to the map's "Not yet specified" at this packet's closure.
- `[FWD]` `support_ironing_pattern` returns when a support-ironing claim seam exists (holder key plus at least a `rectilinear` holder, and a concentric filler for the second canonical value). That is Tier C, and it should be scoped together with packet `260b-support-interface-fill-claim-holders`, which is standing up the neighbouring support-interface filler claim seam.
- `[FWD]` Grilling ruling Q11(a) renames this module's `ironing_speed` to `support_ironing_speed` and leaves its `SPEED_KEYS` (`crates/slicer-ir/src/feedrate.rs`) membership open — the ruling names the table `FEEDRATE_KEYS`, which is not a symbol in this tree. It touches the same manifest; whoever sequences it should expect a trivial merge with this packet, not a conflict of substance.

**No `[BLOCK]`.** The packet needs no new WIT interface, no IR schema bump, no host `ResolvedConfig` field, and no new module.
