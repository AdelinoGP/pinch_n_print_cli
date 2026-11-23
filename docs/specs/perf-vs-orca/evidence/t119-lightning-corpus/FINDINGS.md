# t119 — Lightning corpus consolidation

Resolved 2026-11-05 by an OpenCode wayfinder session. Ticket:
[`issues/119-lightning-corpus-consolidation.md`](../../issues/119-lightning-corpus-consolidation.md).

## Question

Can `lightning_migration_tdd`'s active tests share **one** corpus capture per
`(case_id, arm)` — instead of re-running 27 visual captures across 9 tests — so
the module's active invocation count falls from **41 to 32** with **every
assertion preserved**, and what is the measured before/after module wall?

## Answer

**Yes.** The module now builds one observation per `(case, arm)` — five cases ×
two arms = **10 runs** — in a process-global `OnceLock<Corpus>`, and every other
test reads those observations. The active invocation count falls **41 → 32**
(−9, −21.95 %), every audit-listed assertion still executes, and the serial
module wall measured **688.89 s → 620.44 s** (−68.45 s, −9.94 %) on the same
machine and build profile.

The wall saving is smaller than the t114 audit's pre-cache ~423 s estimate
because the t118 compiled-component cache had already made each capture cheap:
the nine removed captures were ≈7.6 s each of wall in this run. The
consolidation is therefore **claim hygiene plus aggregate CPU/contention**, not
a large wall lever on its own.

<!-- t119-measurement
base-commit: 1e20f52ee7c4e7b041272ea314807df45d6ac1f3
before-wall-s: 688.89
after-wall-s: 620.44
active-invocations: 32
-->

## Invocation budget

| Component | Before | After | How the after count is derived |
| --- | ---: | ---: | --- |
| Module-plan corpus (WASM + native, 5 cases) | 10 | 10 | `observe_wasm` + `observe_native`, one site each × 5 cases |
| Fresh-consumer visual legs | 5 | 0 | reuses the corpus WASM arm |
| Fresh-consumer preparation + cancellation | 10 | 10 | one site each × 5 cases |
| Typed projection | 6 | 6 | 4 `run_visual` + 2 full slices |
| Legacy positive capture | 1 | 0 | reuses the corpus first case |
| Comparator positive capture | 1 | 0 | reuses the corpus `pooled_regions_classic` arm |
| Failure scenarios | 4 | 4 | 1 preparation + 3 consumer failures |
| Selection scenarios | 3 | 2 | external-selection metadata comes from the corpus; inactive + empty-plan remain |
| Compile-retirement positive capture | 1 | 0 | reuses the corpus first case |
| **Active total** | **41** | **32** | |

**Runtime corroboration.** `PNP_LIGHTNING_INVOCATION_TRACE=1` makes each real
pipeline start emit one `lightning-invocation: <entry>` line. The acceptance run
emitted exactly **32** lines: `run_visual` 13, `run_visual_native` 5,
`run_slice_preparation_capture` 6, `run_slice_with_runtime_extensions` 8 — the
same per-entry counts the source inventory derives. The trace is inert when the
variable is unset. See [`after.log`](after.log).

## What each test now uses

| Test | Observation | Assertions retained |
| --- | --- | --- |
| `module_plan_matches_recorded_baseline` | corpus `wasm` + `native` | both arms vs the recorded population, input digest first, the changed-scalar digest control |
| `fresh_consumers_emit_baseline_segments` | corpus `wasm` + own preparation-only + own cancel-after-publication | emitted segment multiset, `SparseInfill` role/width, ordered `variant_chain` identity, lifecycle accounting, cancellation cleanup |
| `typed_projection_is_atomic_and_neutral` | own 6 runs (unchanged) | scene contents, plan-edge containment, capture neutrality, named failures, Ready/no-work |
| `legacy_contract_is_retired` | corpus first case | live-contract absence scan, six doc sections, output equivalence |
| `equivalence_rejects_corrupted_baseline` | corpus `pooled_regions_classic` | dropped/reversed/duplicated/swapped/length-only controls, empty-population controls |
| `failed_or_malformed_plan_never_falls_back` | own 4 runs (unchanged) | 57401/57410/57411/57412 distinctness, atomic publication, cleanup |
| `selection_override_empty_and_foreign_owner` | corpus first case + own 2 runs | inactive owner reads nothing, external artifact wins, Ready/no-work |
| `retirement_leaves_no_host_fallback` | corpus first case + 9 `rustc` witnesses | each retired symbol fails on its own name; slice still emits baseline-equivalent infill |

