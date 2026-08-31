---
status: draft
packet: support-interface-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/18-author-packet-p11-support-interface-support-planner.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P11)
context_cost_estimate: M
---

# Packet Contract: support-interface-keys

## Goal

Declare the four OrcaSlicer support-interface keys (`support_interface_spacing`, `support_bottom_interface_spacing`, `support_interface_pattern`, `support_interface_loop_pattern`) with canonical types/defaults/bounds in both support-family module manifests (`traditional-support.toml`, `tree-support.toml`), align the mis-derived `support_interface_spacing` default from 0.4 to the canonical 0.5 in both modules (removing two doc-15 deviation rows), pin the retained PnP `-1` bottom-interface mirror as a tested divergence (user ruling), and declare the two zero-occurrence pattern keys with-gap (non-perturbing; their canonical decision points — the interface-fill pattern dispatch and the contact-loop processor — do not exist in this tree).

## Scope Boundaries

The packet touches the two support-family geometry modules that consume interface configuration — `traditional-support` and `tree-support` — in their TOML manifests, `src/lib.rs` defaults, and test directories; the scheduler bounds-enforcement arm and the runtime CONFIG_BLOCK arm (one test file each, mirroring packet 259's integration arms); the `orca-matched-config.json` fixture and its consumer `support_family_closure.rs`; and the generated `docs/15_config_keys_reference.md`. It does not introduce interface-fill pattern dispatch (`concentric`/`grid`/`rectilinear_interlaced` generators and canonical's density-dependent `auto` resolution — `SupportParameters::SupportParameters`' `contact_fill_pattern` branch order), the contact-loop generator (`LoopInterfaceProcessor`), the per-filament config model, or the planners' geometry (`support-planner` claim holders); those stay recorded gaps (queue rows). The `-1` bottom-interface mirror is retained per the user ruling of 2026-08-31 and recorded as an intended divergence, not aligned away.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 260 derived from disk at authoring time); ticket 05 (packet-list P11 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence: the two spacing keys are genuinely A, the two pattern keys are re-adjudicated declared-with-gap); ticket 104 (support rename keys — resolved; the support-interface keys are unaffected by the rename workstream).
- Ordering, not gating: packets 257/258/259 precede this packet in the queue but touch different modules — no same-module merge churn. Packet 238c (implemented) is the origin of the current spacing-key wiring this packet aligns/verifies.
- Unblocks: wayfinder ticket 18's resolution; nothing downstream gates on this packet specifically (P12/P13 touch the same support-planner family but different keys — the planners are not this packet's change surface).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `traditional-support.toml` and `tree-support.toml` manifests, **when** each one's `[config.schema]` is parsed, **then** each carries these four tables with exactly these entries — `support_interface_spacing` (`type = "float"`, `default = 0.5`, `min = 0.0`, `max = 2.0`, `group = "Support"` — default aligned from 0.4), `support_bottom_interface_spacing` (`type = "float"`, `default = 0.5`, `min = -1.0`, `max = 2.0`, `group = "Support"` — `min` deliberately kept at -1.0 for the retained mirror sentinel), `support_interface_pattern` (`type = "enum"`, `values = ["auto", "rectilinear", "concentric", "rectilinear_interlaced", "grid"]`, `default = "auto"`, `group = "Support"`), `support_interface_loop_pattern` (`type = "bool"`, `default = false`, `group = "Support"`) — with all four carrying `display` and `group = "Support"`. | `cargo test -p traditional-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a support-family module run over a fixed square region with top-interface plan entries, **when** `support_interface_spacing` is absent from the module config (default path), **then** the emitted top-interface path count equals the count produced with `support_interface_spacing = 0.5` explicitly set and is strictly less than the count with `support_interface_spacing = 0.4` (the pre-alignment fallback) — proving the aligned default reaches the interface-pitch decision point (`pitches_mm`, `slicer_core::support_regularize::interface_density`). | `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a module config with `support_interface_spacing = 0.8` and `support_bottom_interface_spacing = -1.0` over a region with bottom-interface plan entries, **when** the module runs, **then** the bottom-interface path count equals the run with `support_bottom_interface_spacing` absent (the retained PnP `-1` = mirror-top sentinel — recorded divergence per user ruling), and with `support_bottom_interface_spacing = 0.8` explicitly set the count is identical again. | `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** the scheduler's config bounds index loaded from the real `traditional-support.toml` manifest, **when** a CLI/sidecar value `support_interface_pattern = "bogus"` (not in the enum values), `support_interface_loop_pattern = "yes"` (non-bool), `support_interface_spacing = -0.5` (< min 0), or `support_bottom_interface_spacing = -2.0` (< kept min -1.0) is resolved, **then** resolution rejects the value with the standard enum/bool `TypeMismatch` / numeric `OutOfRange` error instead of passing it through to `ConfigView`; and `support_bottom_interface_spacing = -1.0` resolves successfully (the sentinel stays legal). | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a slice run over the support family, **when** the G-code CONFIG_BLOCK is emitted, **then** at defaults the block contains zero `support_interface_*` lines (none of the four keys rides `SUPPORT_CONFIG_DEFAULTS` or `ORCA_CONFIG_PADDING` — `serialize_config_block` in `crates/slicer-gcode/src/serialize.rs`; no padding twins added, packet 254/255/257/258/259 precedent); with an explicit `support_interface_spacing = 0.8` the line `; support_interface_spacing = 0.8` appears exactly once; with an explicit `support_interface_pattern = "rectilinear"` the line `; support_interface_pattern = rectilinear` appears exactly once. | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** the module-key tables carry `support_interface_pattern` and `support_interface_loop_pattern` under the `traditional-support` and `tree-support` owner columns, the `support_interface_spacing` rows show default `0.5`, and the deviations block (`<!-- BEGIN GENERATED: orca-deviations ... -->` … `<!-- END GENERATED: orca-deviations -->`) contains no `support_interface_spacing` data row and exactly 25 data rows (pre-packet count 27, measured at authoring, minus the two aligned rows). | `cargo xtask gen-config-docs --check && rg -q 'support_interface_pattern' docs/15_config_keys_reference.md && rg -q 'support_interface_loop_pattern' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| \`')" = "25" ]; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** a module config carrying the two declared-with-gap keys at non-default values (`support_interface_pattern = "concentric"` / `"grid"` / `"rectilinear_interlaced"` and `support_interface_loop_pattern = true`), **when** the module runs over a fixed region, **then** the emitted interface paths are byte-identical to the same run with the two keys absent — declaring them must not perturb behavior, because their canonical consumers (the `contact_fill_pattern` generator dispatch in `SupportParameters::SupportParameters` / `generate_support_toolpaths`, and `LoopInterfaceProcessor::generate` on `n_contact_loops` in `SupportCommon.cpp`) do not exist in this tree. | `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the manifest schema guard, **when** any of the four keys is removed from either manifest or its `type`/`default`/`values`/`min`/`max` drifts from AC-1's exact table, **then** the guard fails naming the offending key. | `cargo test -p traditional-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test support_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support --test tree_support_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifests and both `src/lib.rs` files are guest-fingerprint inputs (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker needs the `[config.schema]` contract; the enum `values` field form is grounded in-tree (`tree-support-planner.toml` `[config.schema.support_style]`).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for `support_interface_pattern` and `support_interface_loop_pattern` (owner columns `traditional-support` and `tree-support`), updates the two `support_interface_spacing` default cells 0.4 → 0.5 (rows for both modules), and its generated deviations block loses the two `support_interface_spacing` rows (27 → 25 data rows). The doc has no per-module subheadings, so verification is key-presence + row-count, not headings. Verification greps: `rg -q 'support_interface_pattern' docs/15_config_keys_reference.md`, `rg -q 'support_interface_loop_pattern' docs/15_config_keys_reference.md`, and the AC-6 deviation-block probes. The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 4), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the four keys (`support_interface_spacing` coFloat default 0.5 min 0; `support_bottom_interface_spacing` coFloat default 0.5 min 0 **with no -1 sentinel**; `support_interface_pattern` coEnum `SupportMaterialInterfacePattern` default smipAuto, values via `get_enum_values()`; `support_interface_loop_pattern` coBool default false — the canonical type correction this packet records); authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence (dispatched canonical reads, 2026-08-31) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `SupportMaterialInterfacePattern` enum (smipAuto/smipRectilinear/smipConcentric/smipRectilinearInterlaced/smipGrid) and the `ConfigOptionBool` member for `support_interface_loop_pattern`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters(const PrintObject&)` (the `top_interface_spacing = (ironing ? 0 : value) + flow.spacing()` / `top_interface_density = min(1, flow.spacing()/spacing)` formulas this port's `slicer_core::support_regularize` mirrors; the `contact_fill_pattern` branch order: grid→ipGrid, rectilinear_interlaced→ipRectilinear, (auto+zero-gap)∥concentric→ipConcentric, density > 0.95→ipRectilinear, else ipSupportBase) and `support_interface_angle()` (smipRectilinear → interface_angle, snug −45°; smipRectilinearInterlaced → interface_angle ±45° by parity; smipGrid → base_angle; auto/concentric → interface_angle).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths` (filler construction from `contact_fill_pattern` via `Fill::new_from_type`; `LoopInterfaceProcessor` with `n_contact_loops = config.support_interface_loop_pattern.value ? 1 : 0`) and `LoopInterfaceProcessor::generate` (the absent contact-loop generator).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportMaterial::has_contact_loops` (returns the loop-pattern bool).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::generate_toolpaths` (same spacing/density formula and filler-angle handling for the tree family).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::invalidate_state_by_config_options` (support invalidation for the four keys).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
