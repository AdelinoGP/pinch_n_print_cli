# Design — Packet 128: Paint-Segmentation Shell-Index Invariant

## Controlling Code Paths

- Primary code path: `execute_paint_segmentation` at `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — the propagation block at lines 887-916, the Phase 6/7 None arm at lines 1252-1296, and the function return at line 1333 (`Ok(Arc::new(working))`).
- Neighboring tests or fixtures: existing `phase6_7_none_arm_stamps_shell_index_on_new_region` at mod.rs:2391-2581 (single-object fixture — model the new tests on its helper structure, but extend to two objects). `compose_variants.rs:166-179` (debug_assert precedent). `colorize.rs:652-654` (debug_assert-fires test precedent).
- OrcaSlicer comparison surface: none — internal quality fix, no parity.

## Architecture Constraints

- Packet-specific constraint: the propagation block's accumulator MUST be scoped by `SlicedRegion.object_id` (field at `crates/slicer-ir/src/slice_ir.rs:1228`, type `ObjectId`). The producer (`slice_postprocess_prepass.rs:362-373`) computes depths per-object; the propagation must preserve that per-object identity, not coalesce across objects on a layer.
- Packet-specific constraint: the `debug_assert!` invariant is per-object-per-layer, NOT per-layer-global. Cross-object regions on the same layer are explicitly allowed to disagree. The negative test AC-N2 exists to prevent the wrong (per-layer-global) invariant from being re-introduced.
- Packet-specific constraint: no `SlicedRegion` schema change. The fields `object_id`, `top_shell_index`, `bottom_shell_index` already exist with correct types; this packet only changes how the propagation block populates the latter two. No WIT bump, no guest WASM rebuild.
- (No `wasm-staleness` snippet — host-only change, no path feeds guest WASM.)
- (No `coord-system` snippet — packet touches shell-depth bookkeeping, not geometry or mm↔unit conversion.)

## Code Change Surface

- Selected approach: per-object `HashMap` accumulator. Introduce a `HashMap<ObjectId, (Option<u8>, Option<u8>)>` (or two `HashMap<ObjectId, Option<u8>>`s) populated by the propagation block at mod.rs:887-916, keyed by `SlicedRegion.object_id`, with first-`Some`-wins accumulation scoped to each object. The stamping at 912-913 looks up by each `new_region.object_id`. The Phase 6/7 None arm at 1252-1296 looks up by the new region's `object_id`. The end-of-function `debug_assert!` at ~1332 groups `working[l].regions` by `object_id` and asserts within-group agreement.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `execute_paint_segmentation` (mod.rs) — propagation block scope change + None arm lookup change + end-of-function debug_assert + invariant doc comment + invariant helper fn.
  - Inline `#[cfg(test)] mod tests` (mod.rs) — four new test fns: `shell_index_invariant_multi_object`, `shell_index_invariant_multi_color`, `shell_index_invariant_assert_fires`, `shell_index_invariant_cross_object_legal`.
- Rejected alternatives:
  - **Restructure the propagation loop to objects-outermost** (keep `saved_top_idx` as a scalar reset per object) — rejected because it reorders the loop the post-mortem's "small fix" depended on and produces a larger diff; the HashMap is a smaller, more localized change.
  - **Per-layer-global invariant (the original post-mortem proposal)** — rejected after grilling found it would cement the cross-object corruption bug with a `debug_assert`; the producer is per-object, so the invariant must be per-object.
  - **ADR on the harmonisation-scope decision in this packet** — deferred per user call; the ADR lands after the multi-object test empirically validates the scope choice.

## Files in Scope (read + edit)

Target ≤ 3 primary files. This packet touches 1 primary code file + 2 doc files (small appends).

- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — role: the propagation block, Phase 6/7 None arm, function return, and inline tests all live here; expected change: scope accumulator by `ObjectId`, update None arm lookup, add debug_assert + invariant doc comment + 4 tests.
- `CONTEXT.md` — role: project glossary; expected change: append **Shell depth** entry under §Terms.
- `docs/07_implementation_status.md` — role: backlog tracker; expected change: append `TASK-253` row (one line, status `[ ]`). Delegate the edit; do not load the full file.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read large files.

- `crates/slicer-ir/src/slice_ir.rs` — read lines 1226-1300 only — purpose: confirm `SlicedRegion` fields (`object_id: ObjectId` at 1228, `top_shell_index: Option<u8>` at 1249, `bottom_shell_index: Option<u8>` at 1252) and the doc comment at 1247 ("Minimum depth (in layers, 0 = exposed) within the top shell zone. `None` outside any top shell.").
- `crates/slicer-core/src/algos/slice_postprocess_prepass.rs` — read lines 150-400 only — purpose: confirm the per-object producer semantics (timelines keyed by `(object_id, region_id)` at 362-373) so the propagation scope change is grounded in how the depths are authored.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — read lines 600-916, 1252-1333, 2391-2581 only (NOT the full ~2581-line file) — purpose: propagation block, None arm, function return, and the existing test to model new tests on.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks against this list.

