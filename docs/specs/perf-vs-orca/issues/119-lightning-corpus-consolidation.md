# Lightning corpus consolidation: share the 5-case captures across the 10 tests

Type: task
Label: wayfinder:task
Status: resolved (2026-11-05; **41 → 32 active invocations, 688.89 s → 620.44 s
serial module wall (−9.94 %), every assertion preserved**; see ## Answer)
Assignee: 2026-11-05 OpenCode wayfinder session — the human said "continue the
  perf vs orca map, pick up the next afk ticket" (no ticket named; 117 is the
  human's grilling, so 119 is the first AFK frontier ticket in order)
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

## Answer

**Yes: the corpus is shared, the budget is 32, and every audit-listed assertion
still runs.** The module now builds one observation per `(case_id, arm)` — five
committed cases × two arms = **10 runs** — in a process-global
`OnceLock<Corpus>`, and every other test reads those observations. The nine
removed captures are the five `fresh_consumers` visual legs, the legacy positive
capture, the comparator positive capture, the selection external-artifact run,
and the compile-retirement positive capture.

| Component | Before | After |
| --- | ---: | ---: |
| Module-plan corpus (WASM + native) | 10 | 10 |
| Fresh-consumer visual legs | 5 | 0 (shared) |
| Fresh-consumer preparation + cancellation | 10 | 10 |
| Typed projection | 6 | 6 |
| Legacy positive capture | 1 | 0 (shared) |
| Comparator positive capture | 1 | 0 (shared) |
| Failure scenarios | 4 | 4 |
| Selection scenarios | 3 | 2 (external artifact shared) |
| Compile-retirement positive capture | 1 | 0 (shared) |
| **Active total** | **41** | **32** |

**Measured module wall: 688.89 s → 620.44 s** (−68.45 s, −9.94 %) with the
acceptance command at `--test-threads 1`, 9 passed / 0 failed / 1 ignored on
both arms. A second after observation without the trace variable measured
611.64 s. The box was **not idle** (Opera, Stremio and Chrome; 43 % CPU sampled
before the before arm), the arms are not interleaved, and per-test walls are not
claimed because stable libtest rejects `--report-time`. Full conditions and
disclosures are in the evidence bundle.

The saving is much smaller than t114's pre-cache ~423 s estimate because t118's
compiled-component cache had already made each capture cheap; the consolidation
is now claim hygiene and aggregate CPU, not a large wall lever.

### Audit fixes in the same take

- The typed-projection scene-sharing assertion requested one layer, so its
  `Arc::ptr_eq` pair check was vacuous; it now requests two layers and requires
  `scenes.len() >= 2`.
- The selection test's test-local `participates` Boolean assertion is dropped;
  the production gate is exercised end to end by the inactive-owner leg.
- `Trace::max_retained_after_disposal()` now returns `Option<u64>`, so a cleanup
  assertion cannot pass when no disposal was recorded. Cleanup sites require
  `Some(0)`; the 57412 path accepts `None | Some(0)` only because its no-Ready
  assertion already rules out a published plan.
- A real cross-owner control was added: a second store's owner is refused as
  `InvalidOwner` by `begin`, `current` and `dispose_owner`. The disposed-owner
  block is narrowed to its actual disposal claim.
- The zero-slice selection predicate and store-ownership controls moved to
  `crates/slicer-runtime/tests/contract/lightning_selection_contract_tdd.rs`
  (2 tests, 0 slices, 0.00 s). The remaining zero-slice blocks stay in the e2e
  file: they no longer take a slice, and moving the decode/retirement helpers
  would not change the budget or the wall. Recorded as a deliberate deviation
  from the plan's hygiene item.

### Verification

- `cargo test -p slicer-runtime --test e2e -- --test-threads 1 --nocapture lightning_migration_tdd::`
  — 9 passed, 1 ignored, 620.44 s.
- `cargo test -p slicer-runtime --test contract lightning_selection_contract_tdd`
  — 2 passed, 0.00 s.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask check-test-quality --report` — 0 findings.
- `cargo xtask build-guests --check` — exit 0.
- [`verify.py`](../evidence/t119-lightning-corpus/verify.py) exits 0
  checkout-only; `--self-test` rejects 7/7 negative controls.
- Execution trace: the acceptance run emitted exactly 32
  `lightning-invocation:` lines (`run_visual` 13, `run_visual_native` 5,
  preparation 6, full slice 8), matching the source inventory entry by entry.

Evidence: [`evidence/t119-lightning-corpus/FINDINGS.md`](../evidence/t119-lightning-corpus/FINDINGS.md)
with [`verify.py`](../evidence/t119-lightning-corpus/verify.py),
`before.log` and `after.log`; the verifier exits 0 checkout-only and its
`--self-test` rejects all seven negative controls. No production edit.

## Answer

**Yes. The module now observes each `(case, arm)` exactly once, the active
invocation count is 32, every audit-listed assertion still executes, and the
serial module wall measured 688.89 s → 620.44 s (−68.45 s, −9.94 %).**

### What changed

`lightning_migration_tdd` now builds a process-global `OnceLock<Corpus>` with
one observation per case and arm — five cases × two arms = 10 real runs, arms
joined on Rayon. Each observation carries the capture **and** the lifecycle
trace that produced it. Tests that previously re-took the same print now read
the shared observation:

| Test | Before | After |
| --- | --- | --- |
| `module_plan_matches_recorded_baseline` | 10 own runs | 10 corpus runs |
| `fresh_consumers_emit_baseline_segments` | 5 visual + 5 prep + 5 cancel | corpus visual + 5 prep + 5 cancel |
| `legacy_contract_is_retired` | 1 own visual | corpus first case |
| `equivalence_rejects_corrupted_baseline` | 1 own visual | corpus `pooled_regions_classic` |
| `retirement_leaves_no_host_fallback` | 1 own visual | corpus first case |
| `selection_override_empty_and_foreign_owner` | 1 external visual + 1 inactive + 1 empty-plan | corpus visual + 1 inactive + 1 empty-plan |
| `typed_projection_is_atomic_and_neutral` | 6 | 6 (unchanged) |
| `failed_or_malformed_plan_never_falls_back` | 4 | 4 (unchanged) |

The corpus tap superset is `[Layer::Slice, Layer::Infill]`, all layers, no
views. The infill-only assertions filter by `CapturedIr` type, so the added
`Slice` tap cannot change them. The input digest still hashes the overrides the
run was actually handed (`ArmObservation::config`), never the baseline record.

### Invocation budget

| Component | Before | After |
| --- | ---: | ---: |
| Module-plan corpus | 10 | 10 |
| Fresh-consumer visual legs | 5 | 0 (shared) |
| Fresh-consumer preparation + cancellation | 10 | 10 |
| Typed projection | 6 | 6 |
| Legacy positive capture | 1 | 0 (shared) |
| Comparator positive capture | 1 | 0 (shared) |
| Failure scenarios | 4 | 4 |
| Selection scenarios | 3 | 2 |
| Compile-retirement positive capture | 1 | 0 (shared) |
| **Active total** | **41** | **32** |

The runtime corroborates it: `PNP_LIGHTNING_INVOCATION_TRACE=1` emitted exactly
**32** lines on the acceptance run — `run_visual` 13,
`run_visual_native` 5, `run_slice_preparation_capture` 6,
`run_slice_with_runtime_extensions` 8.

### Measurement

`cargo test -p slicer-runtime --test e2e -- --test-threads 1 --nocapture
lightning_migration_tdd::`

| arm | test wall | roster |
| --- | ---: | --- |
| before | **688.89 s** | 9 passed / 0 failed / 1 ignored |
| after | **620.44 s** | 9 passed / 0 failed / 1 ignored |

A second after run (no `--nocapture`, no trace) measured **611.64 s**; the
conservative 620.44 s is quoted. The box was **not idle** (Opera, Stremio and
Chrome running; 43 % CPU sampled before the before arm), both arms used the warm
wasmtime disk cache, and the arms are same-session rather than interleaved. The
saving is smaller than the audit's pre-cache ~423 s estimate because the t118
cache had already made each capture cheap (the nine removed captures were
≈7.6 s each of wall here). Full caveats are in the FINDINGS link below.

### Audit fixes carried in the same take

- `typed_projection_is_atomic_and_neutral` now requests **two** layers and
  asserts `scenes.len() >= 2` before the `Arc::ptr_eq` check, so the
  shared-scene claim is no longer vacuous.
- The test-local `participates = invoke && !targets.is_empty()` assertion is
  dropped; the production gate is exercised end to end by the inactive-owner
  leg.
- `Trace::max_retained_after_disposal()` returns `Option<u64>`; cleanup
  assertions require `Some(0)`, so a run that never disposes cannot read as
  clean. The 57412 path accepts `None | Some(0)` only with the no-Ready
  assertion already in place.
- The foreign-owner block is now a real cross-owner control:
  `begin`/`current`/`dispose_owner` on another store's owner return
  `PreparationFailure::InvalidOwner`. The disposed-owner block keeps the
  `AccessDenied` claim and is described as disposal.
- The zero-slice selection predicate and store-ownership controls moved to
  `crates/slicer-runtime/tests/contract/lightning_selection_contract_tdd.rs`
  (2 tests, 0 slices, 0.00 s).

**Deliberate deviation:** the artifact decode, digest arithmetic, comparator
controls, retirement text and compile witnesses remain in the e2e file. They
take no slice now, so moving them changes neither the invocation budget nor the
wall, and the strict decode helpers are shared with the corpus's baseline load.
No assertion or measured claim depends on the move.

### Evidence

[`evidence/t119-lightning-corpus/`](../evidence/t119-lightning-corpus/FINDINGS.md)
— `verify.py` re-derives the before inventory from `base-commit`
`1e20f52e…` and the after inventory from the current source, checks the
committed logs' walls and rosters, and cross-checks the execution trace.
`verify.py --self-test` rejects 7/7 negative controls.

### What is not claimed

- No full e2e-bucket wall. The ticket's gate is the module command; the shared
  corpus serialises the first lightning test behind its build, so the bucket
  effect is not inferred from the module.
- No aggregate CPU measurement.
- No idle-box claim; the numbers are a same-session screen under browser load.
