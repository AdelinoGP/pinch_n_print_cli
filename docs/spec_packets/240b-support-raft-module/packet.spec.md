---
status: draft
packet: 240b-support-raft-module
depends_on: 240a-support-raft-substrate
task_ids:
  - TASK-414
  - TASK-415
  - TASK-416
  - TASK-417
  - TASK-418
  - TASK-537
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: M
---

# Packet Contract: 240b-support-raft-module

## Goal

Close G-06 by building the raft consumer on 240a's substrate: a new
`com.core.raft-default` `Layer::Infill` synthesizer holding `claim:raft-fill`
that reads `SupportPlanIR.raft_plan` (through 240a's
`paint-region-layer-view.raft-plan` accessor), `SliceIR`, and `LayerPlanIR` and
writes deterministic raft footprint polygons into `SlicedRegion.raft_fill`;
plus the three net-new canonical raft config keys (none of them exists
anywhere under `modules/` or `crates/` today - they are introduced here for the
first time), the raft-key wire-or-record sweep across the existing support
manifests, the formal ADR-0009 Decision-5 amendment, and the Human Validation
Gate.

## Scope Boundaries

This packet owns the consumer and the key surface only. Every type, index, and
transport it stands on — the signed negative prefix band, the `raft_fill`
carrier, the `raft-plan` read accessor — is 240a's and must already be green.
Extrusion-path conversion happens downstream under the claim-holder emit path
(design.md §ADR-0009 Reconciliation); no pattern algorithm or renderer lives in
this module. Independent support-Z (239) and the AGG rasterizer (241) are
excluded.

## Prerequisites and Blockers

- Depends on: **240a-support-raft-substrate** — HARD BLOCKER. 240a's AC-1..AC-7
  must be green before Step 1 here. Re-derive at activation
  (`grep '^status:' docs/spec_packets/240a-support-raft-substrate/packet.spec.md`);
  it is `draft` at authoring time, so every reference below to `raft_fill`,
  `raft-plan`, `is-raft-prefix`, or an `i32` layer index is a FORWARD-DEP on
  240a, reconciled name-for-name against 240a's `design.md`.
- Also depends on: **236-support-stabilization** (`implemented` at authoring
  time — G-21 validator, ADR-0059 acceptance are shipped facts).
