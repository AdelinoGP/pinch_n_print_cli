# Requirements: 111_arachne-beading-strategy-stack

## Packet Metadata

- Grouped task IDs:
  - `T-210` — Define `BeadingStrategy` trait in `slicer_core::beading` covering all 5 strategies' surface.
  - `T-211` — Port `DistributedBeadingStrategy` (Gaussian-weighted width distribution).
  - `T-212` — Port `RedistributeBeadingStrategy` (preserve outer-wall width consistency — decorator).
  - `T-213` — Port `WideningBeadingStrategy` (thin-feature single-wall regime — decorator).
  - `T-214` — Port `OuterWallInsetBeadingStrategy` (outer-wall toolpath offset — decorator).
  - `T-215` — Port `LimitedBeadingStrategy` (max-bead-count cap; 0-width sentinel insertion). Sentinels stay internal — see T-215b for strip-pass.
  - `T-215b` — Implement strip-pass: drop zero-width beads from BeadingStrategy output before `WallLoop` assembly per the D-9 decision in the roadmap. Register the implementation rationale as a new `D-111-ARACHNE-SENTINEL-STRIP` entry in `docs/DEVIATION_LOG.md` (D-9 is a roadmap ID, not a log ID; the log entry uses the `D-<pkt>-<SLUG>` convention).
  - `T-216` — Port `BeadingStrategyFactory` stack composition (`Distributed → Redistribute → Widening → OuterWallInset → Limited`).
  - `T-218` — Register all 11 Arachne `m_params.*` config keys in `docs/15_config_keys_reference.md` and in `arachne-perimeters.toml`.
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's Arachne wall generator selects per-segment bead widths through a stack of `BeadingStrategy` decorators. Each decorator transforms the `Beading` produced by the inner strategy: `Distributed` is the base (Gaussian-weighted thickness distribution), `Redistribute` preserves outer-wall width consistency, `Widening` handles features below `min_input_width` as single thin walls, `OuterWallInset` shifts the outer toolpath inward by a configured offset, and `Limited` caps total bead count and inserts zero-width sentinel beads at the cap boundary (an internal data invariant the propagation pass uses). Without all 5 strategies AND the canonical wrapping order from `BeadingStrategyFactory::create_strategy`, the centrality + bead-count passes in P112 (T-220..T-222) cannot produce wall widths that match OrcaSlicer.

The zero-width sentinels from `Limited` are an internal book-keeping mechanism: downstream centrality propagation reads them to keep bead-index alignment, but the wall-loop output should never carry zero-width entries. P96 originally surfaced this as D-9 (Arachne zero-width-sentinel handling) with two options: (a) coordinate with infill modules to recognize and skip sentinels, (b) strip sentinels before external output. D-9 closed via option (b) — T-215b implements the strip-pass at `LimitedBeadingStrategy::compute_and_strip`, and the deviation closure entry records the rationale.

