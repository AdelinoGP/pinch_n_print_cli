# Requirements: 129_clip-polylines

## Packet Metadata

- Grouped task IDs:
  - `TASK-254`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The infill-linker (packet 133) must re-clip raw infill segments against overlap-inset
boundaries, and today the only polyline-vs-polygon clipping in the workspace is gyroid's
per-vertex ray-casting `clip_polyline_to_expolygon`
(`modules/core-modules/gyroid-infill/src/lib.rs:611-636`), which misclassifies any segment
whose boundary crossing falls between sample points. The workspace has no generic open-path
clip primitive, although the `clipper2-rust 1.0.3` dependency already exposes one
(`Clipper64::add_open_subject` + `execute` with `solution_open`). The API was recorded from
the crate source `engine_public.rs:296,335` inside the cargo registry — this is a
`clipper2-rust` crate file, NOT a repo path, and is OUT-OF-BOUNDS for reading (verified
2026-07-01; pure Rust, wasm32-clean). Without this primitive, every downstream infill packet
would have to invent its own clipping.

## In Scope

- `pub fn clip_polylines(polylines: &[Vec<Point2>], clip: &[ExPolygon]) -> Vec<Vec<Point2>>`
  in `crates/slicer-core/src/polygon_ops.rs`, implemented on `Clipper64` open-subject
  execution (`ClipType::Intersection`, `FillRule::NonZero`, contours + holes fed as closed
  clip paths of one `Clipper64` run per the ExPolygon set).
- 8 TDD tests in `crates/slicer-core/tests/polygon_ops_tdd.rs` (AC-1…AC-6, AC-N1, AC-N2).
- One-line helper-surface mention in `docs/05_module_sdk.md`.

## Out of Scope

- Wiring any consumer (linker consumes in packet 133; gyroid's broken clipper is deleted in
  packet 135).
- `offset` / `inflate` behavior changes, `multiline_fill`, or any other `polygon_ops` change.
- Preserving polyline direction guarantees beyond what Clipper2 provides (document observed
  behavior in the function docs; the linker does not depend on direction).

## Authoritative Docs

- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` (~700 lines) — load §Phase 0 only
  (lines 124-176); delegate anything else.
- `docs/08_coordinate_system.md` (383 lines) — delegate SUMMARY if needed; this packet does no
  mm↔unit conversion (inputs and outputs are integer units).
- `docs/05_module_sdk.md` — line 63 region only.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-6` in `packet.spec.md`. Refinements: boundary-endpoint tolerance
  is ±2 units (Clipper2 integer rounding); AC-4's hole assertion is point-in-polygon strict
  interior (`< 0` distance), not on-boundary.
- Negative cases: `AC-N1` (outside dropped), `AC-N2` (empty inputs, no panic).
- Cross-packet impact: unblocks packet 133 (sole planned consumer). No behavior change for any
  existing caller — the function is new.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --test polygon_ops_tdd 2>&1 \| tee target/test-output.log \| grep "^test result"` | all 8 new tests + existing polygon_ops suite green | FACT pass/fail + counts |
| `cargo clippy -p slicer-core --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo check --workspace --all-targets` | no downstream breakage from the new export | FACT pass/fail |
| `cargo xtask build-guests --check` | slicer-core feeds guest builds; confirm freshness state | FACT clean/STALE list |
| `rg -q 'clip_polylines' docs/05_module_sdk.md && echo HIT` | Doc Impact grep | FACT HIT/miss |

## Step Completion Expectations

None. (Single-surface packet; per-step contracts in `implementation-plan.md` suffice.)

## Context Discipline Notes

- Large files in the read-only path: `crates/slicer-core/src/polygon_ops.rs` is > 600 lines —
  read only the imports (~line 78), the existing `intersect_64` wrapper pattern, and the
  insertion point; do not read the whole file.
- Likely temptation reads: the `clipper2-rust` crate source in the cargo registry — do NOT
  read it; the API facts are already recorded (`Clipper64::add_open_subject(&Paths64)`,
  `execute(clip_type, fill_rule, &mut closed, Some(&mut open)) -> bool`,
  `engine_public.rs:296,335`). If a signature mismatch surfaces, delegate a FACT dispatch for
  the exact signature rather than opening the crate.
- Sub-agent return-format hints: all cargo commands return FACT pass/fail; on failure,
  SNIPPETS with the failing assertion + ≤20 lines from `target/test-output.log`.
