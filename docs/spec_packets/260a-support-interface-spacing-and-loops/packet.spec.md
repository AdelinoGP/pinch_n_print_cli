---
status: draft
packet: support-interface-spacing-and-loops
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/18-author-packet-p11-support-interface-support-planner.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P11, re-authored to the map's Authoring rules 1–7; split 260 → 260a/260b, 259a/b + 262a/b precedent)
context_cost_estimate: M
---

# Packet Contract: support-interface-spacing-and-loops

## Goal

Build the support **contact-loop** decision point in both support renderers (`traditional-support`, `tree-support`) so `support_interface_loop_pattern = true` emits a closed `SupportInterface` loop around each top-interface island and shrinks the scan-fill area beneath it (canonical `LoopInterfaceProcessor::generate`, dispatched from `generate_support_toolpaths` on `n_contact_loops`), and correct the two already-live interface spacing keys: align `support_interface_spacing` from the mis-derived 0.4 to the canonical 0.5 in both modules, and pin the retained PnP `-1 == mirror top` bottom-spacing sentinel as a tested divergence.

## Scope Boundaries

The packet touches the two support-family renderer modules that consume interface configuration — `traditional-support` and `tree-support` — in their TOML manifests, `src/lib.rs` (config read, `pitches_mm`, the interface emission path around `fill_expolygon`), and test directories; the scheduler bounds-enforcement arm and the runtime CONFIG_BLOCK arm (one test file each); the `orca-matched-config.json` fixture and its consumer `support_family_closure.rs`; and the generated `docs/15_config_keys_reference.md`.

It does **not** implement interface-fill pattern dispatch: `support_interface_pattern` is **not in this packet** — it is returned to the queue and carried by packet `260b-support-interface-fill-claim-holders`, which needs a claim seam that does not exist today. It does not touch the planners (`traditional-support-planner`, `tree-support-planner` — they emit `SupportPlanIR` and read no interface configuration), the per-filament config model, or `ORCA_CONFIG_PADDING` / any CONFIG_BLOCK twin (Authoring rule 2 — padding is never a deliverable here).

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — number and `a` suffix re-derived from disk at authoring time); ticket 05 (packet-list P11 membership); ticket 104 (support key renames — resolved; the interface spacing keys are unaffected).
- Ordering, not gating: earlier queue packets touch different modules — no same-module merge churn. Packet 238c (implemented) is the origin of the spacing-key wiring this packet aligns.
- Unblocks: `260b`'s pattern-dispatch work consumes this packet's interface emission seam; nothing else gates on it.
- Activation blockers: none. Tier **B** — the packet builds a decision point (contact loops); see `design.md` §Tier Derivation.

## Acceptance Criteria

