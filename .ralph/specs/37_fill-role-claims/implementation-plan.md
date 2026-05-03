# Implementation Plan: fill-role-claims

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-167.
- TDD first, then minimal infrastructure (catalog + SDK), then manifests, then module-side filter, then acceptance.
- Each step honors the context-discipline preamble.

## Steps

### Step 0: FACT-confirm catalog location, SDK accessor availability, and existing module emission patterns

- Task IDs:
  - `TASK-167`
- Objective: read-only discovery — locate the host claim catalog file, confirm whether the SDK already exposes a per-call held-claim set, and FACT what `gyroid-infill` and `lightning-infill` currently emit (top/bottom/bridge or sparse-only).
- Precondition: Step 0 not yet run.
- Postcondition: three FACTs recorded.
- Files allowed to read: none directly (delegate only).
- Files allowed to edit (≤ 3): none.
- Expected sub-agent dispatches:
  - "Find the host claim catalog (search `crates/slicer-host/src/` for `ClaimId`, `KNOWN_CLAIMS`, or claim-registry constants). Return FACT with file:line and the addition pattern."
  - "Does `SliceRegionView` or any sibling SDK type expose a held-claims accessor or equivalent? Return FACT yes/no with file:line."
  - "Do `gyroid-infill` and `lightning-infill` currently emit only sparse paths, or do they also emit top/bottom/bridge? Return FACT for each module."
  - "Summarize `docs/04_host_scheduler.md` § 'Claim Resolution with Runtime Disable Rules' in ≤ 200 words. Return SUMMARY."
- Context cost: `S`.
- Authoritative docs: `docs/04_host_scheduler.md`.
- OrcaSlicer refs: none.
- Verification: the four FACTs/SUMMARY.
- Exit condition: Steps 1–3 plans firmed up based on outcomes.

### Step 1: Author failing TDD file

- Task IDs:
  - `TASK-167`
- Objective: create `crates/slicer-host/tests/fill_role_claims_tdd.rs` covering all AC + negative cases. Append `benchy_default_claims_emit_all_role_families` to `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`. Tests fail until Steps 2–4 land.
- Precondition: Step 0 complete.
- Postcondition: every new test compiles and FAILS.
- Files allowed to read:
  - `crates/slicer-host/tests/external_surface_classification_tdd.rs` — pattern reference.
  - `crates/slicer-host/src/scheduler.rs` (or wherever validation pass 2 lives) — only the validation-error enum (range-read ≤ 60 lines).
  - `crates/slicer-host/tests/dispatch_tdd.rs` — fixture/test patterns for scheduler validation tests.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/fill_role_claims_tdd.rs` (new).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (append).
- Files explicitly out-of-bounds for this step: production code.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test fill_role_claims_tdd`; return FACT (every test FAIL)."
- Context cost: `M`.
- Authoritative docs: `docs/04_host_scheduler.md` § Phase 3 (delegate SUMMARY).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — delegate FACT confirming the pattern-by-role mapping is the parity reference.
- Verification: tests compile + every new test FAILS.
- Exit condition: TDD scaffolding present.

### Step 2: Register claim IDs and (if needed) add SDK accessor

- Task IDs:
  - `TASK-167`
- Objective: register the four claim IDs in the host claim catalog. If Step 0 FACT showed no SDK accessor for the held-claim set, add `SliceRegionView::held_claims() -> &[ClaimId]` (or equivalent) and wire it from the dispatch-side resolved claims.
- Precondition: Step 1 complete.
- Postcondition: workspace builds; `four_fill_claims_registered_in_catalog` test PASSES.
- Files allowed to read:
  - the claim catalog file (Step 0 FACT).
  - `crates/slicer-sdk/src/views.rs` — full file (small).
  - `crates/slicer-host/src/wit_host.rs` — only the `slice-region-data` definition area (lines `135-180`) and dispatch-side claim resolution (delegate FACT for the right line range).
- Files allowed to edit (≤ 3):
  - the claim catalog file.
  - `crates/slicer-sdk/src/views.rs` (only if SDK accessor needed).
  - `crates/slicer-host/src/wit_host.rs` (only if held-claims need surfacing through WIT/host record).
- Files explicitly out-of-bounds for this step: scheduler.rs source (read-only via FACT), `dispatch.rs` (delegate any check).
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail."
  - "Run `cargo test -p slicer-host --test fill_role_claims_tdd four_fill_claims_registered_in_catalog`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/04_host_scheduler.md` § Phase 3.
- OrcaSlicer refs: none.
- Verification: targeted cargo test.
- Exit condition: catalog test PASSES; build green; remaining tests still FAIL.

### Step 3: Update three module manifests + add config schema keys

