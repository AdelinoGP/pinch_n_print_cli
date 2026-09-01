# Design: infill-pattern-keys

## Controlling Code Paths

- Primary code path: the infill modules' config reads and emission loops —
  `RectilinearInfill::from_config` + `run_infill` (`modules/core-modules/rectilinear-infill/src/lib.rs`:
  `base_angle` from `infill_direction` at `from_config`, `angle_deg = self.base_angle` at
  the top of `run_infill`, `scan_expolygon` per role with `x_shift_units` from
  `infill_shift_step`) and `GyroidInfill::from_config` + `fill_expolygon`
  (`modules/core-modules/gyroid-infill/src/lib.rs`: `base_angle` from `infill_direction`,
  `infill_direction_rad = (base_angle + CORRECTION_ANGLE_DEG).to_radians()`, expolygon
  rotate-in/back-rotate-out). The wired keys add per-role per-layer angles and the sparse
  multiline copies at these existing seams.
- Neighboring tests/fixtures:
  `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` (the
  `make_config` + `RectilinearInfill::from_config` + `run_infill` + `InfillOutputBuilder`
  harness this packet's AC-2/3/4/5 arms extend — `angle_45_rotated_output_matches_unrotated_after_inverse`
  and `pattern_shift_interleaves_layers` are the angle/per-layer patterns);
  `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` (the AC-2/3/4 gyroid
  arms); guard-pattern source
  `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` (TOML-direct
  parse; part-cooling's Cargo.toml carries the `toml = "0.8"` dev-dependency
  rectilinear-infill will need, verified absent at authoring). Integration arms:
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (loads the
  real `rectilinear-infill.toml` via `load_module_from_paths` + `ConfigBoundsIndex::from_modules`
  + `resolve_global_config` — the `rejects_value_below_min` /
  `rejects_unknown_support_style_value` arms are the pattern) and
  `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
  (proven CONFIG_BLOCK driver at packet 258/259/260/261 authoring time:
  `run_pipeline_with_raw_config` + `region_between`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- snake_case config key strings only (repo convention): all 7 keys and the existing
  `config.get` strings are already snake_case by construction here.
- The wired keys are default-path identity: at canonical defaults the emitted `InfillIR`
  is byte-identical to pre-packet behavior (AC-2). The `from_config` fallbacks for the
  new keys are the canonical defaults (45.0 / "" / 1), so absent-key behavior matches
  declared-default behavior.
- The declared-with-gap keys are declared in the manifests but **never read** in any
  module source — declaring them must not perturb behavior (AC-2).
- The template parser is duplicated per module (rectilinear + gyroid), not shared via
  `slicer-sdk`: a slicer-sdk change would ripple into all 44 guests' fingerprints; the
  modules already duplicate small helpers (`solid_fill_role` exists in both).
- The enum `values` lists are canonical-exact (26-value InfillPattern list for
  `sparse_infill_pattern`; 8-value top-fill list for `internal_solid_infill_pattern`;
  everywhere/topbottom/nowhere for `gap_fill_target`) — the manifest enum validation
  rejects values outside the list (AC-6's `"bogus"` arm).
- ADR-0027 `gyroid-multi-role-fill-holder` conformance: this packet does not change the
  default `*_fill_holder` config (Decision #2 — the defaults stay
  `"rectilinear-infill"`), does not point solid roles at gyroid (Future-Reviewer note),
  and does not remove top/bottom/bridge emission from gyroid-infill (Future-Reviewer
  note). The wired solid-role angle keys apply to whichever module holds the solid-fill
  claim; the pattern keys are declared-with-gap and are NOT wired to the holder
  resolution. No amendment deviation is required.

## Code Change Surface

- Selected approach: declare the 17 tables across the three infill manifests (AC-1);
  wire the four keys in rectilinear (solid-role angle, per-layer templates, sparse
  multiline) and the three angle keys in gyroid (solid-role angle, per-layer templates);
  correct the `sparse_infill_pattern` padding twin; add the guard, behavior,
  bounds/enum, and CONFIG_BLOCK arms; regenerate the docs and rebuild the guests.
- Exact functions, traits, manifests, tests, and fixtures:
  `rectilinear-infill.toml` / `gyroid-infill.toml` / `lightning-infill.toml`
  `[config.schema]` (17 tables, AC-1); `infill_config_schema_tdd.rs` (net-new guard,
  AC-1/N1/N2); `rectilinear-infill/src/lib.rs` (`from_config` reads + `run_infill`
  per-role angle + sparse multiline + module-local `template_angle`/translate helpers);
  `gyroid-infill/src/lib.rs` (`from_config` reads + `fill_expolygon` per-role angle +
  module-local `template_angle`); `rectilinear_raw_emit_tdd.rs` (AC-2/3/4/5 arms);
  `gyroid_infill_tdd.rs` (AC-2/3/4 arms); `crates/slicer-gcode/src/serialize.rs`
  (padding twin `("sparse_infill_pattern", "grid")` → `"crosshatch"`);
  `gcode_header_thumbnail_config_blocks_tdd.rs` (AC-7 arms);
  `config_bounds_enforcement_tdd.rs` (AC-6 arms); the module's `Cargo.toml`
  (`toml = "0.8"` dev-dep, add-if-absent); `docs/15_config_keys_reference.md`
  (generated, Step 7).
- Rejected alternatives and reasons:
  - *Wiring the pattern keys to the holder resolution* (map `sparse_infill_pattern` /
    `internal_solid_infill_pattern` → `*_fill_holder` in the host) — rejected: the port
    implements 3 of 26 canonical patterns; a mapping would silently degrade the other 23
    to rectilinear, which is worse than an honest declared-with-gap. The mapping is
    host-side config-resolution work, recorded as the port-side decision point.
  - *Wiring `gap_fill_target` to the perimeter-side gap fill* (suppress
    classic-perimeters/arachne-perimeters gap emission at `nowhere`) — rejected: the
    port's gap fill is canonical's `process_classic` perimeter mechanism, which
    canonical's `gap_fill_target` does not gate; suppressing it would change default
    behavior against canonical. The fill-side gap fill (`_create_gap_fill`) does not
    exist in-tree; declared-with-gap.
  - *Porting the template metalanguage* — rejected: the metalanguage (joints, repeats,
    units, shell counts) is a large parser for an exotic form; the comma-separated list
    form is the wired scope, metalanguage strings fall back to the base angle with a
    logged warn (recorded degradation, default "" unaffected).
  - *Wiring `fill_multiline` in gyroid/lightning* — rejected: offsetting curved gyroid
    paths (and lightning tree segments) needs real curve-offset machinery (canonical
    uses Clipper2 `ClipperOffset` with Round joins); straight scan lines translate
    exactly. Tier B+; declared-with-gap in both.
  - *Sharing the template parser via `slicer-sdk`* — rejected: a slicer-sdk change
    ripples into all 44 guests' fingerprints; the two modules already duplicate small
    helpers (`solid_fill_role`). Duplicated ~15-line parser per module.
  - *Adding `ORCA_CONFIG_PADDING` twins for the other five keys* — rejected: packet
    254/255/257/258/259/260/261 precedent says module-manifest defaults do not thread
    into raw config; the block carries exactly the two existing padding lines at
    defaults (AC-7 pins the honest absence).
  - *Leaving the `("sparse_infill_pattern", "grid")` padding twin uncorrected* —
    rejected: the value contradicts the canonical default (`crosshatch`); ticket 14's
    `fuzzy_skin` padding-correction precedent requires alignment.

## Files in Scope (read + edit)

- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — role: owner manifest (default fill holder); expected change: 7 tables added (AC-1).
- `modules/core-modules/gyroid-infill/gyroid-infill.toml` — role: owner manifest (alternate fill holder); expected change: 7 tables added (AC-1).
- `modules/core-modules/lightning-infill/lightning-infill.toml` — role: owner manifest (sparse-only holder); expected change: 3 tables added (AC-1/AC-N2).
- `modules/core-modules/rectilinear-infill/tests/infill_config_schema_tdd.rs` — role: net-new guard test (AC-1/N1/N2); expected change: created.
- `modules/core-modules/rectilinear-infill/src/lib.rs` — role: wired module source; expected change: `from_config` reads + `run_infill` per-role angle + sparse multiline + helpers (AC-2/3/4/5).
- `modules/core-modules/gyroid-infill/src/lib.rs` — role: wired module source; expected change: `from_config` reads + `fill_expolygon` per-role angle + helper (AC-2/3/4).
- `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` — role: module suite; expected change: AC-2/3/4/5 arms.
- `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` — role: module suite; expected change: AC-2/3/4 arms.
- `modules/core-modules/rectilinear-infill/Cargo.toml` — role: dev-deps; expected change: +`toml = "0.8"` (add-if-absent).
- `crates/slicer-gcode/src/serialize.rs` — role: padding table; expected change: one value correction (AC-7).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK arm; expected change: +AC-7 tests.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — role: scheduler arm; expected change: +AC-6 tests.
- `docs/15_config_keys_reference.md` — role: generated; expected change: regenerated via `cargo xtask gen-config-docs` (AC-8).

## Read-Only Context

- `modules/core-modules/rectilinear-infill/src/lib.rs` — lines `40-84` (struct fields), `88-196` (`from_config`), `198-260` (`run_infill` head + angle), `280-400` (sparse/top/bottom/bridge scans), `577-640` (`scan_expolygon` signature) — purpose: the wiring seams.
- `modules/core-modules/gyroid-infill/src/lib.rs` — lines `88-160` (`from_config`), `330-360` (`fill_expolygon` angle), `670-700` (`rotate_expolygon` — the module-local helper precedent) — purpose: the wiring seams.
- `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs` — full — purpose: guard-test pattern source.
- `modules/core-modules/seam-planner-default/seam-planner-default.toml` lines `27-33` — purpose: the `enum` + `values` manifest form.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` lines `29-41` — purpose: the `string` manifest form.
- `crates/slicer-gcode/src/serialize.rs` lines `490-560` (`ORCA_CONFIG_PADDING` — `("sparse_infill_pattern", "grid")` at ~504, `("gap_fill_target", "nowhere")` at ~552) — purpose: AC-7's one-value correction.
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — full (~460 lines) — purpose: AC-6 arm pattern (real-manifest load + `OutOfRange`/`TypeMismatch`/enum assertions).
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — lines `1-120` (setup) + grep for an existing CONFIG_BLOCK assertion to mirror — purpose: AC-7 arm form.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (sibling path `..\pinch_n_print_cli\OrcaSlicerDocumented`).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/core-modules/classic-perimeters/` and `modules/core-modules/arachne-perimeters/` — the perimeter-side gap fill is context, not surface; never read beyond the `filter_out_gap_fill` gate's needs.
- `crates/slicer-gcode/src/serialize.rs` — read-only beyond the one padding value (AC-7 pins no other edits).
- `docs/spec_packets/253* … 261*` — other packets' directories are read-only context; only the named reference files above may be consulted.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does `config_bounds_enforcement_tdd.rs` drive the real `rectilinear-infill.toml` manifest through the bounds index for int/float/enum keys, and which existing test arms to mirror for the AC-6 cases (two int `OutOfRange`, two float `OutOfRange`, one `TypeMismatch`, one unknown-enum)?; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` + `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 6.
- Question: does the runtime CONFIG_BLOCK driver thread explicit module-declared keys (e.g. an explicit `sparse_infill_pattern = "gyroid"`) into `raw_config` for `serialize_config_block`, and does the padding twin get suppressed by the emitted-key dedup?; scope: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` + `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: Step 5.
- Question: does `cargo xtask gen-config-docs --check` pass after regeneration, do the 7 keys appear in the module-key table under the three owner columns, and does the deviations block still count 26?; scope: `docs/15_config_keys_reference.md` + xtask; return: `FACT`; purpose: Step 7.

## Data and Contract Notes

- IR/manifest contracts: the int/float/enum/string tables use the in-tree forms
  (`seam-planner-default.toml` `seam_position` for enum+values; `machine-gcode-emit.toml`
  `machine_start_gcode` for string; the `description` field is parsed by
  `crates/slicer-scheduler/src/manifest.rs`); bounds enforcement is host-side generic via
  `ConfigBoundsIndex::from_modules` — numeric min/max in `ConfigResolutionError::OutOfRange`,
  non-numeric value in `TypeMismatch`, unknown enum value rejected by the enum index
  (`resolve_global_config`, `crates/slicer-scheduler/src/config_resolution.rs`), verified
  for packet 259/260/261's keys.
- WIT boundary: none touched — no WIT/world changes; the 7 keys ride the existing
  `ConfigView` string/int/float/bool plumbing.
- Determinism/scheduler constraints: the wired keys are read once in `from_config` and
  applied per layer from `layer_index` (deterministic); the template list cycles by
  `layer_index % len` (no RNG); the multiline copies are pure translations of the
  deterministic scan output. The declared-with-gap keys are unread and cannot reach any
  computation. AC-2's byte-identity comparison relies on the modules' existing
  determinism (same inputs → same paths — the suites' existing angle tests already
  depend on this).

## Locked Assumptions and Invariants

- Default-path identity: with the 7 keys absent or explicit-canonical-default, the
  rectilinear and gyroid modules emit byte-identical `InfillIR` (AC-2).
- The wired keys are read only in the modules that declare them; the declared-with-gap
  keys are read nowhere (AC-2).
- `lightning-infill.toml` does not declare the 4 solid keys (AC-N2).
- `serialize_config_block` and the padding table are untouched beyond the one
  `sparse_infill_pattern` value correction — no other CONFIG_BLOCK twins (AC-7).
- No WIT/IR schema changes; no deviation-table additions — the block stays at 26 data
  rows (AC-8).
- The template metalanguage is out of scope: metalanguage strings fall back to the base
  angle with a logged warn (recorded degradation).

## Risks and Tradeoffs

- The pattern keys are honest-but-inert today: a user setting `sparse_infill_pattern` or
  `internal_solid_infill_pattern` sees no behavior change until a pattern-dispatch packet
  lands (host-side holder mapping). This is the queue's declared-with-gap contract
  (packet 259/260/261 precedent), pinned by AC-2 so the inertness is tested, not assumed.
- The recorded divergences (port default sparse = rectilinear vs canonical crosshatch;
  port solid = rectilinear vs canonical monotonic) are port-state records; a future
  pattern-dispatch packet must consciously revisit them.
- The multiline implementation (translate-scan-translate) is behaviorally equivalent to
  canonical's pre-expanded polygon + `multiline_fill` for straight scan lines, but the
  emitted line set differs in edge cases canonical's ClipperOffset handles (Round joins
  on non-straight paths) — irrelevant here because rectilinear scan lines are straight;
  recorded in the wiring notes.
- The guard tests require the `toml = "0.8"` dev-dependency in rectilinear-infill's
  Cargo.toml — add-if-absent per packet 257/258/259/260/261 precedent (verify, don't
  assume).
- The guest rebuild after Steps 1-4 is mandatory before the integration arms (Steps
  5-6) dispatch real guests — a stale guest surfaces as unrelated failures (wasm-staleness
  snippet).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 — rectilinear wiring: `lib.rs` ranged reads + behavior arms)
- Highest-risk dispatch and required return format: the CONFIG_BLOCK driver question —
  `FACT` (a wrong assumption about how explicit keys reach `raw_config` and whether the
  padding dedup suppresses the twin would make AC-7 unbuildable; the dispatch must pin
  the mechanism).

## Open Questions

- `[FWD]` Does the rectilinear module have an existing polygon/path translate utility,
  or does the multiline step need a new module-local translate helper (gyroid's
  `rotate_expolygon` is the precedent)? Either answer changes no contract here; the
  Step-3 dispatch settles it.
- `[FWD]` Does the CONFIG_BLOCK driver in `gcode_header_thumbnail_config_blocks_tdd.rs`
  thread explicit module-declared keys into `raw_config` via the same per-test config
  injection packet 258/259/260/261 used, or does it need a new injection path for the
  seven keys? Either answer changes no contract here; the Step-5 dispatch settles it.
- No `[BLOCK]` questions.
