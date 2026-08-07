# Task Map: 201-integrated-module-registry-tier5

No `docs/07_implementation_status.md` TASK rows exist for the multi-edition program (verified in `docs/specs/multi-edition-distribution-plan.md` §"Backlog anchoring [FWD]"); the crosswalk therefore maps ADR-0056 decision clauses to packet steps.

| Backlog anchor | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `ADR-0056 §Decision 1` (one model, ingestion generalized over artifact source) | Steps 1–2 | `docs/adr/0056-integrated-modules-native-dispatch.md`, `docs/04_host_scheduler.md` §Phase 1 | `crates/slicer-scheduler/src/manifest.rs` | none | M | Text-source ingestion + provenance prove the "identical ingestion" clause |
| `ADR-0056 §Decision 2` (tier 5, first-root-wins unchanged) | Steps 1–2 | `docs/01_system_architecture.md` §Module Search Path | `crates/slicer-scheduler/src/manifest.rs` | none | S | Dedup + shadow diagnostic (AC-2/AC-N1) |
| `ADR-0056 §Decision 1` (manifest embedded via `include_str!`) | Step 3 | `docs/adr/0057-three-editions-and-integrated-tier.md` | `crates/slicer-integrated-modules/` (new) | none | S | Registry home decision recorded in design.md |
| `ADR-0056 §Decision 1–2` (production pipeline carries the tier) | Steps 4–5 | `docs/adr/0056-integrated-modules-native-dispatch.md` | `crates/slicer-wasm-host/src/execution_plan_live.rs`, `crates/slicer-runtime/src/run.rs` | none | M | Compile-skip guard is the 201→202 seam |
| `ADR-0056 §Consequences` (shadow diagnostic wording) | Step 6 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | docs only | none | S | Doc greps AC-N3 |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
