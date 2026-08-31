# Requirements: fuzzy-skin-keys

## Packet Metadata

- Grouped task IDs: none — queue packet (wayfinder precedent: packets 234a, 253–258 carry `task_ids: []`); implementation is recorded against wayfinder ticket 14.
- Backlog source: `docs/specs/orca-feature-gap/issues/14-author-packet-p07-others-fuzzy-skin-fuzzy-skin.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P07).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet P07 (Others / Fuzzy Skin — owner `fuzzy-skin`) is the next uncovered slice of the
OrcaSlicer FFF feature-gap queue (`05-asset-packet-list.md` §P07 — 7 keys, Tier A).
All seven keys have **zero occurrences as config keys** in `modules/`, `crates/`,
`resources/`, or `xtask/` (authoring-time tree survey). Two of them — `fuzzy_skin`
and `fuzzy_skin_mode` — do appear as **padding strings** in the `ORCA_CONFIG_PADDING`
table (`crates/slicer-gcode/src/serialize.rs`): `("fuzzy_skin", "none")` and
`("fuzzy_skin_mode", "displacement")`, emitted into the CONFIG_BLOCK only when the
key is absent from the resolved config. The `fuzzy_skin` padding value `"none"`
contradicts the canonical default `disabled_fuzzy` this packet declares and is
corrected in Step 3 (no entries gained or lost). The only other in-tree `fuzzy_skin`
spellings are the `WallFeatureFlags.fuzzy_skin` per-vertex paint flag and the 3MF
sidecar String allowlist in `crates/slicer-model-io/src/loader.rs`, neither a config
key. The owner module exists and already implements a point-displacement fuzzy
generator reading three of its own schema keys (`fuzzy_skin_thickness`,
`fuzzy_skin_point_distance`, `apply_to_all`), so declaring the new keys lands in a
live, shipped module rather than a stub. This is one coherent slice: one manifest,
one owner, one wiring pass, one generated-docs regeneration.

## In Scope

- Seven new `[config.schema]` tables in `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`:
  `fuzzy_skin`, `fuzzy_skin_first_layer`, `fuzzy_skin_mode`, `fuzzy_skin_noise_type`,
  `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_scale`, each with
  canonical defaults/bounds and `group = "Fuzzy Skin"` (AC-1).
- Manifest guard test file `modules/core-modules/fuzzy-skin/tests/fuzzy_config_schema_tdd.rs`
  (net-new), mirroring the part-cooling guard pattern (`cooling_config_schema_tdd.rs`);
  requires the `toml = "0.8"` dev-dependency add-if-absent in
  `modules/core-modules/fuzzy-skin/Cargo.toml`.
- Two live decision-point wirings in `modules/core-modules/fuzzy-skin/src/lib.rs`
  (`FuzzySkinModule::from_config` + `run_wall_postprocess`): `fuzzy_skin`'s
  loop-selection gate (which wall loops are fuzz candidates, per the canonical
  `FuzzySkinType` mapping) and `fuzzy_skin_first_layer`'s layer-0 gate (AC-2/3).
  `apply_fuzzy_skin` itself is unchanged.
- Test fallout in the existing module suites: `fuzzy_skin_tdd.rs` and
  `closed_loop_tdd.rs` run at layer 0 and rely on `apply_to_all` alone — both
  gates change that (default `disabled_fuzzy` is inert; layer 0 passes through at
  default `fuzzy_skin_first_layer = false`), so the existing perturbation tests
  are updated to set `fuzzy_skin = "all"` (or `"external"`) and run at layer 1
  (or set `fuzzy_skin_first_layer = true`), with measured justification in the
  step notes.
- Bounds/enum rejection arms in the existing scheduler integration binary
  `config_bounds_enforcement_tdd` and a CONFIG_BLOCK reachability arm in the
  existing runtime integration binary `gcode_header_thumbnail_config_blocks_tdd`
  (AC-4/5).
- One-line value correction in `crates/slicer-gcode/src/serialize.rs`
  `ORCA_CONFIG_PADDING`: the pre-existing `("fuzzy_skin", "none")` entry becomes
  `("fuzzy_skin", "disabled_fuzzy")` — the canonical default this packet declares;
  `fuzzy_skin_mode`'s `"displacement"` and the thickness/point-distance entries
  already match canonical and stay. No entries are gained or lost (AC-5).
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs` (AC-6).

## Out of Scope

- **Coherent-noise generation (`fuzzy_skin_noise_type` ≠ `classic`)** — canonical
  decision point is `FuzzySkin.cpp` `get_noise_module` (libnoise `Perlin`/`Billow`/
  `RidgedMulti`/`Voronoi` modules plus the `Ripple` dispatch to
  `fuzzy_polyline_ripple`/`fuzzy_extrusion_line_ripple`); the port's xorshift RNG
  is the `UniformNoise` (classic) analogue, so the default path is behaviorally
  faithful, and the five non-classic values have no in-tree implementation.
  Declared-with-gap, pinned in AC-N1 as non-perturbing.
