# Requirements: support-interface-keys

## Packet Metadata

- Grouped task IDs: none — queue packet (wayfinder precedent: packets 234a, 253–259 carry `task_ids: []`); implementation is recorded against wayfinder ticket 18.
- Backlog source: `docs/specs/orca-feature-gap/issues/18-author-packet-p11-support-interface-support-planner.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P11).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet P11 (Support / Interface — owner `support-planner`) is the next uncovered slice of
the OrcaSlicer FFF feature-gap queue (`05-asset-packet-list.md` §P11 — 4 keys, Tier A).
Authoring-time re-derivation against the tree overturns the tier table's one-owner picture,
as the map's "re-derive, don't trust" rule warns:

- **Two keys are already live and correctly wired** — `support_interface_spacing` and
  `support_bottom_interface_spacing` are declared in both `traditional-support.toml` and
  `tree-support.toml` and consumed by both modules' `from_config` + `pitches_mm`
  (`modules/core-modules/{traditional-support,tree-support}/src/lib.rs`), which derive the
  interface scan-line pitch through `slicer_core::support_regularize::{interface_density,
  bottom_interface_density}` — a formula canonically identical to `SupportParameters`'
  `top_interface_density = min(1, flow.spacing()/top_interface_spacing)`. What remains is a
  **mis-derived default** (0.4 in the port, canonical 0.5 — the port's own comments claim
  Orca's default is 0.4; packet 238c already corrected the bottom key to 0.5) and a
  **PnP-invented `-1` mirror sentinel** on the bottom key (canonical has no sentinel; min 0;
  the canonical `-1` sentinel belongs to a different key — `support_interface_bottom_layers`).
- **Two keys are zero-occurrence gaps** — `support_interface_pattern` (canonical coEnum
  `SupportMaterialInterfacePattern`, default `auto`: the interface fill-pattern dispatch)
  and `support_interface_loop_pattern` (canonical **coBool** default `false` — a type
  correction against the common-but-wrong enum assumption; the contact-loop processor)
  appear nowhere in `modules/`, `crates/`, `xtask/`, or `resources/`.

The owner correction: the tier table's owner `support-planner` is the claim held by
`tree-support-planner` and `traditional-support-planner`, but those planners emit
`SupportPlanIR` geometry and never read interface configuration; the decision points live in
the two support geometry modules. The packet declares in the decision-point modules and
records the correction.

User rulings (2026-08-31): align `support_interface_spacing` default 0.4 → 0.5 (canonical;
removes two doc-15 deviation rows); **keep** the `-1` bottom-interface mirror as a recorded
divergence (not aligned away).

## In Scope

- Four `[config.schema]` tables per manifest, in both
  `modules/core-modules/traditional-support/traditional-support.toml` and
  `modules/core-modules/tree-support/tree-support.toml`: the two existing spacing tables
  updated to the aligned `support_interface_spacing` default (0.5, with the
  `DEFAULT_INTERFACE_SPACING_MM` fallback const and its "matches Orca" comment corrected to
  0.5 in both `src/lib.rs` files), and the two net-new pattern tables
  (`support_interface_pattern` enum with canonical values/order/default; the
  `support_interface_loop_pattern` bool) — each with `display` and `group = "Support"`
  (AC-1/AC-N2).
- Manifest guard test files `modules/core-modules/traditional-support/tests/
  support_config_schema_tdd.rs` and `modules/core-modules/tree-support/tests/
  support_config_schema_tdd.rs` (net-new, mirroring the part-cooling guard pattern
  `cooling_config_schema_tdd.rs`); requires the `toml = "0.8"` dev-dependency add-if-absent
  in both modules' `Cargo.toml` (verified absent at authoring).
- Default-alignment invariant arms in the existing module suites `traditional_support_tdd.rs`
  and `tree_support_tdd.rs` (AC-2: absent key == explicit 0.5 count; sparser than 0.4),
  mirror-divergence witness arms (AC-3: `-1` == absent == explicit-0.8-with-top-0.8), and
  declared-with-gap non-perturbation arms (AC-N1: pattern values + loop=true byte-identical
  to absent).
- The `orca-matched-config.json` fixture (`crates/slicer-runtime/tests/fixtures/
  support-family/`) value `"support_interface_spacing": 0.4` → `0.5` and the fallout in its
  consumer `crates/slicer-runtime/tests/integration/support_family_closure.rs` (any interface
  block-count expectation that depended on the 0.4 default is re-measured with measured
  justification).
- Bounds/enum rejection arms in the existing scheduler integration binary
  `config_bounds_enforcement_tdd` (AC-4: enum/bool `TypeMismatch`, `OutOfRange` on the two
  floats, plus the positive arm that `support_bottom_interface_spacing = -1.0` stays legal)
  and a CONFIG_BLOCK reachability arm in the existing runtime integration binary
  `gcode_header_thumbnail_config_blocks_tdd` (AC-5).
- Short manifest comments above the two new pattern tables and a divergence comment above
  the kept `support_bottom_interface_spacing` min (tree-support-planner.toml's
  packet-comment precedent).
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`
  (AC-6).

