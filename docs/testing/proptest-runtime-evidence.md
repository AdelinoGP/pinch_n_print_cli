# Property-testing evidence: runtime dispatch, selection authority, modifier transactions and fill partition

Packet: `proptest-openrscad_08_pbt-runtime`. This document is the execution evidence surface for
the four H-RUNTIME records (`H-RUNTIME-DISPATCH`, `H-RUNTIME-SELECTION`,
`H-RUNTIME-MODIFIERS`, `H-RUNTIME-PARTITION`) and the G-FAMILY evidence surface those records
support.
It mirrors the structure of the scheduler evidence file and records provenance, the
pre-implementation baseline, compile and execution measurements, independent oracle models, the
defect and regression ledger, the Windows delivery obligation, rejected-call and shrink status,
and the packet checks.

Seeds are diagnostic artifacts: committed regression history is a named concrete input, never a
seed. Every number below comes from an archived log under `target/evidence/` and from the per-run
`context.json` under `target/property-testing/<run-token>/<ID>/`; none is inferred.

## Provenance

- The local measurements were taken across the packet's implementation checkpoints: the per-run
  `context.json` files span revisions `599c1362`, `96794b60` and `f9d6163c`, and each records its
  own `source_revision`, `dirty_paths`, `source_state_digest`, `cargo_target_dir` and
  `completion_eligible`. The reviewed code is `9c7e059e00449daee42b780f58362f198a473cce`
  (short `9c7e059e`); the reviewer's final-tree extended rerun context is
  `target/property-testing/92640c8f023f4816109d79cbb1284c6f/` and lists only the two documentation
  paths as dirty. The per-run contexts are authoritative for what each measurement actually
  measured; this document does not freeze a revision or dirty-path count that an amend or squash
  would invalidate.
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)` and
  `cargo 1.96.0 (30a34c682 2026-05-25)` on a Windows host.
- Proptest `1.11.0` (`Cargo.lock`), chacha RNG, `isolation = "in_process"`, `timeout_ms = 0`,
  zero local/global/flat-map rejection budgets.
- `dev` profile; `CARGO_TARGET_DIR` unset, so the default workspace `target/` is used.
- Features: `["slicer-runtime/property_testing"]` plus the `slicer-core/host-algos` union that
  runtime's normal core dependency already requires. All four records carry
  `requires_host_algos = true`, `requires_scad = false` and `generator_version = 1`.
- `cargo xtask build-guests --check` exited `0` (`EXIT_FRESH`) before the guest-dependent
  baselines and before every property lane below, so no guest-staleness explanation is available
  for any runtime result here.

## Existing baseline (pre-implementation)

These owner tests were captured before the H-RUNTIME additions. Logs are archived under
`target/evidence/`; the command column is the exact invocation.

| Command | Result |
|---|---|
| `cargo test -p slicer-runtime --test executor -- modifier_region_split_tdd --nocapture` | 15 passed, 0 failed, 223 filtered out |
| `cargo xtask test --summary -p slicer-runtime --test executor -- shared_selection_runtime_tdd --nocapture` | PASS, 2 passed, 0 failed, 236 filtered out |
| `cargo xtask test --summary -p slicer-runtime --test executor -- source_identity_repair_tdd --nocapture` | PASS, 4 passed, 0 failed, 234 filtered out, 4.48s |
| `cargo xtask test --summary -p slicer-runtime --test executor -- apply_commit_regression_tdd --nocapture` | PASS, 3 passed, 0 failed |
| `cargo test -p slicer-wasm-host --lib -- selection::tests --nocapture` | 2 passed, 0 failed, 95 filtered out |
| `cargo test -p slicer-runtime --test unit -- blackboard_layer_arena_tdd --nocapture` | 9 passed, 0 failed, 85 filtered out |
| `cargo test -p slicer-sdk --features property_testing --test property_domain_contracts -- --nocapture` | 11 passed, 0 failed |

`modifier_region_split_tdd` is the pure modifier baseline; `shared_selection_runtime_tdd`,
`source_identity_repair_tdd` and `apply_commit_regression_tdd` are the real executor/guest
regressions and were routed through the xtask freshness gate. `selection::tests` is the pure
helper baseline and `blackboard_layer_arena_tdd` is the runtime arena baseline.

The same modifier baseline was re-run after the packet's production fix:
`15 passed, 0 failed, 223 filtered out` (`target/evidence/step4-modifier-baseline.log`), so the
focused rollback change did not move the existing partition behavior.

## Compile and execution measurements

Compilation and execution are separate measurements. Compile is measured per Cargo target: the two
integration records share `property_runtime`, and the two lib records share the `slicer_runtime`
libtest binary. A profile tier does not change compilation. Both were measured after
`cargo clean -p slicer-runtime` with warm dependency artifacts and the timing captured in the log
itself: `property_runtime` **20.637 s**
(`target/evidence/step7-compile-property-runtime.log`,
`COMPILE_TEST_WALL_SECONDS=20.637`), `slicer_runtime` libtest **15.068 s**
(`target/evidence/step7-compile-lib.log`, `COMPILE_LIB_WALL_SECONDS=15.068`).

Execution is libtest wall from the archived runs with the final shrink audits enabled. The routine
and extended columns are fresh-seed runs; the replay column is `--tier routine --seed 37`.

| Record | Compile (target) | Routine (64 cases) | Extended (512 cases) | Retained-seed replay (routine, seed 37) |
|---|---|---:|---:|---:|
| `H-RUNTIME-DISPATCH` | `property_runtime` 20.637 s | 0.54 s | 0.74 s | 0.55 s |
| `H-RUNTIME-SELECTION` | `property_runtime` 20.637 s | 2.19 s | 2.79 s | 2.15 s |
| `H-RUNTIME-MODIFIERS` | `slicer_runtime` lib 15.068 s | 22.31 s | 64.46 s | 20.97 s |
| `H-RUNTIME-PARTITION` | `slicer_runtime` lib 15.068 s | 13.79 s | 48.07 s | 13.31 s |

Whole-lane wall (xtask preflights, discovery, execution, summary; the `WALL_SECONDS` line is in
each archived log):

| Lane | Wall | Log |
|---|---:|---|
| four-record routine | 57.144 s | `target/evidence/step7-routine-all.log` |
| four-record extended | 133.735 s | `target/evidence/step7-extended-all.log` |
| four-record routine, `--seed 37` | 55.524 s | `target/evidence/step7-routine-seed37.log` |
| combined `--properties --tier routine` (all 19 records) | 72.966 s | `target/evidence/step7-incremental-routine.log` |

The combined routine lane is the CI selection check: it discovered and executed all four H-RUNTIME
records dynamically, plus the fifteen pre-existing records, and printed `VERDICT: PASS`. The
`properties-windows` CI job runs exactly this command
(`cargo xtask test --summary --properties --tier routine`), so the local Windows execution below
is the job command; the hosted job outcome is a separate delivery item, not a substitute for it.

The registry-backed invocation is
`cargo xtask test --summary --properties --property <ID> --tier routine|extended`, with
`--seed <u64>` for replay. Lib records run through `--lib --exact <full libtest name>`; integration
records use their `property_runtime` target identity. Exact libtest identities observed:
`proptest_runtime_semantic_dispatch`, `proptest_runtime_selection_authority`,
`region_partition::property_tests::proptest_runtime_modifier_transactions` and
`region_partition::property_tests::proptest_runtime_fill_partition`.

## Independent models

The shared model file is `crates/slicer-runtime/tests/common/property_runtime_cells.rs`. It is
mounted by both the library submodule `region_partition::property_tests` in
`crates/slicer-runtime/src/region_partition/property_tests.rs` and the integration control target
`crates/slicer-runtime/tests/property_runtime_contracts.rs`, so the generative bodies and the
deterministic controls use one cell model rather than two divergent copies.

### H-RUNTIME-DISPATCH

The oracle is an independent set-intersection predicate over the declared semantic names and the
ordered `variant_chain` of every region. It recomputes `declared.is_empty() || slice.is_none() ||
any chain semantic in declared` directly; it does not call `paint_allows_layer`, and it does not
short-circuit through the same boolean expression order as the SUT. The SUT cannot certify itself
because the whole contract is which of the three clauses fires and whether the per `(module, layer)`
selection keeps every candidate region rather than filtering to matching regions. The property
also drives the real runtime publication seam for a neutral `Layer::SlicePostProcess` binding and
observes `invoke` and the full target table.

### H-RUNTIME-SELECTION

The oracle builds an exact-key authority table from the generated `RegionMapIR` entries and source
regions, recomputing the map-space address from the written rule rather than calling
`region_map_key_for_sliced_region`. It derives the expected `(invoke, config marker, ordered
target keys)` table and the expected `SelectionError`/publication outcome, including map insertion
permutations and `Arc` identity on duplicate publication. The SUT is the thing that builds the map
and caches configs, so using its own builder as the oracle would make every authority defect
invisible.

### H-RUNTIME-MODIFIERS

The oracle is a first-owner cell partition: every elementary cell is assigned to the earliest
intersecting footprint for its object, or to the parent remainder, and ownership/conservation is
checked from those labels. It never calls the private splitter, the raw wrapper, the public prepass
or Clipper differences to derive ownership, because those are the operations under test. Metadata,
annotation clearing, namespace decoding and internal-bridge angle association are checked against
the same labels and the generated index alignment.

### H-RUNTIME-PARTITION

The oracle is a conditional cell-label model: `point_in_region` independently classifies each cell
against the donor `infill_areas`, the slice-region polygons and each original role carrier, then
applies `bridge > bottom > top > sparse` over `W` for nonempty `W`, assigns no role for empty `W`
with walls, and applies the original-bridge/top-minus-bridge fallback for empty `W` without walls.
The conditional empty-W/no-wall fallback versus the nonempty-inset clipping rule is checked as
written. Using a Clipper Boolean result as the oracle would assume the SUT's partition arithmetic;
using the SUT's own role polygons as the expectation would make precedence errors self-consistent.

### Shrink audits

Each record has one fixed-seed (`0x5eed_c0de`) `simplify()` audit that builds the same strategy the
registry entrypoint uses and runs the same case checker. Every audit requires its structural
witness tags to be reached and at least one simplification to occur; the audit walks at most 128
trees and 64 simplification steps, so a run at the cap is never described as minimal.

| Record | Structural tags required per tree / all observed | Checker re-run |
|---|---|---|
| `H-RUNTIME-DISPATCH` | 6 modes / 6 | generated and every simplified tree |
| `H-RUNTIME-SELECTION` | 2 modes (`negative` both ways) / 2 | generated and every simplified tree |
| `H-RUNTIME-MODIFIERS` | 8 (4 modes, holed, empty footprint, later-footprint overlap, role-precedence overlap) / all 25 observed | generated and final simplified tree at the audit cap |
| `H-RUNTIME-PARTITION` | 2 (nonempty effective inset, role-precedence overlap) / all 11 observed | generated and final simplified tree at the audit cap |

The dispatch and selection audits re-run the full case checker on every simplified tree. The
modifier and partition checker stages a real arena per call (~23 ms in the debug test profile), so
those two audits re-run it on the generated tree and on the final simplified tree at the audit cap;
the required tag set is asserted at every step. The rejection witnesses (full-identity collision,
unencodable parent, later-failure rollback) are generated modifier modes whose construction is
literal and `no_shrink`, so their tags cannot be simplified away; the fatal staging cases remain
deterministic control inputs and have no shrink interaction.

## Defect and regression ledger

| Finding | Kind (SUT defect / generator-oracle / none) | Named regression | Status |
|---|---|---|---|
| `sync_perimeter_infill_areas_into_slice` took the staged slice through `take_slice` and then propagated `split_modifier_footprints(...)?` before `set_slice`, so a rejected modifier split left `arena.slice()` as `None`. | SUT defect (confirmed, fixed) | `runtime_modifier_collision_restores_taken_slice` | Reproduced pre-fix (`regression: the hook took the staged SliceIR and returned the split error without re-staging it, leaving the arena without a layer`, `target/evidence/step4-regression-prefix.log`), fixed by running role sync and the split on a working copy and re-staging the entry-state slice on any rejection, regression green post-fix (`target/evidence/step4-regression-postfix.log`). |
| Dispatch property expectation reversed the original keys instead of deriving ids from the permuted committed slice, so the fixture's own order permutation failed. | generator-oracle (test bug) | none (test-local) | Corrected in the test; the production predicate matched the independent model on both orders. |
| Selection fixture could generate the same full source identity on two same-parity layers (same object, same empty-chain ordinary id), violating the packet's uniqueness precondition. | generator-oracle (test bug) | none (test-local) | Corrected by layer-qualifying the generated source ids; the uniqueness precondition is now true by construction. |
| The modifier valid domain could draw two identical footprints for one object, which is a real identity collision, so a `Valid` tag could construct `LaterFailure`. | generator-oracle (test bug) | none (test-local) | The domain now deduplicates nonempty footprints by `(object, prospective id)` and keeps the overlap witness with a fixed fallback rectangle; extended 512-case runs pass. |
| The modifier role-precedence tag was derived from independently drawn carrier cells, so an accidental overlap could disappear during shrinking; a hole-crossing footprint also made the carrier model subtract only parent-claimed cells. | generator-oracle (test bug) | none (test-local) | The tag now requires the structural `no_shrink` bit and its geometric consequence; carrier splitting models the raw footprint (as the kernel does), while child `polygons` model parent-claimed cells. The shrink audit and extended runs are green. |
| No other SUT defect, generator/oracle mistake or `none` disposition was confirmed. | none | none | The conditional partition table matched the inspected production branches on every generated and deterministic witness; no assertion was weakened and no domain was narrowed to obtain a pass. |

## Windows CI delivery

The required foundation `properties-windows` job runs
`cargo xtask test --summary --properties --tier routine` on a Windows runner. That exact command
was executed locally on this Windows host and passed all nineteen registered records, including the
two library identities `region_partition::property_tests::proptest_runtime_modifier_transactions`
and `region_partition::property_tests::proptest_runtime_fill_partition`
(`target/evidence/step7-incremental-routine-full.log`, 72.966 s wall).

The hosted delivery is recorded. Workflow run
<https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43629202926> (CI, head
`9c7e059e00449daee42b780f58362f198a473cce`) concluded `success` with all nine jobs green. The
`Properties (Windows)` job
<https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43629202926/job/132551819057> passed in
13m57s and ran the dynamic routine selector, so the four H-RUNTIME records and both lib identities
were selected and executed on a hosted Windows runner. A routine run that omits the lib targets is
not delivery, and registry-only or compile-only evidence is not a substitute. Packet #17 audits the
delivery; the per-ID accepted counts remain the local Windows measurements above, because the
hosted summary does not echo per-record IDs. The preceding run
<https://github.com/AdelinoGP/pinch_n_print_cli/actions/runs/43628720540> failed only the `Format`
job on the same code before the POSIX-portable `#[path]` mount fix; it is retained here as the
reason for that fix, not as delivery evidence.

