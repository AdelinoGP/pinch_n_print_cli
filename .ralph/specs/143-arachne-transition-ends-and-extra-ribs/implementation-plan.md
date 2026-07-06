# Implementation Plan: 143-arachne-transition-ends-and-extra-ribs

## Execution Rules

- One atomic step at a time.
- Each step maps back to the packet's grouped task IDs (`none` — provenanced by the audit + red tests at `b2ea52b7`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: `BeadingStrategy` trait extension + concrete-strategy overrides

- Task IDs:
  - `none` (N3 trait extension — provenanced by `target/arachne_parity_audit_20260706_020657.md` §N3)
- Objective: Add `get_transitioning_length` / `get_transition_anchor_pos` / `get_nonlinear_thicknesses` to the `BeadingStrategy` trait with default implementations; override on `DistributedBeadingStrategy` (`get_transitioning_length` returns `self.default_transition_length`, removing `#[allow(dead_code)]` on line 43); add delegation overrides on the 4 decorators. **Beading-stack audit**: confirm the 5 concrete strategies' readiness for the 3 new methods before implementation.
- Precondition: A2 (`142`) is `status: implemented` — B builds on A1/A2's junction fans + emission.
- Postcondition: `cargo check -p slicer-core --all-targets` passes (AC-N1) — all 5 concrete strategies compile without caller-side breakage. N1/N2/N4 red tests stay GREEN. N3 red tests still FAIL (Step 2 owns the pipeline stage).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/beading/mod.rs` — full (108 lines); trait surface + existing `wall_transition_angle` default (`:93`).
  - `crates/slicer-core/src/beading/distributed.rs` — full (198 lines); `default_transition_length` (`:43`) + `wall_transition_angle` override (`:195`).
  - `crates/slicer-core/src/beading/{widening,redistribute,outer_wall_inset,limited}.rs` — range-read each `impl BeadingStrategy` block only; the `self.parent.wall_transition_angle()` delegation pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/beading/mod.rs`
  - `crates/slicer-core/src/beading/distributed.rs`
  - `crates/slicer-core/src/beading/widening.rs` (and `redistribute.rs`/`outer_wall_inset.rs`/`limited.rs` — but the skill says ≤ 3 files per step; B's implementer may need to edit all 5 beading files. Justification: the trait extension ripples into all 5 concrete strategies. This is a known exception — the 4 decorators' changes are mechanical (3 lines each, `self.parent.*` delegation). If the implementer's tooling limits to 3 edits per step, split Step 1 into 1a (mod.rs + distributed.rs + widening.rs) and 1b (redistribute.rs + outer_wall_inset.rs + limited.rs).)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (Step 2's scope)
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` (Step 2's emission interpolation)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `BeadingStrategy.h` — ask for the `getTransitioningLength` / `getTransitionAnchorPos` / `getNonlinearThicknesses` signatures + canonical defaults; return ≤ 200 words" — purpose: confirm Step 1's trait extension.
  - "Find all `impl BeadingStrategy for`; return LOCATIONS" — purpose: confirm the 5 delegation sites.
  - "Run `cargo check -p slicer-core --all-targets`; return FACT pass/fail" — purpose: validate AC-N1.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT pass (expected — N1/N2/N4 stay green)" — purpose: gate Step 1 didn't regress A1/A2.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — N3 still red, Step 2 owns it)" — purpose: gate scope.
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` §"Arachne beading strategy stack" (lines ~479-521) — `wall_transition_length` / `wall_transition_filter_deviation` defaults.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.h` — delegate.
- Verification:
  - `cargo check -p slicer-core --all-targets 2>&1 | tee target/test-output-b-step1-neg1.log` — FACT pass (AC-N1).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 | tee target/test-output-b-step1-stays-green.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-b-step1-n3-red.log` — FACT fail (expected).
- Exit condition: AC-N1 passes; N1/N2/N4 stay green; N3 stays red; `cargo check -p slicer-core --all-targets` + `cargo clippy -p slicer-core --all-targets -- -D warnings` pass.

### Step 2: `generate_all_transition_ends` + `filter_transition_mids` + `apply_transitions` rewrite + `generate_extra_ribs` + emission interpolation + N3 call-site update + fixtures + deviation log

- Task IDs:
  - `none` (N3 + N8 — provenanced by §N3 + §N8)
