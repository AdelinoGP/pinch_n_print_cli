---
status: implemented
packet: fill-role-claims
task_ids:
  - TASK-167
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: fill-role-claims

## Goal

Introduce four scheduler claims — `claim:top-fill`, `claim:bottom-fill`, `claim:bridge-fill`, `claim:sparse-fill` — that let users mix infill modules per surface role at `Layer::Infill`. Each existing infill module declares which claims it holds; users select the holder per claim through global config (with per-region overrides via `RegionMapIR`). Validation pass 2 (claim-conflict detection in `docs/04 §Phase 3`) catches double-holders at startup.

This is the "Option β" architecture chosen in plan review: multiple single-pattern modules coexisting with one claim each per role, rather than a single multi-pattern monolithic generator.

## Scope Boundaries

- In scope:
  - registering the four `claim:*-fill` IDs in the host claim catalog (`crates/slicer-host` or `crates/slicer-ir`)
  - extending the manifest schema (`docs/03_wit_and_manifest.md`) to allow `[claims].holds` declarations naming any of the four claims
  - updating `rectilinear-infill`, `gyroid-infill`, `lightning-infill` manifests to declare their default-holder claims
  - extending `rectilinear-infill` runtime logic to filter emitted paths by the claims it holds (e.g. when only `claim:sparse-fill` is held, the module skips top/bottom/bridge regions)
  - global config keys `claims.top-fill`, `claims.bottom-fill`, `claims.bridge-fill`, `claims.sparse-fill` selecting the holder module per claim
  - per-region override support via existing `RegionMapIR` config plumbing (no new plumbing — reuses packet 35's work)
  - scheduler validation pass-2 confirming exactly one effective holder per claim per region
  - new TDD coverage: claim-conflict detection (negative case), per-claim emission (positive case), per-region override
- Out of scope:
  - introducing new infill patterns (concentric, hilbert) — separate packets per pattern
  - bridge-detector parity (packet 36; this packet runs after it)
  - top-surface ironing (packet 38)
  - changing the role-to-G-code-marker mapping in the emitter (already correct for the four roles)
  - WIT signature changes (claims are scheduler concerns, not WIT type concerns)

## Prerequisites and Blockers

- Depends on:
  - packet `35_multi-layer-top-bottom-thickness` — provides the `RegionMapIR` config plumbing pattern this packet reuses for per-claim per-region overrides
  - packet `12-rev1_external-surface-classification-at-slice` — provides the surface flags this packet's modules read to decide which paths to emit
  - packet `36_bridge-detector-orca-parity` — provides the `bridge_areas` field that `claim:bridge-fill` holders consume
- Unblocks:
  - none directly; future "alternative pattern" packets (e.g. concentric-top-fill, hilbert-top-fill) plug into this claim slot system
- Activation blockers:
  - packets 12-rev1, 35, and 36 must all be `implemented`
  - claim-conflict validation infrastructure (validation pass 2 in `docs/04`) must exist — confirmed by FACT in Step 0; if absent, scope expands to add it

## Acceptance Criteria

- **Given** the four claim IDs `claim:top-fill`, `claim:bottom-fill`, `claim:bridge-fill`, `claim:sparse-fill` registered in the host claim catalog, **when** the catalog is queried, **then** all four IDs are present and validation pass 2 recognizes them as conflict-eligible. | `cargo test -p slicer-host --test fill_role_claims_tdd four_fill_claims_registered_in_catalog -- --exact --nocapture`
- **Given** a workspace with only `rectilinear-infill` enabled (default), **when** `Layer::Infill` runs on a region whose flags are `(is_top_surface=true)`, **then** `InfillIR.regions[0].solid_infill` contains at least one `TopSolidInfill` path emitted by `rectilinear-infill` (which holds all four claims by default). | `cargo test -p slicer-host --test fill_role_claims_tdd default_rectilinear_holds_all_claims_emits_top -- --exact --nocapture`
- **Given** a workspace where `gyroid-infill` is configured to hold `claim:sparse-fill` (overriding `rectilinear-infill`'s default), **when** `Layer::Infill` runs on a region whose flags are `(is_top_surface=false, is_bottom_surface=false, is_bridge=false)`, **then** `InfillIR.regions[0].sparse_infill` contains paths emitted by `gyroid-infill` (recognizable by gyroid-style waveform) AND `rectilinear-infill` emits ZERO sparse paths. | `cargo test -p slicer-host --test fill_role_claims_tdd gyroid_holds_sparse_claim_only_emits_sparse -- --exact --nocapture`
- **Given** the same workspace, **when** `Layer::Infill` runs on a region with `(is_top_surface=true)`, **then** `rectilinear-infill` emits the `TopSolidInfill` paths (it still holds `claim:top-fill`) AND `gyroid-infill` emits no top-surface paths. | `cargo test -p slicer-host --test fill_role_claims_tdd gyroid_does_not_emit_for_unheld_top_claim -- --exact --nocapture`
- **Given** a per-region override that switches `claim:sparse-fill` to `lightning-infill` for a specific region, **when** `Layer::Infill` runs on that region, **then** `lightning-infill` emits the sparse paths AND `gyroid-infill` does not (for that region only). | `cargo test -p slicer-host --test fill_role_claims_tdd region_override_redirects_claim_to_alternate_holder -- --exact --nocapture`
- **Given** an unmodified Benchy run with default config (`rectilinear-infill` holds all four claims), **when** the slicer produces G-code, **then** the output still contains `;TYPE:Top surface`, `;TYPE:Bottom surface`, `;TYPE:Bridge infill`, AND `;TYPE:Sparse infill` blocks (no role family disappears). | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_default_claims_emit_all_role_families -- --exact --nocapture`

## Negative Test Cases

- **Given** a workspace where both `rectilinear-infill` and `gyroid-infill` declare `[claims].holds = ["claim:sparse-fill"]` AND no region override resolves the conflict, **when** scheduler validation pass 2 runs, **then** validation fails with `SchedulerError::ClaimConflict { claim: "claim:sparse-fill", module_a, module_b, scope: ConflictScope::Global }` AND no slicing occurs. | `cargo test -p slicer-host --test fill_role_claims_tdd two_holders_for_one_claim_fails_validation -- --exact --nocapture`
- **Given** a workspace with NO module holding `claim:top-fill`, **when** scheduler validation pass 2 runs, **then** it returns a `MissingDependency` (or equivalent) for the `top-fill` capability AND no slicing occurs. | `cargo test -p slicer-host --test fill_role_claims_tdd missing_holder_for_top_fill_claim_fails_validation -- --exact --nocapture`
- **Given** a manifest declaring `[claims].holds = ["claim:invalid-fill"]` (not in the catalog), **when** Phase-1 manifest ingestion runs, **then** a structured `LoadError` names the unknown claim ID and the manifest path. | `cargo test -p slicer-host --test fill_role_claims_tdd unknown_claim_in_manifest_is_load_error -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — `[claims].holds` and `[claims].requires` manifest schema. Read directly; § "Manifest Schema" only.
- `docs/04_host_scheduler.md` — § "Phase 3 — DAG Validation" (claim conflict detection); § "Claim Resolution with Runtime Disable Rules"; § "Composable Multi-Writer Patterns". Delegate SUMMARY for each section needed.
- `docs/02_ir_schemas.md` — `RegionMapIR.entries[*].config` for per-region claim overrides.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills()` per-surface-role pattern selection (lines ~`926-1208`). Delegate SUMMARY ≤ 200 words for "how Orca picks pattern by `stTop`/`stBottom`/`stBridge` and per-region config".
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::new_from_type()` factory dispatching to specific `FillBase` derivatives. Delegate FACT for: factory function name and the enum-to-class map.

All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list
- delegate every cargo run, every doc read, every OrcaSlicer reference
- stop reading at 60% context and hand off at 85%

This packet's risk is in the scheduler validation surface — read `docs/04 §Phase 3` only via SUMMARY dispatch; do not load the full file.
