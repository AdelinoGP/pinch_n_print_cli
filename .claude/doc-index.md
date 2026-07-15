# Doc Index

**When to read this:** when you need to find which doc covers a specific topic (architecture, IR schemas, WIT contracts, scheduler, coordinate system, debugging, etc.). Read these docs directly rather than relying on summaries — they are kept current and authoritative.

Keywords: docs, architecture, IR schemas, WIT, scheduler, coordinate system, glossary, deviations, debugging

---

- `docs/00_project_overview.md` — vision and scope.
- `docs/01_system_architecture.md` — pipeline tiers, data ownership, claim system, memory model, module search path.
- `docs/02_ir_schemas.md` — every IR struct (`MeshIR`, `LayerPlanIR`, `SliceIR`, `SupportPlanIR`, etc.) with versioning rules.
- `docs/03_wit_and_manifest.md` — WIT worlds, host-boundary enforcement, module manifest TOML schema, config validation.
- `docs/04_host_scheduler.md` — DAG validation, four-phase execution, error handling.
- `docs/05_module_sdk.md` — SDK helpers, `#[slicer_module]` macro, builder lifecycles for module authors.
- `docs/07_implementation_status.md` — current phase, task backlog, deviation log. **Source of truth for what's blocked / in progress.**
- `docs/08_coordinate_system.md` — unit system, Z convention, OrcaSlicer porting hazards.
- `docs/09_progress_events.md` — structured runtime event contract.
- `CONTEXT.md` — project glossary (concept-level vocabulary).
- `docs/10_scenario_traces.md` — normative end-to-end scenario traces.
- `docs/11_operational_governance_and_acceptance_gate.md` — Architecture Acceptance Gate criteria and release governance.
- `docs/12_architecture_gate_metrics.md` — objective thresholds for the gate.
- `docs/13_slicer_helpers_crate.md` — polygon/geometry utilities in `slicer-helpers`.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — registered deviations from architecture docs.
- `docs/16_slicer_report.md` — HTML slicer report format, allocator contract, known v1 limitations.
- `docs/17_agent_debugging.md` — agent-facing guide for `pnp_cli slice --instrument-stderr`, `pnp_cli dag <subcommand>`, and `pnp_cli module diagnose`. Paired skill: `.claude/skills/debug-pipeline/SKILL.md`; subagent: `.claude/agents/debug-pipeline.md`.