T-218 registers the 11 `m_params.*` config keys both in `docs/15_config_keys_reference.md` (descriptions + defaults + units) and in `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (manifest schema blocks). The `arachne-perimeters` manifest (`arachne-perimeters.toml`) already exists and carries 5 keys (`wall_count`, `line_width`, `outer_wall_speed`, `inner_wall_speed`, `perimeter_arc_tolerance`). The 11 new keys are disjoint from those 5 — no collision. The module's 512-line `run_perimeters` impl is NOT a stub and is NOT modified; only the manifest receives the 11 new schema blocks. Their values are passed into `BeadingStrategyFactory::create_stack` at P112's T-230 wire-up time.

This is a pure-data packet — no IR changes, no WIT changes, no host changes. Every test runs as a `slicer-core` unit test against recorded OrcaSlicer reference outputs.

## In Scope

- `crates/slicer-core/src/beading/mod.rs` (NEW) — `BeadingStrategy` trait + `Beading` struct + re-exports.
- `crates/slicer-core/src/beading/distributed.rs` (NEW) — `DistributedBeadingStrategy`.
- `crates/slicer-core/src/beading/redistribute.rs` (NEW) — `RedistributeBeadingStrategy`.
- `crates/slicer-core/src/beading/widening.rs` (NEW) — `WideningBeadingStrategy`.
- `crates/slicer-core/src/beading/outer_wall_inset.rs` (NEW) — `OuterWallInsetBeadingStrategy`.
- `crates/slicer-core/src/beading/limited.rs` (NEW) — `LimitedBeadingStrategy` + `compute_and_strip` (T-215 + T-215b).
- `crates/slicer-core/src/beading/factory.rs` (NEW) — `BeadingStrategyFactory`.
- `crates/slicer-core/src/lib.rs` (EDIT) — `pub mod beading;`.
- `crates/slicer-core/tests/beading/*.rs` (NEW) — unit suites per strategy + factory composition + strip-pass. Each file requires a `[[test]]` entry in `crates/slicer-core/Cargo.toml`.
- `crates/slicer-core/Cargo.toml` (EDIT) — add 6 `[[test]]` entries (one per test file in Steps 2–7).
- `crates/slicer-core/tests/fixtures/beading/` (NEW) — recorded OrcaSlicer reference Beading outputs in JSON.
- `docs/15_config_keys_reference.md` (EDIT) — 11 new key entries (no collision with existing 5 arachne-perimeters keys).
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (EDIT) — 11 new `[config.schema.*]` blocks (5 existing keys unchanged).
- `docs/01_system_architecture.md` (EDIT) — register `beading` sub-module.
- `docs/DEVIATION_LOG.md` (EDIT) — add `D-111-ARACHNE-SENTINEL-STRIP` entry (not `D-9` — D-9 is a roadmap ID).
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` (EDIT) — flip T-210..T-218 to DONE.

## Out of Scope

- SkeletalTrapezoidationGraph — FORWARD-DEP on draft P110 (`crates/slicer-core/src/skeletal_trapezoidation/`); not consumed by this packet (pure-data stack, no SKT graph dependency).
- Centrality filtering and bead-count assignment (P112 / T-220, T-221) — these will read the trait/factory built here but live in a separate sub-module.
- Wire-up of `BeadingStrategyFactory::create_stack` into `arachne-perimeters::run_perimeters` (P112 / T-230).
- `ExtrusionLine` / `ExtrusionJunction` IR types (P112 / T-224) — this packet produces `Beading` data; IR conversion is downstream.
- Real `arachne-perimeters` run_perimeters logic — the 512-line working impl in `modules/core-modules/arachne-perimeters/src/lib.rs` is NOT a stub; this packet adds 11 config schema keys to the manifest only, not to the run path.
- Non-Arachne config keys.
- M1 packets.

## Forward Dependencies (explicit — S1/S5)

These symbols do NOT exist in the tree; they are produced by still-draft packets. Do NOT read or import them — use inline equivalents where needed.

| Symbol | Producing packet | Status | Action if needed |
| --- | --- | --- | --- |
| `crates/slicer-core/src/voronoi.rs` (`voronoi_from_segments`, `HalfEdgeGraph`, `VoronoiError`, `Segment`) | P110 | `draft` | Not consumed by P111; reference only for pattern |
| `crates/slicer-core/src/skeletal_trapezoidation/` (`SkeletalTrapezoidationGraph`) | P110 | `draft` | Not consumed by P111 |
| `crates/slicer-core/src/arachne/preprocess.rs` (`preprocess_input_outline`) | P110 | `draft` | Not consumed by P111 |
| `crates/slicer-core/src/flow.rs` (`to_slicer_units`) | P105 | `draft` | Not consumed — use inline `/100` division per `docs/08_coordinate_system.md` |
| `crates/slicer-core/tests/voronoi_stress.rs`, `skt_graph_golden.rs`, `preprocess_golden.rs` | P110 | `draft` | Not pre-existing; test pattern described only for reference |

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~400 lines | Range-read Phase 11 rows (T-210..T-218). |
| `docs/15_config_keys_reference.md` | varies | Read existing entry format; range-read 50 lines for shape. |
| `docs/03_wit_and_manifest.md` | ~600 lines | Range-read `[config.schema.*]` block format. |
| `docs/01_system_architecture.md` | ~150 lines | Read full — where new sub-module lands. |
| `crates/slicer-core/src/lib.rs` | small | Read full — extend `pub mod` set. |
| `crates/slicer-core/Cargo.toml` | small | Read for existing `[[test]]` entry format before adding 6 new entries. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet — ONE dispatch per file:

| File | Dispatch | Return ≤ |
| --- | --- | --- |
| `Arachne/BeadingStrategy/BeadingStrategy.h` | LOCATIONS | 10 entries — virtual method signatures + `Beading` struct fields |
| `Arachne/BeadingStrategy/DistributedBeadingStrategy.cpp` | SUMMARY | 150 words — `compute` algorithm + Gaussian decay constant |
| `Arachne/BeadingStrategy/RedistributeBeadingStrategy.cpp` | SUMMARY | 100 words — outer preservation rule |
| `Arachne/BeadingStrategy/WideningBeadingStrategy.cpp` | SUMMARY | 100 words — sub-min_input_width handling |
| `Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp` | SUMMARY | 100 words — offset application |
| `Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp` | SUMMARY | 150 words — cap + sentinel insertion rule |
| `Arachne/BeadingStrategy/BeadingStrategyFactory.cpp` | LOCATIONS | 10 entries — `create_strategy` body wrapping order |
| `PrintConfig.cpp` for 11 `m_params.*` defaults | LOCATIONS | 20 entries — defaults + units + descriptions |

Recorded OrcaSlicer reference Beading outputs (for the 10-thickness Distributed table, the outer-consistent Redistribute fixture, etc.) live as committed JSON files under `crates/slicer-core/tests/fixtures/beading/`. The implementer authors them once during Step 2's RED phase by running the OrcaSlicer reference (off-tree, manually recorded) — they are NOT regenerated at test time.

## Acceptance Summary

- Positive cases: `AC-1` (trait surface + `Beading` struct), `AC-2` (Distributed 10-thickness table), `AC-3` (Redistribute outer-consistent), `AC-4` (Widening sub-min_input_width), `AC-5` (OuterWallInset offset-outer-only), `AC-6` (Limited sentinel insertion at cap), `AC-7` (T-215b `compute_and_strip` returns no zero-width), `AC-8` (Factory stack composition order + Orca reference match), `AC-9` (11 config keys registered in docs + manifest).
- Negative cases: `AC-N1` (invariant: `toolpath_locations.len() == bead_widths.len()`), `AC-N2` (raw `compute` retains sentinels — only `compute_and_strip` drops them).
- Refinements not captured in Given/When/Then:
  - The 5 strategies follow the OrcaSlicer decorator pattern. Idiomatic Rust: each decorator owns a `Box<dyn BeadingStrategy>` inner. The trait MUST be object-safe (no generics on methods, no `Self::` returns).
  - `Beading.left_over: f64` is the residual thickness Orca tracks for transition placement. Keep the field even though no test asserts on it directly — P112 uses it.
  - Config keys' defaults: `min_feature_size = 25` (units), `min_bead_width = 200`, `wall_transition_filter_deviation = 200`, `wall_transition_length = 4000`, `wall_transition_angle = 10` (degrees), `wall_distribution_count = 1`, `min_length_factor = 0.5`, `initial_layer_min_bead_width = 850`, `outer_wall_offset = 0`, `max_bead_count = 9`, `optimal_width = 4000` — these are translation-of-Orca-defaults through the /100 rule but the implementer MUST confirm each via the PrintConfig.cpp LOCATIONS dispatch and document any divergence in closure-log.md.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo test -p slicer-core beading::trait_surface 2>&1 \| tee target/test-output.log` | AC-1 | FACT pass/fail |
| `cargo test -p slicer-core distributed_beading_strategy_orca_table 2>&1 \| tee target/test-output.log` | AC-2 | FACT pass/fail per thickness |
| `cargo test -p slicer-core redistribute_outer_consistent 2>&1 \| tee target/test-output.log` | AC-3 | FACT pass/fail |
| `cargo test -p slicer-core widening_below_min_input_width 2>&1 \| tee target/test-output.log` | AC-4 | FACT pass/fail |
| `cargo test -p slicer-core outer_wall_inset_offset_outer_only 2>&1 \| tee target/test-output.log` | AC-5 | FACT pass/fail |
| `cargo test -p slicer-core limited_inserts_sentinels_at_cap 2>&1 \| tee target/test-output.log` | AC-6 | FACT pass/fail |
| `cargo test -p slicer-core limited_compute_and_strip_no_zero_widths 2>&1 \| tee target/test-output.log` | AC-7 | FACT pass/fail |
| `cargo test -p slicer-core limited_raw_compute_retains_sentinels 2>&1 \| tee target/test-output.log` | AC-N2 | FACT pass/fail |
| `cargo test -p slicer-core factory_stack_composition_order 2>&1 \| tee target/test-output.log` | AC-8 (composition order) | FACT pass/fail |
| `cargo test -p slicer-core factory_matches_orca_reference 2>&1 \| tee target/test-output.log` | AC-8 (Orca match) | FACT pass/fail |
| `cargo test -p slicer-core beading_invariant_locations_len_eq_widths_len 2>&1 \| tee target/test-output.log` | AC-N1 | FACT pass/fail |
| AC-9 shell loop from `packet.spec.md` (11 keys against both `docs/15_config_keys_reference.md` and `arachne-perimeters.toml`) | AC-9 | FACT pass per key |

## Step Completion Expectations

- Cross-step invariant: each strategy's tests must go GREEN before adding it to the factory composition (Step 7). The factory test (AC-8) MUST fail before Step 7 starts and pass after.
- Step ordering rationale: trait (Step 1) → Distributed base (Step 2) → 4 decorators (Steps 3–6, can be parallel but plan as serial to keep context clean) → factory + composition test (Step 7) → docs + config-key registration (Step 8). Each decorator depends on the trait but NOT on the other decorators; serializing is for context hygiene, not data dependency.
- Shared scratch state: golden JSON files under `crates/slicer-core/tests/fixtures/beading/` are recorded once during Step 2 (Distributed 10-thickness table) and Steps 3–6 (per-strategy fixtures). Step 7's factory test uses a multi-stage fixture composed from the per-stage fixtures via a JSON pre-merge — the implementer records it ONCE and never regenerates.
- `D-111-ARACHNE-SENTINEL-STRIP` entry in `docs/DEVIATION_LOG.md` (Step 8) should cite the WallLoop invariant as the rationale. No ADR is required — the D-9 roadmap entry already records the decision; the log entry documents the implementation-level detail.

## Context Discipline Notes

- This packet has 8 steps. The largest are Step 2 (Distributed Gaussian math + 10-thickness golden) and Step 7 (factory composition + multi-stage golden).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/` contains 6 files, each ~200–500 LOC. Delegate ONE dispatch per file per the OrcaSlicer Reference Obligations table; do NOT direct-read any of them.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` is ~10000 LOC — direct-reading would burn the entire budget. The LOCATIONS dispatch for the 11 `m_params.*` defaults caps at 20 entries (~2 per key).
- Likely temptation: re-read OrcaSlicer's strategy implementations to disambiguate edge cases. **Use the SUMMARY dispatch** + the recorded golden fixtures — the goldens are the source of truth for parity; if a strategy's implementation can't make the golden green, re-dispatch a tighter SUMMARY for that strategy's edge cases.
- Sub-agent return-format for the heaviest dispatch: `Distributed.cpp` SUMMARY must be ≤ 150 words. If it returns > 200, re-dispatch tighter focused on the `compute` body alone.
