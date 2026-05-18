# Requirements: fill-role-claims

## Packet Metadata

- Grouped task IDs:
  - `TASK-167` (NEW)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Today every infill module is a single generator. Whichever infill module is enabled produces fill paths for *all* roles in a region — top, bottom, bridge, and sparse. There is no way to say "use `gyroid-infill` for sparse, but keep `rectilinear-infill` for top/bottom/bridge", which is a standard Orca configuration.

Two architectural options were evaluated during plan review:
- **Option α** — single multi-pattern monolithic module. Rejected: defers complexity rather than addressing it; `rectilinear-infill` becomes a misnomer.
- **Option β (this packet)** — multiple single-pattern modules each holding one or more `claim:*-fill` IDs. Aligns with the existing claim system in `docs/04 §Phase 3` and `docs/04 §Claim Resolution`. Adopted.

This packet introduces the four `claim:*-fill` IDs, updates the three existing infill modules to declare their default-holder relationships, adds the necessary global+region config selectors, and adds claim-conflict validation coverage.

## In Scope

- Register `claim:top-fill`, `claim:bottom-fill`, `claim:bridge-fill`, `claim:sparse-fill` in the host claim catalog (likely `crates/slicer-host/src/scheduler.rs` or `claims.rs`; locate via FACT dispatch in Step 0).
- Manifest schema extension in `docs/03_wit_and_manifest.md` to allow `[claims].holds` declarations naming any of the four claims (likely already supported; FACT confirms).
- Update manifests:
  - `modules/core-modules/rectilinear-infill/manifest.toml` — `[claims].holds = ["claim:top-fill", "claim:bottom-fill", "claim:bridge-fill", "claim:sparse-fill"]` (default: holds all four).
  - `modules/core-modules/gyroid-infill/manifest.toml` — `[claims].holds = ["claim:sparse-fill"]` (sparse only).
  - `modules/core-modules/lightning-infill/manifest.toml` — `[claims].holds = ["claim:sparse-fill"]` (sparse only).
- Runtime claim awareness in `rectilinear-infill`: when the dispatched claim set excludes a role, the module skips emitting paths for that role. The module learns its claim set from a new SDK accessor (`view.held_claims()` or the existing config-view holding the resolved claim set per call).
- Same runtime claim awareness pattern wired into `gyroid-infill` and `lightning-infill` (only the sparse role for them; trivial filter).
- Global config keys in the central config schema: `top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`, `sparse_fill_holder` — each a `String` naming the module ID. Defaults: all `"rectilinear-infill"`.
- Per-region overrides via `RegionMapIR.entries[*].config` (reusing packet 35's plumbing).
- Scheduler claim-resolution: when global config sets `sparse_fill_holder = "gyroid-infill"`, validation pass 2 disables `rectilinear-infill`'s `claim:sparse-fill` declaration (via the `config_disables_module` mechanism in `docs/04 §Claim Resolution`).
- New TDD `crates/slicer-host/tests/fill_role_claims_tdd.rs` covering the AC and negative cases.
- New Benchy E2E test `benchy_default_claims_emit_all_role_families` confirming the default config preserves all role families in G-code.

## Out of Scope

- New patterns (concentric, hilbert, etc.) — separate per-pattern packets.
- Bridge-detector parity (packet 36).
- Top-surface ironing (packet 38).
- WIT signature changes (claims live in scheduler validation, not WIT types).
- Changing the role → G-code marker mapping in `gcode_emit.rs`.
- Per-region pattern selection outside the four enumerated claims.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — § "Manifest Schema" (claims declarations). Read directly.
- `docs/04_host_scheduler.md` — § "Phase 3 — DAG Validation"; § "Claim Resolution"; § "Composable Multi-Writer Patterns". Document is large; delegate SUMMARY ≤ 200 words per section.
- `docs/02_ir_schemas.md` — `RegionMapIR.entries[*].config`. Read directly; one section.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` per-surface-role pattern selection. Delegate SUMMARY ≤ 200 words.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `Fill::new_from_type` factory. Delegate FACT.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases: see `packet.spec.md`. Covers (a) catalog registration, (b) default-holder behavior, (c) selective claim transfer to `gyroid-infill`, (d) other modules don't emit unheld claims, (e) per-region override, (f) Benchy default emits all 4 role families.
- Negative cases: (a) two holders for one claim fail validation, (b) zero holders for `top-fill` fail validation, (c) unknown claim ID in manifest fails Phase-1 ingestion.
- Measurable outcomes:
  - `cargo test --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
- Cross-packet impact: future "alternative pattern" packets plug in as new claim holders.

## Verification Commands

- `cargo test -p slicer-host --test fill_role_claims_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_default_claims_emit_all_role_families -- --nocapture`
- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L).

## Context Discipline Notes

- Large files in the read-only path:
  - `docs/04_host_scheduler.md` (> 600 lines) — delegate per-section SUMMARY.
  - `crates/slicer-host/src/scheduler.rs` or wherever validation pass 2 lives — read only the claim-conflict function and surrounding 60 lines.
- OrcaSlicer trees the implementer must NOT load directly: all of `OrcaSlicerDocumented/`.
- Likely temptation reads:
  - `crates/slicer-host/src/dispatch.rs` — out of scope unless a claim-aware dispatch path needs adjustment (verify via FACT first).
  - Other infill modules' source code — read manifests only unless behavior changes are required there.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail.
  - Doc summaries → SUMMARY ≤ 200 words.
  - Catalog/registry lookup → FACT with file:line.
