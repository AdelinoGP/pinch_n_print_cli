---
status: implemented
packet: 162_wit-lifecycle-export-removal
task_ids:
  - TASK-146a
---

# 162_wit-lifecycle-export-removal

## Goal

Delete the `on-print-start` / `on-print-end` WIT exports and every artifact that mirrors them — SDK trait methods, macro glue and shims, `WORLD_LIFECYCLE_EXPORTS` and its self-referential guard test, the `pnp_cli module new` scaffold — renaming the surviving per-call constructor to `from_config`, and make a stale `pnp_cli` binary fail loudly instead of silently producing a false baseline.

## Problem Statement

`on-print-start` / `on-print-end` are padding squatting on a real concept's name. Grounded against the tree:

- `call_on_print_start` / `call_on_print_end` have **zero callers in the host** — the exports are declared, generated, shipped in all 20 guest `.wasm` artifacts, and never invoked.
- The macro's `on_print_end` glue is hardcoded `fn on_print_end() -> Result<(), ModuleError> { Ok(()) }` (`crates/slicer-macros/src/lib.rs:2771`) and never dispatches to the trait. Every module's `on_print_end` body is unreachable; only `arachne-perimeters` (`modules/core-modules/arachne-perimeters/src/lib.rs:419`) even has one.
- The macro's `on_print_start` glue does `Ok(_m) => Ok(())` (`:2766-2769`) — it constructs the module and discards it. There is no `OnceCell`/`OnceLock`/`static mut`/`thread_local` anywhere in the macro, and all 15 `run_*` arms reconstruct the module per call. `docs/05_module_sdk.md:184`'s "initialize expensive resources once per print" is therefore inverted: it runs once per **layer**, per **stage**.
- `crates/slicer-schema/src/lib.rs:230` `WORLD_LIFECYCLE_EXPORTS` claims all four worlds ship lifecycle exports. Only `world-layer.wit:20-21` declares them; prepass/postpass/finalization declare none. Its guard test `every_world_has_lifecycle_exports` (`:428`) reads that table and asserts against the *same* table — it passes vacuously. This is the identical pathology ADR-0044 documented for `wit_world_major_version_mismatch_rejects_future_major`.
- Consequently the macro's `lifecycle_shim_tokens` emits fake `#[export_name = "on-print-start"]` shims for worlds whose `.wit` declares no such export — and the macro's own comment above `skip_lifecycle_shims` already admits "the world declares none (postpass/prepass/finalization)".
- `docs/04_host_scheduler.md:1449` ("call on-print-start on all modules") describes a step the host has never performed.
- `docs/03_wit_and_manifest.md:559` labels the pair `// Lifecycle — optional`. **The component model has no optional exports** — wasmtime's generated `Indices::new` eagerly resolves every export at `instantiate`, which is ADR-0045's central premise and the entire reason the per-stage split exists. This listing is where the "optional export" fiction was first written down, in the initial commit, before a host existed to contradict it. Every downstream artifact in this packet is a faithful implementation of a sentence that was never true.

Folded in (same blast radius, same tests): `pnp_cli` is a separate package, so `cargo test -p slicer-runtime` never rebuilds it, and **three** independent copies of the lookup probe the filesystem for whatever artifact happens to be on disk — `slicer_cache.rs::pnp_cli_bin` (`:112-146`, prefers a stale `target/release` over a fresh `target/debug`), `benches/gate_evidence.rs:48-74` (same release-then-debug fallback at the `for profile in ["release","debug"]` loop), and `slicer-scheduler`'s `dag_cli_integration.rs::bin` (`:15-31`, debug-then-release). None checks mtime.

**This trap has now burned two consecutive sessions, and the second nearly committed a wrong golden.** The first was this ADR's own prior session, which recorded a false baseline. The second was the parallel `object_id` session: its first `BLESS_GOLDEN=1` run blessed `9dda3c89` — **neither the old id nor the correct one** — because the e2e test spawned a `pnp_cli` that `cargo test` never rebuilt, so it blessed the *old code's* output into a golden file. It was caught only because that session checked the uuid against its derivation instead of trusting a green test. The correct value, after `cargo build --bin pnp_cli`, is `da3bd96b` = `uuid5(NS, "20mmbox-LF.stl#0")`.

