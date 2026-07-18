# Design: 114_host-services-wit-unification

## Controlling Code Paths / Likely Surfaces

- `crates/slicer-schema/wit/deps/common.wit` — already declares `slicer:common` `package` + `interface module-errors`; gains `interface host-services` (moved body).
- `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:3–18` (inline host-services), `:20` (`import host-services;`) — and the identical blocks in `world-prepass`, `world-finalization`, `world-postpass`. Remove the inline interface; change the import to `import slicer:common/host-services;`.
- `crates/slicer-wasm-host/src/host.rs:246–525` — the four `bindgen!` blocks; the prepass (312), finalization (489), postpass (512) blocks gain `with:` remap entries.
- `host.rs:1660–1663` — four `module_errors::Host` empty impls (collapse to one).
- `host.rs:1682–1803` — layer `hs::Host` impl (the surviving canonical one).
- `host.rs:3282+, 3549+, 4211+` — `phs::Host`, `fhs::Host`, `pphs::Host` impls (delete).
- `docs/adr/0002-wit-marshalling-type-unification.md` — "Consequence" amended (Step 5).

## Neighboring Tests / Fixtures

- `crates/slicer-runtime/tests/contract/macro_*_roundtrip_tdd.rs` — real-WASM guest dispatch; the relink guard (AC-N1).
- `crates/slicer-wasm-host/tests/` — host-side host-services tests, if any, must still pass against the single impl.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Layer remains the canonical owner; the other three worlds remap onto `super::layer::slicer::common::{host_services, module_errors}` (ADR-0002 mechanism). `pub mod layer` already precedes the others, so the `super::layer::…` paths resolve.
- The four `bindgen!` invocations stay (count == 4; ADR-0005). This packet edits their `with:` only.
- The import namespace change (`host-services` → `slicer:common/host-services`) is **ABI-visible**: the component import name changes, so every guest must be rebuilt and re-linked. There is no `coord-system` concern (no geometry/mm math).

## Selected Approach

Shared WIT interface + `bindgen!` `with:` remap — exactly ADR-0002's geometry/config mechanism, extended to `slicer:common`. **Rejected**: a host-only `macro_rules! impl_host_services!` emitting the four Rust impls (the session's "A1"). It would delete the Rust duplication without a guest rebuild, but leaves the interface copy-pasted four times in the WIT — the root duplication — and ADR-0002 already set the macro aside. The user chose the deeper WIT-level unification (A2); this packet implements it.

## Explicit Code Change Surface

- `crates/slicer-schema/wit/deps/common.wit` (+interface).
- `crates/slicer-schema/wit/deps/world-{layer,prepass,finalization,postpass}/world-*.wit` (−inline interface, import rewrite) — four files, split across Steps 1–2 to keep ≤3 edits/step.
- `crates/slicer-wasm-host/src/host.rs` (`with:` remaps + delete six impls).
- `docs/adr/0002-…md` (amend Consequence).

## Read-Only Context the Implementer Needs

- ADR-0002 in full (~55 lines) — the remap syntax and the canonical-layer rule.
- One copy of the host-services WIT block (e.g. `world-layer.wit:3–18`) as the body to move — do not open all four.

## Out-of-Bounds Files

- `crates/slicer-wasm-host/src/marshal/**` and the converters (packet 113).
- `modules/core-modules/*/src/**` — guest Rust is **regenerated** by the macro from the schema, not hand-edited; do not open to "fix" imports.
- `target/`, any `Cargo.lock`, generated `bindgen` output, `OrcaSlicerDocumented/**`.

## Expected Sub-Agent Dispatches

- Step 1: "In WIT, what is the exact syntax for a world to import an interface defined in another package (`slicer:common/host-services`)? Return FACT ≤5 lines, citing `docs/03_wit_and_manifest.md` if needed." (only if uncertain).
- Step 3: "Run `cargo build --workspace --all-targets`; return FACT pass/fail + first error file:line." (the compile unit after Steps 1–3).
- Step 4: "Run `cargo xtask build-guests` then `cargo xtask build-guests --check`; return FACT: build ok? any `STALE:`?" and "Run `cargo test -p slicer-runtime --test contract macro_`; return FACT `test result` line + first failing assertion."

## Data and Contract Notes

- New world import line (all four): `import slicer:common/host-services;` (replacing `import host-services;`). The interface's `use slicer:types/geometry.{…}` stays — geometry is already shared.
- host.rs `with:` additions (prepass/finalization/postpass blocks), e.g.:
  `"slicer:common/host-services": super::layer::slicer::common::host_services,`
  `"slicer:common/module-errors": super::layer::slicer::common::module_errors,`
- Surviving impls: `impl layer::slicer::common::host_services::Host for HostExecutionContext` and `impl layer::slicer::common::module_errors::Host for HostExecutionContext` (named paths may differ post-remap; the count, not the path, is what AC-3/AC-4 assert).

## Locked Assumptions and Invariants

- The host-services and module-errors interface *content* is unchanged — function set, signatures, and enums are identical to today's four copies. Relocation only.
- The remapped Rust types are identical to layer's (the whole point); the three deleted impls were exact duplicates.
- `bindgen!` count stays 4.

## Risks and Tradeoffs

- **Guest relink** is the central risk: a guest that fails typed instantiation after the import moves indicates a missed rebuild or a path mismatch. AC-7 (freshness) + AC-N1 (roundtrip) bracket it.
- **WIT-checklist breadth**: per CLAUDE.md, search `wit_host.rs`/`dispatch.rs`/`wit_guest` for the affected interface after the change; a stray hardcoded `host-services` path would break linking. Low likelihood (macro/SDK do not hardcode it), but the search is mandatory.
- **Compile-broken window** across Steps 1–3 — accepted; the gate runs only after Step 3.

## Context Cost Estimate

- Aggregate: **M**.
- Largest single step: Step 3 (host.rs remap + six deletions) — M.
- Highest-risk dispatch: Step 4 guest rebuild + roundtrip (the relink proof).

## Open Questions

- `[FWD]` If the WIT-checklist search or AC-N1 reveals a guest/SDK site that hardcodes the old `host-services` import path and needs a hand-edit, record it in the step note and edit it within the same step; it does not block activation. If instead it reveals an interface-content divergence between worlds (it should not — they are identical), stop and escalate as `[BLOCK]`.
