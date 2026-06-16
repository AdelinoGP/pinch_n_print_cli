# Design: pnp-cli-unification

## Controlling Code Paths

- Primary code path: the `HostCommands::Run` match arm in `crates/slicer-host/src/main.rs:285-741` (≈456 LOC of inline orchestration) is extracted into `crates/slicer-runtime/src/run.rs::run_slice(opts: SliceRunOptions) -> Result<SliceOutcome, SliceRunError>`. The 8 synthetic `LoadedModule` rows currently constructed inline at `main.rs:432-499` move into the 6 writer modules (`mesh_analysis.rs`, `region_mapping.rs`, `prepass_slice.rs`, `support_geometry.rs`, `paint_segmentation.rs`, `gcode_emit.rs`) as `BuiltinProducer` constants. The `DagValidationRequest` consumer in `crates/slicer-host/src/dag.rs:128` (`build_intra_stage_dag`) and the four `dag_cli::run_dag_*` functions (`crates/slicer-host/src/dag_cli.rs:182,213,247,275`) change signature to accept `&[&dyn Producer]`.
- Neighboring tests or fixtures: 4 `cargo_bin`-using test files (`e2e_integration_tdd.rs`, `helpers_cli.rs`, `cli_tdd.rs`, `module_search_path_tdd.rs` under `crates/slicer-host/tests/`) migrate to `crates/pnp-cli/tests/` and switch to the `pnp_cli` binary + noun-namespaced verbs. The remaining `crates/slicer-host/tests/` files (library tests, unit tests) stay in place after the crate rename — they only need `slicer_host::` → `slicer_runtime::` substitutions.
- OrcaSlicer comparison surface: none — this packet has no OrcaSlicer parity work.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The crate rename `slicer-host` → `slicer-runtime` must land atomically (step 1). A partial rename leaves the workspace uncompilable. Use `cargo check --workspace` as the gate before proceeding past step 1.
- The library API extracted in step 4 (`run_slice`) MUST be byte-deterministic with the pre-refactor `HostCommands::Run` arm given identical inputs. The 4-way instrumentation fork is moved INSIDE `run_slice` as a private helper; the call site composition (Report-only / Progress-only / Composite / Noop) is unchanged.
- The `Producer` trait surface is the smallest projection of `LoadedModule` that `validate_startup_dag`, `build_intra_stage_dag`, and the 4 `dag_cli::run_dag_*` functions all need to consume. Adding fields to the trait surface that aren't read by any of these is forbidden — it expands the seam without callers.
- Manifest TOML schema, IR field paths, WIT world strings, scheduler claim semantics, and `wasm-tools component new` invocation shape are NOT touched. `slicer-schema`'s validator constants are pure values relocated from `cli/slicer-cli/src/cmd_validate.rs`; the values must match exactly.

## Code Change Surface

