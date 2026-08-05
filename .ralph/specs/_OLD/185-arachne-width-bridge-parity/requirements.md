# Requirements: 185-arachne-width-bridge-parity

## Packet Metadata

- Grouped task IDs: `TASK-303`, `TASK-212b`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Width/flow resolution in PnP is not role-aware at OrcaSlicer parity. Five core modules (classic-perimeters, arachne-perimeters, rectilinear-infill, gyroid-infill, lightning-infill) each hand-resolve widths from a partial key set, with no shared first-layer/bridge context, no percent transport on the live path, and a non-canonical key name (`first_layer_line_width` where canonical spells `initial_layer_line_width`). Concretely: `parse_percent_default` in `crates/slicer-scheduler/src/manifest.rs` parses `ConfigValue::Percent`/`FloatOrPercent` and then discards the value at both `parse_config_field_entry` call sites (TASK-303, percent-transport gap residual ii), so no live slice can carry a percent-typed width; classic-perimeters still insets its final infill boundary by a raw `-inner_wall_line_width` instead of the canonical `process_classic` formula (classic final-infill-boundary gap residual); and `object_metadata_to_config_data` in `crates/slicer-model-io/src/loader.rs` drops part-level width keys required by TASK-212b before they can reach module config.

This packet supersedes packet 184's residuals (`184-classic-perimeter-flow-parity`, implemented). Packet 184 retyped classic's wall-width keys to `float_or_percent` and documented three forward residuals — the parser discard (TASK-303 / wall-width/percent-transport residual), the absent-key default divergence (`[FWD-1]`), and the final-infill-boundary offset — all absorbed here; 184's own files are not edited (the orchestrator flips its status). It is one coherent slice because the resolver, the transport that feeds it, and the key rename that names it are useless without each other: percent values need the extensions channel to reach modules, modules need the shared resolver to interpret first-layer/bridge/auto precedence uniformly, and the rename must land in the same parsing pass that introduces the alias.

## In Scope

- New shared resolver `resolve_role_width` in `crates/slicer-core/src/flow.rs` (alongside `line_width_to_spacing`, `flow_to_width`, `bridging_flow`), keyed by explicit canonical `ExtrusionRole` plus first-layer/bridge context. Precedence: configured `bridge_line_width`; else positive `initial_layer_line_width` on first layer; else selected role width; zero falls back to `line_width`, then auto (`0` = auto sentinel, canonical `Flow.cpp::auto_extrusion_width`, `1.125 × nozzle_diameter`). Geometric widths/spacing only.
- Consumption of `resolve_role_width` by all five modules: classic-perimeters, arachne-perimeters, rectilinear-infill, gyroid-infill, lightning-infill.
- Key rename: `ResolvedConfig` field `first_layer_line_width` (`crates/slicer-ir/src/resolved_config.rs`) → `initial_layer_line_width`; scheduler accepts a schema-aware alias for legacy profiles spelling `first_layer_line_width` and REJECTS profiles specifying both keys.
- Canonical defaults move to auto (`0`) including global `line_width`; explicit `0.4` remains explicit.
- Percent/`FloatOrPercent` transport: `ConfigValue::Percent` and `ConfigValue::FloatOrPercent { value, is_percent }` (`crates/slicer-ir/src/slice_ir.rs`) preserved through `ResolvedConfig.extensions` (`BTreeMap<String, ConfigValue>`) via schema-aware parsing in `crates/slicer-scheduler/src/manifest.rs` `read_config_schema` and `crates/slicer-scheduler/src/config_resolution.rs`. Closes TASK-303 and the wall-width config-type gap.
- Classic-perimeters parity fixes: port canonical `PerimeterGenerator.cpp::process_classic` final-infill-boundary formula replacing the raw `-inner_wall_line_width` offset in `modules/core-modules/classic-perimeters/src/lib.rs` (closes classic final-infill-boundary gap); topmost `only_one_wall_top` (`top_shell_index == Some(0)`, `crates/slicer-sdk/src/views.rs`) unconditionally forces one wall while the `min_width_top_surface` threshold applies to non-topmost top sub-areas, implemented module-locally following `emit_only_one_wall_top_second_pass` (`modules/core-modules/arachne-perimeters/src/lib.rs`); generic `split_top_surfaces` (`crates/slicer-core/src/top_surface_split.rs`) preserved unchanged.
- Classic overlap selection: `top_bottom_infill_wall_overlap` for layer 0 and regions with `top_shell_index == Some(0)`; `infill_wall_overlap` otherwise.
- Classic overlap transport: `classic-perimeters.toml` owns `infill_wall_overlap` (`percent`, default `15`, ratio base `inner_wall_line_width`) and `top_bottom_infill_wall_overlap` (`percent`, default `25`, ratio base `inner_wall_line_width`); Classic reads the selected key through `ConfigView::get_abs_value`.
- Module-owned manifest keys: each of the five module manifests declares the canonical snake_case flow keys it consumes (`line_width`, `initial_layer_line_width`, `bridge_line_width`; plus role widths `outer_wall_line_width`/`inner_wall_line_width` on perimeters and `sparse_infill_line_width`/`internal_solid_infill_line_width`/`top_surface_line_width` on the infill modules that consume them). No central schema contract, no `schema_imports`, no host injection.
- ADR amendments: add `ADR-0043 amendment` to `docs/DEVIATION_LOG.md`, quoting ADR-0043's locked “plain mm floats, default 0.4, range [0.1, 2.0]” wall-width clause and recording the intentional canonical-parity change to `float_or_percent` with auto-`0`; add `ADR-0014 amendment`, quoting ADR-0014's stale “slicer-core and slicer-helpers are explicitly NOT tracked” freshness rule and recording the current `slicer-core` guest-input rule.
- ADR amendment records: append the approved amendment decisions to `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` and `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md`, each naming its deviation ID and preserving the original decision text.
- Part-level metadata allowlisting: modifier-part metadata is ingested generically into `ModifierVolume.config_delta.fields` in `crates/slicer-model-io/src/loader.rs` (every part key survives via `coerce_string_to_config_value`, including `inner_wall_line_width`, `outer_wall_line_width`, and `sparse_infill_line_width`); `object_metadata_to_config_data` additionally allows the same three keys at object scope; `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` extends the sidecar regression with a production-loader-path test (`part_width_keys_survive_in_config_delta_fields`) asserting the three width keys land in `config_delta.fields`.
- Existing test targets receive the new cases; no new test binary or aggregator is introduced. The targets are `crates/slicer-core/tests/flow_tdd.rs`, `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs` under the `scheduler_integration` aggregator, `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`, `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs`, and `crates/slicer-runtime/tests/integration/per_object_config_override_tdd.rs` under the `integration` aggregator.
- Doc edits: new DEV-102 row and status updates to the classic final-infill-boundary gap, wall-width config-type gap, and percent-transport gap in `docs/DEVIATION_LOG.md`; TASK-303/TASK-212b checkbox flips in `docs/07_implementation_status.md`.

