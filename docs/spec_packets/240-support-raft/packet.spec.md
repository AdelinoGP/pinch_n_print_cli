---
status: draft
packet: 240-support-raft
depends_on: 236-support-stabilization
task_ids:
  - TASK-409
  - TASK-410
  - TASK-411
  - TASK-412
  - TASK-413
  - TASK-414
  - TASK-415
  - TASK-416
  - TASK-417
  - TASK-418
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: M
---

# Packet Contract: 240-support-raft

## Goal

Implement raft geometry end-to-end: a new `com.core.raft-default` `Layer::Infill`
synthesizer holding `claim:raft-fill` that reads `SupportPlanIR.raft_plan`,
`SliceIR`, and `LayerPlanIR` and writes the new `SlicedRegion.raft_fill` with
deterministic fill polygons (extrusion-path conversion happens downstream
under the claim-holder emit path — design.md §ADR-0009 Reconciliation, which
also records the formal ADR-0009 Decision-5 claim reassignment to
`com.core.raft-default`); the u32→i32 signed-layer-index migration;
and the canonical raft config keys — preserving the ADR-0009 contract that rafts
stay signed negative global-layer PREFIX entries, never anchored entities.

## Scope Boundaries

The packet owns the raft consumer for the built-but-unconsumed `RaftPlan`
transport (G-06), the signed negative-index substrate it requires, and the
raft key surface. It absorbs the full scope of deleted-draft 215-raft-geometry
(mapping recorded in `requirements.md`). Independent support-Z (239) and the
AGG rasterizer (241) are excluded.

## Prerequisites and Blockers

- Depends on: **236-support-stabilization** (FORWARD DEPENDENCY — 236 is
  generated as `draft` itself; this packet may be authored now but must not be
  activated until 236 closes its green gate. All phrasing here treats 236's
  outcomes — AC-8 per-region ruling, G-21 validator update, ADR-0059
  acceptance — as expected-to-exist, never as verified facts).
