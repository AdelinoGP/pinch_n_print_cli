# Implementation Plan: 205d-integrated-registry

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-330`.
- Preserve all feature-gated module identities while deleting duplicated output lists.

## Steps

### Step 1: Inventory registry consumers and identity invariants

- Task IDs: `TASK-330`
- Objective: prove vector-order requirements and enumerate every current registry/test list.
- Precondition: 205c is implemented.
- Postcondition: the table shape preserves every enabled feature, ID, origin label, and stage family.
- Files allowed to read: `slicer-integrated-modules/src/lib.rs:1-678`; Cargo metadata summary; consumer locations.
- Files allowed to edit: none.
- Files explicitly out of bounds: pnp-cli features, editions, dispatch.
- Expected sub-agent dispatches: consumer/order `LOCATIONS`; feature mapping `FACT`.
- Context cost: `S`.
- Authoritative docs: ADR-0056 and ADR-0057.
- Verification: `rg -n 'integrated_registrations|native_entries' crates --glob '*.rs'`.
- Exit condition: no consumer or identity invariant is unaccounted for.

### Step 2: Introduce one registry authority

- Task IDs: `TASK-330`
- Objective: replace repeated registration/entry lists with one feature-gated registry authority.
- Precondition: Step 1 inventory is complete.
- Postcondition: both public vectors derive from the same rows and disabled features remain absent.
- Files allowed to read: `lib.rs:1-400`; `Cargo.toml:30-56`.
- Files allowed to edit: `crates/slicer-integrated-modules/src/lib.rs`; optionally `crates/slicer-integrated-modules/Cargo.toml` only for representation support.
- Files explicitly out of bounds: pnp-cli Cargo, xtask, module crates.
- Expected sub-agent dispatches: targeted cargo test; return `FACT`.
- Context cost: `M`.
- Authoritative docs: ADR-0056/0057.
- Verification: full-feature registry tests and default-feature empty test.
- Exit condition: AC-1 and AC-2 pass with no second per-module push list.

### Step 3: Move coverage assertions onto the authority

- Task IDs: `TASK-330`
- Objective: make coverage and external override checks consume generated registry metadata without weakening negative cases.
- Precondition: Step 2 vectors compile.
- Postcondition: registration/entry mismatch and external override failures remain detectable.
- Files allowed to read: `lib.rs:402-678`; external override test.
- Files allowed to edit: `crates/slicer-integrated-modules/src/lib.rs`; `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` only if needed.
- Files explicitly out of bounds: dispatch implementation and edition config.
- Expected sub-agent dispatches: focused test run; return `FACT` or <=20 failure lines.
- Context cost: `S`.
- Authoritative docs: ADR-0056.
- Verification: registry coverage and external override tests.
- Exit condition: AC-3, AC-4, AC-N1, and AC-N2 pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| 1 | S | inventory |
| 2 | M | registry representation |
| 3 | S | coverage consumers |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch.
