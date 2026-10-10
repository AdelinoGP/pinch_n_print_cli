# LayerFinalization whole-print copying: per-module clone/drop on base supports-on

Type: task
Label: wayfinder:task
Status: open
Assignee: unassigned — surfaced by the 2026-10-09 perf refresh
([evidence](../evidence/perf-refresh-2026-10-09/FINDINGS.md)); human selection
pending
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

`execute_layer_finalization_with_instrumentation`
(`crates/slicer-runtime/src/layer_finalization.rs`) deep-clones the entire
`Vec<LayerCollectionIR>` before **every** finalization module
(`let mut candidate = layers.clone()`), runs the module on the clone, assigns it
back (`*layers = candidate`), and thereby drops the replaced vector. The clone
sits inside the module's instrumented bracket; the drop lands in the measured
inter-bracket gaps. The stage's four modules (`overhang-classifier-default`,
`part-cooling`, `skirt-brim`, `wipe-tower`, forced serial) therefore pay eight
whole-print copy/drop operations per slice. `execute_postpass_with_capture`
(`crates/slicer-runtime/src/postpass.rs`) adds one more whole-print copy
(`layer_irs.to_vec()`) plus a per-layer
`slicer_gcode::reconcile_finalization_travel` pass before `PostPass::GCodeEmit`.

Measured at HEAD (2026-10-09 refresh; raw captures gitignored, reduced tables
in `evidence/perf-refresh-2026-10-09/reduced/`):

- base classic supports-on postpass is **100.2–121.3 s in all 24 probe-free
  2026-10-09 A/B runs** (four arms; medians 106,282 / 107,373 / 114,072 /
  115,848 ms) — never attributed before; the post-repair 2026-09-27 run already
  recorded 113.7 s (`t27-serial-floor/results/run-phase-splits.csv`) but was not
  chased.
- Fresh profiled decomposition: stage 110,109 ms = module brackets 62,749 ms
  (15,062 / 15,055 / 15,982 / 16,650) + gaps 47,360 ms
  (8,258 / 12,595 / 13,340 / 13,167); module guest fuel ≤ 1.4 B of the cell's
  13,090 B, so the brackets are host copy time, not module execution.
- Negative control: base classic supports-off stage 2,040 ms (gaps 60 ms) for
  ~half the output.
- ≈11.1 s more sits between the instrumented stage brackets (the
  `to_vec`+reconcile suspect; code position, not probe-attributed).

Reduce the redundant whole-print copying without changing output. Candidate
shapes to falsify, not to assume: clone once per stage run and reuse the working
copy across the module sequence (the per-module clone reads as failure
atomicity, but `fin_result?` aborts the slice on error); gate the clone on
`module.ir_write_mask()`; per-layer copy-on-write; avoiding or cheapening the
pre-emit `to_vec`+reconcile pass. Preserve the documented failure semantics and
the finalization test surface (`crates/slicer-runtime/tests/executor/finalization_*`,
`crates/slicer-wasm-host` finalization tests).

## Acceptance

Output byte-identical on every measured cell (the take qualifies its matrix; the
ticket-57 precedent is the fresh 16-cell matrix), then the standing paired
ordinary + accelerated A/B on the matched classic supports-on benchy + base
cells; measured keep/drop to the human; no auto-commit. Base arachne
supports-on is the likely second beneficiary (the finalization stage is
generator-independent) and must be qualified inside the take, not assumed.

## Notes

- Evidence: `evidence/perf-refresh-2026-10-09/FINDINGS.md`, `reduced/finalization-gaps.json`,
  `reduced/t27-ab-postpass.json`; raw captures under
  `target/perf-refresh-2026-10-09/` and `target/t27-ab/campaign/measure/*/base/`.
- The stage is serial by construction (`pool size 1`), so these are wall
  seconds on the cell, unlike the accumulated per-layer columns.
