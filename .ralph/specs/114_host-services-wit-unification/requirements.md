# Requirements: 114_host-services-wit-unification

## Problem Statement

The `host-services` WIT interface — eight host functions (`log`, `raycast-z-down`, `surface-normal-at`, `object-bounds`, `clip-polygons`, `offset-polygons`, `simplify-polygon`, `now-us`) plus its `log-level`/`clip-operation`/`offset-join-type` enums — is declared **inline and byte-for-byte identical** in all four world WITs (`world-layer`, `world-prepass`, `world-finalization`, `world-postpass`, each lines 3–18). Because the interface is per-world rather than a shared package interface, `bindgen!` generates four distinct `Host` traits, forcing four identical Rust impls in `host.rs` (`hs::Host` 1682–1803, `phs::Host` 3282+, `fhs::Host` 3549+, `pphs::Host` 4211+) whose bodies differ only by the world's enum namespace. The same shape afflicts `module-errors`: it is already a shared `slicer:common` interface, but is not remapped, so four empty `Host` impls exist (1660–1663).

ADR-0002 unified geometry/config types with exactly the fix this packet applies — a shared interface remapped onto the layer world via `with:` — but stopped at `slicer:types`/`slicer:config`. Extending the same remap to the `slicer:common` interfaces collapses four host-services impls to one and four module-errors impls to one, deleting ~360 LoC of duplicated Rust and removing the four-way drift hazard in the WIT itself.

## Task Mapping

No open `docs/07` TASK id. Governed by **ADR-0002** (this packet completes its remap pattern). Related closed work for context: TASK-144/145 (WIT canonical-source unification), TASK-150 (converter widening).

## In Scope

- Move the `host-services` interface body into `crates/slicer-schema/wit/deps/common.wit` (one copy).
- Remove the inline `interface host-services` from all four `world-*.wit`; replace `import host-services;` with `import slicer:common/host-services;`.
- Add `with:` remaps in the prepass/finalization/postpass `bindgen!` blocks for `slicer:common/host-services` and `slicer:common/module-errors`, both pointing at the layer world's generated modules.
- Delete the three redundant `host_services::Host` impls and the three redundant `module_errors::Host` impls in `host.rs`; keep the layer world's one each.
- Rebuild all guests (`cargo xtask build-guests`) and verify freshness + dispatch round-trip.
- Amend `docs/adr/0002` "Consequence" to record the `slicer:common` interfaces joining the remap set.

## Out of Scope

- Any change to the host-services or module-errors interface *content* (function set, signatures, enums) — relocation only; the component ABI surface is otherwise unchanged.
- The marshalling-converter consolidation and `src/marshal/` (packet 113).
- The four `bindgen!` invocation count (stays 4; ADR-0005).
- Guest-side Rust edits: the `#[slicer_module]` macro reads the canonical schema and regenerates; no hand-edit to `modules/core-modules/*/src` is expected. If one proves necessary, that is an [FWD] open question, not silent scope creep.

## Authoritative Docs

- `docs/adr/0002` (~55 lines) — read in full; the remap mechanism and its "Considered and rejected" alternatives.
- `docs/adr/0005` — the four-`bindgen!`-in-`host.rs` invariant (skim; AC-6).
- `docs/03_wit_and_manifest.md` (> 600 lines) — **delegate** a FACT if a WIT import/package rule is in doubt; do not read in full.
- `CLAUDE.md` §"Guest WASM Staleness", §"WIT/Type Changes Checklist" — rebuild + cross-file-search obligations.

## Acceptance Summary

Authoritative criteria are AC-1…AC-7 and AC-N1 in `packet.spec.md`. Refinements:

- AC-1/AC-2 are the WIT side; AC-3/AC-4/AC-5 are the host side; they must all hold together — a green host build with the WIT still duplicated (or vice versa) is an incomplete landing.
- AC-7 is non-negotiable: per CLAUDE.md, a WIT edit that is not followed by a guest rebuild surfaces as unrelated-looking test failures. The rebuild must precede AC-N1.
- AC-N1 is the silent-ABI-break guard: the macro guest-roundtrip tests instantiate a real component and call host services, proving the relocated import name still links.

## Verification Commands

| ID | Command | Delegation hint |
|----|---------|-----------------|
| AC-1 | `rg -n 'interface host-services' crates/slicer-schema/wit/deps/common.wit && ! rg -n 'interface host-services' crates/slicer-schema/wit/deps/world-layer/ crates/slicer-schema/wit/deps/world-prepass/ crates/slicer-schema/wit/deps/world-finalization/ crates/slicer-schema/wit/deps/world-postpass/` | FACT: both clauses pass |
| AC-2 | `rg -n 'import slicer:common/host-services' crates/slicer-schema/wit/deps/world-*/*.wit \| wc -l` | FACT: count == 4 |
| AC-3 | `rg -c 'impl hs::Host for HostExecutionContext' crates/slicer-wasm-host/src/host.rs` and `! rg -n 'impl (phs\|fhs\|pphs)::Host for HostExecutionContext' crates/slicer-wasm-host/src/host.rs` | FACT: count==1, second clause empty |
| AC-4 | `rg -cn 'module_errors::Host for HostExecutionContext' crates/slicer-wasm-host/src/host.rs` | FACT: == 1 |
| AC-5 | `rg -n 'slicer:common/(host-services\|module-errors)' crates/slicer-wasm-host/src/host.rs \| wc -l` | FACT: >= 6 |
| AC-6 | `rg -c 'component::bindgen!' crates/slicer-wasm-host/src/host.rs` | FACT: == 4 |
| AC-7 | `cargo xtask build-guests 2>&1 \| tee target/test-output.log; cargo xtask build-guests --check 2>&1 \| tee -a target/test-output.log; ! rg -i 'STALE' target/test-output.log` | FACT: rebuild ok + no STALE |
| AC-N1 | `cargo test -p slicer-runtime --test contract macro_ 2>&1 \| tee target/test-output.log; rg 'test result:.*0 failed' target/test-output.log` | FACT: pass/fail + first failing assertion |
| Gate | `cargo build --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings` | FACT: exit code + first error |

## Step Completion Expectations (cross-step invariants)

- The WIT edits (Steps 1–2) and the host remap (Step 3) must land together before any build is judged: between them the crate will not compile (the inline interface is gone but the remap not yet added). Treat Steps 1–3 as one compile unit; do not run the freshness/dispatch gate until Step 4.
- Guests MUST be rebuilt (Step 4) before AC-N1 or any guest/dispatch test is trusted (CLAUDE.md staleness rule).

## Context Discipline Notes (packet-specific)

- `host.rs` (5225) exceeds the direct-read limit; edit by the line ranges in `design.md`'s surface map only.
- The four `world-*.wit` host-services blocks are known to be identical (lines 3–18 each); do not re-read all four — diff one against `common.wit` after the move if confirmation is needed.
