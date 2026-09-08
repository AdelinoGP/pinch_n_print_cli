# Task Map: 202-native-adapter-and-dispatch

No `docs/07_implementation_status.md` TASK rows exist for the multi-edition program (verified in `docs/specs/multi-edition-distribution-plan.md` §"Backlog anchoring [FWD]"); the crosswalk therefore maps ADR-0056 decision clauses to packet steps.

| Backlog anchor | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `ADR-0056 Decision item 3` (macro grows a native adapter emitting the same stage contract) | Steps 1–3 | `docs/adr/0056-integrated-modules-native-dispatch.md`, `docs/05_module_sdk.md` | `crates/slicer-sdk/src/native.rs` (new), `crates/slicer-macros/src/lib.rs` | none | M | Table-driven from `slicer_schema::STAGES`; AC-1 |
| `ADR-0056 Decision item 3` (provenance decides dispatch behind the ADR-0005 seam) | Steps 4–5 | `docs/adr/0005-runner-traits-in-slicer-wasm-host.md`, `docs/04_host_scheduler.md` §Phase 4 | `crates/slicer-wasm-host/src/{marshal/native.rs,dispatch.rs,binding.rs}` | none | M | One marshalling answer; AC-3 |
| `ADR-0056 Decision item 4` (parity gate needs a dual-path seam — gate itself is 204) | Step 6 | `docs/adr/0056-integrated-modules-native-dispatch.md` | `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` | none | M | AC-2 structural, no byte-equality |
| `ADR-0056 Decision item 2` (external override → WASM path) | Step 7 | `docs/adr/0056-integrated-modules-native-dispatch.md` | `crates/slicer-wasm-host/src/execution_plan_live.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-integrated-modules/src/lib.rs` | none | M | AC-4, AC-N1, AC-N2 |
| `ADR-0056 Decision item 5` (single-threaded module logic both paths) | Steps 4–8 (constraint) + Step 8 docs | `docs/adr/0056-integrated-modules-native-dispatch.md`, `docs/04_host_scheduler.md` | no rayon in adapters/marshal | none | S | Documented in Phase 4 paragraph; AC-N3 |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
