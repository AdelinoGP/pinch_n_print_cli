---
status: implemented
packet: 76
task_ids: [TASK-220, TASK-221]
deferred_task_ids: [TASK-222]
backlog_source: docs/07_implementation_status.md
---

# Packet 76 — Runtime dedup, stage-order canon, & macro conversion DRY

## Goal

Remove duplicated single-sources-of-truth in `slicer-runtime`, fix a latent
startup-validation bug, collapse the pipeline body duplication, and DRY the
`#[slicer_module]` macro's WIT↔IR conversions — concentrating each invariant in
one place (locality) so the copies cannot drift again. Behaviour-preserving
except two deliberate, test-backed changes: (a) the per-region module config map
gains 7 previously-missing keys, and (b) modules declaring `PrePass::SeamPlanning`
/ `PrePass::SupportGeometry` / `Layer::PaintRegionAnnotation` are no longer
wrongly rejected at startup.

Derived from the architecture depth review of `./crates` (candidates 1, 2, 3).

## Scope Boundaries

The four prior stage-list copies are reconciled onto the existing canonical
`execution_plan::STAGE_ORDER` (no new list is introduced). The canonical WIT
(`crates/slicer-schema/wit/`) and component ABI are untouched. Candidate 2 edits
`crates/slicer-macros` (a universal guest dependency), so it — and only it —
requires `cargo xtask build-guests` before the guest round-trip tests are
trustworthy.

Work items (plan IDs in parentheses map to the depth-review candidates):

- **3a** — wildcard config-key matcher: `crates/slicer-runtime/src/execution_plan.rs`
  (`source_key_matches_declared`, shared by `bind_module_config_view` +
  `config_key_declared`).
- **3b** — canonical `ResolvedConfig::to_config_map`: `crates/slicer-ir/src/resolved_config.rs`;
  `gcode_emit.rs` + `dispatch.rs` delegate. Dispatch's per-region map gains
  `wall_generator`, `infill_type`, `{top,bottom,bridge,sparse}_fill_holder`,
  `support_type` (declared-key-filtered downstream, so harmless to modules).
- **3c** — canonical `slicer_core::transform_point3` (zero-guard + perspective
  divide); 4 copies (`mesh_analysis`, `paint_segmentation`, `model_loader`,
  `prepass_slice`) delegate.
- **3d** — stage-order canon: new `crates/slicer-runtime/src/stage_order.rs`
  derives `known_stage_ids` / `stage_order_index` / `tier_of` from
  `execution_plan::STAGE_ORDER`; `manifest.rs`, `validation.rs`, `dag_cli.rs`
  forward to it. Fixes the `UnknownStage` rejection of seam/support modules.
- **1a** — region-mapping single-pass stamping: `region_mapping.rs`
  (`execute_region_mapping_inner` takes a host-config authority; the commit
  re-stamp loop is deleted).
- **1b** — pipeline body dedup: `pipeline.rs` (`run_pipeline_core` shared by
  `run_pipeline_with_raw_config` + `run_pipeline_with_instrumentation`;
  `run_pipeline_with_events` keeps its distinct bare-gcode body).
**Deferred — TASK-222 (Candidate 2), reassessed as NOT recommended.** The macro
WIT↔IR conversion dedup was deferred from this packet and judged not worth
implementing now: its headline benefit (making the conversions unit-testable) is
impossible under per-world `wit_bindgen` (ADR-0003); the duplication is
compiler-guarded and low-churn (a WIT type change fails every world loudly, not
silently); and a token-emitting emitter would add macro indirection to the most
fragile file (2757-line `slicer-macros`) for pure dedup, requiring a full
~32-guest wasm rebuild to verify. The durable value — ADR-0003, which stops the
shared-crate extraction from being re-proposed — is retained. Full rationale and
the narrow opportunistic exception are in `implementation-plan.md` (TASK-222).

Out of scope: the other depth-review candidates (4–8); shared-crate extraction
of the macro conversions (forbidden by per-world bindgen — see ADR-0003); the
TASK-222 macro rewrite (deferred + not recommended, above).

## Acceptance Criteria

> Repo root `F:\slicerProject\pinch_n_print`; POSIX shell. Integration tests
> bucket into `unit|contract|executor|integration|e2e`.

