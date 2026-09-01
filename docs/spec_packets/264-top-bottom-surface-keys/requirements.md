# Requirements: top-bottom-surface-keys

## Packet Metadata

- Grouped task IDs: none — queue packet (wayfinder precedent: packets 234a, 253–263 carry `task_ids: []`); implementation is recorded against wayfinder ticket 17.
- Backlog source: `docs/specs/orca-feature-gap/issues/17-author-packet-p10-strength-top-bottom-shells-infill-modules.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P10).
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

Packet P10 (Strength / Top/bottom shells — owner `infill modules`) is the next uncovered
slice of the OrcaSlicer FFF feature-gap queue (`05-asset-packet-list.md` §P10 — 4 keys,
Tier A). Authoring-time grounding against canonical (delegated reads, 2026-09-01) and the
tree (measured 2026-09-01) confirms the tier table's owner and re-adjudicates the tier
per key:

- **The two density keys are wired (Tier A plumbing).** `top_surface_density` and
  `bottom_surface_density` (canonical coPercent, default 100, on `PrintRegionConfig`) feed
  the solid-surface fill spacing in canonical `Fill.cpp` `group_fills` (per-surface-type
  `params.density` assignment) → `FillLine.cpp` `FillLine::_fill_surface_single`
  (`line_spacing = flow.spacing() / density`). The port's rectilinear-infill module has
  exactly the same decision point: the top/bottom solid blocks in
  `modules/core-modules/rectilinear-infill/src/lib.rs` compute
  `solid_spacing = line_width / SOLID_DENSITY` with `SOLID_DENSITY = 1.0` (the canonical
  default's fraction). The wire replaces the constant with the key's percent/100 fraction
  at the exposed surface (top_shell_index 0) and keeps 1.0 for internal solid
  (top_shell_index ≥ 1) — canonical `group_fills` gives `stInternalSolid` a **fixed**
  `100.f`, not a key. Canonical's `density <= 0` skip (top branch only) is wired as a
  `density > 0` gate on the top block; `bottom_surface_density`'s canonical min 10 makes
  the bottom gate provably inert. At canonical defaults (100 → fraction 1.0) the emitted
  paths are byte-identical to today (AC-2); non-default values change the solid spacing
  (AC-3) — the value reaches the consumer.
- **The two pattern keys are re-adjudicated declared-with-gap.** `top_surface_pattern`
  (canonical default `ipMonotonicLine`) and `bottom_surface_pattern` (canonical default
  `ipMonotonic`) select the filler class in canonical (`group_fills` → `FillBase.cpp`
  `Fill::new_from_type`); in this port the pattern IS the module identity —
  `rectilinear-infill`, `gyroid-infill`, `lightning-infill` each implement exactly one
  pattern family, and the host selects the module per region via the `*_fill_holder` CLI
  keys (`crates/slicer-ir/src/resolved_config.rs`, default `"rectilinear-infill"`). This
  is packet 262's `internal_solid_infill_pattern` finding, unchanged for the surface
  roles. Canonical's other pattern reads are recorded, not wired: the extra-internal-solid
  fill branch (`group_fills`, `top_surface_pattern` monotonic/monotonicline → that
  pattern else rectilinear, density fixed 100 — the port has no
  `infill_only_where_needed`-style extra-solid pass), `GCode.cpp` `_needSAFC` and
  `retract` (SAFC applicability and the hilbertcurve retraction exclusion — the port has
  neither SAFC nor the hilbertcurve pattern).
- **One CONFIG_BLOCK padding correction.** `ORCA_CONFIG_PADDING` in
  `crates/slicer-gcode/src/serialize.rs` carries `("top_surface_pattern", "monotonic")`;
  the canonical default is `monotonicline` (verified in `PrintConfig.cpp` — `ipMonotonicLine`).
  The padding twin is corrected to `"monotonicline"` (ticket 14's `fuzzy_skin` and packet
  262's `sparse_infill_pattern` padding-correction precedent). `("bottom_surface_pattern",
  "monotonic")` already matches canonical and stays.
- **No recorded behavior divergence at defaults.** Every declared default equals canonical
  (the two percents `100%`; the two enums `monotonicline`/`monotonic`). The wired keys are
  default-path identity (AC-2); the declared-with-gap keys are unread. The deviation gate
  (`render_deviations` in `xtask/src/gen_config_docs.rs`, numeric-only comparison) gains
  no rows: `100%` fails `parse::<f64>` (ticket 106's finding) and never enters the map;
  the enum defaults are strings and never enter it either.
- **The gyroid opt-in solid path is recorded, not wired.** ADR-0027 lets a user set
  `top_fill_holder`/`bottom_fill_holder` = `"gyroid-infill"`; gyroid's solid emission
  (`modules/core-modules/gyroid-infill/src/lib.rs` `emit_polys` over `top_solid_fill()` /
  `bottom_solid_fill()`) rides the module's single `self.density` read from
  `sparse_infill_density` — a pre-existing divergence (gyroid solid at sparse density).
  Wiring the P10 density keys into gyroid would change that opt-in behavior at defaults
  (sparse 0.2 → solid 1.0), which is a behavior change this packet does not make; the
  keys are declared in `rectilinear-infill.toml` only and the omission is pinned (AC-N2)
  so a future gyroid-solid-density packet must consciously update.

No user rulings were required: every wired key is default-path identity with the value
reaching the consumer, every declared-with-gap key is declared with canonical
defaults/bounds, and the padding correction aligns a cosmetic CONFIG_BLOCK line to
canonical.

## In Scope

- 4 `[config.schema]` tables in `rectilinear-infill.toml` (AC-1), each with canonical
  type/default/bounds/values, the canonical-title `display`, `group = "Infill"`, and a
  `description` field recording the disposition (wired consumer or decision-point gap).
- The density wire in `modules/core-modules/rectilinear-infill/src/lib.rs`: 2 new
  `RectilinearInfill` struct fields (`top_surface_density`, `bottom_surface_density`,
  percent/100 fractions read in `from_config` via `get_abs_value` — the
  `sparse_infill_density` read pattern), the top/bottom solid blocks' spacing switched
  from the `SOLID_DENSITY` constant to the per-region density (key fraction at
  top_shell_index 0, `SOLID_DENSITY` at ≥ 1), and the `density > 0` gate on the top block
  (canonical `density <= 0` skip; the bottom gate is provably inert under min 10).
- Manifest guard test file `modules/core-modules/rectilinear-infill/tests/
  top_bottom_surface_config_schema_tdd.rs` (net-new, distinct binary from packet 262's
  `infill_config_schema_tdd` and 263's `infill_pattern_specific_config_schema_tdd` so the
  three packets' net-new files never collide; mirrors the part-cooling guard pattern
  `cooling_config_schema_tdd.rs` and packet 263's guard form); pins the 4 tables and the
  AC-N2 gyroid/lightning omission; requires the `toml = "0.8"` dev-dependency in
  `rectilinear-infill/Cargo.toml` (add-if-absent — absent at 264 authoring; may already
  exist when 262/263's steps land, since they implement first per queue order).
- Identity/reachability/skip arms in the existing module suite
  `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` (AC-2:
  explicit-canonical-defaults vs absent → byte-identical paths; AC-3: density 50 → path
  count approximately halves; AC-N3: density 0 → zero `TopSolidInfill` paths for the
  exposed region while an internal-solid region still emits).
- Bounds/type rejection arms in the existing scheduler integration binary
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (AC-4).
- The `ORCA_CONFIG_PADDING` value correction `("top_surface_pattern", "monotonic")` →
  `("top_surface_pattern", "monotonicline")` in `crates/slicer-gcode/src/serialize.rs`
  (AC-5) — the packet's only edit to that file.
- CONFIG_BLOCK arms in the existing runtime integration binary
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (AC-5: corrected/unchanged pattern twins at defaults, zero density lines at defaults,
  explicit values appear once).
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`
  (AC-6) and a guest rebuild (`cargo xtask build-guests` — `rectilinear-infill.toml` and
  `rectilinear-infill/src/lib.rs` are guest-fingerprint inputs).