- Selected approach: incremental refactor sequenced as a single packet — (1) rename crate atomically, (2) define `Producer` trait, (3) move synthetic rows into writer modules + update DAG consumers, (4) extract `run_slice` + delete dead mod, (5) move validator constants to `slicer-schema`, (6) create `pnp-cli` crate with verb dispatch + migrate CLI tests, (7) port `cmd_new` + scaffold extensions, (8) delete `cli/slicer-cli`, (9) remove `slicer-host` binary target, (10) doc/CI/skill sweep, (11) gate.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **New**: `Producer` trait + `BuiltinProducer` adapter in `crates/slicer-runtime/src/dag.rs`; `runtime_builtins()` registry in `crates/slicer-runtime/src/lib.rs`; `slicer_runtime::run::run_slice` (`crates/slicer-runtime/src/run.rs`); `SliceRunOptions` (renamed from `HostRunOptions`) and `SliceOutcome`/`SliceRunError` types; `MESH_PRODUCER`, `MESH_ANALYSIS_PRODUCER`, `REGION_MAPPING_PRODUCER`, `SLICE_PRODUCER`, `SHELL_CLASSIFICATION_PRODUCER`, `SUPPORT_GEOMETRY_PRODUCER`, `PAINT_SEGMENTATION_PRODUCER`, `GCODE_EMIT_PRODUCER` constants in their owning modules.
  - **Changed signatures**: `build_intra_stage_dag`, `validate_startup_dag` (via `DagValidationRequest.modules` field type), `dag_cli::run_dag_stages`, `run_dag_stage`, `run_dag_depends`, `run_dag_claims` — all accept `&[&dyn Producer]` or `&[Box<dyn Producer>]`.
  - **Moved**: `VALID_STAGES`, `SUPPORTED_WIT_WORLDS`, `VALID_CONFIG_TYPES`, `RECOGNIZED_CLAIMS`, `VALID_SEVERITIES` → `crates/slicer-schema/src/lib.rs`. `cmd_new.rs` → `crates/pnp-cli/src/module_new.rs`. Tests `e2e_integration_tdd.rs`, `helpers_cli.rs`, `cli_tdd.rs`, `module_search_path_tdd.rs` → `crates/pnp-cli/tests/`.
  - **Deleted**: `cli/slicer-cli/` (entire directory); `crates/slicer-runtime/src/main.rs` (after step 4 the function body is empty; step 9 deletes the file); `_stale_build_plan` mod (deleted as part of step 4 in the same edit as `run_slice` extraction); `[[bin]] name = "slicer-host"` target in `Cargo.toml`.
  - **New tests**: `crates/slicer-runtime/tests/run_slice_api_tdd.rs` (AC-3), `crates/slicer-runtime/tests/builtin_producers_tdd.rs` (AC-4), `crates/pnp-cli/tests/module_new_tdd.rs::emits_cargo_config_alias` (AC-10). Migrated CLI tests in `crates/pnp-cli/tests/` retain their existing assertions, only the `cargo_bin` target name and verb shape change.
- Rejected alternatives:
  - Keeping `pnp_cli` as a second `[[bin]]` inside `slicer-host` crate (plan Q1 alternative 1): the crate name remains misleading post-merge. Rejected per plan Q1 in favour of clean two-crate split.
  - Splitting Packet 1 into a refactor packet + a merge packet (plan Q3 alternative): rejected because user is sole reviewer landing back-to-back, so the smaller acceptance ceremonies of a 3-way split don't pay for the metadata overhead.
  - Narrow `Producer` seam (validator-only, leave `dag_cli` on `&[LoadedModule]` — plan Q4 alternative): rejected because today's `dag claims/depends` gap (host built-ins invisible) closes naturally for one dispatcher line of cost.

## Files in Scope (read + edit)

The implementer reads and edits many files because the crate-rename sweep touches ~38 source files mechanically. The atomic-step contract is satisfied: each step except step 1 (the rename) edits ≤ 3 primary files. Step 1's "edit" set is the entire `crates/slicer-host/` directory tree, but the change is a single mechanical operation per file (path move + `slicer_host::` → `slicer_runtime::` substitution); the implementer must not make semantic edits during step 1.

