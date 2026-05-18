# Design: fill-role-claims

## Controlling Code Paths

- Primary code path:
  - Host claim catalog (location TBD; FACT in Step 0) — register the four claim IDs.
  - Scheduler validation pass 2 (`docs/04 §Phase 3`, code in `crates/slicer-host/src/scheduler.rs` or similar) — recognize the four claims as conflict-eligible (likely already generic; FACT confirms).
  - `crates/slicer-host/src/region_mapping.rs` — per-region claim resolution (already exists; reuse).
  - Manifest ingestion (`crates/slicer-host/src/manifest.rs` or similar) — accept `[claims].holds` referencing the new IDs.
  - Module manifests (`modules/core-modules/{rectilinear,gyroid,lightning}-infill/manifest.toml`) — declare claim holders.
  - Module runtime logic (`modules/core-modules/{rectilinear,gyroid,lightning}-infill/src/lib.rs`) — filter emission by held claims.
  - SDK (`crates/slicer-sdk/src/views.rs` or `config.rs`) — surface the resolved-claim set to guest modules so they can self-filter at runtime.
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/fill_role_claims_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append.
  - 12-rev1 / 35 / 36 tests must remain green.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` (`Layer::make_fills`) and `Fill/FillBase.cpp` (`new_from_type`).

## Architecture Constraints

- **Reuse the existing claim system.** `docs/04 §Phase 3` already implements claim-conflict validation. This packet adds new claim IDs, not new infrastructure.
- **No WIT signature changes.** Claims live in scheduler validation; the WIT/IR boundary is unchanged.
- **Per-region overrides via `RegionMapIR`.** Use the plumbing already established in packet 35.
- **Module runtime logic learns its claim set from the SDK.** Either via a new accessor (`view.held_claims()`) or via the existing config-view (resolved claims as config keys). Step 0 FACT picks the cleaner path.

## Code Change Surface

- Selected approach (Option β):
  - Register the four new claim IDs in the host claim catalog.
  - Update three module manifests to declare default holders (rectilinear holds all four; gyroid + lightning hold sparse only).
  - Add four config keys (`top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`, `sparse_fill_holder`) with `"rectilinear-infill"` defaults.
  - When the user sets a non-default holder for a claim, the existing `config_disables_module` mechanism (`docs/04 §Claim Resolution`) disables the default holder's claim for that scope.
  - Module runtime: each infill module reads its resolved-claim set from the SDK and skips emitting paths for unheld claims. For `rectilinear-infill`, the existing role-selection chain (`is_bridge`/`is_top`/`is_bottom`/sparse) gains an early-return `if !self.holds_claim(target_role) { continue }`.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - host claim catalog (location identified in Step 0) — add 4 entries.
  - `modules/core-modules/rectilinear-infill/manifest.toml` — add `[claims].holds` array.
  - `modules/core-modules/gyroid-infill/manifest.toml` — add `[claims].holds` array.
  - `modules/core-modules/lightning-infill/manifest.toml` — add `[claims].holds` array.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — add claim-aware filter in the role chain.
  - `modules/core-modules/gyroid-infill/src/lib.rs` — add claim-aware filter (reject anything that isn't sparse).
  - `modules/core-modules/lightning-infill/src/lib.rs` — same as gyroid.
  - `crates/slicer-sdk/src/views.rs` (or `config.rs`) — add `held_claims` accessor (Step 0 FACT picks the cleaner path).
  - `crates/slicer-host/tests/fill_role_claims_tdd.rs` (NEW).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append `benchy_default_claims_emit_all_role_families`.
  - Central config schema — add 4 new keys with defaults.
- Rejected alternatives that were considered and why they were not chosen:
  - **Option α (single multi-pattern monolith)** — rejected in plan review.
  - **Implicit claims via stage routing** — rejected: less inspectable; the existing claim-conflict validator wouldn't catch double-emission.
  - **Module-side filter via `config_view.get_string("active_claim")`** — rejected unless Step 0 FACT shows no cleaner SDK accessor is feasible; `held_claims()` on `SliceRegionView` is more honest.

## Files in Scope (read + edit)

Primary edit targets (≤ 3 per step; aggregate ≤ 5):

- Step "Catalog + validation": host claim catalog file + `crates/slicer-sdk/src/views.rs` or similar.
- Step "Manifests": three module manifests (one step; they're tiny).
- Step "Runtime filter": three module sources (one step; identical pattern).
- Step "Tests": new TDD file + Benchy E2E append.

## Read-Only Context

- `docs/04_host_scheduler.md` — § Phase 3, § Claim Resolution, § Composable Multi-Writer Patterns. Delegate SUMMARY each.
- `docs/03_wit_and_manifest.md` — § Manifest Schema (claims).
- `crates/slicer-host/src/scheduler.rs` (or equivalent) — read only the claim-conflict function (range-read).
- `crates/slicer-host/src/region_mapping.rs` — public API only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate only.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` — out of scope.
- `wit/` — no WIT changes.
- Other crates outside the listed change surface — delegate fact-checks if needed.

## Expected Sub-Agent Dispatches

- "Where is the host claim catalog defined? Look for `ClaimId`, `KNOWN_CLAIMS`, or claim-registry constants in `crates/slicer-host/src/`. Return FACT with file:line." — purpose: validate Step 0.
- "Does `SliceRegionView` (or any sibling SDK type) already expose `held_claims` or equivalent? Return FACT yes/no with file:line." — purpose: validate Step 0.
- "Summarize `docs/04_host_scheduler.md` § 'Claim Resolution with Runtime Disable Rules' in ≤ 200 words. Return SUMMARY." — purpose: confirm config-disables-module path.
- "Run `cargo test -p slicer-host --test fill_role_claims_tdd`; return FACT pass/fail per test." — validate Step 3.
- "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail." — validate Step 2.
- "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_default_claims_emit_all_role_families`; return FACT pass/fail." — validate Step 4.

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

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 1: catalog registration + SDK accessor).
- Highest-risk dispatch: WASM rebuild after manifest + module changes — FACT-only return.

## Open Questions

- Step 0 dispatch resolves: does `SliceRegionView` already expose claim-set info? If yes, runtime filtering is a one-line check; if no, this packet adds the SDK accessor.
- Step 0 dispatch resolves: which file is the claim catalog? If a single registry file exists, edits are localized; if claims are decentralized (each module's manifest is the only source), the validation may need a small new collection helper — still S cost.