## Out of Scope

- **Pattern dispatch** — canonical's `top_surface_pattern` / `bottom_surface_pattern`
  select the filler class (`FillBase.cpp` `Fill::new_from_type`); the port's pattern is
  module identity selected by the host `*_fill_holder` resolution (packet 262's finding,
  unchanged here). A pattern→module mapping is host-side config-resolution work, not an
  infill-module decision point; the keys are declared-with-gap and the divergence (port
  solid fill = rectilinear scan-line generator vs canonical `ipMonotonicLine` /
  `ipMonotonic` filler classes) is recorded.
- **The gyroid opt-in solid path** — ADR-0027's multi-role gyroid emission rides the
  sparse density; wiring the P10 density keys there would change that opt-in behavior at
  defaults. Recorded divergence; a future gyroid-solid-density packet re-opens it
  (AC-N2 pins the omission).
- **Canonical's surface-expansion density gates** — `PrintObject.cpp`
  `detect_surfaces_type` and `PerimeterGenerator.cpp` `top_fill_replaces_inner_walls`
  gate top-surface expansion geometry on `density > 0`; the port has no top-surface
  expansion pass. Recorded in the density keys' dispositions, not wired.
- **The extra-internal-solid-fill branch** — canonical `group_fills` reads
  `top_surface_pattern` to pick monotonic vs rectilinear for an extra internal solid fill
  when internal voids exist (`infill_only_where_needed` machinery); the port has no such
  pass. Recorded in `top_surface_pattern`'s disposition.
