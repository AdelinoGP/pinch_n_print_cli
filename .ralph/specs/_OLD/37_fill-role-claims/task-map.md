# Task Map: fill-role-claims

Maps backlog task IDs to concrete artifacts produced by this packet.

## TASK-167 — fill-role claim system (Option β)

| Artifact | Path | Role |
| --- | --- | --- |
| Claim catalog | `crates/slicer-host/src/validation.rs` (`FILL_CLAIM_IDS`) | Registers the four `claim:*-fill` IDs and conflict eligibility. |
| Manifest validation | `crates/slicer-host/src/manifest.rs` (`validate_claim_ids`) | Rejects unknown `claim:*` IDs at Phase-1 ingestion. |
| Resolver | `crates/slicer-host/src/validation.rs` (`resolve_held_claims`) | Pure function: `(module_id, declared, config) -> effective held set`. |
| Dispatch wiring | `crates/slicer-host/src/dispatch.rs` (`dispatch_layer_call`); `crates/slicer-host/src/execution_plan.rs` (`CompiledModule.claims`) | Plumbs declared claims and resolved holders into `HostExecutionContext` per call. |
| WIT boundary | `wit/deps/ir-types.wit` (`slice-region-view.held-claims`) | Exposes the resolved held-claim set across the WASM boundary (registered deviation DEV-042). |
| Host record | `crates/slicer-host/src/wit_host.rs` (`SliceRegionData.held_claims`, `sliced_region_to_data`) | Carries held claims into the guest-visible region view. |
| SDK accessor | `crates/slicer-sdk/src/views.rs` (`SliceRegionView::held_claims`, `SliceRegionView::should_emit`) | Surface for guest modules + central convention "empty = holds all". |
| Module manifests | `modules/core-modules/{rectilinear,gyroid,lightning}-infill/<module>.toml` | Declare default holders. |
| Module runtime filters | `modules/core-modules/{rectilinear,gyroid,lightning}-infill/src/lib.rs` | Skip emission for unheld claims via `should_emit`. |
| Global config schema | `crates/slicer-ir/src/slice_ir.rs` (`ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder`) | Four String keys; default `"rectilinear-infill"`. |
| Per-region overrides | `crates/slicer-ir/src/slice_ir.rs` (`RegionMapIR.entries[*].config`) | Reused unchanged from packet 35. |
| Tests | `crates/slicer-host/tests/fill_role_claims_tdd.rs` | AC + negative cases including resolver and RegionMapIR override. |
| Benchy E2E | `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (`benchy_default_claims_emit_all_role_families`) | Default config preserves all four `;TYPE:` markers. |
| Docs | `docs/03_wit_and_manifest.md` (claim catalog table + held-claims convention); `docs/07_implementation_status.md` (TASK-167 row); `docs/DEVIATION_LOG.md` (DEV-042) | Discoverability and traceability. |
