# Requirements: infill-pattern-keys

## Packet Metadata

- Grouped task IDs: none — queue packet (wayfinder precedent: packets 234a, 253–261 carry `task_ids: []`); implementation is recorded against wayfinder ticket 15.
- Backlog source: `docs/specs/orca-feature-gap/issues/15-author-packet-p08-strength-infill-infill-modules.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P08).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet P08 (Strength / Infill — owner `infill modules`) is the next uncovered slice of
the OrcaSlicer FFF feature-gap queue (`05-asset-packet-list.md` §P08 — 7 keys, Tier A).
Authoring-time re-derivation against the tree confirms the tier table's owner and
re-adjudicates the tier per key:

- **Four keys have live decision points and are wired (default-path identity).**
  `solid_infill_direction` (canonical coFloat, default 45, min 0, max 360) maps to the
  solid-role angle read in `RectilinearInfill::from_config` /
  `GyroidInfill::from_config` (`modules/core-modules/rectilinear-infill/src/lib.rs`,
  `modules/core-modules/gyroid-infill/src/lib.rs` — both read `infill_direction` once and
  use it for every role); `sparse_infill_rotate_template` / `solid_infill_rotate_template`
  (canonical coString, default "") map to the per-layer angle computation (the modules
  currently use a constant base angle — `run_infill`'s `angle_deg = self.base_angle` in
  rectilinear, `fill_expolygon`'s `(base_angle + CORRECTION_ANGLE_DEG).to_radians()` in
  gyroid); `fill_multiline` (canonical coInt, default 1, min 1, max 10) maps to the
  sparse scan-line emission (`scan_expolygon` in rectilinear). At canonical defaults the
  wired behavior is byte-identical to today (45 = 45, "" = base angle, 1 = single line).
- **Three keys are re-adjudicated declared-with-gap.** `sparse_infill_pattern` (canonical
  coEnum, 26 values, default `crosshatch`) and `internal_solid_infill_pattern` (canonical
  coEnum, 8 top-fill values, default `monotonic`) select the filler class in canonical
  (`Fill.cpp::group_fills` → `FillBase.cpp::Fill::new_from_type`); in this port the
  pattern IS the module identity — `rectilinear-infill`, `gyroid-infill`,
  `lightning-infill` each implement exactly one pattern family, and the host selects the
  module per region via the `*_fill_holder` CLI keys (`crates/slicer-ir/src/resolved_config.rs`
  `cli "sparse_fill_holder"` etc., default `"rectilinear-infill"`). The port implements 3
  of the 26 canonical patterns; a pattern→module mapping is host-side config-resolution
  work, not an infill-module decision point. `gap_fill_target` (canonical coEnum,
  everywhere/topbottom/nowhere, default `nowhere`) gates canonical's **fill-side** gap
  fill (`FillBase.cpp::Fill::_create_gap_fill`); this port has no fill-side gap fill — its
  gap fill is the **perimeter-side** mechanism (`classic-perimeters` /
  `arachne-perimeters` medial-axis emission, canonical `PerimeterGenerator.cpp`
  `process_classic`, gated by `filter_out_gap_fill`), which canonical's `gap_fill_target`
  does not gate either.
- **Two recorded behavior divergences at defaults (no deviation rows).** The port's
  default sparse pattern is rectilinear (the `sparse_fill_holder` default) where
  canonical's `sparse_infill_pattern` default is `crosshatch`; the port's solid fill is
  the rectilinear scan-line generator where canonical's `internal_solid_infill_pattern`
  default is `monotonic` (a distinct filler class). Both are port-state records, not
  default-value deviations — the declared defaults match canonical, so the deviation gate
  (`render_deviations` in `xtask/src/gen_config_docs.rs`, numeric-only comparison) gains
  no rows.
- **One CONFIG_BLOCK padding correction.** `ORCA_CONFIG_PADDING` in
  `crates/slicer-gcode/src/serialize.rs` carries `("sparse_infill_pattern", "grid")`; the
  canonical default is `crosshatch` (verified in `PrintConfig.cpp`). The padding twin is
  corrected to `"crosshatch"` (ticket 14's `fuzzy_skin` padding-correction precedent).
  `("gap_fill_target", "nowhere")` already matches canonical and stays.

No user rulings were required: every wired key is default-path identity, every
declared-with-gap key is declared with canonical defaults/bounds, and the padding
correction aligns a cosmetic CONFIG_BLOCK line to canonical.

## In Scope

- 17 `[config.schema]` tables across three manifests: `rectilinear-infill.toml` (7),
  `gyroid-infill.toml` (7), `lightning-infill.toml` (3 — the sparse keys only), each with
  canonical type/default/bounds/values and a `description` field recording the
  disposition (AC-1/AC-N1/AC-N2).
- Manifest guard test file `modules/core-modules/rectilinear-infill/tests/
  infill_config_schema_tdd.rs` (net-new, mirroring the part-cooling guard pattern
  `cooling_config_schema_tdd.rs` and packet 260's `support_config_schema_tdd.rs`); pins
  all three manifests and the AC-N2 lightning omission; requires the
  `toml = "0.8"` dev-dependency add-if-absent in `rectilinear-infill/Cargo.toml`
  (verified absent at authoring).
- Rectilinear wiring in `modules/core-modules/rectilinear-infill/src/lib.rs`:
  `from_config` reads for the four wired keys; `run_infill` computes a per-role per-layer
  angle (sparse: `infill_direction` + `sparse_infill_rotate_template`; solid:
  `solid_infill_direction` + `solid_infill_rotate_template`) and applies `fill_multiline`
  to the sparse scan (base spacing × N, N copies at perpendicular offsets of the sparse
  line width, clipped to the region polygon). A module-local `template_angle` helper
  (comma-separated list cycled by layer index; empty → base angle; metalanguage strings
  → logged warn + base angle, recorded gap) and a module-local translate helper (gyroid's
  module-local `rotate_expolygon` is the precedent). Bridge roles keep their existing
  angle source (`region.bridge_orientation_deg()`).
- Gyroid wiring in `modules/core-modules/gyroid-infill/src/lib.rs`: `from_config` reads
  for the three angle keys; `fill_expolygon` takes the per-role angle (sparse vs solid).
  `fill_multiline` is declared-with-gap in gyroid (curve offsetting is Tier B+).
- Non-perturbation + behavior arms in the existing module suites
  (`rectilinear_raw_emit_tdd.rs`, `gyroid_infill_tdd.rs` — AC-2/3/4/5).
- One-value padding correction in `crates/slicer-gcode/src/serialize.rs`
  (`("sparse_infill_pattern", "grid")` → `"crosshatch"`) and a CONFIG_BLOCK reachability
  arm in the existing runtime integration binary
  `gcode_header_thumbnail_config_blocks_tdd.rs` (AC-7).
- Bounds/enum rejection arms in the existing scheduler integration binary
  `config_bounds_enforcement_tdd.rs` (AC-6).
- Regeneration of `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`
  (AC-8) and a guest rebuild (`cargo xtask build-guests` — the three infill manifests and
  module sources are guest-fingerprint inputs).

## Out of Scope

- **Pattern dispatch** — canonical's `sparse_infill_pattern` / `internal_solid_infill_pattern`
  select the filler class (`Fill::new_from_type`); the port's pattern is module identity
  selected by the host `*_fill_holder` resolution. A pattern→module mapping (e.g.
  `sparse_infill_pattern = "gyroid"` → `sparse_fill_holder = "gyroid-infill"`) is
  host-side config-resolution work, not an infill-module decision point; the keys are
  declared-with-gap and the divergence (port default rectilinear vs canonical crosshatch;
  port solid rectilinear vs canonical monotonic) is recorded. The 23 unimplemented
  canonical patterns are out of scope for this packet.
- **Fill-side gap fill** — canonical's `gap_fill_target` gates `Fill::_create_gap_fill`
  (gap fill on solid surfaces in the fill step); the port has no fill-side gap fill. The
  port's perimeter-side gap fill (`classic-perimeters` / `arachne-perimeters` medial
  axis, `filter_out_gap_fill` gate) is a different mechanism that canonical's
  `gap_fill_target` does not gate; it is read-only context here.
- **Template metalanguage** — canonical's `calculate_infill_rotation_angle` metalanguage
  (joints `/NnZz$LlUuQq~^|#`, repeats `*N`, units `% # ' " c m mm`, `B`/`T` shell counts,
  `!` one-time, `-` negative) is declared-with-gap; only the comma-separated list form is
  wired. A metalanguage string falls back to the base angle with a logged warn (recorded
  degradation; default "" unaffected).
