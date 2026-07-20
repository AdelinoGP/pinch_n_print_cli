# Design: support-planner-to-buildplate-pruning

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs::PlannedSupportNode` (line 92) — add `to_buildplate: bool` field.
  - Contact creation block in `plan_for_object` (lines 380-416) — set `to_buildplate` based on the contact XY's relationship to `LayerCollisionCache.collision_polys[L]` for the object at that layer.
  - Propagation pruning block (lines 711-723, the `point_in_any_expoly(collision_polys, cx, cy)` branch) — extend with: when `to_buildplate = true` AND clamped target is inside `collision_polys`, drop the node. The existing collision-prune for ALL nodes is preserved; this is an additional prune trigger.
  - `on_print_start` (line 114) — read `support_on_build_plate_only` config (default `false`), store on `SupportPlanner` struct as a new field.
  - The reject-at-creation block honoring `support_on_build_plate_only = true` (added to the contact-creation loop, lines 380-416).
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` (new) — AC-2, AC-3, AC-4, AC-N1, AC-N2.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — extended with AC-6 invariant (the 10th).
  - Goldens regenerated (small shift).
  - `docs/specs/support-modules-orca-port.md` §Validation Strategy — invariant list extended.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `to_buildplate` flag does NOT cross WIT boundaries. It is internal to the planner; `SupportPlanIR.entries` carries only the resulting geometry.
- The "object's projected footprint at layer L" is the same data the existing `LayerCollisionCache.collision_polys[L]` already carries (the union of `SupportGeometryView.outlines` for that layer). The packet reuses this rather than recomputing.
- The pruning is bounded by the existing prune-on-drop semantics — a pruned node's ancestor chain is not retroactively removed. This packet's Step 3 documents the semantics: a node going into `drop` state means its propagation stops at that layer; segments emitted above (with `dist_to_top` ≤ the prune layer's `dist_to_top`) remain.
- The new `to_buildplate = true` prune condition is ADDITIVE to the existing collision-prune — it does not replace or relax the existing rule.

## Code Change Surface

- Selected approach: minimal struct addition + targeted branch additions at three locations (contact creation, propagation prune check, config plumbing).
- Exact functions/structs/tests to change:
  - `PlannedSupportNode` struct (line 92) — one new field.
  - `plan_for_object` (lines 380-416) — contact-set initialization (set `to_buildplate` per contact); (lines 711-723) propagation prune branch (additional `to_buildplate = true` prune); (lines 380-416) reject-at-creation when `support_on_build_plate_only = true`.
  - `SupportPlanner` struct (line 68-90) — one new field (`support_on_build_plate_only: bool`).
  - `on_print_start` (line 114) — config parse.
  - `support-planner.toml` — new config schema entry.
  - Tests + harness + goldens.
- Rejected alternatives:
  - **Make `to_buildplate` an `Option<bool>` to support `to_model` case later** — rejected: future-proofing for `to_model` is out of scope; binary flag matches Orca's `bool` and is simpler.
  - **Compute the projected footprint per-node** — rejected: O(nodes × layers); reusing the layer-keyed cache is O(layers).
  - **Drop the existing collision-prune for `to_buildplate = false` nodes** — rejected: relaxing the existing rule is out of scope; the packet's rule is strictly a *tightening* for `to_buildplate = true` nodes.

## Files in Scope (read + edit)

- `modules/core-modules/support-planner/src/lib.rs` — struct field + contact creation + propagation prune + on_print_start + reject-at-creation.
- `modules/core-modules/support-planner/support-planner.toml` — config schema entry.
- `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` — new test file.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — new invariant test.
- `docs/specs/support-modules-orca-port.md` — invariant list extension.
- Goldens regenerated.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C5 — directly.
- `support-planner.toml` — current state.
- `support-planner/src/lib.rs` — range-read contact creation + propagation blocks.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- Other modules — not edited.
- `target/`, generated code.
- Other config files; user profiles.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `TreeSupport::drop_nodes` `unsupported_branch_leaves` flow; return SUMMARY ≤ 200 words."
- "Summarize OrcaSlicer `generate_contact_points` `to_buildplate` initialization; return SUMMARY ≤ 200 words."
- "Run `cargo test -p support-planner --test to_buildplate_tdd`; return FACT per-test."
- "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT per-test."
- "Run `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance`; return FACT (regen happened)."
- "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail."
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE."

## Data and Contract Notes

- IR contract: unchanged. The `to_buildplate` flag is internal.
- WIT: none.
- Determinism: preserved.
- The config key `support_on_build_plate_only` is per-print, not per-region.

## Locked Assumptions and Invariants

- All previous wedge invariants (7 from packet 119 + 1 curvature from packet 121 + 1 symmetry from packet 122 = 9 total) continue to hold.
- Default config (`support_on_build_plate_only = false`) preserves current planner behavior (no contact rejection at creation; existing `clamp_to_avoidance` drop semantics unchanged for `to_buildplate = false` nodes).
- The new pruning fires ONLY when `to_buildplate = true` AND the clamped target is inside `collision_polys`.
- The `PlannedSupportNode` field addition is an internal struct change — no IR, no WIT, no manifest.

## Risks and Tradeoffs

- **Risk**: the projected-footprint lookup uses `collision_polys[L]` (per-layer outline union) — but per Orca semantics, "object's projected footprint" might differ (Orca uses `m_layer_outlines_below[layer_nr]`, the *cumulative* union of outlines from 0..layer_nr, not the per-layer outline). **Mitigation**: confirm via Step 1 sub-agent SUMMARY; if cumulative is required, compute lazily from the existing data.
- **Risk**: aggressive pruning may emit fewer support branches than the user expects, causing print failures. **Mitigation**: tested under the build-plate-only invariant (AC-7); the default config still leaves `to_buildplate = false` nodes untouched.
- **Risk**: existing wedge goldens shift slightly because the new prune trigger may fire for some `to_buildplate = true` nodes that were previously surviving. **Mitigation**: AC-8 tolerance check captures the shift; if > 10% drift, the shift is intentional and the goldens are re-anchored with documentation in the commit message.
- **Tradeoff**: a `PlannedSupportNode` field addition forces every struct-literal site for `PlannedSupportNode` to add a `to_buildplate:` line. The implementer audits every site with `rg 'PlannedSupportNode \{' modules/core-modules/support-planner/src/lib.rs` and updates each one.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 — struct field + 3 site updates).
- Highest-risk dispatch: AC-8 golden regen (small shift expected).

## Open Questions

- `[FWD]` Per-layer footprint (`collision_polys[L]`) vs. cumulative-below (`m_layer_outlines_below[L]`) — confirm via Step 1 SUMMARY dispatch. Implementer chooses based on Orca behavior; either is implementable inside Step 2 scope.
