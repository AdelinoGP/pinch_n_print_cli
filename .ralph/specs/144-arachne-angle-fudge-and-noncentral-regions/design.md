# Design: 144-arachne-angle-fudge-and-noncentral-regions

## Controlling Code Paths

- Primary code path: `crates/slicer-core/src/arachne/pipeline.rs:272-277` (`to_centrality_params` — the 0.1× filter-dist fudge) and `:325-339` (the π hack + `filter_central` call site) — the two fudges C deletes + the thread-the-configured-angle change.
- Neighboring code path: `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs:100-200` (`filter_central` + `updateIsCentral` predicate) — C adds `filter_noncentral_regions` here.
- Neighboring tests/fixtures: `arachne_parity_red_junction_bands.rs` (N1 red tests — C's AC-1 oracle, must stay green), `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` (re-baseline), `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs` (NEW — AC-2 dumbbell test).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **C must NOT extend the `BeadingStrategy` trait.** B (`143`) owns the trait extension. C only threads the **already-existing** `wall_transition_angle` (on the trait at `beading/mod.rs:93`, threaded via `BeadingFactoryParams` at `factory.rs:92,157,192`) through `filter_central`. The strategy's `wall_transition_angle()` is the source of truth, not a hardcoded π.
- Packet-specific constraint: **C must NOT wire the whisker-dissolve `filterCentral`.** It is dead code upstream (`SkeletalTrapezoidation.cpp:716-730`, self-contradictory condition). PNP's un-wired helpers (`centrality.rs:263-389`) correctly mirror this dead code; leave them. The audit explicitly flags this as a gotcha — do not "fix" PNP by wiring the dissolve in.
- Packet-specific constraint: **C's removal of the π hack changes runtime behavior for every polygon.** The configured 10° is now the actual gate, not π. This is the intended behavior change (canonical parity), but it shifts centrality classification for many fixtures. The centrality fixture re-baseline records the drift; the commit message surfaces this as a scope decision.
- Packet-specific constraint: **WASM staleness does NOT apply** — C's change surface is `slicer-core`-internal (`arachne/pipeline.rs`, `skeletal_trapezoidation/centrality.rs`); no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.

## Code Change Surface

- Selected approach: delete the two fudges (π hack + 0.1× filter-dist) and thread the configured `wall_transition_angle` through `filter_central`; port `filterNoncentralRegions` as a new `filter_noncentral_regions` function in `centrality.rs`, called unconditionally after `assign_bead_counts` in `pipeline.rs`. The two changes are bundled because they are both centrality-pipeline normalizations — the π hack removal changes which edges are central, and `filterNoncentralRegions` re-promotes fragmented central regions; landing one without the other would produce a transiently worse state.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/arachne/pipeline.rs` — `to_centrality_params` (`:272-277`): delete `* 0.1` scaling on `transition_filter_dist`; pass `params.transition_filter_dist * UNITS_PER_MM` directly. `run_arachne_pipeline` (`:325-339`): delete the `effective_transitioning_angle_rad = std::f64::consts::PI` line + its "TEMPORARY" doc comment; replace with `strategy.wall_transition_angle()` (or `beading_params.wall_transition_angle`) passed to `filter_central`. Insert `filter_noncentral_regions(&mut graph)` call after `assign_bead_counts` (`:343`), mirroring `:633`'s "after `updateBeadCount`" ordering.
  - `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — NEW `filter_noncentral_regions` function mirroring `SkeletalTrapezoidation.cpp:811-862`: promote non-central gaps between same/±1-bead-count central regions (within hardcoded 0.4 mm = 4000 slicer units) back to central; copy bead counts across. The `pub fn filter_noncentral_regions(graph: &mut SkeletalTrapezoidationGraph)` signature matches the existing `filter_central` convention.
  - `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs` (NEW) — dumbbell polygon test (two 3 mm-wide pads joined by a 0.35 mm neck); assert single stitched inset-0 ring pair, not four fragments.
  - `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` — re-baselined via self-capture (angle parameter change + new `filter_noncentral_regions` drift the centrality output).
- Rejected alternatives:
  - **Wire the whisker-dissolve `filterCentral`** — rejected (dead code upstream, `:716-730` self-contradictory condition; the audit explicitly flags this as a gotcha).
  - **Extend the `BeadingStrategy` trait with `getTransitioningAngle`** — rejected (it already exists at `beading/mod.rs:93` as `wall_transition_angle`; C does not add a duplicate. B owns the trait extension for the 3 new methods).
  - **Split N5 and N6 into two packets** — rejected (both are centrality-pipeline normalizations; landing one without the other would produce a transiently worse state. Bundled as one M packet.)
  - **Remove the π hack in A1** — rejected (load-bearing for A1's centrality-gated scheme until A1's rewrite lands; C removes it strictly after A2, per the dependency graph).

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/pipeline.rs` — role: N5 fudge deletion + angle threading + `filter_noncentral_regions` call; expected change: delete `:272-277` 0.1× scaling, delete `:325-334` π hack, thread `wall_transition_angle` at `:335-339`, insert `filter_noncentral_regions` after `:343`.
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — role: N6 `filter_noncentral_regions` port; expected change: add `pub fn filter_noncentral_regions(graph: &mut SkeletalTrapezoidationGraph)` mirroring `:811-862`.
- `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs` — role: AC-2 dumbbell test; expected change: NEW file, dumbbell polygon + single-ring-pair assertion.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-core/src/beading/mod.rs` — full (108 lines); purpose: confirm `wall_transition_angle()` trait method at `:93` (C calls it from `pipeline.rs`; C does NOT edit `beading/`).
- `crates/slicer-core/src/beading/factory.rs` — range-read `:90-100, 150-200`; purpose: confirm `BeadingFactoryParams::wall_transition_angle` (line 92) + `create_stack` threading (lines 157, 192).
- `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); purpose: AC-1 oracle (must stay green).
- `docs/15_config_keys_reference.md` lines ~479-521 — purpose: `wall_transition_angle` default 10.0°, `wall_transition_filter_deviation` 1000 units.
- `docs/08_coordinate_system.md` §"Constant Conversion Table" (~30 lines) — purpose: 0.4 mm = 4000 units for `filterNoncentralRegions`'s hardcoded distance.
- `docs/DEVIATION_LOG.md` `D-141-JUNCTION-BANDS` entry — purpose: addendum target.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — delegate parity checks via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-core/src/beading/*` — B's scope (trait extension); C reads `mod.rs`/`factory.rs` read-only for the `wall_transition_angle` surface but does NOT edit any `beading/` file.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — A1/B's scope; C does not touch propagation.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — A1/A2's scope; C does not touch emission.
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs:263-389` — the un-wired whisker-dissolve helpers (dead-code mirror); read-only confirmation, do NOT wire.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` — Packet F.

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "SUMMARY of `SkeletalTrapezoidation.cpp:811-862` `filterNoncentralRegions` — explicitly ask for the promote-back condition (same/±1-bead-count within 0.4 mm) + the bead-count copy rule; return ≤ 200 words, no code unless asked" — purpose: confirm Step 2's port.
- "SUMMARY of `SkeletalTrapezoidation.cpp:633` call site — confirm `filterNoncentralRegions` is called unconditionally after `updateBeadCount`; return FACT (≤ 5 lines)" — purpose: confirm Step 2's call-site ordering.
- "SUMMARY of `SkeletalTrapezoidation.cpp:716-730` dead `filterCentral` — ask for the self-contradictory condition explicitly (to confirm PNP's `centrality.rs:263-389` helpers correctly mirror dead code, NOT to wire them); return ≤ 200 words" — purpose: confirm the gotcha.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1 (N1 stays green).
- "Run `cargo test -p slicer-core --features host-algos --test arachne_filter_noncentral_regions --nocapture`; return FACT pass/fail" — purpose: validate AC-2 (dumbbell).
- "Run `rg -q 'std::f64::consts::PI' crates/slicer-core/src/arachne/pipeline.rs; test $? -eq 1`; return FACT pass (exit 1 = no match)" — purpose: validate AC-N1 (π hack gone).
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N2/N4/N3 stay green)" — purpose: gate C didn't regress A2/B.
- "Run `cargo test -p slicer-core --features host-algos --test centrality 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.
- "Find all callers of `filter_central`; return LOCATIONS" — purpose: confirm the angle-threading call-site update is complete.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. C's surface is `slicer-core`-internal; no WIT/IR change. C does NOT extend the `BeadingStrategy` trait (B owns the trait extension); C only threads the already-existing `wall_transition_angle`.
- WIT boundary considerations: **none**. No WIT/IR schema change. The host boundary marshals `Vec<ExtrusionLine>`, not centrality parameters.
- Determinism: C's changes preserve determinism (the configured angle is a fixed config value; `filter_noncentral_regions` is a deterministic graph walk with index-ordered tiebreaks).

