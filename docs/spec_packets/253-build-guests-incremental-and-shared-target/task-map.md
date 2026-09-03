# Task Map: 253-build-guests-incremental-and-shared-target

This packet groups one new backlog row. Its numeric `TASK-` id is a mutable ledger fact and is deliberately not frozen here or in `packet.spec.md`. Step 10 re-derives it with `rg -o 'TASK-[0-9]+' docs/07_implementation_status.md | sort -uV | tail -1`, takes the next free number, registers the row through a worker dispatch, and then replaces the `TASK-NEW-BUILD-GUESTS-PERF` placeholder in `packet.spec.md`, `requirements.md`, and this file.

The map is emitted despite the single-task grouping because the row spans four separable phases with distinct code surfaces and one conditional phase, and a reader needs the phase-to-step crosswalk to know which steps may legitimately be skipped.

## Backlog justification

A survey of `docs/07_implementation_status.md` during authoring found no row covering developer or CI build-time performance. Every xtask row is closed correctness work: `TASK-214` replaced the guest-build shell scripts with `cargo xtask build-guests`; `TASK-341` introduced output-based freshness with the artifact-decoding `--check`, `CheckOutcome`, `build_stale_command`, the v2 fingerprint, and the exit-code contract; `TASK-342` replaced the hardcoded shared-crate fingerprint with the per-guest dependency-closure walk; `TASK-343` was the artifact-verified-freshness docs pass and the `cargo test -p xtask` CI wiring. This packet is the first to treat build *time* as the deliverable, so it registers a new row rather than reopening any of those.

## Crosswalk

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| re-derived in Step 10 | `Step 1` | `CONTEXT.md` §"Artifact-verified freshness" | `xtask/src/build_guests.rs`, `xtask/src/main.rs` | none | `S` | Phase A. Proves the default path consults `check_command` and that an infrastructure error aborts rather than falling back. |
| re-derived in Step 10 | `Step 2` | none | `xtask/src/dist.rs` | none | `S` | Phase A. Proves the second caller of the guest build is freshness-aware and `--force-guests` parses. |
| re-derived in Step 10 | `Step 3` | `CLAUDE.md` §"Guest WASM Staleness" | `xtask/src/build_guests.rs` | none | `M` | Phase B. Proves every guest of both trees compiles into `target/guests` and that the stale-WIT recovery path cleans that directory. |
| re-derived in Step 10 | `Step 4` | `CONTEXT.md` §"Artifact-verified freshness" | `xtask/src/build_guests.rs`, `xtask/src/main.rs` | none | `M` | Phase B. Proves lock divergence is detected, reported one line per crate, and classified as staleness rather than an infrastructure error. |
| re-derived in Step 10 | `Step 5` | none | generated `Cargo.lock` set only | none | `S` | Phase B. Proves convergence on the real tree and that no guest build breaks as a result. Creates `measurements.md`. |
| re-derived in Step 10 | `Step 6` | `docs/21_data_defaults_and_fixtures.md` | `xtask/src/build_guests.rs`, `xtask/src/wit_verify.rs` | none | `M` | Phase C. Proves the two version probes and the canonical WIT parse each run once per invocation, with fingerprint values unchanged. |
| re-derived in Step 10 | `Step 7` | `CONTEXT.md` §"Artifact-verified freshness" | `xtask/src/build_guests.rs` | none | `S` | Phase C. Proves the duplicate artifact decode is gone with no observable `StaleReason` changed. |
| re-derived in Step 10 | `Step 8` | none | none; measurement only | none | `S` | Phase D gate. Proves the ship-or-reject decision was made from measured build-time and test-time figures, not intuition. |
| re-derived in Step 10 | `Step 9` | `CONTEXT.md` §"Artifact-verified freshness" | `xtask/src/build_guests.rs`, `xtask/src/dist.rs`, `xtask/src/test.rs` | none | `M` | Phase D, conditional. Skipped entirely if Step 8 recorded a rejection. Owns the `FINGERPRINT_VERSION` bump and its test-assertion fallout. |
| re-derived in Step 10 | `Step 10` | `CLAUDE.md`, `CONTEXT.md`, `docs/03_wit_and_manifest.md` | docs only | none | `S` | Closes the documented-versus-actual gap, records the sccache deferral and its trigger, and registers the backlog row. |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate is `M`; no row is `L`. Step 9 is the split candidate if Phase D ships and overruns, and it is already isolated behind Step 8's decision.

## Reopened or superseded packets

None. This packet supersedes nothing and reopens nothing. No packet directory outside this one is read or modified.
