# Task Map: 200-batched-host-bridge-wasm-arms

`docs/07_implementation_status.md` carries no TASK rows for the multi-edition distribution program (verified in the plan, 2026-08-07; the file is additionally frozen while the parallel 194–199 session is active). This packet therefore anchors to the DEV-094 deviation row and the governing ADRs; if the plan's Backlog anchoring [FWD] later materializes a "Distribution & Editions" workstream in `docs/07`, its packet-200 row maps to the steps below unchanged.

| Backlog anchor | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `DEV-094` (evidence baseline; ADR-0055 open question) | `Step 1` | `docs/adr/0055-fuel-based-module-profiling.md` | none (target/ artifacts) | none | S | A/B "before" leg; without it Step 6 cannot compare |
| `DEV-094` (contract carrier for parity-preserving migration) | `Step 2` | `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` | `crates/slicer-schema/wit/deps/common.wit`, `crates/slicer-wasm-host/src/host.rs` | none | S | `arc-tolerance-mm` on `offset-polygons`/`offset-request` |
| `DEV-094` (the seven unbridged wrappers — core defect) | `Step 3` | `docs/adr/0033-…`, `docs/adr/0049-batched-host-services-over-threaded-guests.md` | `crates/slicer-sdk/src/host.rs`, `crates/slicer-sdk/src/host_batch.rs` | none | M | wasm32 arms; native arms frozen |
| `DEV-094` (proof the bridge is live end to end) | `Step 4` | `docs/05_module_sdk.md` §Host Service Wrappers | `crates/slicer-wasm-host/test-guests/sdk-host-bridge-guest/`, `crates/slicer-runtime/tests/integration/host_bridge_roundtrip_tdd.rs` (+ `main.rs` mod line) | none | M | AC-3/AC-4/AC-N1; pre-fix behavior is impossible to reproduce post-fix — DEV-094 row documents it |
| `DEV-094` (hot-consumer migration, classic-perimeters) | `Step 5` | `docs/adr/0049-…` §Consequences | `modules/core-modules/classic-perimeters/src/lib.rs` | none | M | 12 call sites; zero fixture re-record invariant |
| `DEV-094` (ADR-0055 open question closed on evidence) | `Step 6` | `docs/adr/0055-…` | none (target/ artifacts; conditional Step-5 revert) | none | S | decision rule pre-declared in `design.md` §Risks |
| `DEV-094` (log closure + doc truth) | `Step 7` | `docs/DEVIATION_LOG.md`, `docs/adr/0055-…`, `docs/05_module_sdk.md` | the three doc files | none | S | AC-8/AC-9 greps |

Costs copied from `implementation-plan.md`. Aggregate M; no row is L.
