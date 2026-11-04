# Lightning corpus consolidation: share the 5-case captures across the 10 tests

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 114](114-e2e-bucket-latency.md)'s adversarial audit
(2026-11-04, [`evidence/t114-e2e-latency/AUDIT.md`](../evidence/t114-e2e-latency/AUDIT.md)):
`lightning_migration_tdd` is the e2e bucket's largest module (~1,411 s measured
for 6 of its 10 tests, ~39 min extrapolated) and its corpus tests re-take the
same 5-case × 2-arm captures.

## Question

Can `lightning_migration_tdd`'s active tests share **one** corpus capture per
`(case_id, arm)` — instead of re-running 27 visual captures across 9 tests — so
the module's active invocation count falls from **41 to 32** with **every
assertion preserved**, and what is the measured before/after module wall?

## Context — the audit's plan

The per-test table is in
[`evidence/t114-e2e-latency/AUDIT.md`](../evidence/t114-e2e-latency/AUDIT.md).
The short version, for
`crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`:

- **27 visual captures** (V), plus **6 preparation-only** (P), **5
  cancellation** (C), **3 full G-code** (F) = **41 active invocations**.
- Three tests re-take `anchor_bridge_classic`, WASM, `[Layer::Infill]`, all
  layers: `fresh_consumers_emit_baseline_segments`,
  `legacy_contract_is_retired`, `retirement_leaves_no_host_fallback`. One more
  pooled duplicate sits in `equivalence_rejects_corrupted_baseline`.
- The tap-superset corpus capture (`[Layer::Slice, Layer::Infill]`, all layers,
  no views) already obtained by `module_plan_matches_recorded_baseline` can back
  those infill-only assertions: the helpers filter by `CapturedIr` type.

**Consolidation:** make `module_plan_matches_recorded_baseline` the corpus
orchestrator. Per case, run one WASM `[Slice, Infill]` capture, one native
capture, one metadata-only preparation capture and one cancel-after-publication
run. Apply the existing assertion blocks as helpers to those observations.
Move the zero-slice blocks (artifact decode, digest arithmetic, comparator
controls, retirement text, compile witnesses, selection-builder/store disposal)
to contract/unit scope — they need no slice.

| Component | Before | After |
| --- | ---: | ---: |
| Module-plan corpus | 10 | 10 |
| Fresh-consumer additional executions | 15 | 10 (P + C only) |
| Typed projection | 6 | 6 |
| Legacy positive capture | 1 | 0 |
| Comparator positive capture | 1 | 0 |
| Failure scenarios | 4 | 4 |
| Selection scenarios | 3 | 2 |
| Compile-retirement positive capture | 1 | 0 |
| **Active total** | **41** | **32** |

**Do not** also collapse the 5 preparation/cancellation legs to one, the two
`empty_plan` case/view pairs, or the comparator corruption controls: those drop
case-specific executions the audit found distinct. **Do not** replace the two
G-code neutrality runs with a visual capture: `PreparationVisualCapture` carries
no G-code.

## Also fix in the same take (audit findings, zero wall)

- `typed_projection_is_atomic_and_neutral`'s
  `scenes.windows(2).all(|pair| Arc::ptr_eq(...))` only requests layer `[0]`,
  so it is vacuous. The genuine two-layer sharing claim is
  `preparation_visual_capture_tdd::multi_layer_capture_shares_one_scene_and_one_accounting_entry`;
  either request two layers or drop the assertion with a pointer.
- `selection_override_empty_and_foreign_owner`'s
  `let participates = selection.invoke && !selection.targets.is_empty();`
  asserts test-local Boolean arithmetic after already asserting `targets.is_empty()`.
  Drop it or assert the production participation predicate.
- `Trace::max_retained_after_disposal()` uses `.unwrap_or(0)`, so a cleanup
  assertion passes when **no disposal was recorded**. Require a nonempty
  disposal population first (the cancellation block already does).
- The "foreign owner" block accesses its own live owner; it is a disposal test,
  not a cross-owner `AccessDenied` test. Narrow the claim or add the real
  negative control.
- `baseline_is_deterministic_and_strict` is 0 slices and belongs in
  contract/unit scope; moving it is optional but cleans the e2e roster.

## Acceptance gate

- `cargo test -p slicer-runtime --test e2e -- --test-threads 1 lightning_migration_tdd::`
  passes with **every** audit-listed assertion still executing (no deletion of
  a distinct claim; the audit's "assertions lost: none" list is the checklist).
- The module's active invocation count is **32** (counted from
  `run_visual`/`run_visual_native`/`run_slice*` call sites × loops), and the
  measured module wall is reported **before and after** on a quiet box.
- `verify.py` for this ticket re-derives the invocation count from source and
  from the committed before/after measurement, with negative controls.
- No production edit; no assertion weakened.

## Stop rule

One module, one consolidation. If the shared-corpus refactor cannot preserve an
assertion, stop and record which one rather than dropping it.
