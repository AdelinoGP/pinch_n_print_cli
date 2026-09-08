---
status: implemented
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
  `paint-region-layer-view.raft-plan` accessor), and `SliceIR` and
writes deterministic raft footprint polygons into `SlicedRegion.raft_fill`;
plus the three net-new canonical raft config keys (none of them exists
anywhere under `modules/` or `crates/` today - they are introduced here for the
first time), the raft-key wire-or-record sweep across the existing support
manifests, the formal ADR-0009 Decision-5 amendment, and the Human Validation
Gate.

## Scope Boundaries

This packet owns the consumer and the key surface only. Every type, index, and
transport it stands on — the positive raft offset band, the `raft_fill`
carrier, the `raft-plan` read accessor — is 240a's and must already be green.
Extrusion-path conversion happens downstream under the claim-holder emit path
(design.md §ADR-0009 Reconciliation); no pattern algorithm or renderer lives in
this module. Independent support-Z (239) and the AGG rasterizer (241) are
excluded.

**AD-240B-1 (scope amendment, user-approved 2026-09-05):** verification during
Step 3 proved the guest→`SlicedRegion.raft_fill` write transport and the
`raft_fill`→G-code emitter do NOT exist (WIT region accessors are
getters-only; `InfillOutputCollected` has no polygon carrier; nothing emits
raft_fill). Both are absorbed into this packet: additive
`infill-output-builder::push-raft-fill` WIT method (host-provided resource),
host + SDK builder methods, `InfillIR` additive raft carrier, runtime commit
into `SlicedRegion.raft_fill`, and the emitter at
`assemble_ordered_entities_with_support_identities` (raft_fill ex-polygons →
`ExtrusionRole::RaftInfill` ordered entities at raft band layers). No
`SliceIR` schema field is added and no schema version bumps. Every OTHER
type, index, and transport remains 240a's (verified green: paint-view
accessors, `is_raft` band, `raft_fill` field + partition/restore).

## Prerequisites and Blockers

- Depends on: **240a-support-raft-substrate** — HARD BLOCKER. 240a's AC-1..AC-7
  must be green before Step 1 here. Re-derive at activation
  (`grep '^status:' docs/spec_packets/240a-support-raft-substrate/packet.spec.md`);
  it is `draft` at authoring time, so every reference below to `raft_fill`,
  `raft-plan`, `is-raft-prefix`, or `GlobalLayer.is_raft` is a FORWARD-DEP on
  240a, reconciled name-for-name against 240a's `design.md`. Layer indices are
  `u32` and unchanged by 240a; the raft band is `0 .. N-1` with model layers at
  `N ..` where `N = support_raft_layers`.
- Also depends on: **236-support-stabilization** (`implemented` at authoring
  time — G-21 validator, ADR-0059 acceptance are shipped facts).
- AD-240B-1 (absorbed transport): the guest→`SlicedRegion.raft_fill` write
  transport and the `raft_fill` emitter were verified missing at Step 3 and
  absorbed into this packet by user-approved scope amendment — details and
  evidence in `design.md` §Absorbed Substrate Gap.