- `crates/slicer-host/Cargo.toml` → `crates/slicer-runtime/Cargo.toml` — rename `name`, drop the `slicer-host` `[[bin]]` target in step 9.
- `crates/slicer-host/src/main.rs` → `crates/slicer-runtime/src/main.rs` (transient — deleted in step 9). Primary edit site for steps 3 (synthetic-row block), 4 (`run_slice` extract + `_stale_build_plan` delete).
- `crates/slicer-host/src/dag.rs` → `crates/slicer-runtime/src/dag.rs` — `Producer` trait + `BuiltinProducer` adapter (step 2), then signature changes for `build_intra_stage_dag` (step 3).
- `crates/slicer-host/src/dag_cli.rs` → `crates/slicer-runtime/src/dag_cli.rs` — signature changes for `run_dag_*` (step 3); existing tests in this file update accordingly.
- `crates/slicer-host/src/{mesh_analysis,region_mapping,prepass_slice,support_geometry,paint_segmentation,gcode_emit}.rs` → `crates/slicer-runtime/src/…` — declare `const BUILTIN_PRODUCER` exports (step 3).
- `crates/slicer-host/src/lib.rs` → `crates/slicer-runtime/src/lib.rs` — add `runtime_builtins()` aggregator (step 3); re-export `run_slice` (step 4).
- `crates/slicer-host/src/cli.rs` → `crates/slicer-runtime/src/cli.rs` — `HostRunOptions` → `SliceRunOptions` rename (step 4). The clap parser definitions stay in this file for use by `pnp-cli`'s dispatcher.
- `crates/slicer-host/src/manifest.rs` → `crates/slicer-runtime/src/manifest.rs` — import validator constants from `slicer-schema` (step 5).
- `crates/slicer-schema/src/lib.rs` — add `pub const VALID_STAGES` etc. (step 5).
- `Cargo.toml` (workspace root) — `members` list: replace `crates/slicer-host` with `crates/slicer-runtime`; drop `cli/slicer-cli`; add `crates/pnp-cli` (step 1, 6, 8).
- `crates/pnp-cli/Cargo.toml` — new file, dependencies + dev-dependencies + `[[bin]]` (step 6).
- `crates/pnp-cli/src/main.rs` — new file, clap dispatcher (step 6).
- `crates/pnp-cli/src/module_new.rs` — new file, ported from `cmd_new.rs` (step 7); extended with `.cargo/config.toml` + `README.md` emission.
- `crates/pnp-cli/tests/{e2e_integration_tdd,helpers_cli,cli_tdd,module_search_path_tdd,module_new_tdd}.rs` — migrated/new tests (steps 6, 7).
- `cli/slicer-cli/` (entire directory) — deleted (step 8).
- `.github/workflows/ci.yml` — crate-name updates (step 10).
- `CLAUDE.md`, `docs/00`, `docs/05`, `docs/13`, `docs/16`, `docs/17` — doc-sweep edits (step 10).
- `.claude/skills/**/*.md`, `.agents/skills/**/*.md`, `.claude/agents/**/*.md` — living-skill rename (step 10).

## Read-Only Context

- `crates/slicer-host/src/main.rs` (~925 lines) — range-read only:
  - lines 282–741: the `HostCommands::Run` arm (extract target for step 4).
  - lines 432–499: the synthetic `host_builtin(…)` block (move target for step 3).
  - lines 813–924: the `_stale_build_plan` dead mod (delete target).
  - lines 162–212: the `run_dag_command` function and supporting helpers — read to understand current dispatcher shape before re-implementing it under `pnp-cli` (step 6).
- `crates/slicer-host/src/dag.rs` (~300 lines) — read lines 30–100 (LoadedModule struct + `build_intra_stage_dag` signature) and 128–170 (`build_intra_stage_dag` body) for step 2's trait surface design.
- `crates/slicer-host/src/dag_cli.rs` (~600 lines) — read lines 180–290 (the 4 `run_dag_*` signatures) for step 3's signature changes; existing tests at lines 400–600 are the reference for the migration tests.
- `cli/slicer-cli/src/cmd_validate.rs` (~980 lines) — read lines 11–66 (the duplicate constants) for step 5; the rest of the file is tests + tested validators (these die with the crate).
- `cli/slicer-cli/src/cmd_new.rs` (~400 lines, estimated) — read in full for step 7 port; this is the only `cli/slicer-cli/` source surviving the deletion.
- `crates/slicer-host/Cargo.toml` (~80 lines) — load directly; the `[[bin]]` removal in step 9 targets lines 7–9.
- `crates/slicer-host/tests/{e2e_integration_tdd,helpers_cli,cli_tdd,module_search_path_tdd}.rs` — load each test file directly when migrating it in step 6 (each is < 300 lines per prior count).
- `docs/13_slicer_helpers_crate.md` — read only §"Integration with Host CLI" (lines ~504–540) for step 10's verb-name rename.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — foreign tree; no parity work in this packet.
- `target/`, `Cargo.lock`, any generated `.wasm` under `modules/core-modules/**/wit-guest/target/` — never load.
- The other ~30 `.rs` files under `crates/slicer-host/src/` (everything except the 8 modules explicitly listed in §"Files in Scope") — for the rename sweep, do NOT browse them; rely on `cargo check` output (delegated as SNIPPETS) to find missed `slicer_host::` sites.
- `.ralph/specs/_OLD/**` and any `status: implemented` packet under `.ralph/specs/` — read-only AND edit-forbidden (cross-packet mutation rule).
- `modules/core-modules/**` — these are guest crates; this packet does not edit them. Stale-guest behaviour is documented in `CLAUDE.md`; if a slice smoke fails, run `./modules/core-modules/build-core-modules.sh --check` before debugging.
- `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` — not touched in this packet; delegate any fact-check (none expected).

