# Task Map: 111_arachne-beading-strategy-stack

Maps packet task IDs (T-210..T-218, T-215b) to their source rows in the roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 11 (T-210..T-218).

## Phase 11 — BeadingStrategy stack

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-210 | Define `BeadingStrategy` trait in `slicer-core::beading` (`compute`, `optimal_bead_count`, `get_transition_thickness`, `optimal_thickness`); define `Beading` struct | Phase 11 | Step 1 | pending |
| T-211 | Port `DistributedBeadingStrategy` (Gaussian-weighted width distribution) | Phase 11 | Step 2 | pending |
| T-212 | Port `RedistributeBeadingStrategy` (preserve outer-wall width consistency — decorator over Distributed) | Phase 11 | Step 3 | pending |
| T-213 | Port `WideningBeadingStrategy` (thin-feature single-wall regime — decorator) | Phase 11 | Step 4 | pending |
| T-214 | Port `OuterWallInsetBeadingStrategy` (outer-wall toolpath offset — decorator) | Phase 11 | Step 5 | pending |
| T-215 | Port `LimitedBeadingStrategy` (max-bead-count cap; internal 0-width sentinel insertion) | Phase 11 | Step 6 | pending |
| T-215b | Implement strip-pass: `compute_and_strip` drops zero-width beads before `WallLoop` assembly per D-9; register `D-111-ARACHNE-SENTINEL-STRIP` in `docs/DEVIATION_LOG.md` | Phase 11 | Step 6 | pending |
| T-216 | Port `BeadingStrategyFactory::create_stack` composing `Limited(OuterWallInset(Widening(Redistribute(Distributed))))` | Phase 11 | Step 7 | pending |
| T-218 | Register all 11 Arachne `m_params.*` config keys in `docs/15_config_keys_reference.md` and `arachne-perimeters.toml` | Phase 11 | Step 8 | pending |

## Cross-Packet Contracts

- **P110 prerequisite**: P111 consumes no P110 *code* symbols (the beading stack is pure-data, `slicer-core`-internal; P112 connects it to the SKT graph). BUT AC-9 forward-deps on P110/T-205 CREATING `arachne-perimeters.toml` — the 11 config-schema blocks have nowhere to land until that manifest exists.
- **P112 consumer**: `BeadingStrategyFactory::create_stack(params)` is the entry point P112's T-221 (`assign_bead_counts`) will call per-edge.
- **D-9 closure**: the strip-pass decision (D-9) lives in `docs/specs/perimeter-modules-orca-parity-roadmap.md`, NOT in `docs/DEVIATION_LOG.md`. T-215b implements it and registers a new `D-111-ARACHNE-SENTINEL-STRIP` log entry; the AC greps the log for the new ID.

## Deferred / Deviation Registrations

| Deviation ID | Reason | Registered in Step |
| --- | --- | --- |
| `D-111-ARACHNE-SENTINEL-STRIP` | Sentinels stay internal to `LimitedBeadingStrategy::compute`; stripped before external output via `compute_and_strip` (per D-9 direction) | Step 6 (T-215b) |

## New Test Files

| Test File | Cargo.toml `[[test]]` Entry | Target AC |
| --- | --- | --- |
| `crates/slicer-core/tests/beading/distributed.rs` | `[[test]] name = "beading_distributed"` (add Step 1) | AC-2 |
| `crates/slicer-core/tests/beading/redistribute.rs` | `[[test]] name = "beading_redistribute"` (add Step 1) | AC-3 |
| `crates/slicer-core/tests/beading/widening.rs` | `[[test]] name = "beading_widening"` (add Step 1) | AC-4 |
| `crates/slicer-core/tests/beading/outer_wall_inset.rs` | `[[test]] name = "beading_outer_wall_inset"` (add Step 1) | AC-5 |
| `crates/slicer-core/tests/beading/limited.rs` | `[[test]] name = "beading_limited"` (add Step 1) | AC-6, AC-7, AC-N1, AC-N2 |
| `crates/slicer-core/tests/beading/factory.rs` | `[[test]] name = "beading_factory"` (add Step 1) | AC-8 |