## Rejected calls, shrinking and exhaustion

Every measured run recorded its controller `PNP_PROPERTY_EVENT` begin/end pair, accepted cases,
rejected test calls, `call_counts_complete` and `shrink_completed`. Observed:

| Lane | Accepted cases | Rejected test calls | `call_counts_complete` | `shrink_completed` |
|---|---|---|---|---|
| routine, all four | 64 each | 0 each | `true` each | `null` each |
| extended, all four | 512 each | 0 each | `true` each | `null` each |
| routine seed 37, all four | 64 each | 0 each | `true` each | `null` each |
| combined routine (19 records) | 64 each | 0 each | `true` each | `null` each |

Logs: `target/evidence/step7-routine-all-full.log`, `step7-extended-all-full.log`,
`step7-routine-seed37-full.log`, `step7-incremental-routine-full.log`.

- Accepted cases equal the committed profile case count, so every required case ran.
- The committed rejection budgets are zero and every generator is total; a nonzero rejection count
  would signal a generator change and must abort as `exhausted`, not consume budget silently.
- `call_counts_complete = true` on every run; an incomplete call journal is a failure.
- `shrink_completed = null` for passing runs: Proptest 1.11 cannot report whether shrinking reached
  a limit, so no retained case is claimed minimal. Shrinking is pinned structurally by the shrink
  audits above (mode tags, same checker, nonzero simplifications).
- Exhaustion is an explicit failed/exhausted diagnostic, never a pass. Zero execution (no case
  observed) is a failure even if the process exits cleanly.

## Checks

Both packet checks are expected to PASS:

```text
rg -n 'G-FAMILY|Windows|source_revision|H-RUNTIME|rollback' docs/testing/proptest-runtime-evidence.md
rg -n '^## H-RUNTIME-|paint_allows_layer|transaction|empty.*inset' docs/testing/properties.md
```

The first check resolves the G-FAMILY purpose, the Windows delivery obligation, the
`source_revision` provenance token, every H-RUNTIME table row and the hook-local `rollback` ledger
row. The second resolves all four charter headings, the `paint_allows_layer` contract, the
transaction paragraphs and the conditional `empty ... inset` policy statement.