- Unblocks: 242-support-family-orca-closure (plan §11 queue row #9).
- Activation blockers: the §9 raft-enabled Orca references must exist under
  `tmp/` (human-owned) before the Human Validation Gate can sign. Authoring and
  Steps 1-5 are not blocked by their absence; only the gate is.

## Acceptance Criteria

- **AC-1. Given** the new module directory `modules/core-modules/raft-default/`,
  **when** the host loads the module directory, **then** the manifest declares
  id `com.core.raft-default`, stage `Layer::Infill`,
  `holds = ["claim:raft-fill"]`, `reads = ["SliceIR", "LayerPlanIR",
  "SupportPlanIR"]`, `writes = ["SliceIR"]`, and the guest compiles to a fresh
  component artifact. |
  `rg -q 'id = "com.core.raft-default"' modules/core-modules/raft-default/raft-default.toml && rg -q 'claim:raft-fill' modules/core-modules/raft-default/raft-default.toml && rg -q 'Layer::Infill' modules/core-modules/raft-default/raft-default.toml && cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?`
- **AC-2. Given** the claim machinery already maps
  `ExtrusionRole::RaftInfill` to `"claim:raft-fill"` in
  `SliceRegionView::should_emit` (`crates/slicer-sdk/src/views.rs`), **when**
  `com.core.raft-default` is the sole declared holder, **then**
  `should_emit(ExtrusionRole::RaftInfill)` returns true for its held-claim set
  and the startup claim registry resolves exactly one holder of the exact
  string `claim:raft-fill`. |
  `mkdir -p target && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- ac4_raft_fill_claim_emits_raft_infill --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && test "$(rg -l 'claim:raft-fill' modules/core-modules/*/[a-z-]*.toml | wc -l)" -eq 1`
- **AC-3. Given** `SupportPlanIR.raft_plan` is `Some` and the layer's global
  index is negative, **when** `com.core.raft-default`'s `run_infill` executes,
  **then** it writes `SlicedRegion.raft_fill` with object-independent raft
  footprint polygons honoring `RaftPlan.raft_layers` /
  `.base_raft_layers` / `.interface_raft_layers`, and two runs over identical
  inputs produce byte-identical `raft_fill` (no RNG, no iteration-order
  dependence, identical across the wasm and native legs). |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_fill_is_deterministic_across_two_runs --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4. Given** the canonical expansions, **when** the band is synthesized,
  **then** the first printed raft layer is expanded by
  `raft_first_layer_expansion` and the remaining layers by `raft_expansion`
  (both mm, divided by 100 at the unit boundary per E8, applied as iterated
  offsets preserving canonical's multi-step inflation), interface-band
  footprints are derived at `raft_contact_distance`-based spacing, and the
  first raft layer's area strictly exceeds every upper raft layer's area. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_first_layer_expansion_exceeds_upper_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5. Given** `support_raft_layers > 0`, **when** the pipeline executes end
  to end, **then** raft geometry is emitted at the negative global-layer prefix
  entries only, those entries sort strictly before model layer `0` in the
  G-code, and no `AnchoredEntity` is minted for any raft entry. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry_orders_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- raft_mints_no_anchored_entities --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6. Given** the three canonical raft keys are net-new (no manifest under
  `modules/` and no source under `crates/` declares any of them today; the
  names and defaults come from `docs/ORCA_CONFIG_REFERENCE.md` and canonical
  `init_fff_params` in `PrintConfig.cpp`, never from a pre-existing manifest),
  **when** `com.core.raft-default` is dispatched, **then**
  `raft_contact_distance` (default 0.1), `raft_expansion` (default 1.5), and
  `raft_first_layer_expansion` (default 2.0) are declared in
  `modules/core-modules/raft-default/raft-default.toml`'s `[config.schema]`
  with those defaults and each is read by the geometry it controls. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_keys_declared_and_wired --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-7. Given** the raft-related keys actually declared by the existing
  core-module manifests (re-derived at execution time by grepping
  `modules/core-modules/*/*.toml` - do not assume a fixed set; at authoring the
  grep returns only `support_raft_layers` plus tree-support-planner's
  `raft_first_layer_density` / `base_raft_layers` / `interface_raft_layers`),
  **when** the wire-or-record pass completes, **then** every (key, manifest)
  pair the grep returns has a written decision in `requirements.md`
  §Wire-or-Record Decisions naming the key, the manifest, the verdict (`wired`
  or `stays dead`), and the reason - one table row per declaration site, none
  still reading `pending Step 5`. |
  `DECL="$(rg --no-filename -o '^\[config\.schema\.[a-z_]*raft[a-z_]*\]' modules/core-modules -g '*.toml' -g '!raft-default.toml' | wc -l)"; ROWS="$(rg -c '^\| .[a-z_]*raft[a-z_]*. \| .[a-z0-9-]+. \| [a-z]' docs/spec_packets/240b-support-raft-module/requirements.md)"; test "$DECL" -ge 1 && test "${ROWS:-0}" -eq "$DECL" && ! rg -q 'pending Step 5' docs/spec_packets/240b-support-raft-module/requirements.md`
- **AC-8. Given** DEV-124's clamp is index-convention-dependent and 240a filed
  a reopen row for it, **when** the raft path is live, **then**
  `classic_clamp_follows_raft_layers_not_layer_zero` and
  `classic_clamp_unchanged_when_no_raft_configured`
  (`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`) are
  re-run under a raft-configured config view and the outcome — pass, or the
  corrected predicate and the fix — is written into `requirements.md`
  §DEV-124 Re-verification. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test contract -- classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

Every AC names exact fields, paths, counts, errors, variants, or output
fragments and ends with its own runnable command. Repeat shared commands; never
write "see AC-N". Commands that dump more than 200 successful output lines must
be wrapped or filtered so a subagent can return a FACT.

AC verification command rule: `slicer-sdk --test should_emit_raft_fill_claim_tdd`
(case `ac4_raft_fill_claim_emits_raft_infill`) and
`slicer-runtime --test contract` (the two `only_one_wall_first_layer_tdd`
cases) exist today — verified against the tree at authoring time. The
`slicer-runtime --test integration` and `--test contract` cases new to this
packet are authored into those aggregated binaries with their `mod`
registration in the same step (Steps 4 and 6).

## Negative Test Cases

- **AC-N1. Given** `com.core.raft-default` is the intended single holder (plan
  §12), **when** a second loaded `Layer::Infill` manifest also declares
  `claim:raft-fill`, **then** startup DAG validation surfaces the duplicate as
  a structured `SchedulerError::ClaimConflict` — a FOUR-field variant
  (`claim: String`, `module_a: ModuleId`, `module_b: ModuleId`,
  `scope: ConflictScope`) — naming both module ids in its `module_a` /
  `module_b` fields, with `claim` equal to the raft-fill claim string — not
  silence, not a panic. |
  `mkdir -p target && cargo test -p slicer-scheduler --test raft_claim_conflict_tdd -- raft_fill_double_holder_conflicts --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2. Given** the declared raft band is `-N .. -1`, **when** a raft entry
  carries a global layer index below `-N`, **then** the host rejects it with a
  typed validation error naming the offending index instead of emitting
  geometry at an unprintable Z. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_index_outside_band_rejected --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3. Given** the E9 silent-default mechanism, **when** a raft key the
  module consumes is missing from its manifest `[config.schema]`, **then** the
  module's own config-declaration test fails rather than resolving an in-code
  default invisibly — asserted by removing a key in the test fixture, not by
  grepping the manifest for its presence. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- undeclared_raft_key_is_rejected_not_defaulted --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry_orders_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief
  "240-support-raft", §10 absorption mapping, §7 evidence standards, §8 human
  gate, §13 traps T1/T4/T5/T8; direct range read.
- `docs/adr/0009-raft-as-layer-infill-role.md` - role/claim pattern and
  synthesizer shape; short - full read allowed.
- `docs/spec_packets/240a-support-raft-substrate/design.md` - the substrate this
  packet consumes; direct read of §Migration Table and §`raft_plan`
  Read-Path Footprint only.
- `docs/15_config_keys_reference.md` - regenerated, not read in bulk.
- `docs/19_visual_debug.md` + `docs/17_agent_debugging.md` - human-gate bundle
  only; delegated SUMMARY.

## Doc Impact Statement (Required)

- `docs/adr/0009-raft-as-layer-infill-role.md` formal amendment: Status line → `Accepted`, dropping the dangling `lands with docs/specs/raft-default-module.md` parenthetical, and replacing the other two `docs/specs/raft-default-module.md` pointers in the ADR (Decision-3 carrier parenthetical and the References list) — three occurrences in all. That path does not exist; the doc was archived to `docs/specs/_OLD/raft-default-module.md`, which is historical context only (it uses `raft_expansion_mm` / `raft_z_gap_mm` / `raft_layer_height_mm` / `raft_pattern`, superseded here by the canonical Orca names) and must not be cited as the contract; additive `## Amendment — <date> (packet 240b)` section recording the Decision-5 claim reassignment to `com.core.raft-default` and quoting the original clause verbatim - ``rg -A2 '^## Status' docs/adr/0009-raft-as-layer-infill-role.md | rg -q 'Accepted' && rg -q '^## Amendment' docs/adr/0009-raft-as-layer-infill-role.md && rg -q 'com\.core\.raft-default' docs/adr/0009-raft-as-layer-infill-role.md && test "$(rg -c 'rectilinear-infill` declaring the claim' docs/adr/0009-raft-as-layer-infill-role.md)" -ge 2`` and ``rg -q 'raft-default-module\.md' docs/adr/0009-raft-as-layer-infill-role.md && exit 1 || true``
- `docs/DEVIATION_LOG.md` gains the ADR-amendment row required whenever a packet supersedes an ADR's normative clause (live convention: `D-285-ADR-0051-AMENDED`, `D-286-ADR-0005-AMENDED`) - `rg -q 'ADR-0009-AMENDED' docs/DEVIATION_LOG.md`
- `docs/15_config_keys_reference.md` regenerated for the three new keys - `rg -q 'raft_contact_distance' docs/15_config_keys_reference.md && rg -q 'raft_first_layer_expansion' docs/15_config_keys_reference.md`
- `docs/03_wit_and_manifest.md` module inventory gains `com.core.raft-default` - `rg -q 'com\.core\.raft-default' docs/03_wit_and_manifest.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_raft_base`: object-independent raft layer construction; first-layer expansion via `raft_first_layer_expansion` (`inflate_factor_1st_layer`), contact/inflate logic, separate loops for base-raft and interface-raft layers, "Inflate in multiple steps to avoid leaking", classic-columns-above-raft vs organic-raft-on-bed branches; consumes `brim_type`/`brim_object_gap` and `slicing_params.raft_layers()`/`base_raft_layers`/`interface_raft_layers`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `init_fff_params`: canonical defaults `raft_contact_distance = 0.1`, `raft_expansion = 1.5`, `raft_first_layer_expansion = 2.0` (mm).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Human Validation Gate

Blocking sign-off; a date + verdict line recorded below flips nothing until
every artifact-producing command has run and every checklist item has a written
verdict (E2: inspection is satisfied by the written checklist, never by PNG
existence).

Artifact-producing commands (run from repo root; matched profiles
`tmp/support-family-config-tree-matched.json` / `-normal-matched.json`):

- `cargo run --bin pnp_cli --release -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --output tmp/p240b-pnp-raft.gcode` (with `support_raft_layers >= 2` in the matched profile copy saved as `tmp/p240b-profile.json`)
- Regenerated Orca references (§9, human-owned): `tmp/p240b-orca-tree-raft.gcode` and `tmp/p240b-orca-normal-raft.gcode` sliced with `raft_layers > 0`. **These references must exist before this gate can sign** — the gate blocks without them.
- Visual-debug bundle for the raft boundary: `tmp/p240b-vd-raft.json` request → PNGs + `manifest.json` per `docs/19_visual_debug.md`.

Checklist — standard five items (each: source, layer/tap, verdict):

1. Termination: raft reaches the plate beneath the object overhang for both families.
2. Coverage: raft area covers the supported footprint at every raft layer.
3. Collision freedom: raft does not intersect object walls above it.
4. Interfaces: interface-raft layers distinct from base-raft layers in spacing/density.
5. Block counts vs Orca references: raft `;TYPE:` block counts compared against `tmp/p240b-orca-*-raft.gcode`.

Raft-specific observations (required additions):

6. Raft layers present below plate contact — negative-index entries emitted before model layer 0 in Z order.
7. First-layer expansion visible: the first printed raft layer is wider than the upper raft layers by roughly `raft_first_layer_expansion` (2.0 mm canonical).
8. No anchored-entity leakage: no raft geometry appears through the anchored-event path (the G-code viewer shows raft as ordinary ordered entities at negative-index layers).

Sign-off: _(date + verdict pending; required before `status: implemented`)_
