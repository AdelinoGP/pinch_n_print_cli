# Requirements — Packet 128: Paint-Segmentation Shell-Index Invariant

## Packet Metadata

- Grouped task IDs:
  - `TASK-253` (new — the deferred follow-up to TASK-250 / TASK-252)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M` (one M step for the propagation+None-arm+assert refactor, two S steps for tests, one S step for the docs/07 + CONTEXT.md writes)

## Problem Statement

Packet `126_mmu-painted-cube-parity` shipped an ad-hoc fix for a Phase 6/7 None arm that left `top_shell_index = None` on freshly-created regions (post-mortem root cause: the None arm created a `SlicedRegion { ..Default::default() }` outside the propagation block's harmonisation loop). That fix is correct for the single-object case and the 233/233-green suite passed — but a grilling session found the deeper root cause one function up: the propagation block at `mod.rs:887-916` harmonises shell depths **per-layer-global** via `saved_top_idx = saved_top_idx.or(r.top_shell_index)` (first-`Some`-wins) across ALL regions on a layer, with no `object_id` guard.

The producer (`slice_postprocess_prepass.rs:362-373`) computes shell depths **per-object** — each object's regions get depths from that object's own shell zone. On a multi-object mixed-height build (e.g. a 10 mm cube and a 50 mm cube on one plate), at a layer near the short cube's top, the short cube's region is `Some(0)` (exposed) and the tall cube's region is `None` (deep interior, outside any shell zone of that object). The per-layer-global `.or()` picks the short cube's `Some(0)` and stamps it onto the tall cube's regions — causing `top-surface-ironing` (lib.rs:321) to iron the tall cube's mid-body, `gyroid-infill` (lib.rs:189) to route it to the exposed-solid-fill role, and `only_one_wall_top` (classic-perimeters lib.rs:198, arachne-perimeters lib.rs:204) to drop walls on the wrong object.

Why this matters now: no existing paint_segmentation test uses more than one object (the `phase6_7_none_arm_stamps_shell_index_on_new_region` fixture at mod.rs:2391-2581 creates a single `"obj1"`), so the latent cross-object corruption is uncaught by a green suite. The ad-hoc Phase 6/7 fix was a symptom treatment; this packet closes the underlying scope bug and locks the correct invariant so it cannot silently regress.

## In Scope

- Scope the propagation block at `crates/slicer-core/src/algos/paint_segmentation/mod.rs:887-916` to accumulate `saved_top_idx` / `saved_bottom_idx` per `ObjectId` (HashMap or equivalent), instead of a single layer-global scalar.
- Update the propagation stamping (mod.rs:912-913) to write per-object values onto `new_regions` by `object_id`.
- Update the Phase 6/7 None arm at `mod.rs:1252-1296` to look up the per-object hoisted value by the new region's `object_id` (replacing the current sibling-query of `working[l].regions`).
- Add one end-of-function `debug_assert!` at mod.rs ~1332 (before `Ok(Arc::new(working))` at line 1333) that groups `working[l].regions` by `object_id` and asserts `top_shell_index` / `bottom_shell_index` agree within each object group. `#[cfg(debug_assertions)]` gated, following `compose_variants.rs:166-179` precedent.
- Add an `// INVARIANT:` doc comment at the propagation block (mod.rs:887) stating: shell depths are per-object; cross-object regions on a layer are NOT required to agree; any new `SlicedRegion` construction post-propagation must source its depths from the per-object accumulator by `object_id`; the four Phase 5 sites at mod.rs:724-802 are pre-propagation and harmonised by this block.
- Add structural invariant test `shell_index_invariant_multi_object` — two cubes of different heights (10 mm + 50 mm); at a layer near the short cube's top, assert short cube's regions are `Some(0)` and tall cube's regions are `None` (not harmonised to `Some(0)`).
- Add structural invariant test `shell_index_invariant_multi_color` — single-object 3-colour partial-paint; assert all regions on every layer share the same `top_shell_index` / `bottom_shell_index`.
- Add `debug_assert!`-fires negative test `shell_index_invariant_assert_fires` — same-object mismatched depths panic (via `std::panic::catch_unwind`), following `colorize.rs:652-654` precedent.
- Add `debug_assert!`-does-not-fire negative test `shell_index_invariant_cross_object_legal` — different-object different depths do NOT panic (the assertion that prevents the original wrong invariant from being re-introduced).
- Add the **Shell depth** glossary entry to `CONTEXT.md` §Terms (deferred write crystallized during grilling).
- Add `TASK-253` row to `docs/07_implementation_status.md` (one-line append; the implementer must NOT load the full backlog — delegate the edit).

## Out of Scope

- `SlicedRegion` schema change — none. `object_id: ObjectId` (slice_ir.rs:1228), `top_shell_index: Option<u8>` (1249), `bottom_shell_index: Option<u8>` (1252) already exist with correct types and doc comments. No WIT or schema bump.
- Guest WASM — this is a host-only kernel change; no path under `modules/core-modules/*/src/` or `crates/slicer-wasm-host/test-guests/` is touched, so `cargo xtask build-guests --check` is not in the gate.
- OrcaSlicer parity — internal quality fix; no `OrcaSlicerDocumented/` reference.
- The `region_partition.rs` empty-wall-inset fallback (already landed in packet 127 / TASK-252) — referenced not edited.
- The existing e2e test `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs` — kept as a gate (AC-N4), not modified.
- An ADR on the harmonisation-scope decision — deferred (per user call) until the fix lands and the scope choice is empirically validated by the multi-object test.

