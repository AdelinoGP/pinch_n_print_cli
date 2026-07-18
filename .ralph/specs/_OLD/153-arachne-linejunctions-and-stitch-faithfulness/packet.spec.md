---
status: implemented
packet: 153-arachne-linejunctions-and-stitch-faithfulness
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
implemented_on: 2026-07-13
---

# Packet Contract: 153-arachne-linejunctions-and-stitch-faithfulness

## Goal

Reduce the two remaining PnP-vs-OrcaSlicer Arachne divergences that ADR-0035's "faithful algorithm-level port" bar does not yet cover: (1) `EdgeJunctions` storage is a PnP-internal `(from_junctions, to_junctions)` split with `perimeter_index`-slot indexing + `default_extrusion_junction()` placeholders, instead of OrcaSlicer's single `LineJunctions` `Vec<ExtrusionJunction>` per edge ordered peak-side to boundary-side; (2) `stitch_extrusions` lacks OrcaSlicer's `canReverse` (even-line reversal blocking) and the `chain_length + dist < 3 * max_stitch_distance` tiny-polygon non-closure rule.

## Scope Boundaries

This packet is a host-side faithfulness refactor, not a bug fix — the `arachne_annulus_split` test passes today (inset 0 = 1 closed outer loop, 165 junctions) and the N1–N13 chain closed in 2026-07-08 (D-147-CHAIN-CLOSURE). The refactor brings the two divergent functions closer to their canonical implementations so future maintainers and parity audits don't have to reason about PnP-internal storage and merge conventions. Full in/out-of-scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: P141 (A1), P142 (A2), P143 (B), P144 (C), P145 (D), P146 (E), P147 (F) — all `status: implemented`.
- Unblocks: any future Arachne parity audit or re-baseline; ADR-0035's bar is then fully met at this surface.
- Activation blockers: D-153-ARACHNE-PERIMETER-PARITY-STALE-GOLDENS (closed) and D-147-CHAIN-CLOSURE (closed) must remain `closed`; no open packet may depend on the divergent `EdgeJunctions` storage or the un-`canReverse`d stitcher.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** `EdgeJunctions` is restructured to `Vec<ExtrusionJunction>` per edge (OrcaSlicer `LineJunctions` layout, peak-side to boundary-side), **when** `cargo test -p slicer-core --features host-algos --test arachne_junction_upward_half_edge_only -- --nocapture` runs, **then** the 3 tests in that file pass with destructuring updated from `(from_junctions, to_junctions)` to single `Vec`. | `cargo test -p slicer-core --features host-algos --test arachne_junction_upward_half_edge_only -- --nocapture 2>&1 | tee target/test-output-153-ac1.log`
- **AC-2. Given** the storage restructure, **when** `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression -- --nocapture` runs, **then** all 3 tests pass with destructuring updated; emitted junction widths remain within 0.01 mm of pre-refactor values for the 6 canonical fixtures. | `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression -- --nocapture 2>&1 | tee target/test-output-153-ac2.log`
- **AC-3. Given** the storage restructure + `default_extrusion_junction()` removal, **when** `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture` runs, **then** the test passes and `inset0: lines=1 closed=1 sizes=[45]` is the recorded output (no regression from the pre-refactor annulus behavior). | `cargo test -p slicer-core --features host-algos --test arachne_annulus_split -- --nocapture 2>&1 | tee target/test-output-153-ac3.log`
- **AC-4. Given** the storage restructure, **when** `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- --nocapture` runs, **then** `generate_toolpaths_tapered_wedge` passes (after re-recording `tests/fixtures/arachne/toolpaths_tapered_wedge.json` if the per-bead line counts shift due to junction-count changes) and `outer_wall_closes_for_simple_polygon` passes. | `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- --nocapture 2>&1 | tee target/test-output-153-ac4.log`
- **AC-5. Given** the N1–N4 red test suite is regression-locked, **when** `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --test arachne_parity_red_chain_junctions --no-fail-fast` runs, **then** all tests pass (no regression from the storage restructure or the stitch faithfulness fixes). | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --test arachne_parity_red_chain_junctions --no-fail-fast 2>&1 | tee target/test-output-153-ac5.log`
- **AC-6. Given** the `canReverse` stitch fix (even lines must not flip direction), **when** a unit test feeds two even (`is_odd = false`) `ExtrusionLine`s whose endpoints are within `max_gap` and whose only valid join is a reversal, **then** `stitch_extrusions` leaves them unjoined (no merge). | `cargo test -p slicer-core --test arachne_stitch_can_reverse -- --nocapture 2>&1 | tee target/test-output-153-ac6.log`
- **AC-7. Given** the tiny-polygon non-closure rule, **when** a unit test feeds one even `ExtrusionLine` whose total polyline length + closing-segment distance is `< 3 * max_gap`, **then** `stitch_extrusions` returns it as an open line (`is_closed = false`), not a closed loop. | `cargo test -p slicer-core --test arachne_stitch_tiny_polygon -- --nocapture 2>&1 | tee target/test-output-153-ac7.log`
- **AC-8. Given** all eight ACs are green, **when** `cargo xtask test -p slicer-core --features host-algos --summary` runs, **then** the summary reports PASS for the `slicer-core` `host-algos` test suite (no workspace-wide gate is required for a refactor packet). | `cargo xtask test -p slicer-core --features host-algos --summary 2>&1 | tee target/test-output-153-ac8.log`

## Negative Test Cases

- **AC-N1. Given** `EdgeJunctions` is the new `Vec<ExtrusionJunction>` layout, **when** any test in the `arachne_*` integration test directory is dispatched, **then** no test fails to compile due to the destructuring change (all 21 test binaries update consistently or the storage is re-exported under a compatibility alias for the transition window). | `cargo check -p slicer-core --features host-algos --all-targets 2>&1 | tee target/test-output-153-n1.log`
- **AC-N2. Given** `stitch_extrusions` blocks even-line reversal, **when** a unit test feeds two odd (`is_odd = true`) `ExtrusionLine`s whose endpoints are within `max_gap` and whose valid join is a reversal, **then** `stitch_extrusions` joins them (the odd-line reversal is still permitted). | `cargo test -p slicer-core --test arachne_stitch_can_reverse -- --nocapture 2>&1 | tee target/test-output-153-n2.log`
- **AC-N3. Given** `stitch_extrusions` applies the `3 * max_gap` tiny-poly rule, **when** a unit test feeds one even `ExtrusionLine` whose total polyline length + closing-segment distance is `>= 3 * max_gap`, **then** `stitch_extrusions` closes it into a loop (the rule does not over-reject). | `cargo test -p slicer-core --test arachne_stitch_tiny_polygon -- --nocapture 2>&1 | tee target/test-output-153-n3.log`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check -p slicer-core --features host-algos --all-targets`
- `cargo clippy -p slicer-core --features host-algos --all-targets -- -D warnings`
- `cargo xtask test -p slicer-core --features host-algos --summary 2>&1 | tee target/test-output-153-gate.log`

