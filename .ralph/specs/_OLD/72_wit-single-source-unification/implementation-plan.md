# Implementation Plan: 72_wit-single-source-unification

## Execution Rules

- One atomic step at a time; each maps back to `TASK-144`/`TASK-145`.
- TDD-leaning: Step 5 conformance test is authored to fail first, then satisfied.
- After any step that edits a guest-WASM input, rebuild guests before trusting a test (see `design.md` §Architecture Constraints).
- The field set below is the budget contract — honor files-to-read/edit and dispatch hints verbatim.

## Steps

### Step 0: Spike the codegen mechanism

- Task IDs: `TASK-144`
- Objective: pick guest mechanism (A nested-package inline vs B flatten) and confirm host `bindgen! path:` viability before authoring.
- Precondition: none.
- Postcondition: a throwaway proof that (a) `wit_bindgen 0.57` `generate!{ inline }` accepts nested-package form with cross-package `use`, and (b) `wasmtime 43` `bindgen!{ path }` resolves a multi-package `deps/` dir with remapped `with:` keys — or a recorded decision to use the documented fallback.
- Files allowed to read: `crates/slicer-macros/src/lib.rs` (≈478–558 only); `crates/slicer-runtime/src/wit_host.rs` (one `bindgen!` site, ≈1066–1170).
- Files allowed to edit (≤3): a scratch file only (discard before Step 1) — no packet file edited.
- Files explicitly out-of-bounds: both consumer files in full; `target/`.
- Expected sub-agent dispatches:
  - `Run the scratch build for the spike world; scope crates/slicer-runtime; return FACT pass/fail + first error ≤20 lines.`
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: scratch `cargo build -p slicer-runtime` for the spiked world — dispatch as FACT.
- Exit condition: A-or-B chosen and `path:`-or-build.rs chosen, recorded in `design.md` §Open Questions resolution; scratch reverted.

### Step 1: Author the canonical WIT contract

- Task IDs: `TASK-144`, `TASK-145`
- Objective: create `crates/slicer-schema/wit/` (4 worlds + `deps/{types,config,ir-types,common}.wit`) reconciled to the two compiled copies, legal `extrusion-path3d`, shared `module-error` in `common.wit`, no `gcode-output-interface`, no `extrusion-mode`.
- Precondition: Step 0 complete.
- Postcondition: AC-1, AC-4, AC-5, AC-6 pass; the dir is valid WIT (proven later by AC-9).
- Files allowed to read: current `wit/deps/*.wit` + `wit/world-*.wit` (reconciliation inputs, small); `wit_host.rs` postpass/prepass `bindgen!` sites (≈1066, 493) and `slicer-macros` world literals (≈567, 1283) to confirm the compiled shape where docs differ.
- Files allowed to edit (≤3): `crates/slicer-schema/wit/` (the new tree — counted as one surface).
- Files explicitly out-of-bounds: `wit_host.rs`/`slicer-macros` in full.
- Expected sub-agent dispatches:
  - `Summarize docs/03_wit_and_manifest.md world/interface inventory + wit-world allowlist; return SUMMARY ≤200 words.`
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` (delegate SUMMARY); `docs/01_system_architecture.md` (delegate if ambiguous).
- OrcaSlicer refs: none.
- Verification: AC-1/AC-4/AC-5/AC-6 grep commands — dispatch as FACT `EXIT=0`.
- Exit condition: all four greps return `EXIT=0`.

### Step 2: Repoint the guest macro at the canonical source

- Task IDs: `TASK-144`
- Objective: source the four worlds + deps from `crates/slicer-schema/wit/`; delete flatten/rename machinery (option A) or repoint flatten inputs (option B).
- Precondition: Step 1 complete.
- Postcondition: AC-2 passes; guests rebuild; AC-7 + IR-access + benchy green.
- Files allowed to read: `crates/slicer-macros/src/lib.rs` (≈478–558 + four world literals only).
- Files allowed to edit (≤3): `crates/slicer-macros/src/lib.rs`.
- Files explicitly out-of-bounds: `wit_host.rs`; macro file in full.
- Expected sub-agent dispatches:
  - `Run cargo xtask build-guests then --check; return FACT clean or STALE: list.`
  - `Run cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd; FACT pass/fail + assertion ≤20 lines.`
- Context cost: `M`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-2 grep (`EXIT=0`); rebuild guests; AC-7 (`macro_all_worlds_roundtrip_tdd`), `core_module_ir_access_contract_tdd`, `benchy_end_to_end_tdd` — all FACT pass.
- Exit condition: AC-2 `EXIT=0` and the three regression tests pass against freshly built guests.

### Step 3: Delete top-level `wit/` and repoint the staleness walk

- Task IDs: `TASK-144`
- Objective: remove the phantom; keep guest-freshness detection accurate.
- Precondition: Step 2 complete (guest no longer reads top-level `wit/`).
- Postcondition: `wit/` absent; `xtask` walks the canonical dir; AC-8 clean.
- Files allowed to read: `xtask/src/build_guests.rs` (≈470–500 only).
- Files allowed to edit (≤3): `xtask/src/build_guests.rs`; delete `wit/` tree.
- Files explicitly out-of-bounds: `xtask/src/build_guests.rs` in full (>600? if so range only).
- Expected sub-agent dispatches:
  - `Run cargo xtask build-guests --check; return FACT clean or STALE: list.`
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-1 (`test ! -e wit`), AC-8 (`build-guests --check` clean).
- Exit condition: AC-1 `EXIT=0`, AC-8 reports no `STALE:`.

### Step 4: Migrate the host onto `bindgen! path:` + collapse `module-error`

- Task IDs: `TASK-144`, `TASK-145`
- Objective: replace the four `inline:` bindgen blocks with `path:` against the canonical dir; remap `with:` keys; sweep generated name churn; point the `ModuleError` re-export at the shared world type.
- Precondition: Step 1 complete (canonical dir exists). May run before or after Steps 2–3 but the regression gate must be re-run here.
- Postcondition: AC-3 passes; workspace builds; AC-7 + benchy green.
- Files allowed to read: `crates/slicer-runtime/src/wit_host.rs` (four `bindgen!` sites + `with:` maps + ≈455 only); `dispatch.rs` only at the compiler-flagged error lines.
- Files allowed to edit (≤3): `crates/slicer-runtime/src/wit_host.rs`; `crates/slicer-runtime/src/dispatch.rs` (only if name churn reaches it).
- Files explicitly out-of-bounds: `wit_host.rs`/`dispatch.rs` in full — fix by compiler error, do not browse.
- Expected sub-agent dispatches:
  - `List wit-bindgen-generated type names in wit_host.rs referencing the layer world's module-error/extrusion-path types; return LOCATIONS.`
  - `Run cargo build -p slicer-runtime; return FACT pass/fail + first error ≤20 lines.`
  - `Run cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd; FACT pass/fail.`
