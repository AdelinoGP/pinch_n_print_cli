# Design: 201-integrated-module-registry-tier5

## Controlling Code Paths

- Primary code path: `load_modules_from_roots` → `discover_manifest_paths` → `ingest_manifest` → `LoadedModuleBuilder::build` (`crates/slicer-scheduler/src/manifest.rs`); consumed by `load_live_modules_for_plan_profiled` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) and by two loader call sites in `crates/slicer-runtime/src/run.rs`, which use **different** entry points: `run_slice_with_collector` calls `load_live_modules_for_plan_profiled`, and `prepare_prepass_context` calls `load_live_modules_for_plan_with_config` (the latter is *defined* in `crates/slicer-wasm-host/src/execution_plan_live.rs`, not in `run.rs`). Both entry points live in that same wasm-host file. Because the two sites differ, site 2 must pass `profile: false` when it switches to the integrated-aware entry point.
- Neighboring tests/fixtures: `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` (fixture patterns for synthetic roots and the real `modules/core-modules` root; aggregator `crates/slicer-scheduler/tests/integration/main.rs`); `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`.
- OrcaSlicer comparison: none — no OrcaSlicer behavior in this packet; §OrcaSlicer Reference Obligations deliberately absent from `packet.spec.md`/`requirements.md`.

## Architecture Constraints

- ADR-0056 Decision item 1 "One model" requires only that ingestion be **generalized over artifact source**, and that scheduling, claims, and config resolution never learn what "native" means. This packet satisfies that: claims, DAG, and config machinery never branch on provenance.
- **Packet decision (not an ADR clause):** the integrated tier enters at the *loader*, not the root assembler — `assemble_search_roots` keeps returning `Vec<PathBuf>` untouched; tier 5 is a post-roots ingestion phase inside `load_modules_from_roots_with_integrated`. ADR-0056 makes no loader-vs-assembler statement; this choice is argued on its merits in §Rejected alternatives ("Tier 5 as a synthetic search root"). Downstream packets 202 and 203 treat *this file*, not the ADR, as the authority for it.
- ADR-0056 Decision item 2 "Lowest search priority": integrated entries are processed **after** every disk root through the same `seen_ids` set, so first-root-wins dedup by `module.id` is literally the same code path.
- `IntegratedModuleRegistration` carries only manifest text and an origin label. Dispatch information (native entries) is a packet-202 concern that lives at the wasm-host layer, never in `slicer-scheduler`.
- Downstream invariants untouched (verified present at authoring): `dedup_same_claim_modules_with_wall_generator` (`crates/slicer-scheduler/src/execution_plan.rs`), `ExecutionPlanError::DuplicateModuleBinding` (same file), `validate_startup_dag` (`crates/slicer-scheduler/src/validation.rs`). No edit to any of them.
- No wasm-staleness snippet: this packet's change surface (`slicer-scheduler`, `slicer-wasm-host`, `slicer-runtime`, new `slicer-integrated-modules`, docs) is host-only — none of it feeds guest WASM builds per the applies-to list in `.claude/skills/spec-packet-generator/references/snippets/wasm-staleness.md`.
- No coord-system snippet: manifest/loader wiring, no geometry or mm/unit conversion.

## Code Change Surface

