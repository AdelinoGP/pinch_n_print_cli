# Task Map: lightning kernel portability (Gate B)

Canonical task crosswalk: `TASK-576` in `docs/07_implementation_status.md`, specifically the
native/WASM portability sub-gate split out of source-plan queue row 07 by explicit approval.
Packet `layer-module-preparation_07_lightning-geometry-portability-gate` owns the canonical input
fidelity sub-gate and publishes the fixtures this packet consumes. The split does not waive either
sub-gate, and neither packet authorizes row-08 generation until both verified outcomes and
implemented generic framework acceptance exist. `TASK-577` remains blocked.

| Task | Packet steps | Primary authority | Expected code surface | Canonical refs | Cost | Obligation |
| --- | --- | --- | --- | --- | --- | --- |
| TASK-576 | 1–2 | Approved portability gate; docs08/attribution/22 | Kernel `tree.rs`, `planner.rs` (+ lock) | `Generator::generateTrees`; `Layer::getBestGroundingLocation`/`attach`; `Node::convertToPolylines`; `Filler::_fill_surface_single` | M | Own canonical-correct planning and clipping; no host producer, no expectation lookup |
| TASK-576 | 3 | docs03/05/22; guest discovery/freshness | Test guest manifest/source/lock | Recorded graph/parameter references only | S | Existing `slicer:layer-infill` world; planned tags 1–3 and clipped two-point output |
| TASK-576 | 4–5 | docs03/05/21/22; AGENTS all-target/freshness | Host dev dependencies/root lock; `main.rs`, `lightning_portability_tdd.rs`, `lightning_portability_support.rs` | Independent recorded graph/grounding/clip references | M | Native + WASM execution and independent comparison; per-dispatch contributor handling |
| TASK-576 | 6 | Plan row-07 recording, ADR0066, docs22 | Outcome writer; append-only plan section; `TASK-576` B line | Kernel/guest/revision hashes | S | Truthful B status; `[x]` only after both gates pass; migration blocked otherwise |

Acceptance criteria belong exclusively to `packet.spec.md`. Gate A construction and fixture
publication belong to packet 07. This packet does not assert a B result; it authorizes the
experiment and the truthful record.
