# Requirements: 205c-native-dispatch-seam

## Packet Metadata

- Grouped task IDs: `TASK-329`
- Backlog source: `docs/specs/integrated-modules-architecture-205c-205e-plan.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The integrated-modules effort added a second native IR-to-view translation beside the WASM translation, duplicated held-claim resolution, and left native response commits lossy in several families. The live binding also represents two mutually exclusive dispatch modes with optional fields and a placeholder pool, allowing missing native entries to fail late. This packet restores locality at the native dispatch seam without changing module semantics.

## In Scope

- One authoritative view-construction path shared by native and WASM adapters.
- Removal of the duplicate `resolve_layer_held_claims_map` logic in favor of one scheduler-owned authority, with per-region held claims preserved.
- Lossless native response commits for supported prepass, layer, support-origin, and postprocess fields; no silent fallback for supported variants.
- Explicit integrated/external dispatch mode and load-time rejection of an integrated module without a native entry.
- Regression coverage for empty perimeter, resolved seam origin, held claims, external override, and missing native entry.

## Out of Scope

- Module algorithm changes or new integrated modules.
- Edition membership, search priority, external override semantics, or CLI flags.
- WIT package/version changes.
- The deferred support-stage origin contract if it requires a new WIT or IR field beyond the current native envelope.

## Acceptance Summary

- Positive: `AC-1` through `AC-5` in `packet.spec.md`.
- Negative: `AC-N1`.
- Cross-packet impact: 205d consumes the stable registry seam; 205e consumes the stable native/WASM parity setup.

## Verification Commands

| Command | Purpose | Return format hint |
|---|---|---|
| `cargo test -p slicer-runtime --test contract --all-targets` | Native/WASM dispatch regressions and parity | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integrated_tier_tdd --all-targets` | Integrated/external live-binding contract | FACT pass/fail |
| `cargo check --workspace --all-targets` | Struct and dispatch blast radius | FACT pass/fail; <=20 failure lines |

## Step Completion Expectations

The authoritative view type must be selected before changing callers. Native and WASM adapters must remain thin; no module crate may gain knowledge of the transport choice. Any deferred field must be represented as an explicit `[BLOCK]` or out-of-scope contract, never silently dropped.

## Context Discipline Notes

The dispatch and native marshal files are large; read only the bounded functions named in `design.md`. Delegate cargo checks and cross-crate trait tracing.