## Expected Sub-Agent Dispatches

- "Run `cargo check --workspace` after the rename in step 1; return SNIPPETS of the first 5 distinct compile errors, OR FACT pass if green." — purpose: catch missed `slicer_host::` substitutions without browsing all ~38 files.
- "Run `cargo test -p slicer-runtime --test builtin_producers_tdd`; return FACT pass/fail." — purpose: AC-4 verification.
- "Run `cargo test -p slicer-runtime --test run_slice_api_tdd`; return FACT pass/fail." — purpose: AC-3 verification.
- "Run `cargo run --release --bin pnp_cli -- dag claims --module-dir modules/core-modules --no-default-module-paths`; return SNIPPETS of the JSON output truncated to the first 30 lines." — purpose: AC-5 verification + sanity-check the dag_cli output shape.
- "Append a TASK-213 entry to `docs/07_implementation_status.md` under the current Phase section, using the existing tick-box format. The entry text: 'TASK-213 Merge slicer-host and slicer-cli into a single pnp_cli binary; rename slicer-host crate → slicer-runtime (library only); extract run_slice() library API; externalise host built-ins onto Producer trait reaching dag_cli; consolidate manifest validator constants into slicer-schema. **Closed YYYY-MM-DD via packet 69_pnp-cli-unification.**'. Return FACT done." — purpose: backlog book-keeping at packet close.
- "Run `grep -rln 'slicer-host\|slicer-cli\|\\bslicer\\b' .claude/skills/ .agents/skills/ .claude/agents/`; return LOCATIONS of every match for the step-10 living-skill sweep." — purpose: enumerate living skill files needing the binary-name rename.
- "Summarize `docs/05_module_sdk.md`'s 'Developer CLI' section; return SUMMARY ≤ 200 words noting every binary invocation and verb name." — purpose: locate the rename targets without loading the 700-line file.

## Data and Contract Notes

- **IR contracts**: unchanged. The 8 synthetic-row IR write paths (`MeshIR`, `SurfaceClassificationIR`, `RegionMapIR`, `SliceIR`, `SupportGeometryIR`, `PaintRegionIR`, `GCodeIR`) are preserved verbatim as the `ir_writes` field on the corresponding `BuiltinProducer` constants — these are observable in `pnp_cli dag claims` JSON.
- **WIT contracts**: unchanged. No `wit/**/*.wit` file is edited in this packet.
- **Manifest schema**: unchanged. The validator constants relocate but the values are identical. `slicer-schema`'s public API gains the constants but no other shape change.
- **Scheduler semantics**: unchanged. The DAG validator's input projection narrows (`LoadedModule` → `&dyn Producer`), but every field it actually reads is preserved, including the IR-schema-version compat fields used by `IrVersionCompatibility` pass.
- **Determinism**: `run_slice()` is byte-deterministic with the pre-refactor `HostCommands::Run` arm for identical `SliceRunOptions`. AC-3's test asserts non-empty output; the AC-2 smoke against benchy.stl provides the regression signal. If a downstream snapshot diff appears, the rename or the synthetic-row move dropped a field the validator reads.

## Locked Assumptions and Invariants

