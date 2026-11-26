# Design: support-interface-spacing-and-loops

## Tier Derivation

**Tier B.** Authoring rule 1 requires a packet that builds a decision point to be re-tiered B or C. This packet builds one — the contact-loop pass behind `support_interface_loop_pattern` — inside two existing modules, using existing IR and an existing claim. It adds no claim, no IR field, no WIT change, and no host config field, so it does not reach C. The two spacing keys are A-grade plumbing corrections carried inside the B packet; they do not set the tier. The pre-rules packet 260's Tier A assignment is superseded, and the `04-asset-tier-assignment.md` row for ticket 18 needs the same correction (reported, not applied here).

## Controlling Code Paths

- `modules/core-modules/traditional-support/src/lib.rs` — `TraditionalSupport::from_config` (reads `support_interface_spacing`, `support_bottom_interface_spacing`, `support_interface_flow`; gains the loop-pattern read), `TraditionalSupport::pitches_mm` (the interface pitch/density derivation, including the `< 0.0 → top gap` mirror branch), `TraditionalSupport::run_support` (drives interface emission), `TraditionalSupport::fill_expolygon` (the scan-line filler the loop pass wraps).
- `modules/core-modules/tree-support/src/lib.rs` — the tree renderer's equivalents: `from_config`, `run_support`, `pitches_mm`, and `scan_fill_region` (**not** `fill_expolygon` — the two renderers named their per-expolygon filler differently; verified at authoring).
- `crates/slicer-core/src/support_regularize.rs` — `interface_density`, `bottom_interface_density`, `body_density`; read-only here (the formulas are already canonical-exact; this packet changes the *input default*, not the formula).
- `crates/slicer-gcode/src/serialize.rs` — `serialize_config_block`, `SUPPORT_CONFIG_DEFAULTS`, `ORCA_CONFIG_PADDING`. Read-only and asserted-against only. **No edit here is a deliverable** (Authoring rule 2).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Claims:** unchanged. The contact loops are `SupportInterface` extrusions emitted by the module that already holds `support-generator` and its `support-family:<id>` (`traditional-support` → `support-family:traditional`, `tree-support` → `support-family:tree`, per `docs/03_wit_and_manifest.md` § Support-family claims). This packet declares no new claim and repoints no holder.
- **Mechanism carrying the new data:** none is added. The loop pass reads the top-interface geometry already present in the `SupportPlanIR` layer entry the renderer consumes (the plan entry's top-interface geometry field) and writes into the same `SupportIR` output the renderer already writes. No prepass IR field, no `SliceRegionView` metadata, no `PostPass` claim, no manifest/SDK contract change — only two new `[config.schema]` entries (one net-new key plus a default change) inside manifests the modules already own.
- **Struct-literal / blast-radius:** no new struct field and no public schema or version constant moves, so the struct-literal churn gate (`cargo xtask check-literals`) has no new watched-type fallout. The blast radius that does exist is *test-expectation* fallout from the 0.4 → 0.5 default change; Step 2 owns it explicitly.

## Divergences Recorded (rule 4: improving on canonical is in scope)

1. **`-1 == mirror the top gap` on `support_bottom_interface_spacing`.** Canonical declares this key `min 0` with no sentinel; the `-1 == same as top` convention belongs to a different canonical key (`support_interface_bottom_layers`). The port's mirror is a usability improvement — it lets a user express "bottom follows top" without restating the value, which canonical cannot. Retained per user ruling, `min = -1.0` kept in both manifests, AC-3 is the behaviour witness and AC-6 keeps `-1.0` legal in the bounds index. Both manifests carry a comment naming the divergence so a later parity sweep does not "fix" it.
2. **Declared bounds `max = 2.0` on both spacing keys.** Canonical declares no max. Recorded as a declared-bounds divergence, deliberately unchanged: the cap is conservative, default-agnostic, and widening it has no queue backing.
3. **Contact loops share one implementation across both families.** Canonical reaches the tree family's interface toolpaths through a separate generator; the port can put the pass in `slicer-core` and call it from both renderers, so the two families cannot drift. Recorded as a structural improvement, not a gap.

## Code Change Surface

- `modules/core-modules/traditional-support/traditional-support.toml` — `[config.schema.support_interface_spacing]` default 0.4 → 0.5; net-new `[config.schema.support_interface_loop_pattern]` (bool, default false); divergence comment on `[config.schema.support_bottom_interface_spacing]`. `support_interface_pattern` is deliberately **not** added.
- `modules/core-modules/tree-support/tree-support.toml` — the same three edits.
- `modules/core-modules/traditional-support/src/lib.rs` — align the interface-spacing fallback constant and its comment to 0.5; read `support_interface_loop_pattern` in `from_config`; call the contact-loop pass in the top-interface branch of `run_support`, and hand `fill_expolygon` the trimmed area when a loop was emitted.
- `modules/core-modules/tree-support/src/lib.rs` — the same four edits, wrapping `scan_fill_region` rather than `fill_expolygon`.
- `crates/slicer-core/src/support_regularize.rs` (or a sibling module in the same crate) — net-new contact-loop helper shared by both renderers: given an interface `ExPolygon`, the interface line width, and a loop count, return the closed loop polyline(s) plus the trimmed fill area, yielding no loop when the inward offset empties the area.
- `modules/core-modules/traditional-support/tests/support_config_schema_tdd.rs` — net-new manifest guard (AC-1, AC-N1).
- `modules/core-modules/tree-support/tests/support_config_schema_tdd.rs` — net-new manifest guard (AC-N1).
- `modules/core-modules/traditional-support/tests/support_contact_loops_tdd.rs` — net-new (AC-4, AC-N2).
- `modules/core-modules/tree-support/tests/support_contact_loops_tdd.rs` — net-new (AC-5).
- `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` — AC-2 arms; re-measure any 0.4-pinned expectation.
- `modules/core-modules/tree-support/tests/tree_support_tdd.rs` — AC-3 arms; re-measure any 0.4-pinned expectation.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-6 arms.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — AC-7 arms.
- The `orca-matched-config.json` fixture and its consumer `support_family_closure.rs` — the 0.4 → 0.5 value.
- `docs/15_config_keys_reference.md` — regenerated output only, via `cargo xtask gen-config-docs`.
- `modules/core-modules/{traditional-support,tree-support}/Cargo.toml` — a `toml` dev-dependency for the manifest guards. Verified at authoring: neither crate has one today (both carry only `slicer-sdk` and `slicer-wasm-host` as dev-deps), so unless the guard parses the manifest through an existing helper, this add is required, not optional.

## Files in Scope (read + edit)

The Code Change Surface list above is authoritative. No file outside it may be edited.

## Read-Only Context

- `crates/slicer-core/src/support_regularize.rs` interface-density helpers (before the loop helper is added alongside them).
- `crates/slicer-gcode/src/serialize.rs` — `SUPPORT_CONFIG_DEFAULTS` / `ORCA_CONFIG_PADDING`, to confirm the AC-7 premise. Read only; never edited.
- `modules/core-modules/lightning-infill/**` — the shape to copy for a net-new module test file and manifest guard.
- `docs/03_wit_and_manifest.md` § Support-family claims; `docs/08_coordinate_system.md`.

## Out-of-Bounds Files

- `docs/spec_packets/260b-support-interface-fill-claim-holders/**` and every other packet directory.
- `docs/specs/orca-feature-gap/map.md` and `docs/specs/orca-feature-gap/issues/**` — the map and tickets are updated by the map's own workflow, not by this packet.
- `modules/core-modules/traditional-support-planner/**`, `modules/core-modules/tree-support-planner/**`.
- `crates/slicer-gcode/src/serialize.rs`, `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-schema/wit/**` — no contract or padding edit is in this packet.
- `OrcaSlicerDocumented/**` — delegated reads only.

## Expected Sub-Agent Dispatches

- `SUMMARY` (≤200 words): `LoopInterfaceProcessor::generate` in `Support/SupportCommon.cpp` — loop count semantics, the inward offset applied per loop, how the remaining fill area is trimmed, and what it returns when the area empties. Required before Step 3.
- `FACT` (≤5 lines): does `slicer_sdk::host::offset_polygons` (or its prelude-resolved form) accept a negative delta for the inward offset, and what does it return for an emptied area?
- `FACT` (≤5 lines): every test expectation in the two modules' test directories that pins `0.4` as the interface spacing default (file + test name), to size Step 2's blast radius.
- `FACT` (pass/fail): each cargo command in `requirements.md` §Verification Commands.

## Data and Contract Notes

- `support_interface_loop_pattern` is **bool**, not an enum, in canonical (`PrintConfig.cpp`). The name reads like an enum and a future worker may try to widen it; AC-1 and AC-N1 pin the type.
- The loop is a closed `SupportInterface` path: first point equals last point. AC-4 asserts closure explicitly because an open polyline around an island would pass a naive "one extra path" count.
- Interface line width is `line_width * resolved_interface_flow_ratio(interface_flow_percent) / 100.0` (both modules' `pitches_mm`); the loop pass must use that width, not the body `line_width`, or the trimmed area will be wrong at non-default `support_interface_flow`.

## Locked Assumptions and Invariants

1. Both renderers already receive top- and bottom-interface geometry per plan entry from `SupportPlanIR` — verified in-tree (`SupportPlanIR`'s layer entries carry top-, base-, and bottom-interface geometry in `crates/slicer-ir/src/slice_ir.rs`). No IR change is needed to reach the islands.
2. Both renderers already read the two spacing keys and derive the pitch from `slicer_core::support_regularize` — verified in `from_config` / `pitches_mm` in both `src/lib.rs` files.
3. Neither spacing key nor the loop key rides `SUPPORT_CONFIG_DEFAULTS` or `ORCA_CONFIG_PADDING` — re-derived from `crates/slicer-gcode/src/serialize.rs` at authoring (`SUPPORT_CONFIG_DEFAULTS` is `support_expansion` / `support_top_z_distance` / `support_bottom_z_distance`).
4. The generated deviations block in `docs/15_config_keys_reference.md` carried exactly two `support_interface_spacing` rows at authoring, inside a 26-row block (2026-09-01). The absolute count is a ledger fact and MUST be re-derived at implementation time; only the delta (-2) and the zero-remaining assertion are binding.
5. The interface emission path in both modules is single-threaded per layer, so a loop pass inserted before the scan fill cannot reorder other roles.

## Risks and Tradeoffs

- **The 0.4 → 0.5 alignment changes default output.** Default interface fill becomes slightly sparser in both families. This is intended and is the point of the alignment; the risk is a worker "fixing" a now-failing baseline by reverting the default. Step 2 requires each moved expectation to be re-measured with the new value recorded, never relaxed.
- **Contact loops change support-interface path ordering when enabled.** Off by default, so no existing baseline moves; AC-5 pins `false`-path byte-identity to catch an accidental unconditional change.
- **Shared helper vs. duplicated pass.** Putting the loop helper in `slicer-core` costs a guest-fingerprint rebuild for both modules (both already depend on `slicer-core`) but prevents the two families drifting. Accepted; the alternative — two copies — is the drift canonical already suffers.
- **`support_interface_pattern` absence may read as a regression** to someone comparing against the pre-rules packet 260. AC-1/AC-N1 assert the absence deliberately, and `requirements.md` §Returned to Queue names the missing feature and the packet that carries it.

## Context Cost Estimate

M. Two module `src/lib.rs` files (ranged reads only), two manifests, one `slicer-core` helper, six test files, one fixture, one generated doc. Sum of per-step costs in `implementation-plan.md`; no step is L.

## Open Questions

- `[FWD]` Should the contact-loop count become configurable beyond canonical's 0/1 (canonical's `n_contact_loops` is an int the config can only set to 0 or 1)? This packet ships the canonical 0/1 semantics; a multi-loop extension would be a per-region override the claim system could carry later. Recorded so `260b` does not invent one.
- `[FWD]` The tier table's owner attribution for ticket 18's keys (`support-planner` → the two renderer modules) is a map-side correction this packet cannot apply; it is listed for ticket 18's closure.
