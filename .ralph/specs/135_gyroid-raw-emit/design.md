# Design: 135_gyroid-raw-emit

## Controlling Code Paths

- Primary code path: `modules/core-modules/gyroid-infill/src/lib.rs` — the per-region role
  loop (lines ~180-210, stays) → wave generation entry (~332-352, rotation block replaced) →
  clipping call site (~356, deleted; raw emission replaces ~375's clipped-fragment push).
- Manifest: `modules/core-modules/gyroid-infill/gyroid-infill.toml` — `claims.holds` gains
  three entries.
- Neighboring tests or fixtures: `modules/core-modules/gyroid-infill/tests/
  gyroid_infill_tdd.rs` — point-in-polygon tests deleted with their functions; wave-core
  tests (`gyroid_f_no_nan`, `make_one_period_produces_points`) kept; new AC tests added.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Raw-emit boundary (ADR-0025): no clipping, no filtering, no chaining in the module.
- ADR-0027 Future-Reviewer notes are binding: do not remove the multi-role emission "to match
  OrcaSlicer"; do not change default fill-holders.

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
  rotation block + deletions + align_to_grid + expand (~120 lines net negative).
- `modules/core-modules/gyroid-infill/gyroid-infill.toml` — role: multi-role claims; expected
  change: 3 lines.
- `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` — role: TDD; expected
  change: +7 tests, point-in-polygon tests removed.

## Read-Only Context

- `crates/slicer-sdk/src/views.rs` — the 131 config accessor + `should_emit` (lines
  ~466-482) — ranged.
- `docs/DEVIATION_LOG.md` — the DEV-082 row only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `modules/core-modules/{rectilinear,lightning}-infill/**`,
  `modules/core-modules/infill-linker/**` — other packets' surfaces.
- Host crates; `target/`; `Cargo.lock` — never load.

## Expected Sub-Agent Dispatches

- "SUMMARY + SNIPPETS (≤30 lines) of FillGyroid.cpp:300-376: exact rotation ordering, where
  align_to_grid and the expand are applied, and what CorrectionAngle combines with" — Step 2
  driver.
- "FACT: the exact grid constant passed to align_to_grid at FillGyroid.cpp:322 (≤3 lines)".
- "Run `cargo test -p gyroid-infill 2>&1 | tee target/test-output.log | grep '^test
  result'`; FACT + counts; SNIPPETS ≤20 on failure" — every gate.
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE".

## Data and Contract Notes

- IR/manifest contracts: manifest claim addition only — the claim ids already exist in the
  fill-claim catalog and `FILL_CLAIM_IDS`; no scheduler change (ADR-0027 consequence note).
- WIT boundary: none.
- Determinism: wave generation is closed-form; the only ordering concern is emission order of
  waves — keep generation order (spec: "module emits waves in generation order").

## Locked Assumptions and Invariants

- Wave core + constants byte-identical (requirements cross-step invariant).
- Default fill-holders unchanged: default prints produce sparse-only gyroid (AC-N1,
  DEV-082's opt-in promise).
- Raw waves may extend past the polygon (bounded by the expanded bbox); downstream clipping
  is the linker's (AC-1 pins no-clipping).
- `solid_fill_role` mapping stays (shared shape with rectilinear; divergence between the two
  copies is out of scope per ADR-0027 note).

## Risks and Tradeoffs

- Existing tests that pinned clipped output go red — rewritten alongside (each rewrite names
  the deleted behavior), same discipline as packet 134.
- The 10× expand increases raw emission volume ~linearly with perimeter length; the linker
  clips it away — memory transient only, acceptable at current path counts.
- The z-phase orientation choice interacts with align_to_grid — AC-3's snapping test plus the
  kept wave-core tests guard the composition.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (rotation-order fix + deletions)
- Highest-risk dispatch: the 300-376 SUMMARY — single section, bounded.

## Open Questions

None. (The grid constant and expand factor are extracted by FACT dispatches; everything else
is locked by ADR-0027 and the spec's Phase 3 lists.)
