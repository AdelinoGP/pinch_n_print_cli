# Implementation Plan: 162_wit-lifecycle-export-removal

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Steps 3-7 leave the workspace non-compiling by design.** Their exits are grep counts, not builds. The first `cargo check --workspace --all-targets` gate is Step 8. Do not repair an intermediate step by re-adding `on_print_start`.
- Where a step's edit surface is a scripted rename over a glob, the "files allowed to edit" entry names the glob and the exact command. Hand-authored files per step never exceed 3.

## Steps

### Step 1: Confirm the GREEN parity baseline, then add the RED guard test

- Task IDs: `TASK-146a`
- Objective: Establish — by running them, not by assumption — that the parity set is **green before** any edit, and add `no_lifecycle_exports_anywhere` (AC-N1), which must fail now.
- Precondition: Clean tree containing **`ff21378e`** (the `object_id` fix — it is committed, so the green baseline reproduces from HEAD rather than depending on anyone's working tree). `cargo xtask build-guests --check` reports no `STALE:`. **A fresh `pnp_cli`: run `cargo build --bin pnp_cli` first — the freshness gate this packet builds does not exist yet, so `legacy_zero_matches_golden` will otherwise spawn whatever is on disk. This step is the last one exposed to the very trap Step 10 closes; treat its result with suspicion until Step 10 lands.**
- Postcondition: `perimeter_parity` → `12 passed; 0 failed; 11 ignored` and `legacy_zero_matches_golden` → `1 passed; 0 failed` are recorded in the swarm's working notes as the baseline to preserve. Both were verified on a clean tree at `b7f17f75` during authoring; a mismatch means the environment differs, not that the packet is wrong. `no_lifecycle_exports_anywhere` exists and FAILS, listing >100 offenders.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` - the `no_versioned_world_identifiers_outside_canonical_wit` fn (locate by name)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (the file's **only** location — there is no top-level `tests/perimeter_parity.rs`), `crates/slicer-model-io/src/loader.rs` (where `path_object_id` actually lives — **not** `slicer-runtime`, which has no `loader.rs`), and every parity golden/baseline — run them, record results, never edit
  - `OrcaSlicerDocumented/`, `target/`, `*.wasm`
