# Design — Packet 74

## Controlling Code Paths / Likely Surfaces

- **Guest tree (move target):** `test-guests/` → `crates/slicer-runtime/test-guests/`. 12 guest crates, each a standalone `[workspace]` `cdylib` (per packet 70). The empty `sdk-layer-plan-guest/` (no `Cargo.toml`) is deleted.
- **Guest builder:** `xtask/src/build_guests.rs`.
  - `discover_guests` (lines ~88–259): the test-guest branch starts at `let tg_root = ws_root.join("test-guests");` (line 175) and records `artifact_path = format!("test-guests/{dir_name}.component.wasm")` (line 242). Both strings change to the new location. Validation (`has_cdylib` + `has_workspace_sentinel` + `has_wit_bindgen`) is **unchanged** — D1 keeps per-guest `[workspace]` sentinels.
  - `build_one` (lines ~345–413): add a shared `CARGO_TARGET_DIR` env to the per-guest `cargo build` (D1), and recompute the intermediate `wasm32-unknown-unknown/release/<lib>.wasm` input path it hands to `wasm-tools component new` so it reads from the shared target dir instead of each guest's local `target/`. (The current intermediate path is hardcoded as `spec.guest_dir.join("target/wasm32-unknown-unknown/release").join("<lib>.wasm")` at `:372–376`, so it must be updated in lockstep with the shared `CARGO_TARGET_DIR`.)
