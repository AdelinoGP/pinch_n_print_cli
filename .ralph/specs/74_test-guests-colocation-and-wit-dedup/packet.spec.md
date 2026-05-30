---
status: active
packet: 74
task_ids: [TASK-215]
backlog_source: docs/07_implementation_status.md
---

# Packet 74 — Test-guest co-location, target-dir consolidation, and WIT de-duplication

## Goal

Relocate the `test-guests/` tree under `crates/slicer-runtime/`, collapse its per-guest `target/` directories into one shared build target, repoint the four hand-rolled guests at the canonical WIT source, and extract the positional test-witness encoding into a shared codec — with zero change to host/runtime behavior or the canonical WIT contract.

## Scope Boundaries

This packet moves and tidies test fixtures and their build wiring only. It touches the `test-guests/` guest crates, `xtask/src/build_guests.rs`, the `slicer-runtime` integration tests that load guest `.component.wasm` artifacts, `.gitignore`, and path references in `CLAUDE.md`. It does **not** alter any production crate behavior, the canonical WIT at `crates/slicer-schema/wit/`, or the core-module guests under `modules/core-modules/*/wit-guest/`. The four hand-rolled "raw" guests are **retained** (they are the macro-vs-hand-rolled differential oracle); only their inline WIT text is removed. Full in/out lists live in `requirements.md`.

## Acceptance Criteria

> Verification commands assume repo root `F:\slicerProject\pinch_n_print` and a POSIX shell (Git Bash). `cargo xtask` is the documented alias for the guest builder. Each command is delegation-friendly (exit code or a single integer).

**AC-1 — Guests discovered at the new location.**
Given the `test-guests/` tree has been moved to `crates/slicer-runtime/test-guests/`, When the guest builder enumerates guests, Then all 12 test-guests are discovered with artifact paths under `crates/slicer-runtime/test-guests/` and none is reported missing. | `cargo xtask build-guests --list 2>/dev/null | grep -c "crates/slicer-runtime/test-guests/.*\.component\.wasm"` → `12`

**AC-2 — One shared target directory (D1).**
Given the D1 shared-`CARGO_TARGET_DIR` build, When every test-guest is built, Then exactly one `target/` directory exists in the relocated tree (the shared build dir), not one per guest. | `bash -c 'cargo xtask build-guests >/dev/null 2>&1; find crates/slicer-runtime/test-guests -maxdepth 2 -type d -name target | wc -l'` → `1`

**AC-3 — Raw guests carry no inline WIT and read the canonical source (A).**
Given the four raw guests (`prepass-guest`, `layer-infill-guest`, `finalization-guest`, `postpass-guest`), When their `src/lib.rs` is inspected, Then none contains an `inline:` WIT blob and each `wit_bindgen::generate!` uses `path:` resolving to `crates/slicer-schema/wit`. | `bash -c 'cd crates/slicer-runtime/test-guests; test $(grep -l "inline:" {prepass,layer-infill,finalization,postpass}-guest/src/lib.rs | wc -l) -eq 0 && test $(grep -l "path:.*slicer-schema/wit" {prepass,layer-infill,finalization,postpass}-guest/src/lib.rs | wc -l) -eq 4; echo $?'` → `0`

**AC-4 — Drift coverage updated for canonical-source guests (A).**
Given the inline copies are gone, When the drift test runs, Then it passes and the now-obsolete sub-test that grepped guest source for inline WIT package strings is removed. | `bash -c 'grep -c "fn handwritten_test_guests_use_payload_extrusion_role_variants" crates/slicer-runtime/tests/wit_drift_detection_tdd.rs'` → `0`, and `cargo test -p slicer-runtime --test wit_drift_detection_tdd` → all pass (exit 0)

**AC-5 — Differential oracle preserved (B-guard).**
Given de-duplication only removed inline WIT text, When the guest set is listed, Then all four raw guests still exist with their `src/lib.rs`. | `ls crates/slicer-runtime/test-guests/{prepass,layer-infill,finalization,postpass}-guest/src/lib.rs 2>/dev/null | wc -l` → `4`

**AC-6 — Witness encoding has a single owning module, used by producer and consumer (C).**
Given the witness codec crate exists at `crates/slicer-runtime/test-guests/witness/`, When `sdk-layer-infill-guest` and `sdk-finalization-guest` and the migrated host decoders are inspected, Then the codec exposes `encode`/`decode` for the infill and finalization layouts, the two SDK guests call `encode`, and the migrated host tests pass. | `bash -c 'test -f crates/slicer-runtime/test-guests/witness/src/lib.rs && grep -lq "witness::" crates/slicer-runtime/tests/dispatch_tdd.rs'; echo $?` → `0`, and `cargo test -p slicer-runtime --test dispatch_tdd` → all pass (exit 0)

**AC-7 — Guest round-trip behavior unchanged.**
Given the full relocate+D1+A+C change set, When the broad guest round-trip suites run, Then they pass unchanged. | `cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd --test finalization_live_tdd` → all pass (exit 0)

**AC-N1 — No stale `../../test-guests/` path survives (regression guard).**
Given every consuming test was repointed, When the test tree is searched, Then no `slicer-runtime` test references the pre-move `../../test-guests/` artifact path. | `grep -rl "\.\./\.\./test-guests/" crates/slicer-runtime/tests --include=*.rs | wc -l` → `0`

**AC-N2 — Canonical-source guests still satisfy the host WIT boundary (silent-regression guard for A).**
Given the raw guests now bind the canonical WIT instead of an inline copy, When the WIT boundary test runs, Then the host still instantiates and round-trips them. | `cargo test -p slicer-runtime --test wit_boundary_tdd` → all pass (exit 0)

## Verification (closure gate)

- `cargo xtask build-guests --check` (then rebuild if `STALE:`) — guests fresh at new location.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test -p slicer-runtime --test wit_drift_detection_tdd --test wit_boundary_tdd --test dispatch_tdd --test macro_all_worlds_roundtrip_tdd` — green.

Full per-AC matrix with delegation hints lives in `requirements.md`.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT worlds, host-boundary, `bindgen!`/macro single-source rule.
- `docs/05_module_sdk.md` — guest build flow (`cargo build --target wasm32-unknown-unknown` + `wasm-tools component new`) and the `cargo xtask build-guests` two-step.
- `.ralph/specs/70_workspace-aware-guest-builder/` — predecessor: established the validated filesystem-walk guest builder this packet edits (read via SUMMARY, do not re-read all 5 files).
- `.ralph/specs/72_wit-single-source-unification/` — predecessor: unified host+macro onto canonical WIT; this packet closes the surviving exception (the four raw inline-WIT guests).

## Doc Impact Statement (Required)

- `CLAUDE.md` — update every `test-guests/*` path reference (Guest WASM Staleness section, Post-Merge naming note, WIT/Type Changes checklist) to `crates/slicer-runtime/test-guests/*`; note the single shared guest `target/` location.
- `docs/05_module_sdk.md` — update the guest build-flow paths if it cites `test-guests/`.
- The two `skills/**/wasm-staleness.md` snippet files that cite `test-guests/*/src` — update paths.

## OrcaSlicer Reference Obligations

None — this packet is test-infrastructure and build wiring; there is no OrcaSlicer parity surface.

## Context Discipline Note

<!-- snippet: context-discipline -->
Treat your context window as a scarce, non-renewable resource. Reading is the
most expensive thing you do; a sub-agent that returns one fact is cheaper than
opening one large file. Before opening any file, ask whether a delegated
dispatch could return just the answer. Read by line-range, never whole large
files. Stop reading at 60% of budget and finalize, hand off, or delegate.