## Out of Scope

- **`support_interface_pattern` dispatch** — canonical decision point is the
  `contact_fill_pattern` branch order in `SupportParameters::SupportParameters` plus the
  filler construction in `SupportCommon.cpp` `generate_support_toolpaths` (grid→`FillGrid`,
  rectilinear_interlaced→`FillRectilinear` with ±45° angle alternation via
  `support_interface_angle()`, concentric and auto-with-zero-gap→`FillConcentric`,
  density > 0.95→`FillRectilinear`, else `ipSupportBase` — the `FillSupportBase ::
  FillRectilinear` filler at `spacing/density`). The port's interface generation is a single
  scan-line path with a universal 90°-per-layer alternation; no pattern dispatch, angle
  specialization (snug −45°, interlaced ±45°, grid = `base_angle`), or concentric/grid
  generators exist. Declared-with-gap; AC-N1 pins non-perturbation. Note the structural
  closeness: canonical's default `auto` at this port's density regime resolves to
  `ipSupportBase`, a rectilinear-derived filler parameterized `spacing/density` — the same
  family as the port's scan-line pitch — so the divergence is in dispatch/angle fidelity,
  not in fundamental fill shape.
- **`support_interface_loop_pattern` implementation** — canonical consumer is
  `LoopInterfaceProcessor` (`n_contact_loops = value ? 1 : 0` in `generate_support_toolpaths`;
  `SupportMaterial::has_contact_loops`); the port has no contact-loop generator. Declared
  with-gap; the canonical type is **coBool** (default `false`), recorded as a correction
  against the snapshot row's `coBool`-adjacent hand-column ambiguity and any enum-model
  assumption. Disabled at default → byte-identical output.
- **The kept `-1` mirror sentinel alignment** — ruled OUT by the user (2026-08-31):
  canonical `support_bottom_interface_spacing` is min 0 with no sentinel, but the PnP
  mirror is retained and pinned by AC-3 as an intended divergence. Documented in the
  manifest comment and this requirements table.
