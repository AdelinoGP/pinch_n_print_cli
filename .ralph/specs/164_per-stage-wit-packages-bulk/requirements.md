# Requirements: 164_per-stage-wit-packages-bulk

## Packet Metadata

- Grouped task IDs: `TASK-146c`
- Backlog source: `docs/07_implementation_status.md` (TASK-146 reopened by ADR-0044/0045; sub-lettered per `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Task mapping")
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 163 proved the per-stage versioned-package mechanism on the three cheapest stages, deliberately leaving two contract mechanisms live in-tree: postpass/finalization on per-stage packages, layer/prepass on monolithic tier worlds. That intermediate is an accepted deviation **owned by this packet**, and the plan's dependency note is blunt about why it cannot linger: leaving it undone reproduces the exact failure ADR-0045 exists to end (cf. `run.rs`'s 2026-05 "pragmatic fix", still load-bearing 14 months later). The layer tier is where the original pain landed — `arachne-perimeters` was invalidated by packet 130's infill change — and it is still exposed: every layer module ships benign-`Ok` padding for 7 sibling stages it never implements, violating ADR-0015 by construction. Meanwhile `wit-world` / `SUPPORTED_WIT_WORLDS` / `validate_wit_world` continue comparing one hand-written string to another (ADR-0044's "unfalsifiable ceremony"), and after 163 they additionally name two packages that no longer exist.

**Grounded counts (falsifies the plan's arithmetic — see `design.md` for evidence):** the plan and ADR-0045 say "17 packages: 10 layer + 4 prepass + 2 postpass + 1 finalization". The tree says otherwise: `slicer_schema::STAGES` has 16 rows — **8** Layer, **5** PrePass, 2 PostPass, 1 Finalization — and `world-layer.wit` declares 8 stage exports (10 was stage exports **plus** the two lifecycle exports packet 162 deletes). Of the 5 PrePass rows, `PrePass::PaintSegmentation` is **host-built-in** (packet 97; executes in `crates/slicer-runtime/src/prepass.rs`, no WIT export in `world-prepass.wit`, no core module). The delivered end state is therefore **15 per-stage packages** (8+4+2+1), of which 163 shipped 3 and this packet ships 12.

## In Scope

- 12 new per-stage WIT packages at `@1.0.0` under `crates/slicer-schema/wit/deps/<pkg>/<pkg>.wit`, named by 163's rule (tier from `StageSpec.world_id`, stage name from the `stage_id` local part, kebab-case): `layer-slice-postprocess`, `layer-perimeters`, `layer-perimeters-postprocess`, `layer-infill`, `layer-infill-postprocess`, `layer-support`, `layer-support-postprocess`, `layer-path-optimization`, `prepass-mesh-analysis`, `prepass-layer-planning`, `prepass-seam-planning`, `prepass-support-geometry`. Note `Layer::PerimetersPostProcess` packages as `layer-perimeters-postprocess` even though its legacy export is `run-wall-postprocess` — the rule keys on `stage_id`, never on `wit_export` (163's ledger is explicit).
- One new **unversioned** flat dep package `crates/slicer-schema/wit/deps/prepass-types.wit` (`package slicer:prepass-types;`) holding the view records shared by seam-planning and support-geometry (`mesh-object-view`, `paint-value-view`, `paint-stroke-view`, `paint-layer-view`).
- Deletion of `crates/slicer-schema/wit/deps/world-layer/` and `deps/world-prepass/`.
- `crates/slicer-schema/src/lib.rs`: the 12 rows' `wit_dir`/`wit_package`/`wit_interface`/`wit_world` filled, `wit_export` → `"run"`; the `PrePass::PaintSegmentation` row's five WIT columns set to `""` with a host-built-in comment; the 163 totality guard relaxed accordingly; `SUPPORTED_WIT_WORLDS` deleted.
- `crates/slicer-wasm-host/src/host.rs`: `pub mod layer` / `pub mod prepass` → 12 per-stage `bindgen!` mods (five-key `with:` block each; shared `slicer:prepass-types/prepass-types` bindings defined once and `with:`-aliased elsewhere); `layer_impls`-equivalent resource impl blocks and prepass resource impls repointed at the new mods.
- `crates/slicer-wasm-host/src/dispatch.rs`: `dispatch_layer_call`/`call_layer_export` and `dispatch_prepass_call`/its export router rewired to per-stage instantiate-and-`run`; `TypedInstantiation` reasons name `qualified_export_for_stage_id`.
- `crates/slicer-macros/src/lib.rs`: `StageGlueKind::Layer` → 8 variants, `::Prepass` → 4; `resolve_stage_glue`'s remaining trait-name fallbacks deleted; `build_layer_world_glue`/`build_prepass_world_glue` → 12 per-stage builders on 163's template; per-package `include_str!` call sites.
- Metadata qualification (163's `[FWD]`, consumed here): `SlicerModuleSchema.stage_export` and `__slicer_wit_exports()` carry the qualified `slicer:<pkg>/<iface>@1.0.0#run` spelling for all migrated stages, in one move, now that every WASM stage is migrated.
- `wit-world` retirement: key dropped from all 20 `modules/core-modules/*/*.toml`; `crates/slicer-scheduler/src/manifest.rs` loses the `wit_world` field, accessor, builder parameter, `required_string` parse, and `validate_wit_world`; legacy manifests carrying the key load with it ignored; `crates/pnp-cli/src/module_new.rs` stops scaffolding it (drops `wit_world_for_stage` + `wit_world_mapping` test; scaffold comment names the qualified export).
- Hand-written test guests retargeted: `prepass-guest`, `layer-infill-guest`, `infill-postprocess-echo-guest`, `path-optimization-multi-read` (`wit_bindgen::generate!` `world:` + interface-grouped `Guest` impl). Macro-authored guests (`sdk-*`, 15 core modules) regenerate with no source edits.
- Test updates: the layer/prepass core-module `slicer_module_binding_tdd.rs` expectations — derive the file set at point of use with `ls modules/core-modules/*/tests/slicer_module_binding_tdd.rs` (14 exist at authoring time; `arachne-perimeters` has **none** and gains one in this packet — it is the ADR's headline isolation example and must pin its own binding surface); `wit_single_source_tdd.rs` / `wit_drift_detection_tdd.rs` to the 15-world surface; a layer case added to `stage_miss_is_fatal_at_instantiation`; scheduler/runtime test fixtures that construct manifests with `wit-world` updated.
- Docs per `packet.spec.md` §Doc Impact; deviation-row closure per AC-8.

## Out of Scope

- The three pilot stages' packages, glue, bindgen mods, dispatch fns — 163's, consumed as-is.
- `WORLD_LAYER` / `WORLD_PREPASS` / `WORLD_POSTPASS` / `WORLD_FINALIZATION` consts and `StageSpec.world_id` — **retained** as tier vocabulary (their doc comments are corrected to say "tier id; not a loadable WIT package since packet 164"). 163's design listed them among things "#3 retires", but that overreached the plan's queue row: 30+ files consume them as tier identity (scheduler DAG, instrumentation, macro metadata, dozens of tests), the naming rule itself reads the tier from `world_id`, and deleting them buys no honesty a doc comment doesn't. Recorded as an explicit divergence from 163's expectation in `design.md` §Open Questions.
- `ExportKind` / `ExportBinding` structural collapse beyond the metadata qualification above.
- Per-stage staleness granularity for **test** guests (`[package.metadata.slicer] stage_id` in 12 `Cargo.toml`s) — conservative over-rebuild is safe and correct; deferred again with rationale in `design.md`.
- The `pnp_cli` binary-locator extraction (packet 165), DEV-085 (custom G-code injection points), DEV-087 (MissingComponent laundering — referenced by dispatch work here, not resolved), DEV-026 (advisory DAG).
- The 7 known-red parity tests' own fixes (their fix landed pre-162 per the plan's §"Status since approval"; this packet only keeps the baseline green).
- `OrcaSlicerDocumented/` — no parity surface exists for a WASM contract refactor (see `design.md` §Controlling Code Paths).

## Authoritative Docs

- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` — long; ranged reads of the five sections named in `packet.spec.md`.
- `.ralph/specs/163_per-stage-wit-packages-pilot/design.md` — long; direct read of §"Exports handed to packet #3", §"Naming", §"Data and Contract Notes" only.
- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` — long; ranged reads of §"Grounding corrections", §"Packet Queue", §"Exports ledger".
- `docs/adr/0006-export-for-stage-id-sole-lookup.md`, `docs/adr/0002-wit-marshalling-type-unification.md` — short; direct read.
- `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` — delegated SUMMARY.
- `docs/03_wit_and_manifest.md` — 1870 lines; delegate a LOCATIONS survey for every `world-layer` / `world-prepass` / `wit-world` mention before editing.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (WIT shape ×12 + prepass-types + tier dirs gone), `AC-2` (STAGES totality incl. the host-built-in exception), `AC-3` (15 `bindgen!` mods, `with:` discipline), `AC-4` (dispatch rewiring + executor suite), `AC-5` (macro glue split, fallbacks dead), `AC-6` (decoded guest exports), `AC-7` (`wit-world` retirement end-to-end), `AC-8` (deviation closure), `AC-9` (contract-guard suites on the 15-package surface).
- Negative: `AC-N1` (fatal-on-miss, layer case, engine's expected-only wording), `AC-N2` (isolation: touch one stage's `.wit`, unrelated core guests stay FRESH and byte-identical — the ADR's headline claim, now on the motivating tier), `AC-N3` (manifest loads without `wit-world`; legacy key tolerated-ignored), `AC-N4` (behavior-neutrality baseline).
- Cross-packet impact: consumes 163's exports ledger wholesale; closes the deviation row 163 files; leaves nothing for a successor — after this packet exactly one contract mechanism exists. Diverges from 163's expectation on `WORLD_*`/`world_id` retirement (kept as vocabulary; see Out of Scope).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` (rebuild without `--check` if STALE) | guest freshness before believing any failure | FACT clean/STALE list |
| `cargo check --workspace --all-targets` | tree compiles incl. test targets | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p slicer-schema 2>&1 \| tee target/test-output.log` (unfiltered) | STAGES columns, lookups, relaxed totality guard | FACT pass/fail |
| `(cargo test -p slicer-runtime --test contract -- wit_single_source 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed' \|\| echo FAIL` | canonical WIT resolves, 15 worlds | FACT pass/fail |
| `(cargo test -p slicer-runtime --test contract -- wit_drift_detection 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed' \|\| echo FAIL` | package pins, `major >= 1` across all 15 | FACT pass/fail |
| `(cargo test -p slicer-runtime --test contract -- stage_miss_is_fatal_at_instantiation 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed' \|\| echo FAIL` | fatal-on-miss incl. new layer case | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor 2>&1 \| tee target/test-output.log` (unfiltered) | layer/prepass/postpass/finalization dispatch round-trips | FACT pass/fail + failing names |
| `(cargo test -p slicer-scheduler --test integration -- manifest_ingestion 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed' \|\| echo FAIL` | manifest parses without `wit-world`; legacy key ignored | FACT pass/fail |
| `(cargo test -p xtask -- stage_wit 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed' \|\| echo FAIL` | per-stage staleness charging still holds with 12 more dirs | FACT pass/fail |
| `(cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed' \|\| echo FAIL` | behavior-neutrality (12 passed expected) | FACT pass/fail |
| `(cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed' \|\| echo FAIL` | golden G-code unchanged | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask test --summary --workspace` | packet-close acceptance ceremony ONLY, after all narrower gates pass | dispatch to sub-agent; FACT pass/fail + failing names only |

## Step Completion Expectations

- The tree does **not** compile between the WIT-split step and the end of the dispatch step — six layers move together, exactly as in 163. No step before the first `cargo check` gate lists a compile exit; an implementer must not "fix" a red `cargo check` mid-sequence by touching out-of-scope files.
- The `wit-world` retirement steps are **independent** of the WIT-split steps and are sequenced after the tree is green again, so a retirement bug cannot be confused with a migration bug.
- `cargo xtask build-guests` (full rebuild) must run after the macro/WIT/schema steps and before any executor or e2e verification — every guest is invalidated by this packet's surface.
- The deviation-row closure and docs steps run last, after all code ACs pass, so the closure note describes a landed state.

## Context Discipline Notes

- `crates/slicer-macros/src/lib.rs` (~2900 lines pre-163), `crates/slicer-wasm-host/src/host.rs` (~4100), `crates/slicer-wasm-host/src/dispatch.rs` (~2500), `docs/03_wit_and_manifest.md` (1870): ranged reads only, per the line hints in `design.md` — re-verify every hint against the post-163 tree at the moment of use; 163's edits will have shifted them.
- The 20 core-module manifests and the `ls`-derived binding-test set (14 at authoring time, plus the new arachne-perimeters file) are one-line/one-file mechanical edits; never open more than the matched line ± 5 (the new file copies `classic-perimeters`' test whole).
- `cargo xtask test --summary --workspace` runs once, at the acceptance ceremony, dispatched to a sub-agent returning FACT pass/fail + failing names. Never absorb its output.
- All ledger facts (deviation ID, line counts, "N manifests") must be re-derived at point of use per `CLAUDE.md` §"Ledger Facts Must Be Re-derived, Not Quoted"; this packet's own counts were true at authoring time (2026-07-17, tree at `spec(163)` HEAD) and must be re-checked after 162/163 land.
