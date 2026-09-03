# Requirements: 241-support-agg-rasterizer

## Packet Metadata

- Grouped task IDs: `TASK-419`..`TASK-428`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` §12 brief
  "241-support-agg-rasterizer". TASK-419..TASK-428 are reserved for this packet by row #8 of
  that file's trailing `## Packet Queue` table (NOT by the §12 brief, and NOT by the §11
  "Packet queue" table — §11 carries no TASK column at all). Absorbs register row G-07
  (`docs/specs/support-parity-gap-register.md`). The
  stub `docs/spec_packets/stubs/stub-support-agg-rasterizer.md` no longer exists — it was
  deleted when the G-07 premise was corrected (the register records "stub deleted"), so
  there is nothing left to absorb and no `docs/spec_packets/stubs/` directory to touch.
- Packet status: `draft`
- Depends on: `238c-support-renderer-flow-interfaces` — SATISFIED (verified 2026-09-03: its
  `packet.spec.md` frontmatter reads `status: implemented`, as do 236, 238a and 238b). No
  forward dependency blocks activation. Ledger fact — re-derive at activation.
- Aggregate context cost: `M` (per-step roll-up in `implementation-plan.md`; no step rated L)

## Problem Statement

The gap register's G-07 row was filed with a "needs-research first" premise: that the canonical
`SupportGridPattern` AGG rasterizer changes support outline shape but not termination, coverage,
or collision freedom. **Ruling 7 of the governing plan refuted that premise** with upstream
history: `fb7b995050` reworked grid projection onto the AGG rasterizer precisely to stop supports
leaking through or around object walls (a collision-freedom defect), via ≤8×8 oversampling plus
expansion restricted inside the cell; `a95607d7bf` fixed support columns missing abruptly when
going down (a coverage/termination defect) caused by grid-extraction contour filtering. The
research question is settled — this packet is a PORT.

PnP's traditional planner (`modules/core-modules/traditional-support-planner/src/lib.rs`,
1274 lines as of 2026-09-03, port of the `SupportMaterial.cpp` orchestration) implements
only the *semantic* half:
propagate-without-growth carry, trimmed per layer at `support_object_xy_distance`. It has no
byte-grid projection, no oversampling, no in-cell expansion restriction, no seed fill, and no
contour extraction — so it reproduces neither fix. This packet ports the rasterizer as a
Ruling-8 knob: `support_area_rasterizer = agg` (canonical) default, `legacy_semantic` selectable;
both paths tested; parity evidence runs the default.

## In Scope

- New guest-side rasterizer module
  `modules/core-modules/traditional-support-planner/src/agg_raster.rs`: byte-grid construction
  (oversampled ≤8×8, macro blocks, boundary ring), polygon→grid rasterization (AGG gray8
  scanline semantics on PnP scaled-integer coordinates), trimming-mask dilation (3×3),
  macro-cell seed fill (4-direction propagation steps), contour extraction (marching-squares-
  equivalent line chaining, `fill_holes`, `offset_in_grid`), island sample-containment filter.
- Manifest knob `[config.schema.support_area_rasterizer]` (`enum`,
  `values = ["agg", "legacy_semantic"]`, `default = "agg"`) in
  `traditional-support-planner.toml`; module config parsing + rejection of out-of-vocabulary
  values at the module boundary.
- Rewiring of `plan_candidate`'s propagation loop to consume the rasterizer when `agg` is
  selected, preserving termination bookkeeping (structured `NoRoute` decline), interface
  anchoring, bottom-contact derivation inputs, and demand/body ID threading.
- Measurement harness (integration tests): pre-port baseline capture, post-port wall-leakage
  (penetration events + penetrated area vs occupancy grown by `support_object_xy_distance`) and
  column-continuity (abrupt column drops, total-area drift guard) metrics per AC-6/AC-7/AC-8.
- New test SUBMODULE `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`
  — not a new test binary. It joins the existing aggregated `integration` binary and MUST be
  registered with a `mod support_agg_rasterizer_tdd;` line in
  `crates/slicer-runtime/tests/integration/main.rs` (which currently aggregates 70 modules).
  Without that line the file never compiles and `cargo test --test integration <name>`
  reports "0 tests run" — a false pass. Separately, a new guest test file
  `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs` gets its own
  `[[test]]` stanza in the crate's `Cargo.toml`, matching the existing explicit
  `[[test]] name = "traditional_family_tdd" / path = "tests/traditional_family_tdd.rs"` stanza.
  Note this is a CONVENTION choice, not a compilation requirement: the workspace is edition
  2021 (`Cargo.toml` `[workspace.package] edition = "2021"`) and the crate sets no
  `autotests = false`, so target autodiscovery remains ON and the file would be picked up
  even without the stanza. Declaring it explicitly keeps the crate's two test targets
  symmetric and keeps the `--test agg_rasterizer_tdd` name pinned rather than inferred.
  (Do not carry the older, false rationale that an explicit `[[test]]` stanza disables
  autodiscovery — it does not.)
- Doc impact items listed in `packet.spec.md` §Doc Impact Statement (config-key reference regen;
  TASK registration).

## Out of Scope

- Tree-family rendering/planner surfaces — owned by 238b (done there). Canonical maps tree
  styles to `smsGrid` inside `SupportGridPattern`, but this packet wires the knob ONLY into the
  traditional planner's area propagation; extending it elsewhere is not this slice.
- Renderer flow/density/interface semantics (G-10/G-11/G-12/G-13/G-18, base-interface role,
  regularize consolidation) — owned by 238c; consumed as its output state.