- **Bounds alignment on the two spacing keys** — canonical has no max on either key
  (min 0); the port's declared `max = 2.0` (and the kept `min = -1.0` on the bottom key) are
  port-specific bounds. Recorded as declared-bounds divergences, not changed: the max cap
  does not interact with defaults, and widening bounds is a separate schema decision with no
  queue backing. (Packet 100's `wipe_tower_max_purge_speed` min-10 note is the same class.)
- **`SUPPORT_CONFIG_DEFAULTS` / `ORCA_CONFIG_PADDING`** (`crates/slicer-gcode/src/
  serialize.rs`) — verified at authoring to contain none of the four keys; no twins are
  added (254/255/257/258/259 precedent), so at defaults none of the four appears in the
  CONFIG_BLOCK (AC-5). No padding edits.
- **The planners (`tree-support-planner` / `traditional-support-planner`)** — they hold the
  `support-planner` claim (the tier table's owner) but are not this packet's surface; the
  owner correction is recorded, not acted on.
- **`support_interface_top_layers` / `support_interface_bottom_layers` /
  `support_interface_flow` / `support_interface_filament` / `support_base_pattern(_spacing)`**
  — live support-interface keys owned by other queue packets; read-only context.
- **`docs/ORCA_CONFIG_REFERENCE.md` hand-maintained column** — untouched (ticket 07
  ruling; the queue never reads it).
- **Tier-table row updates** (`04-asset-tier-assignment.md`) — ride ticket 18's closure
  (ticket 12/13/14 precedent), not this packet's files.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated (~1000 lines); delegated reads only.
  Regenerated by this packet (AC-6); never hand-edited.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK contract — delegated SUMMARY; governs AC-5's
  no-padding-twins prohibition.
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker
  needs the `[config.schema]` contract; the enum `values` field form is grounded in-tree
  (`tree-support-planner.toml` `[config.schema.support_style]`).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the four keys (types, defaults, min/max, enum value order via `get_enum_values()`; the coBool `support_interface_loop_pattern`); the `-1`-sentinel contrast: canonical `support_interface_bottom_layers` has it, `support_bottom_interface_spacing` does not. Authoring-time evidence already captured in §Per-Key Canonical Evidence (dispatched canonical reads, 2026-08-31) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `SupportMaterialInterfacePattern` enum member list and the `ConfigOptionBool` member for the loop key.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters(const PrintObject&)` (density formulas; `contact_fill_pattern` branch order) and `support_interface_angle()` (per-pattern angle math).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths` (filler construction, `LoopInterfaceProcessor` `n_contact_loops`) and `LoopInterfaceProcessor::generate`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportMaterial::has_contact_loops`.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::generate_toolpaths` (tree-family spacing/density handling).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::invalidate_state_by_config_options` (support invalidation).

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
contract AC-1 pins. Canonical declares all four keys on `PrintObjectConfig`
(`PrintConfig.cpp` `build_print_config_options`); the port declares them scalar-global in
the two owner manifests per the queue's established pattern (packet 254/255/257/258/259
precedent — the per-filament/per-object model is the Tier-D fog's question, not this
packet's).

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `support_interface_spacing` | coFloat | `0.5` (port had 0.4 — **aligned**) | min 0, no max (port keeps max 2.0) | float, default 0.5, min 0, max 2 | `SupportParameters::SupportParameters` (`top_interface_spacing = (ironing ? 0 : value) + flow.spacing()`, `top_interface_density = min(1, flow.spacing()/top_interface_spacing)`); `TreeSupport::generate_toolpaths` (same formula); `PrintObject::invalidate_state_by_config_options` | **Wired + default aligned** — the port's `slicer_core::support_regularize::{interface_density}` is the same formula (`flow/(gap+flow)`, capped at 1.0) and `pitches_mm` derives the pitch from it in both modules; AC-2 proves the aligned default reaches the decision point |
| `support_bottom_interface_spacing` | coFloat | `0.5` (matches port) | min 0, **no -1 sentinel** (port keeps `min -1.0` + mirror — recorded divergence, user ruling) | float, default 0.5, min −1.0, max 2 | `SupportParameters::SupportParameters` (`bottom_interface_spacing = value + flow.spacing()`, `bottom_interface_density = min(1, flow.spacing()/bottom_interface_spacing)`); `TreeSupport::generate_toolpaths`; `PrintObject::invalidate_state_by_config_options` | **Wired + divergence pinned** — port's `bottom_interface_density` mirrors the formula; the `< 0.0 → top gap` branch in both modules' `pitches_mm` is a PnP extension (canonical treats the value directly, min 0). Recorded divergence per user ruling; AC-3 locks it as a witness, AC-4 keeps `-1.0` legal in the bounds index |
| `support_interface_pattern` | coEnum `SupportMaterialInterfacePattern` (`auto`, `rectilinear`, `concentric`, `rectilinear_interlaced`, `grid`) | `auto` | — | enum, values in canonical order, default `auto` | `SupportParameters::SupportParameters` (`contact_fill_pattern` branch order: smipGrid→ipGrid; smipRectilinearInterlaced→ipRectilinear; (smipAuto ∧ zero-gap) ∨ smipConcentric→ipConcentric; contact_interface_density > 0.95→ipRectilinear; else ipSupportBase) + `support_interface_angle()` (snug −45°; interlaced ±45° by parity; grid = base_angle); `SupportCommon.cpp` `generate_support_toolpaths` (filler construction via `Fill::new_from_type`) | **Declared-with-gap** — zero occurrences in the tree at authoring; the port's single scan-line generator is the rectilinear family (canonical's sparse-density default resolves to `ipSupportBase`, a `FillSupportBase : FillRectilinear` filler at `spacing/density`, so the port's pitch parameterization is structurally faithful) but no pattern dispatch or angle specialization exists. Explicit `rectilinear` is behaviorally faithful by construction; `concentric`/`grid`/`rectilinear_interlaced` need new generators (Tier B+ geometry work, queue rows) |
| `support_interface_loop_pattern` | **coBool** (type correction: not an enum) | `false` | — | bool, default false | `SupportCommon.cpp` `generate_support_toolpaths` (`loop_interface_processor.n_contact_loops = config.support_interface_loop_pattern.value ? 1 : 0`), `LoopInterfaceProcessor::generate`; `SupportMaterial.hpp` `SupportMaterial::has_contact_loops` (returns the bool) | **Declared-with-gap** — zero occurrences in the tree at authoring; canonical type is bool with no contact-loop generator in the port. Disabled at default → byte-identical output; explicit `true` pins non-perturbation (AC-N1) |

### Wiring notes (port-specific decisions the canonical reads forced)

- **Default alignment (user ruling).** Canonical `support_interface_spacing` default is
  0.5; the port shipped 0.4 in both modules with comments claiming Orca's default is
  0.4 (mis-derived — packet 238c already corrected the bottom key to 0.5 in the same
  family pass). Step 2 aligns: both toml defaults, both `DEFAULT_INTERFACE_SPACING_MM` consts
  and their comments, the `orca-matched-config.json` fixture, and any test expectation that
  pinned the 0.4 default (re-measured with justification). The change makes default output
  slightly sparser (interface pitch = gap + flow spacing grows by 0.1 mm) and removes the
  two known doc-15 deviation rows (`support_interface_spacing` × both modules; the
  pre-packet deviation block measures 27 data rows at authoring — AC-6 pins 25 after).
- **Mirror sentinel kept (user ruling).** Canonical `support_bottom_interface_spacing`
  has no `-1` sentinel (min 0, independent default 0.5); the port's
  `bottom_interface_spacing_mm < 0.0 → top gap` mirror in both modules' `pitches_mm` is a
  PnP extension modeled on a *different* canonical key (`support_interface_bottom_layers`,
  which does carry the `-1 == same as top` sentinel). Per the user ruling it is retained and
  recorded, not aligned away: the manifest keeps `min = -1.0`, AC-3 pins the mirror as a
  witness invariant, AC-4 asserts `-1.0` stays legal in the bounds index, and a manifest
  comment documents the divergence on both modules.
- **Declared-with-gap keys are unread.** Neither pattern key is read in either module's
  `src/lib.rs` — declaring them must not perturb behavior (AC-N1 pins byte-identity for
  `concentric`/`grid`/`rectilinear_interlaced`/loop-true vs absent). This is the packet
  259 pattern; the canonical consumers are recorded above so a future geometry packet can
  consume the declarations.
- **Owner correction.** The tier table's owner `support-planner` is the claim held by the
  two planner modules, but neither planner reads interface configuration (they emit
  `SupportPlanIR`); the interface decision points live in `traditional-support` and
  `tree-support` (both read the spacing keys and emit `SupportInterface` role paths). The
  packet declares in the decision-point modules; the `04-asset-tier-assignment.md` owner row
  correction rides ticket 18's closure.
- **Tier re-adjudication.** The two spacing keys are genuinely Tier A plumbing (owner +
  decision point exist; AC-2/3 prove reach). The two pattern keys are re-adjudicated
  declared-with-gap: their decision points (pattern dispatch, contact-loop processor) do not
  exist — the tier-table A row is corrected at authoring, mirroring P02/P03's
  one-live-key-out-of-thirteen findings.
- **Bounds divergences recorded, not changed.** Canonical declares no max on either
  spacing key (min 0 on both); the port's `max = 2.0` (both keys) and the retained
  `min = -1.0` (bottom) are declared-bounds divergences. The max cap is conservative and
  default-agnostic; widening it is a separate schema decision with no queue backing.
- **CONFIG_BLOCK.** None of the four keys rides `SUPPORT_CONFIG_DEFAULTS` or
  `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs` (verified at authoring:
  the support defaults list is `support_expansion`/`support_top_z_distance`/
  `support_bottom_z_distance` only). At defaults the block therefore carries zero
  `support_interface_*` lines; explicit values reach it once through the raw_config sorted
  dump (`serialize_config_block` + `emit_config_kv` dedup) — AC-5 pins both states; no
  padding twins are added (254/255/257/258/259 precedent).
- **Tier A/B status:** AC-1/4/5/6 are Tier A plumbing (declare + default-matches +
  reaches-consumer); AC-2/3 are invariant arms on existing decision points; AC-N1 is the
  declared-with-gap non-perturbation pin.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest exactness ×2 modules), `AC-2` (aligned default reaches the
  pitch decision point), `AC-3` (retained mirror divergence locked), `AC-4` (bounds/enum
  rejection + sentinel legality), `AC-5` (CONFIG_BLOCK states), `AC-6` (generated docs:
  pattern keys present, deviation block 27 → 25).
- Negative: `AC-N1` (declared-with-gap keys non-perturbing), `AC-N2` (schema guard fails
  naming the drifted key).
- Cross-packet impact: packets 254/255/257/258/259 precedent governs the no-padding-twins
  rule; packet 238c is the origin of the wiring this packet verifies. No other queued packet
  consumes these four keys. Post-packet doc-15 state: pattern keys gain 4 module-key rows
  (2 keys × 2 modules), spacing rows show 0.5, deviation block loses exactly 2 rows.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p traditional-support --test support_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1/N2 manifest guard (traditional) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p tree-support --test support_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1/N2 manifest guard (tree) | FACT pass/fail |
| `cargo test -p traditional-support --test traditional_support_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2/3/N1 traditional arms | FACT pass/fail |
| `cargo test -p tree-support --test tree_support_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2/3/N1 tree arms | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 enum/bool/bounds rejection + sentinel legality | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5 CONFIG_BLOCK emission | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration support_family_closure 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | fixture-default fallout green | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-6 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest + src edits) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

Only cross-step invariants: the manifest tables and guard tests (Step 1) land before the
alignment/wiring steps read the keys; the aligned default and the mirror witness (Step 2)
land together with every test that pins the old 0.4 default — never left red for a later
step; the fixture `orca-matched-config.json` value and its `support_family_closure.rs`
consumer are updated in the same step as the const changes (Step 2); Step 4's regeneration
runs after both manifests are final. None beyond that.

## Context Discipline Notes

- `docs/15_config_keys_reference.md` is generated and ~1000 lines — never load it; verify
  via `--check` and targeted `rg`/`sed` (AC-6).
- Both module `src/lib.rs` files are the change surface (`traditional-support` ~430 lines,
  `tree-support` ~760 lines — ranged reads only outside the edited regions); the module
  test files (`traditional_support_tdd.rs` ~415 lines, `tree_support_tdd.rs` ~340 lines)
  are bounded full reads.
- `crates/slicer-gcode/src/serialize.rs` is read-only context beyond the
  `serialize_config_block`/`SUPPORT_CONFIG_DEFAULTS`/`ORCA_CONFIG_PADDING` questions
  (AC-5); no padding edits.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` (~800 lines) — ranged
  reads only (setup + interface-block-count assertions).
- Do not read the planner modules' sources — the `support-planner` claim is context, not
  surface.
