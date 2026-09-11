# Requirements: core-parity

## Packet Metadata

- Grouped task IDs: `core/PARITY` (plan wave/item IDs replace `TASK-###` per the plan's packet-queue exemption)
- Backlog source: `docs/specs/test-quality-remediation-plan.md` §5.1 PARITY row + `Packet Queue` row #14
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

The plan §5.1 PARITY row (`core/PARITY`, disposition FIX — additive; never remove exact pins) names nine parity surfaces in `crates/slicer-core/tests/` whose current assertions are weaker than the canonical contract they protect: the beding side-table radius boundary is exercised only by loose per-vertex contract checks and never at the exact inclusive radius; the transitions perpendicular-foot test only covers horizontal source segments where linear interpolation coincides with the foot; the rib-split test leaves an exact-position assertion as a `let _ =` placeholder; node distances are locked by bbox/non-negativity bounds rather than the F5-fix semantics the builder now implements; the dumbbell topology test asserts only presence (`>= 4` junctions) instead of the dissolve/not-dissolve topology; the limited-beading cap tests have no at-cap (`bead_count == max_bead_count`) case pinning the single centre-sentinel odd shape; junction reachability is asserted via loose ordering/span inequalities while the file documents exact interpolated positions; the arachne module-fallback contract pins only 2 of the 7 wired-key defaults; and the postprocess-order test's "old order" branch is degenerate (2-junction line — simplify is a no-op). This packet adds the missing exact assertions, one file per step, preserving every existing exact pin and touching no production code.

## In Scope

- Test-only additions in exactly nine files under `crates/slicer-core/tests/`, one per step (S1–S9):
  1. `arachne_beding_propagation_side_table.rs` — exact-radius-boundary nearest lookup.
  2. `arachne_construction_apply_transitions_mirror_fix.rs` — diagonal-source `mid_r` + foot-sentinel contract.
  3. `arachne_construction_insert_node_rib_split.rs` — exact split topology counts + cross-twin patch + filler for the placeholder.
  4. `arachne_construction_node_distance_perp_foot.rs` — F5-fix per-node distance oracle.
  5. `arachne_filter_noncentral_regions.rs` — exact dumbbell ring topology (wide-gap not dissolved; narrow-gap dissolved contrast).
  6. `beading/limited.rs` — at-cap single centre-sentinel case (odd-cap boundary; fixture JSON untouched).
  7. `arachne_stitch_chain_junctions_t_to_fix.rs` — exact interpolated junction positions (reachability).
  8. `arachne_pipeline.rs` — complete per-key module-fallback contract (local behavior; no Orca counterpart).
  9. `arachne_postprocess_order.rs` — canonical-order vs old-order divergence fixture.
- Each step's new `#[test]` lives in the same integration-test binary it drives (no shims, no new test binaries), so the pipe-suffixed AC commands drive the asserted behavior directly.
- The plan §7 `core` ledger row update ONLY (Step 10).
- Ambiguity resolutions (committed, both evidence-backed, BLOCKED on neither):
  - Item 6 "odd-cap boundary" = the at-cap boundary branch of `LimitedBeadingStrategy::compute` (`beading/limited.rs`), where the single centre sentinel makes the at-cap total odd — NOT the N4 is-odd line-marking concern (`arachne_beading_is_odd_semantics.rs`), which §5.1 does not name.
  - Item 8 "module fallback" = the local fallback contract mirrored in `arachne_pipeline.rs` (empty `ConfigView` → `ArachneParams::default()` per key; the guest's `arachne_params_from_config` cannot be called from this crate), NOT a WASM module fallback table.

## Out of Scope

- Production behavior, thresholds, or tolerances in `crates/slicer-core/src/**` (no edits; never loosen an exact pin).
- Removal or weakening of any existing test assertion; the sixteen pre-existing test-name anchors in the nine files must all still resolve.
- `docs/specs/test-quality-remediation-census.json` (never touch).
- Other packet directories (`docs/spec_packets/*` other than `core-parity/`), the plan's `Packet Queue`, and every other §5.x crate wave.
- The `sdk host_wrappers_tdd.rs` export from `core-cross` — name only, no dependency.
- Guest WASM surfaces (no module, WIT, or host changes; wasm-staleness considerations deliberately omitted).

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - 767 lines; ranged reads only: §5.1 PARITY row (155-166), §6 patterns (269-289), §7 ledger (291-311), `Packet Queue` (312-352).
- `docs/22_test_quality.md` - delegate; the remediation gate (report mode until ADR-0065).
- `docs/21_data_defaults_and_fixtures.md` - direct range read; FRU/churn rule for new test struct literals of watched types.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Canonical upstream pinned: `https://github.com/OrcaSlicer/OrcaSlicer` at commit `40eab797c6a60a5949c0f92d00798da414c4b44a` (main, 2026-08-04); every path below resolves at that revision. Cite file + function, never line numbers.

Files to inspect for this packet:

- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` — `getNearestBeading` radius-inclusive nearest lookup and `connectJunctions` incident-edge seeding (items 1 and 7); `filterNoncentralRegions` gap-dissolve rule (item 5).
- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.hpp` — `BeadingPropagation` side-table struct (item 1).
- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.hpp` — `getTransitionThickness` (items 2/3) and `getNonlinearThicknesses` (item 3).
- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp` — `compute`'s odd-centre cap-boundary branch (item 6).
- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` — `makeRib` perpendicular foot (item 4).
- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationJoint.hpp` — `distance_to_boundary` semantics (item 4).
- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/slic3r/GUI/PostProcessor.cpp` — `run_post_process_scripts` sequential execution (item 9).
- `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/WallToolPaths.cpp` — `generate`'s post-process call sequence ordering removal before simplification (item 9; the local test header already pins this — preserve it).
- Item 8 (module fallback) has NO Orca counterpart — local fallback behavior; delegated confirmation reads only if needed, never to invent parity.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-10`; only refinements absent from the Given/When/Then text: every AC command tees to `target/test-output.log` and greps the `test result:` line; AC-5 runs two filters in a loop; AC-10 is the sole ledger predicate and accepts any `core` row carrying the packet's content tokens (the row is jointly owned by the remaining core-wave packets, so state may read `partial` with any parenthesized detail).
- Negative: `AC-N1` (silent-green without `--features host-algos`), `AC-N2` (file-scope: new names resolve to exactly one file each and never appear in `src/`; all sixteen pre-existing test-name anchors still resolve).
- Cross-packet impact: none — no exported symbols; `core-cross`'s single exported test name is name-only.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test arachne_beding_propagation_side_table -- --exact get_nearest_beding_includes_vertex_at_exact_radius_boundary` (`2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log`) | AC-1: exact inclusive radius boundary | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_construction_apply_transitions_mirror_fix -- --exact apply_transitions_diagonal_source_mid_r_and_foot_sentinels` (tee+grep as above) | AC-2: diagonal transition mid_r + foot sentinels | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_construction_insert_node_rib_split -- --exact apply_transitions_split_topology_exact_counts_and_cross_twin_patch` (tee+grep) | AC-3: exact split topology + placeholder replacement | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_construction_node_distance_perp_foot -- --exact f5_invariant_node_distances_match_rib_geometry_and_boundary` (tee+grep) | AC-4: F5-fix per-node distance oracle | FACT pass/fail |
| `for t in dumbbell_wide_gap_not_dissolved_pins_exact_ring_topology dumbbell_narrow_gap_dissolves_to_single_closed_ring; do cargo test -p slicer-core --features host-algos --test arachne_filter_noncentral_regions -- --exact "$t" ...; done` | AC-5: dumbbell topology pair | FACT pass/fail per filter |
| `cargo test -p slicer-core --features host-algos --test beading_limited -- --exact limited_inserts_single_centre_sentinel_at_cap_boundary` (tee+grep) | AC-6: odd-cap at-cap sentinel | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_stitch_chain_junctions_t_to_fix -- --exact chain_junctions_land_at_documented_interpolated_positions` (tee+grep) | AC-7: exact junction positions | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- --exact arachne_params_absent_keys_fall_back_to_defaults_per_key` (tee+grep) | AC-8: per-key fallback contract | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order -- --exact canonical_remove_small_first_keeps_line_simplify_first_drops_it` (tee+grep) | AC-9: order-divergence fixture | FACT pass/fail |
| AC-10 python ledger predicate (see `packet.spec.md`) | AC-10: §7 core row contains packet delta | FACT pass/fail |
| `cargo test -p slicer-core --test arachne_stitch_chain_junctions_t_to_fix -- --exact chain_junctions_land_at_documented_interpolated_positions` (tee+grep `0 passed`) | AC-N1: silent-green hazard without the feature | FACT pass/fail |
| AC-N2 python file-scope predicate (see `packet.spec.md`) | AC-N2: new names single-home + never in `src/`; 16 pre-existing anchors resolve | FACT pass/fail |
| `cargo check --workspace --all-targets 2>&1 \| tee target/core-parity-check.log` | acceptance gate: whole tree compiles incl. tests/benches/examples | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/core-parity-clippy.log` | acceptance gate: clippy clean on all targets | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate (FRU/waivers in new test literals) | FACT pass/fail |
| `cargo xtask check-test-quality --report` | test-quality gate in report mode; fix or waive findings in touched test code | FACT pass/fail (report) |

All cargo test invocations in this packet run with `--features host-algos` (feature-correct per the plan §6 patterns); the sole exception is AC-N1, which proves the flag is mandatory. Logs: `target/test-output.log` for the AC filters, dedicated `target/core-parity-*.log` for the gates.

## Step Completion Expectations

Only cross-step invariants, non-obvious ordering, or shared scratch state. Per-step pre/postconditions belong in `implementation-plan.md`.

- None of the nine steps share scratch state or ordering dependencies; each is an independent single-file addition. The ledger step (S10) is last and may only append to the existing `core` row (state → `partial (core-parity)`), never create a second row.
- Every new test must use struct-literal `..` rest forms (FRU) for watched types and carry a `// test-quality:`-shaped claim only where the gate flags it (see `docs/22_test_quality.md` report mode).
- The plan's §7 ledger is the only mutable document; never touch `Packet Queue` or the census.

## Context Discipline Notes

Only packet-specific hazards: large ranged/delegated files, tempting reads to skip, and heavy-dispatch return limits.

- `docs/specs/test-quality-remediation-plan.md` is 767 lines — only §5.1/§6/§7/`Packet Queue` ranges may be read; never the full file.
- The nine touched test files and the builder/propagation source ranges listed in `design.md` are direct reads; everything else (OrcaSlicerDocumented, `docs/22_test_quality.md`) is delegated.
- The plan §5.1 row and §6 pattern block are the authoritative invocation and disposition sources — re-read them at the point of use, never from memory.
