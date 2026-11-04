# Task Map: lightning canonical input oracle (Gate A)

Canonical task crosswalk: `TASK-576` in `docs/07_implementation_status.md`, no prerequisite and
blocking `TASK-577`. Status is mutable; re-derive it before execution and before updating it.
Source-plan queue row 07 is split by explicit approval into this packet (Gate A: canonical input
fidelity) and `layer-module-preparation_09_lightning-kernel-portability` (Gate B: native/WASM
portability). The split does not waive either sub-gate. The row-08 packet was re-scoped on
2026-10-20 to recorded behavioural equivalence, so it is not gated by these feasibility gates;
`TASK-576` remains `[~]` until both gates pass. (corrected 2026-10-25 against the plan's
2026-10-20 row-08 re-scope)

| Task | Packet steps | Primary authority | Expected code surface | Canonical refs | Cost | Obligation |
| --- | --- | --- | --- | --- | --- | --- |
| TASK-576 | 1–2 | Approved plan: lightning gate; docs08/21/22 | `tools/lightning_gate/record.py` + README; three fixture files | Recorded `replay.json`/`domains.json`/`planning.json`; `PrintObject::bridge_over_infill` | M | Publish the five Classic cases with strict A/B/reference separation and hashes; never synthesise an expectation |
| TASK-576 | 3–3a | docs02/08/attribution/22 | Kernel `model.rs`, `domain.rs`, `polygon.rs`, `distance_field.rs` | `PrintObject::bridge_over_infill`; `Generator::generateInitialInternalOverhangs` | M | Source-only reconstruction; a mismatch is a recorded fail, not repaired |
| TASK-576 | 4–5 | docs05/21/22, AGENTS test discipline | Host dev deps; `main.rs`, `lightning_oracle_tdd.rs`, `lightning_oracle_support.rs` | Independent recorded A reference | M | Five exact tests: provenance, decision, discriminator, isolation, outcome |
| TASK-576 | 6 | Plan row-07 recording, ADR0066, docs22 | Outcome writer; append-only plan section; `TASK-576` A line | Recorded revision/patch identity | S | Truthful A status; `[x]` only after both gates pass; migration stays blocked |

Acceptance criteria belong exclusively to `packet.spec.md`. Gate B evidence, clipped final-fill
comparison, and the `TASK-576 Gate B outcome:` line belong to packet 09. This packet does not
assert an A result; it authorizes the experiment and the truthful record.
