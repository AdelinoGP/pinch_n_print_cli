---
status: draft
packet: 135_gyroid-raw-emit
task_ids:
  - TASK-260
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 135_gyroid-raw-emit

## Goal

Bring `gyroid-infill` to raw-emit parity: fix the rotation order (rotate the polygon first,
per FillGyroid.cpp:300-376), delete the broken per-vertex clipping, add `align_to_grid` phase
coherence and the 10× expand factor, and make the module multi-role by adding the three solid
claims to its manifest (ADR-0027 / DEV-082). The DEV-082 row is currently `Open` (recorded
2026-07-03) and this packet realizes the fix.

## Scope Boundaries

One module's targeted fixes plus its manifest claims — the wave-generation core (`gyroid_f`
at `lib.rs:394`, `make_one_period` at `lib.rs:430`, `make_wave` at `lib.rs:491`, orientation
choice, constants) is verified correct and stays untouched. Clipping, short-filtering, and
chaining leave the module (the 133 linker owns them; TASK-258 currently OPEN, but the
module ships standalone). Default fill-holder config keeps solid roles on rectilinear — the
multi-role claims are opt-in capability, not a default change (the default is `rectilinear-infill`
across all four fill-holder keys; gyroid is not referenced in `resolved_config.rs` defaults).

## Prerequisites and Blockers

- Depends on: `131_per-region-config-delivery` (TASK-256, closed 2026-07-19 — per-region
  density is readable via the SDK region accessor in the region loop).
  `133_infill-linker-module` (TASK-258, currently OPEN) is nominally upstream — when it
  lands, raw waves are clipped + linked downstream; until then output is raw waves (deliberate
  per ADR-0025). This packet ships regardless.
- WIT contract (`run-infill-postprocess` with `prior-infill: list<prior-infill-region>`)
  is in place per ADR-0028 (TASK-255 closed 2026-07-17). `clip_polylines` in
  `slicer-core::polygon_ops` is in place per ADR-0026 (TASK-254 closed 2026-07-16).
- Unblocks: `136_infill-parity-integration`.
- Activation blockers: none. The `TASK-258` open state is a known trade-off, not a packet
  blocker.

## Acceptance Criteria

- **AC-1. Given** a 10 mm square sparse polygon at z = 0.2 mm, **when** the module runs,
  **then** it emits raw wave polylines in world space with NO clipping applied — every
  polyline is a continuous wave (point count > 2), and emitted points may extend beyond the
  polygon but stay within the expanded generation bbox (expand = 10 × spacing). The existing
  `square_region_produces_paths` and `paths_at_correct_z` tests cover convex-square wave
  emission at z=0 and at multiple z values; this AC is the no-clipping regression pin
  (the existing tests may currently rely on the per-vertex clipper returning truncated
  polylines — if so, the new test will go RED until the deletion lands). | `cargo test -p gyroid-infill -- square_10mm_z_0p2_emits_raw_waves 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the same polygon at `infill_angle = 45°` and `0°`, **when** the 45° output
  is rotated by −45°, **then** it matches the 0° output within 2 units per point (regression
  pin for rotate-polygon-FIRST: rotate ExPolygon by −(base + correction), generate
  axis-aligned waves in the rotated bbox, rotate points back). | `cargo test -p gyroid-infill -- rotated_square_45_matches_unrotated_after_inverse 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** wave generation for any layer, **when** the generation bbox is computed,
  **then** `bb.min` is snapped to a multiple of `2π × scale_factor` (`align_to_grid`,
  FillGyroid.cpp:322) so adjacent layers' waves are phase-coherent. The new
  `align_to_grid` helper at `lib.rs` is asserted to exist and produce that snapping. |
  `cargo test -p gyroid-infill -- align_to_grid_snaps_bbox_min 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** the generation bbox expansion, **when** computed, **then**
  `expand == 10.0 × spacing_mm` (FillGyroid.cpp:326; replaces the current
  `4.0 * spacing_mm` at `lib.rs:259`). | `cargo test -p gyroid-infill -- expand_factor_is_10x_spacing 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** the manifest, **when** grepped, **then** `claims.holds` contains all four:
  `claim:sparse-fill`, `claim:top-fill`, `claim:bottom-fill`, `claim:bridge-fill`
  (ADR-0027). | `rg -c 'claim:(sparse|top|bottom|bridge)-fill' modules/core-modules/gyroid-infill/gyroid-infill.toml | grep -q '^4$' && echo CLAIMS-OK`
- **AC-6. Given** the module source, **when** grepped, **then**
  `clip_polyline_to_expolygon` (lib.rs:611), `point_in_expolygon` (lib.rs:570),
  `point_in_polygon` (lib.rs:585), and `polygon_bbox_mm` (lib.rs:551) are deleted
  (zero definitions), and the wave core (`gyroid_f`, `make_one_period`, `make_wave`) plus
  the existing `asin_nan_protection` test remain. | `rg -c 'fn (clip_polyline_to_expolygon|point_in_expolygon|point_in_polygon|polygon_bbox_mm)' modules/core-modules/gyroid-infill/src/lib.rs | grep -q '^0$' && rg -q 'fn gyroid_f' modules/core-modules/gyroid-infill/src/lib.rs && echo DELETED-OK`

## Negative Test Cases

- **AC-N1. Given** default fill-holder config (all four fill-holder keys resolve to
  `rectilinear-infill`; gyroid has no default in `resolved_config.rs`), **when** a region
  dispatches, **then** gyroid's held set now contains the four fill claims (after AC-5) but
  the dispatch only fires gyroid when a user explicitly sets the fill-holder key to
  `gyroid-infill`; the existing `square_region_produces_paths` test continues to exercise
  the sparse path and the new test asserts that even with the four declared claims, the
  default config does not route top/bottom/bridge to gyroid (DEV-082 opt-in guard). |
  `cargo test -p gyroid-infill -- default_holders_gyroid_sparse_only 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p gyroid-infill 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy -p gyroid-infill --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (rebuild if STALE; the manifest edit feeds the guest
  build)
- `cargo check --workspace --all-targets` (no downstream breakage; the WIT contract is
  already in place per ADR-0028)
- the AC-5/AC-6 rg one-liners — structural claims + deletion checks

## Authoritative Docs

- `docs/adr/0027-gyroid-multi-role-fill-holder.md` — binding; full read (short).
- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 3 — the stays/deleted lists
  are binding; load Phase 3 only. Phase 0 (`clip_polylines`) and Phase 1 (WIT contract) are
  already realized (TASK-254 + TASK-255 closed).
- `docs/DEVIATION_LOG.md` DEV-082 — the recorded divergence this packet realizes.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:300-376` — `_fill_surface_single`: the rotate-polygon-first ordering being ported (rotate ExPolygon → rotated bbox → axis-aligned waves → rotate points back).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:322` — `align_to_grid` call and its grid constant.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp:326` — the 10× expand factor.

## Doc Impact Statement (Required)

**`none`** — the multi-role divergence and its opt-in semantics are already documented
(ADR-0027 + DEV-082, landed in commit cddc9f76); no IR/WIT/scheduler/SDK contract changes;
the manifest claim addition realizes exactly what those docs describe.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