- Raft geometry (`RaftPlan` consumer, raft keys, signed negative layers) — owned by
  `240a-support-raft-substrate` / `240b-support-raft-module` (packet 240 was split; there is no
  bare `240-support-raft` directory).
- Independent support-layer Z (`is_same_z_entity` filter, off-grid entities) — owned by 239.
- The EdgeGrid/SDF branch (`SUPPORT_USE_AGG_RASTERIZER` compiled-out path) — canonical itself
  does not use it; we port the active AGG path only, no dual implementation behind a cfg.
- Orca toolpath identity: behavioral parity measured by wall-leak/column-continuity deltas and
  block counts, not byte-equal G-code (plan §15).
- `docs/DEVIATION_LOG.md` edits and `docs/07` queue-table edits beyond the TASK registration
  gate step.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief, §3 Rulings 7/8, §6
  invariant 16, §7 E1–E9, §8 human gate, §13 traps T1/T4/T5/T7. Ranged reads (~755 lines).
- `docs/specs/support-parity-gap-register.md` - G-07 row (corrected premise); direct range read.
- `docs/19_visual_debug.md` - bundle contract for human-gate taps; ranged read.
- `docs/15_config_keys_reference.md` - regenerated output target; consult table format around
  the existing `support_*` rows only.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — class `SupportGridPattern` constructor (`smsGrid` branch: oversampling formula `std::clamp(int(scale_(m_support_spacing) / (extrusion_width_scaled + 100)), 1, 8)`, `m_pixel_size = max(extrusion_width_scaled + 21, scale_(m_support_spacing / oversampling))`, bbox grid alignment + one-pixel offset, macro-block arithmetic, `rasterize_polygons` for support and trimming polygons, `seed_fill_block(m_grid2, …, dilate_trimming_region(…))`); static `rasterize_polygons` (gray8 scanline even-odd fill — the semantics replicated on PnP coordinates); static `contours_simplified` (boundary-edge collection, lexicographic chaining, `fill_holes` left/right+top/bottom rule, `assert(abs(2*offset) < pixel_size - 10)` in-cell bound); `extract_support` (trimming difference → islands, `island_samples` containment filter, expanding-vs-shrinking sample set choice by `offset_in_grid` sign); statics `dilate_trimming_region` (3×3 all-set mask) and `seed_fill_block` (top-down/bottom-up/left/right propagation steps gated by the dilated mask).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportGridParams` (`grid_resolution`, `expansion_to_propagate` vs `expansion_to_slice` distinction, `extrusion_width`, `support_closing_radius`, `support_angle`, style) and `SupportMaterialStyle` enum mapping (`smsDefault`→`smsGrid`; tree styles coerced to `smsGrid`).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1`..`AC-8` (knob declaration; canonical grid formulas; extraction + island
  filtering; in-cell restriction; default routing; wall-leakage measurement; column-continuity
  measurement; both-modes divergence).
- Negative: `AC-N1` (invalid knob value rejected at module boundary, never defaulted);
  `AC-N2` (explicit `legacy_semantic` keeps every existing planner behavior green).
- Cross-packet impact: confined to `modules/core-modules/traditional-support-planner/**`, the
  two doc files above, and the runtime test surface
  (`crates/slicer-runtime/tests/integration/{main.rs,support_agg_rasterizer_tdd.rs}` plus the
  new `crates/slicer-runtime/tests/fixtures/golden/` directory, which does not yet exist and
  must be created). No production `crates/**` code changes. 238c / 239 / 240a / 240b packets' directories
  are untouched. The knob adds one
  key to the shared config surface — no WIT, IR, or schema-version change.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -ge 6 && echo PASS` | Full guest rasterizer suite (AC-2..AC-5, AC-N1) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS` | Legacy-mode regression guard (AC-N2) | FACT pass/fail |
| `( cargo test -p slicer-runtime --test integration -- agg_wall_leakage_measurement_beats_baseline --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- agg_column_continuity_measurement_beats_baseline --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- agg_and_legacy_modes_both_function_and_diverge --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && echo PASS` | Measurement-as-gate trio on real fixture slices (AC-6..AC-8); three exact commands chained with `&&` — Cargo accepts one TESTNAME per invocation | FACT pass/fail + recorded metric numbers |
| `cargo xtask build-guests --check && echo FRESH` | Guest freshness gate (E4/T4) before any attribution | exit code 0 + FRESH |
| `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo xtask check-literals` | Closure gates | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step 1 (baseline) MUST land before any behavior change; Steps 2+ build on a recorded,
  committed baseline artifact. Reversing the order invalidates AC-6/AC-7 comparisons.
- Every step touching `modules/core-modules/traditional-support-planner/**` runs
  `cargo xtask build-guests --check` before attributing any failure (T4/E4); rebuild without
  `--check` if stale.
- Metric numbers quoted anywhere in docs/tests must come from a logged run
  (`target/test-output.log` or the baseline artifact), never estimated (No Unverified Metrics).
- The knob's default flips ON in the same step that rewires propagation (no intermediate commit
  where the code exists but nothing routes through it).

## Context Discipline Notes

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` is ~3.3k lines — delegate
  everything; ranged reads only via LOCATIONS/SUMMARY returns (T1: verify existence by direct
  listing, globs miss gitignored paths).
- `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` is **2466
  lines** with **28** `#[test]` functions (verified 2026-09-03; AC-N2's `-gt 10` guard is
  therefore satisfiable). Ranged reads only — helpers at the top, then targeted tests.
- Do NOT load `target/`, golden fixture bodies, or generated WASM bindings.