- **Arachne extrusion-line width-modifying fuzz (`fuzzy_skin_mode`)** — canonical
  decision point is `FuzzySkin.cpp` `fuzzy_extrusion_line`'s `switch (cfg.mode)`
  (perpendicular offset / width set / both), which exists only on the
  `Arachne::ExtrusionLine` path; the port's module is a `fuzzy_polyline`
  (Polygon-path) port over `WallLoop` IR with no Arachne junction path. Default
  `displacement` matches the port's point-displacement algorithm. Declared-with-gap.
- **Hole-loop identification (`fuzzy_skin = "hole"` / `"all"`'s hole half)** — the
  IR's `LoopType` has no `Hole` variant (Outer/Inner/ThinWall/NonPlanarShell/GapFill),
  and classic-perimeters emits hole boundaries as `LoopType::Outer` walls at
  `perimeter_index 0` (offset result of the region polygon, bucketed by inset index
  only) — indistinguishable from the outer contour. `hole` is therefore inert and
  `all` degrades to `external` (contour only). Recorded divergence, not fixed
  (hole-loop identification is IR work, queue-sized).
- **Painted-region segmentation (canonical `apply_fuzzy_skin_segmentation`,
  `PrintObjectSlice.cpp`)** — canonical's `none` ("Painted only") value feeds
  regions segmented from fuzzy painting; the port's per-vertex
  `WallFeatureFlags.fuzzy_skin` path is its own painted-only mechanism (DEV-126:
  the flag is never written by production paint segmentation today, so the path is
  live only in tests). The packet wires `none` to the existing flag path and does
  not build region segmentation.
- **`apply_to_all` (the PnP-specific bool)** — untouched per ticket 07 (34
  Pinch-specific keys stay); its interaction with the new enum is defined in the
  wiring notes (it keeps its "ignore per-vertex flags" meaning, scoped to the
  loops the `fuzzy_skin` type selects).
- **`ORCA_CONFIG_PADDING` entry count** — the table gains and loses no entries;
  the single `fuzzy_skin` value correction (`"none"` → `"disabled_fuzzy"`) is in
  scope (AC-5) because the pre-existing value contradicts the canonical default
  this packet declares. The 254/255/257/258 read-only rule (no twins) holds.
- **`docs/ORCA_CONFIG_REFERENCE.md` hand-maintained column** — untouched (ticket
  07 ruling; the queue never reads it). Note: the snapshot's `fuzzy_skin` row
  (coEnum, "Disabled") is the hand-maintained column; the machine-read canonical
  default is `disabled_fuzzy` (verified against `PrintConfig.cpp` at authoring).
- **Tier-table row updates** (`04-asset-tier-assignment.md`) — ride ticket 14's
  closure (ticket 12/13 precedent), not this packet's files.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated (~1000 lines); delegated reads
  only. Regenerated by this packet (AC-6); never hand-edited.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK contract — delegated SUMMARY; governs
  the padding-touching prohibition in AC-5.
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a
  worker needs the `[config.schema]` contract; the enum `values` field form is
  grounded in-tree (`seam-planner-default.toml`, `tree-support-planner.toml`).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` — `should_fuzzify` (type/first-layer gates), `apply_fuzzy_skin` (Polygon + Arachne overloads), `fuzzy_polyline`, `fuzzy_extrusion_line` (mode switch), `get_noise_module` (noise construction), `fuzzy_polyline_ripple`/`fuzzy_extrusion_line_ripple`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the seven keys (types, defaults, min/max, enum order); authoring-time evidence already captured in §Per-Key Canonical Evidence and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `PRINT_CONFIG_CLASS_DEFINE(PrintRegionConfig, ...)` member list (all seven keys in `PrintRegionConfig`).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic`/`process_arachne` call sites, `group_region_by_fuzzify` (recorded-gap context for `none`).
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `apply_fuzzy_skin_segmentation` (recorded-gap context for `none`).

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

Canonical evidence per ticket 02's standard (delegated canonical read; in-tree
grounding from the authoring survey). Type/default/bounds columns record the
manifest contract AC-1 pins. All seven keys are declared in canonical
`PrintRegionConfig` (the per-region config class, `PrintConfig.hpp`
`PRINT_CONFIG_CLASS_DEFINE(PrintRegionConfig, ...)`); the port declares them
scalar-global in the owner manifest per the queue's established pattern (packet
254/255 precedent — the per-region model is the Tier-D fog's question, not this
packet's).

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `fuzzy_skin` | coEnum `FuzzySkinType` (`"none"`, `"external"`, `"hole"`, `"all"`, `"allwalls"`, `"disabled_fuzzy"`) | `disabled_fuzzy` | — | enum, values in canonical order | `FuzzySkin.cpp` `should_fuzzify` (type gates loop selection: `fuzzify_contours = (loop_idx == 0 && type != Hole) \|\| type == AllWalls`, `fuzzify_holes = (type == Hole \|\| type == All \|\| type == AllWalls) && (loop_idx == 0 \|\| type == AllWalls)`), called from `apply_fuzzy_skin` (Polygon + Arachne overloads) | **Wired (loop-selection gate)** — see wiring notes |
| `fuzzy_skin_first_layer` | coBool | `false` | — | bool, default false | `FuzzySkin.cpp` `should_fuzzify` (`if (!config.fuzzy_first_layer && layer_id <= 0) return false`) | **Wired (layer gate)** — layer 0 passes through at default (AC-3) |
| `fuzzy_skin_mode` | coEnum `FuzzySkinMode` (`"displacement"`, `"extrusion"`, `"combined"`) | `displacement` | — | enum, values in canonical order | `FuzzySkin.cpp` `fuzzy_extrusion_line` (`switch (cfg.mode)`: perpendicular offset / width set / both) — Arachne extrusion-line path only | **Declared-with-gap** — the port's module is a `fuzzy_polyline` (Polygon-path) port over `WallLoop` IR; no Arachne junction path exists; default `displacement` matches the port's point-displacement algorithm |
| `fuzzy_skin_noise_type` | coEnum `NoiseType` (`"classic"`, `"perlin"`, `"billow"`, `"ridgedmulti"`, `"voronoi"`, `"ripple"`) | `classic` | — | enum, values in canonical order | `FuzzySkin.cpp` `get_noise_module` (libnoise `Perlin`/`Billow`/`RidgedMulti`/`Voronoi` modules; `UniformNoise` for classic; Ripple dispatch to `fuzzy_polyline_ripple`/`fuzzy_extrusion_line_ripple`) | **Declared-with-gap** — the port's xorshift RNG is the `UniformNoise` (classic) analogue, so the default path is behaviorally faithful; the five non-classic modules have no in-tree implementation |
| `fuzzy_skin_octaves` | coInt, min 1 max 10 | `4` | [1, 10] | int, min 1, max 10 | `FuzzySkin.cpp` `get_noise_module` (`SetOctaveCount(cfg.noise_octaves)` on Perlin/Billow/RidgedMulti) | **Declared-with-gap** — consumed only by coherent modules; unused by classic |
| `fuzzy_skin_persistence` | coFloat, min 0.01 max 1 | `0.5` | [0.01, 1] | float, min 0.01, max 1 | `FuzzySkin.cpp` `get_noise_module` (`SetPersistence(cfg.noise_persistence)` on Perlin/Billow) | **Declared-with-gap** — consumed only by coherent modules; unused by classic |
| `fuzzy_skin_scale` | coFloat, min 0.1 max 500 | `1.0` | [0.1, 500] | float, min 0.1, max 500 | `FuzzySkin.cpp` `get_noise_module` (`SetFrequency(1 / cfg.noise_scale)` on coherent modules) | **Declared-with-gap** — consumed only by coherent modules; unused by classic |

