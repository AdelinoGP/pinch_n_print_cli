# Implementation Plan: 114_host-services-wit-unification

## Execution Rules

- Steps 1–3 are one **compile unit**: the crate does not build between them (the inline interface is removed before the host remap is added). Do not run a build gate until Step 3 completes.
- A WIT edit is not effective in guests until rebuilt: Step 4 runs `cargo xtask build-guests`; no guest/dispatch test is trusted before it (CLAUDE.md staleness rule).
- Delegate every `cargo`/`xtask` run; absorb only `FACT` pass/fail + first error/assertion. Tee to `target/test-output.log`.
- No step edits more than 3 files or rates L.

## Steps

### Step 1 — Define shared `host-services`; migrate layer + prepass worlds
- Task ids: ADR-0002 extension (A2). Objective: move the host-services interface body into `common.wit`; point the layer and prepass worlds at it.
- Precondition: packet 113 need not be done — independent; but if both run, do 113 first (lower risk).
- Postcondition: `common.wit` declares `interface host-services`; `world-layer.wit` and `world-prepass.wit` have no inline interface and `import slicer:common/host-services;`.
- Read: `world-layer.wit:1–34` (one copy of the block + imports); ADR-0002.
- Edit (≤3): `deps/common.wit`, `deps/world-layer/world-layer.wit`, `deps/world-prepass/world-prepass.wit`.
- Dispatches: WIT import-syntax FACT (only if uncertain).
- Context cost: **M**.
- Verify (deferred to Step 3): `rg -n 'interface host-services' crates/slicer-schema/wit/deps/common.wit`.
- Exit condition: `common.wit` has the interface; the two migrated worlds import it.

### Step 2 — Migrate finalization + postpass worlds
- Objective: same edit for the remaining two worlds.
- Precondition: Step 1 done.
- Postcondition: all four worlds import `slicer:common/host-services`; no inline copy remains anywhere (AC-1, AC-2 satisfiable).
- Read: none (mirror Step 1's edit).
- Edit (≤2): `deps/world-finalization/world-finalization.wit`, `deps/world-postpass/world-postpass.wit`.
- Dispatches: none.
- Context cost: **S**.
- Verify (deferred to Step 3): `rg -n 'import slicer:common/host-services' crates/slicer-schema/wit/deps/world-*/*.wit | wc -l` == 4.
- Exit condition: AC-1, AC-2 grep clauses hold.

### Step 3 — Host remap + collapse impls
- Objective: add `with:` remaps for `slicer:common/host-services` and `slicer:common/module-errors` to the prepass/finalization/postpass `bindgen!` blocks; delete the three `phs/fhs/pphs host_services::Host` impls and the three redundant `module_errors::Host` impls; keep the layer one each. Run the CLAUDE.md WIT-checklist search for stray `host-services` references.
- Precondition: Steps 1–2 done.
- Postcondition: AC-3, AC-4, AC-5, AC-6 hold; `cargo build --workspace --all-targets` passes (host side; guests not yet rebuilt).
- Read: `host.rs:246–525` (bindgen blocks), `:1660–1663`, `:3282+, 3549+, 4211+`.
- Edit (≤1): `host.rs`.
- Dispatches: `cargo build --workspace --all-targets` (FACT pass/fail + first error); WIT-checklist grep for `host-services` across `host.rs`/`dispatch.rs`/`wit-guest` (FACT: any stray site?).
- Context cost: **M**.
- Verify: `rg -c 'impl hs::Host for HostExecutionContext' crates/slicer-wasm-host/src/host.rs` == 1 (the `hs` alias is `layer::slicer::common::host_services`) and `! rg -n 'impl (phs|fhs|pphs)::Host for HostExecutionContext' …`; `rg -cn 'module_errors::Host for HostExecutionContext' …` == 1; `rg -c 'component::bindgen!' …` == 4.
- Cheapest falsifier: build fails (path mismatch) or any count wrong.

### Step 4 — Rebuild guests; prove relink
- Objective: rebuild all guests; confirm freshness; run the macro guest-roundtrip tests to prove typed instantiation still links against the relocated import.
- Precondition: Step 3 done (host builds).
- Postcondition: AC-7 (no `STALE:`) and AC-N1 (roundtrip `0 failed`) pass.
- Read: none.
- Edit (≤1): only if the WIT-checklist/roundtrip surfaces a guest-side hardcoded path ([FWD]); otherwise none.
- Dispatches: `cargo xtask build-guests` then `--check` (FACT: ok? STALE?); `cargo test -p slicer-runtime --test contract macro_` (FACT pass/fail + first failing assertion).
- Context cost: **M**.
- Verify: `cargo xtask build-guests --check` no `STALE:`; macro roundtrip bucket `0 failed`.
- Cheapest falsifier: a guest fails typed instantiation → relink broke.

### Step 5 — Amend ADR-0002
- Objective: record in `docs/adr/0002` "Consequence" that `slicer:common/{host-services, module-errors}` are remapped onto layer alongside geometry/config, and a future world MUST remap them.
- Precondition: Steps 1–4 done (decision realized).
- Postcondition: ADR-0002 reflects the extended remap set.
- Read: `docs/adr/0002` (full).
- Edit (≤1): `docs/adr/0002-wit-marshalling-type-unification.md`.
- Dispatches: none.
- Context cost: **S**.
- Verify: `rg -n 'host-services' docs/adr/0002-wit-marshalling-type-unification.md`.
- Exit condition: amendment present.

### Step 6 — Packet completion gate
- Objective: full gate green.
- Precondition: Steps 1–5 done.
- Postcondition: all ACs pass.
- Edit: none (fixes only within in-scope files if a gate fails).
- Dispatches: `cargo build --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask build-guests --check` (FACT each).
- Context cost: **S**.
- Verify: gate subset in `packet.spec.md` all green; AC-1…AC-7, AC-N1 all pass.
- Exit condition: every AC verification command passes.

## Per-Step Budget Roll-Up

M, S, M, M, S, S → aggregate **M**. No L step. Largest: Steps 1/3/4.

## Packet Completion Gate

- AC-1…AC-7 and AC-N1 all pass.
- `cargo build --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no `STALE:`; `rg -c 'component::bindgen!' host.rs` == 4 (ADR-0005 untouched).
- ADR-0002 amended.

## Acceptance Ceremony

Run the gate subset, then each per-AC command, recording FACTs. This packet does **not** require `cargo test --workspace`; AC-N1's targeted macro-roundtrip bucket plus the build/clippy/freshness gates cover the contract change. If closure policy mandates the full suite, delegate it to a sub-agent returning only `FACT pass/fail + first failing test`.
