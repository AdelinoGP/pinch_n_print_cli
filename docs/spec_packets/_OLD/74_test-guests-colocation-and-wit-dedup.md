---
status: implemented
packet: 74
task_ids: [TASK-215]
---

# 74_test-guests-colocation-and-wit-dedup

## Goal

Relocate the `test-guests/` tree under `crates/slicer-runtime/`, collapse its per-guest `target/` directories into one shared build target, repoint the four hand-rolled guests at the canonical WIT source, and extract the positional test-witness encoding into a shared codec — with zero change to host/runtime behavior or the canonical WIT contract.

## Problem Statement

The `test-guests/` tree is the host integration-test fixture set: 12 WASM-component guests consumed exclusively by `crates/slicer-runtime/tests/*`. Three frictions have accumulated, all hurting agent navigability and contradicting decisions already made in this repo:

1. **The fixtures sit two levels away from their only consumer.** Every test loads `../../test-guests/<g>.component.wasm`. An agent reading `slicer-runtime/tests/` cannot see the fixtures beside the code that uses them.
2. **Each guest is its own workspace, minting its own `target/`.** Twelve independent `target/` trees bloat disk and slow every filesystem scan — `.gitignore` already lists `test-guests/*/target/` to cope.
3. **Four "raw" guests still inline a verbatim copy of the WIT world.** Packet 72 unified host + macro onto the canonical `crates/slicer-schema/wit/` single source, but `prepass-guest`, `layer-infill-guest`, `finalization-guest`, and `postpass-guest` still paste the contract into `wit_bindgen::generate!({ inline: … })`. The copies are policed by a drift sub-test instead of being made structurally impossible.

A fourth, softer friction: the guest↔host-test signal is smuggled through positional `Point3WithWidth` fields (`point[0].x = region_count`, …) whose meaning lives only in comments on both sides, re-derived in ~5 test files.

This packet co-locates the fixtures with their consumer, collapses the build sprawl, removes the inline-WIT duplication, and gives the witness encoding one owning module — without touching runtime behavior. It continues packet 72 (de-dup) and packet 70 (guest builder), and explicitly **does not** delete the raw guests, which remain the only differential check that the `#[slicer_module]` macro's emitted glue matches hand-rolled `wit-bindgen`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
When this packet's change surface includes any path that feeds guest WASM
(`crates/slicer-schema/wit/**`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`,
`crates/slicer-ir/**`, `crates/slicer-schema/**`, `modules/core-modules/*/src/**`,
`modules/core-modules/*/wit-guest/**`, `test-guests/*/src/**`, or
`test-guests/*/Cargo.toml`), the guest `.wasm` artifacts are **not** rebuilt by
`cargo build`/`cargo test`. Run `cargo xtask build-guests --check` and rebuild
(drop `--check`) on `STALE:` before attributing any guest/host/dispatch test
failure to your edit.
- **D1 preserves packet-70 invariants.** Per-guest `[workspace]` sentinels and the `discover_guests` validation contract stay intact; the only build change is a shared `CARGO_TARGET_DIR`. Do not remove sentinels (that is D2, out of scope).
- **Canonical WIT is read-only here.** A repoints guests *at* `crates/slicer-schema/wit/`; it must not edit those files. If an inline copy diverged, reconcile the *guest* toward canonical, never the reverse.
- **Differential oracle is load-bearing.** The four raw guests must survive; their value is being authored without the `#[slicer_module]` macro.

(Coordinate-system snippet omitted — no geometry / mm↔unit conversion in this packet.)

## Data and Contract Notes

- `wit_bindgen::generate!` resolves `path:` relative to the guest `CARGO_MANIFEST_DIR`; at the new depth the canonical dir is `../../../slicer-schema/wit`.
- SDK guest path deps shift from `../../crates/slicer-X` to `../../../slicer-X` (one level deeper, dropping the now-redundant `crates/` segment because three `../` already lands in `crates/`).
- The `witness` crate must compile for both `wasm32-unknown-unknown` (guest dep) and host (slicer-runtime dev-dep); it may depend only on `slicer-ir` (already wasm-compatible). It is a plain lib (no `cdylib`, no `[workspace]` sentinel, no `wit-bindgen`), so `discover_guests` will list it under SKIP — benign.
- **Workspace-membership check (do not assume "benign"):** because `witness` is a sentinel-less plain lib nested under the `slicer-runtime` package directory, confirm the root workspace does **not** auto-capture it as a member (the per-guest crates avoid this by being `[workspace]` roots; `witness` is not). If the root `Cargo.toml` uses a glob like `members = ["crates/*"]`, the nested `crates/slicer-runtime/test-guests/witness` is not matched and the path dev-dep resolves cleanly; if it uses a deeper/recursive glob, either add an explicit `exclude` or give `witness` its own `[workspace]` sentinel. Verify both build directions before closing Step 5: `cargo check -p witness` (host) **and** `cargo check -p witness --target wasm32-unknown-unknown` (guest), plus `cargo metadata --no-deps` must not list `witness` as an unintended root-workspace member.

## Locked Assumptions and Invariants

- The 12 buildable guests and their world/package names are unchanged by relocation; only paths move. A may *reconcile* a raw guest's surface toward canonical if it diverged, but introduces no new WIT types.
- D1 introduces no behavior lock: removing the shared `CARGO_TARGET_DIR` reverts to per-guest targets with no source change. Per-guest `[workspace]` sentinels remain, so the change is reversible.
- Production runtime behavior and the canonical WIT contract are invariant across this packet.

## Risks and Tradeoffs

- **Inline-vs-canonical divergence (A):** a raw guest's pasted WIT may have drifted (extra/renamed items). `wit_boundary_tdd` + `wit_drift_detection_tdd` are the safety net; reconcile toward canonical. Risk: medium, contained by tests.
- **`build_one` intermediate path (D1):** if the shared `CARGO_TARGET_DIR` is set but the intermediate `.wasm` lookup isn't updated in lockstep, builds fail fast (missing-intermediate error) — loud, not silent.
- **Path-constant lockstep:** the dominant error source is missing one of {xtask, 18 tests, gitignore, CLAUDE.md}. AC-N1 + AC-1 catch leftovers.
- **Witness codec scope creep (C):** keep to SDK guests + the 5 named decoders; do not migrate raw guests.
