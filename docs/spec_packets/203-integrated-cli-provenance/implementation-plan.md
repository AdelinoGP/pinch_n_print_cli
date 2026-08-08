# Implementation Plan: 203-integrated-cli-provenance

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **FORWARD-DEP rule for every step:** packets 201 and 202 are authored and preflight-passed (2026-08-07) but still `status: draft` and **unimplemented** — no `generated` status string exists on disk. Every symbol this packet consumes — `ModuleProvenance`, `LoadedModule::provenance()`, `IntegratedModuleRegistration`, `load_modules_from_roots_with_integrated`, `load_live_modules_for_plan_with_integrated`, the crate `crates/slicer-integrated-modules/` with `integrated_registrations()` / `native_entries()`, the `classic-perimeters` registry feature, and the shadow-diagnostic string — must be **re-derived from the landed tree** at the moment the step runs. Never quote a name or an argument order out of `design.md`, `requirements.md`, or `docs/specs/multi-edition-distribution-plan.md` §Exports ledger as though it were a code fact; those are pre-implementation contracts. If a step's re-derivation contradicts the packet, stop and report rather than improvising a shape.
- Every `cargo test` invocation tees to `target/test-output.log` (CLAUDE.md §Test output). Never run `cargo test --workspace`.

## Steps

### Step 1: `SliceRunOptions.no_integrated_modules` + slice-path seam + struct-literal blast radius

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: add `pub no_integrated_modules: bool` to `SliceRunOptions` (`crates/slicer-runtime/src/run.rs`), gate the integrated inputs handed to the live loader on it — the loader call lives in `run_slice_with_collector`, which `run_slice` delegates to, not in `run_slice` itself (locate with `rg -n 'load_live_modules' crates/slicer-runtime/src/run.rs`) — add the `--no-integrated-modules` clap arg to `Cmd::Slice` and bind it in the `SliceRunOptions` literal in `crates/pnp-cli/src/main.rs`, and repair every other `SliceRunOptions { .. }` construction site in the same step so `cargo check --workspace --all-targets` never goes red.
- Precondition: packets 201 and 202 are implemented and their symbols resolve in the tree. Concretely, all four of these must hold before any edit — verify by dispatch, do not assume:
  1. `crates/slicer-integrated-modules/` exists and exports `integrated_registrations()` and `native_entries()`;
  2. `load_live_modules_for_plan_with_integrated` exists in `crates/slicer-wasm-host/src/execution_plan_live.rs` and its landed parameter list (including 202's `native_entries` position) is known verbatim;
  3. `crates/slicer-runtime/Cargo.toml` **already** depends on `slicer-integrated-modules` — **packet 201** owns that dependency (`201/design.md` §Code Change Surface item 6: "`crates/slicer-runtime/Cargo.toml` gains the `slicer-integrated-modules` path dep (no features)"). If it is absent, STOP: adding it is outside this packet's Code Change Surface (`design.md` §Code Change Surface lists `crates/pnp-cli/Cargo.toml` as the only manifest edit) and the packet must be re-scoped, not silently widened;
  4. `run_slice`'s live-loader call region already calls the integrated-aware entry point rather than `load_live_modules_for_plan_profiled` (the pre-201 form on disk at authoring). **Packet 201** owns that call-site switch (`201/design.md` item 6: "`run.rs`: both live-loader call sites switch to the new entry point"); packet 202 only extends the entry point's signature with `native_entries`. If `run.rs` still calls the profiled form, 201 has not landed — STOP.
- Postcondition: `SliceRunOptions` carries the new field with a doc comment citing ADR-0057; with the field `true`, `run_slice` passes empty registrations **and** empty native entries to the integrated-aware loader; with it `false`, it passes whatever 201/202 wired. `pnp_cli slice --help` lists `--no-integrated-modules`. `cargo check --workspace --all-targets` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` - 1175 lines at authoring; read **only** the `SliceRunOptions` struct block (locate with `rg -n 'pub struct SliceRunOptions' crates/slicer-runtime/src/run.rs`) and the live-loader call region (locate with `rg -n 'load_live_modules' crates/slicer-runtime/src/run.rs`), ±40 lines each. Never the whole file.
  - `crates/pnp-cli/src/main.rs` - 762 lines at authoring; read **only** the `Cmd::Slice` variant and its match arm (locate with `rg -n 'SliceRunOptions \{' crates/pnp-cli/src/main.rs`). Skip the mesh, visual-debug, and support-preview arms.
  - `docs/adr/0057-three-editions-and-integrated-tier.md` - 55 lines; full read (the flag's semantics and its composition with `--no-default-module-paths`).
  - `crates/slicer-scheduler/src/module_search_path.rs` - `assemble_search_roots` doc comment only; purpose: confirm `--no-default-module-paths` drops the config-dir and exe-dir tiers and nothing else.
- Files allowed to edit (primary, at most 3):
  - `crates/slicer-runtime/src/run.rs`
  - `crates/pnp-cli/src/main.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/diagnose.rs` (Step 2), `crates/pnp-cli/Cargo.toml` (Step 3), `crates/pnp-cli/tests/**` (Step 4), `docs/17_agent_debugging.md` (Step 5).
  - `crates/slicer-wasm-host/**`, `crates/slicer-scheduler/**`, `crates/slicer-integrated-modules/**`, `modules/core-modules/**` — read-only here; 201/202/204 own them.
  - `docs/spec_packets/194-*` … `docs/spec_packets/202-*`, `docs/spec_packets/204-*`, `docs/07_implementation_status.md`, `docs/adr/*`, `docs/specs/multi-edition-distribution-plan.md`, `CONTEXT.md` — never modify.
- Blast-radius discipline (mandatory — this step adds a struct field):
  - The new `SliceRunOptions` field breaks every struct-literal construction site. **Re-derive the list first** with `rg -n 'SliceRunOptions \{' crates/` — it is a ledger fact, and the parallel struct-literal plan (`docs/specs/struct-literal-churn-gate-plan.md`, packets 194–199) may have converted some sites to functional-record-update by then, shrinking the sweep. Any site that already ends in `..Default::default()` or an FRU base needs no edit.
  - Verified against the tree at authoring (2026-08-07) — **13 construction sites across 11 files**; re-derive both numbers before editing. None ends in `..Default::default()` or an FRU base, so all 13 need the new field. `crates/pnp-cli/src/main.rs` holds 1 site and is a primary edit above; the other 12 sites in 10 files are mechanical one-line additions of `no_integrated_modules: false`:
    - `crates/slicer-runtime/tests/e2e/mm_real_fixture_gcode_tdd.rs` (`run_slice(SliceRunOptions {`)
    - `crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs`
    - `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` (options-builder helper returning `SliceRunOptions`)
    - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (**3 sites**)
    - `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`
    - `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs`
    - `crates/slicer-runtime/tests/executor/cube_4color_sparse_infill_per_painted_region_tdd.rs`
    - `crates/slicer-runtime/tests/unit/cancel_flag_tdd.rs` (fully-qualified `slicer_runtime::SliceRunOptions {`)
    - `crates/slicer-runtime/tests/unit/profile_flag_tdd.rs` (`unprofiled_options()` helper)
    - `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs`
  - These 10 blast-radius files (12 sites) are inside this step's allowed-edit surface by the template's blast-radius exception, over and above the 3-file primary cap. Do not let a follow-up `cargo check` discover them.
  - No schema/version constant is touched by this step (`design.md` §Architecture Constraints), so there is no constant-value test-assertion fallout.
- Expected sub-agent dispatches:
  - Question: verbatim landed signature of `load_live_modules_for_plan_with_integrated` (parameter names and order, including 202's `native_entries`), and does `crates/slicer-runtime/Cargo.toml` list `slicer-integrated-modules` as a dependency?; scope: `crates/slicer-wasm-host/src/execution_plan_live.rs`, `crates/slicer-runtime/Cargo.toml`; return: `FACT` (≤5 lines).
  - Question: exact return types of `slicer_integrated_modules::integrated_registrations()` and `native_entries()`, and what the crate re-exports; scope: `crates/slicer-integrated-modules/src/lib.rs`; return: `FACT` (≤5 lines). Resolves `design.md` §Open Questions `[FWD]` #1 (`Vec` vs `&'static [..]`) — the empty-disable expression must match the landed type (`&[]`, `Vec::new()`, or an empty slice literal).
  - Question: every `SliceRunOptions { .. }` construction site currently in the tree; scope: `crates/`; return: `LOCATIONS` (≤20 entries, one context line each). Re-derives the blast-radius list above.
  - Question: after 201, does `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`) load through the integrated-aware entry point or the external-only one?; scope: `crates/slicer-runtime/src/run.rs`; return: `FACT` (≤3 lines). Resolves `design.md` §Open Questions `[FWD]` #2. **Either answer keeps `Cmd::SupportPreview` and `prepare_prepass_context` out of scope** (`requirements.md` §Out of Scope) — do not edit them; record which answer held in the Step 1 commit message.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` - 55 lines, direct read
  - `docs/spec_packets/201-integrated-module-registry-tier5/design.md` §Code Change Surface item 5 and `docs/spec_packets/201-integrated-module-registry-tier5/packet.spec.md` AC-N2 - the "add an entry point, never a parameter" rule and the `&[]`-is-identity contract. **Read-only** (packet 201 is out of bounds for edits); prefer a `SUMMARY` dispatch over a full read. This rule is 201's, not an ADR's
- OrcaSlicer refs:
  - none — no OrcaSlicer behavior is involved in this packet.
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail (proves the blast-radius sweep is complete)
  - `cargo test -p slicer-runtime --test unit 2>&1 | tee target/test-output.log` - FACT pass/fail; the `unit` bucket owns `cancel_flag_tdd.rs` and `profile_flag_tdd.rs`, two of the edited literal sites (slicer-runtime has no `[[test]]` sections; buckets are the auto-discovered `tests/<dir>/main.rs` binaries `unit`, `contract`, `executor`, `integration`, `e2e`)
  - `cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log` - FACT pass/fail; covers `mm_real_fixture_gcode_tdd.rs` and `run_slice_api_tdd.rs`
  - `cargo run -p pnp-cli --bin pnp_cli -- slice --help | rg -q -- '--no-integrated-modules' && rg -q 'no_integrated_modules' crates/slicer-runtime/src/run.rs && rg -q 'no_integrated_modules' crates/pnp-cli/src/main.rs` - FACT pass/fail (AC-1; bare-token greps accept every name-resolution-equivalent binding form)
- Exit condition: FALSIFIED if `cargo check --workspace --all-targets` reports any unconstructed-field error, if `slice --help` omits the flag, if the disable branch passes anything other than empty registrations **and** empty native entries, or if any loader signature in `crates/slicer-wasm-host/` gained a `bool` parameter — packet 201's contract forbids it (`201/design.md` §Code Change Surface item 5: "add an entry point, never a parameter, to keep existing call sites untouched"; `201/packet.spec.md` AC-N2 makes the `&[]` call a strict identity). Disabling is expressed by the caller's inputs alone.

### Step 2: integrated-aware `run_diagnose` with a `modules` provenance array

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: change `run_diagnose` (`crates/slicer-runtime/src/diagnose.rs`) to `run_diagnose(module_dir, no_default_module_paths, no_integrated_modules)`, load through `load_modules_from_roots_with_integrated` (empty registrations when the flag is set), extend the local `DiagnoseOut` with `modules: Vec<DiagnoseModuleOut>` where each entry is `{ id, provenance }`, and update the sole caller — the `ModuleCmd::Diagnose` arm in `crates/pnp-cli/src/main.rs` — plus that variant's new `--no-integrated-modules` clap arg, in the same step.
- Precondition: Step 1 complete and `cargo check --workspace --all-targets` green. `ModuleProvenance` and `LoadedModule::provenance()` resolve from `slicer_runtime` (201 re-exported them through `slicer_scheduler` and `slicer_runtime` — re-derive the actual import path rather than assuming the re-export landed).
- Postcondition: `pnp_cli module diagnose` emits `{pass, modules_loaded, stages, modules: [{id, provenance}], diagnostics: [...]}`; `pass`, `modules_loaded`, `stages`, `diagnostics` keep exactly their current semantics and the exit-code contract in `docs/17_agent_debugging.md` §Diagnose (`0` / `1` / `2`) is unchanged. `provenance` is the lowercase string `"integrated"` or `"external"`. `module diagnose --help` lists `--no-integrated-modules`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/diagnose.rs` - 84 lines at authoring; full read
  - `crates/pnp-cli/src/main.rs` - 762 lines; read **only** the `ModuleCmd::Diagnose` variant and its match arm (locate with `rg -n 'run_diagnose' crates/pnp-cli/src/main.rs`)
  - `docs/17_agent_debugging.md` - 287 lines; read **only** §"Diagnose" (locate with `rg -n '^## Diagnose' docs/17_agent_debugging.md`, then ±25 lines). Delegate anything else in that file.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/diagnose.rs`
  - `crates/pnp-cli/src/main.rs`
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/manifest.rs` — 201 owns `ModuleProvenance`, `LoadedModule`, `LoadModulesReport`, and `load_modules_from_roots_with_integrated`. **Do not add `serde::Serialize`, `Display`, or any other impl to `ModuleProvenance`**; map it with a local `match` in `diagnose.rs`.
  - `crates/slicer-runtime/src/run.rs` (Step 1 owns it), `crates/pnp-cli/Cargo.toml` (Step 3), `crates/pnp-cli/tests/**` (Step 4), `docs/17_agent_debugging.md` (Step 5 — read-only here).
  - `docs/spec_packets/194-*` … `docs/spec_packets/202-*`, `docs/spec_packets/204-*`, `docs/07_implementation_status.md`, `docs/adr/*`, `CONTEXT.md` — never modify.
- Blast-radius discipline: not applicable — `DiagnoseOut` and `DiagnosticOut` are function-local `#[derive(serde::Serialize)]` structs private to `run_diagnose`, with no external construction sites, and no schema/version constant is bumped. `run_diagnose` is a public signature change with exactly one caller (verified: the only `run_diagnose` reference outside `diagnose.rs` is the `ModuleCmd::Diagnose` arm in `crates/pnp-cli/src/main.rs`); re-confirm with `rg -n 'run_diagnose' crates/` before editing and widen the step only if a second caller appeared.
- Expected sub-agent dispatches:
  - Question: verbatim landed signature of `load_modules_from_roots_with_integrated` and the exact path/name of the accessor that yields a loaded module's provenance (`LoadedModule::provenance()` or otherwise), plus the `ModuleProvenance` variant names; scope: `crates/slicer-scheduler/src/manifest.rs`; return: `FACT` (≤5 lines).
  - Question: did `ModuleProvenance` land with a `serde::Serialize` or `Display` impl?; scope: `crates/slicer-scheduler/src/manifest.rs`; return: `FACT` (≤3 lines). Resolves `design.md` §Open Questions `[FWD]` #3 — prefer the landed form if one exists, but the emitted JSON strings stay exactly `"integrated"` / `"external"` either way.
  - Question: every reference to `run_diagnose` outside `crates/slicer-runtime/src/diagnose.rs`; scope: `crates/`; return: `LOCATIONS` (≤20 entries).
- Context cost: `S`
- Authoritative docs:
  - `docs/17_agent_debugging.md` §"Diagnose" - the current output contract and exit codes this step must preserve
  - `docs/adr/0056-integrated-modules-native-dispatch.md` Decision item 2 - tier/dedup and the provenance-aware-diagnostic consequence
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo run -p pnp-cli --bin pnp_cli -- module diagnose --module-dir modules/core-modules --no-default-module-paths | rg -q '"provenance"' && cargo run -p pnp-cli --bin pnp_cli -- module diagnose --module-dir modules/core-modules --no-default-module-paths | rg -q '"external"'` - FACT pass/fail. With default features the registry is empty, so every module is `"external"`; this is the shape check only. The integrated-provenance assertions (AC-3, AC-N1, AC-N2) land in Step 4.
  - `cargo run -p pnp-cli --bin pnp_cli -- module diagnose --help | rg -q -- '--no-integrated-modules'` - FACT pass/fail
- Exit condition: FALSIFIED if the diagnose JSON loses or renames `pass` / `modules_loaded` / `stages` / `diagnostics`, if provenance is serialized with any casing other than lowercase `"integrated"` / `"external"`, if a `serde` or `Display` impl was added to `ModuleProvenance`, or if `crates/slicer-scheduler/` was modified at all.

### Step 3: `--no-integrated-modules` on `config-schema` and the four `dag` verbs

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: add the shared file-local helper `cli_integrated_registrations(no_integrated_modules: bool)` to `crates/pnp-cli/src/main.rs` (returning `slicer_integrated_modules::integrated_registrations()` or the empty equivalent), add the `--no-integrated-modules` clap arg to `ModuleCmd::ConfigSchema` and to `DagCmd::{Stages, Stage, Depends, Claims}`, give `load_dag_modules` a third `no_integrated_modules: bool` parameter and switch it (and the `ModuleCmd::ConfigSchema` arm, inline) from `load_modules_from_roots` to `load_modules_from_roots_with_integrated`, and add the `slicer-integrated-modules` dependency plus the `integrated-classic-perimeters` passthrough feature to `crates/pnp-cli/Cargo.toml` (its value resolved by the reconciliation rule below, not assumed).
- Precondition: Step 2 complete and `cargo check --workspace --all-targets` green.
- **Feature-name reconciliation (mandatory, do this before writing the passthrough):** the expected pair is now fixed on both sides — verify, don't choose.
  - `201/design.md` §Code Change Surface item 4 and §Open Questions both lock the registry crate's own feature to the **bare module-directory name** `classic-perimeters`. That `[FWD]` is CLOSED: the bare name is load-bearing because 205 composes edition features as `integrated-<name> = ["slicer-integrated-modules/<name>"]`, so a prefixed registry feature would break 205's AC-7.
  - This packet's `pnp-cli` feature is named `integrated-classic-perimeters` (locked — `packet.spec.md` AC commands spell it), and its value is therefore `["slicer-integrated-modules/classic-perimeters"]`.
  - **Verification, not assumption:** read the `[features]` table of the landed `crates/slicer-integrated-modules/Cargo.toml` before writing the passthrough. If the registry did *not* land on the bare `classic-perimeters`, that is a **201 defect, not a 203 adaptation** — STOP, do not paper over it with a renamed passthrough, and report it against 201 (205's AC-7 will fail for the same reason). Record the confirmed pair in the Step 3 commit message.
- Postcondition: all seven verbs (`slice`, `module diagnose`, `module config-schema`, `dag stages`, `dag stage`, `dag depends`, `dag claims`) accept `--no-integrated-modules`; `load_dag_modules` and the `ConfigSchema` arm load through the integrated-aware entry point; `crates/pnp-cli/Cargo.toml` declares `slicer-integrated-modules` and a feature named exactly `integrated-classic-perimeters` whose value forwards to the landed registry feature (see the reconciliation rule above). `--no-default-module-paths` keeps its exact prior meaning at all four `dag` call sites.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/main.rs` - 762 lines; read **only** the `ModuleCmd::ConfigSchema` variant + arm, the four `DagCmd` variants + arms, and `load_dag_modules` / `dag_producers` (locate with `rg -n 'load_dag_modules|dag_producers|ConfigSchema' crates/pnp-cli/src/main.rs`)
  - `crates/pnp-cli/Cargo.toml` - short; full read
  - `crates/slicer-scheduler/Cargo.toml` - `[[test]]` block only; purpose: precedent for explicit `[[test]]` targets coexisting with auto-discovery (verified at authoring: `scheduler_contract`, `scheduler_integration`, `scheduler_unit`, `region_split_manifest_tdd`, `region_split_aggregation_tdd`)
  - `crates/slicer-scheduler/src/dag_cli.rs` - `StagesOut` / `StageSummary` declarations only; purpose: confirm AC-4's asserted `StageSummary.id` field is what `run_dag_stages` prints. **Read-only — the `dag` output schemas are explicitly out of scope** (`requirements.md` §Out of Scope).
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/main.rs`
  - `crates/pnp-cli/Cargo.toml`
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/dag_cli.rs` — no provenance fields in `StagesOut` / `StageOut` / `DependsOut` / `ClaimsOut`; this packet gives the `dag` verbs integrated-aware *loading* only.
  - `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-integrated-modules/**`, `crates/slicer-wasm-host/**`, `modules/core-modules/**`.
  - `crates/slicer-runtime/src/{run.rs,diagnose.rs}` (Steps 1–2 own them), `crates/pnp-cli/tests/**` (Step 4), `docs/17_agent_debugging.md` (Step 5).
  - `docs/spec_packets/194-*` … `docs/spec_packets/202-*`, `docs/spec_packets/204-*`, `docs/07_implementation_status.md`, `docs/adr/*`, `CONTEXT.md` — never modify.
- Blast-radius discipline: not applicable — no struct field and no schema/version constant is added. `load_dag_modules` is file-local to `crates/pnp-cli/src/main.rs` with four call sites (the four `DagCmd` arms); re-confirm the count with `rg -n 'load_dag_modules' crates/pnp-cli/src/main.rs` before editing.
- Expected sub-agent dispatches:
  - Question: the exact per-module cargo feature name for the classic-perimeters registry entry, and the crate's package name as it must be written in a `[dependencies]` path entry; scope: `crates/slicer-integrated-modules/Cargo.toml`; return: `FACT` (≤4 lines).
  - Question: does enabling `slicer-integrated-modules/classic-perimeters` from `pnp-cli` pull any additional feature into the default `pnp-cli` build (i.e. is the passthrough strictly opt-in and absent from `default`)?; scope: `crates/pnp-cli/Cargo.toml`, `crates/slicer-integrated-modules/Cargo.toml`; return: `FACT` (≤3 lines). Also covers `design.md` §Open Questions `[FWD]` #4 — if packet 204 has landed by now, confirm no pilot feature is enabled transitively by this passthrough.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` - the flag's semantics and its composition with `--no-default-module-paths` (neither implies the other; the `SLICER_MODULE_PATH` env tier is untouched by both)
  - `docs/17_agent_debugging.md` §"DAG introspection" - the current per-subcommand flag list this step extends (read-only here; the doc edit is Step 5)
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `for v in "module config-schema" "dag stages" "dag stage" "dag depends" "dag claims"; do cargo run -q -p pnp-cli --bin pnp_cli -- $v --help | rg -q -- '--no-integrated-modules' || { echo "MISSING: $v"; exit 1; }; done; echo OK` - FACT `OK` / `MISSING: <verb>`
  - `cargo build -p pnp-cli --features integrated-classic-perimeters` - FACT pass/fail (proves the passthrough feature resolves)
  - `cargo clippy -p pnp-cli --all-targets -- -D warnings 2>&1 | tail -5` - FACT pass/fail
- Exit condition: FALSIFIED if any of the five verbs above omits the flag, if `crates/slicer-scheduler/` was modified, if `integrated-classic-perimeters` appears in `pnp-cli`'s `default` feature list, or if the helper duplicates registration-sourcing logic in more than one place in `main.rs` (it must be the single shared seam).

### Step 4: `integrated_provenance_tdd` feature-gated AC fixture

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: author `crates/pnp-cli/tests/integrated_provenance_tdd.rs` with the six `assert_cmd` tests that carry AC-2 … AC-5, AC-N1, AC-N2, and register it in `crates/pnp-cli/Cargo.toml` as `[[test]] name = "integrated_provenance_tdd" path = "tests/integrated_provenance_tdd.rs" required-features = ["integrated-classic-perimeters"]`.
- Precondition: Steps 1–3 complete and `cargo check --workspace --all-targets` green. **`cargo xtask build-guests --check` returns clean** — AC-2 slices with `--module-dir modules/core-modules` and AC-N2's diagnose requires every manifest's companion `.wasm` to exist on disk (`load_modules_from_roots` hard-errors on a missing companion; `docs/17_agent_debugging.md` §Diagnose exit code `2`). If it reports `STALE:`, rebuild (drop `--check`) before running anything in this step, and never attribute a failure here to this packet's edits until `--check` has returned clean.
- Postcondition: the six tests exist with exactly the names the ACs cite — `slice_flag_disables_integrated_tier`, `diagnose_lists_integrated_provenance`, `dag_stages_sees_integrated_tier`, `config_schema_includes_integrated_module`, `no_integrated_modules_empties_diagnose`, `diagnose_shows_external_shadowing_integrated` — and all six pass under `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/tests/slice_cancel_tdd.rs` and `crates/pnp-cli/tests/m73_progress_tdd.rs` - the `assert_cmd` binary-driving pattern (`Command::cargo_bin("pnp_cli")`) and the slice-invocation shape
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs` - the `stl_fixture_path` helper only; purpose: workspace-root derivation from `env!("CARGO_MANIFEST_DIR")` + `../../`
  - `crates/pnp-cli/tests/module_search_path_tdd.rs` - search-path/env-clearing semantics
  - `crates/pnp-cli/Cargo.toml` - short; full read (including Step 3's additions)
  - `docs/spec_packets/203-integrated-cli-provenance/packet.spec.md` §Acceptance Criteria — the verbatim asserted strings
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/integrated_provenance_tdd.rs` (new)
  - `crates/pnp-cli/Cargo.toml`
- Files explicitly out of bounds:
  - Every other file under `crates/pnp-cli/tests/` — do not modify or generalize an existing fixture into a shared helper module.
  - `crates/slicer-runtime/**`, `crates/slicer-scheduler/**`, `crates/slicer-wasm-host/**`, `crates/slicer-integrated-modules/**`, `modules/core-modules/**`, `resources/**`.
  - `docs/spec_packets/194-*` … `docs/spec_packets/202-*`, `docs/spec_packets/204-*`, `docs/07_implementation_status.md`, `docs/adr/*`, `CONTEXT.md` — never modify.
- Test-authoring constraints (all mandatory, from `design.md` §Architecture Constraints):
  - Every `Command` invocation calls `.env_remove("SLICER_MODULE_PATH")` — that env tier is untouched by both flags, so a developer's exported path would otherwise inject external modules and silently flip AC-3 / AC-N1.
  - Every "no search roots" test passes `--no-default-module-paths` and **no** `--module-dir`, so only tier 5 can contribute.
  - JSON assertions use `serde_json` (already a `pnp-cli` dev-dependency) against parsed values, not substring matching, wherever the AC names a field and a value (`modules_loaded`, the `modules` array's `id` / `provenance`, the `diagnostics` entry's `level` and `message`).
  - AC-N2 asserts the shadow message **verbatim**: `external module 'com.core.classic-perimeters' shadows integrated module 'com.core.classic-perimeters'`. That string is 201's contract — assert it, never restate or reformat it in product code.
  - AC-2's A/B is a stderr assertion: run one contains `shadows integrated module`, run two contains no occurrence of that substring; both runs exit `0`.
- Blast-radius discipline: not applicable — no struct field or schema constant. Adding an explicit `[[test]]` alongside `pnp-cli`'s auto-discovered test targets does not disable autodiscovery (`autotests` defaults to true on edition 2021); precedent verified in `crates/slicer-scheduler/Cargo.toml`. Confirm after the edit that the other 17 `crates/pnp-cli/tests/*.rs` binaries still build.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check` at the repo root; scope: repo root; return: `FACT` — `clean`, or the list of `STALE:` guests.
  - Question: after adding the explicit `[[test]]`, does `cargo test -p pnp-cli --features integrated-classic-perimeters --no-run` still build every pre-existing `crates/pnp-cli/tests/*.rs` target?; scope: repo root; return: `FACT` — count of compiled test binaries and pass/fail.
  - Question: run `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd 2>&1 | tee target/test-output.log`; scope: repo root; return: `FACT` pass/fail plus, on failure, `SNIPPETS` of ≤20 lines around the first `panicked at`.
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"Feature-gated test files report green when they don't compile" - why every command in this step spells `--features integrated-classic-perimeters`; a bare `cargo test -p pnp-cli` skips this target silently and prints a clean green wall
  - `CLAUDE.md` §"Guest WASM Staleness" - the `--check` precondition above
  - `docs/17_agent_debugging.md` §Diagnose - exit-code contract asserted by AC-3 / AC-N1 / AC-N2
- OrcaSlicer refs:
  - none.
- Verification (each is one AC; run them individually so a failure names its AC):
  - `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd slice_flag_disables_integrated_tier 2>&1 | tee target/test-output.log` - AC-2; FACT pass/fail
  - `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd diagnose_lists_integrated_provenance 2>&1 | tee target/test-output.log` - AC-3; FACT pass/fail
  - `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd dag_stages_sees_integrated_tier 2>&1 | tee target/test-output.log` - AC-4; FACT pass/fail
  - `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd config_schema_includes_integrated_module 2>&1 | tee target/test-output.log` - AC-5; FACT pass/fail
  - `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd no_integrated_modules_empties_diagnose 2>&1 | tee target/test-output.log` - AC-N1; FACT pass/fail
  - `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd diagnose_shows_external_shadowing_integrated 2>&1 | tee target/test-output.log` - AC-N2; FACT pass/fail
  - `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd 2>&1 | tee target/test-output.log | rg '^test result'` - FACT: must read `6 passed; 0 failed`. A `0 passed` line means the target compiled to zero tests — treat that as FAIL, never as green.
- Exit condition: FALSIFIED if the run reports fewer than 6 executed tests, if any test passes without `.env_remove("SLICER_MODULE_PATH")`, if AC-N2's message assertion is loosened below verbatim equality, or if `cargo xtask build-guests --check` was not run clean before the first failure was diagnosed.

### Step 5: `docs/17_agent_debugging.md` — provenance and the flag

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: document the new surface in the two sections named by `packet.spec.md` §Doc Impact — §"Diagnose" gains the `modules` array (one `{id, provenance}` entry per surviving module, `provenance` ∈ `"integrated"` / `"external"`), the `--no-integrated-modules` flag, and the note that the provenance-aware shadow warning appears in `diagnostics`; §"DAG introspection" extends the "All `dag` subcommands take …" flag list with `--no-integrated-modules`.
- Precondition: Steps 1–4 complete; the JSON shape being documented is the one the Step 4 tests assert, not the one this packet predicted.
- Postcondition: both AC-N3 greps pass; the §Diagnose exit-code list (`0` / `1` / `2`) is unchanged; no other section of the file is edited.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/17_agent_debugging.md` - 287 lines; read **only** §"DAG introspection" (locate with `rg -n '^## DAG introspection'`, then +12 lines) and §"Diagnose" (locate with `rg -n '^## Diagnose'`, then +25 lines). Delegate any other question about this file as a `SUMMARY`.
  - `crates/slicer-runtime/src/diagnose.rs` - the landed `DiagnoseOut` struct only; purpose: document the emitted field names exactly as serialized.
- Files allowed to edit (at most 3):
  - `docs/17_agent_debugging.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (the completion gate updates it via a worker dispatch, not this step), `docs/adr/*`, `docs/specs/multi-edition-distribution-plan.md`, `CONTEXT.md`, `docs/00_project_overview.md`.
  - All `crates/**` and `modules/**` — this is a docs-only step.
  - `docs/spec_packets/194-*` … `docs/spec_packets/202-*`, `docs/spec_packets/204-*` — never modify.
- Blast-radius discipline: not applicable — docs-only step, no struct field or constant.
- Expected sub-agent dispatches:
  - Question: does any file under `docs/` other than `docs/17_agent_debugging.md` document the `module diagnose` JSON output shape or the `dag` subcommand flag list?; scope: `docs/`; return: `LOCATIONS` (≤20 entries). If a second home exists, report it — do not widen this step's edit list without re-scoping.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` - the flag semantics being described
  - `docs/17_agent_debugging.md` - the file being edited
- OrcaSlicer refs:
  - none.
- Verification:
  - `rg -q -- '--no-integrated-modules' docs/17_agent_debugging.md && rg -qi 'provenance' docs/17_agent_debugging.md` - AC-N3; FACT pass/fail
  - `rg -c '^## ' docs/17_agent_debugging.md` - FACT: the section count must be unchanged from before the edit (no new top-level sections)
- Exit condition: FALSIFIED if either AC-N3 grep fails, if the §Diagnose exit-code list changed, if a field name in the documented JSON differs from the one `crates/slicer-runtime/src/diagnose.rs` actually serializes, or if any file outside `docs/17_agent_debugging.md` was touched.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | `SliceRunOptions` field + slice-path seam + struct-literal sweep of 13 sites across 11 files (re-derive both numbers); carries the heaviest FORWARD-DEP reconciliation (landed loader signature, registry return types) |
| Step 2 | S | 84-line `diagnose.rs` + one caller arm; ranged read of `main.rs` and docs/17 §Diagnose only |
| Step 3 | S | Six clap args, one helper, two loader-call switches, one manifest edit; all ranged reads |
| Step 4 | M | New 6-test `assert_cmd` fixture; two real slices of `20mmbox-LF.stl` in a debug build plus the guest-freshness precondition |
| Step 5 | S | Two sections of one doc file |

Aggregate: `M` (matches `packet.spec.md` `context_cost_estimate` and `design.md` §Context Cost Estimate). No step is rated L; no split is required before activation.

## Packet Completion Gate

- All five steps and their exit conditions complete.
- Every pipe-suffixed AC command in `packet.spec.md` (AC-1 … AC-5, AC-N1 … AC-N3) returns PASS, each dispatched with a FACT pass/fail return.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are green.
- `cargo xtask build-guests --check` returns clean (precondition for AC-2 / AC-N2; re-run at the gate, not just at Step 4).
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read. This program has no `TASK-###` rows (`requirements.md` §Packet Metadata) — the dispatch records the ADR-0056/ADR-0057 CLI-and-provenance slice as delivered; it must not invent a TASK row.
- Reconcile reopened/superseded status transitions: none expected — this packet supersedes nothing. Confirm packets 201 and 202 read `status: implemented` before flipping this one; if either is still `draft`, this packet cannot close.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and the three packet-level gate commands from `packet.spec.md` §Verification. **Every `integrated_provenance_tdd` invocation must spell `--features integrated-classic-perimeters`** — the bare form skips the target silently and prints a clean green wall (CLAUDE.md §Feature-gated test files). A binary-count or test-count drop versus Step 4's run means the ceremony run was blind; reconcile before concluding anything.
- Record remaining packet-local risk: the `native_entries()` half of the disable seam is never exercised with non-empty input until packet 204 lands (AC-2 proves only the registrations half); the `dag` verbs gain integrated-aware loading but no provenance in their JSON output; packet 205 will be the first consumer of the locked lowercase `"integrated"` / `"external"` strings.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use `--all-targets` where the command form admits it (`--test <name>` selects a single target and is the narrower, preferred form for per-step verification).