- **Consumers:** 18 files under `crates/slicer-runtime/tests/` reference the old `test-guests/` location, in **four** path-construction forms (do not assume a single find/replace covers them — it covers only the 13 Form-1 files):
  - **Form 1 (13 files):** `concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-guests/<g>.component.wasm")`. `CARGO_MANIFEST_DIR` stays `crates/slicer-runtime` and the new tree sits directly under it, so **all** `..` segments drop: the literal becomes `/test-guests/<g>.component.wasm`. (A `/../test-guests/` replacement resolves to the nonexistent `crates/test-guests/`; AC-N1's static guard is blind to the single-`..` form — only the runtime `fs::read` tests catch it.)
  - **Form 2 (3 files — `guest_fixture_freshness_tdd.rs`, `macro_all_worlds_roundtrip_tdd.rs`, `macro_finalization_deep_copy_tdd.rs`):** multiline `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("test-guests")`. Drop one `.join("..")`. No `../../test-guests/` literal → escapes a literal sweep and the old single-line AC-N1 grep.
  - **Form 3 (1 file — `live_layer_support_tdd.rs:869`):** climbs to repo root via a `.parent()` chain, then `.join("test-guests/layer-infill-guest.component.wasm")`. Re-base onto `crates/slicer-runtime/`.
  - **Form 4 (1 file — `wit_drift_detection_tdd.rs:629` `test_guest_lib_rs_content`):** `workspace_root().join(format!("test-guests/{guest}/src/lib.rs"))`. Deleted with the obsolete drift sub-test in Step 4 (sole caller `:464`); repoint only if retained.
- **Raw guests (A):** `prepass-guest`, `layer-infill-guest`, `finalization-guest`, `postpass-guest` use `wit_bindgen::generate!({ inline: r#"…"# , world: "<world>-module" })`. Replace `inline:` with `path: "../../../slicer-schema/wit"` (relative to each guest manifest at the new depth) keeping the existing `world:`.
- **Drift test (A):** `crates/slicer-runtime/tests/wit_drift_detection_tdd.rs::handwritten_test_guests_use_payload_extrusion_role_variants` (lines ~436–486) greps guest `src/lib.rs` for inline package strings; it becomes meaningless once inline WIT is gone — delete it. Keep `macro_uses_canonical_dep_includes` (it checks the macro/host, not the guests).
- **Witness codec (C):** new `crates/slicer-runtime/test-guests/witness/` plain lib (dep: `slicer-ir`). Producers: `sdk-layer-infill-guest`, `sdk-finalization-guest`. Consumers: `dispatch_tdd.rs`, `finalization_world_deep_copy_tdd.rs`, `macro_all_worlds_roundtrip_tdd.rs`, `wit_boundary_tdd.rs`, `macro_finalization_deep_copy_tdd.rs`.

## Neighboring Tests / Fixtures

- `crates/slicer-runtime/tests/guest_fixture_freshness_tdd.rs` enumerates guest names + paths — must track the new location.
- `crates/slicer-runtime/tests/wit_boundary_tdd.rs` and `dispatch_tdd.rs` exercise the raw guests at the host WIT boundary (AC-N2 / AC-6 anchors).

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

## Selected Approach

Four sequenced, independently-verifiable steps in one packet: **(2) relocate + repoint, (3) D1 shared target dir, (4) A de-dup, (5) C witness codec** (Step 1 is the orphan deletion). Relocation precedes A and C because A's `path:` literal and C's path deps are written against the final location; D1 follows relocation because it edits the same `build_one`/builder surface.

**Rejected alternatives:** (a) D2 true single workspace — reverses packet 70's deliberate per-guest-workspace builder and forces a `discover_guests` validation rework; rejected for cost/risk. (b) Two packets (infra vs witness) — rejected per the user's "as few packets as possible"; C is kept as the final, clearly-bounded step. (c) Deleting the raw guests ("B") — rejected; they are the differential oracle.

## Explicit Code Change Surface

Primary (≤3 logical surfaces):
1. `xtask/src/build_guests.rs` — `tg_root`, artifact prefix, `build_one` `CARGO_TARGET_DIR` + intermediate path.
2. The relocated guest crates — manifests (SDK path deps) + the four raw guests' `wit_bindgen::generate!` blocks.
3. `crates/slicer-runtime/tests/*` — 18 path references across 4 construction forms (13 literal + 3 multiline `.join` + 1 `.parent()`-chain + 1 `format!`) + drift sub-test deletion + 5 witness-decoder migrations; plus new `crates/slicer-runtime/test-guests/witness/`.

Supporting: `.gitignore`, `CLAUDE.md`, `docs/05_module_sdk.md`, two `skills/**/wasm-staleness.md`.

## Read-Only Context

- `xtask/src/build_guests.rs:88–259` (discover) and `:345–413` (build_one; intermediate-path computation at `:372–376`).
- `crates/slicer-runtime/src/wit_host.rs:241` / `:314` / `:488` / `:509` — confirm the host `bindgen!` `path: "../slicer-schema/wit"` form A mirrors (do not edit).
- One raw guest (`prepass-guest/src/lib.rs`) and one SDK guest (`sdk-layer-infill-guest/src/lib.rs`) as templates.

## Out-of-Bounds Files

- `crates/slicer-schema/wit/**` (canonical contract — read-only).
- `modules/core-modules/*/wit-guest/**` (core-module guests — not moved).
- Any `target/` tree, generated bindgen output, lockfiles.
- `.ralph/specs/**` historical packets (archival; their `test-guests/` mentions are not edited).

## Expected Sub-Agent Dispatches

- "List every `crates/slicer-runtime/tests/*.rs` line matching `../../test-guests/` and confirm zero remain after the sweep." → `LOCATIONS` (≤20) / `FACT` count.
- "Summarize how `build_one` computes the intermediate `.wasm` path fed to `wasm-tools component new`." → `SNIPPETS` (≤1, ≤30 lines, with file:line).
- "Run `cargo test -p slicer-runtime --test <file>` for the named guest-consuming tests; return pass/fail + first failing assertion." → `FACT`.
- "SUMMARY of `.ralph/specs/70_workspace-aware-guest-builder/design.md`: does the builder rely on each guest having its own `target/`?" → `SUMMARY` (≤200 words).

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

## Context Cost Estimate

- Aggregate: **M**. Largest single step: Step 5 (C) — new crate + 2 producers + 5 consumers (**M**). Highest-risk dispatch: the `build_one` intermediate-path summary (Step 4/D1).

## Open Questions

- `[FWD]` Exact shared-target layout for D1 (`CARGO_TARGET_DIR=crates/slicer-runtime/test-guests/target` vs a sibling). Resolve during Step 3; either satisfies AC-2 — pick the one that keeps `build_one`'s intermediate-path math simplest.
- ~~`[FWD]` Whether `test_guest_lib_rs_content` (helper in `wit_drift_detection_tdd.rs`) has other callers after deleting the handwritten sub-test; remove only if unused.~~ **Resolved:** the helper is defined at `:628` and has exactly one caller, `:464`, inside `handwritten_test_guests_use_payload_extrusion_role_variants` (the sub-test being deleted). After Step 4 it is unused → delete the helper too. (This also retires the only Form-4 `format!("test-guests/…")` reference.)
- None `[BLOCK]`.
