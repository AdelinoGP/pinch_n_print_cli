# Requirements: raft-keys

## Packet Metadata

- Grouped task IDs: none — queue packet (wayfinder precedent: packets 234a, 253–260 carry `task_ids: []`); implementation is recorded against wayfinder ticket 19.
- Backlog source: `docs/specs/orca-feature-gap/issues/19-author-packet-p12-support-raft-support-planner.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P12).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet P12 (Support / Raft — owner `support-planner`) is the next uncovered slice of
the OrcaSlicer FFF feature-gap queue (`05-asset-packet-list.md` §P12 — 2 keys, Tier A).
Authoring-time re-derivation against the tree confirms the tier table's owner and
re-adjudicates the tier:

- **Both keys are zero-occurrence gaps** — `raft_contact_distance` (canonical coFloat,
  default 0.1, min 0, no max) and `raft_expansion` (canonical coFloat, default 1.5,
  min 0, no max) appear nowhere in `modules/`, `crates/`, `xtask/`, or
  `docs/15_config_keys_reference.md`. No raft geometry generator exists in-tree: the
  `com.core.raft-default` module of draft packet 240-support-raft (support-families
  plan) is unimplemented, and the `RaftPlan` record (`crates/slicer-ir/src/slice_ir.rs`
  `RaftPlan`) carries only layer counts (`raft_layers`, `raft_first_layer_density`,
  `base_raft_layers`, `interface_raft_layers`) — no contact distance, no expansion.
- **The owner `support-planner` is confirmed, narrowed to one claim holder.**
  `tree-support-planner` owns the raft config cluster: it declares
  `support_raft_layers` / `raft_first_layer_density` / `base_raft_layers` /
  `interface_raft_layers` in `tree-support-planner.toml`, reads them in
  `SupportPlanner::from_config` (`modules/core-modules/tree-support-planner/src/lib.rs`),
  and emits one configuration-only `RaftPlan` when `support_raft_layers > 0`.
  `traditional-support-planner` has **no raft surface at all** — no raft keys declared,
  no `RaftPlan` emitted, and the traditional-family geometry module (`traditional-support`)
  has no raft handling either. Raft is tree-family-only in this port (a port state, not a
  canonical one — canonical supports raft for both families). The packet declares in
  `tree-support-planner.toml` and pins the traditional omission.
- **The decision points do not exist** — canonical consumes `raft_contact_distance` as
  the Z gap between raft and object (`SlicingParameters::SlicingParameters` in
  `Slicing.cpp`: `raft_z_gap` → `gap_raft_object` → `object_print_z_min =
  raft_contact_top_z + gap_raft_object`, forced to 0 when `raft_z_gap == 0.0 ||
  zero_topZ_contact`; plus the `GCode.cpp` `_print_z` support-gap warning rounding) and
  `raft_expansion` as the XY expansion of the raft footprint
  (`SupportMaterial::generate_contact_polygons` layer_id==0 branch:
  `contact_polygons = raft_expansion > 0 ? expand(overhang_polygons,
  scaled(raft_expansion)) : overhang_polygons`; `TreeSupport3D::generate_raft_contact`
  and `finalize_raft_contact`). Neither decision point exists in this tree, so both keys
  are re-adjudicated **declared-with-gap** (packet 260's pattern-key precedent), not
  wired.

No user rulings were required: the keys are net-new declarations with canonical defaults
(0.1 / 1.5) and canonical bounds (min 0, no max — the in-tree `max_bridge_length` table
is the no-max float precedent), so nothing is aligned away and no divergence is created.

## In Scope

- Two `[config.schema]` tables in `modules/core-modules/tree-support-planner/
  tree-support-planner.toml`: `raft_contact_distance` (float, default 0.1, min 0.0, no
  max, `display = "Raft Contact Distance"`, `group = "Support"`) and `raft_expansion`
  (float, default 1.5, min 0.0, no max, `display = "Raft Expansion"`,
  `group = "Support"`), each with a `description` comment recording the decision-point
  gap and the canonical consumer (AC-1/AC-N1).
- Manifest guard test file `modules/core-modules/tree-support-planner/tests/
  raft_config_schema_tdd.rs` (net-new, mirroring the part-cooling guard pattern
  `cooling_config_schema_tdd.rs` and packet 260's `support_config_schema_tdd.rs`);
  requires the `toml = "0.8"` dev-dependency add-if-absent in the module's `Cargo.toml`
  (verified absent at authoring). The guard also pins AC-N2: `traditional-support-planner.toml`
  does NOT declare the two keys (deliberate omission).
- Non-perturbation arms in the existing module suite `orca_parity_tdd.rs` (AC-2: explicit
  `raft_contact_distance = 0.5` / `raft_expansion = 3.0` produce byte-identical
  `SupportPlanIR` entries + `RaftPlan` to absent keys — `SupportPlanEntry` and `RaftPlan`
  both derive `PartialEq`).
- Bounds/enum rejection arms in the existing scheduler integration binary
  `config_bounds_enforcement_tdd.rs` (AC-3: `OutOfRange` on `-0.5` / `-1.0`, `TypeMismatch`
  on `"abc"` for the float key) and a CONFIG_BLOCK reachability arm in the existing
  runtime integration binary `gcode_header_thumbnail_config_blocks_tdd.rs` (AC-4).
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`
  (AC-5).

