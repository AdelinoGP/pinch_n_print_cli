# Design: support-planner-to-buildplate-pruning

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs::PlannedSupportNode` — add `to_buildplate: bool` field.
  - Contact creation block in `plan_for_object` (around the `contacts_by_layer[layer_idx].push(...)` calls) — set `to_buildplate` based on the contact XY's relationship to the object's projected footprint at that layer.
  - Propagation pruning block — at the post-`clamp_to_avoidance` step, branch on `to_buildplate`: if `true` AND target is inside `collision_polys`, drop the node.
  - `on_print_start` — read `support_on_build_plate_only` config (default false), store on `SupportPlanner` struct.
  - The reject-at-creation block honoring `support_on_build_plate_only = true`.
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` (new) — AC-2, AC-3, AC-4, AC-N1.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — extended with AC-6 invariant.
  - Goldens regenerated (small shift).
  - `docs/specs/support-modules-orca-port.md` §Validation Strategy — invariant list extended.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `to_buildplate` flag does NOT cross WIT boundaries. It is internal to the planner; `SupportPlanIR.entries` carries only the resulting geometry.
- The "object's projected footprint at layer L" is the same data the existing `LayerCollisionCache.collision_polys[L]` already carries (the union of `SupportGeometryView.outlines`). The packet reuses this rather than recomputing.
- The pruning is bounded by the existing prune-on-drop semantics — a pruned node's ancestor chain is not retroactively removed. This packet's Step 3 documents the semantics: a node going into `drop` state means its propagation stops at that layer; segments emitted above (with `dist_to_top` ≤ the prune layer's `dist_to_top`) remain.

## Code Change Surface

- Selected approach: minimal struct addition + targeted branch additions at three locations (contact creation, propagation prune check, config plumbing).
- Exact functions/structs/tests to change:
  - `PlannedSupportNode` struct — one new field.
  - `plan_for_object` — contact-set initialization, propagation prune branch, and (when `support_on_build_plate_only`) early-reject.
  - `SupportPlanner` struct — one new field (`support_on_build_plate_only: bool`).
  - `on_print_start` — config parse.
  - `support-planner.toml` — new config schema entry.
  - Tests + harness + goldens.
- Rejected alternatives:
  - **Make `to_buildplate` an `Option<bool>` to support `to_model` case later** — rejected: future-proofing for `to_model` is out of scope; binary flag matches Orca's `bool` and is simpler.
  - **Compute the projected footprint per-node** — rejected: O(nodes × layers); reusing the layer-keyed cache is O(layers).

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
- "Run xtask golden-regen; return FACT."
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE."

## Data and Contract Notes

- IR contract: unchanged. The `to_buildplate` flag is internal.
- WIT: none.
- Determinism: preserved.
- The config key `support_on_build_plate_only` is per-print, not per-region.

## Locked Assumptions and Invariants

- All previous wedge invariants continue to hold.
- Default config (`support_on_build_plate_only = false`) preserves current planner behavior (no contact rejection at creation; existing `clamp_to_avoidance` drop semantics unchanged for `to_buildplate = false` nodes).
- The pruning fires ONLY when `to_buildplate = true` AND the clamped target is inside `collision_polys`.

## Risks and Tradeoffs

- **Risk**: the projected-footprint lookup is `collision_polys[L]` — but per Orca semantics, "object's projected footprint" might differ (Orca uses `m_layer_outlines_below[layer_nr]`, the *cumulative* union of outlines from 0..layer_nr, not the per-layer outline). **Mitigation**: confirm via Step 1 sub-agent SUMMARY; if cumulative is required, compute lazily.
- **Risk**: aggressive pruning may emit fewer support branches than the user expects, causing print failures. **Mitigation**: tested under the build-plate-only invariant (AC-7); the default config still leaves `to_buildplate = false` nodes untouched.
- **Risk**: existing wedge goldens shift slightly. **Mitigation**: AC-8 tolerance check captures the shift; if > 10% drift, the shift is intentional and the goldens are re-anchored with documentation in the commit message.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`

## Open Questions

- `[FWD]` Per-layer footprint (`collision_polys[L]`) vs. cumulative-below (`m_layer_outlines_below[L]`) — confirm via Step 1 SUMMARY dispatch. Implementer chooses based on Orca behavior; either is implementable inside Step 2 scope.
