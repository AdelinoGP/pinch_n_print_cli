# Packet 76 — Requirements

## Source

Architecture depth review of `./crates` (HTML report, 2026-05-30) →
grilling session (`/grill-with-docs`) → approved plan
`cryptic-cuddling-minsky.md`. Candidates 1, 2, 3 of 8.

## In scope

1. **Single sources of truth (Candidate 3).** Four invariants each had 2–4
   drifting copies:
   - config-key wildcard matching (`prefix:*`) — 2 copies in `execution_plan.rs`.
   - `ResolvedConfig` → flat config map — 2 *divergent* copies (`gcode_emit`,
     `dispatch`); the dispatch copy was missing 7 keys.
   - 4×4 affine point transform — 4 copies with subtly different semantics
     (zero-matrix guard present in 2, perspective w-divide present in 1).
   - canonical stage order — `execution_plan::STAGE_ORDER` is authoritative, but
     `manifest::known_stage_ids` and `validation::stage_order_index` kept drifted
     copies; the latter omitted 3 stages.

2. **Region-mapping double-stamp (Candidate 1a).** `commit_region_mapping_builtin`
   ran the inner builder then *re-ran* modifier+paint stamping with a different
   base, discarding the inner result. Collapsed to a single pass via a host-config
   authority threaded into the inner builder.

3. **Pipeline body dedup (Candidate 1b).** Three `run_pipeline_*` entry points;
   two had byte-identical bodies → shared `run_pipeline_core`. The third
   (`run_pipeline_with_events`) is deliberately *not* merged: it emits bare gcode
   with no CONFIG_BLOCK, a real behavioural difference (locked by `pipeline_tdd`).

4. **Macro conversion DRY (Candidate 2).** Per-world WIT↔IR/SDK converters in
   `slicer-macros` duplicated across worlds → generated from one emitter as
   `From`/`Into` impls. Stays per-world (ADR-0003).

## Correctness changes (deliberate, test-backed)

- **3b:** the per-region module `ConfigView` now carries 7 additional canonical
  keys. Safe because dispatch builds the view via
  `ConfigView::from_declared(map, module.declared_keys)` — modules only ever see
  keys they declared.
- **3d:** a module manifest declaring `PrePass::SeamPlanning`,
  `PrePass::SupportGeometry`, or `Layer::PaintRegionAnnotation` previously failed
  startup DAG validation with `UnknownStage`; it is now accepted. Genuinely
  misspelled stages are still rejected.

## Out of scope

- Depth-review candidates 4–8 (phase-type unify, SDK builders, self-asserting IR,
  runtime module grouping, CLI↔runtime seam).
- Shared-crate extraction of macro conversions (forbidden by per-world
  `wit_bindgen::generate!`; see ADR-0003).
- Any change to canonical WIT or the component ABI.

## Non-functional

- Zero clippy warnings under `--all-targets -- -D warnings`.
- Guest ABI stable; `cargo xtask build-guests --check` clean at close.
- Narrow-test discipline during iteration; full `--workspace` only at close.
