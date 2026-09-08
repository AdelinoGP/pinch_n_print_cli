# Task Map: 229-wit-verify-declaration-model

Single-task packet, mapped here because the task row is **net-new**: `TASK-340` does not yet exist in `docs/07_implementation_status.md` (no `TASK-340` row exists on disk) and must be created by Step 8 under "Workstream 5 — Governance and closure drift". Re-derive the highest id at write time with `rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -1` and renumber on collision.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-340` | `Step 1` | `docs/specs/guest-freshness-artifact-verification-plan.md` | `xtask/src/wit_verify.rs` (read), `xtask/src/build_guests.rs` `build_one` (read) | none — no parity surface | `S` | Proves the deletion list and the single-consumer claim before any rewrite |
| `TASK-340` | `Step 2` | `docs/specs/guest-freshness-artifact-verification-plan.md` §C12 | `xtask/Cargo.toml`, `xtask/src/wit_verify.rs` `macro_embedded_wit_files` | none | `S` | AC-1: canonical list == the macro's 20 `include_str!` files, `root.wit` excluded |
| `TASK-340` | `Step 3` | `docs/specs/guest-freshness-artifact-verification-plan.md` §C3, R5-5 | `xtask/src/wit_verify.rs` `WorldModel`, `canonical_world_model`, `embedded_world_model`, `VerifyError` | none | `M` | AC-12, AC-N1..AC-N3: both sides on `wit_parser`, fail-closed errors |
| `TASK-340` | `Step 4` | `docs/specs/guest-freshness-artifact-verification-plan.md` §C3, R5-1, R5-8 | `xtask/src/wit_verify.rs` `compare_worlds`, `Drift`, `StageExpectation`, `SHARED_PACKAGES` | none | `M` | AC-3..AC-10, AC-N4: three comparison directions plus ABI order sensitivity |
| `TASK-340` | `Step 5` | `CLAUDE.md` §"In-Tree Citation Style" | `xtask/src/wit_verify.rs` (deletions), `xtask/src/build_guests.rs` `build_one` + `BuildError` (new variant plus `StaleEmbeddedWorld` payload retyped to `Vec<Drift>`) | none | `S` | AC-13: no hand-rolled scanner symbol survives, `TypeMismatch` included |
| `TASK-340` | `Step 6` | `CLAUDE.md` §"Guest WASM Staleness" | `crates/slicer-macros/build.rs` | none | `S` | AC-2: watch list == the same 20 files; 4 dead `world-*.wit` paths removed |
| `TASK-340` | `Step 7` | `docs/specs/guest-freshness-artifact-verification-plan.md` R5-10 | `xtask/src/wit_verify.rs` `#[cfg(test)] mod tests` | none | `S` | AC-11: real prepass + finalization artifacts verify clean, assert not skip |
| `TASK-340` | `Step 8` | `docs/07_implementation_status.md` §"Workstream 5 — Governance and closure drift" | `docs/07_implementation_status.md` | none | `S` | AC-14: the `TASK-340` row itself, plus the closure gates |

Costs copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate `M`; no row is `L`, so no split is required before activation.
