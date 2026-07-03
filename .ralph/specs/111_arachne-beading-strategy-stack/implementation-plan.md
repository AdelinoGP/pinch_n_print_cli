# Implementation Plan: 111_arachne-beading-strategy-stack

## Step Order Rationale

Trait (Step 1) → Distributed base (Step 2) → 4 decorators (Steps 3–6) → factory composition (Step 7) → docs + config keys (Step 8).

The trait is a hard prerequisite — every strategy implements it. Distributed is the base of the decorator chain (Steps 3–6 each wrap an inner `Box<dyn BeadingStrategy>`, and the chain must start somewhere). The 4 decorators are independent of each other at the data level — each test fixture exercises one decorator over a recorded "pretend-inner" stub. Factory composition (Step 7) wires them in canonical order; the factory test asserts both composition order AND end-to-end Orca reference match. Docs + config-key registration is last because they reference paths added by earlier steps.

The packet does not touch IR, WIT, host, or any other crate. All work is `slicer-core`-internal except the manifest schema blocks in `arachne-perimeters.toml` (Step 8).

## Step 1 — Trait + Base Struct (T-210)

- **Tasks:** T-210.
- **Objective:** Define `BeadingStrategy` trait and `Beading` struct in `crates/slicer-core/src/beading/mod.rs`. Register the module in `lib.rs`. Write a trait-surface compile-time test (AC-1).
- **Precondition:** None.
- **Postcondition:** `cargo check -p slicer-core` green; AC-1 falsifiable via `rg` checks.
- **Files allowed to read:** `crates/slicer-core/src/lib.rs` (current `pub mod` set), `crates/slicer-core/Cargo.toml` (existing `[[test]]` entries — to know how to append new ones).
- **Files allowed to edit:** `crates/slicer-core/src/lib.rs`, `crates/slicer-core/src/beading/mod.rs` (NEW), `crates/slicer-core/Cargo.toml` (add `[[test]] name = "beading_*"` entries — one entry per test file created across Steps 2–7; add all upfront in Step 1 to avoid re-opening the file 6 times).
- **Expected sub-agent dispatches:** ONE OrcaSlicer LOCATIONS dispatch for `BeadingStrategy.h` — return ≤ 10 entries listing virtual method signatures + `Beading` struct fields.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer base interface (via dispatch).
- **Test registration note:** `slicer-core` uses explicit `[[test]] name = "..."` entries per file. The 6 new test files (`beading/distributed.rs`, `beading/redistribute.rs`, `beading/widening.rs`, `beading/outer_wall_inset.rs`, `beading/limited.rs`, `beading/factory.rs`) must each have a `[[test]]` entry added in `Cargo.toml`. Registering upfront in Step 1 keeps subsequent steps within their 3-edit cap.
- **Narrow verification:** `cargo check -p slicer-core 2>&1 | tee target/test-output.log && rg -q 'fn compute' crates/slicer-core/src/beading/mod.rs && rg -q 'struct Beading' crates/slicer-core/src/beading/mod.rs`.
- **Cheapest falsifying check:** AC-1 commands — three `rg -q` predicates on the trait file.

## Step 2 — DistributedBeadingStrategy (T-211)

- **Tasks:** T-211 + AC-N1.
- **Objective:** Implement `DistributedBeadingStrategy` with the Gaussian-weighted width distribution. Record the 10-thickness OrcaSlicer reference golden. Write AC-2 + AC-N1 tests.
- **Precondition:** Step 1 done (trait exists).
- **Postcondition:** `cargo test -p slicer-core distributed_beading_strategy_orca_table` green; AC-2 + AC-N1 falsifiable.
- **Files allowed to read:** `crates/slicer-core/src/beading/mod.rs` (Step 1 output).
- **Files allowed to edit:** `crates/slicer-core/src/beading/distributed.rs` (NEW), `crates/slicer-core/src/beading/mod.rs` (add `pub mod distributed;`), `crates/slicer-core/tests/beading/distributed.rs` (NEW — `[[test]]` entry already added in Step 1), `crates/slicer-core/tests/fixtures/beading/distributed_10_thickness.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `DistributedBeadingStrategy.cpp` — return ≤ 150 words: `compute` body + Gaussian decay constant. ONE `cargo test -p slicer-core distributed_beading_strategy_orca_table` — return FACT pass/fail per thickness.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer DistributedBeadingStrategy.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core distributed_beading_strategy_orca_table 2>&1 | tee target/test-output.log && cargo test -p slicer-core beading_invariant_locations_len_eq_widths_len 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Inspect Step 2's output `distributed_10_thickness.json` after first compute against a recorded value at thickness = `optimal_width * 1.0` (single-bead regime). Output must match Orca's single-bead output (width ≈ thickness, single toolpath at center).

## Step 3 — RedistributeBeadingStrategy (T-212)

- **Tasks:** T-212.
- **Objective:** Implement `RedistributeBeadingStrategy` as decorator over `Distributed`. Outer-wall widths held to `optimal_width` exactly; residual absorbed by inner beads. Write AC-3 test.
- **Precondition:** Step 2 done.
- **Postcondition:** `cargo test -p slicer-core redistribute_outer_consistent` green.
- **Files allowed to read:** `crates/slicer-core/src/beading/distributed.rs` (Step 2 output), `crates/slicer-core/src/beading/mod.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/beading/redistribute.rs` (NEW), `crates/slicer-core/src/beading/mod.rs` (add `pub mod redistribute;`), `crates/slicer-core/tests/beading/redistribute.rs` (NEW), `crates/slicer-core/tests/fixtures/beading/redistribute_outer_consistent.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `RedistributeBeadingStrategy.cpp` — ≤ 100 words. ONE `cargo test`.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer RedistributeBeadingStrategy.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core redistribute_outer_consistent 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Single-thickness fixture (thickness = optimal_width * 5) — outer `bead_widths[0]` and `bead_widths[-1]` should equal `optimal_width` to within 1 unit.

