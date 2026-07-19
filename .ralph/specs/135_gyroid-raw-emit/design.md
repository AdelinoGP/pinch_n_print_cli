# Design: 135_gyroid-raw-emit

## Controlling Code Paths

- Primary code path: `modules/core-modules/gyroid-infill/src/lib.rs` — the per-region role
  loop (around `fill_expolygon` at lib.rs:219) stays; the rotation block at lib.rs:344
  (comment: "Apply rotation around bbox center") is replaced with polygon-first ordering;
  the `4.0 * spacing_mm` expand at lib.rs:259 becomes `10.0 * spacing_mm`; the four clipper
  helpers at lib.rs:551, 570, 585, 611 are deleted.
- Manifest: `modules/core-modules/gyroid-infill/gyroid-infill.toml` — `claims.holds` gains
  three entries.
- Neighboring tests or fixtures: `modules/core-modules/gyroid-infill/tests/
  gyroid_infill_tdd.rs` — 11 existing tests stay green (with the rotation-block-affected
  ones rewritten alongside, each rewrite names the deleted behavior); new AC tests added.
  No point-in-polygon tests exist in the test file (verified 2026-07-19), so the spec's
  "delete point-in-polygon tests" is moot.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Raw-emit boundary (ADR-0025): no clipping, no filtering, no chaining in the module.
- ADR-0027 Future-Reviewer notes are binding: do not remove the multi-role emission "to
  match OrcaSlicer"; do not change default fill-holders (the four fill-holder keys
  resolve to `rectilinear-infill` per `crates/slicer-ir/src/resolved_config.rs`).
- WIT contract is already in place (TASK-255 closed 2026-07-17). Do NOT re-add
  WIT/scheduler work — the host partition contract is realized.

## Code Change Surface

- Selected approach: mirror FillGyroid.cpp:300-376 ordering exactly — integer-rotate the
  ExPolygon by −(base_angle + CORRECTION_ANGLE), take ITS bbox, `align_to_grid(bb.min,
  2π × scale_factor)`, expand by 10 × spacing_mm, generate waves axis-aligned in that bbox
  (mm domain, unchanged core), rotate the wave points back by +(base_angle +
  CORRECTION_ANGLE) at emission. Emit every wave polyline raw.
- Exact changes: rotation block replacement; four function deletions
  (`clip_polyline_to_expolygon`, `point_in_expolygon`, `point_in_polygon`,
  `polygon_bbox_mm`); `align_to_grid` helper (new, ~10 lines); expand constant; manifest +3
  claims; per-region density read via the 131 accessor; tests.
- Rejected alternatives: (a) keeping module-side clipping "for tidiness" — rejected: it is
  both broken (per-vertex) and the linker's job; (b) rotating waves around the rotated bbox
  center (half-fix) — rejected: only polygon-first matches Orca and makes AC-2 hold;
  (c) adding the claims without the emission code — moot: the emission code already exists
  and is correct (ADR-0027 point 3).

## Files in Scope (read + edit)

- `modules/core-modules/gyroid-infill/src/lib.rs` — role: the fixes; expected change:
  rotation block replacement (lib.rs:344), four function deletions (lib.rs:551, 570, 585,
  611), `align_to_grid` helper added (~10 lines), expand constant at lib.rs:259 (~120 lines
  net negative). Per-region `infill_density` / `line_width` read via the 131 accessor
  (forwarded to each `fill_expolygon` call).
- `modules/core-modules/gyroid-infill/gyroid-infill.toml` — role: multi-role claims; expected
  change: 3 lines added to `claims.holds`.
- `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` — role: TDD; expected
  change: +7 new tests (AC-1, AC-2, AC-3, AC-4, AC-7, AC-8, AC-N1) plus the regression
  helper `adjacent_layers_have_phase_coherent_bbox`; the existing 11 tests stay green;
  rotation-block-affected ones are rewritten with header comments naming each encoded bug.
  No point-in-polygon tests to remove (FACT I 2026-07-19: none exist in the test file).
- `modules/core-modules/rectilinear-infill/src/lib.rs` — role: per-region density
  consumer (parity fix for the same gap; rectilinear shares the 131 accessor pattern with
  gyroid). Per-region `infill_density` / `line_width` read via the 131 accessor
  (forwarded to each `scan_expolygon` call).
- `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` — role: TDD;
  expected change: +1 new test (AC-9 per-region density); existing tests stay green.
