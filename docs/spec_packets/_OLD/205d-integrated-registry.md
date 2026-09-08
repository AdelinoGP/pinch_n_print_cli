---
status: implemented
packet: 205d-integrated-registry
task_ids:
  - TASK-330
---

# 205d-integrated-registry

## Goal

Derive integrated manifest registrations, native entries, and coverage checks from one registry authority while preserving all 21 feature names, module IDs, origin labels, stage families, and edition behaviour.

## Problem Statement

The integrated registry repeats the same 21 module names across feature-gated manifest constants, `integrated_registrations()`, `native_entries()`, feature coverage tests, `slicer-integrated-modules/Cargo.toml`, and pnp-cli passthrough features. The current interface is shallow: adding a module requires synchronized edits across several lists and cfg walls. This packet deepens the registry representation without changing the externally observable module set.

## Architecture Constraints

- Preserve deterministic registration order unless the existing callers explicitly treat order as irrelevant; if order changes, add the exact ordering invariant to the packet tests.
- Preserve `com.core.<name>` IDs and `integrated://<name>` origin labels.
- Preserve the empty default feature set and all 21 Cargo feature names.

## Data and Contract Notes

- Registry rows carry manifest text/origin and native entry identity; both must agree on module ID.
- Cargo feature gating remains the compile-time selector.
- External modules still win by existing search priority.

## Locked Assumptions and Invariants

- Exactly 21 core module feature names remain available.
- Default features remain empty.
- Registry vectors contain one entry per enabled feature and no duplicate module IDs.

## Risks and Tradeoffs

- Macro-generated function pointers may complicate feature-gated imports; prefer a small local declarative macro over a proc-macro or build script.
- A table storing function pointers may require explicit cfg blocks for imports; those cfg blocks are acceptable if the module row itself is not repeated across outputs.
