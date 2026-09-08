# Requirements: 205d-integrated-registry

## Packet Metadata

- Grouped task IDs: `TASK-330`
- Backlog source: `docs/specs/integrated-modules-architecture-205c-205e-plan.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The integrated registry repeats the same 21 module names across feature-gated manifest constants, `integrated_registrations()`, `native_entries()`, feature coverage tests, `slicer-integrated-modules/Cargo.toml`, and pnp-cli passthrough features. The current interface is shallow: adding a module requires synchronized edits across several lists and cfg walls. This packet deepens the registry representation without changing the externally observable module set.

## In Scope

- A single registry authority for enabled module metadata and native entry construction.
- Generated or table-driven registration and entry vectors.
- Coverage tests derived from the same authority, including manifest ID and stage-family checks.
- Static or compile-time checks that preserve the 21 feature and passthrough mapping.

## Out of Scope

- Adding or removing modules.
- Changing Cargo feature names, manifest IDs, origin labels, edition lists, or search priority.
- Changing `pnp-cli` feature definitions or xtask edition planning.
- Changing native dispatch or parity test execution.

## Acceptance Summary

- Positive: `AC-1` through `AC-4`.
- Negative: `AC-N1` and `AC-N2`.
- Cross-packet impact: 205e may read the registry inventory but must not duplicate its authority.

## Verification Commands

| Command | Purpose | Return format hint |
|---|---|---|
| `cargo test -p slicer-integrated-modules --all-targets --features <full-set>` | Registry and stage-family coverage | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration --all-targets -- full_coverage_external_override_forces_wasm` | External override preservation | FACT pass/fail |
| `cargo check --workspace --all-targets` | Feature and generated-surface blast radius | FACT pass/fail |

## Step Completion Expectations

The single authority must remain compatible with Cargo feature gating. Disabled features must produce no registrations or native entries, and enabled features must produce both. Tests must derive expected sets from the registry metadata rather than reintroducing a second literal module list.

## Context Discipline Notes

The registry source is under 700 lines but repetitive; read only manifest constants, registration/entry functions, and coverage tests. Use metadata summaries for Cargo manifests.
