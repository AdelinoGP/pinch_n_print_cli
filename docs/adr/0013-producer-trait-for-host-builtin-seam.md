# ADR-0013 — Producer Trait for Externalising Host Built-Ins onto the Scheduler Seam

## Status

Accepted (Packet 69 / TASK-213).

## Context

Eight pipeline stages are implemented as host-synthesised "builtin producers" rather than WASM guests: `MESH`, `MESH_ANALYSIS`, `REGION_MAPPING`, `SLICE`, `SHELL_CLASSIFICATION`, `SUPPORT_GEOMETRY`, `PAINT_SEGMENTATION`, and `GCODE_EMIT`. Each of these has the same "shape" as a real module from the scheduler's perspective (it writes a slot in the Blackboard, it has IR-access claims, it participates in topological ordering) but lives natively in `slicer-runtime` rather than in a `.wasm` artefact.

Before Packet 69, the scheduler's DAG validator (`validate_startup_dag`) and the `pnp_cli dag` introspection commands (`run_dag_stages`, `run_dag_stage`, `run_dag_depends`, `run_dag_claims`) only saw `LoadedModule` instances. The host built-ins were threaded through as ad-hoc synthetic rows whose claim-visibility was incomplete: `dag claims` reported empty for stages whose primary writer was a built-in, and `dag depends` could not show why a guest module read from a builtin-produced slot.

The packet 69 pnp-cli unification pass exposed this gap as a sharp edge: the new `pnp_cli dag` subcommand promised a single source of truth for what writes what, but four of the eight built-ins were invisible to that source. A trait projection was needed.

## Decision

**A `Producer` trait is the smallest shared projection of `LoadedModule` and host built-ins that the scheduler's DAG layer needs.** Concretely:

- The trait exposes the four fields the validator and `dag_cli` actually consume: stage, claims, IR-reads, IR-writes. It is intentionally narrow — no module-id, no manifest, no instance pool.
- `LoadedModule` and each host builtin descriptor implement `Producer`. A blanket `impl Producer for &LoadedModule` preserves all existing call sites; existing module-walking code continues to compile unchanged.
- The four DAG functions (`validate_startup_dag`, `build_intra_stage_dag`, plus the four `run_dag_*` helpers) take `&dyn Producer` slices instead of `&[LoadedModule]`.
- Host built-in descriptors live alongside their algorithms in `slicer-runtime/src/builtins/` and are wired into the scheduler's `producers()` accessor next to the loaded modules.

This makes the synthetic rows visible to `dag_cli` without requiring the host built-ins to pretend they are loadable modules.

## Consequences

- **`pnp_cli dag claims` and `pnp_cli dag depends` are now complete.** A guest reading `PaintRegionIR` sees that the canonical writer is `paint-segmentation` (when a module is loaded) or `PAINT_SEGMENTATION_PRODUCER` (when host built-in handles the stage).
- **The validator's claim-visibility check is uniform.** No more conditional walks for "is this a real module or a synthetic row?".
- **Future host-service abstractions can follow the same pattern.** A future "pluggable repair / decimation module" that replaces a host built-in can simply implement `Producer` instead of forcing the host to pretend it's a `LoadedModule`.
- **Trait surface is small on purpose.** Anything beyond stage/claims/IR-access stays out of `Producer`. New cross-cutting needs must be added consciously and not because "I needed this field over there".

## Rejected alternatives

- **Continue passing synthetic `LoadedModule` rows for host built-ins.** Forces every host builtin to fabricate a `wasm_path`, `instance_pool`, `instance` field, etc. Visible to the validator only as `Option<None>` which broke dispatch invariants.
- **Make `dag_cli` consult two parallel lists (modules and host built-ins) and union them at every render site.** Multiplies the number of code paths that can drift; one of the reasons the gap existed in the first place.
- **A wider trait covering instantiation + dispatch.** Mixes scheduling and execution concerns; rejected for the smallest-useful-projection reason.

## Future reviewers

- Do not extend `Producer` to cover dispatch or execution concerns; those live on `LoadedModule` and the runner traits in `slicer-wasm-host` (ADR-0005).
- Do not move host built-in descriptors out of `slicer-runtime/src/builtins/`; the locality with the algorithm body is intentional.
- New stages that could be either a host built-in or a guest module should implement `Producer` from day one, regardless of which form ships first.
