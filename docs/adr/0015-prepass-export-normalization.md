# ADR-0015 — PrePass Export Normalisation: ConfigView + Output Resource + `result<_, module-error>`

## Status

Accepted (Packet 73 / TASK-163c).

## Context

Four PrePass stage exports — `run-layer-planning`, `run-seam-planning`, `run-support-geometry`, `run-paint-segmentation` — emerged at different times across packets 23-rev1 (seam-planning), 30 (support-geometry foundation), 42–43 (paint-segmentation parity), and earlier. By the end of 2026 Q1 they had drifted into three different export shapes:

1. Some took a `config-view` parameter; one (`run-support-geometry`) had `_config` ignored at the host with an empty `ConfigView` injected.
2. Some wrote through a named output resource (`layer-plan-output`, `seam-planning-output`); others returned a bare record.
3. Some returned `result<_, module-error>` and surfaced fatals as `DispatchError`; one silently swallowed module fatals inside the macro/host glue and continued.

Concretely: `run-support-geometry` was the last holdout. It still ran with an empty ConfigView (so `enable_support = false` and `support_raft_layers = N` were both invisible to the planner) and its fatals were caught and discarded by the host. The misalignment had real consequences — disabling support at the config layer still ran the planner; planner panics were observable only as missing geometry, not as errors.

Packet 73 normalised the boundary across all four exports. The normalisation is sufficiently load-bearing for future stages that it deserves an ADR rather than a one-time packet note.

## Decision

**All PrePass stage exports follow a uniform contract:** `func(config: config-view, ..., output: <stage-output>) -> result<_, module-error>`.

- **`config: config-view`** — every PrePass export accepts a `ConfigView` carrying the keys the module declared in its `[config.schema]`. Modules with no schema receive an empty view. Keys absent from the view return `None` on lookup. No more empty-injection.
- **Named output resource** — each stage gets a named builder resource (e.g. `layer-plan-output`, `seam-planning-output`, `support-geometry-output`, `paint-segmentation-output`). Modules push entries through resource methods; the host harvests the output into the corresponding IR at commit time. Bare-record returns are forbidden.
- **`result<_, module-error>` return** — errors are explicit at the WIT level. The host's runner converts `module-error` into the runtime's `PrepassRunnerError` and ultimately into `DispatchError` via `From` impls (lossless variant remap). The macro/host glue MUST NOT catch and discard module fatals; they propagate up to the slice command.

The contract applies to every existing PrePass export and every future one.

## Consequences

- **Config-driven planner behaviour works end-to-end.** `enable_support = false` actually disables the planner. `paint_order:` overrides reach the paint-segmentation kernel. Configuration becomes a real control surface for prepass modules.
- **Errors are visible.** A `support-planner` panic surfaces as `Err(DispatchError::PrepassFatal { stage, module, source })`. The user sees a stack-traceable error path; the host does not silently produce broken geometry.
- **The four stage runners and their `*Output` resources are symmetric.** Authoring a new PrePass stage is a copy-edit job: define the WIT export with this shape, define the output resource, harvest into the IR, done. No exception cases.
- **Backwards compatibility is finite.** A prepass module shipped before packet 73 that depended on the empty-ConfigView behaviour is now broken in a defensible direction: it should have been honouring the config it declared all along. No grace-period shim is offered.
- **`PrepassRunnerError` is the canonical narrow error type.** It lives in `slicer-ir`; `slicer-runtime`'s broader `PrepassExecutionError` implements `From<PrepassRunnerError>` with a one-to-one variant mapping. Future runner error types follow this split-error pattern (ADR-0005 codified it for the runner-trait layer; this ADR pins it for prepass specifically).

## Rejected alternatives

- **Per-stage exception cases.** "Support-geometry has historically not used ConfigView; let's keep that working." Rejected — the historical reason was an implementation gap, not a design choice, and the gap had user-visible cost.
- **Backwards-compatible empty-ConfigView fallback for modules with no schema.** Already provided — modules with no `[config.schema]` receive an empty view at the API. The deviation was that the host was injecting empty views even when the module declared a schema. That's what the normalisation kills.
- **Allow bare-record returns alongside resource-based output.** Rejected for symmetry and to keep the harvesting path uniform.

## Future reviewers

- Do not propose adding a "compat mode" empty-ConfigView injection for older modules; the module is the source of truth for what it reads.
- Do not catch module fatals in the macro or host glue. If a fatal needs translation (e.g. for diagnostic enrichment), translate it into a richer `DispatchError` variant — do not swallow.
- New PrePass stages must adopt this contract from day one; no exceptions.
