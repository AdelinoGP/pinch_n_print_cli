# Requirements: infill-pattern-specific-keys

## Packet Metadata

- Grouped task IDs: none — queue packet (wayfinder precedent: packets 234a, 253–262 carry `task_ids: []`); implementation is recorded against wayfinder ticket 16.
- Backlog source: `docs/specs/orca-feature-gap/issues/16-author-packet-p09-strength-infill-pattern-specific-infill-modules.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P09).
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

Packet P09 (Strength / Infill pattern-specific — owner `infill modules`) is the next uncovered
slice of the OrcaSlicer FFF feature-gap queue (`05-asset-packet-list.md` §P09 — 10 keys, Tier A).
Authoring-time grounding against canonical (delegated reads, 2026-09-01) and the tree (measured
2026-09-01: all 10 keys have **zero occurrences** in `crates/` + `modules/`) confirms the tier
table's owner and re-adjudicates the tier per key:

- **All 10 keys are re-adjudicated declared-with-gap.** Every canonical consumer lives in a
  pattern class this port does not ship, or behind pattern gating this port's patterns never
  activate:
  - Six keys (`infill_lock_depth`, `skin_infill_density`, `skin_infill_depth`,
    `skin_infill_line_width`, `skeleton_infill_density`, `skeleton_infill_line_width`) are
    consumed only by `FillRectilinear.cpp` `FillLockedZag::fill_surface_locked_zag` — the
    locked-zag pattern, one of the 23 canonical `InfillPattern` values this port does not
    implement (the port's pattern IS module identity: rectilinear, gyroid, lightning).
  - Two keys (`lateral_lattice_angle_1`, `lateral_lattice_angle_2`) are consumed only by
    `FillRectilinear.cpp` `FillLateralLattice::fill_surface` — the lateral-lattice pattern,
    unshipped.
  - One key (`infill_overhang_angle`) is consumed only by `FillRectilinear.cpp`
    `FillLateralHoneycomb::fill_surface` — the lateral-honeycomb pattern, unshipped.
  - One key (`symmetric_infill_y_axis`) is read by core machinery but **pattern-gated**: in
    `Fill.cpp` `Layer::make_fills` the flag is assigned only when the region's sparse pattern
    is `ipZigZag`/`ipCrossZag`/`ipLockedZag` (sparse-pattern-driven, role-independent;
    verified verbatim). The port ships no zigzag-family pattern, and canonical never activates
    the flag for plain `ipRectilinear` — so no port-reachable configuration would activate the
    double mirror (`MultiPoint::symmetric_y`: `pt(0) = 2 * x_axis - pt(0)` about
    `extended_object_bounding_box().center().x()`, applied to the input expolygon in
    `Layer::make_fills` and mirrored back to each generated polyline in `fill_surface_by_lines`).
- **No recorded behavior divergence at defaults.** Every declared default equals canonical
  (the five parseable floats 1.0/60.0/−45.0/45.0/2.0; the two percents `25%`; the two widths
  `100%`; the bool `0`). The keys are unread, so absent and explicit-canonical-default runs
  are byte-identical (AC-2). The deviation gate (`render_deviations` in
  `xtask/src/gen_config_docs.rs`, numeric-only comparison) gains no rows: `25%`/`100%` fail
  `parse::<f64>` (ticket 106's finding) and never enter the map; the bool `false` matches
  canonical `0` under the ticket-100 bool comparison; the five floats match directly.
- **No CONFIG_BLOCK padding twins.** `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs`
  carries none of the 10 keys (zero occurrences in `crates/` at authoring), so the block gains
  nothing at defaults (AC-4) — the packet-254/255/257/258/259/260/261/262 no-padding-twins rule.

No user rulings were required: every key is declared with canonical defaults/bounds and its
honest disposition recorded; nothing is wired, so no behavior changes at any value.

## In Scope

- 10 `[config.schema]` tables in `rectilinear-infill.toml` (AC-1), each with canonical
  type/default/bounds, the canonical-title `display`, `group = "Infill"`, and a `description`
  field recording the disposition (canonical consumer function + the unshipped pattern class).
- Manifest guard test file `modules/core-modules/rectilinear-infill/tests/
  infill_pattern_specific_config_schema_tdd.rs` (net-new, distinct binary from packet 262's
  `infill_config_schema_tdd` so the two packets' net-new files never collide; mirrors the
  part-cooling guard pattern `cooling_config_schema_tdd.rs` and packet 262's guard form);
  pins the 10 tables and the AC-N2 gyroid/lightning omission; requires the `toml = "0.8"`
  dev-dependency in `rectilinear-infill/Cargo.toml` (add-if-absent — absent at 263 authoring;
  may already exist when 262's steps land, since 262 implements first per queue order).
- Non-perturbation arm in the existing module suite
  `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` (AC-2:
  explicit-canonical-defaults vs absent → byte-identical `InfillIR`).
- Bounds/type rejection arms in the existing scheduler integration binary
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (AC-3).
- CONFIG_BLOCK arms in the existing runtime integration binary
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (AC-4: zero lines at defaults; explicit `skin_infill_density = 30.0` appears once).
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`
  (AC-5) and a guest rebuild (`cargo xtask build-guests` — `rectilinear-infill.toml` is a
  guest-fingerprint input).