The corpus tap superset is `[Layer::Slice, Layer::Infill]`, all layers, no views.
The readers filter by `CapturedIr` type, so the added `Slice` tap cannot change
an infill-only assertion.

## Audit findings fixed in the same take

- **`typed_projection_is_atomic_and_neutral`'s scene-sharing assertion was
  vacuous.** It requested layer `[0]`, so `scenes.windows(2)` had no pair.
  The request is now two layers, and the test asserts `scenes.len() >= 2`
  before the `Arc::ptr_eq` claim.
- **`selection_override_empty_and_foreign_owner`'s `participates` assertion was
  test-local Boolean arithmetic.** Dropped. The production gate
  (`resolve_preparation_visual_request`'s `invoke && !targets.is_empty()`) is
  exercised end to end by the inactive-owner leg.
- **`Trace::max_retained_after_disposal()` returned `0` when no disposal was
  recorded.** It now returns `Option<u64>`; cleanup assertions require
  `Some(0)`. The 57412 failure path accepts `None | Some(0)` only because the
  no-Ready assertion already rules out a published plan that leaked.
- **The "foreign owner" block was a disposal test.** It now has a real
  cross-owner control: an owner created by a second store is refused as
  `PreparationFailure::InvalidOwner` by `begin`, `current` and `dispose_owner`.
  The disposed-owner claim is narrowed to disposal.
- **Zero-slice selection claims moved to contract scope.** The selection
  predicate and the store-ownership controls now live in
  `crates/slicer-runtime/tests/contract/lightning_selection_contract_tdd.rs`
  (2 tests, 0 slices, 0.00 s). The e2e module keeps the two real-print legs.

**Zero-slice blocks left in place, deliberately.** The artifact decode,
digest arithmetic, comparator controls, retirement text and compile witnesses
remain in the e2e file. They take no slice now (the corpus supplies any real
population they need), so moving them changes neither the invocation budget nor
the wall, while the strict decode helpers are shared with the corpus's own
baseline load. This is a recorded deviation from the plan's hygiene item, not
from an assertion or a measured claim.

## Measurement

```text
cargo test -p slicer-runtime --test e2e -- --test-threads 1 --nocapture lightning_migration_tdd::
```

| arm | test wall | roster | invocation lines |
| --- | ---: | --- | ---: |
| before | **688.89 s** | 9 passed / 0 failed / 1 ignored | — (instrumentation added in the after arm) |
| after (acceptance) | **620.44 s** | 9 passed / 0 failed / 1 ignored | **32** |

- A second after run without `--nocapture` and without the trace variable
  measured **611.64 s**. The two after observations bracket the reported
  620.44 s; the conservative one is quoted above.
- Both arms ran on the same Windows dev box and the same debug build profile,
  warm wasmtime disk cache (`%TEMP%/pnp-wasmtime-cache-v1`). The box was **not
  idle**: Opera, Stremio and Chrome were running (43 % CPU load sampled before
  the before arm). The arms are not interleaved; they are same-session.
- Per-test walls were not captured: stable libtest rejects `--report-time`
  unless the toolchain is nightly. Only the module total is claimed.
- **No production edit.** The change is confined to the e2e test file, a new
  contract test file, and its aggregator entry.

## Reproduce

```bash
cargo xtask build-guests --check          # exit 0
cargo build --bin pnp_cli                 # locator gate
cargo test -p slicer-runtime --test e2e --no-run
PNP_LIGHTNING_INVOCATION_TRACE=1 \
  cargo test -p slicer-runtime --test e2e -- --test-threads 1 --nocapture lightning_migration_tdd::
python docs/specs/perf-vs-orca/evidence/t119-lightning-corpus/verify.py
python docs/specs/perf-vs-orca/evidence/t119-lightning-corpus/verify.py --self-test
```

`verify.py` re-derives the after inventory from the current test source, the
before inventory from the recorded `base-commit` (override with
`--base-sha`), the case count from the committed baseline, the per-entry counts
from `after.log`, and the walls from both logs. It exits 0 against a plain
checkout; `--self-test` mutates a temporary copy and requires the verifier to
reject every control.

## What is not claimed

- No full e2e-bucket wall. The ticket's gate is the module command; the bucket
  runs tests concurrently and the shared corpus serialises the first lightning
  test behind its build, so the bucket effect is not inferred from the module.
- No aggregate CPU measurement. The 9 removed captures are a real work
  reduction, but this take measured wall only.
- No idle-box claim. The numbers are a same-session screen under browser load.