- Selected approach — **provenance field + text-source ingestion + post-roots tier, registry in a dedicated crate**:
  1. `manifest.rs`: add `pub enum ModuleProvenance { External, Integrated }` (Copy, Eq); `LoadedModule` gains `pub(crate) provenance: ModuleProvenance`; `LoadedModuleBuilder` gains a `.provenance(...)` setter defaulting to `External` (all construction goes through the builder. Verified at authoring: most `LoadedModule` fields are `pub(crate)` — two, `region_splits` and `region_split_semantics`, are `pub`, but a literal needs *every* field visible, so no crate outside `slicer-scheduler` can construct one. The struct-literal blast radius is therefore the single literal inside `LoadedModuleBuilder::build`).
  2. `manifest.rs`: split `ingest_manifest` — the disk wrapper keeps its exact signature and semantics (`ensure_same_stem_wasm_exists`, `fs::read_to_string`, `is_placeholder_wasm`), then delegates to a new `ingest_manifest_text(manifest_text, diagnostics_path, wasm_path, placeholder_wasm, provenance)` core holding everything from TOML parse to `build()`. Integrated ingestion calls the core directly with `placeholder_wasm = false`, `provenance = Integrated`, and `wasm_path`/`diagnostics_path` = `PathBuf::from(origin_label)`.
  3. `manifest.rs`: `pub fn load_modules_from_roots_with_integrated(search_roots: &[PathBuf], integrated: &[IntegratedModuleRegistration]) -> Result<LoadModulesReport, LoadError>` — existing disk loop verbatim, then the integrated loop over the same `seen_ids`; `load_modules_from_roots` becomes a delegation with `&[]`. Duplicate handling in the integrated loop: if the id was seen and the earlier winner's `provenance()` is `External`, push the shadow warning `external module '{id}' shadows integrated module '{id}'` (level Warning, `field: Some("module.id")`, `path` = origin label); integrated-vs-integrated duplicates keep the existing generic duplicate-id message.
  4. `crates/slicer-integrated-modules/` (new workspace member): `Cargo.toml` with one feature per module directory name and no default features; `src/lib.rs` with per-feature `include_str!("../../../modules/core-modules/<name>/<name>.toml")` constants, `pub fn integrated_registrations() -> Vec<IntegratedModuleRegistration>` assembling the enabled set, `origin_label` convention `integrated://<module-dir-name>`. Proving instance: feature `classic-perimeters` plus a `#[cfg(all(test, feature = "classic-perimeters"))]` test asserting id `com.core.classic-perimeters` ingests with Integrated provenance.
  5. `execution_plan_live.rs`: `pub fn load_live_modules_for_plan_with_integrated(search_roots, host_parallelism, config_source, profile, integrated: &[IntegratedModuleRegistration])` holds the current `load_live_modules_for_plan_profiled` body with two changes — it calls `load_modules_from_roots_with_integrated`, and its compile loop starts with `if module.provenance() == ModuleProvenance::Integrated { push LiveModuleBinding { module, instance_pool, wasm_component: None }; continue; }`. `load_live_modules_for_plan_profiled` delegates with `&[]` (same pattern the file already documents for the profiled entry point: add an entry point, never a parameter, to keep existing call sites untouched).
  6. `run.rs`: both live-loader call sites switch to the new entry point passing `slicer_integrated_modules::integrated_registrations()`; `crates/slicer-runtime/Cargo.toml` gains the `slicer-integrated-modules` path dep (no features); `crates/slicer-runtime/src/lib.rs` re-exports `ModuleProvenance` next to the existing `LoadedModule` re-export (docs/01 documents that loader types are reachable under `slicer_runtime::`).
