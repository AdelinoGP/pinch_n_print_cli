---
status: implemented
packet: 128_paint-segmentation-shell-index-invariant
task_ids:
  - TASK-253
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract — Packet 128: Paint-Segmentation Shell-Index Invariant

## Goal

Scope the paint_segmentation shell-depth propagation by `object_id` so each object's regions carry their own per-object depths, and lock the per-object-per-layer invariant with a `debug_assert!`, a multi-object mixed-height test, and a propagation-block doc contract.

## Scope Boundaries

This packet fixes the shell-depth propagation block in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (currently a per-layer-global `Option::or` accumulator) to group by `SlicedRegion.object_id`, and propagates that per-object state into the Phase 6/7 None arm. It adds a single end-of-function `debug_assert!` enforcing per-object agreement, a propagation-block invariant doc comment, two structural invariant tests (multi-object mixed-height and single-object multi-colour), and two `debug_assert!`-behaviour negative tests. Out of scope: the `SlicedRegion` schema (unchanged), guest WASM, OrcaSlicer parity, the `region_partition.rs` fallback (already landed), and the existing 4-colour e2e gate (referenced, not edited).

## Prerequisites and Blockers

- Depends on: TASK-250 and TASK-252 (both `[x]` in docs/07) — this packet is their deferred follow-up, tracked as the new TASK-253.
- Unblocks: correct multi-object mixed-height prints where shell-depth-driven roles (ironing, solid-fill role, `only_one_wall_top`) currently fire on the wrong object.
- Activation blockers: none. No other packet is `status: active`. All grilling-opened questions resolved.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** a two-object mixed-height `LayerPlanIR` (a 10 mm cube and a 50 mm cube on one build plate), **when** `execute_paint_segmentation` runs, **then** at every layer where the short object's region has `top_shell_index == Some(0)` and the tall object's region has `top_shell_index == None`, the returned `SliceIR` preserves that distinction (tall object's regions are NOT stamped `Some(0)`). | `cargo test -p slicer-core --lib --features host-algos paint_segmentation::driver_v2_tests::shell_index_invariant_multi_object -- --nocapture 2>&1 | tail -5`
- **AC-2. Given** the propagation block at `mod.rs:887-916`, **when** the accumulator runs, **then** `saved_top_idx` / `saved_bottom_idx` are keyed per `ObjectId` (a `HashMap` or equivalent per-object grouping), not a single layer-global scalar. | `rg -n "HashMap<.*ObjectId>|saved_top_idx.*insert|saved_top_idx\.get" crates/slicer-core/src/algos/paint_segmentation/mod.rs | head -10`
- **AC-3. Given** the Phase 6/7 None arm at `mod.rs:1252-1296`, **when** it constructs a new `SlicedRegion`, **then** it reads `saved_top_idx` / `saved_bottom_idx` by the new region's `object_id` and does NOT reference `working[l].regions` for shell-index lookup. | `rg -n "working\[.*\]\.regions.*(top|bottom)_shell_index" crates/slicer-core/src/algos/paint_segmentation/mod.rs`
- **AC-4. Given** a single-object 3-colour partial-paint `LayerPlanIR`, **when** `execute_paint_segmentation` runs, **then** every region on every layer shares the same `top_shell_index` and `bottom_shell_index` (the degenerate single-object case of the per-object invariant). | `cargo test -p slicer-core --lib --features host-algos paint_segmentation::driver_v2_tests::shell_index_invariant_multi_color -- --nocapture 2>&1 | tail -5`

## Negative Test Cases

- **AC-N1. Given** a hand-built `SliceIR` with two regions of the SAME `object_id` on one layer having mismatched `top_shell_index` (`Some(0)` vs `Some(2)`), **when** the invariant helper runs under `#[cfg(debug_assertions)]`, **then** it panics via `debug_assert!`. | `cargo test -p slicer-core --lib --features host-algos paint_segmentation::driver_v2_tests::shell_index_invariant_assert_fires -- --nocapture 2>&1 | tail -5`
- **AC-N2. Given** a hand-built `SliceIR` with two regions of DIFFERENT `object_id` on one layer having different `top_shell_index` (`Some(0)` vs `None`), **when** the invariant helper runs under `#[cfg(debug_assertions)]`, **then** it does NOT panic (cross-object disagreement is legal). | `cargo test -p slicer-core --lib --features host-algos paint_segmentation::driver_v2_tests::shell_index_invariant_cross_object_legal -- --nocapture 2>&1 | tail -5`
- **AC-N3. Given** the packet's changes, **when** `cargo clippy --workspace --all-targets -- -D warnings` runs, **then** it exits 0 (no warnings). | `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`
- **AC-N4. Given** the existing 4-colour ironing e2e test, **when** `cube_4color_ironing_per_painted_top_color_tdd` runs, **then** it passes unchanged (single-object print; the per-object scope fix does not regress it). | `cargo test -p slicer-runtime --test executor -- cube_4color_ironing_per_painted_top_color_tdd --nocapture 2>&1 | tail -5`
- **AC-N5. Given** a two-object slice where a painted color has NO existing region on a layer for its owning object (the Phase 6/7 None-arm path), **when** `execute_paint_segmentation` runs, **then** the freshly-created region is stamped with the painted color's source `object_id` (not `working[l].regions.first().object_id` — a layer-global pick that is wrong on multi-object layers). | `cargo test -p slicer-core --lib --features host-algos paint_segmentation::driver_v2_tests::shell_index_invariant_none_arm_multi_object -- --nocapture 2>&1 | tail -5`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --lib --features host-algos paint_segmentation::driver_v2_tests::shell_index_invariant_multi_object -- --nocapture`

## Authoritative Docs

- `docs/02_ir_schemas.md` — load directly only for the `SlicedRegion` section (verify `object_id: ObjectId`, `top_shell_index: Option<u8>`, `bottom_shell_index: Option<u8>` field names and the depth semantics); delegate the rest if > 300 lines.
- `docs/07_implementation_status.md` — delegate the TASK-253 edit; the implementer must NOT load the full backlog into context (the docs/07 mutation is a one-line append via a worker dispatch).
- `CONTEXT.md` — load directly to append the **Shell depth** glossary entry (deferred write crystallized during grilling; lands in the same packet as the code fix).

## Doc Impact Statement

1. Specific doc sections this packet adds or modifies, with one verification grep per section so closure can be checked mechanically:

   - `CONTEXT.md` §Terms — `rg -q '### Shell depth' CONTEXT.md`
   - `docs/07_implementation_status.md` (TASK-253 row append) — `rg -q 'TASK-253' docs/07_implementation_status.md`

   The `CONTEXT.md` **Shell depth** entry and the `docs/07` TASK-253 row must land in the same packet (not deferred); the verification greps above are appended to the acceptance gate and the `spec-review` skill checks them before the packet may flip to `status: implemented`.

   No `docs/02_ir_schemas.md` edit is required: the `SlicedRegion` schema is unchanged (the fields already exist with correct types and doc comments); this packet only changes how the propagation block populates them.

## Context Discipline Note

<!-- snippet: context-discipline -->
This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- [AC-1/AC-4/AC-N1/AC-N2 verification commands] — Specified: `paint_segmentation::tests::<name>` (no feature flag) | Implemented: `paint_segmentation::driver_v2_tests::<name>` with `--features host-algos` | Reason: mod.rs only declares `mod driver_v2_tests` (line 1562); the spec's path ran 0 tests. Acceptance ceremony uses the corrected path; all four tests green.
- [Design.md / implementation-plan.md test-module name] — Specified: `#[cfg(test)] mod tests` | Implemented: `#[cfg(test)] mod driver_v2_tests` (mod.rs:1562) | Reason: matches the file's existing test-module name; spec was stale.
- [AC-3 None-arm semantics + AC-N5 added] — Specified: the Phase 6/7 None arm reads `object_id` from the new region's owning object; no test for the multi-object None-arm path | Implemented: the None arm (mod.rs:1283-1317) reads `object_id` from `source_objects` (a `BTreeSet<String>` tracked per `(sname, value)` painted color in the extended `painted_subsets` value type at mod.rs:994-1001), then looks up a same-object region on the layer to inherit `region_id` and shell indices. Added AC-N5 + regression test `shell_index_invariant_none_arm_multi_object` (mod.rs:3256+) covering the multi-object None-arm path. | Reason: the original None arm picked `working[l].regions.first().object_id` — a layer-global `first()` pick that would stamp the wrong `object_id` on a freshly-created region in a multi-object layer. The fix threads source `object_id` through `painted_subsets` so the None arm sources ids from the painted color's owning object, not from a layer-global pick.