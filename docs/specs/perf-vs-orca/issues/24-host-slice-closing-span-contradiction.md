# host:slice closing_ex span contradiction

Type: task
Status: open

## Question

`host:slice`'s `closing_ex` scope spans (~22.3 s accumulated) contradict its
`module_complete` wall (~3.2 s). Which number is real — and does
`fold_marks` (`crates/slicer-wasm-host/src/profiling.rs`) handle thread-mapped
scope marks correctly?

This is lead 4 of the [emit_walls premise
falsified](08-emit-walls-premise-falsified.md) ranking. The working hypothesis
is the accumulated-worker-elapsed trap (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §3.3: summed
worker wall-clock includes waits and is not CPU), but it must be **verified**
against `fold_marks`' thread handling before either number is believed — the
§9.7 history shows a wrong reading here once motivated a dead hypothesis (the
allocator A/B).

Attribution-only: no optimization proposed or authorized until the
contradiction is explained. Small ticket — the answer either retires the
`host:slice` item from the lead list or promotes a real 22 s cost.
