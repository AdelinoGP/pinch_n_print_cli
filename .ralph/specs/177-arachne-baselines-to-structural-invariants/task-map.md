# Task Map: 177-arachne-baselines-to-structural-invariants

This packet has no `docs/07_implementation_status.md` task IDs. It is an
audit-driven remediation sourced from `D-112-SELFCAPTURED-BASELINES` in
`docs/DEVIATION_LOG.md`; no `TASK-###` is invented.

Verify the deviation at point of use:

```text
rg -n 'D-112-SELFCAPTURED-BASELINES' docs/DEVIATION_LOG.md
```

| Backlog source | Packet step | Primary evidence | Expected surface | Context cost |
| --- | --- | --- | --- | --- |
| D-112: odd defaults | Step 1a/1b | recovery Track B; canonical `WallToolPaths.cpp::generate` | production defaults and surviving test helpers | S |
| D-112: self-captured JSON oracles | Step 2/5 | ADR-0042 structural classes | four core consumers, `arachne_invariants`, eight core + eleven perimeter JSON deletions | M |
| D-112 + ADR-0042: coverage floor | Step 3/4 | ADR-0042 coverage class; coordinate checklist | shared runtime harness, standalone coverage binary, five STL subjects | M |
| D-112: tapered-wedge conversion | Step 5 | recovery Track B; no-recapture rule | perimeter integration and deleted expected IR | M |
| Track B hygiene | Step 6 | D-104f deviation row | nine exact red-test moves and runtime header | S |
| D-112 closure + ADR instantiation | Step 7 | D-112 row and ADR-0042 Consequences | recovery doc, ADR, deviation log, glossary | S |

## Decisions Captured

- All eight core snapshots and all eleven perimeter expected-IR snapshots are
  deleted, not retained as archives or change-detectors.
- Coverage subjects are the five Arachne perimeter STLs; D5 is synthetic-only.
- Coverage lives in a standalone runtime test binary with a shared harness under
  `tests/common/`.
- The default correction remains `10`, with corrected canonical rationale.
- The repeatability margin is derived from same-subject/same-Z reruns and capped
  at `0.02`; fixture spread cannot widen it.
- The tapered-wedge expected IR snapshot is deleted.
- Track B recovery prose is corrected in this packet.