- **AC-1. Given** the `traditional-support.toml` and `tree-support.toml` manifests, **when** each one's `[config.schema]` is parsed, **then** each carries `support_interface_spacing` (`type = "float"`, `default = 0.5`, `min = 0.0`, `max = 2.0`, `group = "Support"` — default aligned from 0.4), `support_bottom_interface_spacing` (`type = "float"`, `default = 0.5`, `min = -1.0`, `max = 2.0`, `group = "Support"` — `min` deliberately kept at -1.0 for the retained mirror sentinel), and `support_interface_loop_pattern` (`type = "bool"`, `default = false`, `group = "Support"`), each with a `display` string; and **neither** manifest declares `support_interface_pattern` (that key is 260b's, and declaring it here would be a declaration-only key under Authoring rule 1). | `cargo test -p traditional-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a support-family module run over a fixed square region with top-interface plan entries, **when** `support_interface_spacing = 1.2` is set explicitly, **then** the emitted top-interface path count is strictly less than the count at `support_interface_spacing = 0.2` and strictly less than the count with the key absent — the non-default value moves the interface pitch through `pitches_mm` / `slicer_core::support_regularize::interface_density`; **and** the absent-key run equals the explicit `0.5` run (the aligned default) and differs from an explicit `0.4` run (the pre-alignment value). | `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the same fixture with `support_interface_spacing = 0.8` over a region carrying bottom-interface plan entries, **when** `support_bottom_interface_spacing = 1.6` is set explicitly, **then** the bottom-interface path count is strictly less than the count at `support_bottom_interface_spacing = 0.2`; **and** with `support_bottom_interface_spacing = -1.0` the bottom-interface path count equals the `0.8` run (the retained PnP mirror sentinel — recorded divergence; canonical has no `-1` on this key) and differs from the count with the key absent (default 0.5). | `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a support-family module run over a fixed region with top-interface plan entries and default spacing, **when** `support_interface_loop_pattern = true` is set, **then** the layer's `SupportInterface` extrusions contain exactly one **closed** loop per top-interface island (first point equals last point; island count taken from the plan entry's top-interface geometry) which is absent at `false`, **and** the open scan-line count strictly decreases versus the `false` run because the loop's width is subtracted from the fill area — matching canonical `LoopInterfaceProcessor::generate` at `n_contact_loops = 1`. | `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** the same contact-loop fixture on the tree family, **when** `support_interface_loop_pattern = true` is set, **then** the tree renderer emits the same one-closed-loop-per-island shape with the same reduced scan-line count relation as AC-4 (canonical applies the loop processor to both families through `generate_support_toolpaths`), **and** at `false` the emitted paths are byte-identical to the pre-packet baseline captured in the same test file. | `cargo test -p tree-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** the scheduler's config bounds index loaded from the real `traditional-support.toml` manifest, **when** a CLI/sidecar value `support_interface_loop_pattern = "yes"` (non-bool), `support_interface_spacing = -0.5` (< min 0), or `support_bottom_interface_spacing = -2.0` (< kept min -1.0) is resolved, **then** resolution rejects the value with the standard bool `TypeMismatch` / numeric `OutOfRange` error instead of passing it through to `ConfigView`; and `support_bottom_interface_spacing = -1.0` resolves successfully (the sentinel stays legal). | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** a slice run over the support family, **when** the G-code CONFIG_BLOCK is emitted, **then** at defaults the block contains zero `support_interface_spacing` / `support_bottom_interface_spacing` / `support_interface_loop_pattern` lines (none of the three rides `SUPPORT_CONFIG_DEFAULTS`, which is `support_expansion` / `support_top_z_distance` / `support_bottom_z_distance` only, and none is added to `ORCA_CONFIG_PADDING` — `serialize_config_block` in `crates/slicer-gcode/src/serialize.rs`); with an explicit `support_interface_spacing = 0.8` the line `; support_interface_spacing = 0.8` appears exactly once; with an explicit `support_interface_loop_pattern = true` the line `; support_interface_loop_pattern = true` appears exactly once. | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** the module-key tables carry `support_interface_loop_pattern` under both the `traditional-support` and `tree-support` owner columns, and the generated deviations block contains **zero** `support_interface_spacing` data rows (it carried exactly 2 at authoring, one per module, inside a 26-row block measured 2026-09-01 — the absolute count is a ledger fact, so re-derive it before and after and assert the delta is exactly -2 rather than pinning an absolute number). | `cargo xtask gen-config-docs --check && rg -q 'support_interface_loop_pattern' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c 'support_interface_spacing')" = "0" ]; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** the manifest schema guard, **when** any of the three keys is removed from either manifest, or its `type`/`default`/`min`/`max` drifts from AC-1's exact table, or `support_interface_pattern` is added to either manifest, **then** the guard fails naming the offending key. | `cargo test -p tree-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a top-interface island whose area is smaller than the space one contact loop needs (an island narrower than twice the interface line width), **when** `support_interface_loop_pattern = true`, **then** the module emits no loop for that island and falls back to the plain scan fill without error and without emitting a zero-length or self-intersecting path — canonical's `LoopInterfaceProcessor::generate` likewise yields nothing when the inward offset empties the area. | `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo test -p traditional-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test support_contact_loops_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — both manifests and both `src/lib.rs` files are guest-fingerprint inputs (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/03_wit_and_manifest.md` — `[config.schema]` shape; delegated SUMMARY if a worker needs the contract.
- `docs/08_coordinate_system.md` — the contact-loop inward offset is a geometry distance; see `design.md` §Architecture Constraints.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for `support_interface_loop_pattern` (owner columns `traditional-support` and `tree-support`), updates the two `support_interface_spacing` default cells 0.4 → 0.5, and its generated deviations block loses the two `support_interface_spacing` rows. The doc has no per-module subheadings, so verification is key-presence + row-delta, not headings. Verification greps: `rg -q 'support_interface_loop_pattern' docs/15_config_keys_reference.md` plus the AC-8 deviation-block probe. The doc is generated — the edit lands through `cargo xtask gen-config-docs`, never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations (`support_interface_spacing` coFloat default 0.5 min 0; `support_bottom_interface_spacing` coFloat default 0.5 min 0 **with no -1 sentinel**; `support_interface_loop_pattern` **coBool** default false — the canonical type correction this packet records); authoring-time evidence is captured in `requirements.md` §Per-Key Canonical Evidence and is not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters(const PrintObject&)`: `top_interface_spacing = (ironing ? 0 : value) + flow.spacing()` and `top_interface_density = min(1, flow.spacing()/top_interface_spacing)`, the formulas `slicer_core::support_regularize` mirrors.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths` (`loop_interface_processor.n_contact_loops = config.support_interface_loop_pattern.value ? 1 : 0`) and `LoopInterfaceProcessor::generate` — **the generator this packet ports**: the loop count, the inward-offset step, how the remaining fill area is trimmed, and the empty-result path AC-N2 mirrors.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportMaterial::has_contact_loops`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::generate_toolpaths` (the tree family's interface spacing/density formula and its loop handling).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::invalidate_state_by_config_options` (support invalidation for these keys).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
