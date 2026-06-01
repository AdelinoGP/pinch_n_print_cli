# Packet 83 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- Files allowed to read per step are bounded. `wit_host.rs` (5 259 LOC) and `dispatch.rs` (3 148 LOC) are NEVER loaded in full; section-by-section grep + ±40-line reads only.
- The packet closure gate runs `cargo test --workspace` per the deepening-batch checkpoint policy; partial test runs are not sufficient.
- P81 AND P82 MUST be closed before this packet starts (Step 0 verifies).
- The schema edit triggers guest WASM staleness; Step 7 rebuilds guests and re-verifies `--check` clean before Step 8 runs the full test suite.

---

## Step 0 — Verify P81 and P82 closure, capture pre-packet baselines

**Objective.** Confirm both predecessors are closed and the workspace is in the expected post-P82 state. Capture pre-packet g-code SHA (carries forward from P81's baseline if no semantic change occurred in P82 — Step 0 explicitly re-checks).

**Precondition.** P81 and P82 statuses are `superseded`. Working tree clean.

**Postcondition.** Baselines in the implementation log: g-code SHA against `resources/benchy.stl`; `cargo test --workspace` pre-packet pass count.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: confirm `test ! -f crates/slicer-runtime/src/{cli,helpers_cmd,model_loader,model_loader_sidecar,model_writer}.rs && test -f crates/slicer-model-io/Cargo.toml`. Return FACT pass/fail.
- Dispatch: g-code SHA. `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/p83-baseline.gcode && sha256sum /tmp/p83-baseline.gcode`. Return FACT `<hex>`.
- Dispatch: `cargo test --workspace 2>&1 | tail -5`. Return SNIPPET (≤ 5 lines containing the test count + final result).

**Context cost: S.**

**Narrow verification.** P81+P82 verification pass. Baseline SHA captured. Pre-packet test count recorded.

**Falsifying check / exit condition.** Any verification fails → abort; the predecessor work is incomplete.

---

## Step 0.5 — Prework: relocate stage I/O types to `slicer-ir` + investigate `LoadedModule`

**Objective.** Resolve the AC-N3 vs trait-signature contradiction (see design.md "Borrow-struct pattern for trait inputs") by moving the eight stage I/O types out of `slicer-runtime` and into `slicer-ir` so the runner traits — which move to `slicer-wasm-host` in Step 4 — can compile inside `slicer-wasm-host` without a back-edge dep on `slicer-runtime`. Investigate `slicer-runtime::manifest::LoadedModule` to decide whether it moves whole-cloth or splits Static/Live.

**Precondition.** Step 0 green. (In a session resuming from a partially-implemented packet, Steps 2/3 may already be done — that does not invalidate this step; it can still run before Step 4.)

**Postcondition.** All eight stage I/O types defined in `slicer-ir`. Transitional `pub use slicer_ir::{...}` re-exports added in `crates/slicer-runtime/src/lib.rs` so existing import sites compile unchanged. `cargo build --workspace` green. `LoadedModule` decision recorded in the implementation log: either (a) move-whole-to-wasm-host (deferred to Step 4), or (b) Static/Live split with `LoadedModuleStatic` + `LoadedModuleLive` and transitional `pub type LoadedModule = LoadedModuleStatic;` alias.

**Files allowed to read.**
- `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — grep-only, locate the 8 stage-I/O type defs.
- `crates/slicer-runtime/src/manifest.rs` — full file (≤ 500 LOC OK; load to inspect `LoadedModule`).
- `crates/slicer-ir/src/lib.rs` — find appropriate insertion module for the new types.

**Files allowed to edit (≤ 6).**
1. `crates/slicer-ir/src/stage_io.rs` — CREATE (or distribute the 8 types across existing IR modules — implementer's call). The 8 types: `LayerStageOutput`, `LayerStageError`, `PrepassStageOutput`, `PrepassExecutionError`, `FinalizationOutput`, `FinalizationError`, `PostpassOutput`, `PostpassError`.
2. `crates/slicer-ir/src/lib.rs` — `pub mod stage_io; pub use stage_io::*;`.
3. `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — DELETE the local type defs; replace with `use slicer_ir::{...};`.
4. `crates/slicer-runtime/src/lib.rs` — add `pub use slicer_ir::{LayerStageOutput, LayerStageError, ...};` transitional re-exports so external sites (tests, benches, downstream callers) compile unchanged.

**Expected sub-agent dispatches.**
- Dispatch: enumerate the 8 type defs in the four executor files; report file:line and full field/variant lists (SNIPPETS).
- Dispatch: read `crates/slicer-runtime/src/manifest.rs::LoadedModule` definition; report fields, derived traits, and any direct wasmtime references (SNIPPET ≤ 60 lines).
- Dispatch: execute the moves + re-exports. Return FACT pass/fail on `cargo build --workspace` and `cargo build -p slicer-ir`.

**Context cost: S.**

**Narrow verification.** `cargo build -p slicer-ir` green. `cargo build --workspace` green. `grep -rE 'pub (enum|struct) (LayerStageOutput|LayerStageError|PrepassStageOutput|PrepassExecutionError|FinalizationOutput|FinalizationError|PostpassOutput|PostpassError)' crates/slicer-ir/` returns 8 hits. `grep -rE 'pub (enum|struct) (LayerStageOutput|LayerStageError|PrepassStageOutput|PrepassExecutionError|FinalizationOutput|FinalizationError|PostpassOutput|PostpassError)' crates/slicer-runtime/src/` returns 0 hits.

**Falsifying check / exit condition.** If any of the 8 types has a field whose type is `slicer-runtime`-internal (e.g., something that itself needs to move), surface the chain and stop — moving the field's type may or may not be in scope, and the planner must decide before proceeding. If the 8 types only reference `std`, `slicer-ir`-resident types, `slicer-sdk` types, or primitive Rust types, the move is clean.

---

## Step 1 — Locate trait defs, callers, and side imports

**Objective.** Build the precise lists of edit sites the packet will touch.

**Precondition.** Step 0 green.

**Postcondition.** Four lists in the implementation log:
- (a) The four `pub trait *StageRunner` declarations with exact file:line and full signatures.
- (b) Test files importing `wit_host::*`, `dispatch::*`, `wasm_instance::*`, `instance_pool::*`, or constructing `CompiledModule` directly.
- (c) Non-moving runtime files referencing the same.
- (d) Callers of `dispatch::export_name_for_stage` (expected ≤ 3 sites).

**Files allowed to read.** None directly — four dispatches.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch #1 (= design.md dispatch #1): trait declarations and signatures. SNIPPETS (4 snippets, ≤ 30 lines each).
- Dispatch #2: tests touching the four modules or constructing `CompiledModule`. LOCATIONS.
- Dispatch #3: non-moving runtime files referencing the four modules. LOCATIONS.
- Dispatch #4: `export_name_for_stage` callers. LOCATIONS.

**Context cost: S.**

**Narrow verification.** Four lists populated.

**Falsifying check / exit condition.** A site surfaces at `cargo build` in step 5 that isn't on any list → return here and widen.

---

## Step 2 — Add `export_for_stage_id` to `slicer-schema` + its test

**Objective.** Introduce the consolidated lookup before deleting the duplicate in `dispatch.rs`.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `slicer-schema` builds with the new fn. The new test (`tests/export_for_stage_id_tdd.rs`) passes. Guest WASMs are now stale (per CLAUDE.md; the schema edit invalidates them).

**Files allowed to read.** `crates/slicer-schema/src/lib.rs` (full file ≤ 390 LOC — OK).
**Files allowed to edit.**
1. `crates/slicer-schema/src/lib.rs` — add `pub fn export_for_stage_id`.
2. `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` — CREATE the lookup test.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build -p slicer-schema && cargo test -p slicer-schema`. Return FACT pass/fail.
- Dispatch: `cargo xtask build-guests --check`. Expected: STALE for all guests (because the schema edit invalidates them).

**Context cost: S.**

**Narrow verification.** Schema builds; test passes; `--check` reports STALE (this is expected — Step 7 rebuilds).

**Falsifying check / exit condition.** Schema doesn't build → fix the fn signature/body.

---

## Step 3 — Create the `slicer-wasm-host` crate scaffold

**Objective.** New crate exists; empty `lib.rs` compiles; workspace `Cargo.toml` knows about it.

**Precondition.** Step 2 green.

**Postcondition.** `cargo build -p slicer-wasm-host` succeeds against an empty lib.rs scaffold.

**Files allowed to read.** Workspace `Cargo.toml`, `crates/slicer-runtime/Cargo.toml` (to copy `wasmtime` version).
**Files allowed to edit.**
1. Workspace `Cargo.toml` — add `"crates/slicer-wasm-host"` to `members`.
2. `crates/slicer-wasm-host/Cargo.toml` — CREATE with `wasmtime`, `slicer-ir`, `slicer-schema`, `slicer-sdk`.
3. `crates/slicer-wasm-host/src/lib.rs` — CREATE with module declarations only (`pub mod host; pub mod dispatch; pub mod instance; pub mod pool; pub mod traits; pub mod binding;`) and empty submodule files.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-wasm-host`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Crate builds (empty).

**Falsifying check / exit condition.** Workspace inheritance issue for `wasmtime` → check `workspace.dependencies` in root `Cargo.toml`.

---

## Step 4 — Move the four files and the four trait defs; preserve `bindgen!` `with:` remap

**Objective.** The bulk move. After this step, AC-2, AC-3, AC-4 gate green (file presence/absence and bindgen-count checks). The workspace does NOT yet build — `slicer-runtime` still has stale `pub mod` declarations and old trait imports. Step 5 fixes those.

**Precondition.** Step 3 complete.

**Postcondition.** All four files relocated. The four trait defs in `crates/slicer-wasm-host/src/traits.rs`. `pub struct CompiledModuleLive<'s>` in `crates/slicer-wasm-host/src/binding.rs`. `dispatch.rs::export_name_for_stage` deleted; its callers (per Step 1 dispatch #4) switched to `slicer_schema::export_for_stage_id`. The four `impl *StageRunner for WasmRuntimeDispatcher` blocks updated to take `&CompiledModuleLive<'_>`.

**Files allowed to read.**
- `crates/slicer-runtime/src/wit_host.rs` — line-range ONLY. Grep for `bindgen!`, `impl ... for HostExecutionContext`, `pub struct`. Move section-by-section, never loading > 200 lines at a time.
- `crates/slicer-runtime/src/dispatch.rs` — line-range ONLY. Focus on L1–80 (imports + `export_name_for_stage`), L340–360 (`WasmRuntimeDispatcher` struct), and the four impl blocks per Step 1 dispatch.
- `crates/slicer-runtime/src/wasm_instance.rs` (299 LOC — OK to load full).
- `crates/slicer-runtime/src/instance_pool.rs` (182 LOC — OK to load full).
- The four trait-source files (executor / prepass / postpass / layer_finalization) at the lines surfaced by Step 1 dispatch #1.

**Files allowed to edit.**
1. `crates/slicer-wasm-host/src/host.rs` — receive `wit_host.rs` content.
2. `crates/slicer-wasm-host/src/dispatch.rs` — receive `dispatch.rs` content minus `export_name_for_stage`; impls updated to `&CompiledModuleLive<'_>`.
3. `crates/slicer-wasm-host/src/instance.rs` — receive `wasm_instance.rs` content.
4. `crates/slicer-wasm-host/src/pool.rs` — receive `instance_pool.rs` content.
5. `crates/slicer-wasm-host/src/traits.rs` — declare the four runner traits (lifted from executor / prepass / postpass / layer_finalization).
6. `crates/slicer-wasm-host/src/binding.rs` — declare `CompiledModuleLive<'s>`.
7. `crates/slicer-wasm-host/src/lib.rs` — public re-exports.
8. Delete the four files in `crates/slicer-runtime/src/`.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build -p slicer-wasm-host`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** `cargo build -p slicer-wasm-host` green. `grep -rE 'wasmtime::component::bindgen!' crates/slicer-wasm-host/src/` returns 4. `grep -rE '"slicer:types/geometry": super::layer::slicer::types::geometry' crates/slicer-wasm-host/src/` returns 3.

**Falsifying check / exit condition.** Bindgen path resolution fails → confirm `pub mod layer` (or whichever module owns the layer bindgen) is declared FIRST in `lib.rs`.

---

## Step 5 — Update `slicer-runtime`: remove `pub mod`s, rewire trait imports, split `CompiledModule`, swap Cargo.toml deps

**Objective.** Make `slicer-runtime` compile against the new crate. After this step, the workspace builds.

**Precondition.** Step 4 complete; `slicer-wasm-host` builds.

**Postcondition.** `cargo build --workspace` green. `slicer-runtime` no longer declares `wasmtime` directly. `CompiledModule` renamed to `CompiledModuleStatic` with `pub type CompiledModule = CompiledModuleStatic;` alias. The four executor files import their runner trait from `slicer_wasm_host::`.

**Files allowed to read.**
- `crates/slicer-runtime/src/lib.rs` (full).
- `crates/slicer-runtime/src/execution_plan.rs` (L650–730 around `CompiledModule`).
- `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — only the trait declaration lines + their callers (already located in Step 1).
- `crates/slicer-runtime/src/dag_cli.rs` — only if Step 1 dispatch #4 surfaced it as a caller of `export_name_for_stage`.
- `crates/slicer-runtime/Cargo.toml`.

**Files allowed to edit.**
1. `crates/slicer-runtime/src/lib.rs` — drop four `pub mod`s + their `pub use`s; add `pub use slicer_wasm_host::{…}` re-exports.
2. `crates/slicer-runtime/src/execution_plan.rs` — rename struct; delete wasmtime fields/accessors; add type alias.
3. `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — delete local trait decl; add `use slicer_wasm_host::*StageRunner;`.
4. `crates/slicer-runtime/Cargo.toml` — remove `wasmtime`; add `slicer-wasm-host`.
5. `crates/slicer-runtime/src/dag_cli.rs` — switch `export_name_for_stage` calls to `slicer_schema::export_for_stage_id` if needed.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail + first failing crate.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** Workspace builds; clippy clean.

**Falsifying check / exit condition.** Build error referencing a missing trait → confirm the runner-trait `use slicer_wasm_host::*StageRunner;` line in each executor file.

---

## Step 6 — Migrate or rewire tests in `slicer-runtime/tests/`

**Objective.** Tests that imported `wit_host::*`, `dispatch::*`, etc., now import from `slicer_wasm_host::`. Tests that constructed `CompiledModule` with `instance_pool: ...` fields now construct `CompiledModuleStatic` + a separate `CompiledModuleLive`.

**Precondition.** Step 5 complete.

**Postcondition.** `cargo test -p slicer-runtime` green (without `--workspace`).

**Files allowed to read.** The test files surfaced in Step 1 dispatch #2.
**Files allowed to edit.** Those same test files; `crates/slicer-runtime/tests/{integration,executor}/main.rs` aggregators if any `mod` declarations need adjusting.

**Expected sub-agent dispatches.**
- Dispatch: `cargo test -p slicer-runtime`. Return FACT pass/fail + count + delta vs pre-packet.

**Context cost: M.**

**Narrow verification.** Test runs pass; count delta near zero.

**Falsifying check / exit condition.** A test that previously passed now fails on type-construction → the `CompiledModuleStatic`/`Live` split needs more work.

---

## Step 7 — Rebuild guest WASMs and confirm `--check` clean

**Objective.** Step 2's schema edit invalidated guests; rebuild them. Confirm `--check` reports zero STALE after rebuild.

**Precondition.** Step 5 complete (otherwise the host build doesn't even succeed).

**Postcondition.** `cargo xtask build-guests --check` reports clean for all guests.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `cargo xtask build-guests`. Return FACT pass/fail + duration.
- Dispatch: `cargo xtask build-guests --check`. Return FACT clean/STALE-list.

**Context cost: S.**

**Narrow verification.** Both dispatches green; `--check` reports no STALE entries.

**Falsifying check / exit condition.** A guest fails to build → the schema edit was wrong; investigate (likely cause: a typo in `export_for_stage_id` body).

---

## Step 8 — Workspace test gate (checkpoint)

**Objective.** Confirm the full ~1 000-test suite passes. Per the deepening-batch policy, this gate runs at P83 (checkpoint).

**Precondition.** Steps 1–7 green.

**Postcondition.** `cargo test --workspace` passes; count delta vs Step 0 baseline within +1/-1 (any larger delta investigated and explained in the log).

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test --workspace 2>&1 | tail -5`. Return SNIPPET (≤ 5 lines: count + final result). Then FACT pass/fail + count delta.

**Context cost: M.**

**Narrow verification.** Pass; count delta within ±1.

**Falsifying check / exit condition.** Any test fails → triage by test name. Likely causes (in order of probability): (1) stale guest from Step 7 missed; (2) `CompiledModule` construction site in a test missed in Step 6; (3) `wasmtime::` direct import in a non-moved file missed in Step 5; (4) bindgen `with:` remap pattern subtly wrong.

---

## Step 9 — Post-packet g-code SHA parity and AC-8 dep-tree assertion

**Objective.** Confirm the byte-identical g-code SHA carries through; confirm `slicer-runtime` has no direct wasmtime dep.

**Precondition.** Step 8 green.

**Postcondition.** Post-packet SHA = Step 0 baseline SHA. `cargo tree -p slicer-runtime --depth 1` does NOT list `wasmtime`.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p83.gcode && sha256sum /tmp/benchy-p83.gcode`. Return FACT `<hex>`.
- Dispatch: `cargo tree -p slicer-runtime --depth 1 --edges normal`. Return SNIPPET (≤ 30 lines).

**Context cost: S.**

**Narrow verification.** SHAs match. Dep-tree depth-1 listing contains `slicer-wasm-host`, does NOT contain `wasmtime`.

**Falsifying check / exit condition.** SHA divergence → bisect via temporary reverts of Step 4 sections to isolate the divergent edit. Dep-tree mentions `wasmtime` directly → check `crates/slicer-runtime/Cargo.toml` for stray `wasmtime = ...` line.

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 Verify P81/P82 + baselines | S |
| 0.5 Prework: stage I/O → slicer-ir + LoadedModule survey | S |
| 1 Enumerate edit sites | S |
| 2 Schema `export_for_stage_id` + test | S |
| 3 New crate scaffold | S |
| 4 Bulk move + bindgen relocation + trait lift + struct split | M |
| 5 Runtime rewire + Cargo.toml swap | M |
| 6 Test migration / rewiring | M |
| 7 Guest rebuild + `--check` clean | S |
| 8 Workspace test gate | M |
| 9 g-code SHA + dep-tree check | S |

Aggregate: **L overall but no single step is L.** Total step count: 11 (Step 0.5 prework added to resolve the AC-N3 vs trait-signature contradiction surfaced mid-implementation).

## Packet Completion Gate

This is a checkpoint packet — workspace tests run at close per the deepening-batch policy.

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo xtask build-guests` (rebuild) green, then `cargo xtask build-guests --check` clean.
4. `cargo test --workspace` — green; count delta within ±1.
5. AC-9 post-packet SHA = Step 0 baseline.
6. AC-8 dep-tree depth-1 listing does NOT include `wasmtime`.
7. ADR-0004 and ADR-0005 drafted in `docs/adr/` and reviewed before status flip.

## Acceptance Ceremony

- All 11 ACs (AC-1 .. AC-11) and 3 negative cases (AC-N1, AC-N2, AC-N3) gate green per the inline verification commands in `packet.spec.md`.
- ADR-0004 (`docs/adr/0004-runner-traits-in-slicer-wasm-host.md`) and ADR-0005 (`docs/adr/0005-export-for-stage-id-sole-lookup.md`) written and committed.
- Implementation log records: Step 0 baseline SHA, Step 9 post-packet SHA, pre/post workspace test counts, guest-rebuild duration, list of files moved (count and total LOC), list of `pub trait *StageRunner` declarations lifted (4 expected), list of `export_name_for_stage` call sites collapsed (per Step 1 dispatch #4).
- `status: draft` → `status: superseded` once gate green AND ADRs in place AND user confirms closure.
