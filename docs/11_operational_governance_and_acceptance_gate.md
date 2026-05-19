# ModularSlicer — Operational Governance & Architecture Acceptance Gate

This document is normative for release governance and architecture acceptance decisions.

---

## 1) Module & Claim Rollout Checklist (Required)

Use this checklist when introducing a new module, changing claim ownership, or changing default enablement.

### A. Pre-merge checks

- Module manifest has unique reverse-domain ID.
- `claims.holds`, `claims.requires`, `compatibility.incompatible-with`, and `compatibility.requires` are declared.
- `wit-world`, `min-host-version`, `min-ir-schema`, and `max-ir-schema` are set.
- Declared `ir-access.reads`/`writes` are least-privilege and validated.
- New/changed config keys are namespaced and documented.

### B. Safety checks

- No new non-deterministic write conflict introduced.
- Claim uniqueness remains valid per `(layer, object, region, claim)`.
- No per-layer claim holder transitions for same object unless explicitly supported by stage contract.
- Module failure behavior is classified (`fatal` vs `non-fatal`) and tested.

### C. Rollout mode

- Release N: ship module disabled by default.
- Release N+1: enable for opt-in profile(s) only.
- Release N+2: allow default enablement if no unresolved regressions.

### D. User-facing requirements

- Migration notes include config changes and any claim conflicts users may encounter.
- `slicer validate` reports actionable remediation for conflicts.
- Frontend warning text exists for degraded success (`degraded=true`).

---

## 2) Compatibility Policy (Host / WIT / IR / Manifest)

Compatibility decisions must use all four dimensions:

1. Host semver (`min-host-version`)
2. WIT world compatibility (package + major version)
3. IR schema range (`min-ir-schema <= host < max-ir-schema`)
4. Manifest schema compatibility

### Policy rules

- Additive fields/variants: minor bumps.
- Rename/remove/type change: major bump.
- Host must reject incompatible modules at startup with explicit diagnostics.
- Compatibility shims are allowed only for one major host line and must be documented.

### Required startup diagnostics

- Expected vs actual host version
- Expected vs actual WIT world version
- Expected IR range vs host IR version
- First blocking symbol/path causing incompatibility

---

## 3) Architecture Acceptance Gate (Release Blocking)

A release candidate is approved only if all categories below are PASS.

All numeric thresholds are defined in:

- `./12_architecture_gate_metrics.md`

Evidence artifacts must be stored under:

- `./docs/evidence/<release-id>/`

<!-- VERIFY: `docs/evidence/` does not exist at the time of writing — it
     will be created when evidence is first staged for a gate decision. -->


## Gate Rubric

| Category         | PASS Criteria                                                                                              | Evidence                                               |
|------------------|------------------------------------------------------------------------------------------------------------|--------------------------------------------------------|
| Determinism      | Same input/config produces equivalent `LayerCollectionIR` ordering and claim holders across repeated runs. | Repeat-run diff report + deterministic conflict checks |
| Recoverability   | Fatal errors abort; non-fatal errors produce degraded success with mandatory telemetry.                    | Event logs + failure-injection tests                   |
| Resource Bounds  | Layer count, memory, timeout budgets enforced with explicit failure behavior.                              | Budget tests + runtime metrics                         |
| Coupling Control | No undeclared IR reads/writes; manifest access masks are enforced.                                         | Validation output + contract tests                     |
| Compatibility    | Module load checks cover Host/WIT/IR/manifest compatibility matrix.                                        | Startup validation logs                                |
| Operability      | Progress events emitted per schema and surfaced by frontend.                                               | JSON schema validation + UI integration check          |

### Gate Decision States

- `PASS`: all categories pass with no open critical deviations.
- `CONDITIONAL`: only medium/low deviations with documented mitigation and owner/date.
- `FAIL`: any critical deviation, unknown determinism issue, or missing recoverability evidence.

### Conditional Decision Workflow (Normative)

When gate state is `CONDITIONAL`, all items below are mandatory:

1. Create a mitigation record per deviation with owner, due date, and rollback trigger.
2. Assign a single decision owner (`Architecture Owner` or delegated `Release Owner`).
3. Set an escalation SLA: unresolved conditional items must be re-evaluated within 7 calendar days.
4. Record explicit ship/no-ship scope constraints (for example feature flags or disabled modules).
5. Add links to mitigation evidence in `./docs/evidence/<release-id>/conditional/`.

`CONDITIONAL` automatically downgrades to `FAIL` if:

- due date passes without mitigation evidence,
- a conditional item is reclassified to high/critical risk,
- determinism or recoverability evidence becomes stale after code changes.

---

## 4) Deviation Handling

Any intentional deviation from architecture docs must include:

- Deviation ID (sequential `DEV-NNN`, e.g. `DEV-014`, matching the existing
  numbering in `docs/DEVIATION_LOG.md`)
- Affected `docs/` sections
- Risk classification (`critical|high|medium|low`)
- Rationale
- Mitigation plan and target closure date
- Owner

Critical deviations block release unless explicitly waived by architecture owner.

Deviation records are maintained in:

- `./docs/DEVIATION_LOG.md`

No gate decision may be marked `PASS` if `DEVIATION_LOG.md` contains open critical entries.

---

## 5) Release Checklist (Minimum)

- Architecture Acceptance Gate result recorded in implementation status.
- No unresolved critical deviations.
- Scenario traces from `./docs/10_glossary_and_scenario_traces.md` validated against current implementation.
- Compatibility matrix checks executed on representative module set.
- Performance and memory targets re-verified on reference fixture set.
