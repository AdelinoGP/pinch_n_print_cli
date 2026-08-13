---
status: implemented
packet: 205c-native-dispatch-seam
task_ids:
  - TASK-329
backlog_source: docs/specs/integrated-modules-architecture-205c-205e-plan.md (queue row 1)
context_cost_estimate: M
---

# Packet Contract: 205c-native-dispatch-seam

## Goal

Deepen the native dispatch seam so native and WASM legs use one authoritative view translation, one held-claim resolver, lossless stage commits (including per-region support origins), and an explicit load-time dispatch mode.

## Scope Boundaries

This packet owns the host/SDK native-dispatch seam: IR-to-view marshalling, held-claim resolution, native response commits, the support-origin contract (an additive WIT method plus a per-region `SupportIR` shape), and the integrated/external live-binding mode. It may edit the corresponding host, SDK, slicer-ir `SupportIR`, the slicer-runtime IR consumers required by the `SupportIR` shape change, and focused regression-test surfaces. It does not change module algorithms, edition membership, WIT package versions, or the external override rule.

## Prerequisites and Blockers

- Depends on: `205b-native-transport-completion` implemented; the native claim, empty-perimeter, and resolved-seam parity fixes are committed in HEAD (`9685cd03`, `f8400649`) and must be incorporated rather than reverted.
- Unblocks: 205d and 205e.
- Activation blockers: none. The former `[BLOCK]` (support-output origin preservation) is resolved: preservation requires an additive `set-current-origin` method on the WIT `support-output-builder` resource and a per-region `SupportIR` shape, both in scope here; no WIT package/version bump is needed (the dep package is unversioned).

## Acceptance Criteria

- **AC-1. Given** the native and WASM layer dispatch paths, **when** the marshalling implementation is inspected, **then** both paths consume one named authoritative view-construction module or type family, and no second field-by-field IR-to-view conversion remains in `crates/slicer-wasm-host/src/marshal/native.rs`. | `test -f crates/slicer-wasm-host/src/marshal/native.rs && rg -q 'SliceRegionView|PerimeterRegionView' crates/slicer-wasm-host/src/marshal/native.rs && rg -q 'SliceRegionView|PerimeterRegionView' crates/slicer-wasm-host/src/marshal/in_.rs && ! rg -q 'completeness mirror|Completeness mirror' crates/slicer-wasm-host/src/marshal/native.rs && echo PASS`
- **AC-2. Given** a configured region whose fill holder is not the native module, **when** native `Layer::Infill` dispatch runs, **then** the module receives no held fill claim for that role and emits no paths for that role, while the configured holder still emits its paths. | `sh -c 'cargo test -p slicer-runtime --test contract -- native_gyroid_holds_nothing_by_default >/tmp/205c-a.log 2>&1 && cargo test -p slicer-runtime --test contract -- native_rectilinear_holds_sparse_by_default >/tmp/205c-b.log 2>&1 && rg -q "^test result: ok" /tmp/205c-a.log && rg -q "^test result: ok" /tmp/205c-b.log'`
- **AC-3. Given** native postprocess inputs with no committed `PerimeterIR`, and a resolved seam emitted for an active region, **when** the native `Layer::PerimetersPostProcess` transport runs, **then** it returns `Ok(Some(LayerStageCommit::PerimetersPostProcess(None)))` for the empty case and preserves `resolved_seam_origin` for the active case. | `sh -c 'cargo test -p slicer-runtime --test contract --all-targets -- native_fuzzy_skin_without_committed_perimeter_does_not_fatal >/tmp/205c-c.log 2>&1 && cargo test -p slicer-runtime --test contract --all-targets -- native_seam_placer_aligned_commits_resolved_seam_with_origin >/tmp/205c-d.log 2>&1 && rg -q "^test result: ok" /tmp/205c-c.log && rg -q "^test result: ok" /tmp/205c-d.log'`
- **AC-4. Given** native outputs for every currently supported stage family, **when** their commits are applied, **then** prepass metadata (layer plan, seam plan with candidate reason, support plan, mesh analysis) and layer postprocess output are preserved without a silent `PrepassStageOutput::None`, a defaulted `region_id` of `0` on parse failure, or an unsupported-stage fatal for a stage whose output the native envelope declares (`PrePass::PaintSegmentation` and `Layer::SlicePostProcess`). `PrePass::PaintSegmentation` is intentionally outputless in this commit seam — the WASM leg has no output variant for it and paint output delivery is out of scope here — so its commit mirrors the WASM leg exactly (no fatal), as asserted by the targeted regression test. | `sh -c 'cargo test -p slicer-runtime --test contract --all-targets -- native_paint_segmentation_commit_mirrors_wasm_leg >/tmp/205c-4a.log 2>&1 && cargo test -p slicer-runtime --test contract --all-targets 2>&1 | rg -q "test result: ok|0 failed" && rg -q "^test result: ok" /tmp/205c-4a.log'`
 - **AC-5. Given** an integrated module binding, **when** the live loader completes, **then** its existing `native_entry: Option<NativeStageEntry>` invariant is validated as present before dispatch; an external binding has a WASM component and no native entry. | `sh -c 'cargo test -p slicer-runtime --test integration --all-targets -- integrated_binding_attaches_native_entry >/tmp/205c-5a.log 2>&1 && cargo test -p slicer-runtime --test integration --all-targets -- external_override_forces_wasm_dispatch >/tmp/205c-5b.log 2>&1 && rg -q "^test result: ok" /tmp/205c-5a.log && rg -q "^test result: ok" /tmp/205c-5b.log'`
