# Arachne critical-tail attribution — authorized diagnostic pair

**The measured tail owner is the `com.core.infill-linker` dispatch in
`Layer::InfillPostProcess`, not the Arachne perimeter dispatch.** The diagnostic
pair completed; adoption remains inconclusive and ordinary remains production.

## Scope and integrity

Exactly one ordinary and one accelerated supports-off Benchy Arachne slice ran
sequentially, at 12 threads, with `--instrument-stderr` and no fuel profiling.
Both used the original manifest-selected model/config and mode-specific frozen
production snapshots. Commands, identities, event reductions and raw-file hashes
are in [attempt-1/results.json](attempt-1/results.json).

The existing reference identity gate passed before and after. Recursive ordinary
and accelerated snapshot identities and recorded campaign inputs also matched.
All original campaign raw and tracked inventory entries remain unchanged.
Both new outputs are byte-identical to the frozen Arachne reference, with clean
completion, zero error/degraded counts, and validated Arachne config/output/
dispatch markers. The benchmark's real shared validator ran for both slices.

Raw G-code and JSONL are retained in the distinct durable namespace named by
`durable_raw_root` in the results. The `-Instrumented` harness route emits only
a CSV header, not retained timing rows: these are **diagnostics, not acceptance
samples**. No workspace tests, builds, extra slices, acceptance retry, skipped
cell, reference update, production fix, commit or default switch ran.

## Tail measurements

These are observed instrumented elapsed intervals from one invocation per mode;
they are neither process CPU nor a statistically established speed comparison.
Layers 1 and 2 are again the two longest layers in both captures.

| Mode | Global layer (zero-based) | Full layer | Infill-linker dispatch | Share of layer interval | Arachne perimeter dispatch |
|---|---:|---:|---:|---:|---:|
| Ordinary | 1 | 4584 ms | 4126 ms | 90.01% | 105 ms |
| Ordinary | 2 | 2808 ms | 2715 ms | 96.69% | 17 ms |
| Accelerated | 1 | 4451 ms | 4047 ms | 90.92% | 64 ms |
| Accelerated | 2 | 2735 ms | 2633 ms | 96.27% | 21 ms |

The corresponding `Layer::InfillPostProcess` stage intervals are 4126/2716 ms
ordinary and 4049/2634 ms accelerated. This agrees with localization to that
dispatch, rather than substantial unaccounted work surrounding it. Stage and
module durations are nested: do not add them together.

### The boundary that remains unmeasured

The `on_module_start` → `on_module_end` bracket in `execute_single_layer_inner`
(`crates/slicer-runtime/src/layer_executor.rs`) encloses host region-view
preparation and `runner.run_stage`, including dispatch/marshalling and guest
execution. It ends before applying the returned commit. Therefore the four
linker timings above **do not prove that the guest connection algorithm alone
consumes them**.

`InfillLinker::run_infill_postprocess`
(`modules/core-modules/infill-linker/src/lib.rs`) delegates to
`orchestrate_infill` (`modules/core-modules/infill-linker/src/orchestrate.rs`).
That path includes role/group preparation, overlap offset, per-path re-clipping,
connection and output ownership assignment. `link_paths_without_offset` calls
`clip_to_offset_boundary` and then `chain_or_connect_infill`; multi-region paths
also use `majority_owner`. Their individual costs were not measured here.

The linker has no custom sub-operation profiling scopes in its current source.
The next study must distinguish host preparation/marshalling from guest work
before choosing a linking, clipping or ownership optimization. This is the
**one promoted lead**: low-layer linker dispatch subcost attribution, carried by
[ticket 21](../../issues/21-infill-linker-attribution.md). It needs separate
authorization; no fuel probe or temporary instrumentation was added here.

### Current artifact identity caveat

The frozen ordinary and accelerated linker manifests match, but their WASM
hashes **differ**. [Independent verification](attempt-1/verification.json)
records both actual hashes. Ticket 21's historical byte-identity statement is
not an invariant of these current snapshots. Do not infer either identical
execution or a particular acceleration benefit from that old statement.

## Prepass observation

Prepass was 6940 ms ordinary versus 8202 ms accelerated in this diagnostic pair.
Its largest stage intervals were:

| Stage | Ordinary | Accelerated |
|---|---:|---:|
| `PrePass::Slice` | 2512 ms | 3437 ms |
| `PrePass::ShellClassification` | 3204 ms | 3325 ms |
| `PrePass::OverhangAnnotation` | 821 ms | 1008 ms |

This is evidence of substantial observed prepass variation, not proof of a
mode regression or external load. A single sequential diagnostic pair has no
load telemetry or repeated-sample inference. It cannot overturn the original
acceptance verdict or justify selective sample removal.

## Validation

- `python docs/specs/perf-vs-orca/evidence/t43-critical-tail/run_diagnostics.py`:
  exit 0, exactly two authorized slices, all before/after gates passed. The
  one-shot guard forbids repeating this attempt.
- `python docs/specs/perf-vs-orca/evidence/t43-critical-tail/verify_diagnostics.py`:
  exit 0; re-read raw traces, recomputed reductions, matched stage/module start
  and completion identities, verified complete layer coverage, raw hashes and
  preservation of the original campaign. This command launches no slices.
- No Rust tests or production-code gates were run: no production code changed.
  Guest-vs-host subcost attribution and a production fix remain unmeasured.