- `crates/slicer-sdk/src/config_resolution.rs` — role: shared `resolve_float` helper
  (one place that owns the per-region vs. global resolution rule; both modules consume
  it). Expected change: new file, ~30 lines + 3 unit tests.

## Read-Only Context

- `crates/slicer-sdk/src/views.rs` — the 131 config accessor + `should_emit` — ranged.
- `crates/slicer-ir/src/resolved_config.rs` — fill-holder key names only (gyroid is not
  referenced in defaults).
- `docs/DEVIATION_LOG.md` — the DEV-082 row only.
- `docs/03_wit_and_manifest.md` — the `claim:(sparse|top|bottom|bridge)-fill` catalog
  entries (the four claim IDs already exist; we add three to the module's `holds`).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `modules/core-modules/lightning-infill/**`,
  `modules/core-modules/infill-linker/**` — other packets' surfaces.
- Host crates; `target/`; `Cargo.lock` — never load.
- `modules/core-modules/rectilinear-infill/**` is **partially in-scope** (the per-region
  density read at lib.rs:158 and the new AC-9 test); other rectilinear surfaces (perimeter
  selection, scan_expolygon internals) remain out-of-bounds.

## Expected Sub-Agent Dispatches

- "SUMMARY + SNIPPETS (≤30 lines) of FillGyroid.cpp:300-376: exact rotation ordering, where
  align_to_grid and the expand are applied, and what CorrectionAngle combines with" — Step 2
  driver.
- "FACT: the exact grid constant passed to align_to_grid at FillGyroid.cpp:322 (≤3 lines)".
- "Run `cargo test -p gyroid-infill 2>&1 | tee target/test-output.log | grep '^test
  result'`; FACT + counts; SNIPPETS ≤20 on failure" — every gate.
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE".
- "FACT: from `crates/slicer-sdk/src/views.rs`, the exact accessor for per-region config
  (≤ 5 lines)" — Step 1 driver.

## Data and Contract Notes

- IR/manifest contracts: manifest claim addition only — the four claim ids already exist in
  the fill-claim catalog; no scheduler change (ADR-0027 consequence note).
- WIT boundary: none.
- Determinism: wave generation is closed-form; the only ordering concern is emission order of
  waves — keep generation order (spec: "module emits waves in generation order").

## Locked Assumptions and Invariants

- Wave core + constants byte-identical (requirements cross-step invariant). The byte-identical
  set is `gyroid_f` (lib.rs:394), `make_one_period` (lib.rs:430), `make_wave` (lib.rs:491),
  the orientation choice, and the constants `DENSITY_ADJUST`, `CORRECTION_ANGLE_DEG`,
  `PATTERN_TOLERANCE`.
- Default fill-holders unchanged: gyroid is not in `resolved_config.rs` defaults; default
  prints produce sparse-only gyroid when the user explicitly sets a fill-holder key
  (AC-N1, DEV-082's opt-in promise).
- Raw waves may extend past the polygon (bounded by the expanded bbox); downstream clipping
  is the linker's (AC-1 pins no-clipping).
- `solid_fill_role` mapping stays (shared shape with rectilinear; divergence between the two
  copies is out of scope per ADR-0027 note).
- The linker (packet 133, currently OPEN) is NOT a code-blocker. Output is raw waves until
  133 lands; the roadmap's degraded-not-failed trade-off (ADR-0025) is documented in
  the implementation plan and exercised by packet 136's AC-N1.

## Risks and Tradeoffs

- Existing tests that pinned clipped output go red — rewritten alongside (each rewrite names
  the deleted behavior), same discipline as packet 134.
- The 10× expand (vs current 4×) increases raw emission volume ~linearly with perimeter
  length; the linker clips it away — memory transient only, acceptable at current path
  counts.
- The z-phase orientation choice interacts with align_to_grid — AC-3's snapping test plus the
  kept wave-core tests guard the composition.
- The packet is shippable with TASK-258 open, but the user-visible print is degraded until
  133 lands. Document this in the implementation-plan pre-condition and in any closure
  log; the integration packet (136) AC-N1 pins the trade-off at the e2e level.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (rotation-order fix + deletions)
- Highest-risk dispatch: the 300-376 SUMMARY — single section, bounded.

## Open Questions

None. (The grid constant and expand factor are extracted by FACT dispatches; everything else
is locked by ADR-0027 and the spec's Phase 3 lists.)