**AC-3a — one wildcard matcher.**
`grep -c "fn source_key_matches_declared" crates/slicer-runtime/src/execution_plan.rs` → `1`;
`cargo test -p slicer-runtime --test contract config_view_binding_tdd` → pass.

**AC-3b — one config serializer; gcode output unchanged.**
`grep -c "fn to_config_map" crates/slicer-ir/src/resolved_config.rs` → `1`; the
two `resolved_config_to_map` sites delegate; `cargo test -p slicer-runtime --test integration gcode_header` → pass (golden unchanged);
`cargo test -p slicer-runtime --test contract` → pass.

**AC-3c — one transform.**
`grep -c "fn transform_point3" crates/slicer-core/src/lib.rs` → `1`; the 4 prior
copies delegate; `cargo test -p slicer-core --lib transform_point3` → pass.

**AC-3d — seam/support modules accepted; canon derived, not copied.**
`grep -c "CANONICAL\|STAGE_ORDER" crates/slicer-runtime/src/stage_order.rs` shows
derivation from `execution_plan::STAGE_ORDER` (no inline 19/22-entry literal in
`validation.rs`/`manifest.rs`);
`cargo test -p slicer-runtime --test unit stage_canon_seam_support_tdd` → pass;
`cargo test -p slicer-runtime --test unit dag_validation_tdd` → pass.

**AC-1a — single-pass region stamping; modifier e2e unchanged.**
`grep -c "host_config: Option" crates/slicer-runtime/src/region_mapping.rs` → `1`
(positive structural evidence: `execute_region_mapping_inner` now accepts the
host-config authority parameter that lets stamping happen once inside the inner
builder, replacing the prior post-hoc re-stamp loop in
`commit_region_mapping_builtin`);
`cargo test -p slicer-runtime --test e2e` → pass.

**AC-1b — shared pipeline core; entry semantics preserved.**
`grep -c "fn run_pipeline_core" crates/slicer-runtime/src/pipeline.rs` → `1`;
`cargo test -p slicer-runtime --test integration pipeline_tdd` and
`--test contract dispatch_tdd` → pass.

**AC-2 — DEFERRED (TASK-222, not recommended).** No macro change ships in this
packet; TASK-220/221 touch no guest input, so this packet does not affect guest
`.wasm` freshness.

**AC-GATE — clippy + targeted suites.**
`cargo clippy --workspace --all-targets -- -D warnings` → exit 0.

**AC-CLOSE — full suite (packet close).**
Full `cargo test --workspace` via sub-agent with a pass/fail FACT, after
`cargo xtask build-guests --check` is clean.

## Doc Impact Statement

Two additions, no edits to authoritative `docs/<NN>_*.md`:

- `docs/adr/0003-macro-per-world-wit-conversions.md` — new ADR documenting the
  per-world `wit_bindgen::generate!` invariant that blocks shared-crate
  extraction of `#[slicer_module]`'s WIT↔IR conversions (counterpart to host-side
  ADR-0002). Verification: `rg -q '^# ADR-0003' docs/adr/0003-macro-per-world-wit-conversions.md`.

No `docs/<NN>` edits are required:

- **docs/02 (IR schemas)** — `ResolvedConfig::to_config_map` is an additive
  method on an existing type; the 7 newly-included keys (`wall_generator`,
  `infill_type`, `{top,bottom,bridge,sparse}_fill_holder`, `support_type`)
  already exist as `ResolvedConfig` fields and are documented at the field
  level. No schema-version bump, no IR field rename.
- **docs/03 (WIT/manifest)** — canonical WIT and manifest schema are untouched;
  module manifests do not change.
- **docs/04 (host scheduler)** — `STAGE_ORDER` already encodes the runtime
  order (SupportGeometry-after-RegionMapping, PaintRegionAnnotation slot, etc.).
  `stage_order.rs` derives all helpers from `STAGE_ORDER`; no new stage IDs,
  no order change. The bug fix is removal of two parallel short copies, not a
  contract change.
- **docs/05 (module SDK)** — `#[slicer_module]` macro behaviour, public SDK
  surface, and module authoring rules are unchanged. TASK-222 (macro DRY) is
  deferred + not recommended; ADR-0003 is the only durable artifact.

## ADRs

- **ADR-0003** (`docs/adr/0003-macro-per-world-wit-conversions.md`) — Candidate 2:
  guest-side WIT↔IR conversions stay generated per-world (counterpart to the
  host-side ADR-0002).
