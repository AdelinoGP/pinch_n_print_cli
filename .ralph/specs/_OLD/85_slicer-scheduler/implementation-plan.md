# Packet 85 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- Files allowed to read per step are bounded. The nine moved files total ~5 500 LOC; NEVER load any in full. Read only `use` lines and `pub` surfaces.
- P85 is a workspace-test checkpoint: `cargo test --workspace` is part of the closure gate, NOT optional.
- P83 MUST be closed (Step 0 verifies). P84 is recommended (so the diff doesn't collide with algorithm moves) but not strictly required — the moved planning files don't overlap with the moved algorithm files.
- No guest-feeding path is edited; `cargo xtask build-guests --check` should stay clean throughout. STALE here means investigate, not paper over.

---

## Step 0 — Verify P83 closure + capture pre-packet baselines

**Objective.** Confirm `slicer-wasm-host` is in place with `CompiledModuleStatic` renamed in `slicer-runtime/src/execution_plan.rs`. Capture three pre-packet SHAs: g-code, `pnp_cli dag stages` output, `pnp_cli dag claims` output. Capture `cargo test --workspace` pre-packet count.

**Precondition.** P83 is `superseded`. Working tree clean.

**Postcondition.** Four baselines in the implementation log.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: confirm `test -f crates/slicer-wasm-host/Cargo.toml && grep -qE 'pub struct CompiledModuleStatic' crates/slicer-runtime/src/execution_plan.rs && grep -qE 'pub type CompiledModule = CompiledModuleStatic' crates/slicer-runtime/src/execution_plan.rs`. Return FACT pass/fail.
- Dispatch: g-code SHA — `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/p85-baseline.gcode && sha256sum /tmp/p85-baseline.gcode`. Return FACT `<hex>`.
- Dispatch: dag-stages SHA — `cargo run --bin pnp_cli --release -- dag stages --module-dir modules/core-modules > /tmp/p85-stages-baseline.txt && sha256sum /tmp/p85-stages-baseline.txt`. Return FACT `<hex>`.
- Dispatch: dag-claims SHA — analogous. Return FACT `<hex>`.
- Dispatch: workspace test count — `cargo test --workspace 2>&1 | tail -5`. Return SNIPPET.

**Context cost: S.**

**Narrow verification.** All five returns positive.

**Falsifying check / exit condition.** Any verification fails → abort; P83 is incomplete.

---

## Step 1 — Enumerate moved-symbol consumers + SDK imports + `toml` consumers

**Objective.** Surface every external reference that needs rewriting.

**Precondition.** Step 0 green.

**Postcondition.** Four lists in the log per design.md dispatches #1–#4.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.** Dispatches #1, #2, #3, #4 from design.md §"Expected Sub-Agent Dispatches".

**Context cost: S.**

**Narrow verification.** Four lists populated.

**Falsifying check / exit condition.** If dispatch #1 says moved files import SDK trait types, plan to add `slicer-sdk` to `slicer-scheduler/Cargo.toml`. If dispatch #2 says `toml` has other runtime consumers, do NOT drop it from `slicer-runtime/Cargo.toml`.

---

## Step 2 — Scaffold `slicer-scheduler` crate

**Objective.** New crate exists; empty `lib.rs` compiles.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `cargo build -p slicer-scheduler` succeeds against an empty scaffold.

**Files allowed to read.** Workspace `Cargo.toml`, `crates/slicer-runtime/Cargo.toml`.
**Files allowed to edit.**
1. Workspace `Cargo.toml` — add `"crates/slicer-scheduler"` to `members`.
2. `crates/slicer-scheduler/Cargo.toml` — CREATE per design.md §Code Change Surface row 1.
3. `crates/slicer-scheduler/src/lib.rs` — CREATE with module declarations and empty submodule files (each `// placeholder`).

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-scheduler`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Crate builds.

**Falsifying check / exit condition.** Build fails on a dep version → check workspace inheritance.

---

## Step 3 — Move the nine planning files verbatim

**Objective.** Nine files exist under `slicer-scheduler/src/`, identical content; nine old files deleted from `slicer-runtime/src/`. Workspace does NOT build yet (lib.rs declarations and the runtime's `pub mod` declarations are stale).

**Precondition.** Step 2 complete.

**Postcondition.** The nine new files exist in `slicer-scheduler/src/`; the nine old files absent from `slicer-runtime/src/`. Inside the new files, `use crate::*` paths still resolve because all referenced modules moved together (same-crate same-name).

**Files allowed to read.** `use crate::*` lines from each moved file (top of file, ≤ 20 lines per file).
**Files allowed to edit.**
1. Nine `crates/slicer-scheduler/src/<file>.rs` files — populate verbatim from their `slicer-runtime/src/` counterparts.
2. Delete the nine files from `slicer-runtime/src/`.

**Expected sub-agent dispatch.**
- (No build check this step — Step 4 introduces the instrumentation split and finally fixes the build.)

**Context cost: M.**

**Narrow verification.** `for f in manifest config_resolution dag validation execution_plan topology stage_order module_search_path dag_cli; do test ! -f crates/slicer-runtime/src/$f.rs && test -f crates/slicer-scheduler/src/$f.rs; done` returns success per file.

**Falsifying check / exit condition.** A file move leaves the runtime referencing a now-missing module → expected; Step 4 fixes it.

---

## Step 3.5 — Relocate the live-loader cluster to `slicer-wasm-host`; strip wasmtime fields from `CompiledModuleStatic`/`CompiledModuleBuilder`/`ExecutionModuleBinding`; rewire callsites

**Objective.** Complete the Static/Live field migration P83 started at the type level but didn't finish at the field level. Six "live loader" symbols (`LiveModuleBinding`, `build_live_execution_plan`, `LiveModuleLoadOutput`, `LiveModuleLoadError`, `load_live_modules_for_plan`, `compile_module_component`) move from the just-moved `slicer-scheduler/src/execution_plan.rs` into a new `crates/slicer-wasm-host/src/execution_plan_live.rs`. The two wasmtime fields (`instance_pool: Arc<WasmInstancePool>`, `wasm_component: Option<Arc<WasmComponent>>`) are stripped from `CompiledModuleStatic`, `CompiledModuleBuilder`, and `ExecutionModuleBinding` and moved to `CompiledModuleLive<'s>`. Every callsite in `slicer-runtime` that read those fields off Static now reads them off Live.

**Precondition.** Step 3 complete (nine files moved verbatim to scheduler; runtime build is expected to fail).

**Postcondition.** `cargo build -p slicer-scheduler` green. `cargo build -p slicer-wasm-host` green. `cargo build --workspace` may still fail because runtime callsites haven't been rewired yet — that gets resolved in Step 5. `slicer-scheduler/Cargo.toml` does NOT contain `slicer-wasm-host` (AC-N1 holds). `slicer-scheduler/src/execution_plan.rs` contains zero `use slicer_wasm_host::` lines and zero `Arc<Wasm*>` field declarations.

**Files allowed to read.** `crates/slicer-scheduler/src/execution_plan.rs` (find the six live-cluster symbols by line range — implementer's diagnostic surfaced them at lines 117, 135, 232, 253, 333, 496, plus fields on `CompiledModuleStatic` L661, `CompiledModuleBuilder` L739, `ExecutionModuleBinding` L834). `crates/slicer-wasm-host/src/binding.rs` (current `CompiledModuleLive<'s>` shape).
**Files allowed to edit.**
1. `crates/slicer-wasm-host/src/execution_plan_live.rs` — CREATE. Move the six live-cluster symbols verbatim from `slicer-scheduler/src/execution_plan.rs`, adjusting their imports (the `use slicer_wasm_host::*` line becomes intra-crate `use crate::*` imports).
2. `crates/slicer-wasm-host/src/binding.rs` (or wherever `CompiledModuleLive<'s>` lives) — extend `CompiledModuleLive<'s>` with the two wasmtime fields (`instance_pool: Arc<WasmInstancePool>`, `wasm_component: Option<Arc<WasmComponent>>`). Add the matching `pub fn instance_pool(&self)` / `pub fn wasm_component(&self)` accessor methods.
3. `crates/slicer-wasm-host/src/lib.rs` — add `pub mod execution_plan_live;` and re-export the cluster surface.
4. `crates/slicer-scheduler/src/execution_plan.rs` — delete the six relocated symbols. Strip `instance_pool` and `wasm_component` fields from `CompiledModuleStatic`, `CompiledModuleBuilder`, and `ExecutionModuleBinding`. Delete the corresponding accessor methods on `CompiledModuleStatic` (`pub fn instance_pool`, `pub fn wasm_component`, `pub fn as_live`). Delete the `use slicer_wasm_host::*` import line. The remaining file should be a wasmtime-free planning data structure module.
5. `crates/slicer-runtime/src/{layer_executor,pipeline,prepass,postpass,layer_finalization}.rs` (and any other runtime file the build error surfaces) — rewire `compiled_module.instance_pool()` → `live_binding.instance_pool()` and analogous for `wasm_component()`. The runtime constructs `CompiledModuleLive` per tick from `(static_module, instance_pool, wasm_component)` via a constructor on the Live side; the previous `as_live()` on Static is replaced by `CompiledModuleLive::new(static_module, instance_pool, wasm_component)` at the runtime callsite.
6. **REVERT** any `slicer-wasm-host = { path = "../slicer-wasm-host" }` line that the auto-response Step 3 added to `crates/slicer-scheduler/Cargo.toml`. AC-N1 forbids it; the proper resolution is the field migration here, not the back-edge dep.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build -p slicer-scheduler`. Return FACT pass/fail. Must pass — scheduler is now genuinely wasmtime-free.
- Dispatch: `cargo build -p slicer-wasm-host`. Return FACT pass/fail. Must pass — wasm-host owns the live cluster + extended Live binding.
- Dispatch: `cargo build --workspace 2>&1 | grep -cE '^error\[E0599\]' | head -1`. Return FACT integer (count of "method not found on Static" errors — these are the remaining callsite rewires for Step 5 to absorb, OR a sign that this step missed some).
- Dispatch: `! grep -qE '^slicer-wasm-host *=' crates/slicer-scheduler/Cargo.toml && ! rg -q 'use slicer_wasm_host::' crates/slicer-scheduler/src/`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** First two dispatches pass; fourth dispatch passes. Third dispatch's integer is small (≤ ~20) — those are the callsite rewires Step 5 finishes. If it's much higher, this step missed a wholesale rewrite that should land here, not in Step 5.

**Falsifying check / exit condition.** If `cargo build -p slicer-scheduler` still has `use slicer_wasm_host` imports, the live-cluster move was incomplete — re-grep `crates/slicer-scheduler/src/execution_plan.rs` for `slicer_wasm_host::` and relocate the missed symbol. If `cargo build -p slicer-wasm-host` fails on a missing borrow lifetime, the `CompiledModuleLive<'s>` field extension didn't compose with the existing borrow shape — surface the trait/lifetime details before continuing.

---

## Step 4 — Split `instrumentation.rs`; move `CompiledModuleStatic`; delete the type alias

**Objective.** Two key structural changes:
- Split `crates/slicer-runtime/src/instrumentation.rs` between scheduler (planning side) and runtime (trait + runtime hooks).
- Move `CompiledModuleStatic` from `crates/slicer-runtime/src/execution_plan.rs` (where it briefly lived after P83's rename) into `crates/slicer-scheduler/src/execution_plan.rs`. Delete the `pub type CompiledModule = CompiledModuleStatic;` transitional alias.

After this step, `slicer-scheduler` builds standalone; `slicer-runtime` does not yet (it still has stale `pub mod` declarations).

**Precondition.** Step 3 complete.

**Postcondition.** `cargo build -p slicer-scheduler` green. `slicer-runtime` build fails (expected; fixed in Step 5).

**Files allowed to read.** `crates/slicer-runtime/src/instrumentation.rs` (line ranges), `crates/slicer-runtime/src/execution_plan.rs` (lines around `CompiledModuleStatic` declaration and the alias).
**Files allowed to edit.**
1. `crates/slicer-scheduler/src/instrumentation.rs` — fill with planning-side content from runtime's instrumentation.rs (`compute_serial_edges_for_stage`, `EdgeReason`, `SerialEdge`, supporting types).
2. `crates/slicer-runtime/src/instrumentation.rs` — truncate to runtime-side content (`PipelineInstrumentation` trait, `Phase`, `TierKind`, `compute_serial_edges_from_compiled`, runtime-bracket-hook scaffolding). Update internal `use` paths if `EdgeReason`/`SerialEdge` are still referenced from runtime fn signatures (`use slicer_scheduler::{EdgeReason, SerialEdge};`).
3. `crates/slicer-scheduler/src/execution_plan.rs` (already a verbatim copy from Step 3) — already contains `CompiledModuleStatic`. Delete the `pub type CompiledModule = CompiledModuleStatic;` alias if it's still in the copied content.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-scheduler`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** `slicer-scheduler` builds. `rg 'pub type CompiledModule =' crates/` returns empty. `rg -l 'pub struct CompiledModuleStatic' crates/` → exactly `crates/slicer-scheduler/src/execution_plan.rs`.

**Falsifying check / exit condition.** Scheduler build fails on missing internal types → trace the `use` chain.

---

## Step 5 — Rewire `slicer-runtime/src/lib.rs`, Cargo.toml; rewire `slicer-wasm-host` borrow + Cargo.toml; rewire `pnp-cli`

**Objective.** Workspace builds. `slicer-wasm-host` borrows `&'s slicer_scheduler::CompiledModuleStatic`. `pnp-cli`'s `dag` subcommand calls into `slicer-scheduler::run_dag_*`.

**Precondition.** Step 4 complete; scheduler builds.

**Postcondition.** `cargo build --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` green.

**Files allowed to read.** `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/Cargo.toml`, `crates/slicer-wasm-host/src/binding.rs` (or wherever `CompiledModuleLive` lives), `crates/slicer-wasm-host/Cargo.toml`, `crates/pnp-cli/src/main.rs` (or its dag-subcommand module), `crates/pnp-cli/Cargo.toml`.
**Files allowed to edit.**
1. `crates/slicer-runtime/src/lib.rs` — drop 9 `pub mod` declarations; add `pub use slicer_scheduler::{...}` transitional re-exports per dispatch #3's findings; keep `pub mod instrumentation;`.
2. `crates/slicer-runtime/Cargo.toml` — add `slicer-scheduler` dep; drop `toml` IF dispatch #2 confirmed no other runtime consumer.
3. `crates/slicer-wasm-host/src/binding.rs` — change borrow type to `&'s slicer_scheduler::CompiledModuleStatic`.
4. `crates/slicer-wasm-host/Cargo.toml` — add `slicer-scheduler` dep.
5. `crates/pnp-cli/src/main.rs` (or dag subcommand file) — rewire `use slicer_runtime::run_dag_*` to `use slicer_scheduler::run_dag_*`.
6. `crates/pnp-cli/Cargo.toml` — add `slicer-scheduler` dep.
7. `crates/slicer-runtime/src/builtins/*` (from P84) — if any wrapper does `use crate::dag::BuiltinProducer;` or similar, rewrite to `use slicer_scheduler::dag::BuiltinProducer;` (or via the transitional re-export).

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail + first failing crate.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** Both green. `grep -qE '^slicer-scheduler *=' crates/slicer-runtime/Cargo.toml` AND `crates/slicer-wasm-host/Cargo.toml` AND `crates/pnp-cli/Cargo.toml`.

**Falsifying check / exit condition.** Build error on a missing re-export → consult dispatch #3 and add the named symbol to the transitional `pub use` block.

---

## Step 6 — Migrate or rewire `slicer-runtime/tests/` per dispatch #4

**Objective.** Plan-shape regression tests run in `slicer-scheduler/tests/` (no wasmtime); runtime tests that consume `ExecutionPlan` continue passing.

**Precondition.** Step 5 complete.

**Postcondition.** `cargo test -p slicer-scheduler -p slicer-runtime -p slicer-wasm-host -p pnp-cli` green.

**Files allowed to read.** Test files from Step 1 dispatch #4.
**Files allowed to edit.**
1. Move test files whose SUT is a moved symbol into `crates/slicer-scheduler/tests/`.
2. Update imports in non-moved runtime tests: `slicer_runtime::ExecutionPlan` → `slicer_scheduler::ExecutionPlan` (or rely on the transitional re-export).
3. `crates/slicer-runtime/tests/{integration,executor}/main.rs` — drop `mod` declarations for moved tests.

**Expected sub-agent dispatches.**
- Dispatch: `cargo test -p slicer-scheduler`. Return FACT pass/fail + count.
- Dispatch: `cargo test -p slicer-runtime`. Return FACT pass/fail + count + delta vs Step 0 baseline.
- Dispatch: `cargo test -p slicer-wasm-host`. Return FACT pass/fail.
- Dispatch: `cargo test -p pnp-cli`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** All four green. `slicer-runtime` count delta = -(tests moved); `slicer-scheduler` count delta = +(same count + new tests if any).

**Falsifying check / exit condition.** A test fails on missing import → add the symbol to the runtime's `pub use slicer_scheduler::*;` transitional block.

---

## Step 7 — Confirm guest WASMs stay clean (AC: `cargo xtask build-guests --check`)

**Objective.** Verify no guest-feeding path was inadvertently edited.

**Precondition.** Step 6 green.

**Postcondition.** `cargo xtask build-guests --check` reports zero STALE.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo xtask build-guests --check`. Return FACT pass/fail + STALE list.

**Context cost: S.**

**Narrow verification.** Clean.

**Falsifying check / exit condition.** STALE → investigate; do not rebuild without identifying which P85 edit touched a guest-feeding path (shouldn't have happened).

---

## Step 8 — Workspace test gate (checkpoint)

**Objective.** Full ~1 000-test suite passes per the deepening-batch policy.

**Precondition.** Steps 1–7 green.

**Postcondition.** `cargo test --workspace` green. Count delta vs Step 0 baseline within ±1 (any larger delta investigated and explained in the log).

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test --workspace 2>&1 | tail -5`. Return SNIPPET. Then FACT pass/fail + count delta + duration.

**Context cost: M.**

**Narrow verification.** Pass.

**Falsifying check / exit condition.** Any failure → triage by test name. Most likely cause: a test that imports `slicer_runtime::*` for a symbol now only in `slicer-scheduler`; fix the import OR add the transitional re-export.

---

## Step 9 — Post-packet SHA parity for g-code + dag commands

**Objective.** Confirm AC-9 (g-code SHA matches Step 0 baseline) AND AC-10 (dag stages/claims SHAs match).

**Precondition.** Step 8 green.

**Postcondition.** Three post-packet SHAs equal three Step 0 baselines.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: post-packet g-code SHA. Compare to Step 0 g-code baseline.
- Dispatch: post-packet `pnp_cli dag stages` SHA. Compare to Step 0 baseline.
- Dispatch: post-packet `pnp_cli dag claims` SHA. Compare to Step 0 baseline.

**Context cost: S.**

**Narrow verification.** All three match.

**Falsifying check / exit condition.** Any mismatch → bisect; the divergence is in one of the moved files (most likely candidate: `validation.rs` if claim-ordering changes, OR `dag_cli.rs`'s output format).

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 P83 verification + 4 baselines | S |
| 1 Enumerate consumers | S |
| 2 Crate scaffold | S |
| 3 Verbatim move (9 files) | M |
| 3.5 Live-cluster relocation + wasmtime-field strip + callsite rewire | M |
| 4 Instrumentation split + CompiledModule alias delete | M |
| 5 Runtime + wasm-host + pnp-cli rewire | M |
| 6 Test migration | M |
| 7 Guest --check clean | S |
| 8 Workspace test gate | M |
| 9 SHA parity (g-code + dag stages + dag claims) | S |

Aggregate: **L overall but no single step is L.** Total step count: 11 (Step 3.5 added mid-flight after P83's incomplete Static/Live field migration surfaced during Step 4 execution — recorded in design.md §Risks).

## Packet Completion Gate

Checkpoint packet — workspace tests run at close per the deepening-batch policy.

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo xtask build-guests --check` — clean.
4. `cargo test --workspace` — green; count delta within ±1 vs Step 0 baseline.
5. AC-9 post-packet g-code SHA = Step 0 baseline.
6. AC-10 post-packet `dag stages` and `dag claims` SHAs = Step 0 baselines.
7. AC-N2 `cargo tree -p slicer-scheduler` contains no `wasmtime`.
8. ADR-0006 drafted in `docs/adr/` and reviewed before status flip.

## Acceptance Ceremony

- All 11 ACs (AC-1 .. AC-11) and 3 negative cases (AC-N1, AC-N2, AC-N3) gate green per the inline verification commands in `packet.spec.md`.
- ADR-0006 (`docs/adr/0006-compiled-module-static-live-split.md`) committed.
- Implementation log records: Step 0 baselines, Step 8 workspace test count + duration, Step 9 post-packet SHAs, list of files moved, list of transitional `pub use slicer_scheduler::*;` re-exports added to `slicer-runtime/src/lib.rs` (for the follow-up cleanup packet).
- `status: draft` → `status: superseded` after gate green AND ADR in place AND user confirms closure.