- `crates/slicer-host` has zero external library importers in the workspace (verified by `grep -rln 'slicer-host =\|use slicer_host::' --include='Cargo.toml' --include='*.rs'` returning only internal hits). The rename does not need to ripple outside the crate.
- The `Producer` trait's smallest valid surface is exactly `(id: &str, stage: &str, ir_writes: &[String], ir_reads: &[String], claims_holds: &[String], claims_requires: &[String], requires_modules: &[String], min_ir_schema: SemVer, max_ir_schema: SemVer)`. Any caller needing more is using a `LoadedModule`-specific field that doesn't apply to host built-ins (e.g., `wasm_path`) and must not be part of this trait.
- The 4-way instrumentation fork composition (`Composite(progress, report)` / `report-only` / `progress-only` / `noop`) is invariant — `run_slice` reproduces it inside its body. If the implementer notices a 5th case during the move, they have introduced a regression — stop and reconcile.
- `pnp_cli`'s clap definitions and dispatcher logic depend on `slicer-runtime` exporting `cli::HostCli` / `HostCommands` / `DagSubcommand` / `OutputFormat` / `HostRunOptions` (renamed `SliceRunOptions`) verbatim. The runtime crate must keep these public.
- `.cargo/config.toml` aliases generated by `pnp_cli module new` are scoped to the new module's directory; they do not interact with the workspace's own `.cargo/config.toml` (which is empty today — verified by `Glob`).

## Risks and Tradeoffs

- **Rename atomicity**: a partial rename in step 1 leaves the workspace uncompilable. Mitigation: run `cargo check --workspace` as the step-1 gate (delegated); do not progress past step 1 until green.
- **Producer trait shape drift**: the trait projection might be slightly off (a field needed by `validate_startup_dag::IrVersionCompatibility` that I missed). Mitigation: AC-3's `run_slice_api_tdd` exercises the validator end-to-end against benchy modules; a mis-projection surfaces as a test failure.
- **dag_cli signature change blast radius**: the 4 `run_dag_*` signature changes touch existing tests in `dag_cli.rs:400-600`. Mitigation: each test gets updated alongside the signature change in step 3; the test count stays the same.
- **CI gate ordering**: the doc/CI sweep (step 10) updates `.github/workflows/ci.yml`. If the implementer pushes the PR before step 10 lands, CI fails with `cargo test -p slicer-host: package not found`. Mitigation: step 11 (the gate) runs locally before push; step 10 lands before step 11.
- **Living-skill sweep scope creep**: the LOCATIONS dispatch in step 10 might return many matches in skill files. Mitigation: dispatch with a max-line bound (`head_limit: 50`) and triage; only edit files that emit CLI invocations.

## Context Cost Estimate

- Aggregate (sum across all steps): `M` — 11 steps, mix of S and M, no L; the rename sweep (step 1) is the largest single budget item but is bounded by relying on `cargo check` output rather than browsing.
- Largest single step: `M` (steps 1, 3, 4, 6 are all M-class).
- Highest-risk dispatch: the post-rename `cargo check --workspace` (step 1). If the implementer asks for the full output instead of the first-5-distinct-errors SNIPPETS, the return blows the context budget on a noisy rename failure. Required return format: SNIPPETS ≤ 20 lines of the first 5 distinct compile errors.

## Open Questions

- `[FWD]` Step 4's `run_slice` signature: does the function return `Result<SliceOutcome, SliceRunError>` where `SliceOutcome` is `{ gcode_text: String, layer_count: u32, wallclock_ms: u64 }`, or just `Result<String, SliceRunError>` (the gcode_text alone)? The implementer chooses based on what AC-3's test needs and what `pnp_cli slice` displays at the end. The plan-mode discussion noted this as implementer-detail; either choice is defensible.
- `[FWD]` Step 6's clap structure: does `pnp_cli module new` use the same clap derive as today's `slicer new`, or a fresh inline `clap::Subcommand` definition? Either works; the implementer picks the lower-line-count option.
- `[FWD]` Step 7's scaffolded `README.md` content: a few lines of "post-build wasm-tools incantation" is enough. The exact wording is the implementer's choice as long as AC-10's test passes.