## Out of Scope

- **Raft geometry implementation** — canonical's decision points for both keys live in
  the raft generator (`SlicingParameters` Z-gap math, `generate_contact_polygons` /
  `generate_raft_contact` XY expansion, `SupportCommon.cpp` `generate_raft_base` layer
  construction). The port has no raft geometry generator: draft packet 240-support-raft
  (support-families plan) owns `com.core.raft-default` and plans to declare these keys in
  its manifest and wire them to geometry (its AC-5 requires a written wire-or-record
  decision for the four support-module manifests — this packet's declarations and the
  traditional omission pin are that record's input). Declared-with-gap; AC-2 pins
  non-perturbation.
- **Wiring the keys into `RaftPlan`** — adding `raft_contact_distance` / `raft_expansion`
  fields to the WIT record (`crates/slicer-schema/wit/deps/prepass-support-geometry/
  prepass-support-geometry.wit` `raft-plan`) would be speculative pre-wiring for a module
  that does not exist, would force a WIT change + guest rebuilds, and duplicates packet
  240's plan (raft-default reads config directly). Rejected; the keys ride the existing
  `ConfigView` plumbing.
- **`raft_first_layer_expansion`** — the third raft key is P13's surface (queue packet
  list §P13), not this packet's; read-only context here.
- **`traditional-support-planner.toml`** — deliberately untouched beyond the omission
  pin (AC-N2): the traditional family has no raft surface in this port. A future packet
  that wires raft for the traditional family must update the guard.
- **`SUPPORT_CONFIG_DEFAULTS` / `ORCA_CONFIG_PADDING`** (`crates/slicer-gcode/src/
  serialize.rs`) — verified at authoring to contain neither key (`ORCA_CONFIG_PADDING`
  carries `("raft_layers", "0")` — the canonical layer-count key, not these two); no
  twins are added (254/255/257/258/259/260 precedent), so at defaults neither key appears
  in the CONFIG_BLOCK (AC-4). No padding edits.
- **The `raft_layers` 1→3 split** (`support_raft_layers` + `base_raft_layers` +
  `interface_raft_layers`) — a recorded strict-superset divergence (map ticket 07 ruling),
  untouched; the existing planner raft cluster is read-only context.
- **`docs/ORCA_CONFIG_REFERENCE.md` hand-maintained column** — untouched (ticket 07
  ruling; the queue never reads it).