- **Emission-time pattern reads** — `GCode.cpp` `_needSAFC` (SAFC applicability) and
  `retract` (hilbertcurve exclusion); the port has neither SAFC nor the hilbertcurve
  pattern. Recorded, not wired.
- **Module-source reads of the two pattern keys** — the packet adds zero reads for them;
  the pattern keys are declared-with-gap (AC-2's byte-identity arm covers the unread
  contract).
- **`docs/ORCA_CONFIG_REFERENCE.md` hand-maintained column** — untouched (ticket 07
  ruling; the queue never reads it).
- **Tier-table row updates** (`04-asset-tier-assignment.md`) — ride ticket 17's closure
  (ticket 12/13/14/15/16/18/19 precedent), not this packet's files.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated (~1000 lines); delegated reads only.
  Regenerated by this packet (AC-6); never hand-edited.
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker
  needs the `[config.schema]` contract (the `enum` + `values` form is grounded in-tree:
  `seam-planner-default.toml` `[config.schema.seam_position]`; the `description` field is
  parsed by `crates/slicer-scheduler/src/manifest.rs`).
- `docs/adr/0027-gyroid-multi-role-fill-holder.md` — the gyroid opt-in solid path this
  packet records, not wires (read-only context).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the 4 keys (all on `PrintRegionConfig`): `top_surface_pattern` coEnum default `ipMonotonicLine` with the 8-value `InfillPattern` list; `top_surface_density` coPercent default 100 min 0 max 100; `bottom_surface_pattern` coEnum (same 8 values) default `ipMonotonic`; `bottom_surface_density` coPercent default 100 min 10 max 100. Authoring-time evidence already captured in §Per-Key Canonical Evidence (dispatched canonical reads, 2026-09-01) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills` (per-surface-type assignment: `stTop` → `top_surface_pattern` + `top_surface_density` with the `density <= 0` skip; `stBottom` → `bottom_surface_pattern` + `bottom_surface_density`; `stInternalSolid` → `internal_solid_infill_pattern` + fixed `100.f`; the extra-internal-solid-fill branch reading `top_surface_pattern`), `Layer::make_fills` (the `0.01 * density` percent normalization).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLine.cpp` — `FillLine::_fill_surface_single` (the spacing formula this packet's wire mirrors: `line_spacing = flow.spacing() / density`).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `top_fill_replaces_inner_walls` (the `density > 0` gate this packet records, not wires).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `detect_surfaces_type` (the `density > 0` top-surface-expansion gate this packet records, not wires), `invalidate_state_by_config_options` (slice-step invalidation mapping).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_needSAFC`, `retract` (emission-time pattern reads this packet records, not wires).

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
contract AC-1 pins. Canonical declares all 4 keys on `PrintRegionConfig`
(`PrintConfig.cpp` `PrintConfigDef::PrintConfigDef`); the port declares them
scalar-global in the owner manifest per the queue's established pattern (packet
254/255/257/258/259/260/261/262/263 precedent — the per-filament/per-object model is the
Tier-D fog's question, not this packet's). Authoring-time tree facts (measured
2026-09-01): the two density keys have zero occurrences in `crates/` and `modules/`; the
two pattern keys occur only as `ORCA_CONFIG_PADDING` twins in
`crates/slicer-gcode/src/serialize.rs` (`("top_surface_pattern", "monotonic")`,
`("bottom_surface_pattern", "monotonic")`); `ORCA_CONFIG_PADDING` carries neither density
key; the deviations block of `docs/15_config_keys_reference.md` holds 26 data rows.

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `top_surface_density` | coPercent | `100` | min 0, max 100 | float, default 100.0, min 0.0, max 100.0 (canonical-percent convention, ticket 107: modules divide by 100 when consuming) | `Fill.cpp` `group_fills` (stTop → `params.density = top_surface_density`, skip when `density <= 0`); `FillLine.cpp` `FillLine::_fill_surface_single` (`line_spacing = flow.spacing() / density`); `PrintObject.cpp` `detect_surfaces_type` + `PerimeterGenerator.cpp` `top_fill_replaces_inner_walls` (density > 0 surface-expansion gates — no in-port pass, recorded) | **Wired** — rectilinear top block, exposed surface (top_shell_index 0) spacing `line_width / (percent/100)`, `density > 0` gate; internal solid (index ≥ 1) keeps `SOLID_DENSITY` 1.0 (canonical fixed `100.f`) |
| `bottom_surface_density` | coPercent | `100` | min 10, max 100 | float, default 100.0, min 10.0, max 100.0 | `Fill.cpp` `group_fills` (stBottom → `params.density = bottom_surface_density`; no ≤0 skip — min 10 makes it unreachable); `FillLine.cpp` `FillLine::_fill_surface_single` (same spacing formula) | **Wired** — rectilinear bottom block, exposed surface (bottom_shell_index 0) spacing `line_width / (percent/100)`; internal solid keeps 1.0; the `density > 0` gate is provably inert under min 10 |
| `top_surface_pattern` | coEnum (8 `InfillPattern` values) | `monotonicline` (`ipMonotonicLine`) | — | enum, 8 canonical values, default `"monotonicline"` | `Fill.cpp` `group_fills` (stTop → `params.pattern = top_surface_pattern` → `FillBase.cpp` `Fill::new_from_type` filler selection; extra-internal-solid-fill branch: monotonic/monotonicline → that pattern else rectilinear, density fixed 100 — no in-port pass, recorded); `GCode.cpp` `_needSAFC` / `retract` (emission-time reads — no in-port analogue, recorded) | **Declared-with-gap** — filler selection is module identity (packet 262's finding); padding twin corrected `"monotonic"` → `"monotonicline"` |
| `bottom_surface_pattern` | coEnum (8 `InfillPattern` values) | `monotonic` (`ipMonotonic`) | — | enum, 8 canonical values, default `"monotonic"` | `Fill.cpp` `group_fills` (stBottom → `params.pattern = bottom_surface_pattern` → `Fill::new_from_type` filler selection); `GCode.cpp` `_needSAFC` / `retract` (emission-time reads — no in-port analogue, recorded) | **Declared-with-gap** — filler selection is module identity; padding twin `"monotonic"` matches canonical and stays |

### Declaration notes (port-specific decisions the canonical reads forced)

- **The 4 tables land only in `rectilinear-infill.toml`, and the two pattern keys are
  never read in any module source.** The density wire's decision point lives in
  `rectilinear-infill` (the default `top_fill_holder`/`bottom_fill_holder`); gyroid's
  opt-in solid path (ADR-0027) rides the sparse density — a pre-existing divergence this
  packet records, not fixes — so the keys are not declared there (AC-N2 pins the
  omission). Lightning holds only `claim:sparse-fill` and has no solid-fill surface
  (AC-N2). A future gyroid-solid-density packet consumes the declarations and must update
  the guard's omission pins.
- **The density wire is exposed-surface-only.** Canonical `group_fills` gives
  `stInternalSolid` a fixed `100.f` (verified verbatim: `params.density = 100.f` in the
  `surface.is_solid_infill()` branch), so the port's deeper shell layers
  (top_shell_index/bottom_shell_index ≥ 1) keep `SOLID_DENSITY` 1.0 and only the exposed
  surface (index 0) reads the key. This mirrors the existing width split
  (`resolve_role_width` picks `top_surface_line_width` vs `internal_solid_infill_line_width`
  by the same index).
- **The `density <= 0` skip is top-only in canonical** (the `stTop` branch's
  `if (params.density <= 0.0f) continue;`; `bottom_surface_density` min 10 makes it
  unreachable for bottom). The wire gates the top block on `density > 0` (live behavior,
  AC-N3) and the bottom block identically (provably inert — AC-4's min-10 arm pins the
  bound that makes it so).
- **Percent keys use the ticket-107 convention** (canonical-percent numbers with canonical
  bounds: `100.0`/`[0, 100]` and `[10, 100]` — the modules divide by 100 when consuming,
  the `sparse_infill_density` read pattern in `from_config`).
- **Deviation gate: zero new rows, block stays at 26.** The two enum defaults
  (`monotonicline`, `monotonic`) are strings and never enter the numeric comparison map;
  the two percent defaults (`100%`) fail `parse::<f64>` (ticket 106's finding) and never
  enter it either. AC-6's probe re-measures the block at implementation time (ledger fact
  — 26 measured at 264 authoring, 2026-09-01).
- **CONFIG_BLOCK: one corrected twin, one unchanged twin, two honest absences.** The
  pattern keys' padding twins exist and emit at defaults — `top_surface_pattern` is
  corrected to the canonical `monotonicline` (ticket-14/262 precedent), `bottom_surface_pattern`
  stays `monotonic`; the density keys have no twins, so at defaults the block carries
  nothing for them (AC-5 pins all four states). An explicit value reaches the block once
  through the raw-config sorted dump (`serialize_config_block` + `emit_config_kv` dedup,
  packet-257 AC-5 form), suppressing its padding twin.
- **Tier A/B status:** AC-2/AC-3 are the wired keys' Tier A plumbing contract
  (default-matches + reaches-the-consumer); AC-4/AC-5/AC-6 are the declaration/bounds/
  block/docs arms; AC-N1/N2 are the guard arms; AC-N3 is the skip's negative case. The 04
  tier rows stand (`Tier A`, owner `infill modules`) — the re-adjudication changes
  dispositions, not the tier/owner columns.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest exactness — 4 tables, one manifest), `AC-2` (default-path
  identity), `AC-3` (wire reachability — density 50 halves the solid path count), `AC-4`
  (bounds/type rejection), `AC-5` (CONFIG_BLOCK: corrected/unchanged pattern twins at
  defaults, zero density lines, explicit values appear once), `AC-6` (generated docs: 4
  keys present, deviation block unchanged at 26).