- **Gyroid/lightning multiline** — canonical `FillGyroid` / `FillLightning` call
  `multiline_fill`, but offsetting curved gyroid paths (and lightning's tree segments)
  is Tier B+ geometry; `fill_multiline` is declared-with-gap in both modules.
- **`top_surface_pattern` / `bottom_surface_pattern`** — canonical's top/bottom surface
  pattern keys (P10's surface, tickets 16/17) are read-only context; this packet's
  `internal_solid_infill_pattern` covers only the internal-solid role.
- **`infill_direction`** — already renamed and wired (ticket 105); read-only context as
  the sparse base angle.
- **`SUPPORT_CONFIG_DEFAULTS` / `ORCA_CONFIG_PADDING` beyond the one correction** —
  `("sparse_infill_pattern", "grid")` → `"crosshatch"` is the only padding edit;
  `("gap_fill_target", "nowhere")` matches canonical and stays; no twins are added for
  the other five keys (254/255/257/258/259/260/261 precedent), so at defaults they do not
  appear in the CONFIG_BLOCK (AC-7).
- **`docs/ORCA_CONFIG_REFERENCE.md` hand-maintained column** — untouched (ticket 07
  ruling; the queue never reads it).
- **Tier-table row updates** (`04-asset-tier-assignment.md`) — ride ticket 15's closure
  (ticket 12/13/14/18/19 precedent), not this packet's files.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — generated (~1000 lines); delegated reads only.
  Regenerated by this packet (AC-8); never hand-edited.
- `docs/03_wit_and_manifest.md` — manifest schema shape; delegated SUMMARY if a worker
  needs the `[config.schema]` contract (the `enum` + `values` form is grounded in-tree:
  `seam-planner-default.toml` `[config.schema.seam_position]`; the `string` form:
  `machine-gcode-emit.toml` `[config.schema.machine_start_gcode]`; the `description`
  field is parsed by `crates/slicer-scheduler/src/manifest.rs`).
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; the multiline spacing math converts
  via `mm_to_units` (delegated SUMMARY).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical declarations of the 7 keys (`fill_multiline` coInt default 1 min 1 max 10; `sparse_infill_pattern` coEnum 26 values default `crosshatch`; `sparse_infill_rotate_template` coString default ""; `internal_solid_infill_pattern` coEnum 8 top-fill values default `monotonic`; `solid_infill_direction` coFloat default 45 min 0 max 360; `solid_infill_rotate_template` coString default ""; `gap_fill_target` coEnum everywhere/topbottom/nowhere default `nowhere`). Authoring-time evidence already captured in §Per-Key Canonical Evidence (dispatched canonical reads, 2026-09-01) and not re-read unless a worker disputes it.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` (the `erInternalInfill` multiline branch and the sparse/solid angle branches), `group_fills` (pattern assignment per surface type), `calculate_infill_rotation_angle` (the template parser).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `multiline_fill`, `Fill::_create_gap_fill`, `Fill::new_from_type`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` — `fill_surface_by_multilines`.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `combine_infill`.

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
contract AC-1 pins. Canonical declares all 7 keys on `PrintObjectConfig`
(`PrintConfig.cpp` `build_print_object_config_options`); the port declares them
scalar-global in the owner manifests per the queue's established pattern (packet
254/255/257/258/259/260/261 precedent — the per-filament/per-object model is the Tier-D
fog's question, not this packet's).

| Key | Canonical type | Canonical default | Bounds | Manifest declaration | Canonical decision point (file + function) | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `fill_multiline` | coInt | `1` | min 1, max 10 | int, default 1, min 1, max 10 | `Fill.cpp` `Layer::make_fills` (`params.multiline = params.extrusion_role == erInternalInfill ? int(region_config.fill_multiline) : 1` — sparse only); `FillBase.cpp` `multiline_fill` (offset copies at multiples of spacing; odd N: center + rings at i·spacing, even N: 0.5·spacing + i·spacing); `FillRectilinear.cpp` `fill_surface_by_multilines` (base `line_spacing = spacing * multiline / density`, polygon pre-expanded by `0.5 * multiline * spacing`) | **Wired (rectilinear sparse)** — the sparse scan-line emission (`scan_expolygon`) is the live decision point; default 1 is identity. **Declared-with-gap (gyroid, lightning)** — curve/tree-segment offsetting is Tier B+ |
| `sparse_infill_pattern` | coEnum (26 values) | `crosshatch` | 26 canonical values | enum, 26 values, default `crosshatch` | `Fill.cpp` `group_fills` (default pattern for non-solid surfaces); `FillBase.cpp` `Fill::new_from_type` (pattern → filler class); `PrintObject.cpp` `combine_infill` (density 100% branch) | **Declared-with-gap** — pattern is module identity in this port (3 of 26 patterns implemented); the decision point is the host `*_fill_holder` resolution, not an infill-module read. Divergence recorded: port default sparse = rectilinear (holder default) vs canonical `crosshatch` |
| `sparse_infill_rotate_template` | coString | `""` | — | string, default "" | `Fill.cpp` `Layer::make_fills` (`params.angle = calculate_infill_rotation_angle(..., infill_direction, sparse_infill_rotate_template)`; `params.fixed_angle = !template.empty()`); `Fill.cpp` `calculate_infill_rotation_angle` (empty → base angle; else comma-separated list cycled by layer id, or the metalanguage) | **Wired (rectilinear, gyroid)** — the per-layer sparse angle computation; default "" is identity. Metalanguage declared-with-gap (falls back to base angle with a logged warn). **Declared-with-gap (lightning)** — lightning's tree geometry is angle-independent (inert even in canonical) |
| `internal_solid_infill_pattern` | coEnum (8 top-fill values) | `monotonic` | 8 canonical values | enum, 8 values, default `monotonic` | `Fill.cpp` `group_fills` (`surface.is_solid_infill()` → this pattern, density forced 100); `FillBase.cpp` `Fill::new_from_type`; `PrintObject.cpp` `combine_infill` (sparse density 100% branch) | **Declared-with-gap** — same module-identity finding as `sparse_infill_pattern`. Divergence recorded: port solid fill = rectilinear scan-line generator vs canonical `monotonic` filler class |
| `solid_infill_direction` | coFloat | `45` | min 0, max 360 | float, default 45.0, min 0.0, max 360.0 | `Fill.cpp` `Layer::make_fills` (solid-role base angle into `calculate_infill_rotation_angle`; overridden by `top_layer_direction`/`bottom_layer_direction` when set) | **Wired (rectilinear, gyroid)** — the solid-role angle read; default 45 = `infill_direction` 45, identity |
| `solid_infill_rotate_template` | coString | `""` | — | string, default "" | `Fill.cpp` `Layer::make_fills` (solid-role template into `calculate_infill_rotation_angle`; `fixed_angle = !template.empty()`); `calculate_infill_rotation_angle` | **Wired (rectilinear, gyroid)** — the per-layer solid angle computation; default "" is identity. Metalanguage declared-with-gap |
| `gap_fill_target` | coEnum | `nowhere` | everywhere/topbottom/nowhere | enum, 3 values, default `nowhere` | `FillBase.cpp` `Fill::_create_gap_fill` (`nowhere` → no gap fill; `topbottom` → `stInternalSolid` surfaces excluded; `everywhere` → all solid surfaces) | **Declared-with-gap** — gates canonical's fill-side gap fill; the port has no fill-side gap fill (its gap fill is the perimeter-side `process_classic` mechanism, already ported in classic-perimeters/arachne-perimeters, gated by `filter_out_gap_fill` — a mechanism canonical's `gap_fill_target` does not gate) |

