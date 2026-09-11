# Design: core-support-dup-merges

## Controlling Code Paths

- Primary code paths: `slicer_core::algos::overhang_annotation::detect_support_contacts`, reached through test helper `sweep`; `slicer_core::algos::support_geometry::build_emit_schedule`; and `slicer_core::algos::support_geometry::execute_support_geometry`. Their production bodies are read-only.
- Neighboring tests/fixtures: `pillar_then_cap`, `params`, `sweep`, and `area_mm2` in `support_overhang_detection_tdd.rs`; `make_active_region`, `make_2_layer_plan`, and `make_two_object_plan` in `algo_support_geometry_tdd.rs`; the duplicate inline helpers with those same support-geometry names are deleted with their `#[cfg(test)]` module.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- This is a merge-with-union test consolidation. It may remove only `coplanar_step_does_not_hide_the_contact`, `support_geometry_emits_for_2_layer_fixture`, and the inline definition of `build_emit_schedule_two_objects_per_object_semantics`; every distinct fixture and assertion remains in a named survivor.
- Both integration targets carry `required-features = ["host-algos"]` in `crates/slicer-core/Cargo.toml` and in the census manifest. Every target command must enable `--features host-algos`; a bare narrow run is inadmissible because Cargo can skip the target and report success.
- The `support_geometry.rs` edit is confined to `#[cfg(test)] mod tests`; `support_geometry` itself is compiled only under `host-algos` in `slicer_core::algos`. The removed block is neither a doc example nor a guest-WASM input, so guest freshness is not implicated.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Preserve canonical parity supremacy: the exact `64.0 mm²` expanded-back area witness and its `< 0.1 mm²` tolerance are retained, never loosened or replaced by non-emptiness.

## Code Change Surface

- Selected approach: use two explicit survivor maps. First, move the absorbed test's RC-1 coplanarity rationale into `overhang_is_detected_once_at_the_step_layer` and delete only `coplanar_step_does_not_hide_the_contact`. Second, enrich the integration schedule survivor's messages with the inline `got` diagnostics, then delete the complete duplicate `#[cfg(test)] mod tests` from `support_geometry.rs`; keep the integration helpers and all three integration tests.
- Exact functions, tests, and assertions:
  - Overhang absorbed → survivor: `coplanar_step_does_not_hide_the_contact` → `overhang_is_detected_once_at_the_step_layer`; identical `pillar_then_cap()` / `params(45.0, 0.2)` input. Absorbed non-empty assertion is a strict subset of survivor layer equality `vec![3_usize]`; survivor also keeps `expected = 2.0 * 8.0 * 4.0` and area error `< 0.1`.
  - Inline emission absorbed → survivor: `support_geometry_emits_for_2_layer_fixture` → `emits_for_2_layer_fixture`; identical two-layer plan, `execute_support_geometry` call, `result.is_ok()`, and non-empty `SupportGeometryIR.entries` assertions.
  - Inline schedule absorbed → survivor: inline `build_emit_schedule_two_objects_per_object_semantics` → same-named integration test; identical six-layer/two-object plan, `obj-A == {1,3,5}`, and `obj-B == {0..5}` assertions. Preserve the inline `got {a_sched:?}` / `got {b_sched:?}` diagnostics in the integration survivor.
  - Distinct survivor: `empty_plan_produces_empty_support` remains unchanged with its default-plan, `result.is_ok()`, and empty-`entries` assertions.
- Rejected alternatives: retain inline wrappers (leaves duplicate coverage); move integration-only `empty_plan_produces_empty_support` inline (opposes the selected single test home); parameterize or rename survivors (unnecessary churn and breaks the approved map); alter production logic or tolerance (outside scope and violates parity supremacy); delete the source documentation header or production module docs (the twins are tests, not doc examples).

## Files in Scope (read + edit)

- `crates/slicer-core/tests/support_overhang_detection_tdd.rs` - role: overhang integration-test home; expected change: preserve the RC-1 rationale in `overhang_is_detected_once_at_the_step_layer` and delete only `coplanar_step_does_not_hide_the_contact`.
- `crates/slicer-core/tests/algo_support_geometry_tdd.rs` - role: selected support-geometry test home; expected change: retain all three tests and add the inline schedule twin's `got` diagnostics to its same-named survivor.
- `crates/slicer-core/src/algos/support_geometry.rs` - role: duplicate inline-test owner; expected change: delete only the complete `#[cfg(test)] mod tests` block, leaving production code unchanged.
- `docs/specs/test-quality-remediation-plan.md` §7 `core` row only - role: mandatory program bookkeeping after both merges; expected change: retain `partial` state, record this packet's survivor/validation evidence, and name `core-wall-sequence-dup-merges` as remaining work without changing the Packet Queue.

## Read-Only Context