- **AC-6. Given** a native support dispatch whose builder carries per-region origins, **when** the support output is committed, **then** the committed `SupportIR` carries per-region regions (`object_id`/`region_id`) and the native leg reads the builder's origin accessors rather than substituting empty origins. | `cargo test -p slicer-runtime --test contract --all-targets -- native_support_dispatch_preserves_per_region_origins 2>&1 | rg -q '^test result: ok'`

## Negative Test Cases

- **AC-N1. Given** an integrated manifest with no matching native entry, **when** live bindings are built, **then** loading fails with a module/stage diagnostic naming the missing native entry rather than creating a binding that later reports `MissingComponent`. | `cargo test -p slicer-runtime --test integration --all-targets -- integrated_without_native_entry_fails_loud 2>&1 | rg -q '^test result: ok'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract --all-targets`

## Authoritative Docs

- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` - direct read of the runner seam and native-entry amendment.
- `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` - direct read of the marshalling-boundary decision.
- `docs/adr/0056-integrated-modules-native-dispatch.md` - direct read of Decisions 1, 3, and 4.
- `docs/adr/0057-three-editions-and-integrated-tier.md` - direct read of integrated-tier and override rules.
- `CONTEXT.md` - delegated lookup for `Marshalling boundary`, `Integrated module`, `External module`, and `Per-region output origin`.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/07_implementation_status.md` - mark `TASK-329` complete when this packet closes; verify with `rg -q 'TASK-329' docs/07_implementation_status.md`.
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` - amend the native-entry section if the live-binding shape changes; verify with `rg -q 'native_entry' docs/adr/0005-runner-traits-in-slicer-wasm-host.md`.
- `docs/01_system_architecture.md` - update the existing integrated-module/native-dispatch architecture anchor if the named view authority changes; verify with `rg -q 'integrated' docs/01_system_architecture.md`.
- `docs/02_ir_schemas.md` - update the `SupportIR` section for the per-region shape; verify with `rg -q 'SupportIR' docs/02_ir_schemas.md`.
- `docs/03_wit_and_manifest.md` - update the `support-output-builder` resource section for the additive `set-current-origin` method; verify with `rg -q 'support-output-builder' docs/03_wit_and_manifest.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
