# Requirements — Packet 74

## Problem Statement

The `test-guests/` tree is the host integration-test fixture set: 12 WASM-component guests consumed exclusively by `crates/slicer-runtime/tests/*`. Three frictions have accumulated, all hurting agent navigability and contradicting decisions already made in this repo:

1. **The fixtures sit two levels away from their only consumer.** Every test loads `../../test-guests/<g>.component.wasm`. An agent reading `slicer-runtime/tests/` cannot see the fixtures beside the code that uses them.
2. **Each guest is its own workspace, minting its own `target/`.** Twelve independent `target/` trees bloat disk and slow every filesystem scan — `.gitignore` already lists `test-guests/*/target/` to cope.
3. **Four "raw" guests still inline a verbatim copy of the WIT world.** Packet 72 unified host + macro onto the canonical `crates/slicer-schema/wit/` single source, but `prepass-guest`, `layer-infill-guest`, `finalization-guest`, and `postpass-guest` still paste the contract into `wit_bindgen::generate!({ inline: … })`. The copies are policed by a drift sub-test instead of being made structurally impossible.

A fourth, softer friction: the guest↔host-test signal is smuggled through positional `Point3WithWidth` fields (`point[0].x = region_count`, …) whose meaning lives only in comments on both sides, re-derived in ~5 test files.

This packet co-locates the fixtures with their consumer, collapses the build sprawl, removes the inline-WIT duplication, and gives the witness encoding one owning module — without touching runtime behavior. It continues packet 72 (de-dup) and packet 70 (guest builder), and explicitly **does not** delete the raw guests, which remain the only differential check that the `#[slicer_module]` macro's emitted glue matches hand-rolled `wit-bindgen`.

## Task IDs

- **TASK-215** — Test-guest co-location, target-dir consolidation, WIT de-duplication, and witness-codec extraction (this packet). Continues TASK-214 (packet 70 guest builder) and TASK-144/145 (packet 72 WIT single-source).

## In Scope

