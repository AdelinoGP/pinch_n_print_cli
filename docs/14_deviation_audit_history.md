# Deviation Audit History

Last updated: 2026-04-16

## Purpose

This file preserves the minimum historical context from the retired audit artifacts:

- `docs/deviationList.xml`
- `docs/14_deviation_audit_tasks_1.md`
- `docs/14_deviation_audit_tasks_2.md`

Use this file for provenance, chronology, and legacy-reference lookup only.

- Use `docs/DEVIATION_LOG.md` for the live architecture deviation registry.
- Use `docs/07_implementation_status.md` for the active remediation backlog.
- Use `docs/11_operational_governance_and_acceptance_gate.md` and `docs/12_architecture_gate_metrics.md` for release-gate policy and evidence thresholds.

## Artifact Replacement Map

| Retired artifact | Former role | Canonical replacement |
| --- | --- | --- |
| `docs/deviationList.xml` | Working audit registry, blocker rollup, and legacy numbering source | `docs/DEVIATION_LOG.md` for live rows, `docs/07_implementation_status.md` for active tasks, this file for legacy crosswalk and audit chronology |
| `docs/14_deviation_audit_tasks_1.md` | Generated audit instructions and handoff template | This file's audit method summary plus the normalized `docs/DEVIATION_LOG.md` rows |
| `docs/14_deviation_audit_tasks_2.md` | Audit execution results and point-in-time summaries | This file's outcome summary plus the current row state in `docs/DEVIATION_LOG.md` |

## Audit Method Summary

- The 2026-04 audit ran 24 verification-only checks against the implementation and the architecture docs.
- Each audit used the same pattern: read the normative docs first, inspect the decisive implementation path, verify test evidence, then record any still-open drift in `docs/DEVIATION_LOG.md`.
- Fixed or stale legacy claims were closed in the live deviation log instead of being preserved as independent work items.
- Several XML-era topics were merged, split, or retired during that cleanup, so the old `deviation #N` labels are not stable identifiers anymore.

## Outcome Summary

### Key closures and stale legacy claims confirmed during audit

- `DEV-012` closed: the `#[slicer_module]` macro now emits typed WIT export glue for all four worlds.
- `DEV-018` closed: prepass segmentation dispatch is wired on the live host path.
- `DEV-019` closed: paint-annotation execution and warning propagation are wired through the live pipeline.
- `DEV-021` closed: all 17 core modules now ship real component-model `.wasm` artifacts.
- `DEV-022` closed: `ConfigView` immutability and declared-key filtering are enforced on the live path.
- `DEV-028` closed: the layer planner no longer depends on a hand-written duplicate `wit-guest` implementation.
- `DEV-029` closed: blocker-summary drift was merged into the broader planning row `DEV-030`.

### Remaining live architecture themes after audit normalization

- Contract enforcement: `DEV-002`, `DEV-003`, `DEV-004`, `DEV-005`, `DEV-008`
- Runtime data boundaries and WIT compatibility: `DEV-006`, `DEV-013`, `DEV-014`, `DEV-015`, `DEV-016`, `DEV-025`, `DEV-026`, `DEV-027`
- Feature parity and live-path behavior: `DEV-009`, `DEV-023`, `DEV-024`
- Governance and planning closure: `DEV-010`, `DEV-020`, `DEV-030`

## Legacy Backlog Crosswalk

The status backlog previously referenced XML-era labels such as `deviation #14b` and `deviation #23`. Those labels were retired because the audit cleanup merged several historical entries and rewrote the live registry around stable `DEV-###` rows.

Use the topic-based mapping below when reading older notes or commit history.

| Legacy backlog topic                                                  | Retired XML-era references            | Current canonical tracker                                |
|-----------------------------------------------------------------------|---------------------------------------|----------------------------------------------------------|
| Manifest `ir-access` completeness                                     | `#1`                                  | `DEV-002`, `TASK-121`                                    |
| Runtime access-audit feeding and undeclared-access rejection          | `#2`, `#8`, `#17`                     | `DEV-003`, `TASK-123`, `TASK-124`                        |
| Claim-transition enforcement and related scheduler conflict semantics | `#3`, `#11`, `#18`                    | `DEV-004`, `TASK-125`, `TASK-126`                        |
| Non-planar Z-envelope enforcement                                     | `#4`, `#20`                           | `DEV-005`, `TASK-127`                                    |
| Prepass and layer boundary correctness                                | `#5`, `#6`, `#14`                     | `DEV-006`, `DEV-025`, `TASK-128`, `TASK-129`, `TASK-130` |
| Manifest config-schema completeness                                   | `#7`                                  | `DEV-008`, `TASK-122`                                    |
| Benchy feature parity and regression coverage                         | `#14a`, `#14b`, `#14c`, `#14d`, `#25` | `DEV-009`, `TASK-120a` to `TASK-120d`, `TASK-135`        |
| Progress-event evidence and Python bridge follow-up                   | `#23`, `#24`                          | `DEV-010`, `DEV-024`, `TASK-136`, `TASK-137`             |
| Phase G status drift and dead `Noop*Runner` cleanup                   | `#12`                                 | `DEV-020`, `TASK-139`                                    |
| Acceptance-gate closure and deviation-registry hygiene                | `#15`, `#16`                          | `DEV-010`, `DEV-026`, `DEV-030`, `TASK-140`, `TASK-141`  |

## Deletion Rationale

The retired XML and audit-task files were useful as temporary working surfaces while the deviation audit was in progress. They became liabilities once the audit results were synchronized into the live docs because they preserved stale numbering, stale blocker summaries, and duplicate status claims that could drift away from `docs/DEVIATION_LOG.md` and `docs/07_implementation_status.md`.

This file exists so the repository retains the audit story without keeping those temporary working files as live reference material.
