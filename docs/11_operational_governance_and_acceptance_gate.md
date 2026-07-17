# Pinch 'n Print — Operational Governance & Architecture Acceptance Gate

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
2. WIT world compatibility (package name; unversioned — see `docs/03` §"Why `wit-world` carries no version")
3. IR schema range (`min-ir-schema <= host < max-ir-schema`)
4. Manifest schema compatibility

### Policy rules

- Additive fields/variants: minor bumps.
- Rename/remove/type change: major bump. Adding a parameter to an existing WIT
  export is a type change, hence major — packet 130 shipped `prior-infill` on
  `run-infill-postprocess` as `1.1.0` before this was caught, and it is now
  `2.0.0`.
- **WIT world versions are advisory.** They are erased from guest binaries at
  compile time, so no check anywhere compares them; they document intent in the
  `.wit` `package` line only. Do not treat a minor world bump as a compatibility
  guarantee: every world change is currently breaking for every module bound to
  that world (`docs/05` §SDK Versioning). IR schema versions (dimension 3) *are*
  enforced, fatally, at startup.
- Host must reject incompatible modules at startup with explicit diagnostics.
- Compatibility shims are allowed only for one major host line and must be documented.
- Host semver (dimension 1) is enforced: startup DAG validation pass 14 (`HostVersionCompatibility`, `crates/slicer-scheduler/src/validation.rs`) compares each loaded module's declared `min-host-version` against the running host version and fails fatally (`SchedulerError::HostVersionIncompatible`) if the host is older than required — see `docs/04_host_scheduler.md` §"Validation Passes (in order)" pass 14. Closes DEV-026 gap (1).

### Required startup diagnostics

- Expected vs actual host version
- Expected vs actual WIT world version
- Expected IR range vs host IR version
- First blocking symbol/path causing incompatibility

### CLI output wire contracts

JSON emitted by `pnp_cli` subcommands for external consumers (e.g. the
`pinch_n_print_studio` frontend) carries a top-level `schema_version` string
that follows the same bumping rules as the IR/WIT/manifest dimensions above:
additive fields → minor; rename / remove / type change / semantic shift →
major; clarifications → patch. Format is semver
`"<major>.<minor>.<patch>"`; consumers gate on the major component only.

Active wire-version constants:

- `CONFIG_SCHEMA_WIRE_VERSION` in `crates/slicer-scheduler/src/manifest.rs` —
  versions the JSON emitted by `pnp_cli module config-schema`.
- `PROGRESS_EVENT_SCHEMA_VERSION` in `crates/slicer-runtime/src/progress_events.rs` —
  versions the JSONL stream emitted by `pnp_cli slice --instrument-stderr`.

The two constants are independent: each tracks the wire surface of its own
emitter. Adding a new key to either JSON shape bumps only the relevant
constant's minor.

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
- Scenario traces from `./docs/10_scenario_traces.md` validated against current implementation.
- Compatibility matrix checks executed on representative module set.
- Performance and memory targets re-verified on reference fixture set.

## 6) Test Fixture Determinism Contract (Normative — Packet 90)

All test fixtures committed to the repository must be regeneratable from a deterministic authoring procedure. The contract is binding because regression assertions hash fixture-derived outputs (G-code SHAs, slice extents, paint state) and a fixture that varies across regenerations would invalidate every downstream test using it.

Rules:

- **Pin a canonical SHA-256 at commit time.** Every fixture file carries an explicit hash in its packet closure-log (and in the file's authoring-procedure comment block where applicable). Reviewers can rerun the procedure and compare.
- **Prefer parametric scripts.** A small Python emitter that writes binary STL/3MF deterministically (IEEE 754 bit patterns, explicit byte order, no timestamp metadata) is the canonical pattern. Packet 90's `regression_wedge.stl` authoring procedure (`.ralph/specs/90_regression-wedge-stl-swap/closure-log.md` §Authoring Procedure) is the worked example.
- **If determinism is unavoidably broken** (e.g. third-party tool with non-deterministic output), document the source explicitly: tool name, version, OS, timestamp seed. Pin the resulting SHA and treat the tool version as part of the contract.
- **Regeneration instructions stay in the closure-log.** They are NOT inlined into source comments (would bloat the artefact); they ARE referenced from the related test file with a one-line pointer.
- **Verification gate.** Before any fixture is replaced, regenerate it from the documented procedure and confirm SHA equivalence. If the fixture must change shape (new feature, new geometry), pin a NEW SHA in the closure-log along with a rationale; do not silently update the old one.

Recorded as a contract because Packet 90's investigation surfaced that fixture-regeneration practice had drifted across packets; future fixture authoring follows this rule by default.
