# Requirements: 256-wipe-tower-bed-exclude-area

## Packet Metadata

- Grouped task IDs: none — the feature-gap queue's established pattern is `task_ids: []` (packets 234a, 253, 254, 255 precedent); `docs/07_implementation_status.md` holds no TASK row for this queue.
- Backlog source: `docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P04).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet P04 — Printer / Machine / Print volume — wipe-tower is one key, `bed_exclude_area`, that OrcaSlicer reads in `Print.cpp` (validation), `GCode.cpp` (filament-change cutter area), `GCodeProcessor.cpp` (viewer), and `TimelapsePosPicker.cpp` (camera-safe area), and Pinch 'n Print implements nowhere: the key has **zero occurrences in `crates/`, `modules/`, `xtask/`, or `resources/`** (authoring-time grep; it exists only in `docs/ORCA_CONFIG_REFERENCE.md` and the wayfinder gap docs, which mark it `❌ no`).

Canonically:

- `PrintConfig.cpp::PrintConfigDef` defines `bed_exclude_area` as `coPoints`, mode `comAdvanced`, no min/max, default `ConfigOptionPoints{ Vec2d(0, 0) }` — a degenerate single point that excludes nothing.
- `PrintConfig.cpp::get_bed_excluded_area` builds **one polygon** from *all* configured points (counter-clockwise, no rectangle pairing), so any point count below 3 real vertices geometrically excludes nothing.
- `Print.cpp::Print::validate` routes to `layered_print_cleareance_valid` (or `sequential_print_clearance_valid` for by-object sequencing), which intersects **each model volume's 2D convex hull** with the exclude polygon and fails **fatally** with `"<object name> is too close to exclusion area, there may be collisions when printing."`. The wipe tower itself is never tested against `bed_exclude_area` in canonical.

This port's only live bed-validation decision point is the wipe-tower module's own: `run_finalization` validates the 4 corners of the tower rectangle against the `printable_area` bed polygon and fatally rejects a corner outside (added with the tower's bed-bounds work; its test file carries the ticket-100 regression pinning Orca point-string ingestion of `printable_area`). The tier table (ticket 04) places `bed_exclude_area` at exactly this decision point — "wipe-tower (bed_shape)" — which is what makes the packet Tier A plumbing: the owner and decision point both exist; only the declaration and the check are missing.

The coherent slice: declare the key in the owning manifest, parse it through the module's existing polygon reader (which already handles both interleaved floats and Orca 3MF point strings — ticket 100's `bed_shape`→`printable_area` value-format adaptation), and extend the corner validation with an exclusion test whose failure mode mirrors canonical's (fatal, collision-risk message).

## In Scope

1. **Manifest declaration:** `[config.schema.bed_exclude_area]` in `modules/core-modules/wipe-tower/wipe-tower.toml` — `type = "float-list"`, **no `default`** (canonical's default is a degenerate point that excludes nothing; the module's absent-key fallback is the same semantics, and an absent default renders as `—` in doc-15 exactly like the sibling key `printable_area`), no min/max, `display = "Excluded bed area"`, `group = "Printer"`, `advanced = true` (Orca `comAdvanced`; parser field exists, precedent `arachne-perimeters.toml`).
2. **Module read:** `WipeTower::from_config` parses `bed_exclude_area` via the existing `float_list_from_config` reader (accepts `ConfigValue::List` of floats/ints/**Orca point strings** via `slicer_ir::parse_orca_point_string`), storing the polygon on `WipeTower` as `Vec<(f32, f32)>`, default empty.
3. **The wiring — one decision point:** `run_finalization`'s corner loop gains a second check: after the existing bed-polygon pass, any tower corner **inside** the `bed_exclude_area` polygon (same `point_in_polygon` even-odd test, on-edge counts as inside) returns `ModuleError::fatal` naming the key and the corner, with message text carrying canonical's collision-risk semantics. A degenerate polygon (< 3 vertices, i.e. including Orca's default single point and any too-short value) skips the check entirely — no exclusion. The port's decision point validates the *tower rectangle*; canonical validates *object hulls*; the tower-rectangle check is the superset-conservative local translation available at this seam (see Out of Scope).
4. **Recorded reduced-semantics gap:** canonical's object-hull↔exclude intersection and secondary consumers (`GCode.cpp::get_path_of_change_filament` 4-point cutter form, `GCodeProcessor.cpp::apply_config` viewer copy, `TimelapsePosPicker.cpp::construct_printable_area_by_printer` subtractive use) are decision-point gaps recorded per-key below — not silently dropped, not built here.
5. **Emission-surface reachability for user-set values:** a user/Orca-supplied `bed_exclude_area` rides the existing extensions bucket into the G-code CONFIG_BLOCK (generic transport; no host change). No schema default exists to thread, so no CONFIG_BLOCK line is added at defaults — output stays byte-identical.

## Out of Scope

- **Building the object-hull decision point** — intersecting per-object volume convex hulls with the exclude polygon is Print::validate-level work this tree's orchestration stage does not have; Tier B/C future work (04's rubric). Recorded as the key's gap.
- **The GCode.cpp 4-point cutter-area interpretation, the viewer copy, and the Timelapse subtractive use** — secondary consumers of the same key; they need their own decision points and are gap-recorded, not imitated (canonical itself disagrees between them: `get_bed_excluded_area` reads one polygon, `Model.cpp` groups 4-point rectangles, `get_path_of_change_filament` requires exactly 4 points — this packet follows the validation consumer).
- **`printable_height` / `extruder_printable_area` / `extruder_printable_height` / `extruder_clearance_*`** — siblings from the same Print-volume section, queued in later packets (P18/P19 rows of `05-asset-packet-list.md`); not keys of this packet.
- **Host-crate logic changes** — the module-config plumbing is fully generic (exact-name delivery through `bind_module_config_view` → `ConfigView::from_declared`; loader coercion is key-agnostic), so no `crates/**` production file changes.
- **Baseline byte-identicality for CONFIG_BLOCK at defaults** — preserved trivially: no default is added, so nothing new is emitted at defaults.

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md` — the packet ticket; direct read.
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` — P04 row at `### P04 — Printer / Machine / Print volume — wipe-tower`; ranged read ~7 lines.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — the `bed_exclude_area` Tier A row; ranged read ~15 lines. Over 300 lines total: delegate beyond these rows.
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — ~80 lines; direct read.
- `docs/15_config_keys_reference.md` — large; regeneration + grep verification only, never read in full.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef` (the `bed_exclude_area` definition facts) and `get_bed_excluded_area` (polygon interpretation).
- `src/libslic3r/Print.cpp` — `Print::validate`, `layered_print_cleareance_valid`, `sequential_print_clearance_valid` (the fatal validation semantics this packet's error mirrors).
- `src/libslic3r/GCode.cpp` — `get_path_of_change_filament` (4-point cutter-area consumer; gap evidence only).
- `src/libslic3r/GCode/GCodeProcessor.cpp` — `apply_config`; `src/libslic3r/GCode/TimelapsePosPicker.cpp` — `construct_printable_area_by_printer` (gap evidence only).

## Verified Grounding

All claims below were verified against the tree and the canonical checkout at authoring time (2026-08-30):

- **Manifest state:** `modules/core-modules/wipe-tower/wipe-tower.toml` declares exactly 8 keys today (`enable_prime_tower`, `wipe_tower_x`, `wipe_tower_y`, `prime_tower_width`, `prime_volume`, `line_width`, `printable_area`, `retract_length`); packets 254/255 (P02/P03, same owner) are authored `draft` and **not landed** (`grep -c wipe_tower_extra_flow` → 0). AC-1's union assertion therefore pins only the new key's shape and re-derives the rest of the union from disk at implementation time (FAILS the step only if the *new* key is missing/mistyped).
- **The key is a true gap:** `bed_exclude_area` has zero occurrences in `crates/`, `modules/`, `xtask/`, `resources/` (grep-verified); only `docs/ORCA_CONFIG_REFERENCE.md` and `docs/specs/orca-feature-gap/issues/*` mention it.
- **Decision point:** `run_finalization` (`modules/core-modules/wipe-tower/src/lib.rs`) parses the bed via `float_list_from_config(config, "printable_area")` → `parse_printable_area`, then validates the 4 tower-rectangle corners with `point_in_polygon`, returning `ModuleError::fatal(3, "wipe-tower corner (x, y) lies outside bed polygon")` on failure. `process()` (the legacy path) carries no bed-bounds validation — the SDK authoring path is the live one; the wired check therefore lives in `run_finalization` only (see Locked Assumptions).
- **Polygon reader adapts both serialisations:** `float_list_from_config` accepts `ConfigValue::List` of `Float`/`Int`/**`String`**, where strings go through `slicer_ir::parse_orca_point_string` (`crates/slicer-ir/src/resolved_config.rs`) — the ticket-100 fix. Its doc comment states Orca 3MF plates serialise polygon keys as point-string arrays (`["0x0", "250x0", "250x210", "0x210"]`), and `bed_bounds_tdd.rs::orca_point_string_bed_is_parsed_not_silently_defaulted` pins that form end-to-end for `printable_area`. The module-side reader (not the host `ResolvedConfig` path) is what `ConfigView` consumers hit, so `bed_exclude_area` needs **no host-side change** to ingest Orca 3MF data.
- **Config delivery is generic and exact-name:** the 3MF loader (`crates/slicer-model-io/src/loader.rs::parse_project_settings_json` → `json_to_config_value` → `coerce_string_to_config_value`) is key-agnostic (the key name feeds only `0/1` bool-schema and `%`-suffix decisions); `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) pre-filters the source map to declared keys; `ConfigView::from_declared` copies values untransformed. Declaring `bed_exclude_area` in the manifest is sufficient for an Orca 3MF value to reach the module.
- **Manifest parser semantics:** `ConfigFieldEntry` (`crates/slicer-scheduler/src/manifest.rs`) marks `type` required and every other field optional; `default = table.get("default").map(...)` → `None` when absent (valid for non-percent types — the entry simply carries no schema default); `required = true` is not read by the parser (it decorates `printable_area`'s entry today; this packet includes it for readability, matching the sibling key's shape). `advanced = true` is an existing parsed field (`arachne-perimeters.toml` uses it). Bounds machinery is numeric-only, so an unbounded float-list key declares no min/max.
- **Doc-15 rendering of an absent default:** `printable_area` renders type `float-list`, default `—`, no deviation row (`xtask/src/gen_config_docs.rs::default_num_of` compares booleans/numerics only — a key with no numeric default produces no comparand, so no deviation row can appear). `bed_exclude_area` inherits the same non-row.
- **Fatal-code convention:** `ModuleError::fatal(code, …)` in this module uses codes 2 (config parse), 3 (bed bounds), 4 (insertion); the new exclusion rejection extends the code-3 site.
- **Guest-staleness baseline:** the wipe-tower manifest and `src/lib.rs` both feed the guest fingerprint (`cargo xtask build-guests --check` gates both steps); ticket 101 established guests embed config key names, so a manifest rename/addition must rebuild guests before test runs.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

### Per-key parity evidence (ticket 02 standard)

| Key | Canonical def | Canonical consumer (file + function) | Behaviour in canonical | Disposition here |
| --- | --- | --- | --- | --- |
| `bed_exclude_area` | coPoints, default `{ Vec2d(0,0) }` (degenerate), comAdvanced, no bounds | `PrintConfig.cpp::get_bed_excluded_area` (polygon build) → `Print.cpp::Print::validate` via `layered_print_cleareance_valid` / `sequential_print_clearance_valid` (object volume convex hulls) | fatal validation: `<object name> is too close to exclusion area, there may be collisions when printing.`; wipe tower never tested against it | **WIRED** at the port's bed-validation decision point: tower-rectangle corners checked against the exclusion polygon, fatal on hit (message names the key); degenerate polygon excludes nothing; **gap**: object-hull intersection + secondary consumers (GCode.cpp `get_path_of_change_filament` 4-point cutter form; GCodeProcessor.cpp `apply_config` viewer copy; TimelapsePosPicker.cpp `construct_printable_area_by_printer` subtractive use) recorded as future work |

Evidence reads performed at authoring time (delegated, results quoted above): `PrintConfig.cpp` `PrintConfigDef` block + `get_bed_excluded_area`; `Print.cpp` validation pair; the three secondary-consumer reads.

## Verification Commands

Full matrix (AC-level commands live in `packet.spec.md`):

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"`
- `cargo test -p slicer-scheduler --test wipe_tower_p04_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- `cargo xtask gen-config-docs --check 2>&1 | tail -3`
- `cargo xtask check-literals 2>&1 | tail -3`
- `cargo xtask build-guests --check; echo "exit=$?"`