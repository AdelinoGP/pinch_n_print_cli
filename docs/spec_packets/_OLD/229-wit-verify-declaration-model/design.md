# Design: 229-wit-verify-declaration-model

## Controlling Code Paths

- Primary code path: `xtask/src/wit_verify.rs` — today exports `TypeMismatch`, `VerifyError`, `extract_type_blocks`, `canonical_type_blocks`, `module_stage_wit_dir`, `embedded_wit_text`, `verify_embedded_world`, with private `matching_brace`, `strip_comments`, `normalize`, `ambiguous_type_names` and the const `BRACED_KEYWORDS`. Its `#[cfg(test)] mod tests` carries 11 `#[test]`s.
- Sole consumer: `build_one` in `xtask/src/build_guests.rs`, which calls `module_stage_wit_dir` → `canonical_type_blocks` → `verify_embedded_world`, retries once via `force_rebuild_wit_bindings`, and finally returns `BuildError::StaleEmbeddedWorld`.
- Canonical WIT source of truth: `crates/slicer-schema/wit/` — `root.wit` plus 20 files under `deps/` (5 flat, 15 per-stage). All 21 use statement-form `package x:y;`.
- Macro embedding: the `include_str!` constants `TYPES_WIT`, `CONFIG_WIT`, `IR_TYPES_WIT`, `COMMON_WIT`, `PREPASS_TYPES_WIT` and the per-stage `wit_inline` `include_str!` calls in `crates/slicer-macros/src/lib.rs`; the watch list in `crates/slicer-macros/build.rs`.
- Stage vocabulary: `slicer_schema::STAGES` (16 rows), `StageSpec`'s `stage_id` / `wit_dir` / `wit_package` / `wit_interface` / `wit_world` fields, and the lookups `wit_dir_for_stage_id`, `package_for_stage_id`, `interface_for_stage_id`, `qualified_export_for_stage_id`.
- Existing `wit_parser` precedent in-tree: `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs` uses `wit_parser::Resolve::new()` with a directory push and `wit_parser::UnresolvedPackageGroup::parse`.
- Neighboring tests/fixtures: the existing real-artifact tests `built_core_module_components_embed_canonical_world` and `detects_drift_against_a_real_built_artifact` in `xtask/src/wit_verify.rs`; both currently `eprintln!("skipping…")` and return when an artifact or `wasm-tools` is absent.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- `crates/slicer-macros/**` is on that guest-WASM input list, so the `build.rs` edit in Step 6 marks guests stale by design. That is expected, not a defect; it must be rebuilt, not explained away.
- **The canonical file list must be derived, never hardcoded.** If the verifier hardcodes the 20 paths and the audit test compares against that same constant, AC-1 becomes tautological. The list is produced by parsing `crates/slicer-macros/src/lib.rs` for `include_str!` targets ending in `.wit`, multiline-aware, and `crates/slicer-schema/wit/root.wit` is filtered out because the macro does not embed it.
- **Declaration bodies are ABI-ordered.** `wit_parser` preserves record field order and variant case order; the rendering used as a comparison key must preserve them too. Sorting is permitted only for the *set* of declarations within an interface and for `use` targets.
- **Fail closed on infrastructure.** No path in the verifier may convert "could not read / could not parse / nothing found" into a clean result. Every such condition is a `VerifyError`.
- **No new dependency version enters the graph.** `wit-parser = "0.247"` must match the string already in `crates/slicer-runtime/Cargo.toml` and `crates/slicer-wasm-host/Cargo.toml`; verify with `rg -n 'wit-parser' crates/*/Cargo.toml` before editing `xtask/Cargo.toml`.
- **No public schema/version constant is bumped by this packet**, so the schema-constant locking rule does not apply here; the fingerprint version prefix (`v1-` → `v2-`) belongs to packet 230.
- `xtask` has no `[lib]` and no `xtask/tests/` directory. All tests are `#[cfg(test)] mod tests` inside `xtask/src/*.rs`, so every AC command is `cargo test -p xtask <path::to::test> -- --exact`. There is no aggregator `mod` registration to add.

## Code Change Surface

### Selected approach

Parse both sides with `wit_parser` into one `WorldModel`, then compare with a per-package policy.

Net-new public surface in `xtask/src/wit_verify.rs` (packet 230 consumes all of it):