### Wiring notes (port-specific decisions the canonical reads forced)

- **Loop-selection mapping.** Canonical `should_fuzzify` selects loops by
  `FuzzySkinType` against `loop_idx` (0 = outermost contour). The port's IR
  classifies walls by `LoopType` (Outer/Inner/ThinWall/NonPlanarShell/GapFill —
  no Hole variant) and `perimeter_index` (0 = outermost). The port's mapping:
  `disabled_fuzzy` → no candidates (inert); `external` → `LoopType::Outer`
  (perimeter_index 0); `all` → `LoopType::Outer` (canonical's hole half has no
  IR representation — recorded divergence); `allwalls` → every wall loop in the
  region; `none` → `LoopType::Outer` with the per-vertex
  `WallFeatureFlags.fuzzy_skin` gate (painted-only; the port's existing flag
  path); `hole` → no candidates (no hole-loop identification — recorded gap).
  Within selected loops, the existing `apply_to_all || flags.any(fuzzy_skin)`
  gate is unchanged.
- **Hole-boundary indistinguishability.** Classic-perimeters emits hole
  boundaries as `LoopType::Outer` walls at `perimeter_index 0` (the offset
  result of the region polygon, bucketed by inset index only —
  `all_wall_polygons` in `modules/core-modules/classic-perimeters/src/lib.rs`),
  so the module cannot tell a hole boundary from the outer contour. Canonical's
  `hole`/`all` values therefore cannot be wired faithfully: `hole` is inert and
  `all` degrades to `external` (contour only). Recorded divergence, not fixed —
  hole-loop identification is IR work, queue-sized.