- Task IDs:
  - `TASK-167`
- Objective: update manifests for `rectilinear-infill`, `gyroid-infill`, `lightning-infill` with `[claims].holds` arrays. Add the four config keys (`top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`, `sparse_fill_holder`) to the central config schema with `"rectilinear-infill"` defaults. Confirm validation pass 2 catches the artificial double-holder negative case.
- Precondition: Step 2 complete.
- Postcondition: `two_holders_for_one_claim_fails_validation`, `missing_holder_for_top_fill_claim_fails_validation`, and `unknown_claim_in_manifest_is_load_error` PASS.
- Files allowed to read:
  - the three module manifests (small files).
  - `docs/03_wit_and_manifest.md` § Manifest Schema (claims).
  - the central config schema file (locate via FACT).
- Files allowed to edit (≤ 3 per pass; multiple passes):
  - `modules/core-modules/rectilinear-infill/manifest.toml`
  - `modules/core-modules/gyroid-infill/manifest.toml`
  - `modules/core-modules/lightning-infill/manifest.toml`
  - (separate pass) the central config schema file.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test fill_role_claims_tdd two_holders_for_one_claim_fails_validation missing_holder_for_top_fill_claim_fails_validation unknown_claim_in_manifest_is_load_error -- --exact`; return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- OrcaSlicer refs: none.
- Verification: targeted cargo test.
- Exit condition: 3 negative-case tests PASS; rest still fail.

### Step 4: Implement runtime claim-aware filter in three module sources

- Task IDs:
  - `TASK-167`
- Objective: in each of the three module `src/lib.rs` files, add an early-return that skips emitting paths for unheld claims. Use the SDK accessor from Step 2.
- Precondition: Step 3 complete.
- Postcondition: positive AC tests PASS — `default_rectilinear_holds_all_claims_emits_top`, `gyroid_holds_sparse_claim_only_emits_sparse`, `gyroid_does_not_emit_for_unheld_top_claim`, `region_override_redirects_claim_to_alternate_holder`.
- Files allowed to read:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — full.
  - `modules/core-modules/gyroid-infill/src/lib.rs` — full.
  - `modules/core-modules/lightning-infill/src/lib.rs` — full.
  - `crates/slicer-sdk/src/views.rs` — held-claims accessor surface only.
- Files allowed to edit (≤ 3):
  - the three module `src/lib.rs` files (one step; one consistent pattern).
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail with failing module name on fail."
  - "Run `cargo test -p slicer-host --test fill_role_claims_tdd default_rectilinear_holds_all_claims_emits_top gyroid_holds_sparse_claim_only_emits_sparse gyroid_does_not_emit_for_unheld_top_claim region_override_redirects_claim_to_alternate_holder -- --exact`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md` (delegate FACT for SDK call patterns).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — delegate FACT confirming role-keyed dispatch is the parity reference.
- Verification: rebuild + targeted cargo test.
- Exit condition: rebuild succeeds; positive AC tests PASS.

### Step 5: Acceptance — Benchy E2E + workspace gates + doc updates

- Task IDs:
  - `TASK-167`
- Objective: confirm `benchy_default_claims_emit_all_role_families` PASSES; full workspace test + clippy PASS; update `docs/03_wit_and_manifest.md` with the four claim IDs; update `docs/07_implementation_status.md` with TASK-167.
- Precondition: Step 4 complete.
- Postcondition: every AC PASSES; docs updated.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 3):
  - `docs/03_wit_and_manifest.md`
  - `docs/07_implementation_status.md` (delegate row insertion)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_default_claims_emit_all_role_families`; return FACT pass/fail."
  - "Run `cargo test --workspace`; return FACT pass/fail with failing test list (max 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail."
  - "Insert TASK-167 row into `docs/07_implementation_status.md`; return FACT confirming the new line:line."
- Context cost: `S`.
- Authoritative docs: `docs/03_wit_and_manifest.md`, `docs/07_implementation_status.md`.
- OrcaSlicer refs: none.
- Verification: every AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; docs carry the four claim IDs and TASK-167.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Four FACT/SUMMARY dispatches. |
| Step 1 | M | TDD scaffolding. |
| Step 2 | M | Catalog + (conditionally) SDK accessor. |
| Step 3 | S | Three manifest edits + config schema keys. |
| Step 4 | M | Three module source edits + rebuild. |
| Step 5 | S | Acceptance + doc updates. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every AC verification command PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `docs/03_wit_and_manifest.md` documents the four claim IDs.
- `docs/07_implementation_status.md` carries TASK-167.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command.
- Confirm `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` PASS.
- Record any remaining packet-local risk (especially: discoverability of the four claims for users).
- Confirm implementer's peak context usage stayed under 70%.
