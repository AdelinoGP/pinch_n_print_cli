---
status: draft
packet: 114_host-services-wit-unification
task_ids: []
backlog_source: docs/adr/0002-wit-marshalling-type-unification.md
context_cost_estimate: M
---

# Packet Contract: 114_host-services-wit-unification

## Goal

Hoist the four byte-identical per-world `host-services` WIT interfaces into one shared `slicer:common/host-services`, remap it (and the already-shared `slicer:common/module-errors`) onto the layer world via `bindgen!`'s `with:`, and collapse the four duplicated host `Host`-trait impls to one each — extending ADR-0002's remap pattern from geometry/config to the `slicer:common` interfaces.

## Scope Boundaries

This packet changes the WIT contract's interface *location* (host-services moves from four inline copies to one shared package interface) and the host-side bindgen remap, then deletes the three redundant Rust `Host` impls per interface. It is an ABI-import-namespace change, so all guests must be rebuilt and re-linked. The marshalling-converter consolidation (ADR-0021) is a separate concern and lives in packet 113; this packet does not touch `src/marshal/` or the converters.

## Acceptance Criteria

Origin/backlog note: net-new architecture-review work governed by ADR-0002 (this packet completes its remap pattern); no open `docs/07` TASK id. Full scope/matrix in `requirements.md`.

- **AC-1** — Given the relocation, When this packet lands, Then `interface host-services` is defined exactly once, in `crates/slicer-schema/wit/deps/common.wit`, and no `world-*.wit` declares an inline `interface host-services`. | `rg -n 'interface host-services' crates/slicer-schema/wit/deps/common.wit && ! rg -n 'interface host-services' crates/slicer-schema/wit/deps/world-layer/ crates/slicer-schema/wit/deps/world-prepass/ crates/slicer-schema/wit/deps/world-finalization/ crates/slicer-schema/wit/deps/world-postpass/`

- **AC-2** — Given the shared interface, When this packet lands, Then each of the four world WITs imports it via `import slicer:common/host-services;`. | `rg -n 'import slicer:common/host-services' crates/slicer-schema/wit/deps/world-*/*.wit | wc -l` (expect `4`)

- **AC-3** — Given the host-side remap, When this packet lands, Then exactly one `impl <…>host_services::Host for HostExecutionContext` block remains in `host.rs` (the layer world's; `phs`/`fhs`/`pphs` copies deleted). | `rg -cn 'host_services::Host for HostExecutionContext' crates/slicer-wasm-host/src/host.rs` (expect `1`)

- **AC-4** — Given the same remap extended to `module-errors`, When this packet lands, Then exactly one `impl <…>module_errors::Host for HostExecutionContext` block remains in `host.rs` (was four at host.rs:1660–1663). | `rg -cn 'module_errors::Host for HostExecutionContext' crates/slicer-wasm-host/src/host.rs` (expect `1`)

- **AC-5** — Given the remap is what collapses the impls, When this packet lands, Then the prepass/finalization/postpass `bindgen!` blocks each remap `slicer:common/host-services` and `slicer:common/module-errors` onto the layer world in their `with:`. | `rg -n 'slicer:common/(host-services|module-errors)' crates/slicer-wasm-host/src/host.rs | wc -l` (expect `>= 6`)

- **AC-6** — Given ADR-0005's invariant, When this packet lands, Then `host.rs` still contains exactly four `bindgen!` invocations. | `rg -c 'bindgen!' crates/slicer-wasm-host/src/host.rs` (expect `4`)

- **AC-7** — Given the WIT change, When guests are rebuilt, Then `cargo xtask build-guests --check` reports no `STALE:` afterward. | `cargo xtask build-guests 2>&1 | tee target/test-output.log; cargo xtask build-guests --check 2>&1 | tee -a target/test-output.log; ! rg -i 'STALE' target/test-output.log`

### Negative Test Cases

- **AC-N1** — Given the host-services import namespace moved from world-local to `slicer:common` (an ABI-visible change), When a guest is dispatched after rebuild, Then typed instantiation succeeds and the host-service round-trips — proving no silent linking/ABI break. The macro guest-roundtrip contract tests pass. | `cargo test -p slicer-runtime --test contract macro_ 2>&1 | tee target/test-output.log; rg 'test result:.*0 failed' target/test-output.log`

## Verification (gate subset)

- `cargo xtask build-guests --check` (after a clean rebuild) — no `STALE:`
- `cargo build --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/adr/0002-wit-marshalling-type-unification.md` — the pattern this packet extends; its "Consequence" note is amended by Step 5.
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — the four-`bindgen!`-in-`host.rs` invariant (AC-6).
- `docs/03_wit_and_manifest.md` — WIT world/interface/import rules; **delegate** a FACT if a specific rule is needed.
- `CLAUDE.md` §"Guest WASM Staleness" and §"WIT/Type Changes Checklist" — the rebuild + cross-file-search obligations.

## Doc Impact Statement (Required)

Amend `docs/adr/0002` "Consequence" (and/or add a short ADR) to record that the shared `slicer:common` interfaces (`host-services`, `module-errors`) are now remapped onto the layer world alongside geometry/config, and that a future fifth world MUST remap them too. This doc edit is Step 5 of this packet.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
