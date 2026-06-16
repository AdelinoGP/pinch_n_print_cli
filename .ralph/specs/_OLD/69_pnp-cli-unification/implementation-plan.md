# Implementation Plan: pnp-cli-unification

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-213` (this packet's sole task ID; sub-step granularity is documented per-step but the backlog entry is single).
- TDD first for new behaviour (steps 3, 4, 7), then implementation, then narrow falsifying validation.
- Each step honors the context-discipline preamble. The fields below are the budget contract — not optional metadata.

## Steps

### Step 1: Rename crate `slicer-host` → `slicer-runtime`

- Task IDs:
  - `TASK-213`
- Objective: Move `crates/slicer-host/` → `crates/slicer-runtime/`; update the crate's own `Cargo.toml` `name`; update workspace `Cargo.toml` `members`; substitute every internal `slicer_host::` reference with `slicer_runtime::`. No semantic changes in this step.
- Precondition: `cargo check --workspace` green on a clean working tree.
- Postcondition: `cargo check --workspace` green; `crates/slicer-host/` does not exist; `crates/slicer-runtime/` exists; `grep -rln 'slicer_host::' crates/slicer-runtime/` returns empty.
- Files allowed to read:
  - `Cargo.toml` (workspace root) — load directly (~110 lines)
  - `crates/slicer-host/Cargo.toml` — load directly
- Files allowed to edit (≤ 3 categories; the rename sweep is mechanical per-file):
  - `Cargo.toml` (workspace root) — `members` entry rename
  - `crates/slicer-host/Cargo.toml` → `crates/slicer-runtime/Cargo.toml` — `name = "slicer-runtime"`
  - Every `.rs` file under the renamed directory — substitute `use slicer_host::` / `slicer_host::` → `slicer_runtime` (use editor mass-replace; do not introduce semantic changes)
- Files explicitly out-of-bounds for this step:
  - `cli/slicer-cli/` (step 8 deletes it; it currently imports nothing from `slicer-host`)
  - All other workspace crates (none import `slicer-host` — verified)
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return SNIPPETS of the first 5 distinct compile errors, OR FACT pass." — purpose: catch missed substitutions without browsing 38 files.
- Context cost: `M`
- Authoritative docs:
  - none — pure mechanical rename
- Verification:
  - `test ! -d crates/slicer-host && test -d crates/slicer-runtime` — FACT pass/fail
  - `! grep -rln 'slicer_host::' crates/slicer-runtime/` — FACT empty
  - `cargo check --workspace` — FACT pass/fail
- Exit condition: workspace compiles; no `slicer_host::` residue inside the renamed crate.

### Step 2: Define `Producer` trait + `BuiltinProducer` adapter

- Task IDs:
  - `TASK-213`
- Objective: In `crates/slicer-runtime/src/dag.rs`, define `pub trait Producer { fn id(&self) -> &str; fn stage(&self) -> &str; fn ir_writes(&self) -> &[String]; fn ir_reads(&self) -> &[String]; fn claims_holds(&self) -> &[String]; fn claims_requires(&self) -> &[String]; fn requires_modules(&self) -> &[String]; fn min_ir_schema(&self) -> SemVer; fn max_ir_schema(&self) -> SemVer; }` (exact field set matches `LoadedModule`'s validator-read projection). Implement the trait for `&LoadedModule` (blanket-ish). Define a concrete `pub struct BuiltinProducer { … }` with `const fn`-friendly fields (`&'static str` for id/stage; `&'static [&'static str]` for ir paths/claims). Implement `Producer` for `&BuiltinProducer`.
- Precondition: Step 1 green.
- Postcondition: `Producer` trait callable; existing call sites still compile (no signature changes in this step — those land in step 3).
- Files allowed to read:
  - `crates/slicer-runtime/src/dag.rs` — lines 30–100 (current `LoadedModule` shape + helpers)
  - `crates/slicer-runtime/src/manifest.rs` — `LoadedModule` definition (whichever file owns it post-rename)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/dag.rs` — add trait + adapter
  - `crates/slicer-runtime/src/lib.rs` — re-export `Producer`, `BuiltinProducer`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-runtime/src/dag_cli.rs` — signature changes wait for step 3
  - `crates/slicer-host/src/main.rs` synthetic-row block — move waits for step 3
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --lib dag::`; return FACT pass/fail." — purpose: confirm trait + adapter compile + existing dag tests stay green.
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — delegate a SUMMARY of "validator passes consume which `LoadedModule` fields" if uncertain which fields belong in the trait. Likely not needed if the implementer reads `validate_startup_dag` directly.
- Verification:
  - `cargo test -p slicer-runtime --lib dag::` — FACT pass/fail
- Exit condition: `Producer` trait public, `LoadedModule` and `BuiltinProducer` both implement it; the `cargo doc -p slicer-runtime` (if dispatched) shows both adapters.

### Step 3: Externalise the 8 synthetic host built-ins + broaden the DAG seam

- Task IDs:
  - `TASK-213`
- Objective: Declare `pub const … : BuiltinProducer` in each writer module (`mesh_analysis.rs::MESH_PRODUCER` + `MESH_ANALYSIS_PRODUCER`; `region_mapping.rs::REGION_MAPPING_PRODUCER`; `prepass_slice.rs::SLICE_PRODUCER` + `SHELL_CLASSIFICATION_PRODUCER`; `support_geometry.rs::SUPPORT_GEOMETRY_PRODUCER`; `paint_segmentation.rs::PAINT_SEGMENTATION_PRODUCER`; `gcode_emit.rs::GCODE_EMIT_PRODUCER`). Add `pub fn runtime_builtins() -> Vec<&'static dyn Producer>` (or equivalent slice-returner) in `crates/slicer-runtime/src/lib.rs`. Change signatures: `DagValidationRequest.modules: Vec<&dyn Producer>`, `build_intra_stage_dag(stage: StageId, producers: &[&dyn Producer])`, `dag_cli::run_dag_stages|run_dag_stage|run_dag_depends|run_dag_claims` all take `&[&dyn Producer]`. Delete the inline `host_builtin(…)` constructor block in `crates/slicer-runtime/src/main.rs` (originally `main.rs:432-499`); replace with `dag_modules.extend(runtime_builtins())` followed by `dag_modules.extend(loaded.bindings.iter().map(|b| &b.module as &dyn Producer))`. Write `crates/slicer-runtime/tests/builtin_producers_tdd.rs` asserting the exact 8 `(id, stage, ir_writes)` tuples from AC-4.
- Precondition: Step 2 green.
- Postcondition: `main.rs` no longer contains `host_builtin(`; the new test passes; all existing `cargo test -p slicer-runtime` tests still pass.
- Files allowed to read:
  - `crates/slicer-runtime/src/main.rs` — lines 432–499 (synthetic-row block) and 423–578 (the surrounding context that wires them into `validate_startup_dag`)
  - `crates/slicer-runtime/src/dag_cli.rs` — lines 180–290 (the 4 `run_dag_*` signatures) and 400–600 (existing tests to update)
- Files allowed to edit (≤ 3 per primary surface; the trait-broadening touches multiple files but each edit is small):
  - `crates/slicer-runtime/src/main.rs` — remove synthetic block + restate validator call site
  - `crates/slicer-runtime/src/dag_cli.rs` — signature changes + test updates
  - `crates/slicer-runtime/src/{mesh_analysis,region_mapping,prepass_slice,support_geometry,paint_segmentation,gcode_emit}.rs` — declare `BuiltinProducer` constants
  - `crates/slicer-runtime/src/lib.rs` — add `runtime_builtins()`
  - `crates/slicer-runtime/tests/builtin_producers_tdd.rs` — new TDD test
- Files explicitly out-of-bounds for this step:
  - `pnp-cli` (does not exist yet; the dispatcher in step 6 will call `runtime_builtins()`)
  - `cli/slicer-cli/` (step 8)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test builtin_producers_tdd`; return FACT pass/fail; if fail, return SNIPPETS ≤ 20 lines of the assertion." — purpose: AC-4.
  - "Run `cargo test -p slicer-runtime --lib dag::`; return FACT pass/fail." — purpose: validator + dag_cli tests still green after signature changes.
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — delegate SUMMARY of "13-pass startup DAG validation order" if the implementer is unsure which fields are read by which pass.
- Verification:
  - `cargo test -p slicer-runtime --test builtin_producers_tdd` — FACT pass
  - `cargo test -p slicer-runtime --lib dag::` — FACT pass
  - `! grep -q 'host_builtin(' crates/slicer-runtime/src/main.rs` — FACT empty
- Exit condition: TDD test green; no `host_builtin(` calls in `main.rs`; dag_cli signature is `&[&dyn Producer]`.

### Step 4: Extract `slicer_runtime::run::run_slice()` + delete `_stale_build_plan`

- Task IDs:
  - `TASK-213`
- Objective: Move the body of `HostCommands::Run` (originally `main.rs:285-741`) into a new `crates/slicer-runtime/src/run.rs::run_slice(opts: SliceRunOptions) -> Result<SliceOutcome, SliceRunError>`. Rename `HostRunOptions` → `SliceRunOptions` in `crates/slicer-runtime/src/cli.rs`. Define `SliceOutcome { gcode_text: String, layer_count: u32, wallclock_ms: u64 }` and `SliceRunError` (sum type wrapping the existing inline error paths). Move the 4-way instrumentation fork (originally `main.rs:673-722`) into a private helper inside `run.rs`. Update `main.rs::HostCommands::Run` arm to call `run_slice(opts).map(|outcome| /* write output */)`. Delete the `_stale_build_plan` mod (originally `main.rs:813-924`) as part of this edit. Write `crates/slicer-runtime/tests/run_slice_api_tdd.rs` exercising AC-3.
- Precondition: Step 3 green.
- Postcondition: `run_slice` is callable from `crates/slicer-runtime/tests/`; benchy.stl + core-modules end-to-end through `run_slice` produces a non-empty gcode string; `_stale_build_plan` mod is gone.
- Files allowed to read:
  - `crates/slicer-runtime/src/main.rs` — lines 282–741 (the full Run arm) and 813–924 (the dead mod)
  - `crates/slicer-runtime/src/cli.rs` — lines 251–271 (`HostRunOptions`)
  - `crates/slicer-runtime/src/pipeline.rs` — `run_pipeline_with_*` signatures (whichever exist)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/run.rs` — new file
  - `crates/slicer-runtime/src/main.rs` — shrink the Run arm; delete `_stale_build_plan`
  - `crates/slicer-runtime/src/cli.rs` — rename `HostRunOptions` → `SliceRunOptions`
  - `crates/slicer-runtime/src/lib.rs` — re-export `run::run_slice`, `SliceRunOptions`, `SliceOutcome`, `SliceRunError`
  - `crates/slicer-runtime/tests/run_slice_api_tdd.rs` — new TDD test
- Files explicitly out-of-bounds for this step:
  - `pnp-cli` (still doesn't exist)
  - `crates/slicer-runtime/tests/e2e_integration_tdd.rs` etc. (those migrate in step 6)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test run_slice_api_tdd -- --nocapture`; return FACT pass/fail; if fail, SNIPPETS ≤ 30 lines of the assertion." — purpose: AC-3.
  - "Run `./modules/core-modules/build-core-modules.sh --check`; return FACT pass/fail." — purpose: ensure guest WASMs are fresh before the run_slice TDD test runs benchy through them.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` — delegate a SUMMARY of "what does the Run pipeline do (PrePass → PerLayer → PostPass) and what does main.rs orchestrate" if needed.
- Verification:
  - `cargo test -p slicer-runtime --test run_slice_api_tdd` — FACT pass
  - `! grep -q '_stale_build_plan' crates/slicer-runtime/src/main.rs` — FACT empty
  - `cargo build --workspace --release --bin slicer-host` — FACT pass (still works until step 9 removes it)
- Exit condition: TDD test green; `_stale_build_plan` gone; `slicer-host` binary still functional and routes through `run_slice`.

### Step 5: Consolidate manifest validator constants into `slicer-schema`

- Task IDs:
  - `TASK-213`
- Objective: Move `VALID_STAGES`, `SUPPORTED_WIT_WORLDS`, `VALID_CONFIG_TYPES`, `RECOGNIZED_CLAIMS`, `VALID_SEVERITIES` from `cli/slicer-cli/src/cmd_validate.rs:11-66` into `crates/slicer-schema/src/lib.rs` as `pub const` arrays. Update `crates/slicer-runtime/src/manifest.rs` to import them from `slicer-schema` where it currently has any equivalent (verify by grep — `manifest.rs` may have its own copy of `STAGE_ORDER`; that one stays in `execution_plan.rs` per the canonical list).
- Precondition: Step 4 green.
- Postcondition: `slicer-schema` has the 5 pub consts; `slicer-runtime::manifest` imports from `slicer-schema` for whichever of these it uses; `cli/slicer-cli/src/cmd_validate.rs` still compiles (it imports from `slicer-schema` until step 8 deletes the file).
- Files allowed to read:
  - `cli/slicer-cli/src/cmd_validate.rs` — lines 11–66 (the duplicate constants)
  - `crates/slicer-schema/src/lib.rs` — current public surface
  - `crates/slicer-runtime/src/manifest.rs` — load-validation logic (grep for `STAGE_ORDER`, etc., to understand existing usage)
- Files allowed to edit (≤ 3):
  - `crates/slicer-schema/src/lib.rs` — add the 5 `pub const` arrays
  - `crates/slicer-runtime/src/manifest.rs` — import from `slicer-schema`
  - `cli/slicer-cli/src/cmd_validate.rs` — transitional import from `slicer-schema` (replaces the inline definitions); the file dies in step 8 anyway, so this is for compile-correctness only
- Files explicitly out-of-bounds for this step:
  - All other slicer-cli source — the constants only live in `cmd_validate.rs`
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-schema`; return FACT pass/fail." — purpose: schema crate test pass.
  - "Run `cargo test -p slicer-runtime --lib manifest::`; return FACT pass/fail." — purpose: manifest loader tests still pass.
  - "Run `cargo build -p slicer-cli`; return FACT pass/fail." — purpose: transitional import compiles.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegate SUMMARY of "canonical validator constant list" if uncertain. (Most likely not needed — the values are pure data.)
- Verification:
  - `cargo test -p slicer-schema` — FACT pass
  - `cargo test -p slicer-runtime --lib manifest::` — FACT pass
  - `grep -q 'pub const VALID_STAGES' crates/slicer-schema/src/lib.rs` — FACT pass
- Exit condition: Constants in `slicer-schema`; both consumers compile and pass tests.

### Step 6: Create `crates/pnp-cli/` + dispatch + migrate CLI tests

- Task IDs:
  - `TASK-213`
- Objective: Create `crates/pnp-cli/` workspace member with `Cargo.toml` declaring `[[bin]] name = "pnp_cli"`, deps `slicer-runtime`, `slicer-schema`, `clap`, dev-deps `assert_cmd`, `predicates`, `tempfile`. Author `crates/pnp-cli/src/main.rs` as a thin clap dispatcher over the noun-namespaced verb tree: `slice` → `slicer_runtime::run::run_slice`; `module new|diagnose|config-schema` → local `module_new` (placeholder until step 7) + `slicer_runtime`'s existing `run_diagnose` / `build_config_schema_json`; `mesh repair|decimate|import` → `slicer_runtime::helpers_cmd::run_*`; `dag stages|stage|depends|claims` → `slicer_runtime::dag_cli::run_dag_*` with `runtime_builtins()` injected as producers (AC-5 wiring). Add `crates/pnp-cli` to workspace `Cargo.toml`. Migrate `e2e_integration_tdd.rs`, `helpers_cli.rs`, `cli_tdd.rs`, `module_search_path_tdd.rs` from `crates/slicer-runtime/tests/` to `crates/pnp-cli/tests/`; update each `Command::cargo_bin("slicer-host")` → `cargo_bin("pnp_cli")` and rewrite verb sequences (`["run", …]` → `["slice", …]`; `["repair", …]` → `["mesh", "repair", …]`; `["dag", "stages"]` unchanged; `["diagnose", …]` → `["module", "diagnose", …]`; `["config-schema", …]` → `["module", "config-schema", …]`).
- Precondition: Step 5 green; `slicer-runtime` exports `run_slice`, `runtime_builtins`, and the rest of the dispatcher targets.
- Postcondition: `pnp_cli` binary builds; all 4 migrated tests pass against the new binary; `slicer-host` binary still builds (step 9 removes it).
- Files allowed to read:
  - `crates/slicer-runtime/src/cli.rs` — the existing clap derive (this might re-export to `pnp-cli` or be re-implemented inline)
  - `crates/slicer-runtime/src/main.rs::run_dag_command` — lines 162–212 (current dispatcher shape)
  - The 4 source test files in `crates/slicer-runtime/tests/` listed above — load each in full when migrating it (each < 300 lines per probe)
- Files allowed to edit (≤ 3 per atomic sub-action; the step has multiple sub-actions but each touches a small primary file):
  - `crates/pnp-cli/Cargo.toml` (new)
  - `crates/pnp-cli/src/main.rs` (new)
  - `Cargo.toml` workspace root — add `crates/pnp-cli` member
  - `crates/pnp-cli/tests/{e2e_integration_tdd,helpers_cli,cli_tdd,module_search_path_tdd}.rs` — migrated copies
- Files explicitly out-of-bounds for this step:
  - `cli/slicer-cli/` (step 8 deletes; do not edit here)
  - `crates/slicer-runtime/tests/` — only those 4 files migrate; other test files stay
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace --release --bin pnp_cli`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: binary builds (AC-1).
  - "Run `cargo test -p pnp-cli --test e2e_integration_tdd`; return FACT pass/fail." — purpose: E2E test passes against pnp_cli.
  - "Run `cargo test -p pnp-cli --test helpers_cli && cargo test -p pnp-cli --test cli_tdd && cargo test -p pnp-cli --test module_search_path_tdd`; return FACT pass/fail." — purpose: 3 remaining migrated tests pass.
  - "Run `cargo run --release --bin pnp_cli -- slice --help && cargo run --release --bin pnp_cli -- module --help && cargo run --release --bin pnp_cli -- mesh --help && cargo run --release --bin pnp_cli -- dag --help`; return FACT pass/fail." — purpose: AC-9 verb tree wiring.
- Context cost: `M`
- Authoritative docs:
  - `docs/05_module_sdk.md` — delegate SUMMARY of the `slicer-cli` verb names being replaced (for the `module new` clap parser shape).
- Verification:
  - `cargo build --workspace --release --bin pnp_cli` — FACT pass
  - `cargo test -p pnp-cli` (all migrated tests) — FACT pass
  - All 4 verb-tree `--help` invocations exit 0 — FACT pass
- Exit condition: `pnp_cli` binary built; 4 migrated tests green; verb tree responds to `--help`.

### Step 7: Port `cmd_new.rs` + extend scaffold with `.cargo/config.toml` and `README.md`

- Task IDs:
  - `TASK-213`
- Objective: Move the body of `cli/slicer-cli/src/cmd_new.rs::execute_in` into `crates/pnp-cli/src/module_new.rs::execute_in` (signature preserved: `(dir: &Path, name: &str, stage: &str) -> Result<(), NewError>`). Migrate the test from `cli/slicer-cli/tests/cmd_new_tdd.rs` into `crates/pnp-cli/tests/module_new_tdd.rs`. Extend the template output: `execute_in` writes (a) `<dir>/<name>/.cargo/config.toml` containing `[alias]\nbuild-wasm = "build --target wasm32-unknown-unknown --release"`, and (b) `<dir>/<name>/README.md` containing a short module-author intro that mentions running `cargo build-wasm` followed by `wasm-tools component new target/wasm32-unknown-unknown/release/<name_underscore>.wasm -o target/slicer/<name>.wasm`. Add a new test `emits_cargo_config_alias` in `module_new_tdd.rs` asserting (i) `.cargo/config.toml` exists with the literal substring `build-wasm = "build --target wasm32-unknown-unknown --release"`, and (ii) `README.md` exists with the literal substring `wasm-tools component new`.
- Precondition: Step 6 green; `pnp-cli` crate exists with dispatch wiring for `module new`.
- Postcondition: `pnp_cli module new tmp-scaffold-test --stage Layer::Infill` creates the new files; AC-10 test passes.
- Files allowed to read:
  - `cli/slicer-cli/src/cmd_new.rs` — load in full (estimated < 400 lines)
  - `cli/slicer-cli/tests/cmd_new_tdd.rs` — load in full
- Files allowed to edit (≤ 3):
  - `crates/pnp-cli/src/module_new.rs` (new — body ported)
  - `crates/pnp-cli/tests/module_new_tdd.rs` (new — original tests + `emits_cargo_config_alias`)
  - `crates/pnp-cli/src/main.rs` — wire dispatcher to local `module_new::execute_in` (was placeholder in step 6)
- Files explicitly out-of-bounds for this step:
  - `cli/slicer-cli/` — read-only here; deletion is step 8
- Expected sub-agent dispatches:
  - "Run `cargo test -p pnp-cli --test module_new_tdd`; return FACT pass/fail." — purpose: all `cmd_new` tests + AC-10 pass.
- Context cost: `S`
- Authoritative docs:
  - none — `cmd_new.rs` is self-contained
- Verification:
  - `cargo test -p pnp-cli --test module_new_tdd emits_cargo_config_alias` — FACT pass
  - `cargo test -p pnp-cli --test module_new_tdd` (all tests) — FACT pass
- Exit condition: scaffold test green; `pnp_cli module new` end-to-end emits `.cargo/config.toml` + `README.md`.

### Step 8: Delete `cli/slicer-cli/`

- Task IDs:
  - `TASK-213`
- Objective: Remove `cli/slicer-cli` from workspace `Cargo.toml` `members`; delete the entire `cli/slicer-cli/` directory (and `cli/` if it becomes empty).
- Precondition: Step 7 green; everything `slicer-cli` owned has either been ported (`cmd_new`) or deleted (`cmd_build`, `cmd_test`, `cmd_validate`, `cmd_run`).
- Postcondition: `cli/slicer-cli/` does not exist; workspace compiles; AC-6, AC-7, AC-N2 hold.
- Files allowed to read:
  - `Cargo.toml` — workspace `members`
- Files allowed to edit (≤ 3):
  - `Cargo.toml` — drop `cli/slicer-cli` from `members`
  - `cli/slicer-cli/` — directory deletion (one operation)
- Files explicitly out-of-bounds for this step:
  - everything else
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return FACT pass/fail." — purpose: nothing else imports from `cli/slicer-cli/` (verified earlier).
  - "Run `! cargo build --workspace --release --bin slicer 2>&1`; return FACT (pass = failure-as-expected)." — purpose: AC-N2.
- Context cost: `S`
- Authoritative docs:
  - none
- Verification:
  - `test ! -d cli/slicer-cli` — FACT pass
  - `! grep 'cli/slicer-cli' Cargo.toml` — FACT empty
  - `cargo check --workspace` — FACT pass
  - `! cargo build --workspace --release --bin slicer 2>&1` — FACT exit-non-zero
- Exit condition: directory gone, workspace compiles, `slicer` bin no longer buildable.

### Step 9: Remove `slicer-host` `[[bin]]` target + delete `crates/slicer-runtime/src/main.rs`

- Task IDs:
  - `TASK-213`
- Objective: Delete the `[[bin]] name = "slicer-host"` block from `crates/slicer-runtime/Cargo.toml`. Delete the file `crates/slicer-runtime/src/main.rs` (after step 4 it is a thin shim that calls `run_slice`; `pnp_cli` is now the only binary). The runtime crate has no `[[bin]]` targets after this step.
- Precondition: Step 8 green; `pnp_cli` exercises every flow that `slicer-host` did.
- Postcondition: AC-N1 holds (`slicer-host` bin no longer exists).
- Files allowed to read:
  - `crates/slicer-runtime/Cargo.toml` — the `[[bin]]` block
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/Cargo.toml` — drop `[[bin]]` lines
  - `crates/slicer-runtime/src/main.rs` — deleted
- Files explicitly out-of-bounds for this step:
  - everything else
- Expected sub-agent dispatches:
  - "Run `! cargo build --workspace --release --bin slicer-host 2>&1`; return FACT (pass = failure-as-expected)." — purpose: AC-N1.
  - "Run `cargo check --workspace`; return FACT pass/fail." — purpose: nothing referenced `main.rs` symbols externally.
- Context cost: `S`
- Authoritative docs:
  - none
- Verification:
  - `test ! -f crates/slicer-runtime/src/main.rs` — FACT pass
  - `! grep -E 'name *= *"slicer-host"' crates/slicer-runtime/Cargo.toml` — FACT empty
  - `! cargo build --workspace --release --bin slicer-host 2>&1` — FACT exit-non-zero
- Exit condition: `slicer-host` binary no longer buildable; workspace still builds.

### Step 10: Doc + CI + skill sweep (scope B)

- Task IDs:
  - `TASK-213`
- Objective: Rename old names across living documentation surface. Specifics:
  - `CLAUDE.md`: substitute `slicer-host` (binary) → `pnp_cli` in invocation examples; `slicer-host` (crate name) → `slicer-runtime`; add a translation note in a fitting section: "post-merge naming: `slicer-host` library → `slicer-runtime`, `slicer-cli` crate deleted, `slicer`/`slicer-host` binaries → `pnp_cli`."
  - `docs/00_project_overview.md` (~lines 120–135): rewrite the source-tree map: drop `cli/slicer-cli`, rename `crates/slicer-host` → `crates/slicer-runtime`, add `crates/pnp-cli`.
  - `docs/05_module_sdk.md`: rename binary references only (full build-flow rewrite waits for Packet 2).
  - `docs/13_slicer_helpers_crate.md` §"Integration with Host CLI" (lines ~504–540): rename verbs (`slicer-host repair` → `pnp_cli mesh repair` etc.).
  - `docs/16_slicer_report.md` §"CLI" (~lines 25–30): rename `slicer-host run` → `pnp_cli slice`.
  - `docs/17_agent_debugging.md`: rename all CLI invocations.
  - `.github/workflows/ci.yml` (lines 47, 49): substitute `cargo test -p slicer-host` → `cargo test -p slicer-runtime`; drop the `cargo test -p slicer-cli` term; add `cargo test -p pnp-cli`.
  - `.claude/skills/**/*.md`, `.agents/skills/**/*.md`, `.claude/agents/**/*.md`: any file emitting CLI invocations gets the same `slicer-host` / `slicer` → `pnp_cli` rename.
  - Active `.ralph/specs/<NN>_*/packet.spec.md` files (status: active at landing): rename references.
- Precondition: Step 9 green; `pnp_cli` is the canonical binary name across the workspace.
- Postcondition: AC-11 holds; every doc-impact grep in `packet.spec.md` returns the expected hit/no-hit.
- Files allowed to read:
  - All files in the bullet list above — load directly where < 300 lines; range-read otherwise.
- Files allowed to edit (≤ 3 categories; the sweep touches many files but each is an isolated edit):
  - canonical docs (6 files)
  - `.github/workflows/ci.yml`
  - skill/agent files (LOCATIONS dispatch returns ≤ 20 paths; only edit ones with CLI invocations)
- Files explicitly out-of-bounds for this step:
  - `.ralph/specs/_OLD/**`
  - Any `.ralph/specs/<NN>_*/packet.spec.md` whose front-matter is `status: implemented` or `status: closed`
- Expected sub-agent dispatches:
  - "Run `grep -rln 'slicer-host\|slicer-cli\|\\bslicer\\b' .claude/skills/ .agents/skills/ .claude/agents/`; return LOCATIONS ≤ 30 entries with one-line context." — purpose: enumerate living-skill files needing rename.
  - "Append a TASK-213 closure entry to `docs/07_implementation_status.md` (see design.md for the exact text); return FACT done." — purpose: backlog book-keeping without loading the full file.
- Context cost: `S`–`M` (depends on the LOCATIONS dispatch return size)
- Authoritative docs:
  - all files listed in the objective
- Verification:
  - Every Doc Impact Statement grep in `packet.spec.md` — FACT pass per grep
  - `grep -q 'slicer-runtime' .github/workflows/ci.yml && grep -q 'pnp-cli' .github/workflows/ci.yml && ! grep -E 'cargo test -p (slicer-host|slicer-cli)' .github/workflows/ci.yml` — FACT pass (AC-11)
- Exit condition: every Doc Impact grep returns the expected hit; `TASK-213` appears in `docs/07_implementation_status.md`.

### Step 11: Packet Completion Gate

- Task IDs:
  - `TASK-213`
- Objective: Run the full acceptance-ceremony command set; confirm every AC verification returns FACT pass.
- Precondition: Steps 1–10 green.
- Postcondition: All AC and AC-N criteria green; packet is ready to flip to `status: implemented`.
- Files allowed to read:
  - `packet.spec.md` — re-read the AC list as the source of truth
- Files allowed to edit (≤ 3):
  - `packet.spec.md` — flip `status: draft` → `status: implemented` at the end
- Files explicitly out-of-bounds for this step:
  - everything else
- Expected sub-agent dispatches:
  - "Run each pipe-suffixed acceptance verification command from `packet.spec.md`; return a single FACT line per AC (`AC-1: pass`, …); on first failure, SNIPPETS ≤ 20 lines of the diagnostic." — purpose: acceptance ceremony.
- Context cost: `S`
- Authoritative docs:
  - none (the gate is mechanical)
- Verification:
  - `cargo build --workspace --release` — FACT pass
  - `cargo clippy --workspace -- -D warnings` — FACT pass
  - `cargo test -p slicer-runtime && cargo test -p slicer-schema && cargo test -p pnp-cli` — FACT pass
  - Every AC- and AC-N-pinned command — FACT pass
- Exit condition: all green; status flipped.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Mechanical rename across 38 files; bounded by SNIPPETS dispatch |
| Step 2 | S | Trait + adapter; ~50 LOC |
| Step 3 | M | Synthetic-row externalisation + dag_cli signature broadening; ~6 modules + 1 test file |
| Step 4 | M | `run_slice` extract + `_stale_build_plan` delete + new TDD test |
| Step 5 | S | Move 5 const arrays + import update |
| Step 6 | M | New `pnp-cli` crate + dispatcher + 4 migrated tests |
| Step 7 | S | Port `cmd_new` + scaffold extension + 1 new test |
| Step 8 | S | Directory delete + workspace member drop |
| Step 9 | S | `[[bin]]` removal + main.rs delete |
| Step 10 | S–M | Doc/CI/skill sweep; size depends on LOCATIONS dispatch |
| Step 11 | S | Acceptance ceremony |

Aggregate: M (sum of 4 M-class + 7 S-class steps stays within M envelope because each M is self-contained and bounded by delegation). No L step.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every AC- and AC-N pipe-suffixed command in `packet.spec.md` dispatched and returned FACT pass.
- `docs/07_implementation_status.md` updated with `TASK-213` closure entry (via worker dispatch).
- `.ralph/specs/_OLD/29_slicer-cli-cmd-run-cross-platform/` noted as superseded in this packet's `requirements.md` Problem Statement (already done in this packet's prose — verify by `grep`).
- `packet.spec.md` ready to flip `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-11 and AC-N1 through AC-N3).
- Confirm the 3 gate commands listed in `packet.spec.md` §Verification are green.
- Confirm `cargo build --workspace --release && cargo clippy --workspace -- -D warnings` returns FACT pass.
- Record peak implementer context usage; if it exceeded 70%, log as a packet-authoring lesson.
- Flip `status: draft` → `status: implemented` in `packet.spec.md` frontmatter.