- `pub struct WorldModel { pub packages: BTreeMap<String, PackageModel> }` — key is the package identifier **with** its version suffix as spelled (e.g. `slicer:layer-infill@1.0.0`, `slicer:types`). `pub fn is_empty(&self) -> bool`; `pub fn package_names(&self) -> Vec<&str>`.
- `pub struct PackageModel { pub interfaces: BTreeMap<String, InterfaceModel>, pub worlds: BTreeMap<String, InterfaceModel> }`.
- `pub struct InterfaceModel { pub decls: BTreeMap<String, String>, pub uses: BTreeSet<String> }` — `decls` maps declaration name → order-preserving rendered body covering types, aliases, resources (including their methods), and functions. The rendered body is the comparison key; sorting inside a body is forbidden.
- `pub struct StageExpectation { pub stage_id: String, pub wit_package: String, pub wit_interface: String, pub wit_dir: String, pub qualified_export: String }` plus `pub fn stage_expectation(stage_id: &str) -> Option<StageExpectation>`, built from `slicer_schema::STAGES` via `package_for_stage_id`, `interface_for_stage_id`, `wit_dir_for_stage_id` and `qualified_export_for_stage_id`. Returns `None` for a stage whose `wit_package` is empty.
- `pub enum DriftKind { MissingDeclaration, ExtraDeclaration, DeclarationBody, MissingUse, ExtraUse, UnexpectedPackage, MissingStagePackage, ExportName }` and `pub struct Drift { pub kind: DriftKind, pub package: String, pub interface: Option<String>, pub name: String, pub canonical: Option<String>, pub embedded: Option<String> }` with a `Display` impl printing one line that never contains the substring `STALE:` (packet 230's output contract depends on this).
- `pub const SHARED_PACKAGES: [&str; 5] = ["slicer:types", "slicer:config", "slicer:ir-handles", "slicer:common", "slicer:prepass-types"];` and `pub const ROOT_COMPONENT_PACKAGE: &str = "root:component";`.
- `pub fn macro_embedded_wit_files(ws_root: &Path) -> Result<Vec<PathBuf>, VerifyError>` — the derived 20-file canonical list.
- `pub fn canonical_world_model(ws_root: &Path, stage: Option<&StageExpectation>) -> Result<WorldModel, VerifyError>`.
- `pub fn embedded_world_model(artifact: &Path) -> Result<WorldModel, VerifyError>` — `embedded_wit_text` (retained unchanged) then `wit_parser`.
- `pub fn compare_worlds(embedded: &WorldModel, canonical: &WorldModel, expect: Option<&StageExpectation>) -> Vec<Drift>`.
- `pub fn verify_embedded_world(artifact: &Path, canonical: &WorldModel, expect: Option<&StageExpectation>) -> Result<Vec<Drift>, VerifyError>` — same name, new signature, so `build_one`'s call site changes shape but not intent.
- `VerifyError` gains `CanonicalEmpty`, `CanonicalUnreadable { path: String, reason: String }` and `Parse { artifact: String, reason: String }`; `Decode` is retained.

Deleted (this is the exact list AC-13 greps): `TypeMismatch` — whose only field-level consumer is `BuildError::StaleEmbeddedWorld`'s `mismatches` field, retyped to `Vec<Drift>` in the same step — `extract_type_blocks`, `canonical_type_blocks`, `matching_brace`, `strip_comments`, `normalize`, `ambiguous_type_names`, `BRACED_KEYWORDS`, and the tests that only exercise them (`extracts_variant_body_by_name`, `normalization_ignores_formatting_and_comments`, `detects_missing_variant_case`, `keyword_must_stand_alone`, `nested_braces_are_balanced`, `canonical_wit_yields_types_including_extrusion_role`, `stage_package_declarations_shadow_shared_ones`, `unknown_stage_drops_ambiguous_names_but_keeps_the_rest`). `module_stage_wit_dir` and `core_modules_resolve_their_stage_wit_dir` are **retained** — packet 230 retires them.

### Comparison policy (one function, three branches)

1. Every embedded package name must be in `{ROOT_COMPONENT_PACKAGE}` ∪ `SHARED_PACKAGES` ∪ `{expect.wit_package}` (version-stripped comparison for membership, exact comparison for the export name). Otherwise `DriftKind::UnexpectedPackage`.
2. Stage package: the interface named `expect.wit_interface` is compared with full equality both directions — `MissingDeclaration`, `ExtraDeclaration`, `DeclarationBody`, `MissingUse`, `ExtraUse`. Absence of the stage package entirely is `MissingStagePackage`. Every **other** interface of the stage package is compared subset-direction.
3. Shared packages: subset direction only — for each embedded declaration, drift iff canonical declares the same name with a different body, or canonical does not declare it at all (`ExtraDeclaration`). Whole-member omission by the artifact is never drift. `uses` compared as sets, subset direction.
4. Export name: `expect.qualified_export` compared byte-exactly against the embedded world's export, version suffix included → `DriftKind::ExportName`.

### Rejected alternatives

- **Keep the scanner and extend it to aliases/resources/uses.** Rejected: it would re-derive a WIT parser by hand, keep the byte-vs-char index bug class alive (R5-11), and still lack package qualification. `wit_parser` is already in `Cargo.lock`.
- **Parse canonical with `wit_parser` but keep the scanner for the decoded side.** Rejected explicitly by the user ruling in the approved plan: both sides parse with `wit_parser`; no hand-rolled WIT parsing survives.
- **Use `wasm-tools component wit --json`.** Rejected: it pins the verifier to a `wasm-tools` output schema the repo does not control, whereas `wit_parser` is a versioned crate dependency shared with `slicer-runtime`.
- **Normalize declaration bodies by sorting fields/cases.** Rejected: R5-8 — field and case order is ABI-relevant, and sorting would hide exactly the drift class the verifier exists to catch.

## Files in Scope (read + edit)

Four files; the extra two beyond the recommended three are small, mechanical, and inseparable from the change (a call site that will not compile otherwise, and a one-list manifest edit).

- `xtask/src/wit_verify.rs` — role: the verifier itself; expected change: full rewrite of the model, comparison and error type, plus a new test module.
- `xtask/src/build_guests.rs` — role: the sole consumer; expected change: `build_one`'s verification block migrated to the new API; one new `BuildError` variant for the canonical-unusable class; and `BuildError::StaleEmbeddedWorld`'s existing `mismatches: Vec<crate::wit_verify::TypeMismatch>` field retyped to `Vec<crate::wit_verify::Drift>`, with its `Display` arm updated (`TypeMismatch` is on the deletion list, so this is a mandatory edit to an existing variant, not only an addition). **No** edit to `check_command`, `is_stale`, the fingerprint functions, or `build_one_inner`.
- `crates/slicer-macros/build.rs` — role: macro rebuild trigger; expected change: `rerun-if-changed` list replaced by the same 20-file set the verifier audits.
- `xtask/Cargo.toml` — role: dependency declaration; expected change: one added line, `wit-parser = "0.247"`.

## Read-Only Context

- `xtask/src/build_guests.rs` — long; ranged reads only. Read only `GuestSpec` (fields `crate_name`, `lib_name`, `manifest_path`, `guest_dir`, `artifact_path`, `tree`, `stage_id`), `BuildError`, and `build_one`. Purpose: the exact call-site shape to migrate.
- `crates/slicer-schema/src/lib.rs` — over 600 lines; read only the `StageSpec` definition and the `wit_dir_for_stage_id` / `package_for_stage_id` / `interface_for_stage_id` / `qualified_export_for_stage_id` bodies. Purpose: build `StageExpectation` from real fields. Fact already established: `STAGES` has 16 rows and the `PrePass::PaintSegmentation` row has `wit_package: ""`.
- `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs` — read only the `Resolve` / `UnresolvedPackageGroup::parse` call sites. Purpose: the in-tree `wit_parser` usage pattern.
- `crates/slicer-schema/wit/deps/types.wit` and one per-stage file (e.g. `deps/finalization-layer-finalization/finalization-layer-finalization.wit`) — purpose: real declaration names (`region-key`, `extrusion-role`, `layer-idx`, `layer-collection-view`) for the fixtures. Do not read the other 19.
- `docs/03_wit_and_manifest.md` — section "Build & Freshness Contract (Normative)" only.

## Out-of-Bounds Files

- `crates/slicer-macros/src/lib.rs` — very large; never load. Extract the `include_str!` set by dispatch or scripted regex only. (Its `build.rs` sibling *is* in scope.)
- Every other packet directory under `docs/spec_packets/**` — never edit; user-ruled out of scope for this plan.
- `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs` — packet 230's surface; do not edit here.
- `.github/workflows/ci.yml`, `CLAUDE.md`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `docs/adr/**`, `.claude/skills/**` — packet 232's surface.
- `crates/pnp-cli-locator/**` — packet 231's surface.
- `target/`, `Cargo.lock`, `modules/core-modules/*/*.wasm` (binary), `crates/slicer-wasm-host/test-guests/**/*.wasm` — never load; artifacts are inputs to `wasm-tools`, not reading material.
- `OrcaSlicerDocumented/` — not applicable; this packet has no parity surface.

## Expected Sub-Agent Dispatches

- Question: "List every distinct `.wit` path passed to `include_str!` in `crates/slicer-macros/src/lib.rs`, multiline-aware, sorted and deduped, with the count."; scope: `crates/slicer-macros/src/lib.rs`; return: `FACT` (count + paths); purpose: Step 2 and AC-1's expected set.
- Question: "Exact declaration text of `record region-key`, `variant extrusion-role`, the `layer-idx` alias, and `resource layer-collection-view` in the canonical WIT, with the owning file."; scope: `crates/slicer-schema/wit/**`; return: `SNIPPETS` (<=3, <=30 lines each); purpose: Step 4 fixtures.
- Question: "Does `cargo check --workspace --all-targets` pass after the `wit-parser` addition and the `wit_verify` rewrite? If not, first 20 lines of the first error."; scope: workspace; return: `FACT pass/fail` + bounded `SNIPPETS`; purpose: Steps 2-6 verification.
- Question: "Decode `modules/core-modules/wipe-tower/wipe-tower.wasm` with `wasm-tools component wit` and report only the package declaration lines and the export line."; scope: that artifact; return: `FACT` (<=10 lines); purpose: Step 7 real-artifact assertions, without loading the whole decode.

## Data and Contract Notes

- IR/manifest contracts: none changed. Module manifests are read only through the retained `module_stage_wit_dir`.
- WIT boundary: no canonical `.wit` file is edited. The packet changes only how those files are read; the WIT/Type Changes Checklist in `CLAUDE.md` is therefore not triggered on the WIT side. It **is** triggered on the `crates/slicer-macros/**` side purely as a guest-staleness consequence.
- Determinism/scheduler constraints: `WorldModel` uses `BTreeMap`/`BTreeSet` throughout so the drift list is deterministic and diffable across runs; packet 230's per-guest reporting depends on that determinism.
- Version handling: package membership is decided on the version-stripped name; the export-name comparison is exact including version. These are deliberately different and must not be unified.

## Locked Assumptions and Invariants

- The allowed embedded-package set is exactly `root:component` ∪ the 5 shared packages ∪ the resolved stage package. This is fail-closed by intent: a genuinely new shared package requires editing `SHARED_PACKAGES` in the same commit that introduces it.
- `crates/slicer-schema/wit/root.wit` is never part of the canonical model set, because the macro does not `include_str!` it.
- Declaration body rendering preserves source order of record fields and variant cases, permanently. A future "normalize for readability" change would silently defeat AC-3/AC-4.
- `Drift`'s `Display` output never contains the substring `STALE:` — packet 230's reporting contract puts the reason on a second line that must not be mistaken for a stale marker.
- `module_stage_wit_dir` survives this packet unchanged, so `build_one`'s behaviour for a never-built or manifest-less guest is unchanged here.

## Risks and Tradeoffs

- **`wit_parser` may reject the decoded text of some artifact that the old scanner tolerated.** Mitigation: AC-11 runs the real prepass and finalization artifacts through the full path before the packet closes; `AC-N3` pins that a parse failure is an error, not a pass. If a real artifact fails to parse, that is a finding to report, not a reason to loosen the model.
- **Fail-closed loading could break `build_one` in an environment with a partial checkout.** Accepted deliberately: R5-7 rules that an unreadable canonical set is an infrastructure error. The new `BuildError` variant makes the cause explicit rather than silently passing.
- **The `crates/slicer-macros/build.rs` fix marks all 42 guests stale on first run after the edit** (21 core-module guests plus 21 test guests; `crates/slicer-wasm-host/test-guests/witness/` is the one test-guest directory `discover_guests` skips, having no cdylib). This is the correct behaviour finally arriving (16 of 20 embedded files previously triggered no rebuild); it costs one full guest rebuild.
- **Extra/missing-declaration full equality on the exported interface may surface pre-existing drift** in an artifact that has silently disagreed with canonical. If so, the artifact is rebuilt — the finding is real, and must not be worked around by weakening the comparison.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4, the comparison engine and its fixtures)
- Highest-risk dispatch and required return format: the decoded-artifact inspection for Step 7 — it must return `FACT` with at most 10 lines (package declarations and export line only), never the full `wasm-tools component wit` output.

## Open Questions

- `[FWD]` `xtask` tests never run in CI (`.github/workflows/ci.yml`'s `test` job runs `-p slicer-runtime`, `-p pnp-cli` and `-p slicer-helpers` only), so every AC in this packet is green locally and dead in CI until packet 232 adds `cargo test -p xtask`. This packet deliberately does not touch CI; the implementer must run the AC commands locally and must not assume CI coverage.
- `[FWD]` `build_one`'s `if canonical.is_empty() { return Ok(()) }` guard becomes unreachable once `canonical_world_model` returns `Err(VerifyError::CanonicalEmpty)` instead of an empty map. It is deliberately left in place here (packet 230 deletes it as part of the fail-open retirement) — the implementer must keep it compiling against `WorldModel::is_empty`, not silently delete it and desynchronize packet 230's step list.
