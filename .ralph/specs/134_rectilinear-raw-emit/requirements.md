# Requirements: 134_rectilinear-raw-emit

## Packet Metadata

- Grouped task IDs:
  - `TASK-259`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The rectilinear module is the default holder of all four fill roles, and its scan-line core is
wrong in ways the 2026-07-01 gap analysis pinned: it merges edges from ALL expolygons into one
global intersection sort (`lib.rs:231-237` — incorrect for overlapping or multi-island
regions), lacks a vertex-safe intersection discipline, ignores `adjust_solid_spacing` (solid
shells get partial lines at boundaries), and resolves angles without the
bridge > per-layer > base priority. Under Architecture A its raw 2-point emission shape is
already right — what's wrong is the geometry inside. Every default print inherits these bugs
for all four roles.

## In Scope

- Rewrite of the scan-line core in `modules/core-modules/rectilinear-infill/src/lib.rs`:
  `infill_direction` port (angle priority, +π/2, bbox-center reference), float-rotate by
  −angle (f64, round to i64; ≤ 50 nm error), per-ExPolygon scan conversion (half-open edge
  test), `adjust_solid_spacing` for solid roles, rotate-back, raw 2-point emission,
  `pattern_shift` applied to the scan-line start x (grilling decision: module-side).
- Per-region config reads through the packet-131 region accessor inside the region loop
  (density/line_width/angle keys; modifier sub-regions get their own density for free).
- Deletion of `fill_expolygon_multi` and `collect_edges` (the global-edge-merge path).
- TDD suite per AC-1…AC-7, AC-N1 in `modules/core-modules/rectilinear-infill/tests/`.
- OrcaSlicer attribution header on the rewritten file(s).

## Out of Scope

- Anything on the spec's "NOT added" list: `ExPolygonWithOffset`, link graph, traversal,
  `connect_infill`/`chain_or_connect_infill`, `INFILL_OVERLAP_OVER_SPACING` — all linker
  concerns (ADR-0025; the linker shipped in 133).
- Rectilinear SUBCLASS patterns (Grid, Triangles, Stars, Cubic, ZigZag, Monotonic, …) — out
  of scope per the spec's Scope section.
- Manifest/claims changes (already holds all four fill-role claims).
- `multiline_fill`, `fill_surface_trapezoidal` — deferred per spec.
- Golden restore (136); any changed-output goldens join the 131 carve list as recorded
  deviations.

## Authoritative Docs

- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 2 — the algorithm contract
  (the "stays / deleted / NOT added" lists are binding); load Phase 2 only.
- `docs/adr/0025-…` — delegate SUMMARY (raw-emit boundary).
- `docs/08_coordinate_system.md` — delegate SUMMARY (rotation rounding, ÷100 rule).
- `docs/ORCASLICER_ATTRIBUTION.md` — header template.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:2979-3143` — scan-line driver (stop before link-graph stages).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:842-1154` — vertical-line slicing + edge intersection discipline.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:3023-3024` — `pattern_shift`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:352-391` — `infill_direction`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:326-340` — `adjust_solid_spacing`.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-7` in `packet.spec.md`. Refinements: AC-1's exact segment-count
  formula makes density→spacing derivation falsifiable; AC-4's inverse-rotation equivalence is
  the regression pin for the rotate-first ordering; AC-7 pins the grilling decision that
  `pattern_shift` lives module-side.
- Negative cases: `AC-N1` — the half-open vertex test, the classic scan-line correctness trap.
- Cross-packet impact: output changes (correct geometry + linker linking it) — affected
  goldens append to the 131 carve list; restored in 136.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p rectilinear-infill 2>&1 \| tee target/test-output.log \| grep "^test result"` | full module suite | FACT + counts |
| `cargo clippy -p rectilinear-infill --all-targets -- -D warnings` | lint gate | FACT |
| `cargo xtask build-guests --check` (rebuild if STALE) | guest freshness after module edit | FACT clean/STALE |
| `cargo check --workspace --all-targets` | no downstream breakage | FACT |

## Step Completion Expectations

- Cross-step invariant: the "stays" list (four-role emission structure, `solid_fill_role`
  mapping, `should_emit` gating, manifest) must survive every step — a diff touching the
  manifest or the role structure is a scope violation, not a refactor.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: the module's own
  `lib.rs` (361 lines — full read allowed once); ALL OrcaSlicer refs delegated (the two
  FillRectilinear ranges total ~500 lines — dispatch sectioned).
- Likely temptation reads: `modules/core-modules/gyroid-infill/**` (packet 135's surface) and
  the linker module — skip both; the raw-emit boundary is specified, not discovered.
- Sub-agent return-format hints: Orca dispatches return SUMMARY + per-section SNIPPETS ≤30
  lines; cargo runs FACT.