## Locked Assumptions and Invariants

- `wall_transition_angle` already exists on the `BeadingStrategy` trait at `beading/mod.rs:93` and is threaded via `BeadingFactoryParams` at `factory.rs:92,157,192`. C does NOT add a duplicate; C only changes the `filter_central` call site from a hardcoded π to `strategy.wall_transition_angle()` (or `beading_params.wall_transition_angle`).
- The 0.1× filter-dist fudge is deleted entirely; `to_centrality_params` passes `params.transition_filter_dist * UNITS_PER_MM` directly (no `* 0.1`).
- `filterNoncentralRegions`'s 0.4 mm distance is in slicer units (4000 units; 1 unit = 100 nm per `docs/08_coordinate_system.md`).
- C must NOT wire the whisker-dissolve `filterCentral` (dead code upstream).
- C keeps N1, N2, N3, N4 red tests GREEN (gated).
- C's removal of the π hack changes runtime behavior for every polygon (configured 10° is now the gate, not π); the centrality fixture re-baseline records the drift.
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.
- `filter_noncentral_regions` is called unconditionally after `assign_bead_counts` in `pipeline.rs`, mirroring `:633`'s "after `updateBeadCount`" ordering.

## Risks and Tradeoffs

- **Removing the π hack changes centrality for every polygon.** With the configured 10°, a square's diagonal spokes (`dR/dD = sin 45° ≈ 0.707`) become non-central (canonical); this is the intended behavior change, but it could surface latent bugs in A1/A2's junction placement that the π hack masked. The N1 red tests gate this (AC-1 must stay green).
- **`filterNoncentralRegions` port risk.** The 0.4 mm hardcoded distance and the same/±1-bead-count condition must be exact; a mis-port could over-promote (fragmenting regions that should stay separate) or under-promote (leaving the fragmentation N6 flags). The dumbbell test (AC-2) is the oracle.
- **Centrality fixture re-baseline may mask regressions.** The self-capture pattern locks in *this* implementation's behavior, not OrcaSlicer ground truth. The N1 red tests + the dumbbell test are the real parity oracles.
- **Bisect across A2→C boundary.** Between A2 and C, the π hack is still in place; C's commit message must record the boundary and the behavior change (configured angle now active).

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — `filter_noncentral_regions` port + dumbbell test + fixture re-baseline, the bulk of the work).
- Highest-risk dispatch: the `filterNoncentralRegions` SUMMARY — its return could blow budget if it returns code instead of prose. Required return format: `SUMMARY ≤ 200 words, no code unless asked`.

## Open Questions

- [FWD] Should `filter_noncentral_regions` take a configurable distance or hardcode 0.4 mm (4000 units)? The canonical `SkeletalTrapezoidation.cpp:811-862` hardcodes 0.4 mm; C should match (hardcode 4000 units) unless a delegated SUMMARY reveals a config key.
- [FWD] Does `filter_noncentral_regions` need the strategy (for bead-count comparison) or just the graph? The canonical function reads `bead_count` from vertices; C's signature should take `&mut SkeletalTrapezoidationGraph` only, matching `filter_central`'s convention.
- [FWD] Should the dumbbell test assert on the stitched output (`run_arachne_pipeline` result) or the raw graph (centrality + bead_count fields)? The stitched output is the user-visible observable (single ring pair); the raw graph is more direct but less faithful. Prefer the stitched output (matches the N1 red tests' pattern).

None activation-blocking.