- **Tier-table row updates** (`04-asset-tier-assignment.md`) — ride ticket 19's closure
  (ticket 12/13/14/18 precedent), not this packet's files.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated (~1000 lines); delegated reads only.
  Regenerated by this packet (AC-5); never hand-edited.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK contract — delegated SUMMARY; governs AC-4's
  no-padding-twins prohibition.
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker
  needs the `[config.schema]` contract; the no-max float form is grounded in-tree
  (`tree-support-planner.toml` `[config.schema.max_bridge_length]`).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the two keys (`raft_contact_distance` coFloat default 0.1 min 0 no max; `raft_expansion` coFloat default 1.5 min 0 no max; both on `PrintObjectConfig`). Authoring-time evidence already captured in §Per-Key Canonical Evidence (dispatched canonical reads, 2026-08-31) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `SlicingParameters::SlicingParameters` (the raft Z-gap: `raft_z_gap` → `gap_raft_object` → `object_print_z_min = raft_contact_top_z + gap_raft_object`; forced to 0 when `raft_z_gap == 0.0 || zero_topZ_contact`).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `generate_contact_polygons` (layer_id==0 branch: `contact_polygons = raft_expansion > 0 ? expand(overhang_polygons, scaled(raft_expansion)) : overhang_polygons`).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport3D.cpp` — `generate_raft_contact` / `finalize_raft_contact` (raft-contact expansion and tree-tip culling inside the expanded raft).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_print_z` / support-gap warning logic (rounds `raft_contact_distance` up to layer height as `extra_gap` when the last extrusion layer is a support layer).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::invalidate_state_by_config_options` (support invalidation for the two keys).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Per-Key Canonical Evidence

Canonical evidence per ticket 02's standard (delegated canonical reads, 2026-08-31; in-tree
grounding from the authoring survey). Type/default/bounds columns record the manifest
contract AC-1 pins. Canonical declares both keys on `PrintObjectConfig`
(`PrintConfig.cpp` `build_print_config_options`); the port declares them scalar-global in
the owner manifest per the queue's established pattern (packet 254/255/257/258/259/260
precedent — the per-filament/per-object model is the Tier-D fog's question, not this
packet's).

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `raft_contact_distance` | coFloat | `0.1` | min 0, no max | float, default 0.1, min 0.0, no max | `Slicing.cpp` `SlicingParameters::SlicingParameters` (`raft_z_gap` → `gap_raft_object` → `object_print_z_min = raft_contact_top_z + gap_raft_object`; forced to 0 when `raft_z_gap == 0.0 \|\| zero_topZ_contact`); `GCode.cpp` `_print_z` support-gap warning (rounds up to layer height as `extra_gap` when the last extrusion layer is a support layer); `PrintObject::invalidate_state_by_config_options` | **Declared-with-gap** — zero occurrences in the tree at authoring; the raft Z-gap decision point lives in the absent raft geometry generator (draft packet 240). The "ignored for soluble interface" tooltip is GUI-only (`ConfigManipulation.cpp` disables the field for soluble support; no slicing branch) — recorded, not ported |
| `raft_expansion` | coFloat | `1.5` | min 0, no max | float, default 1.5, min 0.0, no max | `SupportMaterial.cpp` `generate_contact_polygons` (layer_id==0: `contact_polygons = raft_expansion > 0 ? expand(overhang_polygons, scaled(raft_expansion)) : overhang_polygons`); `TreeSupport3D.cpp` `generate_raft_contact` / `finalize_raft_contact` (raft-contact expansion; tree-tip culling inside the expanded raft); `PrintObject::invalidate_state_by_config_options` | **Declared-with-gap** — zero occurrences in the tree at authoring; the raft-footprint expansion decision point lives in the absent raft geometry generator (draft packet 240) |

### Wiring notes (port-specific decisions the canonical reads forced)

- **Declared-with-gap keys are unread.** Neither key is read in
  `tree-support-planner/src/lib.rs` — declaring them must not perturb behavior (AC-2 pins
  byte-identity for explicit values vs absent). This is the packet 259/260 pattern; the
  canonical consumers are recorded above so a future geometry packet (240) can consume
  the declarations.
- **Owner narrowed, not corrected.** The tier table's owner `support-planner` is the
  claim held by both planner modules, but only `tree-support-planner` has raft surface
  (the raft config cluster + `RaftPlan` emission); `traditional-support-planner` declares
  no raft keys and emits no `RaftPlan`, and the traditional-family geometry module has no
  raft handling. The packet declares in `tree-support-planner.toml` and pins the
  traditional omission (AC-N2) — the 04 owner column stays `support-planner` unchanged.
- **Tier re-adjudication.** Both keys are re-adjudicated declared-with-gap: their
  decision points (raft Z-gap, raft-footprint expansion) do not exist — the tier-table A
  row is corrected at authoring, mirroring P11's pattern-key finding. The owner column is
  not corrected (it was right); the disposition is recorded here and in the ticket
  closure.
- **Canonical bounds adopted, no divergence created.** Canonical declares min 0, no max
  on both keys; the manifest declares `min = 0.0` with no `max` (the in-tree
  `max_bridge_length` table is the no-max float precedent; the schema parser's
  `get_float_opt` and the bounds index's `None`-as-unbounded handling make this legal).
  Unlike packet 260's spacing keys (which kept a port `max = 2.0`), net-new declarations
  adopt canonical bounds outright — no declared-bounds divergence row.
- **CONFIG_BLOCK.** Neither key rides `SUPPORT_CONFIG_DEFAULTS` or `ORCA_CONFIG_PADDING`
  in `crates/slicer-gcode/src/serialize.rs` (verified at authoring: the padding list
  carries `("raft_layers", "0")` — the canonical layer-count key, not these two). At
  defaults the block therefore carries zero `raft_contact_distance` / `raft_expansion`
  lines; explicit values reach it once through the raw_config sorted dump
  (`serialize_config_block` + `emit_config_kv` dedup) — AC-4 pins both states; no padding
  twins are added (254/255/257/258/259/260 precedent).
- **Tier A/B status:** AC-1/3/4/5 are Tier A plumbing (declare + default-matches +
  reaches-consumer); AC-2 is the declared-with-gap non-perturbation pin; AC-N1/N2 are the
  guard arms.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest exactness), `AC-2` (declared-with-gap non-perturbation),
  `AC-3` (bounds/type rejection), `AC-4` (CONFIG_BLOCK states), `AC-5` (generated docs:
  keys present, deviation block unchanged at 27).
- Negative: `AC-N1` (schema guard fails naming the drifted key), `AC-N2` (traditional
  omission pinned).
- Cross-packet impact: packets 254/255/257/258/259/260 precedent governs the
  no-padding-twins rule; draft packet 240-support-raft is the future consumer of these
  declarations (its AC-5 wire-or-record input). No other queued packet consumes these two
  keys. Post-packet doc-15 state: 2 new module-key rows under `tree-support-planner`;
  deviation block unchanged (27 rows).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p tree-support-planner --test raft_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1/N1/N2 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p tree-support-planner --test orca_parity_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 non-perturbation arms | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3 bounds/type rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 CONFIG_BLOCK emission | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-5 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest edit) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

Only cross-step invariants: the manifest tables and guard test (Step 1) land before the
non-perturbation arms read the keys (Step 2); the integration arms (Step 3) run after
both; Step 4's regeneration runs after the manifest is final. None beyond that.

## Context Discipline Notes

- `docs/15_config_keys_reference.md` is generated and ~1000 lines — never load it; verify
  via `--check` and targeted `rg`/`sed` (AC-5).
- `tree-support-planner/src/lib.rs` is read-only context beyond the raft `from_config`
  reads (~1600 lines — ranged reads only); the module test files (`orca_parity_tdd.rs`
  ~1300 lines, `tree_family_tdd.rs` ~700 lines) are bounded full reads or ranged reads.
- `crates/slicer-gcode/src/serialize.rs` is read-only context beyond the
  `serialize_config_block`/`SUPPORT_CONFIG_DEFAULTS`/`ORCA_CONFIG_PADDING` questions
  (AC-4); no padding edits.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (~460
  lines) and `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (~1040 lines) — bounded full read for the former, ranged reads for the latter.
- Do not read the traditional-family modules' sources — the omission is context, not
  surface.
