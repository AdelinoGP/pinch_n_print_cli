# Requirements: 205c-native-dispatch-seam

## Packet Metadata

- Grouped task IDs: `TASK-329`
- Backlog source: `docs/specs/integrated-modules-architecture-205c-205e-plan.md`
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

The integrated-modules effort added a second native IR-to-view translation beside the WASM translation, duplicated held-claim resolution, and left native response commits lossy in several families: prepass stages silently return `PrepassStageOutput::None` where the WASM leg materializes output, seam-plan candidate reasons are hardcoded, `region_id` defaults to `0` on parse failure, `PaintSegmentation` and `SlicePostProcess` commits fatal despite declared envelope outputs, and support origins are hardcoded empty (the WIT `support-output-builder` lacks `set-current-origin`, the SDK builder lacks origin tracking, and `SupportIR` is flat). The live binding also represents two mutually exclusive dispatch modes with optional fields and a placeholder pool, allowing missing native entries to fail late. This packet restores locality at the native dispatch seam without changing module semantics.

## In Scope

- One authoritative view-construction path shared by native and WASM adapters: `SliceRegionView::from_ir` and `PerimeterRegionView::from_ir` in `crates/slicer-sdk/src/views.rs`; the WASM leg adapts the plain views to WIT resource types, the native leg consumes the constructors directly.
- Removal of the duplicate `resolve_layer_held_claims_map` logic in favor of the scheduler-owned `resolve_held_claims` authority (`crates/slicer-scheduler/src/validation.rs:90`), with per-region held claims preserved.
- Lossless native response commits for supported prepass, layer, support-origin, and postprocess fields; no silent fallback for supported variants.
- The support-origin contract: additive `set-current-origin` on the WIT `support-output-builder` resource, SDK `SupportOutputBuilder` origin tracking, host `set_current_origin` implementation, per-region `SupportIR` shape (mirroring `InfillIR`/`PerimeterIR`), and origin-preserving marshal conversion for both legs.
- Explicit integrated/external dispatch mode and load-time rejection of an integrated module without a native entry.
- Regression coverage for empty perimeter, resolved seam origin, held claims, support origins, prepass metadata, external override, and missing native entry.

## Out of Scope

- Module algorithm changes or new integrated modules.
- Edition membership, search priority, external override semantics, or CLI flags.
- WIT package/version changes (the additive method is not a version bump; the dep package is unversioned).
- `SupportIR` consumers outside slicer-runtime's executor/debug-render and the focused test surfaces.

## Authoritative Docs

- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` - direct read of the runner seam and native-entry amendment.
- `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` - direct read of the marshalling-boundary decision.
- `docs/adr/0056-integrated-modules-native-dispatch.md` - direct read of Decisions 1, 3, and 4.
- `docs/adr/0057-three-editions-and-integrated-tier.md` - direct read of integrated-tier and override rules.
- `CONTEXT.md` - delegated lookup for `Marshalling boundary`, `Integrated module`, `External module`, and `Per-region output origin`.

## Acceptance Summary

- Positive: `AC-1` through `AC-6` in `packet.spec.md`.
- Negative: `AC-N1`.
- Cross-packet impact: 205d consumes the stable registry seam; 205e consumes the stable native/WASM parity setup.

## Verification Commands

| Command | Purpose | Return format hint |
|---|---|---|
| `cargo test -p slicer-runtime --test contract --all-targets` | Native/WASM dispatch regressions, support-origin preservation, and parity | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration --all-targets` | Integrated/external live-binding contract | FACT pass/fail |
| `cargo test -p slicer-wasm-host --all-targets` | Host-side support-origin builder contract | FACT pass/fail |
| `cargo check --workspace --all-targets` | Struct and dispatch blast radius | FACT pass/fail; <=20 failure lines |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail; <=20 failure lines |

## Step Completion Expectations

The authoritative view type must be selected before changing callers. Native and WASM adapters must remain thin; no module crate may gain knowledge of the transport choice. The `SupportIR` shape change lands with its consumers (gcode emission, debug render, parity comparator, fixtures) in the same step. Any deferred field must be represented as an explicit `[BLOCK]` or out-of-scope contract, never silently dropped.

## Context Discipline Notes

The dispatch and native marshal files are large; read only the bounded functions named in `design.md`. Delegate cargo checks and cross-crate trait tracing. The WIT/SDK/macros change feeds guest WASM — the implementer must run `cargo xtask build-guests --check` after editing that surface (see `design.md` Architecture Constraints).