## Out of Scope

- Canonical top/bottom/internal-solid flow-**ratio** controls (material-volume parity) — deferred, recorded as new deviation DEV-102 in `docs/DEVIATION_LOG.md`. Do not implement ratios.
- D-152-TOP-AREA-SOURCE (`top_solid_fill` vs canonical `upper_slices`) stays OPEN; this packet must not claim its closure.
- DEV-101 (`min_width_top_surface` percent-base disagreement between the two perimeter generators) stays OPEN; not this packet's reconciliation.
- Any central config-schema contract, `schema_imports` mechanism, or host-side injection of flow keys into modules.
- Nozzle-diameter/extruder model changes, WIT schema changes, speed/feedrate parity, and any golden/baseline re-bless not required by the behavior changes above (see §Step Completion Expectations for ordering).
- Editing packet 184's spec files or flipping its status (orchestrator's job).

## Authoritative Docs

- `docs/DEVIATION_LOG.md` — long, huge rows; delegated ranged reads only. Relevant rows: classic final-infill-boundary gap, D-152-TOP-AREA-SOURCE, wall-width config-type gap, percent-transport gap, DEV-101. Re-derive the next free deviation ID at the moment of the DEV-102 edit (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`).
- `docs/07_implementation_status.md` — long; delegated ranged reads only (TASK-303 entry, TASK-212b entry).
- `docs/08_coordinate_system.md` — direct read of the porting-checklist section only; 1 unit = 100 nm (10⁻⁴ mm), NOT OrcaSlicer's 1 nm — divide canonical constants by 100.
- `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` — read the normative wall-width clause in `Decision` item 2 and append the `ADR-0043 amendment` decision record.
- `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — read the normative freshness list around line 27 and append the `ADR-0014 amendment` decision record.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::new_from_config_width` and `Flow::auto_extrusion_width`: role-keyed width resolution, the `0` auto sentinel, and the `1.125 × nozzle_diameter` auto width being ported into `resolve_role_width`.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic`'s final-infill-boundary inset formula (replaces the raw `-inner_wall_line_width` offset) and its topmost `only_one_wall_top` handling.
- `OrcaSlicerDocumented/src/libslic3r/PrintRegion.cpp` and `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — region-level flow/overlap access feeding the `infill_wall_overlap` vs `top_bottom_infill_wall_overlap` selection branch.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef init_fff_params`: canonical defaults (`0`/auto) and `ratio_over` bases for `line_width`, `initial_layer_line_width`, `sparse_infill_line_width`, `internal_solid_infill_line_width`, `top_surface_line_width`, `bridge_line_width`; the top/bottom/internal-solid flow-ratio keys are deliberately NOT borrowed (DEV-102).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-16`. Measurable refinements: the AC-1 matrix must cover all nine named `ExtrusionRole` variants plus both first-layer and bridge contexts and both canonical top-overlap contexts (layer 0 and `top_shell_index == Some(0)`); AC-6's transport must be proven through `ResolvedConfig.extensions` specifically, not a side channel; AC-13 must prove the three part-level width keys reach `ModifierVolume.config_delta.fields` through the production `load_model` path (not merely the raw sidecar parse); AC-14 and AC-15 must prove the ADR amendments quote their contested clauses; AC-16 must prove the exact Classic overlap schema defaults and ratio base.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: supersedes packet 184's residuals (orchestrator flips 184's status); updates DEVIATION_LOG rows for the classic final-infill-boundary gap, wall-width config-type gap, and percent-transport gap, adds `ADR-0043 amendment` and `ADR-0014 amendment`, and creates the next free DEV row for flow-ratio deferral; flips TASK-303/TASK-212b in `docs/07_implementation_status.md`; gyroid-infill and lightning-infill gain a `slicer-core` dependency edge they do not have today.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands. All cargo test invocations tee to `target/test-output.log`; read the log, never re-run for more output.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask build-guests --check` | guest freshness gate (mandatory before attributing any module/dispatch failure) | FACT clean/STALE list |
| `set -o pipefail; cargo test -p slicer-core --all-targets --test flow_tdd 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | AC-1/2/3 precedence matrix, bridge, auto sentinel | FACT pass/fail + result line |
| `set -o pipefail; cargo test -p slicer-core --all-targets --test flow_tdd 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | existing flow fns unregressed | FACT pass/fail |
| `set -o pipefail; cargo test -p slicer-ir --all-targets --test resolved_config_defaults_tdd 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | AC-4/5 rename + auto-0 defaults | FACT pass/fail |
| `set -o pipefail; cargo test -p slicer-scheduler --all-targets --test scheduler_integration 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | AC-6 percent transport; AC-N1 both-keys rejection; AC-N2 no Float collapse | FACT pass/fail |
| `set -o pipefail; cargo test -p classic-perimeters --all-targets --test classic_perimeters_tdd 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | AC-9 inset formula; AC-10 topmost one-wall; AC-11 overlap selection; AC-16 schema | FACT pass/fail |
| `set -o pipefail; cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | AC-13 part-level width-key allowlist | FACT pass/fail |
| `set -o pipefail; cargo test -p arachne-perimeters --all-targets --test precise_outer_wall_tdd 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | module parity under shared resolver (AC-8 consumer) | FACT pass/fail |
| `set -o pipefail; cargo test -p rectilinear-infill --all-targets 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | module parity (AC-8 consumer; BottomSolidInfill role key) | FACT pass/fail |
| `set -o pipefail; cargo test -p gyroid-infill --all-targets 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | module parity (AC-8 consumer) | FACT pass/fail |
| `set -o pipefail; cargo test -p lightning-infill --all-targets 2>&1 \| tee target/test-output.log \| rg '^test result: ok'` | module parity (AC-8 consumer) | FACT pass/fail |
| AC-7/AC-8/AC-N4 rg loops (see `packet.spec.md`) | manifest keys, resolver consumption, no central schema | FACT per-module OK/MISSING lines |
| AC-12/AC-14/AC-15/AC-N3 doc greps (see `packet.spec.md`) | DEV-102, `ADR-0043 amendment`, and `ADR-0014 amendment` filed; classic final-infill-boundary gap/wall-width config-type gap/percent-transport gap updated; tasks flipped; D-152-TOP-AREA-SOURCE still Open | FACT pass/fail |
| AC-16 manifest grep (see `packet.spec.md`) | Classic overlap keys, percent defaults, and ratio base are declared | FACT pass/fail |
| `cargo xtask test --summary --workspace` (packet-close acceptance ceremony ONLY, delegated to a sub-agent) | closure gate per Test Discipline; runs only after every row above is green | FACT pass/fail + failing-test detail |

## Step Completion Expectations

- The parameterized precedence matrix (AC-1/2/3) and the percent-transport tests (AC-6, AC-N1, AC-N2) must land and pass BEFORE any golden/baseline re-bless. Any golden update made without those tests green is invalid and must be re-blessed after they pass.
- Run `cargo xtask build-guests --check` before attributing any guest, module-dispatch, or host-integration failure to this packet's changes; rebuild (drop `--check`) if STALE and re-run the failing test before concluding. `crates/slicer-core`, `crates/slicer-ir`, and `crates/slicer-scheduler` edits all invalidate guest artifacts.
- The `ResolvedConfig` rename (AC-4) is a breaking field change: land it in the same step as the scheduler alias, or the tree does not compile between steps.
- TASK-212b is not closed by a docs-only checkbox: the loader allowlist and its `threemf_sidecar_classification_tdd` regression must pass before the task row is checked.

## Context Discipline Notes

- `docs/DEVIATION_LOG.md` rows are individually enormous (single lines >5k chars); never full-read — ranged/delegated reads by row anchor only, and re-derive the next free DEV ID at edit time rather than trusting this packet's DEV-102 claim.
- Never load `OrcaSlicerDocumented/` directly; use the delegation contract in §OrcaSlicer Reference Obligations.
- Never read `target/`; test evidence comes from `target/test-output.log` grep/reads only.
- Cargo runs and doc fact-checks are delegated; sub-agent returns are FACT pass/fail with SNIPPETS ≤20 lines on failure. The workspace-wide ceremony run, when required at close, is dispatched per `.claude/skills/swarm/SKILL.md`, never absorbed.