That is the justification for AC-8/AC-9/AC-N2, and it is not hypothetical: a silent stale spawn does not merely produce a red test that someone investigates — it produces a **green** test whose output is wrong, and a `--bless` flow will happily write that wrong output to disk as the new truth. This packet's own blast radius is measured by exactly these tests, so the gate must be trustworthy before the measurement is believed.

**A note on this packet's own ACs, earned the hard way.** The first draft of this packet shipped `AC-N1` and `AC-N2` as bare `cargo test -p <crate> --test <bin> <name> | rg '^test result'`. Against a tree where the test does not yet exist, libtest filters to nothing, prints `ok. 0 passed; 0 failed; … 196 filtered out`, and exits **0** — so the acceptance criterion **passed by doing nothing**. That is precisely the defect this packet exists to delete: `every_world_has_lifecycle_exports` asserts a table against itself and passes vacuously; a `0 passed` AC asserts nothing against nothing and passes vacuously. Writing one *inside the packet that deletes the other* is not irony, it is the same reflex — a green check mistaken for evidence. Every `cargo test <name>` gate here therefore carries a `| rg -v '0 passed'` guard. **The next author will reach for the unguarded form; this paragraph exists to stop them.**

TASK-146 (`docs/07_implementation_status.md:39`) is **reopened as TASK-146a**: it closed by adding `validate_wit_world`, which ADR-0044 showed compares one hand-written string to another with no artifact to check against, and which ADR-0045 retires outright. Sub-lettering follows the existing `TASK-119a/b/c`, `TASK-120a-d`, `TASK-194a/b` convention. This packet lands **before** the per-stage split so the world-layer export surface shrinks 10 → 8 first, giving packets #2/#3 a smaller, honest surface to split.

No packet is superseded by this one.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- This packet's change surface hits **five** of the guest-invalidating input classes simultaneously — `crates/slicer-schema/wit/**/*.wit`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**`, and `modules/core-modules/*/src/**` — so **every** guest and test guest is invalidated. There is no partial rebuild; `AC-6` is unmeasurable until `cargo xtask build-guests` (no `--check`) completes for all 20 core modules and the test guests.
- Config key naming is untouched: this packet adds no config key and renames no manifest key. `[stage] id` stays singular in all 20 manifests.
- The coordinate-system constraint does not apply: this packet contains no geometry and no mm/unit conversion. The `coord-system` snippet is deliberately absent.

## Data and Contract Notes

- **IR/manifest contracts:** unchanged. No IR type, no config key, no manifest key. `[stage] id` stays singular in all 20 manifests; `wit-world` stays until packet #3.
- **WIT boundary:** `world layer-module` in `slicer:world-layer@2.0.0` loses 2 of 10 exports. Per `CLAUDE.md` §"WIT/Type Changes Checklist": no type identity changes, so no cross-file type-identity audit is needed; the deleted funcs have no host-side callers (`call_on_print_start` / `call_on_print_end` — **zero callers**, verified), so `wit_host.rs` / `dispatch.rs` need no edit. The package **version is not bumped** — `world-layer@2.0.0` stays; ADR-0044 established the version is not an identity token and packet #2/#3 reset versioning wholesale. `no_versioned_world_identifiers_outside_canonical_wit` therefore stays green untouched.
- **Determinism/scheduler constraints:** none. The deleted exports were never dispatched, so no ordering, claim, or plan-freeze behavior changes. G-code output must be byte-identical — the **green** parity set (AC-N3) is the check, and it must stay green.
- **Guest artifacts:** all 20 `modules/core-modules/*/*.wasm` and all test guests are regenerated. Their decoded `world root` loses the two exports; this is the only observable runtime change in the packet.

## Locked Assumptions and Invariants

- **Locked:** no module may retain private state across stage calls. This was already true (every `run_*` arm reconstructs via the constructor; no `OnceCell`/`OnceLock`/`static mut`/`thread_local` exists in the macro) — the packet removes the *name* that falsely implied otherwise, not a capability. Re-introducing cross-call state requires a new contract and a new ADR, not a revert.
- **Locked:** `from_config` is a per-call constructor, not an initializer. Its doc must say so.
- **Not locked:** `ExportKind` / `ExportBinding` shape — packet #2 is expected to restructure both.
- **Not locked:** `world-layer@2.0.0`'s version string — packets #2/#3 reset it.