## Out of Scope

- **The three unshipped pattern classes** — locked-zag (`FillLockedZag`), lateral-lattice
  (`FillLateralLattice`), lateral-honeycomb (`FillLateralHoneycomb`), all canonical
  `FillRectilinear` subclasses. Implementing any of them is new geometry (Tier B+) and would
  be a future packet; the 10 keys are declared now so that packet can consume the declarations.
- **Pattern dispatch** — canonical's `sparse_infill_pattern` selects the filler class
  (`FillBase.cpp` `Fill::new_from_type`); the port's pattern is module identity selected by
  the host `*_fill_holder` resolution (packet 262's finding, unchanged here).
- **The symmetric-Y double mirror** — even though the decision point (the rectilinear
  scan-line generator in `modules/core-modules/rectilinear-infill/src/lib.rs`) exists,
  canonical activates the flag only for zigzag/crosszag/lockedzag patterns, none shipped;
  wiring it would implement behavior canonical never activates for this port's patterns.
  A zigzag-family packet must revisit the key.
- **Module-source reads of any of the 10 keys** — the packet adds zero reads; the src
  directories of all four infill modules are pinned read-free (AC-2's no-reads grep).
- **`docs/ORCA_CONFIG_REFERENCE.md` hand-maintained column** — untouched (ticket 07 ruling;
  the queue never reads it).
- **Tier-table row updates** (`04-asset-tier-assignment.md`) — ride ticket 16's closure
  (ticket 12/13/14/15/18/19 precedent), not this packet's files.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated (~1000 lines); delegated reads only.
  Regenerated by this packet (AC-5); never hand-edited.
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker needs
  the `[config.schema]` contract (the `bool` form is grounded in-tree:
  `wipe-tower.toml` `[config.schema.enable_prime_tower]`; the `float` + percent-convention
  form: `rectilinear-infill.toml` `sparse_infill_density`; the `description` field is parsed
  by `crates/slicer-scheduler/src/manifest.rs`).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the 10 keys (all on `PrintObjectConfig`): `infill_lock_depth` coFloat default 1 min 0 max 100; `infill_overhang_angle` coFloat default 60 min 15 max 75; `lateral_lattice_angle_1` coFloat default −45 min −75 max 75; `lateral_lattice_angle_2` coFloat default 45 min −75 max 75; `skeleton_infill_density` coPercent default 25 min 0 max 100; `skeleton_infill_line_width` coFloatOrPercent default 100% min 0; `skin_infill_density` coPercent default 25 min 0 max 100; `skin_infill_depth` coFloat default 2 min 0 max 100; `skin_infill_line_width` coFloatOrPercent default 100% min 0; `symmetric_infill_y_axis` coBool default false. Authoring-time evidence already captured in §Per-Key Canonical Evidence (dispatched canonical reads, 2026-09-01) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` / `group_fills` (per-surface param plumbing of all 10 keys; the `symmetric_infill_y_axis` activation gate).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `FillLockedZag::fill_surface_locked_zag`, `FillLateralHoneycomb::fill_surface`, `FillLateralLattice::fill_surface`, `fill_surface_by_lines` (the mirror branch).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Flow::new_from_config_width`.
- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp` — `MultiPoint::symmetric_y`.

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

Canonical evidence per ticket 02's standard (delegated canonical reads, 2026-09-01; in-tree
grounding from the authoring survey). Type/default/bounds columns record the manifest
contract AC-1 pins. Canonical declares all 10 keys on `PrintObjectConfig`
(`PrintConfig.cpp` `build_print_object_config_options`); the port declares them
scalar-global in the owner manifest per the queue's established pattern (packet
254/255/257/258/259/260/261/262 precedent — the per-filament/per-object model is the Tier-D
fog's question, not this packet's). Authoring-time tree facts (measured 2026-09-01): all 10
keys have zero occurrences in `crates/` and `modules/`; `ORCA_CONFIG_PADDING` carries none of
them; the deviations block of `docs/15_config_keys_reference.md` holds 26 data rows.

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `infill_lock_depth` | coFloat | `1` | min 0, max 100 | float, default 1.0, min 0.0, max 100.0 | `Fill.cpp` `Layer::make_fills` / `group_fills` (param plumbing); `FillRectilinear.cpp` `FillLockedZag::fill_surface_locked_zag` (overlap depth between interior and skin — feeds the `overlap_threshold` that merges skin/skeleton regions) | **Declared-with-gap** — locked-zag pattern unshipped |
| `infill_overhang_angle` | coFloat | `60` | min 15, max 75 | float, default 60.0, min 15.0, max 75.0 | `FillRectilinear.cpp` `FillLateralHoneycomb::fill_surface` (vertical period of the angled lines: `vertical_period = 3 * half_horizontal_period / tan(angle)`; 60° = pure honeycomb) | **Declared-with-gap** — lateral-honeycomb pattern unshipped |
| `lateral_lattice_angle_1` | coFloat | `-45` | min -75, max 75 | float, default -45.0, min -75.0, max 75.0 | `FillRectilinear.cpp` `FillLateralLattice::fill_surface` (first element-set angle in Z: `dx1 = tan(angle) * z` horizontal shift) | **Declared-with-gap** — lateral-lattice pattern unshipped |
| `lateral_lattice_angle_2` | coFloat | `45` | min -75, max 75 | float, default 45.0, min -75.0, max 75.0 | `FillRectilinear.cpp` `FillLateralLattice::fill_surface` (second element-set angle: `dx2 = tan(angle) * z`) | **Declared-with-gap** — lateral-lattice pattern unshipped |
| `skeleton_infill_density` | coPercent | `25` | min 0, max 100 | float, default 25.0, min 0.0, max 100.0 (canonical-percent convention, ticket 107: modules divide by 100 when consuming) | `FillRectilinear.cpp` `FillLockedZag::fill_surface_locked_zag` (skeleton region density, fed as `0.01 * value` via `append_density_param`) | **Declared-with-gap** — locked-zag pattern unshipped |
| `skeleton_infill_line_width` | coFloatOrPercent | `100%` (ratio over `nozzle_diameter`) | min 0 | float, default 0.0, min 0.0, max 2.0 (in-tree width convention: 0.0 = fall back to the base `line_width` — the house form of `sparse_infill_line_width` in `<infill modules>` manifests) | `FillRectilinear.cpp` `FillLockedZag::fill_surface_locked_zag` (`Flow::new_from_config_width` → the skeleton flow) | **Declared-with-gap** — locked-zag pattern unshipped; canonical `100%` default fails `parse::<f64>` (`%` suffix), so no deviation comparison row can form |
| `skin_infill_density` | coPercent | `25` | min 0, max 100 | float, default 25.0, min 0.0, max 100.0 | `FillRectilinear.cpp` `FillLockedZag::fill_surface_locked_zag` (skin region density, `0.01 * value` via `append_density_param`) | **Declared-with-gap** — locked-zag pattern unshipped |
| `skin_infill_depth` | coFloat | `2` | min 0, max 100 | float, default 2.0, min 0.0, max 100.0 | `FillRectilinear.cpp` `FillLockedZag::fill_surface_locked_zag` (skin region depth — the `offset_threshold` that insets the zig region from the surface) | **Declared-with-gap** — locked-zag pattern unshipped |
| `skin_infill_line_width` | coFloatOrPercent | `100%` (ratio over `nozzle_diameter`) | min 0 | float, default 0.0, min 0.0, max 2.0 (in-tree width convention) | `FillRectilinear.cpp` `FillLockedZag::fill_surface_locked_zag` (`Flow::new_from_config_width` → the skin flow) | **Declared-with-gap** — locked-zag pattern unshipped; canonical `100%` outside the numeric comparison |
| `symmetric_infill_y_axis` | coBool | `false` (`0`) | — | bool, default false | `Fill.cpp` `Layer::make_fills` (activation gate: `params.symmetric_infill_y_axis = region_config.symmetric_infill_y_axis` only when the region's sparse pattern is `ipZigZag`/`ipCrossZag`/`ipLockedZag` — sparse-pattern-driven, role-independent; never for `ipRectilinear`); `FillRectilinear.cpp` `fill_surface_by_lines` (double mirror: input expolygon mirrored about `extended_object_bounding_box().center().x()` in `Layer::make_fills`, every generated polyline mirrored back via `MultiPoint.cpp` `MultiPoint::symmetric_y`, `pt(0) = 2 * x_axis - pt(0)`) | **Declared-with-gap** — the flag never activates for any port-shipped pattern (rectilinear/gyroid/lightning); a zigzag-family packet must revisit |

### Declaration notes (port-specific decisions the canonical reads forced)

- **The 10 tables land only in `rectilinear-infill.toml`, and no module source reads them.**
  Every canonical consumer is a `FillRectilinear` subclass (locked-zag, lateral-lattice,
  lateral-honeycomb) or, for `symmetric_infill_y_axis`, a pattern-gated mirror the port's
  patterns never activate. The port's pattern-parameter convention (packet 262's
  module-identity finding) makes `rectilinear-infill` the honest owner: it is the module
  that would implement the rectilinear-family patterns. Declaring the keys anywhere else,
  or reading any of them (AC-2's no-reads grep pins all four module src dirs), would invent
  a decision point canonical does not have. A future locked-zag/lateral-* packet consumes
  the declarations and must update the guard's omission pins (AC-N2).
- **Percent keys use the ticket-107 convention** (canonical-percent numbers with canonical
  bounds: `25.0`/`[0, 100]` — the modules divide by 100 when consuming). The two width keys
  use the in-tree width-table convention (`float`, default `0.0` = fall back to the base
  `line_width`, bounds `[0.0, 2.0]` — the house form of `sparse_infill_line_width`,
  `bridge_line_width`, etc. across the infill/perimeter manifests); canonical declares them
  `coFloatOrPercent` default `100%` (ratio over `nozzle_diameter`, resolved by
  `Flow::new_from_config_width`), which cannot be represented as a numeric default in the
  port's mm-float width model and never enters the deviation comparison map.
- **`symmetric_infill_y_axis` is inert even where its consumer exists.** The wire was
  considered (mirror the region polygon about a computed axis in the rectilinear
  scan-line generator) and rejected: canonical activates the flag only for
  zigzag/crosszag/lockedzag patterns (`Layer::make_fills` gate, verified verbatim), none
  of which this port ships — wiring it would implement behavior canonical never activates
  for the port's patterns, and computing the axis from the region bbox instead of the
  object bbox would be a second divergence. Recorded in the disposition; a zigzag-family
  packet re-opens it.
- **Deviation gate: zero new rows, block stays at 26.** The five parseable float defaults
  (1.0, 60.0, −45.0, 45.0, 2.0) match canonical exactly; `25%`/`100%` fail `parse::<f64>`
  and never enter the numeric comparison map (ticket 106's finding); the bool `false`
  matches canonical `0` under the ticket-100 bool comparison. AC-5's probe re-measures the
  block at implementation time (ledger fact — 26 measured at 263 authoring, 2026-09-01).
- **CONFIG_BLOCK: no twins, honest absence.** None of the 10 keys is in `ORCA_CONFIG_PADDING`
  (zero occurrences in `crates/` at authoring). Module-manifest defaults do not thread into
  raw config (packet-254/255/257/258/259/260/261/262 precedent), so at defaults the block
  carries nothing for these keys; an explicit value reaches the block once through the
  raw-config sorted dump (`serialize_config_block` + `emit_config_kv` dedup, packet-257
  AC-5 form). AC-4 pins both states.
- **Tier A/B status:** all ACs are Tier A plumbing (declare + default-matches +
  inertness-pinned); AC-2's byte-identity is the with-gap contract's invariant pin;
  AC-N1/N2 are the guard arms. The 04 tier rows stand (`Tier A`, owner `infill modules`) —
  the re-adjudication changes dispositions, not the tier/owner columns.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest exactness — 10 tables, one manifest), `AC-2` (default-path
  identity + no module-source reads), `AC-3` (bounds/type rejection), `AC-4` (CONFIG_BLOCK:
  honest absence at defaults, explicit values appear once), `AC-5` (generated docs: 10 keys
  present, deviation block unchanged at 26).
- Negative: `AC-N1` (schema guard fails naming the drifted key), `AC-N2` (gyroid + lightning
  omission of all 10 keys pinned).
- Cross-packet impact: packet 262 (P08) touches the same `rectilinear-infill.toml` and adds
  the `toml` dev-dep to the same Cargo.toml — merge churn only (both append; 262 implements
  first per queue order; P09's guard binary is distinct so no file collision; the dev-dep is
  add-if-absent). P10 (ticket 17) touches the same infill modules — different keys, no
  dependency. Post-packet doc-15 state: 10 new module-key rows (owner `rectilinear-infill`);
  deviation block unchanged (26 rows).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p rectilinear-infill --test infill_pattern_specific_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1/N1/N2 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 inertness arm | FACT pass/fail |
| `rg -n 'infill_lock_depth\|infill_overhang_angle\|lateral_lattice_angle_1\|lateral_lattice_angle_2\|skeleton_infill_density\|skeleton_infill_line_width\|skin_infill_density\|skin_infill_depth\|skin_infill_line_width\|symmetric_infill_y_axis' modules/core-modules/rectilinear-infill/src modules/core-modules/gyroid-infill/src modules/core-modules/lightning-infill/src modules/core-modules/infill-linker/src; [ "$?" = "1" ]; echo "exit=$?"` | AC-2 no-reads pin (expect exit 0 = no matches) | FACT exit code |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-3 bounds/type rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 CONFIG_BLOCK emission | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-5 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest edit) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

Only cross-step invariants: the manifest tables and guard test (Step 1) land before the
behavior/bounds/CONFIG_BLOCK arms that consume them (Steps 2-4); the guest rebuild (Step 4)
runs before the integration arm that dispatches the real rectilinear guest; Step 5's
regeneration runs after the manifest is final. The module unit suites (Steps 1-2) run
native and do not need the guest. None beyond that.

## Context Discipline Notes

- `docs/15_config_keys_reference.md` is generated and ~1000 lines — never load it; verify
  via `--check` and targeted `rg`/`sed` (AC-5).
- `rectilinear-infill.toml` is a bounded full read (~200 lines); the guard pattern source
  `cooling_config_schema_tdd.rs` is a bounded full read; `rectilinear_raw_emit_tdd.rs` and
  `config_bounds_enforcement_tdd.rs` (~460 lines) are bounded full reads;
  `gcode_header_thumbnail_config_blocks_tdd.rs` (~1040 lines) is ranged reads.
- `modules/core-modules/{gyroid-infill,lightning-infill,infill-linker}/src` and
  `rectilinear-infill/src/lib.rs` are read-free pins for the 10 keys — never open them for
  reads (AC-2's grep is the evidence).
- `crates/slicer-gcode/src/serialize.rs` is read-only context (padding absence is proven by
  the authoring grep, not by re-reading the table).
- Do not read the perimeters modules' sources — out of scope (packet 262's gap-fill context,
  not surface).
