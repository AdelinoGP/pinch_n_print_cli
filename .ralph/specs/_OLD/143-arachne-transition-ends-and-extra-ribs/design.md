# Design: 143-arachne-transition-ends-and-extra-ribs

## Controlling Code Paths

- Primary code path: `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:646-740` (`apply_transitions`) — the single-mid-split + `transition_ratio = 0.0` scheme B rewrites to canonical end-based splitting.
- Neighboring code paths: `beading/mod.rs:64-108` (trait surface), `distributed.rs:43` (`default_transition_length` — currently `#[allow(dead_code)]`, becomes live), `arachne/generate_toolpaths.rs` (beading interpolation at emission), `arachne/pipeline.rs:345-346` (stage wiring).
- Neighboring tests/fixtures: `arachne_parity_red_transition_ends.rs` (N3 red tests — B's oracle; call sites updated, assertions untouched), `crates/slicer-core/tests/fixtures/arachne/propagation_*.json` (re-baseline).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **`BeadingStrategy` trait extension is `slicer-core`-internal.** The trait is not exposed across the WIT host boundary (the host boundary marshals `Vec<ExtrusionLine>`, not `BeadingStrategy` trait objects); no WIT/IR schema change. Confirmed during grilling: `beading/` is entirely `slicer-core`-internal.
- Packet-specific constraint: **`wall_transition_angle` already exists** on the trait at `mod.rs:93` (with `DistributedBeadingStrategy` override at `distributed.rs:195` and the 4 decorators delegating). B does NOT add a duplicate; disambiguate during grilling.
- Packet-specific constraint: **`EdgeType::TRANSITION_END` is a PNP invention, currently unused.** B prefers delete unless the rewrite needs an edge marker for the new `TransitionEnd` type.
- Packet-specific constraint: **WASM staleness does NOT apply** — B's change surface is `slicer-core`-internal; no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.

## Code Change Surface

- Selected approach: faithful port of canonical `generateTransitioningRibs` (mids → filter → ends → apply) + `generateExtraRibs` + beading interpolation at emission, plus the `BeadingStrategy` trait extension. The trait extension lands first (Step 1) with default implementations so the 5 concrete strategies compile without caller-side breakage; the pipeline stage (Step 2) uses the new methods.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/beading/mod.rs` — add `get_transitioning_length` / `get_transition_anchor_pos` / `get_nonlinear_thicknesses` to `BeadingStrategy` trait with default impls.
  - `crates/slicer-core/src/beading/distributed.rs` — override `get_transitioning_length` to return `self.default_transition_length` (remove `#[allow(dead_code)]` on line 43); override the other two with canonical defaults.
  - `crates/slicer-core/src/beading/{widening,redistribute,outer_wall_inset,limited}.rs` — each decorator's `impl BeadingStrategy` gains the 3 new methods delegating to `self.parent.*` (matching the existing `wall_transition_angle` delegation pattern).
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — NEW `filter_transition_mids`, `generate_all_transition_ends`, `generate_extra_ribs`; rewrite `apply_transitions:646-740` to consume `TransitionEnd`s (not `TransitionMiddle`s) and insert at END positions with `bead_count = lower` or `lower + 1` per `is_lower_end`, writing fractional `transition_ratio` on traversed nodes.
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — `TransitionEnd` type (NEW) with `pos`, `bead_count`, `mid_r`, `is_lower_end` fields.
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — beading interpolation at emission for nonzero `transition_ratio` nodes.
  - `crates/slicer-core/src/arachne/pipeline.rs:345-346` — insert `generate_all_transition_ends` between `generate_transition_mids` and `apply_transitions`; insert `generate_extra_ribs` after `apply_transitions`.
  - `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` — call-site update (assertions untouched).
  - `crates/slicer-core/tests/fixtures/arachne/propagation_*.json` — re-baselined via self-capture.
- Rejected alternatives:
  - **Make `apply_transitions` absorb end-generation** (single entry point, red tests unchanged) — rejected during grilling (user decision: new `generate_all_transition_ends` pipeline stage, red-test call sites updated). Cleaner long-term seam; `filterTransitionMids` testable in isolation.
  - **Add a duplicate `wall_transition_angle`** — rejected (it already exists at `mod.rs:93`).
  - **Wire the π hack removal into B** — rejected (Packet C's scope, strictly after A2).

## Files in Scope (read + edit)

- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — role: N3 end-generation + `apply_transitions` rewrite + `generateExtraRibs`; expected change: rewrite `:646-740`, add `filter_transition_mids`/`generate_all_transition_ends`/`generate_extra_ribs`.
- `crates/slicer-core/src/beading/mod.rs` — role: N3 trait extension; expected change: add 3 trait methods with default impls.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — role: N3 beading interpolation at emission; expected change: interpolate `compute(thickness, bead_count)` ↔ `compute(thickness, bead_count + 1)` for nonzero `transition_ratio`.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-core/src/beading/distributed.rs` — full (198 lines); purpose: `default_transition_length` (line 43) + the existing `wall_transition_angle` override (line 195) pattern.
- `crates/slicer-core/src/beading/{widening,redistribute,outer_wall_inset,limited}.rs` — range-read each `impl BeadingStrategy` block only; purpose: the `self.parent.wall_transition_angle()` delegation pattern the 3 new methods mirror.
- `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` — full (217 lines); purpose: B's oracle + the call-site update target.
- `crates/slicer-core/src/arachne/pipeline.rs` — lines `:340-360` (stage wiring); purpose: insert `generate_all_transition_ends` + `generate_extra_ribs`.
- `docs/15_config_keys_reference.md` lines ~479-521 — purpose: `wall_transition_length` / `wall_transition_filter_deviation` defaults.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — delegate parity checks via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-core/src/arachne/pipeline.rs:334` (π hack) and `:272-277` (0.1× fudge) — Packet C's scope.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:120-160` (`upward_central_edges`) and `:980-1100` (`propagate_beadings_downward`) — A1's scope.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs:401-758` (`connectJunctions` emission) — A2's scope; B only touches the beading-interpolation site.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` — Packet F.

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "SUMMARY of `SkeletalTrapezoidation.cpp:1247-1403` `generateAllTransitionEnds` — explicitly ask for the recursive travel structure + fractional `transition_ratio` assignment + the lower/upper end walk (backward on `edge.twin` / forward); return ≤ 200 words, no code unless asked" — purpose: confirm Step 2's end-generation.
- "SUMMARY of `SkeletalTrapezoidation.cpp:1007-1076` `filterTransitionMids` — ask for the recursive dissolve condition (same-`lower_bead_count` within `transition_filter_dist`); return ≤ 200 words" — purpose: confirm Step 2's filter.
- "SUMMARY of `SkeletalTrapezoidation.cpp:1487-1543` `applyTransitions` at ends — ask for the `is_lower_end` → `bead_count = lower` or `lower + 1` assignment; return ≤ 200 words" — purpose: confirm Step 2's `apply_transitions` rewrite.
- "SUMMARY of `SkeletalTrapezoidation.cpp:1579-1633` `generateExtraRibs` — ask for the `discretization_step_size` gate + `getNonlinearThicknesses()` radius iteration; return ≤ 200 words" — purpose: confirm Step 2's `generate_extra_ribs`.
- "SUMMARY of `SkeletalTrapezoidation.cpp:1712-1721` `generateSegments` beading interpolation — ask for the `compute(thickness, bead_count)` ↔ `compute(thickness, bead_count + 1)` blend on nonzero `transition_ratio`; return ≤ 200 words" — purpose: confirm Step 2's emission interpolation.
- "SUMMARY of `BeadingStrategy.h` — ask for the `getTransitioningLength` / `getTransitionAnchorPos` / `getNonlinearThicknesses` signatures + canonical defaults; return ≤ 200 words" — purpose: confirm Step 1's trait extension.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1 + AC-2.
- "Run `cargo check -p slicer-core --all-targets`; return FACT pass/fail" — purpose: validate AC-N1 (trait extension compiles).
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT pass (expected — N1/N2/N4 stay green)" — purpose: gate B didn't regress A1/A2.
- "Find all `impl BeadingStrategy for`; return LOCATIONS" — purpose: confirm the 5 concrete strategies' delegation sites.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. `BeadingStrategy` trait is `slicer-core`-internal; not exposed across the WIT boundary. `TransitionEnd` is a new `skeletal_trapezoidation`-internal type (not in `slicer-ir`).
- WIT boundary considerations: **none**. No WIT/IR schema change. The host boundary marshals `Vec<ExtrusionLine>`, not `BeadingStrategy` trait objects.
- Determinism: B's rewrite preserves determinism (the recursive travel is index-ordered; the fractional `transition_ratio` is a deterministic function of the edge geometry + `get_transitioning_length`; `filterTransitionMids`'s dissolve is deterministic under ties via index-ascending).

## Locked Assumptions and Invariants

- `get_transitioning_length` returns `self.default_transition_length` from `DistributedBeadingStrategy` (line 43, `#[allow(dead_code)]` removed); the 4 decorators delegate to `self.parent`.
- `wall_transition_angle` already exists (`mod.rs:93`); B does NOT add a duplicate.
- `EdgeType::TRANSITION_END` is deleted unless the rewrite needs an edge marker.
- N3 red-test call sites are updated (assertions untouched per grilling decision).
- B keeps N1, N2, N4 red tests GREEN (gated).
- B does NOT remove the π hack or the 0.1× filter-dist fudge (Packet C's scope).
- Beading-stack audit is mandatory (B's author confirms the 5 concrete strategies' readiness before implementation).
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.
- `transition_ratio` is fractional (strictly between 0 and 1) on traversed nodes, not `0.0`.

## Risks and Tradeoffs

- **The `generateAllTransitionEnds` recursive travel is the most complex new code in B.** Risk is contained by the N3 red tests (the fractional-ratio observable) + the `propagation` regression suite.
- **The `BeadingStrategy` trait extension could break the 5 concrete strategies if the default impls are wrong.** Mitigated by AC-N1 (`cargo check --all-targets`) + the grilling-confirmed fact that `DistributedBeadingStrategy` already stores `default_transition_length` and the 4 decorators already follow the `self.parent` delegation pattern for `wall_transition_angle`.
- **Beading-stack audit gap.** `crates/slicer-core/src/beading/` was out of the audit's read scope. B's author must confirm readiness; if a strategy needs a non-delegating override, that's a discovery during implementation, not a blocker.
- **`EdgeType::TRANSITION_END` deletion could ripple if downstream code references it.** The audit says it's currently unused; B confirms via grep before deleting.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — the `generate_all_transition_ends` + `filter_transition_mids` + `generate_extra_ribs` + `apply_transitions` rewrite + emission interpolation, the bulk of the work).
- Highest-risk dispatch: the `generateAllTransitionEnds` SUMMARY — its return could blow budget if it returns code instead of prose. Required return format: `SUMMARY ≤ 200 words, no code unless asked`.

## Open Questions

- [FWD] Does `get_transition_anchor_pos` need the `edge` + `mid` as arguments, or just the `lower_bead_count`? The canonical signature (`BeadingStrategy.h`) should clarify via the delegated SUMMARY.
- [FWD] Does `generateExtraRibs`'s `discretization_step_size` come from a config key or a hardcoded constant? The delegated SUMMARY of `:1579-1633` should clarify.
- [FWD] Should `generate_all_transition_ends` be a single function or split into `filter_transition_mids` + `generate_all_transition_ends` (two pub fns)? The canonical `generateTransitioningRibs` calls them sequentially; B's author decides the Rust API surface — two pub fns is more testable.

None activation-blocking.