## Authoritative Docs

- `docs/02_ir_schemas.md` — delegate unless the implementer needs the `SlicedRegion` section only; the field names (`object_id`, `top_shell_index`, `bottom_shell_index`) and depth semantics are already confirmed at `crates/slicer-ir/src/slice_ir.rs:1228,1247,1249,1252`. Expected: small section; load directly if ≤ 300 lines total or range-read the `SlicedRegion` block.
- `docs/07_implementation_status.md` — delegate the TASK-253 append; the implementer must NOT load the full backlog. The expected edit is a single line appended at the relevant task-id position, status `[ ]`.
- `CONTEXT.md` — load directly; append the **Shell depth** glossary entry under §Terms (deferred write from grilling). Small file (164 lines).

## Acceptance Summary

- Positive cases: `AC-1` (multi-object mixed-height preservation — the test no existing fixture exercises), `AC-2` (per-object accumulator shape), `AC-3` (Phase 6/7 None arm consumes per-object value), `AC-4` (single-object multi-colour regression — degenerate case of the per-object invariant).
- Negative cases: `AC-N1` (`debug_assert!` fires on same-object mismatch), `AC-N2` (`debug_assert!` does NOT fire on cross-object mismatch — the guard against re-introducing the wrong per-layer-global invariant), `AC-N3` (clippy clean), `AC-N4` (4-colour e2e gate green — the ad-hoc fix's regression test is not regressed by the deeper fix).
- Cross-packet impact: this packet builds on `126_mmu-painted-cube-parity` (implemented) — it does NOT supersede 126; 126's ad-hoc fix remains correct for the single-object case and stays in tree as the Phase 6/7 None-arm logic, now consuming per-object state instead of a sibling query. No packet is blocked or unblocked beyond the multi-object correctness gap this packet closes.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the 3 gate commands; this section is the authoritative list with delegation hints.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_object --nocapture` | AC-1: multi-object mixed-height shell-depth preservation | FACT pass/fail; SNIPPETS ≤ 20 lines on failure (assertion + fixture values) |
| `rg -n "HashMap<.*ObjectId>\|saved_top_idx.*insert\|saved_top_idx\.get" crates/slicer-core/src/algos/paint_segmentation/mod.rs \| head -10` | AC-2: per-object accumulator shape | LOCATIONS — expect ≥ 1 hit showing per-object keying |
| `rg -n "working\[.*\]\.regions.*(top\|bottom)_shell_index" crates/slicer-core/src/algos/paint_segmentation/mod.rs` | AC-3: Phase 6/7 None arm no longer re-queries `working[l].regions` | FACT — expect 0 matches in the 1252-1296 range |
| `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_color --nocapture` | AC-4: single-object multi-colour regression | FACT pass/fail |
| `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_assert_fires --nocapture` | AC-N1: `debug_assert!` fires on same-object mismatch | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_cross_object_legal --nocapture` | AC-N2: `debug_assert!` does NOT fire on cross-object mismatch | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | AC-N3: clippy clean | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- cube_4color_ironing_per_painted_top_color_tdd --nocapture` | AC-N4: 4-colour e2e gate green | FACT pass/fail |
| `rg -q '### Shell depth' CONTEXT.md` | Doc Impact: Shell depth glossary entry present | FACT exit 0 |
| `rg -q 'TASK-253' docs/07_implementation_status.md` | Doc Impact: TASK-253 row appended | FACT exit 0 |

All verification commands are delegation-friendly (small, parseable output) so the implementer and reviewer can dispatch them to a sub-agent and consume only a FACT or SNIPPETS return.

## Step Completion Expectations

- Cross-step invariant: the propagation-block refactor (Step 1) MUST land before the tests (Steps 2–3) are written — the tests assert against the new per-object behaviour, and writing them first would red against the old per-layer-global code in a way that doesn't drive the refactor (the multi-object test would fail for the wrong reason — because the fixture has two objects, not because the invariant helper is wrong). TDD order here is: refactor the propagation scope, THEN add the invariant helper + tests that lock it.
- Step ordering rationale: Step 1 (propagation + None arm + doc comment + invariant helper) is the load-bearing change; Step 2 adds the multi-object test that proves it; Step 3 adds the single-object and debug_assert negative tests that lock it; Step 4 is the doc writes (CONTEXT.md + docs/07) which must land before the acceptance gate greps can fire.
- Cross-step shared scratch state: none. The invariant helper used by the `debug_assert!` (Step 1) and tested by the negative tests (Step 3) is the same `fn` — it is defined once in Step 1 and invoked by Step 3's tests via `std::panic::catch_unwind`.

## Context Discipline Notes

- `mod.rs` is a large file (~2581 lines). The implementer MUST range-read it: lines 600-916 (propagation block context), 1252-1333 (Phase 6/7 None arm + function return), 2391-2581 (existing test to model the new tests on). Do NOT load the full file.
- `slice_postprocess_prepass.rs` is out-of-bounds for edits but the implementer may range-read lines 150-400 to confirm the per-object producer semantics (already verified during grilling — re-confirm only if a sub-agent's FACT contradicts the packet's Problem Statement).
- Likely temptation read: the four Phase 5 `SlicedRegion { ..Default::default() }` sites at mod.rs:724-802 — these are pre-propagation and do NOT need editing; skip them unless AC-2 / AC-3 grep surfaces an unexpected reference.
- Heaviest dispatch: the AC-1 multi-object test run. Require the sub-agent to return SNIPPETS on failure with the fixture's per-object `top_shell_index` values so the implementer can diagnose without re-running.