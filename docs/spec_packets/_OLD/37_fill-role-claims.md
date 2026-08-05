---
status: implemented
packet: fill-role-claims
task_ids:
  - TASK-167
---

# 37_fill-role-claims

## Goal

Introduce four scheduler claims — `claim:top-fill`, `claim:bottom-fill`, `claim:bridge-fill`, `claim:sparse-fill` — that let users mix infill modules per surface role at `Layer::Infill`. Each existing infill module declares which claims it holds; users select the holder per claim through global config (with per-region overrides via `RegionMapIR`). Validation pass 2 (claim-conflict detection in `docs/04 §Phase 3`) catches double-holders at startup.

This is the "Option β" architecture chosen in plan review: multiple single-pattern modules coexisting with one claim each per role, rather than a single multi-pattern monolithic generator.

## Problem Statement

Today every infill module is a single generator. Whichever infill module is enabled produces fill paths for *all* roles in a region — top, bottom, bridge, and sparse. There is no way to say "use `gyroid-infill` for sparse, but keep `rectilinear-infill` for top/bottom/bridge", which is a standard Orca configuration.

Two architectural options were evaluated during plan review:
- **Option α** — single multi-pattern monolithic module. Rejected: defers complexity rather than addressing it; `rectilinear-infill` becomes a misnomer.
- **Option β (this packet)** — multiple single-pattern modules each holding one or more `claim:*-fill` IDs. Aligns with the existing claim system in `docs/04 §Phase 3` and `docs/04 §Claim Resolution`. Adopted.

This packet introduces the four `claim:*-fill` IDs, updates the three existing infill modules to declare their default-holder relationships, adds the necessary global+region config selectors, and adds claim-conflict validation coverage.

## Architecture Constraints

- **Reuse the existing claim system.** `docs/04 §Phase 3` already implements claim-conflict validation. This packet adds new claim IDs, not new infrastructure.
- **No WIT signature changes.** Claims live in scheduler validation; the WIT/IR boundary is unchanged.
- **Per-region overrides via `RegionMapIR`.** Use the plumbing already established in packet 35.
- **Module runtime logic learns its claim set from the SDK.** Either via a new accessor (`view.held_claims()`) or via the existing config-view (resolved claims as config keys). Step 0 FACT picks the cleaner path.

## Data and Contract Notes

- IR or manifest contracts touched:
  - Claim catalog gains 4 entries.
  - Manifest schema admits the four new claim IDs in `[claims].holds`.
  - Config schema gains 4 new keys.
  - No IR struct changes; no schema-version bumps.
- WIT boundary considerations: none.
- Determinism or scheduler constraints:
  - Validation pass 2 must catch double-holders deterministically.
  - Per-region overrides remain deterministic via `RegionMapIR`'s `(layer, object, region)` keying.
  - Per `docs/04 §Claim Resolution`: claim holder consistency is required per `(object_id, claim)` across all global layers; if region overrides produce holder transitions across layers for the same object, validation fails as non-deterministic. The new claim IDs inherit this rule automatically.

## Locked Assumptions and Invariants

- The existing claim system supports adding new claim IDs without scheduler-source changes (FACT in Step 0).
- `RegionMapIR.entries[*].config` already supports per-region module enable/disable through `config_disables_module`.
- Default holder for all four claims is `rectilinear-infill`, matching today's behavior — Benchy E2E preserves all role families with no config changes.

## Risks and Tradeoffs

- **Existing infill modules need claim-aware filtering.** If a module emits paths it doesn't claim, two holders for one claim might emit duplicate paths (deterministic but quality-bad). The runtime filter is a safety net beyond validation pass 2.
- **`gyroid-infill` and `lightning-infill` may currently emit only sparse paths anyway.** Verify in Step 1 (read manifests + read source). If they don't emit top/bottom/bridge today, the runtime filter is a no-op for them — still worth including for consistency.
- **Discoverability.** Users may not know which claims are available. Mitigation: document in `docs/03_wit_and_manifest.md` claim catalog section.