## Authoritative Docs

- `docs/07_implementation_status.md` — read lines 317-328 (the P141-P147 packet rows and M2 closure) to confirm the N1-N13 chain is `status: implemented` and the residual gap is the `#[ignore]`d `cube_4color_arachne_outer_walls_close_end_to_end` gate.
- `docs/DEVIATION_LOG.md` — read entries D-141-JUNCTION-BANDS, D-142-CONNECTJUNCTIONS-EMISSION, D-147-PARITY-AUDIT-FINDINGS, D-147-CHAIN-CLOSURE to confirm what was already implemented and what the current `EdgeJunctions` / `stitch_extrusions` behavior is.
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` — read full; this packet implements two of the functions ADR-0035 lists as requiring faithful ports (`connectJunctions` storage and `stitch_extrusions`).

For each doc, note whether the implementer should load it directly or delegate the read (delegate when the doc is > 300 lines or only one section is needed).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` §"D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS" — `rg -q 'D-153-ARACHNE-LINEJUNCTIONS-AND-STITCH-FAITHFULNESS' docs/DEVIATION_LOG.md`
- `CONTEXT.md` §"Terms" replace "Junction fan" entry with "Edge junctions" entry — `rg -q 'Edge junctions' CONTEXT.md`
- `docs/adr/0035-arachne-faithful-emission-and-transitions.md` §"Consequences" add cross-reference to packet 153 — `rg -q 'packet 153' docs/adr/0035-arachne-faithful-emission-and-transitions.md`

The doc edits must land in the same packet (not deferred to a follow-up); the verification greps are appended to the Acceptance Criteria above and gate packet close. The `spec-review` skill checks this section is non-empty and that every grep returns a hit before a packet may flip to `status: implemented`.

## OrcaSlicer Reference Obligations

When this packet touches parity with OrcaSlicer's Arachne pipeline, implementers must read OrcaSlicer source through the `OrcaSlicerDocumented` reference at `F:/slicerProject/OrcaSlicerDocumented/src/libslic3r/Arachne/`. Do not load OrcaSlicer source directly into the implementer's context; dispatch a focused sub-agent with the exact `file:line` question and a tight return format (FACT pass/fail, or SNIPPETS ≤ 30 lines, or LOCATIONS ≤ 20 entries). The sub-agent's return is the implementer's input — never the OrcaSlicer source itself. Two files govern this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` lines 2013-2079 (`generateJunctions`, the per-edge `LineJunctions` layout) and lines 2198-2235 (`addToolpathSegment`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.cpp` lines 22-47 (`canReverse`, `canConnect`, `isOdd` for `VariableWidthLines` specialization) and `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/PolylineStitcher.hpp` lines 71-247 (the `PolylineStitcher::stitch` algorithm including the `3 * max_stitch_distance` tiny-poly rule at `:136-141`).

## Context Discipline Note

**Read budget for this packet: 80k absolute.** This packet is a refactor that touches two regression-locked functions and 5 test files. The implementer must:

- **Delegate all OrcaSlicer reads.** The two `OrcaSlicerDocumented` files above are out of bounds for direct load; dispatch a sub-agent per question.
- **Range-read the two source files.** `crates/slicer-core/src/arachne/generate_toolpaths.rs` (1261 lines) and `crates/slicer-core/src/arachne/stitch.rs` (249 lines) are the only change surface. Read in ±40-line windows against a stated hypothesis; do not load either in full.
- **No `cargo test --workspace` runs.** The packet's gate is `cargo xtask test -p slicer-core --features host-algos --summary`. Workspace-wide runs are out of scope (they would consume 11+ minutes and are not needed for a refactor of two functions in one crate).
- **No `target/` loads, no lockfile reads, no full `cargo` output dumps.** All test results must be captured to `target/test-output-153-*.log` per the workspace-wide test discipline and read back with `Grep` / `Read` offset, not by re-running the test.
- **If the implementer hits 60k context:** stop, write a handoff block (completed steps, current state, next concrete action, files to reopen), and surface to the user. Do not push past 80k.