- Context cost: `M`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-3 grep (`EXIT=0`); `cargo build -p slicer-runtime`; rebuild guests; AC-7 + `benchy_end_to_end_tdd` FACT pass.
- Exit condition: AC-3 `EXIT=0`, host builds, regression tests green.

### Step 5: Conformance test + docs

- Task IDs: `TASK-144`, `TASK-145`
- Objective: add `wit_single_source_tdd.rs` (resolve canonical dir via `wit_parser`; assert worlds present; assert an illegal-label fragment is rejected; assert no `inline:` survives in `wit_host.rs`); write the wit `README.md`; edit `docs/03` + `CLAUDE.md`.
- Precondition: Steps 1–4 complete.
- Postcondition: AC-9, AC-N1, and all Doc Impact greps pass.
- Files allowed to read: `crates/slicer-runtime/tests/guest_fixture_freshness_tdd.rs` (≤40 lines, as a `wit_parser`/dev-dep usage pattern); the SUMMARY of `docs/03`.
- Files allowed to edit (≤3): `crates/slicer-runtime/tests/wit_single_source_tdd.rs` (new); `crates/slicer-schema/wit/README.md` (new); `docs/03_wit_and_manifest.md` + `CLAUDE.md` (counted as the doc surface).
- Files explicitly out-of-bounds: `docs/03` in full — edit the section located by the SUMMARY.
- Expected sub-agent dispatches:
  - `Run cargo test -p slicer-runtime --test wit_single_source_tdd; FACT pass/fail + assertion ≤20 lines.`
  - `Confirm wit_parser is reachable as a dev-dependency of slicer-runtime (transitive via wasmtime/wit-bindgen); return FACT yes/no + the Cargo.toml line to add if no.`
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` (edit located section only).
- OrcaSlicer refs: none.
- Verification: AC-9 + AC-N1 (`wit_single_source_tdd`); Doc Impact greps (`rg -q 'crates/slicer-schema/wit' docs/03_wit_and_manifest.md`, `… CLAUDE.md`, `test -f crates/slicer-schema/wit/README.md`).
- Exit condition: conformance test green, all Doc Impact greps return a hit.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | scratch spike; FACT-only dispatch |
| Step 1 | M | author 7 wit files; reconcile to compiled shape |
| Step 2 | M | macro rewrite; gated by roundtrip + benchy |
| Step 3 | S | delete + one path string |
| Step 4 | M | host path: + name churn in 6000-line file (range-read only) |
| Step 5 | M | new test + 3 doc edits |

Aggregate: `M`. No step is `L`.

## Packet Completion Gate

- All steps complete; every exit condition met.
- AC-1…AC-9 + AC-N1 dispatched and PASS; all Doc Impact greps hit.
- `cargo check --workspace` + `cargo clippy --workspace -- -D warnings` green.
- Guests rebuilt; `cargo xtask build-guests --check` clean.
- `docs/07_implementation_status.md` TASK-144/TASK-145 notes updated (via worker dispatch — never load the full backlog) to record that the consolidation is now actually complete.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm the three gate commands are green.
- Record any residual risk (e.g. Step 0 fell back to option B / build.rs) explicitly before flipping to `status: implemented`.
- Confirm implementer peak context stayed < 70%; if not, log it as a packet-authoring lesson.
