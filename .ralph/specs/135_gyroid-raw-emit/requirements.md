# Requirements: 135_gyroid-raw-emit

## Packet Metadata

- Grouped task IDs:
  - `TASK-260`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The gyroid module's wave math is correct, but three things around it are not. (1) It rotates
wave points around the UNROTATED polygon's bbox center (rotation block at `lib.rs:344`)
instead of rotating the polygon first — rotated-infill prints get geometrically wrong
waves. (2) It clips its own waves with per-vertex ray-casting
(`clip_polyline_to_expolygon` at `lib.rs:611`, with helpers `point_in_expolygon` at
`lib.rs:570` and `point_in_polygon` at `lib.rs:585`) that misses boundary crossings between
samples — and under Architecture A it should not clip at all (the 133 linker re-clips
correctly via `clip_polylines`). (3) Its multi-role emission code is dead: the manifest
declares only `claim:sparse-fill`, so the dispatch only routes the sparse role to gyroid
even when a user configures gyroid as the holder for other roles — the contradiction
ADR-0027 resolved by making multi-role a real opt-in. The DEV-082 row tracks this exact
divergence as Open since 2026-07-03; this packet is the realization.

Note: the plan's Phase 0 (`clip_polylines`) and Phase 1 (WIT contract) are already realized
(TASK-254 + TASK-255 closed). This packet implements Phase 3 only.

## In Scope

- Rotation-order fix: rotate the ExPolygon by −(base_angle + CorrectionAngle) first, compute
  the ROTATED bbox, generate axis-aligned waves there, rotate points back to world space,
  emit raw (FillGyroid.cpp:300-376 ordering). The current rotation block is at
  `lib.rs:344`; the comment "Apply rotation around bbox center" is the marker for the
  replacement.
- Delete `clip_polyline_to_expolygon` (lib.rs:611), `point_in_expolygon` (lib.rs:570),
  `point_in_polygon` (lib.rs:585), and `polygon_bbox_mm` (lib.rs:551). The structural-grep
  AC-6 in `packet.spec.md` §Verification is the contract.
- `align_to_grid` (new helper, ~10 lines): snap `bb.min` to a multiple of
  `2π × scale_factor` (FillGyroid.cpp:322).
- Expand factor 4.0 → 10.0 × spacing_mm at `lib.rs:259` (FillGyroid.cpp:326).
- Manifest: `claims.holds` gains `claim:top-fill`, `claim:bottom-fill`,
  `claim:bridge-fill` (ADR-0027; DEV-082).
- Per-region density via the packet-131 accessor in the region loop.
- TDD per AC-1…AC-6, AC-N1. There are no point-in-polygon tests in the test file
  (verified by FACT I 2026-07-19), so the spec's "delete point-in-polygon tests" is moot —
  only the four function deletions in lib.rs. The 11 existing test functions stay
  (with the rotation-block ones rewritten as needed); wave-core tests
  (`asin_nan_protection`, `square_region_produces_paths`, `paths_at_correct_z`,
  `wave_pattern_varies_by_layer`) stay green.

## Out of Scope

- Any change to `gyroid_f` (lib.rs:394), `make_one_period` (lib.rs:430), `make_wave`
  (lib.rs:491), the orientation choice, or the constants `DENSITY_ADJUST = 2.44`,
  `CORRECTION_ANGLE_DEG = -45.0`, `PATTERN_TOLERANCE = 0.2` (verified correct; spec "stays"
  list).
- Default fill-holder changes — `gyroid` is not referenced in
  `crates/slicer-ir/src/resolved_config.rs` defaults; the four fill-holder keys resolve
  to `rectilinear-infill` (DEV-082 opt-in promise).
- `multiline` support (spec P3 — deferred).
- Golden restore (136); changed-output goldens append to the 131 carve list.

## Authoritative Docs

- `docs/adr/0027-…` — binding; full read (short).
- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 3 — stays/deleted lists
  binding; Phase 3 only.
- `docs/DEVIATION_LOG.md` — DEV-082 row only.
- `docs/08_coordinate_system.md` — delegate SUMMARY (gyroid math is mm-domain; the mm↔unit
  boundary sits at polygon input and point emission).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:300-376` — `_fill_surface_single` rotation ordering (the fix being ported).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:322` — `align_to_grid` grid constant.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:326` — expand factor.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-9` in `packet.spec.md`. Refinements: AC-1's "no clipping"
  assertion is the Architecture-A pin (points may exceed the polygon; the linker clips);
  AC-2 is the rotation-fix regression pin (bbox) and AC-8 is its per-point counterpart
  (strict); AC-5/AC-6 are structural greps making the claims + deletions mechanically
  checkable; AC-7/AC-9 are the per-region density assertions (131 accessor wired into
  both `gyroid-infill` and `rectilinear-infill`, reading through
  `slicer_sdk::config_resolution::resolve_float`).
- Negative cases: `AC-N1` — the DEV-082 opt-in guard: default config must produce
  sparse-only gyroid (held-claims gating), keeping default behavior at OrcaSlicer parity.
- Cross-packet impact: output changes (correct rotation + linker-clipped wave extents) —
  affected goldens append to the 131 carve list; 136 restores. The multi-role opt-in path is
  e2e-exercised in 136's integration only if cheap (spec M3 note).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p gyroid-infill 2>&1 \| tee target/test-output.log \| grep "^test result"` | full module suite | FACT + counts |
| `cargo clippy -p gyroid-infill --all-targets -- -D warnings` | lint gate | FACT |
| `cargo xtask build-guests --check` (rebuild if STALE) | guest freshness (src + manifest edited) | FACT clean/STALE |
| `cargo check --workspace --all-targets` | no downstream breakage | FACT |
| the AC-5/AC-6 rg one-liners | structural claims/deletion checks | FACT |

## Step Completion Expectations

- Cross-step invariant: the wave-core functions and constants are byte-identical at packet
  close (`git diff` over their line ranges is empty except moved context) — any change there
  is a scope violation.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: the module's `lib.rs`
  (695 lines — one full read at Step 1, ranged after); OrcaSlicer refs delegated.
- Likely temptation reads: the linker module (how it will clip these waves) — skip; the
  module's contract is "emit raw waves bounded by the expanded bbox", nothing more.
- Sub-agent return-format hints: Orca dispatches SUMMARY + SNIPPETS ≤30 lines; cargo FACT.
