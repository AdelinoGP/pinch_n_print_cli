# Task Map: preparation-capability-pilot

This explicit crosswalk is required despite one task ID because queue row 02 is an independent producer gate for TASK-574's three later rows. Backlog fact was verified against the actual TASK-573 entry in `docs/07_implementation_status.md`; completion is not claimed.

| Backlog task | Packet steps | Primary authorities | Expected code surface | Orca refs | Cost | Boundary |
| --- | --- | --- | --- | --- | --- | --- |
| TASK-573 | 1–3 | ADR-0066; docs/03; docs/05 | Canonical new preparation WIT/schema sidecar; SDK required trait/read facades/native sidecar | None: transport pilot | S per step | Concrete planned producer shapes in packet.spec; no production parsing/storage completion |
| TASK-573 | 4–5 | ADR-0045/0066; docs/03/05 | Macro explicit opt-in/same-artifact exports; capability-aware xtask artifact verification; macro regressions | None | M/S | Preserve plain worlds and existing struct/version literals |
| TASK-573 | 6–7 | ADR-0056/0066; docs/05/08 | Two discovered dual native/guest controlled fixtures | None | S per step | No routing prerequisite or lightning algorithms |
| TASK-573 | 8–9 | docs/03/21/22; ADR-0066 | Registered contract resource host/tests, typed dispatch, schema compile driver | None | M per step | Existing contract binary; no required features; nonzero tests mandatory |
| TASK-573 | 10–11 | Source plan accounting; ADR-0066; docs/22 | Measurements/metadata whitelist and exact doc-impact sections | None | S per step | No unmeasured latency/peak memory claims |

## Backlog and Queue Crosswalk

- `TASK-573`: queue row 02 `preparation-capability-pilot`; prerequisites none; blocks TASK-574. Source `docs/specs/layer-module-preparation-plan.md`, row 02; output directory is this packet.
- `TASK-572` / row 01: production routing/configuration projection independent of this controlled pilot; not consumed or marked complete here.
- `TASK-574` / rows 03–05: row 03 consumes planned WIT/SDK/macro/native seams only after actual pilot success; row 04 owns production whole-print typed projections plus routing dependency; row 05 owns actual lifecycle activation. Their work/status are not edited here.
- This author emits five draft contracts only. Plan/queue/backlog updates and independent preflight belong to the parent orchestration; no activation, implementation, commit or check/test execution occurred.