- Exact functions, traits, manifests, tests, and fixtures: listed above plus new test files `crates/slicer-scheduler/tests/integration/integrated_tier_tdd.rs` (registered in `tests/integration/main.rs`) and new tests inside `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`.
- Rejected alternatives:
  - *Registry embedded in `pnp-cli`*: couples edition selection to the binary crate, leaves `run_slice` (the library slice path) unable to see integrated modules, and gives packet 205 no single feature surface.
  - *Registry constants inside each module crate*: forces native compilation of module crates in this packet (that is 202's macro work) and scatters the registry across 21 crates.
  - *`wasm_path: Option<PathBuf>` on `LoadedModule`*: honest but churns the `wasm_path()` accessor signature across host crates and tests for zero behavioral gain; instead `wasm_path` keeps its type and carries the `integrated://<name>` origin label, with the documented invariant that it names a file only when `provenance() == External` (`compile_module_component` is unreachable for integrated modules by the step-5 guard).
  - *Tier 5 as a synthetic search root*: `assemble_search_roots` returns paths; a non-path tier would force a sum type through every root consumer. Rejected as pure churn.

## Files in Scope (read + edit)

More than 3 primary files because the slice spans two layers plus a new crate; every implementation step stays within the ≤3-edit cap.

- `crates/slicer-scheduler/src/manifest.rs` — role: ingestion/loader; expected change: provenance enum, registration type, text-source split, tier-5 loop, shadow diagnostic.
- `crates/slicer-scheduler/src/lib.rs` — role: re-exports; expected change: export `ModuleProvenance`, `IntegratedModuleRegistration`, `load_modules_from_roots_with_integrated`.
- `crates/slicer-integrated-modules/{Cargo.toml,src/lib.rs}` + root `Cargo.toml` — role: new registry crate + workspace membership.
- `crates/slicer-wasm-host/src/execution_plan_live.rs` — role: live-plan loader; expected change: integrated-aware entry point + compile-skip guard.
- `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/Cargo.toml`, `crates/slicer-runtime/src/lib.rs` — role: production wiring + re-export.
- Tests: `crates/slicer-scheduler/tests/integration/integrated_tier_tdd.rs` (new), `crates/slicer-scheduler/tests/integration/main.rs` (mod registration), `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`.
- Docs: `docs/01_system_architecture.md` (§Module Search Path), `docs/04_host_scheduler.md` (§Phase 1).

## Read-Only Context

- `crates/slicer-scheduler/src/manifest.rs` — lines 28–400 (LoadedModule + builder) and 566–860 (loader/ingestion) only, ranges re-located by symbol at read time — purpose: exact insertion points; long file, never load whole.
- `crates/slicer-wasm-host/src/execution_plan_live.rs` — 353 lines at authoring; read whole or locate by symbol (`rg -n '^pub fn'`), never by a pinned range — purpose: `LiveModuleBinding` shape, entry-point stack, compile loop.
- `crates/slicer-runtime/src/run.rs` — the two loader call regions only (near `assemble_search_roots` uses, around lines 550–575 and 990–1040 at authoring) — purpose: call-site swap.
- `crates/slicer-scheduler/src/module_search_path.rs` — short, may be read whole — purpose: confirm tier semantics; NOT edited.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — read only to sanity-check the include path resolves; never inline into packet artifacts.
- `docs/01_system_architecture.md` — lines 912–1015 only.

## Out-of-Bounds Files

- `.ralph/specs/194-*`, `195-*`, `196-*`, `docs/07_implementation_status.md`, `CONTEXT.md`, `docs/adr/*`, `docs/specs/multi-edition-distribution-plan.md` — never edit.
- `crates/slicer-scheduler/src/execution_plan.rs`, `crates/slicer-scheduler/src/validation.rs`, `crates/slicer-scheduler/src/module_search_path.rs` — invariants; read-only.
- `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `modules/core-modules/*/src/**` — packet 202 territory; do not touch.
- `OrcaSlicerDocumented/` — not consulted; `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- Question: list every test that constructs `LoadedModule` via `LoadedModuleBuilder` and asserts full-struct equality (PartialEq) so the provenance default can be confirmed churn-free; scope: `crates/*/tests/**`; return: LOCATIONS (≤20); purpose: Step 2 blast-radius confirmation.
- Question: which fixture helper do `manifest_ingestion_tdd.rs` tests use to synthesize a module root (`fixture.root()` pattern), and its module path; scope: `crates/slicer-scheduler/tests/**`; return: FACT ≤5 lines; purpose: Step 1 test authoring.
- Question: confirm the exact re-export list in `crates/slicer-runtime/src/lib.rs` covering `LoadedModule`/loader symbols; scope: that file; return: SNIPPETS ≤30 lines; purpose: Step 5.
- Question: run each verification command; scope: repo root; return: FACT pass/fail; purpose: every step's exit.

## Data and Contract Notes

- IR/manifest contracts: manifest TOML schema unchanged (`docs/03_wit_and_manifest.md` §Module Manifest Schema); identity stays `[module].id` (reverse-domain string); directory/file stem matters only for disk discovery pairing, which integrated entries bypass. All five `[compatibility]` keys remain required in embedded manifests — they are the same TOMLs staged by `cargo xtask dist` today. Per ADR-0056, integrated modules are version-locked by construction; the compatibility matrix still parses but cannot fail for them (no behavior change needed here — validation runs identically).
- WIT boundary: untouched.
- Determinism/scheduler constraints: integrated entries are appended in registration order after a sorted disk walk (`discover_manifest_paths` sorts); `integrated_registrations()` must return a deterministic order (feature-gated blocks in fixed source order) so module ordering stays reproducible.
- Config keys: none added; any future key must be snake_case per `CLAUDE.md`.

## Locked Assumptions and Invariants

- Integrated `LoadedModule.wasm_path` carries `integrated://<module-dir-name>` and is meaningful as a file path only for `External` provenance (enforced by the `execution_plan_live.rs` guard; documented on the accessor).
- Pre-202, an integrated module that survives dedup gets `wasm_component: None` and fails dispatch loudly via the existing `DispatchPhase::MissingComponent` path (`crates/slicer-wasm-host/src/dispatch.rs`); production cannot reach this in 201 because the default registry is empty. 202 replaces this seam with native routing.
- The shadow-diagnostic wording `external module 'X' shadows integrated module 'X'` is this packet's canonical contract string (aligned with ADR-0056's consequence bullet, which quotes the same string — `external module X shadows integrated module X` — without the inner quoting around the id) and becomes a contract consumed by 203's diagnostics surfacing; changing it later requires touching 203's tests.
- Empty-registration behavior is a strict identity (AC-N2) — the reversibility lock.

## Risks and Tradeoffs

- Feature unification: no workspace member may enable a `slicer-integrated-modules` feature in normal/dev/test profiles, or every `--workspace` build silently grows an integrated tier. Mitigation: loader tests construct `IntegratedModuleRegistration` values inline; only the registry crate's own AC command passes `--features classic-perimeters` explicitly.
- `ingest_manifest` refactor risk: the disk wrapper must preserve error ordering (MissingWasm before TOML parse) — `manifest_ingestion_tdd.rs` already pins this; run the whole `--test scheduler_integration` binary in Step 2's exit.
- The 202 signature extension of `load_live_modules_for_plan_with_integrated` (native-entry table parameter) is a known planned change; 201 must not accrete other callers of the new entry point beyond `run.rs` to keep that churn bounded.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, manifest.rs implementation)
- Highest-risk dispatch and required return format: the `LoadedModule` PartialEq blast-radius sweep — LOCATIONS, ≤20 entries; if it returns >20, redispatch per-crate.

## Open Questions

- ~~[FWD] Feature naming for the registry crate~~ — **CLOSED at authoring; not implementer's choice.** The registry crate's cargo features MUST be the bare module-directory name (`classic-perimeters`, `arachne-perimeters`, `support-planner`). This is load-bearing, not cosmetic: packet 205 composes edition features as `integrated-<name> = ["slicer-integrated-modules/<name>"]`, supplying the `integrated-` prefix at the *edition* layer. If the registry crate's own features were prefixed, 205's edition features would resolve to `slicer-integrated-modules/integrated-classic-perimeters` and its AC-7 would go red through no fault of 205. The earlier claim that such a rename is "mechanical and confined to the registry crate" was wrong — it silently breaks a downstream packet.
- [FWD] Whether `integrated_registrations()` should return `&'static [..]` instead of `Vec` — implementer's choice; `Vec` is assumed by the ACs' call shape but either satisfies them.
- [FWD] The second `run.rs` loader call site is inside `prepare_prepass_context` (verified at authoring; locate with `rg -n 'load_live_modules_for_plan' crates/slicer-runtime/src/run.rs`, never by line number). It serves support-preview and visual-debug, not the slice path. The step-5 worker must confirm both call sites' roles before swapping, pass `profile: false` at this one (it calls `..._with_config`, not `..._profiled`), and leave any test-only call sites on the old entry points. **Downstream note:** packet 203 scopes `--no-integrated-modules` to exclude this call site, so once this switch lands, `support-preview` and `visual-debug` load the integrated tier with no way to disable it — a gap 203's `requirements.md` §Out of Scope tracks explicitly.
