# Task Map: preparation-runtime-lifecycle

Draft only; this crosswalk records TASK-574's boundary in the approved queue, not task completion. No implementation or activation is authorized by generation.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-574` | Steps 0–3 | `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`, runtime/schema Cargo dependency sections | real PathOptimization/AnchoredEvents reader fixtures, production schema dependency move, selected binding and production forwarding runner | None: framework lifecycle, not algorithm parity | M | #03/#04 remain forward drafts; #01/#02 transitive. Direct runtime → schema is acyclic; do not depend on dev-only schema imports. |
| `TASK-574` | Steps 4–7 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | late preparation, ordinary selected Layer::AnchoredEvents reads, synthetic host closure retention, cancellation/cleanup | None | M | Existing ordinary stage loop is module dispatch; synthetic Anchored::Event commits are not. Runtime joins before disposal; producer Busy never waits. |
| `TASK-574` | Steps 8–11 | `docs/04_host_scheduler.md`, `docs/17_agent_debugging.md`, `docs/22_test_quality.md`, `docs/adr/0066-private-layer-preparation-capability.md` | metadata-only preparation capture, production read accounting, lifecycle/arena/host-closure observations and actual run_slice regressions | None | M | Names/lengths/lossless typed targets only; no raw opaque plan export or host decoder. #06 alone adds opt-in atomic typed diagnostics; AC-10 proves actual module-owned anchored consumption. |
| `TASK-574` | Steps 12–13 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`, `docs/17_agent_debugging.md` | normative runtime lifecycle documentation and narrow acceptance | None | S | Framework acceptance is independent of lightning migration; legacy host lightning remains until separately blocked #08. |

Self-review/authoring validation belongs to the draft-generation report. Backlog and queue updates belong to the parent session; this packet does not edit them during generation.