- `crates/slicer-core/tests/support_overhang_detection_tdd.rs` - `rect` through `sweep` and the blocks for `overhang_is_detected_once_at_the_step_layer` / `coplanar_step_does_not_hide_the_contact` only - fixture identity and assertion subset proof.
- `crates/slicer-core/src/algos/support_geometry.rs` - `build_emit_schedule` / `execute_support_geometry` signatures and the `#[cfg(test)] mod tests` block only - symbol shape and inline-twin disposition.
- `crates/slicer-core/Cargo.toml` - the `algo_support_geometry_tdd` and `support_overhang_detection_tdd` `[[test]]` stanzas only - required feature.
- `docs/specs/test-quality-remediation-census.json` - the two matching `slicer-core` entries only - target registration and required features; it does not carry function counts.
- `docs/specs/test-quality-remediation-plan.md` - §§1–4, §5.1 DUP-CORE, §§6–7, and Packet Queue row #5/resume exports only.
- `docs/22_test_quality.md` §§1–5, ADR-0064, ADR-0065, `docs/02_ir_schemas.md` §IR 9a, and the coordinate-system ranges named in `requirements.md` - contract context only.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - delegated only; canonical `detect_overhangs` behavior named in the existing overhang test documentation.

## Out-of-Bounds Files

- Every other `crates/slicer-core/tests/**` file, including `flow_tdd.rs`, `bridge_false_site_gating_tdd.rs`, and `algo_region_mapping_tdd.rs`; these are owned by the three prior packets and no overlap exists.
- Every `crates/slicer-core/src/**` file except deletion inside the specific `#[cfg(test)] mod tests` block of `algos/support_geometry.rs`; in that file, all production statements and docs before the test module are out of bounds.
- All other crates, `modules/**`, `xtask/**`, WIT/schema/config/manifest files, existing `docs/**` outside this packet directory except the narrowly allowed §7 `core` row, sibling packet directories, generated code, vendored dependencies, `target/`, and `Cargo.lock`.
- Within `docs/specs/test-quality-remediation-plan.md`, the Packet Queue, every non-`core` ledger row, and all prose/sections outside the Step 3 §7 `core` row are out of bounds.
- Direct reads of `OrcaSlicerDocumented/**`; use the bounded delegation contract instead.

## Expected Sub-Agent Dispatches

- Question: re-derive the three pre-edit source inventories and confirm the survivor relationships have not drifted; scope: the three in-scope Rust files; return: `FACT` with `19`, `3`, `2`, the three absorbed names, and both survivor names; purpose: precondition for Steps 1–2.
- Question: confirm canonical `detect_overhangs` lower-layer growth/difference/expand-back behavior without proposing production edits; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp`; return: `SUMMARY` ≤200 words; purpose: protect the retained parity area witness.
- Question: run each feature-correct target and the closure gates; scope: `requirements.md` verification matrix; return: `FACT` pass/fail with ≤20 failure lines; purpose: census, behavior, and lint validation.

## Data and Contract Notes

- IR/manifest contracts: no shape or version changes. Tests continue to inspect `SupportGeometryIR.entries`; schedule keys remain object IDs and values remain global model-layer indices.
- WIT boundary: none.
- Determinism/scheduler constraints: the exact `BTreeSet<u32>` schedule assertions remain deterministic: `obj-A` emits at `{1,3,5}` for `support_layer_height_mm = 0.4` over `0.2 mm` layers, while `obj-B` with `0.0` emits at every layer `{0..5}`.

## Locked Assumptions and Invariants

- Overhang source count is `19 → 18`; support-geometry integration count is `3 → 3`; support-geometry inline count is `2 → 0`. Every delta is accounted by the survivor maps above.
- The overhang survivor is `overhang_is_detected_once_at_the_step_layer`; its exact layer equality subsumes the absorbed non-empty assertion on an identical fixture, and its area assertion remains stronger.
- The support-geometry survivors are `emits_for_2_layer_fixture` and the integration definition of `build_emit_schedule_two_objects_per_object_semantics`; `empty_plan_produces_empty_support` is distinct and retained.
- The census manifest's role is target/feature registration only. Fresh `-- --list` output is the per-function execution census; no packet-time count may substitute for it at implementation.
- No public symbol, new test, new file, or later-packet export is introduced.
- The implementation-time ledger remains `partial`; it records this slice and leaves `core-wall-sequence-dup-merges` as remaining core work. Packet Queue edits are not part of implementation.

## Risks and Tradeoffs

- Deleting the entire inline module could accidentally remove production code if its boundary is misidentified. The edit begins at the sole `#[cfg(test)] mod tests` and ends at that module's closing brace; production ends before it.
- A count-only merge can look correct while losing assertion strength. AC-2 and AC-4 pin exact layer, area, entry, empty-plan, and schedule assertions in the surviving integration homes.
- The external and inline schedule tests have equal operands but different failure diagnostics. Copying `got {a_sched:?}` and `got {b_sched:?}` into the integration survivor preserves the useful union without retaining duplicate registration.
- Feature-gate blindness can produce a false green. Every behavioral command enables `host-algos` and confirms named discovery before accepting the run.
- A broad documentation edit could conflate implementation evidence with queue generation. Step 3 permits only the six-cell §7 `core` row and AC-7 anchors parsing to the Ledger section.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: pre-edit survivor-map revalidation, `FACT` with the three counts plus assertion-subset result.

## Open Questions

None.