- **First-layer gate.** Canonical `should_fuzzify` returns false for
  `layer_id <= 0` when `fuzzy_first_layer` is false. The port gates at the top of
  `run_wall_postprocess` (`layer_index == 0` → pass-through of every wall).
  Default false means layer 0 is never fuzzed — a canonical-alignment behavior
  change (the module previously fuzzed layer 0); the existing layer-0 tests are
  updated to layer 1 or `fuzzy_skin_first_layer = true` with measured
  justification in the step notes.
- **`apply_to_all` interaction.** `apply_to_all` (a PnP-specific key, untouched
  per ticket 07) keeps its documented meaning — perturb all selected walls
  regardless of per-vertex flags — scoped to the loops the `fuzzy_skin` type
  selects. With the enum at its default `disabled_fuzzy`, `apply_to_all = true`
  alone no longer fuzzes (canonical disabled semantics); the existing
  apply-to-all tests are updated to set `fuzzy_skin = "all"` alongside,
  preserving their intent (all outer walls perturbed regardless of flags).
- **Padding correction.** The pre-existing `ORCA_CONFIG_PADDING` entry
  `("fuzzy_skin", "none")` (`crates/slicer-gcode/src/serialize.rs`) is emitted
  into the CONFIG_BLOCK whenever `fuzzy_skin` is absent from the resolved config
  (the `emit_config_kv` dedup skips it when the key is explicitly set). Its
  value contradicts the canonical default `disabled_fuzzy` this packet declares;
  Step 3 corrects it to `("fuzzy_skin", "disabled_fuzzy")`. `fuzzy_skin_mode`'s
  `"displacement"` and the thickness/point-distance entries already match
  canonical and stay. No entries are gained or lost.
- **Tier A/B status:** AC-1/4/5/6 are Tier A plumbing (declare + default-matches +
  reaches-consumer); AC-2/3 are behavioural wirings inside the existing decision
  structure (loop selection, layer gate) — new logic in an existing owner, Tier B
  semantics carried by an A-shaped packet, mirroring how packets 257/258 wired
  their live gates.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest exactness), `AC-2` (enum loop selection),
  `AC-3` (first-layer gate), `AC-4` (bounds/enum rejection), `AC-5`
  (CONFIG_BLOCK single-emission + padding value correction + no-padding-twins),
  `AC-6` (generated docs).
- Negative: `AC-N1` (declared-with-gap keys non-perturbing), `AC-N2` (schema
  guard fails naming the drifted key).
- Cross-packet impact: packet 258 (P06, owner `skirt-brim`) precedes this packet
  in the queue — different module, no same-module ordering note. No other queued
  packet consumes `fuzzy-skin` keys. The seven keys appear in the generated
  doc-15 tables and in no deviation row (all manifest defaults canonical-identical).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p fuzzy-skin --test fuzzy_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1/N2 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p fuzzy-skin --test fuzzy_skin_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2/3/N1 loop-selection + layer gate + gap keys | FACT pass/fail |
| `cargo test -p fuzzy-skin --test closed_loop_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | updated layer-0 fixtures still green | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 enum/bounds rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5 CONFIG_BLOCK emission | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-6 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest + src edits) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

Only cross-step invariants: the manifest tables (Step 1) must land before the
guard test (Step 1, same step) and before the wiring step reads the keys; the
wiring step (Step 2) keeps the default paths byte-identical (AC-2's
`disabled_fuzzy` arm, AC-3's layer-0 arm, AC-N1); Step 4's regeneration runs
after the manifest is final. The existing `fuzzy_skin_tdd.rs` and
`closed_loop_tdd.rs` suites must be updated in the same step as the gates that
break them (Step 2) — never left red for a later step. None beyond that.

## Context Discipline Notes

- `docs/15_config_keys_reference.md` is generated and ~1000 lines — never load
  it; verify via `--check` and targeted `rg` (AC-6).
- `FuzzySkinModule` (`modules/core-modules/fuzzy-skin/src/lib.rs`) is ~336 lines —
  workers may read it in full (it is the change surface), but `OrcaSlicerDocumented/`
  reads stay delegated per the snippet.
- Do not read `crates/slicer-gcode/src/serialize.rs` beyond the padding-entry
  question; `ORCA_CONFIG_PADDING` is read-only context and must not be edited
  (AC-5 pins the no-twins invariant).
- The 3MF sidecar String allowlist for `fuzzy_skin` (`crates/slicer-model-io/src/loader.rs`)
  is a per-object metadata classification, separate from the module config key;
  it is read-only context and must not be edited by this packet.

Packet-specific: none further.