## Risks and Tradeoffs

- **The parity set is GREEN and COMMITTED, and staying green is this packet's behavior-neutrality proof.** The parallel `object_id` session **landed at `ff21378e`**: `object_id` is now `basename + index`, so the baseline reproduces from HEAD rather than from one working tree. Verified on a clean tree at `b7f17f75` (0 stale guests): `perimeter_parity` → `12 passed; 0 failed; 11 ignored` and `legacy_zero_matches_golden` → `1 passed; 0 failed`. **Any earlier guidance in this packet's lineage calling these "known-red, red-before/red-after" is obsolete — do not follow it.** They must be green before Step 2 and green after Step 11; a regression is **caused by this packet** and is a gate failure. Two sub-points an implementer needs: (a) the set is **8** tests, not 7 — `deliberate_broken_fixture_file_is_detected` (`crates/slicer-runtime/tests/integration/perimeter_parity.rs:705`) was masked because `compare_perimeter_ir` stops at the first mismatch and `object_id` mismatched first; it is the harness's own negative control, so a failure there means the harness stopped detecting corruption. (b) `perimeter_parity` lives **only** at `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — a submodule of the `integration` binary, not a top-level test file, so `--test integration -- perimeter_parity` is the only correct invocation. Its `object_id` soft-ignore is **gone**: `:467` now compares strictly and records a mismatch naming `regions[{region_idx}].object_id`. It spawns **no** binary (in-process `load_model`), so the CLI-freshness change cannot affect it in either direction — but `legacy_zero_matches_golden` **does** spawn, and is exactly the test the stale-binary trap corrupted.
- **The stale-binary trap has a demonstrated worst case, and it is not a red test.** The `object_id` session's first `BLESS_GOLDEN=1` run blessed `9dda3c89` — neither the old id nor the correct `da3bd96b` (`uuid5(NS, "20mmbox-LF.stl#0")`) — because the e2e test spawned a `pnp_cli` that `cargo test` never rebuilt, so it wrote the *old code's* output into a golden file as the new truth. It was caught only by checking the uuid against its derivation rather than trusting a green test. Two consecutive sessions were burned this way. This is the risk AC-8/AC-9/AC-N2 retire, and it is why the gate must be loud rather than best-effort: a silent stale spawn produces green tests and wrong artifacts.
- **The sweep is large and the compiler is the only complete check.** 535 occurrences / 110 files. Mitigation: `cargo check --workspace --all-targets` (not plain `cargo check`, which skips test targets) plus `AC-N1`'s walking guard, which catches re-introduction the compiler would happily accept.
- **The freshness gate could become a nuisance.** Every `cargo test -p slicer-runtime` after a `crates/*/src/**` edit will now panic until `cargo build --workspace` runs. This is the intended loudness — the prior behavior silently spawned a stale binary — and the scope exclusion of `tests/`, `benches/`, and `modules/` keeps it from firing on edits that cannot affect the binary. A gate that fires spuriously gets disabled; a gate that never fires is what produced the false baseline.
- **All three stale-binary traps are closed by this packet** (`slicer_cache.rs`, `benches/gate_evidence.rs`, `slicer-scheduler`'s `dag_cli_integration.rs`); only the shared-helper *extraction* is deferred, and it needs its own ADR. The residual risk is the triplication itself: three copies can drift. Accepted deliberately — see `[FWD]`.
- **`docs/03:559-561` is deleted by this packet**, not deferred. The residual risk is a merge conflict with packet #3's listing restructure, which is nil: #3 rewrites the listing's shape, this deletes a stanza that will not exist in any shape.
- **Mid-sweep the tree does not compile** (Steps 3-7). Mitigation: the steps are ordered so the window is short and the exits are grep-based, not build-based; `requirements.md` §"Step Completion Expectations" states this explicitly so no implementer "repairs" it by re-adding the hook.