- Objective: Add `filter_transition_mids`, `generate_all_transition_ends`, `generate_extra_ribs` to `propagation.rs`; rewrite `apply_transitions:646-740` to consume `TransitionEnd`s (not `TransitionMiddle`s), insert at END positions with `bead_count = lower` or `lower + 1` per `is_lower_end`, and write fractional `transition_ratio` on traversed nodes; add beading interpolation at emission in `generate_toolpaths.rs` for nonzero `transition_ratio`; update N3 red-test call sites to invoke `generate_all_transition_ends` before `apply_transitions` (assertions untouched); wire `pipeline.rs:345-346`; re-baseline `propagation_*.json`; add `D-143-TRANSITION-ENDS` deviation-log entry + `D-112-PROPAGATION-ADAPT` addendum; decide `EdgeType::TRANSITION_END` repurpose vs delete.
- Precondition: Step 1 is green (the 3 trait methods exist with defaults + overrides).
- Postcondition: AC-1 (lower+upper end splits) + AC-2 (fractional ratio on spillover) pass. N1/N2/N4 stay GREEN. `propagation` regression green (fixtures re-baselined). `D-143-TRANSITION-ENDS` present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — lines `:640-740` (`apply_transitions`); do NOT read `:120-160`/`:980-1100` (A1's scope).
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — range-read the emission interpolation site only.
  - `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` — full (217 lines); call-site update target.
  - `crates/slicer-core/src/arachne/pipeline.rs` — lines `:340-360` (stage wiring).
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — `TransitionMiddle` struct (for the new `TransitionEnd` type's parallel shape).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs`
  - `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` (call-site update only — assertions untouched)
- (Secondary edits not counted against the ≤ 3: `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` for the `TransitionEnd` type, `crates/slicer-core/src/arachne/pipeline.rs:345-346` for stage wiring, `docs/DEVIATION_LOG.md` for the addendum, `crates/slicer-core/tests/fixtures/arachne/propagation_*.json` for re-baseline — these are surgical insertions, not primary edit surfaces.)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/beading/*` (Step 1's scope — the trait + strategies are done)
  - `crates/slicer-core/src/arachne/pipeline.rs:334` and `:272-277` (Packet C)
  - `OrcaSlicerDocumented/...` (delegate)
- Expected sub-agent dispatches:
  - "SUMMARY of `SkeletalTrapezoidation.cpp:1247-1403` `generateAllTransitionEnds` — recursive travel + fractional `transition_ratio` + lower/upper end walk; return ≤ 200 words" — purpose: confirm end-generation.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:1007-1076` `filterTransitionMids` — recursive dissolve condition; return ≤ 200 words" — purpose: confirm filter.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:1487-1543` `applyTransitions` at ends — `is_lower_end` → `bead_count = lower` or `lower + 1`; return ≤ 200 words" — purpose: confirm `apply_transitions` rewrite.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:1579-1633` `generateExtraRibs` — `discretization_step_size` gate + `getNonlinearThicknesses()`; return ≤ 200 words" — purpose: confirm `generate_extra_ribs`.
  - "SUMMARY of `SkeletalTrapezoidation.cpp:1712-1721` `generateSegments` beading interpolation; return ≤ 200 words" — purpose: confirm emission interpolation.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1 + AC-2.
  - "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT pass (expected — N1/N2/N4 stay green)" — purpose: gate B didn't regress A1/A2.
  - "Run `cargo test -p slicer-core --features host-algos --test propagation 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/b-cube4color.gcode && cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture`; return FACT + summary line — purpose: record e2e closure delta (record-only)."
  - "Run `rg 'EdgeType::TRANSITION_END' crates/slicer-core/src`; return LOCATIONS" — purpose: confirm whether `TRANSITION_END` is referenced before deciding delete vs repurpose.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` §"Constant Conversion Table" — 0.4 mm = 4000 units, 0.1 mm = 1000 units.
  - `docs/DEVIATION_LOG.md` `D-112-PROPAGATION-ADAPT` entry — addendum target.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:881-915` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1007-1076` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1247-1403` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1487-1543` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1579-1633` — delegate.
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:1712-1721` — delegate.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 | tee target/test-output-b-step2-ac.log` — FACT pass (AC-1 + AC-2).
  - `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --no-fail-fast 2>&1 | tee target/test-output-b-step2-stays-green.log` — FACT pass.
  - `cargo test -p slicer-core --features host-algos --test propagation 2>&1 | tee target/test-output-b-step2-regression.log` — FACT pass (fixtures re-baselined).
  - `rg -q 'D-143-TRANSITION-ENDS' docs/DEVIATION_LOG.md` — FACT pass.
- Exit condition: AC-1, AC-2 pass; N1/N2/N4 stay green; `propagation` regression green; `D-143-TRANSITION-ENDS` present; `cargo check -p slicer-core --all-targets` + `cargo clippy -p slicer-core --all-targets -- -D warnings` pass; e2e closure delta recorded (record-only).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (trait extension + concrete strategies) | M | Heaviest dispatch: `BeadingStrategy.h` SUMMARY. |
| Step 2 (pipeline stage + apply_transitions rewrite + emission + fixtures + deviation log) | M | Heaviest dispatch: 5 OrcaSlicer SUMMARYs + 4 test runs. |

Aggregate: M + M = M (Step 2 shares Step 1's `beading/` context partially). If the sum exceeds M aggregate in practice, hand off after Step 1.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1, AC-2, AC-N1 dispatched and returned PASS).
- N1, N2, N4 stay GREEN (scope boundary gates).
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask build-guests --check` returns clean.
- `D-143-TRANSITION-ENDS` present in `docs/DEVIATION_LOG.md` with addendum on `D-112-PROPAGATION-ADAPT`.
- Affected `propagation_*.json` fixtures re-baselined with rationale in commit messages.
- e2e closure delta recorded (record-only — Packet F blocks on green).
- `docs/07_implementation_status.md` updated (via worker dispatch).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1, AC-2, AC-N1).
- Confirm packet-level verification commands are green.
- Confirm N1/N2/N4 "stays green" commands returned as expected.
- Record the e2e closure delta explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.