- Negative: `AC-N1` (schema guard fails naming the drifted key), `AC-N2` (gyroid +
  lightning omission of all 4 keys pinned), `AC-N3` (top_surface_density = 0 → zero
  `TopSolidInfill` paths for the exposed region; internal solid still emits).
- Cross-packet impact: packets 262 (P08) and 263 (P09) touch the same
  `rectilinear-infill.toml` and add the `toml` dev-dep to the same Cargo.toml — merge
  churn only (all append; 262/263 implement first per queue order; P10's guard binary is
  distinct so no file collision; the dev-dep is add-if-absent). P10's `src/lib.rs` wire
  touches the same module 262's angle wire touches — different decision points (solid
  spacing vs solid angle), no overlap. Post-packet doc-15 state: 4 new module-key rows
  (owner `rectilinear-infill`); deviation block unchanged (26 rows).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p rectilinear-infill --test top_bottom_surface_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1/N1/N2 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p rectilinear-infill --test top_bottom_fill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2 identity / AC-3 reachability / AC-N3 skip arms | FACT pass/fail |
| `rg -n 'top_surface_pattern\|bottom_surface_pattern' modules/core-modules/rectilinear-infill/src modules/core-modules/gyroid-infill/src modules/core-modules/lightning-infill/src modules/core-modules/infill-linker/src; [ "$?" = "1" ]; echo "exit=$?"` | pattern keys' no-reads pin (expect exit 0 = no matches) | FACT exit code |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-4 bounds/type rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-5 CONFIG_BLOCK emission | FACT pass/fail |
| `cargo xtask gen-config-docs --check && for k in top_surface_density bottom_surface_density top_surface_pattern bottom_surface_pattern; do rg -q "$k" docs/15_config_keys_reference.md \|\| exit 9; done && [ "$(sed -n '/BEGIN GENERATED: orca-deviations/,/END GENERATED: orca-deviations/p' docs/15_config_keys_reference.md \| grep -c "^| \`")" = "26" ]; echo "exit=$?"` | AC-6 generated docs + deviation-block count | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest + module src edit) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

