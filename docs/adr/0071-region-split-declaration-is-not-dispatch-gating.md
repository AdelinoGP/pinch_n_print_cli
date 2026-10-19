# ADR-0071 — Region-Split Declaration Is Not Dispatch Gating; `paint_only` Is the Opt-In

## Status

Accepted (2026-09-25; region-split core activation).

## Context

Packet 92 introduced the top-level `[[region_split]]` manifest array and a
per-layer host dispatch filter: a module declaring a non-empty semantic set is
skipped on a layer where no region's `variant_chain` mentions any of them. The
filter reads `CompiledModuleStatic::region_split_semantics`, copied from
`LoadedModule::region_split_semantics`, which packet 92 built directly from the
`[[region_split]]` array.

That coupling was invisible while no shipped module declared
`[[region_split]]` (recorded open task TASK-532, "inert paint/tool config
axis"). It became a live defect the moment the core modules declared their
semantics: under the packet-92 coupling, declaring `material` would skip
`classic-perimeters`/`arachne-perimeters` on every unpainted layer, and
declaring `fuzzy_skin` would skip `fuzzy-skin` on unpainted layers — even
though `FuzzySkinModule::run_wall_postprocess` gates per-wall behaviour on
`self.apply_to_all || wall.feature_flags.fuzzy_skin`, so `apply_to_all = true`
is a supported mode that must run on unpainted layers. The unpainted case is
the common case; coupling declaration to filtering would silently drop walls
and fuzzy-skin output from ordinary slices.

A second, related defect: the host seeded `material` and `fuzzy_skin` into
`ExecutionPlan::aggregated_region_split` via `seed_core_region_splits`
regardless of whether any module declared them. That made the aggregate —
and therefore the RegionMapping cross-product input — diverge from module
declarations, and it would have made the core TOML declarations redundant
while forcing every manifest change to fight the seed.

## Decision

**Separate semantic activation from dispatch gating. `[[region_split]]`
activates a semantic in the cross-manifest aggregate; a new top-level
`paint_only = true` metadata is the sole opt-in that also gates per-layer
invocation. The aggregate is declaration-only: nothing is seeded.**

Concretely:

- Top-level manifest key `paint_only` (bool, default `false`). It is metadata,
  not a `[[region_split]]` field.
- `LoadedModule::paint_only` is parsed and stored. `LoadedModule::region_split_semantics`
  is the declared semantic set when `paint_only == true`, and empty otherwise.
  `LoadedModuleBuilder::region_splits(splits)` derives the set from the
  builder's `paint_only` flag in `build()`.
- `paint_only = true` with no `[[region_split]]` entry is rejected at manifest
  load with `LoadErrorKind::PaintOnlyWithoutRegionSplit` (a paint-only module
  with no semantics would be skipped on every layer). A non-boolean
  `paint_only` is a `Schema` error naming the field.
- `CompiledModuleStatic::region_split_semantics` propagates that set unchanged;
  the runtime filter (`module_invocation_allowed_on_layer` in
  `crates/slicer-runtime/src/layer_executor.rs`) is untouched and keeps its
  contract: empty set ⇒ always invoke; non-empty ⇒ invoke iff some region
  carries a matching `variant_chain` semantic.
- `aggregate_region_splits` is the whole aggregate. `seed_core_region_splits`
  is deleted; `build_execution_plan` no longer seeds `material`/`fuzzy_skin`.
- The core manifests declare their semantics and do **not** set `paint_only`:
  - `classic-perimeters.toml` and `arachne-perimeters.toml` declare
    `material @ 100` (`value_type = "tool_index"`).
  - `fuzzy-skin.toml` declares `fuzzy_skin @ 200` (`value_type = "flag"`),
    with a comment pinning why `paint_only` must stay absent
    (`apply_to_all`).

## Consequences

- **Unpainted slices are unchanged by core declaration.** Classic/arachne and
  fuzzy-skin keep an empty dispatch set and run on every layer, so the
  ordinary unpainted toolpath and fuzzy-skin `apply_to_all` mode are preserved.
- **Painted slices gain real variants.** With declarations present, the
  aggregate holds `material`/`fuzzy_skin` and RegionMapping can cross-product
  them; previously the semantics existed only via the implicit seed.
- **The aggregate is a faithful function of module declarations.** Loading the
  same modules always yields the same map; adding or removing a declaration
  changes it by exactly that declaration. The old seed could not be removed or
  overridden from manifests.
- **Community modules have an explicit contract.** A module that truly needs
  paint to function sets `paint_only = true` and gets skipped on unpainted
  layers; a module that merely wants split regions declares semantics and
  filters per-feature internally.
- **The load-time guard fails closed.** `paint_only = true` without a
  declaration is a hard error rather than a module that silently never runs.
- **Runtime is unchanged.** No WIT, SDK, config, or runtime-source surface is
  touched; the runtime filter's semantics are exactly packet 92's.

## Rejected alternatives

- **Gate dispatch directly on a non-empty `[[region_split]]` (keep packet-92
  coupling).** Rejected: breaks unpainted classic/arachne and fuzzy-skin
  `apply_to_all`.
- **Special-case the core module IDs in the runtime filter.** Rejected: puts
  manifest knowledge in the runtime, does not generalise to community modules,
  and would require editing runtime source.
- **Keep `seed_core_region_splits` and omit core declarations.** Rejected:
  leaves TASK-532's axis inert and makes the aggregate unfaithful to
  declarations; the seed's `declaring_modules` are empty, so diagnostics and
  tie-detection lose the declaring module.
- **A `paint_only` per `[[region_split]]` entry instead of top-level.** Rejected:
  dispatch is per-module × layer, so the opt-in belongs at module scope; a
  per-entry flag would invite per-semantic filtering the runtime filter does
  not implement.
- **`paint_only = true` implies an empty declaration set is a warning.** Rejected:
  a paint-only module with no semantics can never run; that is an authoring
  error, not a nudge.

## Future reviewers

- Do not re-couple dispatch to declaration presence. If a new module must be
  skipped when unpainted, set `paint_only = true`.
- Do not reintroduce implicit seeding of core semantics into
  `aggregated_region_split`; the core TOMLs are the declaration source.
- The core perimeter/fuzzy-skin manifests must NOT set `paint_only`:
  fuzzy-skin's `apply_to_all` and unpainted perimeter generation both require
  invocation on layers with no matching `variant_chain`.