## Step 4 — WideningBeadingStrategy (T-213)

- **Tasks:** T-213.
- **Objective:** Implement `WideningBeadingStrategy` as decorator. For thickness < `min_input_width`, produce single bead at `min_bead_width`. At/above threshold, delegate to inner. Write AC-4 test.
- **Precondition:** Step 3 done (independent of redistribute structurally; serialized for context).
- **Postcondition:** `cargo test -p slicer-core widening_below_min_input_width` green.
- **Files allowed to read:** `crates/slicer-core/src/beading/mod.rs`, `crates/slicer-core/src/beading/redistribute.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/beading/widening.rs` (NEW), `crates/slicer-core/src/beading/mod.rs` (add `pub mod widening;`), `crates/slicer-core/tests/beading/widening.rs` (NEW), `crates/slicer-core/tests/fixtures/beading/widening_thin_wedge.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `WideningBeadingStrategy.cpp` — ≤ 100 words. ONE `cargo test`.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer WideningBeadingStrategy.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core widening_below_min_input_width 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Two fixtures: (a) thickness = `min_input_width * 0.5` → single bead at `min_bead_width`; (b) thickness = `min_input_width * 2.0` → delegates to inner unchanged.

## Step 5 — OuterWallInsetBeadingStrategy (T-214)

- **Tasks:** T-214.
- **Objective:** Implement `OuterWallInsetBeadingStrategy` as decorator. Modifies only `toolpath_locations[0]` and `toolpath_locations[-1]` by `±outer_wall_offset`. No-op when `outer_wall_offset == 0`. Write AC-5 test.
- **Precondition:** Step 4 done.
- **Postcondition:** `cargo test -p slicer-core outer_wall_inset_offset_outer_only` green.
- **Files allowed to read:** `crates/slicer-core/src/beading/mod.rs`, `crates/slicer-core/src/beading/widening.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/beading/outer_wall_inset.rs` (NEW), `crates/slicer-core/src/beading/mod.rs` (add `pub mod outer_wall_inset;`), `crates/slicer-core/tests/beading/outer_wall_inset.rs` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `OuterWallInsetBeadingStrategy.cpp` — ≤ 100 words. ONE `cargo test`.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer OuterWallInsetBeadingStrategy.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core outer_wall_inset_offset_outer_only 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Fixture with 5 inner beads + offset = 100 — verify `toolpath_locations[1..4]` are unchanged from inner.

## Step 6 — LimitedBeadingStrategy + Strip-Pass (T-215, T-215b)

- **Tasks:** T-215 + T-215b.
- **Objective:** Implement `LimitedBeadingStrategy` with max-bead-count cap and zero-width sentinel insertion. Implement `compute_and_strip` method that drops sentinels (T-215b strip-pass per D-9). Write AC-6 + AC-7 + AC-N2 tests.
- **Precondition:** Step 5 done.
- **Postcondition:** `cargo test -p slicer-core limited_inserts_sentinels_at_cap` + `cargo test -p slicer-core limited_compute_and_strip_no_zero_widths` + `cargo test -p slicer-core limited_raw_compute_retains_sentinels` all green.
- **Files allowed to read:** `crates/slicer-core/src/beading/mod.rs`, `crates/slicer-core/src/beading/distributed.rs`, `crates/slicer-core/src/beading/outer_wall_inset.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/beading/limited.rs` (NEW), `crates/slicer-core/src/beading/mod.rs` (add `pub mod limited;`), `crates/slicer-core/tests/beading/limited.rs` (NEW), `crates/slicer-core/tests/fixtures/beading/limited_cap_boundary.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `LimitedBeadingStrategy.cpp` — ≤ 150 words: cap rule + sentinel-insertion math. ONE `cargo test`.
- **Context cost:** M (two methods + sentinel mechanics + three tests).
- **Authoritative docs:** OrcaSlicer LimitedBeadingStrategy.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core limited_ 2>&1 | tee target/test-output.log` (runs all three Limited tests by prefix).
- **Cheapest falsifying check:** AC-N2's `limited_raw_compute_retains_sentinels` — falsifies if the implementer accidentally folds strip into `compute`.

## Step 7 — Factory Composition (T-216)

- **Tasks:** T-216 + AC-8.
- **Objective:** Implement `BeadingStrategyFactory::create_stack(params)` wrapping decorators in canonical order `Limited(OuterWallInset(Widening(Redistribute(Distributed))))`. Write AC-8 composition-order test + Orca reference-match test.
- **Precondition:** Steps 1–6 done (all 5 strategies green).
- **Postcondition:** `cargo test -p slicer-core factory_stack_composition_order` + `cargo test -p slicer-core factory_matches_orca_reference` both green.
- **Files allowed to read:** All 5 strategy files (Steps 2–6 outputs); `crates/slicer-core/src/beading/mod.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/beading/factory.rs` (NEW), `crates/slicer-core/src/beading/mod.rs` (add `pub mod factory;`), `crates/slicer-core/tests/beading/factory.rs` (NEW), `crates/slicer-core/tests/fixtures/beading/factory_orca_reference.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer LOCATIONS for `BeadingStrategyFactory.cpp::create_strategy` — ≤ 10 entries showing wrapping order. ONE `cargo test`.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer BeadingStrategyFactory.cpp (via LOCATIONS).
- **Narrow verification:** `cargo test -p slicer-core factory_ 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** `factory_stack_composition_order` reads `type_label` recursively and asserts the path `Limited → OuterWallInset → Widening → Redistribute → Distributed`.

## Step 8 — Config Keys + Docs (T-218 + strip-pass deviation entry)

- **Tasks:** T-218 + AC-9 + `D-111-ARACHNE-SENTINEL-STRIP` log entry + Doc Impact Statement.
- **Objective:** Register the 11 `m_params.*` config keys in `docs/15_config_keys_reference.md` and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`. Register the `beading` sub-module in `docs/01_system_architecture.md` (slicer-core tier-2 pipeline modules). Add a new `D-111-ARACHNE-SENTINEL-STRIP` entry in `docs/DEVIATION_LOG.md` (note: D-9 is a roadmap ID in `docs/specs/perimeter-modules-orca-parity-roadmap.md`, NOT an entry in `DEVIATION_LOG.md` — do not grep the log for D-9). Flip Phase 11 rows to DONE in the roadmap.
- **Precondition:** Step 7 done.
- **Postcondition:** AC-9 + Doc Impact Statement all green.
- **Files allowed to read:** `docs/15_config_keys_reference.md` (existing entry format), `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (FORWARD-DEP on P110/T-205 — created by P110's skeleton; read it at activation to see what keys it declares and confirm none of the 11 new keys collide — do NOT assume the old deleted module's key set), `docs/03_wit_and_manifest.md` (§"Module Manifest TOML"), `docs/01_system_architecture.md`, `docs/DEVIATION_LOG.md` (read to learn entry format + confirm `D-111-ARACHNE-SENTINEL-STRIP` absent before adding).
- **Files allowed to edit:** `docs/15_config_keys_reference.md`, `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `docs/01_system_architecture.md`, `docs/DEVIATION_LOG.md`, `docs/specs/perimeter-modules-orca-parity-roadmap.md`.
- **Expected sub-agent dispatches:** ONE OrcaSlicer LOCATIONS for `PrintConfig.cpp` — return ≤ 20 entries naming each of the 11 `m_params.*` defaults + units + descriptions.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer PrintConfig.cpp (via LOCATIONS).
- **Narrow verification:** The AC-9 loop from `packet.spec.md` § Acceptance Criteria. Then `rg -q 'D-111-ARACHNE-SENTINEL-STRIP' docs/DEVIATION_LOG.md && rg -q 'T-210.*DONE' docs/specs/perimeter-modules-orca-parity-roadmap.md && rg -q 'beading' docs/01_system_architecture.md`.
- **Cheapest falsifying check:** The AC-9 shell loop falsifies any missing key in either docs or manifest in O(11) lookups.

## Packet Completion Gate

- All 9 ACs + 2 AC-Ns pass per their pipe-suffix commands.
- `cargo check --workspace --all-targets` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo test -p slicer-core beading 2>&1 | tee target/test-output.log` shows all new tests passing.
- AC-9 shell loop succeeds against both `docs/15_config_keys_reference.md` and `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`.
- Closure log (`.ralph/specs/111_arachne-beading-strategy-stack/closure-log.md`) authored; records: any divergence between recorded `m_params.*` defaults and the translated-through-/100 values; D-9 closure rationale paragraph; any goldens that needed re-recording with a NOTE explaining why (regenerating during the packet is the closure-blocker — explain or split the regen into a follow-on).
- Status flipped to `implemented` in `packet.spec.md` YAML frontmatter.

## Context Budget Cap

Aggregate cost: M. If the implementer reaches 60% context at any step, they STOP, write the partial-state to closure-log.md, and hand off — the remaining steps inherit. Step 2 (Distributed Gaussian) and Step 7 (factory composition) are the most likely overflow points. If Step 2 overflows, the implementer SHOULD split T-211 + the goldens into a follow-on packet (P111a) and ship the trait + the 4 decorators against `MockStrategy` stubs first.