Only cross-step invariants: the manifest tables and guard test (Step 1) land before the
module wire and its arms (Step 2) and the bounds/CONFIG_BLOCK arms that consume them
(Steps 3-4); the padding correction (Step 4) lands with the CONFIG_BLOCK arm that pins
it; the guest rebuild (Step 4) runs before the integration arm that dispatches the real
rectilinear guest; Step 5's regeneration runs after the manifest is final. The module
unit suites (Steps 1-2) run native and do not need the guest. None beyond that.

## Context Discipline Notes

- `docs/15_config_keys_reference.md` is generated and ~1000 lines — never load it; verify
  via `--check` and targeted `rg`/`sed` (AC-6).
- `rectilinear-infill.toml` is a bounded full read (~210 lines); the guard pattern source
  `cooling_config_schema_tdd.rs` is a bounded full read; `top_bottom_fill_tdd.rs` (~390
  lines) and `config_bounds_enforcement_tdd.rs` (~460 lines) are bounded full reads;
  `gcode_header_thumbnail_config_blocks_tdd.rs` (~1040 lines) is ranged reads.
- `rectilinear-infill/src/lib.rs` is in scope for the wire (Step 2) — the top/bottom
  solid blocks and `from_config`; the rest of the file is read-only context.
- `modules/core-modules/{gyroid-infill,lightning-infill,infill-linker}/src` are read-free
  pins for the 4 keys — never open them for reads (the no-reads grep is the evidence).
- `crates/slicer-gcode/src/serialize.rs` is read-only except the single padding value
  correction (Step 4) — the padding mechanism is proven by the authoring grep, not by
  re-reading the table.
- Do not read the perimeters modules' sources — out of scope (packet 262's gap-fill
  context, not surface).