- `git mv test-guests/ crates/slicer-runtime/test-guests/` (12 guest crates). Remove the stray empty `sdk-layer-plan-guest/` directory (no `Cargo.toml`; already un-built).
- SDK guest manifests: repoint path deps from `../../crates/slicer-{sdk,ir,schema}` to `../../../slicer-{sdk,ir,schema}` (one level deeper).
- `xtask/src/build_guests.rs`: change `tg_root` (`:175`) to `crates/slicer-runtime/test-guests`; change the artifact-path prefix (`:242`) to `crates/slicer-runtime/test-guests/{dir}.component.wasm`; set a single shared `CARGO_TARGET_DIR` for guest `cargo build` invocations in `build_one` (D1) and update the intermediate-`.wasm` path it feeds to `wasm-tools component new`.
- The 18 `slicer-runtime` test files referencing the old `test-guests/` location: repoint to the new location. **These split across four path-construction forms — a single find/replace of one literal will miss 5 of them:**
  - **Form 1 — string-concat literal (13 files):** `concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-guests/<g>.component.wasm")` → replace `/../../test-guests/` with `/test-guests/`. (`CARGO_MANIFEST_DIR` is `crates/slicer-runtime`; the new tree is directly under it, so **all** `..` segments are removed — a `/../test-guests/` replacement would resolve to the nonexistent `crates/test-guests/`. AC-N1's static guard does **not** catch this single-`..` mistake; only the runtime `fs::read` tests do.)
  - **Form 2 — multiline chained `.join` (3 files: `guest_fixture_freshness_tdd.rs` `test_guests_dir()`, `macro_all_worlds_roundtrip_tdd.rs` `guest_component_path()`, `macro_finalization_deep_copy_tdd.rs`):** `…CARGO_MANIFEST_DIR … .join("..").join("..").join("test-guests")` → drop **one** `.join("..")` so it lands at `crates/slicer-runtime/test-guests`. These carry **no** `../../test-guests/` literal and escape a literal find/replace.
  - **Form 3 — workspace-root-relative single join (1 file: `live_layer_support_tdd.rs:869`):** climbs to repo root via `.parent()` then `.join("test-guests/layer-infill-guest.component.wasm")`; repoint so the base resolves to `crates/slicer-runtime/` (e.g. one fewer `.parent()`, or `.join("crates/slicer-runtime/test-guests/…")`).
  - **Form 4 — `format!` helper (1 file: `wit_drift_detection_tdd.rs:629` `test_guest_lib_rs_content`):** `workspace_root().join(format!("test-guests/{guest}/src/lib.rs"))`. This helper is deleted with the obsolete drift sub-test in Step 4 (its only caller is line 464); if it is instead retained, repoint it to `crates/slicer-runtime/test-guests/`.
- `.gitignore`: replace `test-guests/*/target/` with the single shared guest target dir under the new location.
- `CLAUDE.md` path references (Guest WASM Staleness, Post-Merge naming, WIT/Type Changes checklist) and the two `skills/**/wasm-staleness.md` snippet files.
- **A (de-dup):** in the four raw guests, replace `inline: r#"…"#` with `path: "../../../slicer-schema/wit"` (+ the existing `world:`); reconcile any divergence toward canonical. Delete the obsolete `wit_drift_detection_tdd::handwritten_test_guests_use_payload_extrusion_role_variants` sub-test (and its `test_guest_lib_rs_content` helper if then unused); keep `macro_uses_canonical_dep_includes`.
- **C (witness codec):** add a plain lib crate `crates/slicer-runtime/test-guests/witness/` (depends on `slicer-ir`) defining named structs + `encode`/`decode` for the infill, support, and finalization witness layouts; migrate `sdk-layer-infill-guest` and `sdk-finalization-guest` producers and the host decoders in `dispatch_tdd.rs`, `finalization_world_deep_copy_tdd.rs`, `macro_all_worlds_roundtrip_tdd.rs`, `wit_boundary_tdd.rs`, `macro_finalization_deep_copy_tdd.rs`; add `witness` as a `slicer-runtime` dev-dependency.

## Out of Scope

- **Deleting the raw guests** (`prepass/layer-infill/finalization/postpass-guest`). They are the macro-vs-hand-rolled differential oracle; only their inline WIT text is removed. (Rejected candidate "B".)
- **D2 / true single workspace.** No removal of per-guest `[workspace]` sentinels and no rework of `discover_guests`'s sentinel validation. D1 (shared `CARGO_TARGET_DIR`) is the chosen variant; it preserves the packet-70 design.
- Any change to production crates' behavior, to `crates/slicer-schema/wit/` canonical contract, or to core-module guests under `modules/core-modules/*/wit-guest/`.
- Migrating the raw guests to the witness codec (C is scoped to SDK guests + host decoders first).
- OrcaSlicer parity work.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` (~large — read only the host-boundary / single-source section by range).
- `docs/05_module_sdk.md` (guest build-flow section only).
- `.ralph/specs/70_workspace-aware-guest-builder/` and `.ralph/specs/72_wit-single-source-unification/` — inspect via SUMMARY dispatch; do not read all five files of each.

## OrcaSlicer Reference Obligations

None — no parity surface in this packet.

## Acceptance Summary

The packet is accepted when AC-1..AC-7 and AC-N1..AC-N2 (defined in `packet.spec.md`) all pass. Measurable refinements not captured in the Given/When/Then:

- **Guest count is 12** after removing the empty `sdk-layer-plan-guest/` (AC-1). Pre-move count of buildable guests is also 12 (the empty dir was never built).
- **D1, not D2** (AC-2): per-guest `[workspace]` sentinels remain present and unmodified; `discover_guests` validation is unchanged. The single `target/` is achieved purely via `CARGO_TARGET_DIR`.
- **No inline WIT remains** in any of the four raw guests, and each binds canonical (AC-3); the inline-policing sub-test is deleted, not merely skipped (AC-4).
- **Oracle intact** (AC-5): four raw guests still present — guards against silently performing rejected candidate B.
- **Witness encoding centralized** (AC-6): field meanings defined once in `witness`; producer (SDK guests) and consumer (host tests) both reference it.
- **All four path forms repointed** (AC-N1): the 18 referencing files use four distinct constructions (literal concat, multiline chained `.join`, workspace-root single join, `format!`). The regression guard is multiline-aware (`rg -U`) so it cannot false-green on the 5 files that escape a literal find/replace.

## Verification Commands

| ID | Command | Delegation hint |
|----|---------|-----------------|
| AC-1 | `cargo xtask build-guests --list 2>/dev/null \| grep -c "crates/slicer-runtime/test-guests/.*\.component\.wasm"` | FACT: integer == 12 |
| AC-2 | `cargo xtask build-guests >/dev/null 2>&1; find crates/slicer-runtime/test-guests -maxdepth 2 -type d -name target \| wc -l` | FACT: integer == 1 |
| AC-3 | `cd crates/slicer-runtime/test-guests; test $(grep -l "inline:" {prepass,layer-infill,finalization,postpass}-guest/src/lib.rs \| wc -l) -eq 0 && test $(grep -l "path:.*slicer-schema/wit" {prepass,layer-infill,finalization,postpass}-guest/src/lib.rs \| wc -l) -eq 4; echo $?` | FACT: exit 0 |
| AC-4 | `grep -c "fn handwritten_test_guests_use_payload_extrusion_role_variants" crates/slicer-runtime/tests/wit_drift_detection_tdd.rs` (==0) then `cargo test -p slicer-runtime --test wit_drift_detection_tdd` | FACT: 0 + pass/fail |
| AC-5 | `ls crates/slicer-runtime/test-guests/{prepass,layer-infill,finalization,postpass}-guest/src/lib.rs 2>/dev/null \| wc -l` | FACT: integer == 4 |
| AC-6 | `test -f crates/slicer-runtime/test-guests/witness/src/lib.rs && grep -lq "witness::" crates/slicer-runtime/tests/dispatch_tdd.rs; echo $?` then `cargo test -p slicer-runtime --test dispatch_tdd` | FACT: exit 0 + pass/fail |
| AC-7 | `cargo test -p slicer-runtime --test macro_all_worlds_roundtrip_tdd --test finalization_live_tdd` | FACT: pass/fail |
| AC-N1 | `rg -Ul -e '\.\./\.\./test-guests' -e '\.join\("\.\."\)\s*\.join\("\.\."\)\s*\.join\("test-guests"\)' -e 'join\("test-guests/' -e 'format!\("test-guests/' crates/slicer-runtime/tests \| wc -l` | FACT: integer == 0 (multiline 4-pattern guard covering all 4 forms; the old single-`grep` literal guard missed 5 files) |
| AC-N2 | `cargo test -p slicer-runtime --test wit_boundary_tdd` | FACT: pass/fail |
| Gate | `cargo clippy --workspace --all-targets -- -D warnings` | FACT: exit code |

No AC uses `cargo test --workspace`. The workspace-wide suite runs only at the acceptance ceremony (implementation-plan.md), dispatched to a sub-agent returning `FACT pass/fail`.

## Step Completion Expectations (cross-step invariants)

- After **every** step that touches a guest source, a guest `Cargo.toml`, or `crates/slicer-schema/wit/**`, run `cargo xtask build-guests --check` and rebuild if `STALE:` before running any guest-consuming test — a stale artifact yields failures unrelated to the edit.
- The relocation step (Step 2) must leave the tree building green **before** A or C begin; A's `path:` literal and C's path deps are written against the final location only.

## Context Discipline Notes (packet-specific)

- The 18 consuming test files are large; avoid opening them wholesale. **But a single find/replace of `../../test-guests/` → `../test-guests/` is insufficient — it fixes only the 13 Form-1 files.** Handle the 5 non-literal files (Forms 2–4 above) by name with the `±40`-line window around their path-helper (`test_guests_dir`, `guest_component_path`, the `live_layer_support_tdd.rs:869` `.parent()` chain, and — if retained — the `wit_drift` `format!` helper). Verify completeness with AC-N1's **multiline (`rg -U`)** guard, not the old single-line literal grep, which is blind to Forms 2–4.
- `xtask/src/build_guests.rs` is the one file where build logic changes; read the `discover_guests` (lines ~88–259) and `build_one` (lines ~345–413) windows only.