- `OrcaSlicerDocumented/...` — no parity; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `modules/core-modules/*/src/`, `crates/slicer-wasm-host/test-guests/` — no guest WASM change; do not browse.
- `crates/slicer-runtime/src/region_partition.rs` — already-landed fallback from packet 127; referenced not edited.
- `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs` — the e2e gate; do not edit, only dispatch the test run for AC-N4.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_object --nocapture`; return FACT (pass) or SNIPPETS (fail with the per-object `top_shell_index` values from the fixture, ≤ 20 lines)" — purpose: validate AC-1 (Step 2).
- "Run `rg -n 'HashMap<.*ObjectId>|saved_top_idx.*insert|saved_top_idx\.get' crates/slicer-core/src/algos/paint_segmentation/mod.rs | head -10`; return LOCATIONS" — purpose: validate AC-2 (Step 1).
- "Run `rg -n 'working\[.*\]\.regions.*(top|bottom)_shell_index' crates/slicer-core/src/algos/paint_segmentation/mod.rs`; return FACT (0 matches in the 1252-1296 range) or LOCATIONS" — purpose: validate AC-3 (Step 1).
- "Run `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_assert_fires --nocapture`; return FACT pass/fail" — purpose: validate AC-N1 (Step 3).
- "Run `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_cross_object_legal --nocapture`; return FACT pass/fail" — purpose: validate AC-N2 (Step 3).
- "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail" — purpose: AC-N3 (Step 4 gate).
- "Run `cargo test -p slicer-runtime --test executor -- cube_4color_ironing_per_painted_top_color_tdd --nocapture`; return FACT pass/fail" — purpose: AC-N4 (Step 4 gate).
- "Run `rg -q '### Shell depth' CONTEXT.md`; return FACT exit 0" — purpose: Doc Impact grep (Step 4).
- "Run `rg -q 'TASK-253' docs/07_implementation_status.md`; return FACT exit 0" — purpose: Doc Impact grep (Step 4).

## Data and Contract Notes

- IR contracts touched: `SlicedRegion.top_shell_index` / `bottom_shell_index` population semantics (not the schema). The contract change is: these fields now carry per-object depths end-to-end (producer → propagation → consumers), where previously the propagation block corrupted them across objects. The fields' types and doc comments are unchanged.
- WIT boundary considerations: none. `SlicedRegion` is host-internal IR; it does not cross the WIT boundary. Guest modules read shell-index-derived *roles* (e.g. `solid_fill_role()`) via the SDK, not the raw `Option<u8>`.
- Determinism or scheduler constraints: the per-object HashMap iteration order must be deterministic (use `ObjectId` ordering or a `BTreeMap` if `ObjectId` is not `Hash`+`Ord` — confirm via sub-agent if the compiler rejects `HashMap`). The propagation block runs once per layer in `execute_paint_segmentation`; no scheduler re-entry.

## Locked Assumptions and Invariants

- `SlicedRegion.object_id: ObjectId` exists at `slice_ir.rs:1228` and uniquely identifies the owning object per region (confirmed via sub-agent FACT during grilling). No indirection needed.
- The producer computes shell depths per-object (confirmed at `slice_postprocess_prepass.rs:362-373`). The propagation block MUST preserve per-object identity; cross-object coalescence is a bug.
- The invariant to lock: **for each layer, all regions sharing the same `object_id` MUST share the same `top_shell_index` and `bottom_shell_index`**. Cross-object disagreement on a layer is legal and expected for mixed-height builds.
- The four Phase 5 `SlicedRegion { ..Default::default() }` sites at mod.rs:724-802 are pre-propagation and are harmonised by the propagation block; they do NOT need editing.
- The existing single-object tests (including `phase6_7_none_arm_stamps_shell_index_on_new_region`) remain valid because the per-object invariant degenerates to per-layer agreement when only one object exists.

## Risks and Tradeoffs

- `ObjectId` may not implement `Hash`+`Eq` (required for `HashMap` key). Mitigation: confirm via a sub-agent `cargo check` dispatch; if missing, use `BTreeMap` (needs `Ord`) or a `Vec<(ObjectId, _)>` linear scan (small N — regions per layer is typically ≤ tens).
- The HashMap allocation per layer adds a small constant cost to `execute_paint_segmentation`. Mitigation: N (regions per layer) is small; the HashMap is built and dropped per layer. No bench in the gate; if a regression is suspected post-merge, `cargo bench -p slicer-runtime --bench per_stage` is the follow-up (not this packet).
- The `debug_assert!` runs only under `#[cfg(debug_assertions)]`; release builds skip it. This matches `compose_variants.rs:166` precedent. Risk: a per-object mismatch in release is silent. Tradeoff: acceptable — the structural invariant tests catch the mismatch in CI (debug), and the cost of a runtime check in release on every layer is unjustified for a bookkeeping invariant.
- Reframing the invariant from per-layer-global (original post-mortem) to per-object (this packet) changes the contract that packet 126's ad-hoc fix assumed. Risk: 126's single-object tests still pass (the per-object invariant degenerates correctly), confirmed by AC-4 and AC-N4. No 126 test assumes cross-object sharing.

## Context Cost Estimate

- Aggregate (sum across all steps): `M` (Step 1 M + Step 2 S + Step 3 S + Step 4 S)
- Largest single step: `M` (Step 1 — propagation block + None arm + debug_assert + invariant helper + doc comment in one cohesive refactor of `execute_paint_segmentation`)
- Highest-risk dispatch: the AC-1 multi-object test run (Step 2). Required return format: FACT pass, or SNIPPETS on fail with the fixture's per-object `top_shell_index` values (≤ 20 lines) so the implementer can diagnose whether the failure is the fixture or the propagation without re-running.

## Open Questions

- [FWD] Does `ObjectId` implement `Hash`+`Eq` (for `HashMap`) or only `Ord` (requiring `BTreeMap`)? The implementer resolves this in Step 1 via a `cargo check` dispatch; if `HashMap` rejects, fall back to `BTreeMap` or a small `Vec`. Not activation-blocking — the packet's shape is the same either way.
- None activation-blocking. All grilling-opened questions (harmonisation scope, invariant shape, hoisted state shape, object identity field) are resolved and recorded in Locked Assumptions.