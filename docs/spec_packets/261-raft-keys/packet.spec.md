---
status: draft
packet: raft-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/19-author-packet-p12-support-raft-support-planner.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P12)
context_cost_estimate: M
---

# Packet Contract: raft-keys

## Goal

Declare the two OrcaSlicer raft keys (`raft_contact_distance`, `raft_expansion`) with canonical types/defaults/bounds in the raft-config-owning planner manifest (`tree-support-planner.toml`), declared-with-gap (non-perturbing — their canonical decision points, the raft Z-gap in `SlicingParameters::SlicingParameters` and the raft-footprint XY expansion in `SupportMaterial::generate_contact_polygons` / `TreeSupport3D::generate_raft_contact`, do not exist in this tree because no raft geometry generator is implemented), and pin the deliberate omission in `traditional-support-planner.toml` (the traditional family has no raft surface).

## Scope Boundaries

The packet touches `tree-support-planner.toml` `[config.schema]` (two net-new float tables), the planner's test directory (a net-new manifest guard plus non-perturbation arms in the existing `orca_parity_tdd.rs` suite), the scheduler bounds-enforcement arm and the runtime CONFIG_BLOCK arm (one test file each, mirroring packet 260's integration arms), and the generated `docs/15_config_keys_reference.md`. It does not implement raft geometry (the absent `com.core.raft-default` generator — draft packet 240-support-raft owns that surface), does not wire the keys into `RaftPlan` (speculative pre-wiring for a module that does not exist), and does not touch `traditional-support-planner.toml` beyond the recorded omission pin.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — resolved; number 261 derived from disk at authoring time); ticket 05 (packet-list P12 membership); ticket 04 (tier rubric — Tier A membership re-derived in `requirements.md` §Per-Key Canonical Evidence: both keys are re-adjudicated declared-with-gap); ticket 104 (support rename keys — resolved; the raft keys are unaffected by the rename workstream).
- Ordering, not gating: packets 257/258/259/260 precede this packet in the queue but touch different modules — no same-module merge churn. Draft packet 240-support-raft (support-families plan) plans to declare these keys in `com.core.raft-default`'s manifest and wire them to geometry when implemented; this packet's declarations are the config-reachability half and are recorded as 240's wire-or-record input, not a conflict (packet 260's spacing keys are the same-key-in-two-modules precedent).
- Unblocks: wayfinder ticket 19's resolution; nothing downstream gates on this packet specifically (P13 touches the same planner family but different keys — `raft_first_layer_expansion` is P13's surface).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `tree-support-planner.toml` manifest, **when** its `[config.schema]` is parsed, **then** it carries these two tables with exactly these entries — `raft_contact_distance` (`type = "float"`, `default = 0.1`, `min = 0.0`, no `max`, `display = "Raft Contact Distance"`, `group = "Support"`) and `raft_expansion` (`type = "float"`, `default = 1.5`, `min = 0.0`, no `max`, `display = "Raft Expansion"`, `group = "Support"`) — each with a `description` comment recording the decision-point gap. | `cargo test -p tree-support-planner --test raft_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a planner run over the overhang fixture with `support_raft_layers = 3`, **when** `raft_contact_distance = 0.5` and `raft_expansion = 3.0` are set explicitly in the module config, **then** the emitted `SupportPlanIR` entries and the `RaftPlan` are byte-identical to the same run with the two keys absent — declaring them must not perturb behavior, because their canonical consumers (the raft Z-gap in `SlicingParameters::SlicingParameters` and the raft-footprint expansion in `SupportMaterial::generate_contact_polygons` / `TreeSupport3D::generate_raft_contact`) do not exist in this tree. | `cargo test -p tree-support-planner --test orca_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the scheduler's config bounds index loaded from the real `tree-support-planner.toml` manifest, **when** a CLI/sidecar value `raft_contact_distance = -0.5` (< min 0) or `raft_expansion = -1.0` (< min 0) is resolved, **then** resolution rejects the value with the numeric `OutOfRange` error instead of passing it through to `ConfigView`; and `raft_contact_distance = "abc"` (non-float) is rejected with `TypeMismatch`. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a slice run over the support family, **when** the G-code CONFIG_BLOCK is emitted, **then** at defaults the block contains zero `raft_contact_distance` / `raft_expansion` lines (neither key rides `SUPPORT_CONFIG_DEFAULTS` or `ORCA_CONFIG_PADDING` — `serialize_config_block` in `crates/slicer-gcode/src/serialize.rs`; no padding twins added, packet 254/255/257/258/259/260 precedent); with an explicit `raft_contact_distance = 0.5` the line `; raft_contact_distance = 0.5` appears exactly once; with an explicit `raft_expansion = 3.0` the line `; raft_expansion = 3.0` appears exactly once. | `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** the module-key table carries `raft_contact_distance` and `raft_expansion` under the `tree-support-planner` owner column, and the deviations block (`<!-- BEGIN GENERATED: orca-deviations ... -->` … `<!-- END GENERATED: orca-deviations -->`) still contains exactly 27 data rows (pre-packet count, measured at authoring — both keys' declared defaults 0.1/1.5 match the canonical defaults, so no rows are gained or lost). | `cargo xtask gen-config-docs --check && rg -q 'raft_contact_distance' docs/15_config_keys_reference.md && rg -q 'raft_expansion' docs/15_config_keys_reference.md && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md | grep -c '^| \`')" = "27" ]; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** the manifest schema guard, **when** either key is removed from `tree-support-planner.toml` or its `type`/`default`/`min`/`display`/`group` drifts from AC-1's exact table, **then** the guard fails naming the offending key. | `cargo test -p tree-support-planner --test raft_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the deliberate omission ruling, **when** `traditional-support-planner.toml` is parsed, **then** it does NOT declare `raft_contact_distance` or `raft_expansion` (the traditional family has no raft surface — no raft keys declared, no `RaftPlan` emitted; the omission is pinned so a future packet that wires raft for the traditional family must update the guard). | `cargo test -p tree-support-planner --test raft_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tree-support-planner --test raft_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p tree-support-planner --test orca_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — the manifest is a guest-fingerprint input (`guest_input_paths` in `xtask/src/build_guests.rs`), so this must return exit 0 before closure.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated tables regenerate via `cargo xtask gen-config-docs`; verify with `--check` (delegated; the doc is generated, never hand-edited).
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker needs the `[config.schema]` contract; the no-max float form is grounded in-tree (`tree-support-planner.toml` `[config.schema.max_bridge_length]`).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for `raft_contact_distance` and `raft_expansion` (owner column `tree-support-planner`); the generated deviations block is unchanged (27 data rows — both declared defaults match canonical). The doc has no per-module subheadings, so verification is key-presence + row-count, not headings. Verification greps: `rg -q 'raft_contact_distance' docs/15_config_keys_reference.md`, `rg -q 'raft_expansion' docs/15_config_keys_reference.md`, and the AC-5 deviation-block probe. The doc is generated — the edit lands through `cargo xtask gen-config-docs` (Step 4), never hand-written.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the two keys (`raft_contact_distance` coFloat default 0.1 min 0 no max; `raft_expansion` coFloat default 1.5 min 0 no max; both on `PrintObjectConfig`); authoring-time evidence already captured in `requirements.md` §Per-Key Canonical Evidence (dispatched canonical reads, 2026-08-31) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `SlicingParameters::SlicingParameters` (the raft Z-gap: `raft_z_gap` → `gap_raft_object` → `object_print_z_min = raft_contact_top_z + gap_raft_object`; forced to 0 when `raft_z_gap == 0.0 || zero_topZ_contact`).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `generate_contact_polygons` (layer_id==0 branch: `contact_polygons = raft_expansion > 0 ? expand(overhang_polygons, scaled(raft_expansion)) : overhang_polygons`).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport3D.cpp` — `generate_raft_contact` / `finalize_raft_contact` (raft-contact expansion and tree-tip culling inside the expanded raft).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_print_z` / support-gap warning logic (rounds `raft_contact_distance` up to layer height as `extra_gap` when the last extrusion layer is a support layer).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::invalidate_state_by_config_options` (support invalidation for the two keys).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