### Wiring notes (port-specific decisions the canonical reads forced)

- **Wired keys are default-path identity.** At canonical defaults (`solid_infill_direction`
  45 = `infill_direction` 45; both templates "" = base angle; `fill_multiline` 1 = single
  line) the emitted `InfillIR` is byte-identical to pre-packet behavior — AC-2 pins this
  for both wired modules. The `from_config` fallbacks for the new keys are the canonical
  defaults (45.0 / "" / 1), so absent-key behavior matches declared-default behavior.
- **The template parser is module-local and duplicated.** The rectilinear and gyroid
  modules are separate WASM guests; a shared helper in `slicer-sdk` would ripple into all
  44 guests' fingerprints. The modules already duplicate small helpers (`solid_fill_role`
  exists in both) — the ~15-line `template_angle` parser is duplicated per module
  (comma-separated list cycled by `layer_index % len`; empty → base angle; a string
  containing metalanguage characters (`+\-%*@'"cm/NnZz$LlUuQq~^|#`) logs a warn and falls
  back to the base angle — recorded degradation, default "" unaffected).
- **`fill_multiline` mirrors canonical's algorithm.** Sparse base scan spacing × N
  (canonical `line_spacing = spacing * multiline / density`), then N copies per scan
  line at perpendicular offsets of the sparse line width (odd N: 0, ±spacing, ±2·spacing,
  …; even N: ±0.5·spacing, ±1.5·spacing, …), each clipped to the region polygon. The
  port's translate-scan-translate approach (translate the expolygon by −t·normal, scan,
  translate paths back) is behaviorally equivalent to canonical's pre-expanded polygon +
  `multiline_fill` for the emitted lines. Canonical's `multiline_fill` uses Clipper2
  `ClipperOffset` (Round joins) — irrelevant for straight scan lines, which translate
  exactly.
