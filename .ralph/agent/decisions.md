# Decision Journal

Use this file to record consequential implementation-planning decisions with confidence scores.

## DEC-001
- Decision: Choose the initial public surface for TASK-022 DAG validation QA red.
- Chosen Option: Have the QA task define a full-scheduler validation API in `crates/slicer-host/src/validation.rs` that validates all loaded modules plus optional observed/runtime access metadata, returning a structured report with fatal errors and warnings rather than failing fast.
- Confidence: 72
- Alternatives Considered: Reuse `dag.rs` and expose only per-stage validators; defer undeclared-access and stage-mismatch coverage until later runtime-loading tasks.
- Reasoning: `docs/04_host_scheduler.md` defines 13 startup validation passes, including cross-stage and transitive dependency legality plus warning-grade dead writes, so a whole-scheduler report surface is the safest way to encode all passes now without prematurely entangling topological sort or execution-plan construction. Allowing optional observed/runtime access inputs keeps pass 11 and stage/export checks representable within the red contract.
- Reversibility: Moderate; the red API can still be narrowed during coding if the tests show a simpler boundary satisfies all documented passes.
- Timestamp: 2026-03-15T10:17:00Z
