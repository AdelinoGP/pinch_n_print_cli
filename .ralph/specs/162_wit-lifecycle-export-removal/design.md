# Design: 162_wit-lifecycle-export-removal

## Controlling Code Paths

- Primary code path: `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` (the declaration) → `crates/slicer-schema/src/lib.rs` (the mirror table + typed export metadata) → `crates/slicer-macros/src/lib.rs` (the `WORLD_LIFECYCLE` metadata consumer and the independent, hardcoded `impl Guest for __SlicerLayerComponent` glue) → `crates/slicer-sdk/src/traits.rs` (the trait the glue never calls for `on_print_end` and discards for `on_print_start`).
- Secondary code path: `crates/slicer-runtime/tests/common/slicer_cache.rs::pnp_cli_bin` → `run_pnp_cli_uncached` → every cached e2e/integration spawn. Gate mirror: `xtask/src/test.rs::run` Step 1 → `xtask/src/build_guests.rs::{check_command, is_stale, compute_shared_mtime}`.
- Neighboring tests/fixtures: `crates/slicer-macros/tests/binding_surface_tdd.rs` (fixtures `LayerInfillFixture`, `LayerLifecycleOnly`, `PrepassMeshAnalysisFixture`, `PrepassLayerPlanningFixture`, `FinalizationFixture`, `PostpassGcodeFixture`, `PostpassTextFixture`); `crates/slicer-macros/tests/all_worlds_glue_tdd.rs` (source-text assertions over the macro); `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` (`workspace_root`, `collect_files`, `regex_lite_versioned_world`, and the guard `no_versioned_world_identifiers_outside_canonical_wit` this packet's `AC-N1` mirrors); the 20 `modules/core-modules/*/tests/slicer_module_binding_tdd.rs` files, each asserting `exports.contains(&"on-print-start")`.
- OrcaSlicer comparison: **none — the `orca-delegation` snippet deliberately does not apply.** No implementation decision in this packet requires consulting OrcaSlicer. The parity rationale is already settled and recorded in ADR-0045 §"The lifecycle finding": OrcaSlicer has no such hook (zero hits in `libslic3r`); it expresses lifetime by *where the object lives* (`SeamPlacer::init` once per print on a `GCode` member; `Fill` via `Layer::make_fills` and `PerimeterGenerator` via `LayerRegion::make_perimeters` rebuilt per layer), and our tier system already encodes both (per-print = prepass + Blackboard, ADR-0029; per-layer = layer tier). Re-deriving that finding would be re-litigating an accepted ADR, and this packet borrows no canonical behavior, constant, or edge case. Adding the snippet would obligate implementers to reads that cannot change any decision here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- This packet's change surface hits **five** of the guest-invalidating input classes simultaneously — `crates/slicer-schema/wit/**/*.wit`, `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**`, and `modules/core-modules/*/src/**` — so **every** guest and test guest is invalidated. There is no partial rebuild; `AC-6` is unmeasurable until `cargo xtask build-guests` (no `--check`) completes for all 20 core modules and the test guests.
- Config key naming is untouched: this packet adds no config key and renames no manifest key. `[stage] id` stays singular in all 20 manifests.
- The coordinate-system constraint does not apply: this packet contains no geometry and no mm/unit conversion. The `coord-system` snippet is deliberately absent.

## Code Change Surface

### Selected approach

Delete the contract at its root (the `.wit`) and follow the compiler outward. The Rust type system is the sweep's completeness proof: 535 occurrences across 110 files cannot be verified by reading, but `cargo check --workspace --all-targets` cannot be satisfied while one survives. Two hand-written guards (`AC-N1`, `AC-N2`) then cover what the compiler cannot see — re-introduction, and the WASM export table.

### Exact functions, traits, manifests, tests, and fixtures

**WIT** — `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`
- delete `:20` `export on-print-start: func(config: config-view) -> result<_, module-error>;`
- delete `:21` `export on-print-end:   func() -> result<_, module-error>;`
- `world layer-module` drops from 10 exports to 8. No other `.wit` in `crates/slicer-schema/wit/` matches `print-start|print-end` (verified).

**Schema** — `crates/slicer-schema/src/lib.rs` (locate each by name; do not navigate by the coordinates below)
- `enum ExportKind` — delete the `Lifecycle` variant and its doc; keep the enum and its `Stage` variant (see §Rejected alternatives).
- `SlicerModuleSchema.exports` doc — "world lifecycle exports (in lifecycle order) followed by the detected stage export if any" → "the detected stage export, if any (0 or 1 entries)".
- delete the `WORLD_LIFECYCLE_EXPORTS` const (and its `/// WIT worlds and the unconditional lifecycle exports every world ships` doc).
- delete the `lifecycle_exports_for_world` fn.
- `SUPPORTED_WIT_WORLDS` doc — replace "Mirrors the world column of [`WORLD_LIFECYCLE_EXPORTS`]" (a rustdoc intra-doc link that dangles once the table dies → warning under `-D warnings`) with a self-standing sentence. **The const itself stays; it retires in packet #3.**
- delete the test `every_world_has_lifecycle_exports` (in the `#[cfg(test)] mod tests` block).

**SDK** — `crates/slicer-sdk/src/traits.rs` (1700 lines — locate each trait by name, then range-read; never read whole)
- `LayerModule`: trait-level doc + `on_print_start` → `from_config`; delete the defaulted `on_print_end`. Its doc bullets quoting `export on-print-start:` / `export on-print-end:` verbatim go too.
- `PrepassModule`, `PostpassModule`, `FinalizationModule`: same three edits each (doc, rename, delete).
- All four `from_config` signatures must read exactly `fn from_config(config: &ConfigView) -> Result<Self, ModuleError>;` — AC-2 counts that literal string and expects 4.

**Macro** — `crates/slicer-macros/src/lib.rs` (2800+ lines — **locate every item below by name/construct, never by line number**; see §Context Discipline Notes)
- drop `WORLD_LIFECYCLE_EXPORTS as WORLD_LIFECYCLE` from the top-of-file `use slicer_schema::{...}` list.
- delete the `let lifecycle_exports: &[&str] = WORLD_LIFECYCLE.iter()…` lookup and its `// Build the WIT-export list…` comment; `wit_exports` becomes `stage_export_literal` alone (empty when stageless).
- Delete the `let lifecycle_count = …` binding and the **whole `let lifecycle_binding_tokens = lifecycle_exports.iter().map(|e| { … });` closure**, together with its preceding `// Typed structured export bindings…` comment block. **Delete through the closure's terminating `});` — the statement does not end at the last `quote!` brace.** Keep `stage_binding_tokens` but drop its now-meaningless leading `,` inside the `quote!`.
- In the `__slicer_module_schema` / `SLICER_MODULE_SCHEMA` const body, drop `#( #lifecycle_binding_tokens ),*` from the `exports:` array literal.
- Reword the `__slicer_wit_exports` doc and **delete the `__SLICER_LIFECYCLE_EXPORT_COUNT` const** and its doc block (it would be a constant `0`; its only consumers are `binding_surface_tdd.rs::typed_schema_lifecycle_export_count_matches_world_lifecycle_table`).
- Delete the `let lifecycle_shim_tokens: Vec<TokenStream2> = lifecycle_exports.iter().map(|export| { … }).collect();` statement (the one emitting `#[export_name = #export] pub extern "C" fn … -> i32 { 0 }`) and the lifecycle half of the preceding shim comment.
- Fix the `resolve_world_glue` comment claiming "layer (all 8 stage exports + 2 lifecycle exports)".
- Delete the `let skip_lifecycle_shims = real_glue_world.is_some();` and `let active_lifecycle_shims = …` bindings; in the `wasm_export_shims` `quote!`, the shim mod emits `#stage_shim_tokens` only (drop `#( #active_lifecycle_shims )*`).
- `:2763-2771` delete `fn on_print_start` (`:2764`) and `fn on_print_end` (`:2771`) from `impl Guest for __SlicerLayerComponent`. **Verified: the layer world is the only one whose `impl Guest` declares them** — the other three glue blocks (`:862` Postpass, `:1029` Finalization, `:1642` Prepass) declare neither, consistent with their `.wit` files declaring no lifecycle export.
- 15 construction sites → `::from_config(&ir_config)`: `:632`, `:656` (Postpass), `:1036` (Finalization), `:1334`, `:1411`, `:1472`, `:1543` (Prepass), `:1713`, `:1734`, `:1755`, `:1775`, `:1795`, `:1828`, `:1890`, `:1910` (Layer).
- **Count discipline for this file:** `rg -c '::on_print_start\(&ir_config\)'` returns **16** today, not 15. The 16th is `:2766`, inside the `impl Guest` block being deleted above — it is *removed*, not renamed. Post-edit the file must hold exactly 15 `::from_config(&ir_config)` and 0 `on_print_*`. An implementer who renames all 16 has left the dead glue block alive.

**Macro tests**
- `binding_surface_tdd.rs` — by test name, not coordinate:
  - `layer_module_wit_exports_include_lifecycle_plus_detected_stage` → expect `["run-infill"]`, `len() == 1`; rename to drop "lifecycle".
  - `layer_lifecycle_only_module_still_lists_world_lifecycle_exports` → rename to `layer_stageless_module_lists_no_exports`; expect `&[] as &[&str]`.
  - `typed_schema_const_mirrors_string_accessors_for_layer_infill` → expect `exports.len() == 1` and `exports[0] == ExportBinding { name: "run-infill", kind: ExportKind::Stage }`.
  - `typed_schema_const_for_lifecycle_only_impl_has_no_stage_export` → expect `exports.len() == 0`.
  - `typed_schema_lifecycle_export_count_matches_world_lifecycle_table` → **delete** (asserts `__SLICER_LIFECYCLE_EXPORT_COUNT == 2` ×4).
  - the `names_a` ordering assertion → expect `vec!["run-infill"]`.
  - `typed_schema_kinds_distinguish_lifecycle_from_stage` → **delete, or** drop the `lifecycle_count` half and keep `assert_eq!(stage_count, 1)`. **This is the last test in the file** — deleting it takes the file's final `}` with it if you cut by coordinate; cut by construct.
- `all_worlds_glue_tdd.rs` — by test name:
  - `macro_no_longer_emits_placeholder_shim_for_supported_worlds` → drop the `src.contains("let skip_lifecycle_shims = real_glue_world.is_some();")` source assertion (the string it greps for is being deleted from the macro).
  - `macro_layer_world_covers_all_eight_stage_exports_plus_lifecycle` → drop `"fn on_print_start"` / `"fn on_print_end"` from the `export_arm` array and rename to `macro_layer_world_covers_all_eight_stage_exports`.
- `slicer_module_tdd.rs`, `smoke.rs`: fixture impls rename to `from_config`, `on_print_end` bodies deleted.

**CLI scaffold** — `crates/pnp-cli/src/module_new.rs`
- `:11` `use slicer_schema::{lifecycle_exports_for_world, stage_by_id, STAGES};` → drop the first import.
- in `generate_manifest`, `expected_exports` becomes `vec![spec.wit_export]`; the manifest template's trailing `# (lifecycle exports + the stage-specific export emitted by #[slicer_module])` line becomes "the stage-specific export emitted by `#[slicer_module]`".
- `:309-311` scaffold `fn from_config`.
- `:328-332` delete the generated `on_print_start_succeeds()` test.
- the `lib_rs_has_on_print_start_lifecycle` fn delete `lib_rs_has_on_print_start_lifecycle`.

**Mechanical sweep** (scripted; do not read these files)
- 20 × `modules/core-modules/*/src/lib.rs` + their `tests/`; `modules/core-modules/arachne-perimeters/src/lib.rs:419` is the only real `on_print_end` body — delete it, don't rename it.
- 20 × `modules/core-modules/*/tests/slicer_module_binding_tdd.rs` — each asserts `exports.contains(&"on-print-start")` / `&"on-print-end"`; delete those two assertions per file, keep the stage assertion.
- 9 × `crates/slicer-wasm-host/test-guests/*/src/lib.rs`.
- 4 × `crates/slicer-sdk/tests/{layer,prepass,postpass,finalization}_module_tdd.rs`.
- ~30 × `crates/slicer-runtime/tests/**` in-process construction sites.

**Docs (the deletion's own fallout)** — `docs/03_wit_and_manifest.md:558-562`
- Delete the three-line stanza: `// Lifecycle — optional` (`:559`) plus the two `export on-print-*` lines (`:560-561`) it labels.
- **The comment is doubly false and worth understanding before deleting it.** It labels the exports *optional*; the component model has no optional exports — wasmtime's generated `Indices::new` eagerly resolves every export at `instantiate`, which is exactly ADR-0045's premise and the reason the per-stage split exists at all. This listing, written in the initial commit before a host existed, is the origin of the fiction every other artifact in this packet faithfully implements. `#[slicer_module]`'s `Ok(())` padding exists *because* someone believed this line.
- **Coordination with packet #3:** #3 restructures docs/03's WIT listing wholesale (worlds → per-stage packages). This is a disjoint three-line delete of a stanza that will not exist in any shape. No conflict; no ordering constraint.

**CLI freshness — three sites, fixed in place**
- **Do not extract a shared helper.** The triplication below is deliberate and approved. The extraction needs an ADR, not a refactor: the only two shapes are a dev-dep cycle onto a `pnp-cli` lib target, or a new host-side test-support crate. Neither has precedent — ADR-0004 governs only *guest-side* test support in `slicer-sdk`, and `slicer-test`, the crate that might have hosted it, was deleted by packet 78. A reviewer or implementer who "helpfully" DRYs these three copies is making an architecture decision inside a deletion packet. See `[FWD]`.
- `crates/slicer-runtime/tests/common/slicer_cache.rs`: `:107-146` rewrite `pnp_cli_bin`. New shape:
  - `fn newest_source_mtime(root: &Path) -> SystemTime` — mirrors `xtask/src/build_guests.rs::compute_shared_mtime`: max over `crates/*/src/**` (all files) and each `crates/*/Cargo.toml`, plus `crates/slicer-schema/wit/**/*.wit`, plus the workspace `Cargo.toml`. Deliberately **excludes** `tests/`, `benches/`, and `modules/` — those do not link into `pnp_cli`.
  - `pub fn staleness_reason(bin_mtime: Option<SystemTime>, newest_src_mtime: SystemTime) -> Option<String>` — the pure, testable seam. `None` ⇒ absent binary ⇒ `Some`; `Some(art)` with `newest_src_mtime > art` ⇒ `Some` (mirrors `is_stale`'s `Some(art) => newest_src > art` arm). Message names `pnp_cli`, the word `stale`, the resolved path, and the remedy (`cargo build --workspace` or `cargo xtask test`).
  - `pnp_cli_bin()` keeps the profile-inference block (`:123-131`, which is correct and matches the test's own profile) and **deletes the `for profile in ["release", "debug"]` fallback (`:133-140`)**, then panics on `Some(reason)`.
- `crates/slicer-runtime/benches/gate_evidence.rs::pnp_cli_bin` — same profile-inference-then-release-fallback shape, same fix, applied in place. Its doc-comment already says "Mirrors (does not import…)"; keep that honest and drop its "falling back to release-then-debug if profile inference fails" clause, which describes the branch being deleted. **The fn spans the doc-comment through its closing `}` — delete the `for profile in ["release", "debug"] { … }` loop *and* leave the `panic!` reachable only via `staleness_reason`.**
  - **This bench is why site 2 is not optional.** `gate_evidence` times a real `pnp_cli slice` subprocess (`resources/regression_wedge.stl` + `resources/test_config/gate_evidence_50l.json`) and is the **sole source of DEV-026's 50-layer full-slice-time evidence** — the `~438ms median [435.25, 438.01, 440.97] ms` recorded in `docs/DEVIATION_LOG.md`'s DEV-026 row, which is what downgraded that governance gap from open to time-evidenced. A stale binary here does not produce a red test; it produces a **plausible number attributed to the wrong code**, silently invalidating governance evidence someone will cite in an acceptance gate. That is strictly worse than a failing test, because nothing signals it.
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`: `:15-31` `fn bin()` — third copy; probes debug-then-release, no mtime check. Same freshness assert, and **fix the panic message**: it currently reads `"pnp_cli binary not found. Run `cargo build --workspace` first."`, which is advice that builds the binary once and does nothing to keep it fresh — the message actively teaches the habit that produced the trap. Replace with one naming `cargo build -p pnp-cli` and stating the cause (the binary is older than `crates/*/src/**`; `cargo test -p slicer-scheduler` does not rebuild another package's binary).
- `xtask/src/test.rs:129-137`: after `build_guests::check_command`, check `pnp_cli` and rebuild via `cargo build --bin pnp_cli` on staleness, aborting the run on rebuild failure — same shape as the existing guest branch.
- New `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs`, mounted as `mod pnp_cli_freshness_tdd;` in `crates/slicer-runtime/tests/integration/main.rs` — the bucket's aggregator, a 45-entry alphabetical `mod` list; the new line goes between `mod pipeline_tdd;` (`:40`) and `mod region_partition_tdd;` (`:46`). **Without this line the file never compiles and `cargo test --test integration pnp_cli_freshness` reports 0 tests — a false pass.**. Three synthetic-mtime cases: older binary ⇒ `Some` containing `pnp_cli` + `stale`; absent binary ⇒ `Some`; newer binary ⇒ `None`.

**Guard test** — `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`
- Add `no_lifecycle_exports_anywhere`, modelled on `no_versioned_world_identifiers_outside_canonical_wit` (`:668-713`): reuse `workspace_root()` and `collect_files(&root, &["rs", "wit"], &mut files)`; skip the guard file itself by `file_name()` (the same escape the existing guard uses at `:684-689`, and necessary because the test names the forbidden strings); assert `files.len() > 100`; collect `file:line` offenders for `on-print-start|on-print-end|on_print_start|on_print_end` on non-comment lines; assert `offenders.is_empty()` with the offender list in the message.

### Rejected alternatives

- **Collapse `ExportKind` / `ExportBinding` entirely** (make `exports: &'static [&'static str]`, or drop `exports` since it duplicates `stage_export`). Rejected: packet #2 restructures `ExportBinding` and `SlicerModuleSchema.exports` into package+interface form. Collapsing the types now is churn that #2 immediately re-churns, and it would delete a field #2 needs to widen rather than re-create.
- **Keep `ExportKind::Lifecycle` as an unconstructed variant.** Rejected on honesty grounds — it documents an export that no longer exists, which is precisely the class of self-certifying fiction ADR-0044/0045 exist to end.
- **Reuse `xtask::build_guests::is_stale` from `slicer_cache.rs` by importing it.** Rejected: **`xtask` is a bin-only crate** — `xtask/Cargo.toml` declares no `[lib]` and `xtask/src/main.rs` mounts `build_guests` as a private `mod`. There is nothing to `use`. Giving `xtask` a lib target and dev-depending `slicer-runtime` on it would pull `walkdir` + `toml` into every test build to save ~30 lines. The freshness logic is therefore **mirrored**, not imported, and `design.md` pins the exact source function so the two stay legible as siblings.
- **`env!("CARGO_BIN_EXE_pnp_cli")` (the plan's original fix).** **Falsified during grounding** — see the plan's §"Grounding corrections" §1. Cargo sets `CARGO_BIN_EXE_<name>` only for integration tests of the package that *defines* the binary, which is why `crates/pnp-cli/tests/e2e_integration_tdd.rs:394` uses it successfully while every fragile spawn site lives in `slicer-runtime`. A dev-dependency does not make the var available.
- **A conservative "any workspace file newer than the binary" scan.** Rejected: it would fire on every test-file edit, and a gate that always fires gets disabled. The scan is scoped to inputs that actually link into `pnp_cli`.

## Files in Scope (read + edit)

Nine primary files, above the target of 3. **Justification:** the packet is not decomposable below this. The lifecycle contract is a single declaration mirrored across five layers (WIT → schema table → macro metadata → macro glue → SDK trait) plus two consumers (CLI scaffold, guest artifacts); deleting any subset leaves the workspace non-compiling and the packet half-done, which is the state the ADR queue explicitly forbids. Splitting would produce packets that cannot individually pass `cargo check`. The 110-file sweep is mechanical and scripted — it is not read. The CLI-freshness fold-in is two additional files, justified by the plan's §"Fold in": this packet's blast radius is measured by exactly the tests the stale binary corrupts, so the gate must precede the measurement.

- `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` - role: the declaration; expected change: delete lines 20-21, 10 exports → 8.
- `crates/slicer-schema/src/lib.rs` (~440 lines) - role: the self-certifying mirror table; expected change: delete `WORLD_LIFECYCLE_EXPORTS`, `lifecycle_exports_for_world`, `ExportKind::Lifecycle`, the vacuous test; repair two doc comments.
- `crates/slicer-sdk/src/traits.rs` (1700 lines) - role: the four traits; expected change: 4 × rename to `from_config`, 4 × delete `on_print_end`, rewrite 4 doc blocks.
- `crates/slicer-macros/src/lib.rs` (2800+ lines) - role: metadata builder + `impl Guest` glue; expected change: delete lifecycle metadata, shims, count const, and layer glue fns; 15 call-site renames.
- `crates/pnp-cli/src/module_new.rs` - role: scaffold; expected change: single stage export, `from_config` template, two tests deleted.
- `crates/slicer-runtime/tests/common/slicer_cache.rs` - role: the false-baseline source, spawn site 1 of 3; expected change: freshness assertion + release-fallback deletion.
- `crates/slicer-runtime/benches/gate_evidence.rs` - role: spawn site 2 of 3; sole producer of DEV-026's `~438ms` governance evidence; expected change: same freshness assertion at `:44-74`, release-fallback (the `for profile in ["release", "debug"]` loop) deleted, doc-comment `:46-47` corrected.
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` - role: spawn site 3 of 3; expected change: same freshness assertion at `:15-31`, debug-then-release fallback deleted, panic message (`:30`) rewritten to name `cargo build -p pnp-cli` and the staleness cause.
- `xtask/src/test.rs` - role: the gated entry point; expected change: Step 1 also gates `pnp_cli`.
- `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` - role: guard host; expected change: add `no_lifecycle_exports_anywhere`.
- `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` (new) + `tests/integration/main.rs` (one `mod` line) - role: the freshness regression test.

Plus the scripted sweep set (`modules/core-modules/**`, `crates/slicer-wasm-host/test-guests/**`, `crates/slicer-sdk/tests/**`, `crates/slicer-runtime/tests/**`, `crates/slicer-macros/tests/**`) and the doc set (`docs/04_host_scheduler.md`, `docs/05_module_sdk.md`, `docs/07_implementation_status.md`).

## Read-Only Context

- `xtask/src/build_guests.rs` - the `compute_shared_mtime` and `is_stale` fns only (locate by name) - purpose: the exact algorithm being mirrored (`is_stale`'s `Some(art) => newest_src > art` arm; `None => true` for a missing artifact).
- `xtask/src/test.rs` - lines `100-140` only - purpose: Step 1's existing check-then-rebuild-then-abort shape.
- `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` - the `no_versioned_world_identifiers_outside_canonical_wit` fn (locate by name) only - purpose: the `collect_files` + offender-list guard shape `AC-N1` mirrors.
- `crates/slicer-runtime/tests/integration/main.rs` - lines `1-25` only - purpose: how a bucket file is mounted.
- `crates/pnp-cli/tests/e2e_integration_tdd.rs` - line `394` only - purpose: the proof that `CARGO_BIN_EXE_pnp_cli` works in the defining package and only there.
- `docs/05_module_sdk.md` - lines `165-200`, `290-300`, `370-380`, `726-736`, `840-970`, `1140-1150` only.
- `docs/04_host_scheduler.md` - lines `1444-1455` only.
- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` - §"The lifecycle finding", §"Grounding corrections", §"Packet Queue".

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - **do not load, and do not delegate either**: this packet borrows no canonical behavior; see §Controlling Code Paths.
- `target/`, `Cargo.lock`, `*.wasm`, generated code, vendored dependencies - never load. Guest exports are inspected only via `wasm-tools component wit <path> | grep`.
- `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-wasm-host/src/dispatch.rs` - packet #2's seam; this packet's WIT edit removes exports the host never called, so no host dispatch change is required. Confirm by grep, do not browse.
- `CONTEXT.md`, `crates/slicer-schema/wit/README.md` - packet #3 owns these. `docs/03_wit_and_manifest.md` is **in scope for `:559-561` only**; its WIT listing's restructure remains #3's.
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs`, `crates/slicer-model-io/src/loader.rs::path_object_id`, and every parity golden/baseline - the `object_id` session owns them and **landed at `ff21378e`**. Read-only here; run them, never edit them.
- All other crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: "Does `crates/slicer-schema/wit/` contain any `.wit` file other than `world-layer.wit` declaring `print-start` or `print-end`?"; scope: `crates/slicer-schema/wit/**`; return: `FACT`; purpose: Step 2 precondition (grounded answer: no).
- Question: "After the sweep, how many occurrences of `on_print_start|on_print_end` remain outside `wit_drift_detection_tdd.rs`?"; scope: `--include=*.rs crates/ modules/ xtask/`; return: `FACT` (one integer); purpose: Step 7 exit.
- Question: "Run `cargo check --workspace --all-targets`; report pass/fail and, on failure, the first 20 lines of error output."; scope: workspace; return: `FACT` + `SNIPPETS` ≤20 lines; purpose: Step 8 gate.
- Question: "Run `cargo xtask build-guests`; report pass/fail and the tail 20 lines on failure."; scope: workspace; return: `FACT`; purpose: Step 9.
- Question: "Run `cargo xtask test --summary --workspace`; return PASS/FAIL and the names of any failing tests."; scope: workspace; return: `FACT pass/fail`; purpose: acceptance ceremony only.
- Question: "In `docs/07_implementation_status.md`, record TASK-146a per the existing TASK-119a/TASK-194a sub-lettering convention."; scope: `docs/07_implementation_status.md`; return: `FACT` (the added line); purpose: Step 11 — **never read the backlog directly**.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4, the macro — a 2800-line file with 25 enumerated edit sites; mitigated by pinning every line number here so only ±40-line windows are opened)
- Highest-risk dispatch and required return format: `cargo xtask test --summary --workspace` at the acceptance ceremony — must be dispatched to a sub-agent returning `FACT pass/fail` plus failing-test names only. Absorbing that output directly would blow the budget in one call.

## Open Questions

- `[FWD]` **Extracting the three `pnp_cli` lookups into one shared helper.** Deferred to its own packet by explicit decision; this packet fixes all three in place with the same assert. The extraction is an architecture decision needing an ADR, not a refactor: the only shapes are (a) a dev-dep onto a new `pnp-cli` lib target — which inverts the current dependency direction and would make every `slicer-runtime` test build depend on the CLI, or (b) a new host-side test-support crate. Neither has precedent: **ADR-0004 covers only guest-side test support in `slicer-sdk`**, and **`slicer-test`, the crate that could have hosted this, was deleted by packet 78.** A reviewer who DRYs these three copies inside this packet is making that decision silently — the exact failure mode this packet exists to correct. Resolve in the extraction packet; until then, triplication is the honest state.
- `[FWD]` **`ExportKind` decision, recorded rather than asked.** The premise that `ExportKind::Lifecycle` becoming unconstructible is "a `dead_code` risk under `-D warnings`" is **false**: `ExportKind` is a `pub` enum in a lib crate, and rustc's `dead_code` lint does not fire on publicly-reachable items in a library target. Nothing forces the variant's removal. It is deleted anyway, on honesty grounds — an enum variant naming an export that no longer exists is the same self-certifying fiction as the table above it. The enum and `ExportBinding` are **kept** (single `Stage` variant, still constructed by the macro's `stage_binding_tokens`) because packet #2 restructures both into package+interface form. There *is* a real `-D warnings` forcing function in this packet, just a different one: `SUPPORTED_WIT_WORLDS`' rustdoc link to `[`WORLD_LIFECYCLE_EXPORTS`]` becomes a broken intra-doc link when the table is deleted. Fix the comment, not the const.

None blocking. Status stays `draft` pending review.