- Unblocks: 242-support-family-orca-closure (plan §11 queue row #9).
- Activation blockers: the §9 raft-enabled Orca references must exist under
  `tmp/` (human-owned) before the Human Validation Gate can sign. Authoring and
  Steps 1-5 are not blocked by their absence; only the gate is.

## Acceptance Criteria

- **AC-1. Given** the new module directory `modules/core-modules/raft-default/`,
  **when** the host loads the module directory, **then** the manifest declares
  id `com.core.raft-default`, stage `Layer::Infill`,
  `holds = ["claim:raft-fill"]`, `reads = ["SliceIR"]`,
  `writes = ["SliceIR", "InfillIR"]` (the raft-plan accessor rides the
  host-provisioned paint view per the `Layer::Infill` stage contract
  (docs/01 §Module Access Contract)), and the guest compiles to a fresh
  component artifact. |
  `rg -q 'id\s*=\s*"com\.core\.raft-default"' modules/core-modules/raft-default/raft-default.toml && rg -q 'claim:raft-fill' modules/core-modules/raft-default/raft-default.toml && rg -q 'Layer::Infill' modules/core-modules/raft-default/raft-default.toml && cargo xtask build-guests && cargo xtask build-guests --check && echo AC1-PASS`
- **AC-2. Given** the claim machinery already maps
  `ExtrusionRole::RaftInfill` to `"claim:raft-fill"` in
  `SliceRegionView::should_emit` (`crates/slicer-sdk/src/views.rs`), **when**
  `com.core.raft-default` is the sole declared holder, **then**
  `should_emit(ExtrusionRole::RaftInfill)` returns true for its held-claim set
  and the startup claim registry resolves exactly one holder of the exact
  string `claim:raft-fill`. |
  `mkdir -p target && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- ac4_raft_fill_claim_emits_raft_infill --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && test "$(rg -l 'claim:raft-fill' modules/core-modules/*/[a-z-]*.toml | wc -l)" -eq 1`
- **AC-3. Given** `SupportPlanIR.raft_plan` is `Some` and the layer carries
  `GlobalLayer.is_raft == true`, **when** `com.core.raft-default`'s `run_infill` executes,
  **then** it writes `SlicedRegion.raft_fill` with object-independent raft
  footprint polygons honoring `RaftPlan.raft_layers` /
  `.base_raft_layers` / `.interface_raft_layers`, and two runs over identical
  inputs produce byte-identical `raft_fill` (no RNG, no iteration-order
  dependence, identical across the wasm and native legs). |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_fill_is_deterministic_across_two_runs --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4. Given** the canonical expansions, **when** the band is synthesized,
  **then** the first printed raft layer is expanded by
  `raft_first_layer_expansion` and the remaining layers by `raft_expansion`
  (both mm, divided by 100 at the unit boundary per E8, applied as iterated
  offsets preserving canonical's multi-step inflation), interface-band
  footprints are derived at `raft_contact_distance`-based spacing, and the
  first raft layer's area strictly exceeds every upper raft layer's area. |
 `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_first_layer_expansion_exceeds_upper_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-9. Given** `support_raft_layers > 0`, **when** the pipeline executes,
  **then** each raft band layer's raft-role output (`ExtrusionRole::RaftInfill`, emitted under the canonical `;TYPE:Support` label; the retired `;TYPE:Raft` label must NOT reappear) contains open fill lines
  spanning the harvested region — hatch line count ≥
  `floor(band span / raft_line_spacing) − 1` and total raft extrusion exceeds
  the border-outline-only baseline by ≥5x.
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_band_fill_lines_cover_region --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5. Given** `support_raft_layers > 0`, **when** the pipeline executes end
  to end, **then** raft geometry is emitted at the `is_raft` band entries only
  (global indices `0 .. support_raft_layers - 1`), those entries sort strictly
  before the first model layer (index `support_raft_layers`) in the G-code, and
  no `AnchoredEntity` is minted for any raft entry. |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_geometry_orders_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_mints_no_anchored_entities --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6. Given** the three canonical raft keys are net-new (no manifest under
  `modules/` and no source under `crates/` declares any of them today; the
  names and defaults come from `docs/ORCA_CONFIG_REFERENCE.md` and canonical
  `init_fff_params` in `PrintConfig.cpp`, never from a pre-existing manifest),
  **when** `com.core.raft-default` is dispatched, **then**
  `raft_contact_distance` (default 0.1), `raft_expansion` (default 1.5), and
  `raft_first_layer_expansion` (default 2.0) are declared in
  `modules/core-modules/raft-default/raft-default.toml`'s `[config.schema]`
  with those defaults and each is read by the geometry it controls. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_bounds_tdd::raft_keys_declared_and_wired --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
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
  `DECL="$(rg --no-filename -o '^\[config\.schema\.[a-z_]*raft[a-z_]*\]' modules/core-modules -g '*.toml' -g '!raft-default.toml' | wc -l)"; ROWS="$(sed -n '/^## Wire-or-Record Decisions$/,/^## DEV-124 Re-verification$/p' docs/spec_packets/240b-support-raft-module/requirements.md | rg -c '^\| `[a-z_]*raft[a-z_]*` \| `[^`]+` \| (wired|stays dead)')"; test "$DECL" -ge 1 && test "${ROWS:-0}" -eq "$DECL" && ! sed -n '/^## Wire-or-Record Decisions$/,/^## DEV-124 Re-verification$/p' docs/spec_packets/240b-support-raft-module/requirements.md | rg -q '^\|.*PENDING-STEP5-ROW'`
- **AC-8. Given** DEV-124's clamp gates on `layer_index == support_raft_layers`
  and 240a UPHELD it rather than reopening it (the positive band makes the
  shipped predicate correct), **when** the raft path is live, **then**
  `classic_clamp_follows_raft_layers_not_layer_zero` and
  `classic_clamp_unchanged_when_no_raft_configured`
  (`crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`) are
  re-run under a raft-configured config view and the outcome — pass, or the
  observed failure and its cause — is written into `requirements.md`
  §DEV-124 Re-verification. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- only_one_wall_first_layer_tdd::classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && cargo test -p slicer-runtime --test contract -- only_one_wall_first_layer_tdd::classic_clamp_unchanged_when_no_raft_configured --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

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
- **AC-N2. Given** `com.core.raft-default`'s `run_infill` is dispatched on a
  layer whose `paint-region-layer-view.is-raft` returns `false` (a MODEL layer,
  where no raft geometry belongs) while `SupportPlanIR.raft_plan` is still
  `Some`, **then** the module writes ZERO polygons into
  `SlicedRegion.raft_fill` for that layer and returns `Ok(())` — it does not
  synthesize raft geometry on a model layer, and it does not error. Ownership:
  this is module-side behavior implemented in Step 3, not a host validator;
  240a already owns the harvest-time band rejection
  (`noncontiguous_raft_band_rejected`). |
  `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_writes_nothing_on_non_raft_layer --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3. Given** the E9 silent-default mechanism, **when** a raft key the
  module consumes is missing from its manifest `[config.schema]`, **then** the
  module's own config-declaration test fails rather than resolving an in-code
  default invisibly — asserted by removing a key in the test fixture, not by
  grepping the manifest for its presence. |
  `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_bounds_tdd::undeclared_raft_key_is_rejected_not_defaulted --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_geometry::raft_geometry_orders_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- `rg -q 'push-raft-fill' crates/slicer-schema/wit/deps/ir-types.wit`
- `rg -q 'raft_fill' crates/slicer-runtime/src/layer_executor.rs`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief
  "240-support-raft", §10 absorption mapping, §7 evidence standards, §8 human
  gate, §13 traps T1/T4/T5/T8; direct range read.
- `docs/adr/0009-raft-as-layer-infill-role.md` - role/claim pattern and
  synthesizer shape; short - full read allowed.
- `docs/spec_packets/240a-support-raft-substrate/design.md` - the substrate this
  packet consumes; direct read of §`raft_plan` Read-Path Footprint and
  §Architecture Constraints only.
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

- `cargo run --bin pnp_cli --release -- slice --module-dir modules/core-modules --config tmp/p240b-profile.json --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --output tmp/p240b-pnp-raft.gcode` (with `support_raft_layers >= 2` in the matched profile copy saved as `tmp/p240b-profile.json`) **— current certified artifact: `tmp/p240b-pnp-raft-v4.gcode` (2026-09-07, post gate-defect fixes incl. bottom-shell re-anchor; the canonical-path file was viewer-locked during regeneration, so v4 is the authoritative file — re-point or rename when the viewer releases it)**
- Regenerated Orca references (§9, human-owned): `tmp/p240b-orca-tree-raft.gcode` and `tmp/p240b-orca-normal-raft.gcode` sliced with `raft_layers > 0`. **These references must exist before this gate can sign** — the gate blocks without them. **Generated 2026-09-06**: copies of `tmp/SupportTest.3mf` with `Metadata/project_settings.config` patched (`raft_layers=2`, `independent_support_layer_height=1`, `support_layer_height_mm=0.1`; tree copy also `tree_support_branch_angle=40`/`tree_support_branch_diameter=2`/`tree_support_branch_distance=1`; normal copy `support_type=normal(auto)`) and sliced headlessly via `"C:\Program Files\OrcaSlicer\orca-slicer.exe" --slice 1 --allow-newer-file --export-3mf tmp/p240b-orca-<family>-raft.gcode.3mf tmp/p240b-orca-<family>-raft.3mf`, raw G-code extracted from `Metadata/plate_1.gcode` (intermediate `.3mf`/`.gcode.3mf` files retained in `tmp/`). Source 3MF project settings already matched the matched profiles except for the patched keys.
- Visual-debug bundle for the raft boundary: `tmp/p240b-vd-raft.json` request → PNGs + `manifest.json` per `docs/19_visual_debug.md`.

Checklist — standard five items (each: source, layer/tap, verdict):

1. Termination: **PASS (G-code `final_gcode` at raft layers 0-1; re-verified 2026-09-06 on `tmp/p240b-pnp-raft-v3.gcode`)** — raft-bearing layers at Z:0.2 and Z:0.4 (emitted under the canonical `;TYPE:Support` label after the custom-label retirement; identified by band membership, not label) reach the plate before the first model layer (Z 0.6); band footprint encloses the harvested region (X -12..2 / Y -2..22 at layer 0). The support footprint (right slab, X 0..22.2) is **not** covered — support branches start above the band post-suppression.
2. Coverage: **FAIL → FIXED (defect found at human gate 2026-09-06, fixed same day; G-code `final_gcode`, raft layers 0-1, certified on `tmp/p240b-pnp-raft-v3.gcode`)** — DEFECT HISTORY: the pre-fix artifact contained raft *outlines*, not a raft: the two raft blocks carried one contour loop each (layer 0: 41 moves / 2.41 mm extrusion; layer 1: 35 moves / 1.15 mm; 3.57 mm total — vs Orca's band layer at 237 fill blocks). Root cause: `layer_executor.rs` raft_fill→ExtrusionPath3D conversion consumed `polygon.contour.points` only; `design.md` AD-240B-1(d) specified exactly that contour conversion and omitted fill generation. FIX (Fix B, guest-side pattern): generic host hatch engine `hatch_areas` (`crates/slicer-core/src/polygon_ops.rs`) exposed to guests as `hatch-areas` (WIT `crates/slicer-schema/wit/deps/common.wit`); `raft-default` emits the expanded border ring plus hatch lines as open 2-point contours through `push_raft_fill` with new key `raft_line_spacing` (0.5 mm); executor closure contract fixed (2-point contours stay open, rings closed); a second gate defect (the executor's transport/merge hunks dropped during an interrupted worker reconstruction) was repaired the same day. CERTIFIED MEASUREMENT (v3): layer 0 = 91 segments / 24.52 mm E spanning X -12..2 / Y -2..22; layer 1 = 83 unique geometries / 22.06 mm E (bead 0.4 × 0.2 mm — exactly 2x the earlier under-computed 11.03 mm because v3's monotonic Z sequence removed the 0.1 mm interleave that halved the bead height) — ~9x the outline baseline (AC-9 threshold ≥5x); segment-set identity with the separately-tagged raft layer verified 0 extra / 0 missing.
3. Collision freedom: **PASS with upstream note (G-code `final_gcode`, all layers, re-measured 2026-09-07 on v4 post Z-shift + band suppression + bottom-shell re-anchor)** — the earlier BLOCKED-upstream verdict is retired by the fixes: model Z now shifts above the band (first model layer Z 0.6 vs band top 0.4) and band layers emit no model/support content, so the same-Z coexistence no longer exists. Measured on v4: min XY distance raft↔brim 0.40 mm (the configured object gap), band↔nearest model wall 2.2 mm, Z separation raft-top 0.4 / model 0.6; zero negative `;HEIGHT` deltas; model bottom-shell classification correctly re-anchored at the model's first non-band layer (Z 0.6), so the raft adds BELOW the model without altering the model's own layer stack, shell classification, or top Z. RESIDUAL UPSTREAM NOTE (not a raft defect, does not block this item): support branches still interleave the band's Z plane on non-band routing — recorded in Upstream Findings below for the layer-planner-default support surface.
4. Interfaces: **not applicable (planner-side) (pipeline trace, `RaftPlan`)** — `RaftPlan.interface_raft_layers = 0` (tree planner derivation, 238b surface); the module honors the plan; interface-band behavior is covered by the integration tests.
5. Block counts vs Orca references: **PASS with comparability notes (certified on v3; defect history retained; references generated 2026-09-06)** — the pre-fix verdict was wrong: the headline "PASS with comparability notes" measured structural parity (block counts, bounds) and called it done while the band layers carried one contour loop each (3.57 mm total) where Orca's band layers carry dense fill (237 / 224 blocks in the tree reference); "no contradiction found" was a false conclusion — the contradiction was the defect. Item 2's E-volume instrument is the authoritative measure and AC-9 now enforces it. Certified facts on v3: exactly 2 raft-band layers in all three files; both band layers carry dense fill (91 / 83 unique segment geometries, 24.52 / 22.06 mm E); first band layer wider than second in all three (pnp X -12..2 vs -11.5..1.5; orca-tree X 90.8..121.8 vs 93.7..119.3; orca-normal X 90.6..128.9 vs 93.5..107.6); model/support content starts only above the band in all three. Comparability caveats unchanged: this Orca build tags band layers `;TYPE:Support`/`;TYPE:Support interface` — after the label retirement pnp now matches that convention (interface-band labeling remains a deferred refinement while `interface_raft_layers = 0` in this plan surface); Orca rafts the full model footprint while pnp rafts the harvested region per the ADR-0009 amendment; band Z differs (pnp 0.2/0.4 vs orca 0.2/0.575 — Orca's own contact spacing).

Raft-specific observations (required additions):

6. Raft layers present below plate contact: **PASS with note (pipeline trace + G-code `final_gcode`, global layers 0-2; re-verified on v3)** — band entries at global indices 0..1 (Z 0.2/0.4) are emitted before the first model layer (Z 0.6); layer Z sequence is strictly monotonic (0.2, 0.4, 0.6, 0.7, ...), retiring the old Z-overlap note.
7. First-layer expansion visible: **PASS (G-code `final_gcode`, raft layers 0-1; re-verified on v3: X -12..2 → -11.5..1.5)** — first raft layer is wider than the upper raft layer by ~0.5 mm per side = `raft_first_layer_expansion` (2.0) − `raft_expansion` (1.5); the checklist's "roughly 2.0 mm" is the expansion magnitude, not the inter-layer delta.
8. No anchored-entity leakage: **PASS (pipeline trace + G-code `final_gcode`, raft layers 0-1; re-verified post-fix: regenerated `tmp/p240b-trace-v3.jsonl` against the current pipeline, 0 anchored events, 310 raft-default events)** — no raft geometry travels through the anchored-event path; raft appears as ordinary ordered entities at the band layers; band suppression also filters anchored collections at band indices (`layer_executor.rs` anchored-entity filters).

### Upstream Findings (recorded at gate, not 240b defects)

- Z overlap — `DefaultLayerPlanner::run_layer_planning` (`modules/core-modules/layer-planner-default/src/lib.rs`) left model Z unshifted above the raft band. **MODEL HALF FIXED 2026-09-06** (this packet's gate work): model-layer Z now shifts above the band top in both merge paths (`merge_same_height` / `merge_different_heights`), first model layer at Z 0.6 with the 2-layer 0.2/0.4 band; byte-identical when `support_raft_layers = 0`; verified end-to-end on v3 (strictly monotonic Z sequence, zero negative `;HEIGHT` deltas). **REMAINING:** support branches still interleave the raft band plane on non-band routing — same owner, layer-planner-default support surface; the band-layer `;TYPE:Support` label is now shared by genuine support, so band/support disambiguation in G-code assertions must use Z ranges, not labels.
- `RaftPlan.interface_raft_layers = 0` despite `support_interface_top_layers = 2` — tree planner derivation (238b surface); owner suggestion: 238b.
- Band layers' regions carry model-plane content — harvest behavior upstream of this packet; owner suggestion: 240a harvest.

Sign-off: **APPROVED — 2026-09-07 (human gate)** — artifacts reviewed by the approver (`tmp/p240b-pnp-raft-v4.gcode` + visual-debug bundle + Orca references); all checklist items above carry measured verdicts; certification 293/293 binaries (target/test-output.log). Status flipped to `implemented`; TASK-537 closed in docs/07.
