# Task Map: pnp-cli-unification

This packet has a single backlog task ID (`TASK-213`) covering net-new refactor work, plus a supersession of an earlier packet. The map below documents how each implementation step contributes to TASK-213 and notes the superseded predecessor.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-213` | Step 1 (rename crate) | none | `crates/slicer-host/` → `crates/slicer-runtime/`, workspace `Cargo.toml` | none | `M` | Mechanical, bounded by `cargo check` SNIPPETS dispatch. |
| `TASK-213` | Step 2 (Producer trait) | none | `crates/slicer-runtime/src/dag.rs` | none | `S` | Trait + adapter; no validator change yet. |
| `TASK-213` | Step 3 (externalise built-ins + broaden seam) | `docs/04_host_scheduler.md` (SUMMARY) | 6 writer modules + `dag.rs` + `dag_cli.rs` + `main.rs` synthetic block + new TDD test | none | `M` | AC-4 + AC-5 land here. |
| `TASK-213` | Step 4 (`run_slice` extract + dead-mod delete) | `docs/01_system_architecture.md` (SUMMARY) | `crates/slicer-runtime/src/{main,run,cli,lib}.rs` + new TDD test | none | `M` | AC-3 lands here; `HostRunOptions` → `SliceRunOptions`. |
| `TASK-213` | Step 5 (validator consolidation) | `docs/03_wit_and_manifest.md` (SUMMARY, only if uncertain) | `slicer-schema` + `slicer-runtime::manifest` + `slicer-cli::cmd_validate` (transitional) | none | `S` | AC-6 lands here. |
| `TASK-213` | Step 6 (`pnp-cli` crate + CLI tests migration) | `docs/05_module_sdk.md` (SUMMARY) | new `crates/pnp-cli/` + 4 migrated tests | none | `M` | AC-1 + AC-9 land here. |
| `TASK-213` | Step 7 (`cmd_new` port + scaffold extension) | none | `crates/pnp-cli/src/module_new.rs` + `crates/pnp-cli/tests/module_new_tdd.rs` | none | `S` | AC-10 lands here. |
| `TASK-213` | Step 8 (slicer-cli delete) | none | `cli/slicer-cli/` removed; workspace `Cargo.toml` | none | `S` | AC-7 + AC-N2 land here. |
| `TASK-213` | Step 9 (`slicer-host` bin removal) | none | `crates/slicer-runtime/Cargo.toml` + `main.rs` deleted | none | `S` | AC-N1 lands here. |
| `TASK-213` | Step 10 (doc/CI/skill sweep) | `CLAUDE.md`, `docs/00`/`05`/`13`/`16`/`17`, `.github/workflows/ci.yml`, living skill files | doc edits, CI yml, skill files | none | `S–M` | AC-11 + every Doc Impact grep land here. |
| `TASK-213` | Step 11 (gate) | none | `packet.spec.md` status flip | none | `S` | AC-2 (smoke) lands here as part of acceptance ceremony. |

Aggregate context cost: `M`. No step is `L`.

## Superseded packet

- `.ralph/specs/_OLD/29_slicer-cli-cmd-run-cross-platform/` — the original `slicer-cli run` cross-platform workflow packet. The `cmd_run.rs` file it implemented is deleted in step 8 of this packet. The supersession is noted in this packet's `requirements.md` Problem Statement; per the cross-packet mutation rule, the old packet's files are NOT edited by this packet — agents reading the historical packet should consult `CLAUDE.md`'s post-merge naming translation note (added in step 10) to map the old binary/crate names.