- Expected sub-agent dispatches:
  - Question: "Run `cargo build --bin pnp_cli`, then `cargo test -p slicer-runtime --test integration -- perimeter_parity` and `cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden`; return the exact `test result:` lines."; scope: `crates/slicer-runtime/tests/`; return: `FACT` (2 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Out of scope — file, don't fix" item 3 — **read for history only; its "keep these tests known-red" instruction is OBSOLETE.** The `object_id` session landed; the set is green. `requirements.md` §"Step Completion Expectations" is authoritative.
- OrcaSlicer refs:
  - None — this packet borrows no canonical behavior.
- Verification:
  - `cargo test -p slicer-runtime --test contract no_lifecycle_exports_anywhere 2>&1 | rg '^test result'` - FACT: must report `FAILED`
- Exit condition: The guard test fails with an offender list, **and** both parity commands report `0 failed`. If the guard *passes* now, it is not walking the tree — fix it before proceeding. **If either parity command reports a failure, STOP** — the baseline is not what this packet was authored against; do not start deleting, and report the discrepancy.

### Step 2: Delete the WIT lifecycle exports and the self-certifying schema table

- Task IDs: `TASK-146a`
- Objective: Remove the declaration and its Rust mirror: `world-layer.wit:20-21`, `WORLD_LIFECYCLE_EXPORTS`, `lifecycle_exports_for_world`, `ExportKind::Lifecycle`, `every_world_has_lifecycle_exports`; repair two doc comments.
- Precondition: Step 1 exit met.
- Postcondition: `world-layer.wit` declares 8 exports. `crates/slicer-schema/src/lib.rs` names no lifecycle symbol. `SUPPORTED_WIT_WORLDS`' doc no longer links `[WORLD_LIFECYCLE_EXPORTS]` (a broken intra-doc link would fail `-D warnings`). The const itself is untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/src/lib.rs` - lines `180-235`, `290-310`, `348-360`, `420-440`
  - `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` - full (29 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`
  - `crates/slicer-schema/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/README.md`, `docs/03_wit_and_manifest.md` - packet #3
  - `crates/slicer-wasm-host/src/host.rs`, `dispatch.rs` - packet #2; the deleted exports have zero host callers
- Expected sub-agent dispatches:
  - Question: "Do `call_on_print_start` or `call_on_print_end` have any caller under `crates/`?"; scope: `--include=*.rs crates/`; return: `FACT` (expected: none)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` - delegated SUMMARY; the vacuous-guard pathology
- OrcaSlicer refs:
  - None.
- Verification:
  - `python3 -c "import re; s=open('crates/slicer-schema/wit/deps/world-layer/world-layer.wit').read(); print(len(re.findall(r'^\s*export ',s,re.M)), len(re.findall(r'on-print',s)))"` - FACT: must print `8 0`
- Exit condition: `8 0` printed, and `rg -c 'WORLD_LIFECYCLE_EXPORTS|lifecycle_exports_for_world|ExportKind::Lifecycle|every_world_has_lifecycle_exports' crates/slicer-schema/src/lib.rs` finds no match. `crates/slicer-schema` alone still compiles (`cargo check -p slicer-schema --all-targets`); the rest of the workspace does not, as expected.

### Step 3: Rename the SDK constructor and delete `on_print_end` from all four traits

- Task IDs: `TASK-146a`
- Objective: `on_print_start` → `from_config` ×4; delete `on_print_end` ×4; rewrite the four doc blocks that describe the never-existing lifecycle.
- Precondition: Step 2 exit met.
- Postcondition: `traits.rs` declares exactly 4 `fn from_config(config: &ConfigView) -> Result<Self, ModuleError>;` and zero `on_print_*` in code or docs. Docs state that `from_config` constructs one module value per stage call and that per-print state belongs in the prepass tier + Blackboard (ADR-0029).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/traits.rs` - lines `281-310`, `500-530`, `630-660`, `1670-1685` only (file is 1700 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-sdk/tests/` (scripted, 4 files: `layer_module_tdd.rs`, `prepass_module_tdd.rs`, `postpass_module_tdd.rs`, `finalization_module_tdd.rs`)
- Files explicitly out of bounds:
  - `crates/slicer-macros/**` - Step 4
  - `modules/core-modules/**` - Step 5
- Expected sub-agent dispatches:
  - None — every edit site is pinned in `design.md`.
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md` - lines `165-200`, `726-736` only; the prose being corrected in Step 10
- OrcaSlicer refs:
  - None.
- Verification:
  - `python3 -c "import re; s=open('crates/slicer-sdk/src/traits.rs').read(); print(len(re.findall(r'fn from_config\(config: &ConfigView\) -> Result<Self, ModuleError>;',s)), len(re.findall(r'on_print_(start|end)',s)))"` - FACT: must print `4 0`
- Exit condition: `4 0` printed. The workspace does not compile — expected; `slicer-macros` still calls the old name.

### Step 4: Strip all lifecycle machinery from the macro

- Task IDs: `TASK-146a`
- Objective: Remove the `WORLD_LIFECYCLE` import and metadata use, the typed lifecycle bindings, `__SLICER_LIFECYCLE_EXPORT_COUNT`, the fake `#[export_name]` lifecycle shims, and the layer `impl Guest` lifecycle fns; rename the 15 construction sites to `from_config`.
- Precondition: Step 3 exit met.
- Postcondition: `wit_exports` is the stage export alone (0 or 1 entries). `crates/slicer-macros/src/lib.rs` names no lifecycle symbol and carries exactly 15 `::from_config(&ir_config)`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-macros/src/lib.rs` - lines `15-25`, `140-200`, `240-300`, `304-410`, `2755-2780` only (file is 2800+ lines). Open ±40-line windows around each pinned site: `:632`, `:656`, `:1036`, `:1334`, `:1411`, `:1472`, `:1543`, `:1713`, `:1734`, `:1755`, `:1775`, `:1795`, `:1828`, `:1890`, `:1910`.
- Files allowed to edit (at most 3):
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-macros/tests/**` - Step 6
  - `crates/slicer-schema/src/lib.rs` - Step 2 is closed; do not revisit
- Expected sub-agent dispatches:
  - Question: "In `crates/slicer-macros/src/lib.rs`, list every line matching `fn on_print` and state which `impl Guest` block each is inside."; scope: `crates/slicer-macros/src/lib.rs`; return: `LOCATIONS` (≤5); purpose: confirm the layer world is the only glue block with lifecycle fns
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"The lifecycle finding" - direct range read
- OrcaSlicer refs:
  - None.
- Verification:
  - `python3 -c "import re; s=open('crates/slicer-macros/src/lib.rs').read(); print(len(re.findall(r'on_print_start|on_print_end|WORLD_LIFECYCLE|lifecycle_shim_tokens|skip_lifecycle_shims|__SLICER_LIFECYCLE_EXPORT_COUNT',s)), len(re.findall(r'::from_config\(&ir_config\)',s)))"` - FACT: must print `0 15`
- Exit condition: `0 15` printed. If the second number is not 15, a construction site was missed — the sites are pinned in `design.md`; do not guess.

### Step 5: Mechanical sweep — modules, test guests, runtime tests

- Task IDs: `TASK-146a`
- Objective: Rename `on_print_start` → `from_config` and delete `on_print_end` across the 20 core modules, 9 test guests, and ~30 `slicer-runtime` test files; drop the two lifecycle assertions from each `slicer_module_binding_tdd.rs`; delete `arachne-perimeters`' `on_print_end` body.
- Precondition: Step 4 exit met.
- Postcondition: Zero `on_print_start` / `on_print_end` outside `crates/slicer-macros/tests/`, `crates/pnp-cli/src/module_new.rs`, `docs/`, and `wit_drift_detection_tdd.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - lines `410-425` only (the sole real `on_print_end` body)
  - `modules/core-modules/gyroid-infill/tests/slicer_module_binding_tdd.rs` - full (21 lines); the template for the other 19
- Files allowed to edit (at most 3 hand-authored; the rest scripted over a glob):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (hand: delete `on_print_end` at `:419`, do not rename it)
  - glob `modules/core-modules/*/{src,tests}/**/*.rs`, `crates/slicer-wasm-host/test-guests/*/src/lib.rs`, `crates/slicer-runtime/tests/**/*.rs` — scripted: rename `on_print_start` → `from_config`; delete each `fn on_print_end` body and each `exports.contains(&"on-print-start")` / `&"on-print-end")` assertion
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` - the guard names the strings deliberately; the sweep must not touch it
  - `crates/slicer-macros/tests/**` - Step 6
  - Any `.wasm`, `target/`
- Expected sub-agent dispatches:
  - Question: "How many occurrences of `on_print_start|on_print_end` remain under `crates/` and `modules/`, excluding `target/`, `crates/slicer-macros/tests/`, `crates/pnp-cli/src/module_new.rs`, and `wit_drift_detection_tdd.rs`?"; scope: `--include=*.rs crates/ modules/`; return: `FACT` (one integer, expected `0`)
- Context cost: `M`
- Authoritative docs:
  - None — mechanical.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -c 'on_print_start|on_print_end' --type rust crates/ modules/ -g '!target' -g '!**/wit_drift_detection_tdd.rs' -g '!crates/slicer-macros/tests/**' -g '!crates/pnp-cli/src/module_new.rs' | rg -v ':0$' | wc -l` - FACT: must print `0`
- Exit condition: `0` printed. Do **not** read the swept files to confirm; Step 8's `cargo check --workspace --all-targets` is the completeness proof.

### Step 6: Update the macro's own tests to the lifecycle-free surface

- Task IDs: `TASK-146a`
- Objective: `binding_surface_tdd.rs`, `all_worlds_glue_tdd.rs`, `slicer_module_tdd.rs`, `smoke.rs` assert the new surface: stage-only exports, empty exports for a stageless impl, no `__SLICER_LIFECYCLE_EXPORT_COUNT`, no lifecycle glue arms.
- Precondition: Step 5 exit met.
- Postcondition: `LayerInfillFixture::__slicer_wit_exports() == ["run-infill"]`; `__slicer_module_schema().exports == [ExportBinding { name: "run-infill", kind: ExportKind::Stage }]`; `LayerLifecycleOnly` (renamed test `layer_stageless_module_lists_no_exports`) reports `[]` and `exports.len() == 0`; `macro_layer_world_covers_all_eight_stage_exports` lists 8 arms, not 10.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-macros/tests/binding_surface_tdd.rs` - lines `245-300`, `420-475`, `510-570` only
  - `crates/slicer-macros/tests/all_worlds_glue_tdd.rs` - lines `95-140` only
- Files allowed to edit (at most 3):
  - `crates/slicer-macros/tests/binding_surface_tdd.rs`
  - `crates/slicer-macros/tests/all_worlds_glue_tdd.rs`
  - `crates/slicer-macros/tests/slicer_module_tdd.rs` and `smoke.rs` (scripted rename only — no assertion redesign)
- Files explicitly out of bounds:
  - `crates/slicer-macros/src/lib.rs` - Step 4 is closed
- Expected sub-agent dispatches:
  - None.
- Context cost: `S`
- Authoritative docs:
  - None.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-macros 2>&1 | tee target/test-output.log | rg '^test result'` - FACT pass/fail; SNIPPETS ≤20 lines on failure
- Exit condition: All `slicer-macros` test binaries report `ok`. AC-4 and AC-5 are now satisfiable.

### Step 7: Reduce the `pnp_cli module new` scaffold to the stage export

- Task IDs: `TASK-146a`
- Objective: `expected_exports` = the single stage export; scaffold `from_config`; delete the generated `on_print_start_succeeds()` test and the packet-local `lib_rs_has_on_print_start_lifecycle`.
- Precondition: Step 6 exit met.
- Postcondition: A scaffolded `Layer::Infill` module's manifest comment lists exactly `run-infill`; its `lib.rs` has `fn from_config` and no `on_print_start`; `module_new.rs` no longer imports `lifecycle_exports_for_world`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/module_new.rs` - lines `1-20`, `205-235`, `300-340`, `705-720` only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/module_new.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs` - read-only reference for `CARGO_BIN_EXE_pnp_cli` at `:394`; not this step's surface
- Expected sub-agent dispatches:
  - None.
- Context cost: `S`
- Authoritative docs:
  - None.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p pnp-cli --lib module_new 2>&1 | tee target/test-output.log | rg '^test result'` - FACT pass/fail
- Exit condition: `module_new` tests pass and `rg -c 'on_print_start|on_print_end|lifecycle_exports_for_world' crates/pnp-cli/src/module_new.rs` finds no match. AC-7 satisfied.

### Step 8: Workspace compile + clippy gate

- Task IDs: `TASK-146a`
- Objective: Prove the 110-file sweep is complete using the type system, and that no import, variant, or doc link was orphaned.
- Precondition: Steps 2-7 exits all met.
- Postcondition: `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` both pass.
- Files allowed to read, with ranges when over 300 lines:
  - Only files named in a compiler error, ±20 lines.
- Files allowed to edit (at most 3):
  - Only files named in a compiler error, within the surfaces of Steps 2-7.
- Files explicitly out of bounds:
  - Any file outside `design.md`'s change surface. A compile error there means the sweep over-reached — revert, do not widen.
- Expected sub-agent dispatches:
  - Question: "Run `cargo check --workspace --all-targets`; report pass/fail and, on failure, the first 20 lines of error output."; scope: workspace; return: `FACT` + `SNIPPETS` ≤20 lines
  - Question: "Run `cargo clippy --workspace --all-targets -- -D warnings`; report pass/fail and, on failure, the first 20 lines."; scope: workspace; return: `FACT` + `SNIPPETS` ≤20 lines
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Test Discipline" — `--all-targets` is mandatory; plain `cargo check --workspace` does not compile test targets and would leave the sweep unverified.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && echo GATE_PASS` - FACT pass/fail
- Exit condition: `GATE_PASS`. Watch specifically for a broken intra-doc link from `SUPPORTED_WIT_WORLDS` to the deleted `WORLD_LIFECYCLE_EXPORTS` — that is the one real `-D warnings` forcing function in this packet.

### Step 9: Rebuild every guest and prove the export surface shrank

- Task IDs: `TASK-146a`
- Objective: Regenerate all 20 core-module `.wasm` artifacts and the test guests, then prove no lifecycle export survives in any decoded component.
- Precondition: Step 8 `GATE_PASS`.
- Postcondition: `cargo xtask build-guests --check` is clean. Every `modules/core-modules/*/*.wasm` decodes with zero `on-print` hits; `gyroid-infill.wasm` decodes to exactly 8 world-level exports (it decoded to 10, including `on-print-start` / `on-print-end` at lines 12-13, before this packet).
- Files allowed to read, with ranges when over 300 lines:
  - None. Guest artifacts are inspected only through `wasm-tools component wit <path> | grep`.
- Files allowed to edit (at most 3):
  - None (build outputs only).
- Files explicitly out of bounds:
  - `target/`, every `.wasm` (never load; decode and grep)
- Expected sub-agent dispatches:
  - Question: "Run `cargo xtask build-guests`; report pass/fail and the tail 20 lines on failure."; scope: workspace; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Guest WASM Staleness" — this packet edits five guest-invalidating input classes at once, so every guest is stale.
- OrcaSlicer refs:
  - None.
- Verification:
  - `n=$(for w in modules/core-modules/*/*.wasm; do wasm-tools component wit "$w"; done | grep -c 'on-print'); e=$(wasm-tools component wit modules/core-modules/gyroid-infill/gyroid-infill.wasm | grep -c '^  export '); echo "lifecycle=$n exports=$e"` - FACT: must print `lifecycle=0 exports=8`
- Exit condition: `lifecycle=0 exports=8`, and `cargo xtask build-guests --check` prints no `STALE:`. AC-6 satisfied. AC-N1 (`no_lifecycle_exports_anywhere`) now flips from RED to GREEN — run it and confirm.

### Step 10: Make CLI-binary staleness fail loudly — the seam, the test, the xtask gate

- Task IDs: `TASK-146a`
- Objective: Add the pure `staleness_reason` seam in `slicer_cache.rs` + its registered regression test; delete that copy's release-over-debug fallback; gate `pnp_cli` in `xtask test` Step 1.
- Precondition: Step 9 exit met.
- Postcondition: `pnp_cli_bin()` panics on a stale or absent binary naming `pnp_cli`, `stale`, the path, and the remedy. `xtask/src/test.rs` checks and rebuilds `pnp_cli` alongside the guest gate. `crates/slicer-runtime/tests/common/slicer_cache.rs` contains no `for profile in ["release", "debug"]` loop.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` - the `compute_shared_mtime` and `is_stale` fns only (locate by name — mirror, do not import: `xtask` is bin-only and has no lib target)
  - `xtask/src/test.rs` - lines `100-140` only
  - `crates/slicer-runtime/tests/integration/main.rs` - lines `1-25` only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/slicer_cache.rs`
  - `xtask/src/test.rs`
  - `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` (new) + one `mod pnp_cli_freshness_tdd;` line in `crates/slicer-runtime/tests/integration/main.rs`, placed between `mod pipeline_tdd;` (`:40`) and `mod region_partition_tdd;` (`:46`). **The registration is mandatory**: the bucket is an aggregated binary, so an unregistered file never compiles and `cargo test --test integration pnp_cli_freshness` reports 0 tests — a false pass that looks green.
- Files explicitly out of bounds:
  - `crates/slicer-runtime/benches/gate_evidence.rs`, `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` - Step 10b; do not touch here (keeps this step inside the 3-edit cap)
  - `crates/pnp-cli/Cargo.toml`, `crates/slicer-runtime/Cargo.toml` - no dev-dep is added; `CARGO_BIN_EXE_pnp_cli` is unavailable outside the defining package and a dev-dep does not change that
- Expected sub-agent dispatches:
  - None.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Grounding corrections" item 1 - direct range read; the `env!` fix is falsified there
  - `CLAUDE.md` §"`cargo xtask test` — the gated entry point"
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 | tee target/test-output.log | rg '^test result'` - FACT pass/fail
- Exit condition: The three `pnp_cli_freshness_tdd` cases pass (older ⇒ `Some` containing `pnp_cli`+`stale`; absent ⇒ `Some`; newer ⇒ `None`), `rg -q '^mod pnp_cli_freshness_tdd;' crates/slicer-runtime/tests/integration/main.rs` matches, `rg -q 'pnp_cli' xtask/src/test.rs` matches, and the release fallback is gone from `slicer_cache.rs`. AC-8b, AC-9, AC-N2 satisfied.

### Step 10b: Apply the same freshness assert to the other two spawn sites

- Task IDs: `TASK-146a`
- Objective: Close the remaining two copies of the trap — the DEV-026 evidence bench and the scheduler's DAG CLI test — with the same in-place assert.
- Precondition: Step 10 exit met; `staleness_reason`'s shape is settled and can be mirrored verbatim.
- Postcondition: Neither file lets a stale artifact win. `dag_cli_integration.rs`'s panic names `cargo build -p pnp-cli` and states the staleness cause.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/benches/gate_evidence.rs` - lines `36-75` only
  - `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` - lines `10-35` only
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` - the `staleness_reason` + `newest_source_mtime` fns just written, as the text to mirror
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/benches/gate_evidence.rs` (fix `pnp_cli_bin` `:48-74`; delete the `for profile in ["release","debug"]` loop the `for profile in ["release","debug"]` loop; correct the doc-comment `:46-47`, which currently advertises the fallback being deleted)
  - `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` (fix `fn bin()` `:15-31`; delete the debug-then-release probe; rewrite the panic at `:30`)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/common/slicer_cache.rs`, `xtask/src/test.rs` - Step 10 is closed
  - Any attempt to create a shared helper crate or a `pnp-cli` lib target — **explicitly deferred to its own ADR-bearing packet** (`[FWD]` in `design.md`). Copy the logic; do not abstract it.
- Expected sub-agent dispatches:
  - None.
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` DEV-026 row - **delegate a SUMMARY; the row is enormous.** Purpose: confirm `gate_evidence` is the sole source of the `~438ms` 50-layer time evidence, which is why a stale binary there corrupts governance evidence silently rather than failing.
- OrcaSlicer refs:
  - None.
- Verification:
  - `python3 -c "import re; F=['crates/slicer-runtime/benches/gate_evidence.rs','crates/slicer-scheduler/tests/integration/dag_cli_integration.rs']; bad=[p for p in F if re.search(r'for profile in \[\"(release|debug)\", \"(debug|release)\"\]',open(p).read())]; d=open(F[1]).read(); print('PASS' if not bad and 'cargo build -p pnp-cli' in d and 'cargo build --workspace' not in d else f'FAIL {bad}')"` - FACT pass/fail
- Exit condition: The command prints `PASS`, `cargo check --workspace --all-targets` still passes (the bench is a `--all-targets` target), and `cargo test -p slicer-scheduler --test integration dag_cli` passes. AC-8 satisfied.

### Step 11: Correct the docs the deletion makes false

- Task IDs: `TASK-146a`
- Objective: Remove the lifecycle fiction from `docs/03`, `docs/04`, and `docs/05`; record TASK-146a in `docs/07`.
- Precondition: Step 10b exit met.
- Postcondition: None of `docs/03`, `docs/04`, `docs/05` matches `on.print.start|on.print.end`. `docs/03`'s `// Lifecycle — optional` stanza (`:559-561`) is gone. `docs/05` §"Module State Lifecycle (Normative)" states that `from_config` constructs one module value **per stage call** (per layer, per stage), that no state is retained across calls, and that per-print state lives in the prepass tier + Blackboard (ADR-0029). `docs/04:1449`'s `call on-print-start on all modules` plan-freeze step is gone.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/03_wit_and_manifest.md` - lines `555-565` only. The stanza reads `// Lifecycle — optional` followed by the two exports. **Delete all three lines.** The comment is doubly false: the component model has no optional exports (wasmtime's `Indices::new` resolves every export eagerly at `instantiate` — ADR-0045's premise), and the exports it labels are being deleted outright. This line is the origin of the fiction the rest of this packet dismantles.
  - `docs/05_module_sdk.md` - lines `165-200`, `290-300`, `370-380`, `726-736`, `840-970`, `1140-1150` only (file >1100 lines). The `:53` / `:61-62` / `:238-243` refs from earlier plan drafts are **wrong** — those windows are §"Guest Build Invariants" and §`run_infill_postprocess`.
  - `docs/04_host_scheduler.md` - lines `1444-1455` only (file >1400 lines). `:1315`'s "once per print" is a different subject — do not touch it.
- Files allowed to edit (at most 3 hand-authored; `docs/07` is dispatch-only):
  - `docs/03_wit_and_manifest.md` (three-line delete at `:559-561` — **nothing else**; the listing's per-stage restructure is packet #3's and the two edits are disjoint)
  - `docs/04_host_scheduler.md`
  - `docs/05_module_sdk.md`
  - `docs/07_implementation_status.md` (via dispatch only — never read the backlog directly)
- Files explicitly out of bounds:
  - `CONTEXT.md`, `crates/slicer-schema/wit/README.md`, `docs/DEVIATION_LOG.md` - packet #3
  - The rest of `docs/03_wit_and_manifest.md` beyond `:559-561`
- Expected sub-agent dispatches:
  - Question: "In `docs/07_implementation_status.md`, add a TASK-146a entry recording that TASK-146 is reopened (ADR-0045 retires `validate_wit_world`) and that packet 162 deleted the lifecycle WIT contract; follow the existing TASK-119a / TASK-194a sub-lettering convention."; scope: `docs/07_implementation_status.md`; return: `FACT` (the added line)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"The lifecycle finding" — the source of the corrected prose
- OrcaSlicer refs:
  - None.
- Verification:
  - `python3 -c "import re; bad=[p for p in ('docs/03_wit_and_manifest.md','docs/04_host_scheduler.md','docs/05_module_sdk.md') if re.search(r'on.print.start|on.print.end',open(p,encoding='utf-8').read())]; print('PASS' if not bad and 'per stage call' in open('docs/05_module_sdk.md',encoding='utf-8').read() else f'FAIL {bad}')"` - FACT pass/fail
- Exit condition: `PASS`, and every Doc Impact grep in `packet.spec.md` returns 0. AC-10 satisfied.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Baseline + RED guard; one ranged read |
| Step 2 | S | 29-line WIT + 4 ranged windows in a 440-line file |
| Step 3 | S | 4 pinned windows in a 1700-line file; never read whole |
| Step 4 | M | 2800+ line file, 25 pinned sites; ±40-line windows only |
| Step 5 | M | 110-file scripted sweep; zero file reads, FACT-count exit |
| Step 6 | S | 3 ranged windows in the macro's own tests |
| Step 7 | S | 4 ranged windows in one file |
| Step 8 | S | Two delegated cargo gates, FACT returns |
| Step 9 | S | Delegated build + one grep pipeline |
| Step 10 | M | New pure seam + registered regression test + xtask gate |
| Step 10b | S | Mirror the seam into the 2 remaining spawn sites; no new design |
| Step 11 | S | Ranged doc edits (03/04/05); docs/07 via dispatch only |

Aggregate: `M`. No step is `L`. Split before activation if that changes.

Step 10 was split into 10/10b rather than absorbing all three spawn sites: five edit files in one step would bust the ≤3 cap, and the split gives 10b a settled `staleness_reason` to mirror verbatim instead of three parallel improvisations.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command (AC-1 … AC-10, AC-8b, AC-N1, AC-N2, AC-N3) returns PASS.
- `cargo xtask build-guests --check` reports no `STALE:`.
- **The parity set is green-before / green-after** (AC-N3): `perimeter_parity` → `12 passed; 0 failed; 11 ignored`, `legacy_zero_matches_golden` → `1 passed; 0 failed`, matching the Step 1 baseline exactly. The `object_id` session has landed, so any failure here is **caused by this packet** — not inherited, not flaky. Never resolve one by re-blessing a golden: the trap this packet closes is precisely a `BLESS_GOLDEN=1` run writing a stale binary's output to disk as truth.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile the reopened status: TASK-146 → reopened, TASK-146a → this packet.
- Update `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Packet Queue" row #1: `status: pending` → `generated`/`implemented`, `packet dir` → `.ralph/specs/162_wit-lifecycle-export-removal`.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo xtask test --summary --workspace` **via a sub-agent with a `FACT pass/fail` return** — never absorb the full output (>1000 tests, ≥11 minutes). This is the one sanctioned whole-suite run for this packet, permitted only because closure requires it and every narrower verification above has already passed.
- Record remaining packet-local risk: the two surviving stale-binary traps and `docs/03:560-561` (both `[FWD]` in `design.md`).
- Report every net-new symbol packets #2/#3 may consume: `slicer_sdk::traits::{LayerModule,PrepassModule,PostpassModule,FinalizationModule}::from_config`; `SlicerModuleSchema.exports` (now ≤1 entry, `ExportKind::Stage` only); `slicer_cache::staleness_reason`.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