- Unblocks: 242-support-family-orca-closure (queue row #9 depends on this row).
- Activation blockers: none local; activation additionally requires the §9
  raft-enabled Orca references to exist under `tmp/` (human-owned) before the
  Human Validation Gate can sign — authoring is not blocked.

## Acceptance Criteria

- **AC-1. Given** the workspace builds, **when** the signed-index migration
  lands, **then** `GlobalLayer.index`, `ObjectLayerRef.local_layer_index`,
  `ObjectLayerRef.global_layer_index`, `SliceIR.global_layer_index`,
  `InfillIR.global_layer_index`, and `SupportIR.global_layer_index` are `i32`,
  `LayerModule::run_infill` takes `layer_index: i32`, a negative
  `SliceIR.global_layer_index` round-trips through serde unchanged, and every
  `Layer::Infill` guest still instantiates against rebuilt wasm artifacts. |
  `mkdir -p target && cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2. Given** `SlicedRegion` carries the new field, **when**
  `com.core.raft-default` runs on a layer whose
  `SupportPlanIR.raft_plan` is `Some`, **then** it populates
  `SlicedRegion.raft_fill: Vec<ExPolygon>` with deterministic raft
  polygons (identical inputs → identical output across two runs), the
  `slice-region-view` WIT resource exposes a `raft-fill` accessor, and the
  `SliceIR` schema version minor-bumps with the bump recorded in the version
  history doc comment. | `cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_deterministic --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3. Given** the new module directory `modules/core-modules/raft-default/`,
  **when** the host loads the module directory, **then** the module manifest
  declares id `com.core.raft-default`, stage `Layer::Infill`,
  `holds = ["claim:raft-fill"]`, `reads = ["SliceIR", "LayerPlanIR",
  "SupportPlanIR"]`, `writes = ["SliceIR"]`, the guest compiles to a fresh
  component artifact, and `should_emit(ExtrusionRole::RaftInfill)` returns true
  for its held-claim set. | `cargo xtask build-guests --check && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- raft_infill_claim_emits --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4. Given** `support_raft_layers > 0`, **when** the pipeline executes,
  **then** raft layers are emitted as signed negative global-layer prefix
  entries (`-1, ..., -N`) that sort strictly before model layer `0`, the
  synthesized geometry honors `raft_expansion` and
  `raft_first_layer_expansion` (mm, divided by 100 at the unit boundary per
  E8), and no `AnchoredEntity` is minted for any raft entry. |
  `cargo test -p slicer-runtime --test integration -- raft_prefix_orders_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- raft_mints_no_anchored_entities --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5. Given** the canonical raft keys, **when** `com.core.raft-default` is
  dispatched, **then** `raft_contact_distance` (default 0.1),
  `raft_expansion` (default 1.5), and `raft_first_layer_expansion` (default
  2.0) are declared in its manifest `[config.schema]`, each wired to the
  geometry it controls; and for each of the four support-module manifests
  (`tree-support-planner`, `traditional-support-planner`, `tree-support`,
  `traditional-support`) a written wire-or-record decision exists in this
  packet for every raft key it declares or deliberately omits. |
  `cargo test -p slicer-runtime --test contract -- raft_keys_declared_and_wired --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  (single-test form; the AC-5 contract case lives in
  `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs`, registered in
  Step 6b)
- **AC-6. Given** DEV-124 was closed 2026-08-07 (perimeter modules gate
  `only_one_wall_first_layer` on `layer_index == support_raft_layers`),
  **when** the raft path becomes live, **then** the existing contract tests
  pass unchanged under a raft-configured config view and the verify-record
  outcome (including the deliberately-unported `has_bottom_shell_layers`
  residual) is written into this packet's requirements. |
  `cargo test -p slicer-runtime --test contract -- classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test contract -- classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

Every AC names exact fields, paths, counts, errors, variants, or output
fragments and ends with its own runnable command. Repeat shared commands; never
write "see AC-N". Commands that dump more than 200 successful output lines must
be wrapped or filtered so a subagent can return a FACT.

AC verification command rule: AC-1..AC-6 name test binaries that either exist
today (`should_emit_raft_fill_claim_tdd`, `contract`/
`only_one_wall_first_layer_tdd` cases) or are authored by this packet's steps
into binaries with the required driver setup (`signed_layer_indices_tdd` and
`sliced_region_raft_fill_tdd` in `slicer-ir`; integration cases in
`slicer-runtime/tests/integration/main.rs`; `raft_keys_declared_and_wired` in
the contract binary). No AC relies on a driver that does not exist by the time
its step completes.

## Negative Test Cases

- **AC-N1. Given** `com.core.raft-default` (the intended single holder per
  plan §12) plus a second loaded `Layer::Infill` module manifest both declare
  `claim:raft-fill`, **when** startup DAG validation runs, **then** the
  scheduler surfaces the duplicate holder as a structured `ClaimConflict`
  advisory naming both module ids — `com.core.raft-default` and the second
  id (not silence, not a panic), and per-region dispatch resolves to exactly
  one emitter deterministically. |
  `cargo test -p slicer-scheduler --test validation_tdd -- raft_fill_double_holder_conflicts --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2. Given** bounds where they exist, **when** a raft prefix entry
  carries `global_layer_index < -(raft_layers)` (an index outside the declared
  raft band), **then** the host rejects it with a typed validation error
  instead of emitting geometry at an unprintable Z. |
  `cargo test -p slicer-runtime --test contract -- raft_index_outside_band_rejected --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3. Given** the E9 silent-default mechanism, **when** a raft key is
  consumed by `com.core.raft-default` but missing from its manifest
  `[config.schema]`, **then** the manifest-lint check fails the step (a module
  config view filtered to declared keys would otherwise resolve an in-code
  default invisibly). | `rg -q 'raft_contact_distance' modules/core-modules/raft-default/raft-default.toml && rg -q 'raft_expansion' modules/core-modules/raft-default/raft-default.toml && rg -q 'raft_first_layer_expansion' modules/core-modules/raft-default/raft-default.toml`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-ir --test signed_layer_indices_tdd -- signed_layer_indices_round_trip --exact 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief
  "240-support-raft", §10 absorption mapping, §7 evidence standards, §8 human
  gate, §13 traps T1/T4/T5/T8; direct range read.
- `docs/adr/0009-raft-as-layer-infill-role.md` - role/claim pattern and
  synthesizer shape; direct read (<110 lines).
- `docs/02_ir_schemas.md` - sections edited by this packet; delegated SUMMARY.
- `docs/19_visual_debug.md` + `docs/17_agent_debugging.md` - only for the
  human-gate visual-debug bundle; delegated SUMMARY.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section "SliceIR" (schema bump + `SlicedRegion.raft_fill` + signed indices) - `rg -q 'raft_fill' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` section on signed layer indices - `rg -q 'global_layer_index.*i32|negative.*raft' docs/02_ir_schemas.md`
- `docs/adr/0009-raft-as-layer-infill-role.md` formal amendment (status → Accepted; additive Amendments section recording the Decision-5 claim reassignment to `com.core.raft-default`, quoting the original clause) - `rg -q 'Accepted' docs/adr/0009-raft-as-layer-infill-role.md && rg -q 'Amendments' docs/adr/0009-raft-as-layer-infill-role.md && rg -q 'com\.core\.raft-default' docs/adr/0009-raft-as-layer-infill-role.md`
- `docs/15_config_keys_reference.md` regenerated for the three new keys - `rg -q 'raft_contact_distance' docs/15_config_keys_reference.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_raft_base`: object-independent raft layer construction; first-layer expansion via `raft_first_layer_expansion` (`inflate_factor_1st_layer`), contact/inflate logic, separate loops for base-raft and interface-raft layers, "Inflate in multiple steps to avoid leaking", classic-columns-above-raft vs organic-raft-on-bed branches; consumes `brim_type`/`brim_object_gap` and `slicing_params.raft_layers()`/`base_raft_layers`/`interface_raft_layers`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers` → `object.add_support_layer(...)`: insertion of `SupportLayers` at print_z BELOW layer 0 — the canonical analogue of PnP's signed negative prefix indices. Cite functions, never line numbers.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `init_fff_params`: canonical defaults `raft_contact_distance = 0.1`, `raft_expansion = 1.5`, `raft_first_layer_expansion = 2.0` (mm).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

## Human Validation Gate

Blocking sign-off; a date + verdict line recorded below flips nothing until
every artifact-producing command has run and every checklist item has a
written verdict (E2: inspection is satisfied by the written checklist, never
by PNG existence).

Artifact-producing commands (run from repo root; matched profiles
`tmp/support-family-config-tree-matched.json` / `-normal-matched.json`):

- `cargo run --bin pnp_cli --release -- slice --input crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --output tmp/p240-pnp-raft.gcode` (with `support_raft_layers >= 2` in the matched profile copy saved as `tmp/p240-profile.json`)
- Regenerated Orca references (§9, human-owned): `tmp/p240-orca-tree-raft.gcode` and `tmp/p240-orca-normal-raft.gcode` sliced with `raft_layers > 0`. **These references must exist before this gate can sign** — the gate blocks without them.
- Visual-debug bundle for the raft boundary: `tmp/p240-vd-raft.json` request → PNGs + `manifest.json` per `docs/19_visual_debug.md`.

Checklist — standard five items (each: source, layer/tap, verdict):

1. Termination: raft reaches the plate beneath the object overhang for both families.
2. Coverage: raft area covers the supported footprint at every raft layer.
3. Collision freedom: raft does not intersect object walls above it.
4. Interfaces: interface-raft layers distinct from base-raft layers in spacing/density.
5. Block counts vs Orca references: raft `;TYPE:` block counts compared against `tmp/p240-orca-*-raft.gcode`.

Raft-specific observations (required additions):

6. Raft layers present below plate contact (negative-index entries emitted before model layer 0 in Z order).
7. First-layer expansion visible: first printed raft layer wider than upper raft layers by roughly `raft_first_layer_expansion` (2.0 mm canonical).
8. No anchored-entity leakage: no raft geometry appears through the anchored-event path (G-code viewer shows raft as ordinary ordered entities at negative-index layers).

Sign-off: _(date + verdict pending; required before `status: implemented`)_
