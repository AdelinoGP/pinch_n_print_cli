# Design: 256-wipe-tower-bed-exclude-area

## Controlling Code Paths

- Primary code path: `modules/core-modules/wipe-tower/wipe-tower.toml` (declaration) → `modules/core-modules/wipe-tower/src/lib.rs` (`WipeTower` struct + `from_config` read + `run_finalization` corner validation — the one live wiring), both feeding the guest build.
- Neighboring tests/fixtures: `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` (the point-string ingest pin + the bed-bounds fixtures this packet extends), `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (geometry invariants), `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` (the schema-parse test shape to mirror).
- OrcaSlicer comparison: `packet.spec.md` §OrcaSlicer Reference Obligations owns the file list; do not repeat delegation rules here.

## Architecture Constraints

- **Absence-identity is the packet's safety property.** With no `bed_exclude_area` in config, the stored polygon is empty, the exclusion check skips, and `run_finalization`'s output is byte-identical to today's — including the pre-existing "outside bed polygon" fatal. All test fallout is *new positive/negative cases only*; no baseline churn.
- **Degenerate ≠ error.** Canonical's default is a single point `Vec2d(0,0)` and `get_bed_excluded_area` builds one polygon from whatever points exist — a sub-triangle polygon geometrically excludes nothing (`Print::validate`'s hull intersection with it is empty-by-construction). The port mirrors this: an empty list, odd-length, or < 6-value raw list produces an *empty exclusion set* (check skipped), not a `ModuleError`. Do not reuse `parse_printable_area`'s fatal validation for the exclusion polygon — a missing exclusion area is not a misconfiguration; `printable_area` (required, must be a usable bed) and `bed_exclude_area` (optional, degenerate-to-nothing) have deliberately different error contracts.
- **On-edge is inside.** The existing `point_in_polygon` treats on-edge points as inside (the bed check uses that to reject corners exactly on the bed rim); the exclusion check shares it — a corner exactly on the exclusion boundary is rejected, matching the conservative reading of "too close".
- **No host-side transport work exists for this key.** The percent-default threading (packet 185) does not apply — there is no schema default to thread; the CONFIG_BLOCK gains nothing at defaults. If a user/Orca 3MF supplies the key, it rides the generic extensions bucket (verified at authoring time: `bind_module_config_view` → `ConfigView::from_declared` is an untransformed exact-name copy).
- **Do not reuse the `required` key's machinery.** `required = true` decorates the entry for readability (matching `printable_area`) but the manifest parser does not read it (`ConfigFieldEntry` has no `required` field); absence of the key is handled by the module's fallback, not schema enforcement. Never *enforce* `bed_exclude_area`'s presence — canonical treats it as optional.
- **Both bed geometry keys are mm-domain module-local values.** The polygons live in module `f32` mm space end-to-end (config → `from_config` → `run_finalization`); no IR `Point2` unit conversion crosses this boundary (the module emits its own mm-space entities; `docs/08_coordinate_system.md`'s 100 nm rule applies only where IR geometry units are involved — it is not, here).
- <!-- snippet: wasm-staleness -->Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Schema/version constants: none touched. The module's WIT world is unchanged; config travels the existing `ConfigView` path.

## Code Change Surface

- Selected approach: declare one key in the owning manifest; read it into `WipeTower` beside `printable_area`; extend `run_finalization`'s corner loop with the exclusion test; pin contract + behaviour + ingest + non-leakage in tests. Scheduler side gets only tests, zero production changes.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` — +1 `[config.schema.bed_exclude_area]` table (float-list, no default, no min/max, display/group/advanced as AC-1).
  - `modules/core-modules/wipe-tower/src/lib.rs` — `WipeTower` struct: new `bed_exclude_area: Vec<(f32, f32)>` field (default empty); `from_config`: read via `float_list_from_config(config, "bed_exclude_area")`, then `raw.chunks(2)` → `Vec<(f32, f32)>` **accepting anything even-length ≥ 6 values** and producing an **empty vec for empty/odd/<6 raw values** (degenerate-to-nothing, never an error — see Architecture Constraints); `run_finalization`: after the existing 4-corner bed-polygon loop, run the same corner list against `self.bed_exclude_area` (re-parsed from config live, mirroring how `run_finalization` re-reads `printable_area`) via `point_in_polygon`, returning `ModuleError::fatal(3, "wipe-tower corner ({x}, {y}) lies inside bed_exclude_area; risk of collision when printing")` on the first hit. A one-line comment at the check notes the canonical asymmetry (canonical validates object hulls, not the tower; this port's live decision point is the tower rectangle).
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` (extended) — AC-3: one appended test mirroring `orca_point_string_bed_is_parsed_not_silently_defaulted`'s fixture shape — Orca point-string `bed_exclude_area` (`["0x0", "20x0", "20x20", "0x20"]`) with a tower corner at (10, 10) inside it → `run_finalization` errs (the ingest reaches the check, not silently defaulted away).
  - `crates/slicer-scheduler/tests/wipe_tower_p04_binding_tdd.rs` (new; flat file, auto-discovered binary) — AC-N1: mirror the `LoadedModuleBuilder`/`ConfigView::from_declared` fixture shape (`config_resolution_tdd.rs` precedent): a non-owner module's view hides `bed_exclude_area` present in the source, while the wipe-tower module's own view exposes it.
- Rejected alternatives and reasons:
  - *Validating object convex hulls instead of / alongside the tower rectangle*: the object-hull decision point does not exist in this tree (no per-object hull geometry reaches a validation site — that is Print::validate-level work); building it is Tier B/C, not Tier A plumbing. The tower-rectangle check is what this port can enforce at the existing seam; the divergence is gap-recorded.
  - *Reusing `parse_printable_area` (fatal on malformed) for the exclusion polygon*: wrong error contract — the bed is required and must be valid, the exclusion area is optional and degenerates to no-op. A malformed exclusion area killing the slice would make the *default-adjacent* behaviour (Orca's own empty/degenerate values) fatal.
  - *Emulating `GCode.cpp::get_path_of_change_filament`'s "4 points = rectangle" reading*: canonical's consumers disagree (`get_bed_excluded_area` reads one polygon; `Model.cpp` groups 4-point rectangles; `get_path_of_change_filament` demands exactly 4 points); this packet follows the validation consumer — the one semantics a slice must honour to be safe.
  - *Adding a schema `default` (e.g. the degenerate `[0, 0]` point)*: would make doc-15 emit a numeric/default-bearing row inviting a spurious Orca-deviation comparison and would emit a CONFIG_BLOCK line at defaults; the absent-key fallback already carries the identical semantics.
  - *Wiring the check into `process()` (the legacy path)*: it carries no bed-bounds validation today (the bed-bounds work lives in `run_finalization` only); adding it there would be new behaviour on a path marked for retirement (TODO(packet-41)), not plumbing at an existing decision point.

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `modules/core-modules/wipe-tower/wipe-tower.toml` - role: owner manifest (declaration); expected change: +1 schema entry.
- `modules/core-modules/wipe-tower/src/lib.rs` - role: the module (one field + one read + one validation extension + comment); expected change: field, read site, corner-loop extension with fatal message.
- `modules/core-modules/wipe-tower/tests/wipe_tower_bed_exclude_area_tdd.rs` - role: new module test binary (AC-1, AC-2); expected change: new file.

Justified extras (tests only, no production surface): `modules/core-modules/wipe-tower/Cargo.toml` (+ `toml = "0.8"` dev-dep, part-cooling pattern — the schema test parses TOML directly), `crates/slicer-scheduler/tests/wipe_tower_p04_binding_tdd.rs` (new; AC-N1).

## Read-Only Context

Include ranges for files over 300 lines.

- `modules/core-modules/wipe-tower/src/lib.rs` - lines 30-60 (struct + reader), 82-108 (`parse_printable_area`'s error contract — contrast, not reuse), 141-208 (`from_config`), 470-560 (`run_finalization` corner loop) only - purpose: wiring target; the file is ~700+ lines, read ranged.
- `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` - full (≤ 300 lines) - purpose: the point-string fixture shape + `config_from_pairs` helper to reuse.
- `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` - full (≤ 200 lines) - purpose: manifest-parse test shape to mirror.
- `crates/slicer-ir/src/resolved_config.rs` - lines 630-680 only (`parse_orca_point_string` + its doc comment) - purpose: the reader arm's guarantee.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` (sibling path `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- All `crates/**` production files (`resolved_config.rs`, `feedrate.rs`, `config_resolution.rs`, `manifest.rs`, `execution_plan.rs`, `loader.rs`) - the generic plumbing already behaves as asserted; facts are quoted in `requirements.md` §Verified Grounding; delegate any further lookup
- `crates/slicer-gcode/src/serialize.rs` - never edited (no padding literal exists for `bed_exclude_area`; no CONFIG_BLOCK change at defaults)
- Every other module manifest - unrelated owners; delegate symbol lookups
- `docs/DEVIATION_LOG.md` - no row expected; filing one requires human sign-off surfaced first (ticket 02 standard)

## Expected Sub-Agent Dispatches

- Question: does any test pin the absence of extra fatal returns from `run_finalization` or count its error paths (e.g. a test asserting exactly one fatal code-3 site)? scope: `modules/core-modules/wipe-tower/` + `crates/slicer-runtime/tests/`; return: `LOCATIONS` (≤ 10 entries); purpose: Step 3 fallout list — **authoring-time survey found none** (the error surface is asserted only via `is_err()` shapes in `bed_bounds_tdd.rs`), re-derive before editing.
- Question: does any baseline/golden pin wipe-tower output for a config that *includes* a `bed_exclude_area`-shaped key in its source map (which would now appear in a CONFIG_BLOCK where before it was hidden)? scope: `crates/slicer-runtime/tests/` + `crates/slicer-gcode/tests/`; return: `LOCATIONS` (≤ 10 entries); purpose: at defaults nothing changes (no schema default exists); only user-supplied values newly appear in CONFIG_BLOCK — verify no golden pins a config-block body that would gain the key.
- Question: quote the `LoadedModuleBuilder` + `ConfigFieldEntry` fixture shape for a module-config test? scope: `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`; return: `SNIPPETS` (≤ 30 lines); purpose: Step 4 fixture shape — verified at authoring time (§Verified Grounding); re-dispatch only if the file moved.

## Data and Contract Notes

- IR/manifest contracts: no IR shape change; the `[config.schema]` entry follows the existing `ConfigFieldEntry` wire shape (field_type required; default/min/max/display/group/description/advanced optional). `float-list` requires no bounds (`is_numeric_field_type` governs min/max applicability; list bounds enforcement is per-element where declared — none declared here).
- WIT boundary: none touched — the module's WIT world is unchanged; only its config schema grows (transported through the existing `ConfigView` path).
- Determinism/scheduler constraints: the purge entity set, ordering, and insertion positions are unchanged; the exclusion check adds no I/O and no ordering dependency; `layer-parallel-safe = false` in `[hints]` is untouched.

## Locked Assumptions and Invariants

- With no `bed_exclude_area` entry: identical output to today, including the pre-existing bed-bounds fatal for an out-of-bed tower.
- With a degenerate value (empty / odd / < 6 values): no exclusion enforced, no error raised (canonical's degenerate default excludes nothing).
- With a valid exclusion polygon: a tower corner inside it (on-edge counts as inside) → fatal error naming `bed_exclude_area` and the corner; a tower clear of it → unchanged `Ok` output.
- The check lives only in `run_finalization` (the live SDK path); `process()` stays untouched.
- No host-crate production change; no schema/version constant bump; the wipe-tower guest is rebuilt before any test run touching it.
- The single-key parity table carries the object-hull gap and the three secondary consumers as recorded future work (no silent drop).

## Risks and Tradeoffs

- **The wired check is weaker than canonical's** (tower rectangle vs object hulls): a slice with an *object* overlapping the exclusion area but no wipe tower still slices clean where Orca would fail. This is the honest Tier-A boundary at this seam — the gap is recorded in the per-key table, and the check direction (reject on overlap) matches canonical's failure mode. If the human prefers *no* check until the hull decision point exists, that is an authoring-time scoping call deferred to preflight/review — the packet ships the tower-rectangle check as the local translation (see Open Questions).
- **On-edge inclusion could surprise**: a tower *touching* the exclusion boundary is rejected. Canonical's hull-intersection test would likewise reject boundary touching (intersection non-empty); conservative is correct here.
- **A user polygon overlapping the whole bed** (e.g. a mistaken full-bed exclusion): every tower placement fails — intended; the message names the key so the operator can clear it.
- **User-supplied values appear in CONFIG_BLOCK**: intended (Orca round-trip); no golden pins a config block containing this key today (survey dispatched at implementation time).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3: wiring + tests)
- Highest-risk dispatch and required return format: the fatal-surface pin `LOCATIONS` (≤ 10 entries) — if it returns more than 10 pin sites, the change is bigger than surveyed and the step must stop and re-scope to the coordinator.

## Open Questions

- **[FWD]** The reduced-semantics question — whether the port should eventually validate *object* footprints against the exclusion polygon (canonical's semantics) rather than the tower rectangle — is queue work, not this packet's: the object-side decision point does not exist. The per-key gap row holds the pointer; a future Print::validate-level packet (P18/P19 family or Tier-B orchestration work) picks it up.
- No `[BLOCK]` questions.