- **Declared-with-gap keys are unread.** None of `sparse_infill_pattern`,
  `internal_solid_infill_pattern`, `gap_fill_target` (nor `fill_multiline` in
  gyroid/lightning, nor the template keys in lightning) is read in any module source —
  declaring them must not perturb behavior (AC-2 pins byte-identity for explicit values
  vs absent). This is the packet 259/260/261 pattern; the canonical consumers are
  recorded above so a future pattern-dispatch or fill-side-gap-fill packet can consume
  the declarations.
- **Owner confirmed, with a recorded nuance.** The tier table's owner `infill modules`
  is right for the canonical side (`Fill.cpp::group_fills` / `Layer::make_fills` are the
  fill-step decision points). The pattern keys' port-side decision point (holder
  resolution) is host-side — recorded, not corrected; the 04 owner column stays
  `infill modules` unchanged.
- **Enum value lists are canonical-exact.** `sparse_infill_pattern` carries the full
  26-value InfillPattern list (canonical order); `internal_solid_infill_pattern` carries
  the 8-value top-fill list (`def_top_fill_pattern`'s `enum_values` — canonical assigns
  `def->enum_values = def_top_fill_pattern->enum_values`); `gap_fill_target` carries
  everywhere/topbottom/nowhere. The manifest enum validation rejects values outside the
  list (AC-6's `"bogus"` arm).
- **CONFIG_BLOCK.** `ORCA_CONFIG_PADDING` carries `("sparse_infill_pattern", "grid")` —
  corrected to `"crosshatch"` (ticket 14's `fuzzy_skin` padding-correction precedent:
  the padding value contradicted the canonical default) — and `("gap_fill_target",
  "nowhere")` — matches canonical, stays. The other five keys have no padding twins and
  module-manifest defaults do not thread into raw config (254/255/257/258/259/260/261
  precedent), so at defaults the block carries exactly the two padding lines (AC-7);
  explicit values reach it once through the raw_config sorted dump
  (`serialize_config_block` + `emit_config_kv` dedup).
- **Tier A/B status:** AC-1/6/7/8 are Tier A plumbing (declare + default-matches +
  reaches-consumer); AC-2/3/4/5 are the wired-behavior pins (default-path identity +
  invariant behavior); AC-N1/N2 are the guard arms.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (manifest exactness across 3 manifests), `AC-2` (default-path identity
  for both wired modules), `AC-3` (solid-role angle wiring), `AC-4` (template cycling),
  `AC-5` (multiline count), `AC-6` (bounds/type/enum rejection), `AC-7` (CONFIG_BLOCK
  states incl. the padding correction), `AC-8` (generated docs: 7 keys present, deviation
  block unchanged at 26).
- Negative: `AC-N1` (schema guard fails naming the drifted key), `AC-N2` (lightning
  omission of the 4 solid keys pinned).
- Cross-packet impact: packets 254/255/257/258/259/260/261 precedent governs the
  no-padding-twins rule; P09/P10 (tickets 16/17) touch the same infill modules — same
  owner, different keys, no dependency. Post-packet doc-15 state: 17 new module-key rows
  (rectilinear 7, gyroid 7, lightning 3); deviation block unchanged (26 rows).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p rectilinear-infill --test infill_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-1/N1/N2 manifest guard | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p rectilinear-infill --test rectilinear_raw_emit_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2/3/4/5 rectilinear arms | FACT pass/fail |
| `cargo test -p gyroid-infill --test gyroid_infill_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-2/3/4 gyroid arms | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-6 bounds/type/enum rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | AC-7 CONFIG_BLOCK emission | FACT pass/fail |
| `cargo xtask gen-config-docs --check` | AC-8 generated docs | FACT exit code |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness (manifest + module-source edits) | FACT exit code |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

Only cross-step invariants: the manifest tables and guard test (Steps 1-2) land before
the wiring reads the keys (Steps 3-4); the guest rebuild (Step 5) runs before the
integration arms that dispatch real guests (Steps 5-6); Step 7's regeneration runs after
the manifests are final. None beyond that.

## Context Discipline Notes

- `docs/15_config_keys_reference.md` is generated and ~1000 lines — never load it; verify
  via `--check` and targeted `rg`/`sed` (AC-8).
- `rectilinear-infill/src/lib.rs` (~800 lines) and `gyroid-infill/src/lib.rs` (~700 lines)
  are bounded full reads or ranged reads (the `from_config` + `run_infill` /
  `fill_expolygon` regions); the module test files (`rectilinear_raw_emit_tdd.rs`,
  `gyroid_infill_tdd.rs`) are bounded full reads.
- `crates/slicer-gcode/src/serialize.rs` is read-only context beyond the
  `ORCA_CONFIG_PADDING` one-value correction (AC-7); no other padding edits.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (~460
  lines) and `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (~1040 lines) — bounded full read for the former, ranged reads for the latter.
- Do not read the perimeters modules' sources — the perimeter-side gap fill is context,
  not surface.
