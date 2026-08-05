# Task Map: 165_cli-binary-locator-extraction

Single grouped task ID (`TASK-146d`), emitted because the packet is a sub-lettered slice of the reopened TASK-146 governed by queue row 4 of `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §Packet Queue, and the reviewer requires the explicit crosswalk.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-146d` | `Step 1` | `docs/adr/0004-test-support-lives-in-slicer-sdk.md`; `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §Grounding corrections 1/4/6 | `docs/adr/<NNNN>-host-side-test-support-crate.md` (new; number re-derived at write time) | none — no parity content | S | The home decision is the packet's reason to exist; queue row 4 mandates "an ADR deciding that home" |
| `TASK-146d` | `Step 2` | `docs/adr/<NNNN>-host-side-test-support-crate.md` (Step 1 output) | `crates/slicer-test-support/{Cargo.toml,src/lib.rs}` (new); root `Cargo.toml` member line | none | S | The shared home exists and type-checks (AC-1) |
| `TASK-146d` | `Step 3a` | `.ralph/specs/162_wit-lifecycle-export-removal/design.md` §CLI freshness | `crates/slicer-runtime/Cargo.toml`; `crates/slicer-runtime/tests/common/slicer_cache.rs`; `crates/slicer-runtime/benches/gate_evidence.rs` | none | S | Copies 1 and 2 deleted; DEV-026 evidence producer (gate_evidence) now shares the one locator |
| `TASK-146d` | `Step 3b` | `.ralph/specs/162_wit-lifecycle-export-removal/design.md` §CLI freshness | `crates/slicer-scheduler/Cargo.toml`; `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` | none | S | Copy 3 deleted. AC-2/AC-3 do **not** go green here — the premise correction found four further copies, so both first go green at Step 3c |
| `TASK-146d` | `Step 3c` | `.ralph/specs/165_cli-binary-locator-extraction/design.md` §Code Change Surface → Sites 4–7 | `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`; `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`; `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`; `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs` | none | S | Premise correction: copies 4–7 deleted along with their inert `PROFILE` branch; AC-2/AC-3 first go green here, and AC-8 gates it. No manifest edit — Step 3a's `slicer-runtime` dev-dep covers the whole `tests/` tree |
| `TASK-146d` | `Step 4` | `CLAUDE.md` §Test Discipline | `docs/07_implementation_status.md` (TASK-146d row, via dispatch) | none | S | All gates + green baseline prove queue row 4 closed without behavior change |

Costs copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate S; no row is L.
