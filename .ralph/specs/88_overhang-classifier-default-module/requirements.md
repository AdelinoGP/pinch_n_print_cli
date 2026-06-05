# Packet 88 — Requirements

## Problem Statement

After P84 / P86 / P87, the overhang-classification kernel lives in `slicer-core`, `FeedrateConfig` lives in `slicer-ir`, and `slicer-gcode`'s `emit_gcode` body calls `slicer_core::classify_layers` directly — a host-only path. This works, but it bakes overhang-feedrate selection into the host serializer, leaving zero swap-point for users who want different overhang behavior (different angle thresholds, different quartile counts, non-linear speed mappings, per-feature curves, etc.). The deepening-plan grilling's Q3 resolved that overhang classification is a real swap point worth modularising — but Q3's exploration also found the existing `world-finalization::run-finalization` export already provides everything needed: `list<layer-collection-view>` input + `modify-entity(entity_id, set-speed-factor)` output. No new stage, no new WIT export, no contract churn for the 20 existing guest modules.

P88 ships `modules/core-modules/overhang-classifier-default/` — a `FinalizationModule`-implementing WASM module that uses the same `#[slicer_module]` macro and SDK patterns as the existing 20 core-modules. It reads the four overhang-speed config fields from `config-view`, calls `slicer_core::classify_layers`, and emits `set-speed-factor` mutations for each wall entity in a non-Q4 (overhang) quartile. `slicer-gcode`'s direct `classify_layers` call is deleted; the emit path already consumes `set-speed-factor` annotations (the existing `finalization-default` module exercises that path for non-overhang reasons).

Q6 resolution preserved: ship the module; no host fallback. The module ships under `modules/core-modules/`, so the standard `--module-dir modules/core-modules` invocation loads it and behavior matches the pre-batch baseline. Users who curate a different module dir without `overhang-classifier-default` get NO overhang annotation — they opted out by curating.

This is the final packet in the deepening batch; the workspace test gate at close is the batch's final acceptance ceremony.

## Grouped Task IDs

- **TASK-238** (new) — Ship `overhang-classifier-default` core-module; delete host's direct `classify_layers` call. Final task in "Architecture Deepening Phase II".

## In Scope

- Create `modules/core-modules/overhang-classifier-default/` with the standard core-module layout (mirror `modules/core-modules/seam-planner-default/` or any other of the 20 existing core-modules — confirm via dispatch #1; note **no `finalization-default/` exists** in the workspace):
  - `Cargo.toml` declaring `slicer-sdk`, `slicer-schema`, `slicer-ir` as path deps plus the wasm32-only `wit-bindgen` workspace dep. **MUST NOT** depend on `slicer-core`, `slicer-runtime`, `slicer-wasm-host`, `slicer-scheduler`, `slicer-gcode`, `slicer-model-io`, or `wasmtime` — the `host-algos` feature gate on `slicer-core` would contaminate the guest dep tree (P84 lesson).
  - `overhang-classifier-default.toml` (match the existing `<module-name>.toml` manifest filename convention) declaring `stage = "PostPass::LayerFinalization"`, `trait = "FinalizationModule"`, `world = "slicer:world-finalization@1.0.0"`. Config-keys section declares reads of `overhang_1_4_speed`, `overhang_2_4_speed`, `overhang_3_4_speed`, `overhang_4_4_speed` (key names exactly as in `slicer-ir::FeedrateConfig`).
  - **`src/` containing the COMPLETE relocated algorithm** (the user's central catch — no `slicer_core::*` imports anywhere under guest src):
    - `src/lib.rs` with the `#[slicer_module]` impl `FinalizationModule`. Body shape:
      1. Read the four overhang-speed fields via `config_view.get_float(...)` (or the SDK's typed accessor).
      2. Short-circuit when all four are 0.0 (preserves the pre-P84 byte-identical baseline for unconfigured printers).
      3. For each `layer_index` ≥ 1, iterate the per-layer entity stream filtered to wall roles. For each wall entity: compute signed distance to the previous layer's wall geometry via the relocated `LinesDistancer2D` primitive (in `src/lines_distancer.rs`), classify into Q1–Q4 via the relocated `classify_layers` kernel (in `src/classify.rs`), compute `factor = overhang_<q>_4_speed / base_speed_for_role(role, &feedrate_config)` for non-Q4 entities, and call `output.modify_entity(layer_index, entity.entity_id, EntityMutation::SetSpeedFactor(factor))`. Q4 (fully supported) entities emit nothing (default factor 1.0 applies).
    - `src/classify.rs` — relocate the verbatim 319 LOC of `crates/slicer-core/src/algos/overhang_classifier.rs` (the `classify_layers` algorithm and its private helpers). Adjust imports: instead of `use crate::aabb_lines_2d::LinesDistancer2D`, use `use crate::lines_distancer::LinesDistancer2D`. The `slicer_ir::FeedrateConfig` and `slicer_ir::{ExtrusionRole, LayerCollectionIR}` imports stay (slicer-ir IS a guest dep).
    - `src/lines_distancer.rs` — relocate the `LinesDistancer2D` primitive (currently `crates/slicer-core/src/aabb_lines_2d.rs`). If it pulls in any `slicer-core`-only helper, audit and copy that too — the guest must be `slicer_core::*`-free.
  - `tests/basic_tdd.rs` with `#[module_test]` (per the post-P78 SDK convention) — at minimum subsumes the invariants from the P84 golden `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` (which is being deleted in Step 5.5).
- **Delete the slicer-core kernel and its host-side shims** (Step 5.5):
  - `crates/slicer-core/src/algos/overhang_classifier.rs` — DELETE.
  - `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` — DELETE.
  - `crates/slicer-core/src/algos/mod.rs` — drop the `pub mod overhang_classifier;` declaration and any re-export naming `classify_layers`.
  - `crates/slicer-runtime/src/lib.rs:192`'s `pub use slicer_core::algos::overhang_classifier::classify_layers;` — DELETE (P84-era compat shim).
  - `crates/slicer-core/src/aabb_lines_2d.rs` — DELETE if no other slicer-core algo consumes `LinesDistancer2D` (Step 5.5 dispatch verifies; if it has consumers, keep with a note).
- **Delete the direct call AND the obsolete branch from `crates/slicer-gcode/src/emit.rs`**:
  - Delete `use slicer_core::classify_layers;` at L21 and the `classify_layers(&mut layers, &feedrate_config);` call at L226.
  - Delete the `overhang_quartile`-indexed feedrate-lookup branch in `resolve_feedrate` (L106-123 — the `overhang_<q>_4_speed` table indexed by quartile). The multiplicative `set-speed-factor` consumption path (already used by existing finalization-implementing modules — Step 1 dispatch #4 confirms) handles all factoring.
  - If `feedrate_config: FeedrateConfig` field becomes entirely unused, drop it from the struct and any constructor params.
- Add `modules/core-modules/overhang-classifier-default` to the workspace `Cargo.toml`'s `members` list (if xtask discovers modules by `members`; otherwise update xtask's explicit module list — confirm via dispatch #2).
- Run `cargo xtask build-guests` to produce the new `.wasm` artifact. After successful build, `--check` reports clean.
- Capture the AC-7 SHA post-packet. If byte-identical to P87 baseline (`89a329ad…`), document. If LSB-shifted, document the shifted SHA AND the F-word diff scope as the rationale (set-speed-factor multiplicative rounding vs direct overhang-speed lookup).
- Workspace test gate: `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace` green; closes the deepening batch.

## Out of Scope

- `crates/slicer-test/`, `crates/slicer-sdk/` — concurrent work.
- New WIT exports, new stage definitions in `slicer-schema::STAGES`, edits to `crates/slicer-schema/wit/**/*.wit`. AC-N3 verifies.
- Reintroducing a host builtin for overhang annotation (Q6 resolution: ship the module; no host fallback). AC-N2 verifies.
- Refactoring `slicer-gcode`'s `resolve_feedrate` logic. The `set-speed-factor` consumption path already exists (existing `finalization-default` module exercises it); P88 just routes overhang through it.
- Changing the `overhang_quartile` enum or the `FeedrateConfig` field names. They are preserved exactly (`overhang_<q>_4_speed`).
- Documenting the user-override workflow ("here's how to write your own overhang-classifier") — that lives in a future doc packet, not P88.
- Adding `set-flow-factor` mutations to compensate for slowed-down extrusion. The default behavior matches pre-P88: pure speed reduction, no flow compensation. (A future enhancement might add flow compensation; not in scope here.)

## Authoritative Docs

- `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` (130 LOC, OK to load full) — confirms the `entity-mutation::set-speed-factor(f32)`, `finalization-output-builder.modify-entity`, `layer-collection-view.ordered-entities` shapes the new module relies on.
- `docs/05_module_sdk.md` — `FinalizationModule` trait, `#[slicer_module]` macro, manifest schema. Confirm the conventions the new module follows.
- `docs/04_host_scheduler.md` — `PostPass::LayerFinalization` stage; confirms multiple modules can run in the stage sequentially (claim-based ordering in the DAG).
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — preserved.
- `modules/core-modules/finalization-default/` — the template / pattern (read its Cargo.toml + module.toml + lib.rs sketch only; do NOT load the full file). Confirms manifest shape, `#[slicer_module]` usage, output-builder access pattern.
- `crates/slicer-runtime/src/overhang_classifier.rs` (pre-P84; now in `slicer-core/src/algos/overhang_classifier.rs`) — the quartile convention reference and the AC-2 short-circuit baseline. Read lines 1–60 only (the docstring and the entry-fn header) to confirm the convention.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-9, AC-N1..AC-N3). Measurable refinements:

- **AC-7 — Byte-identical OR documented LSB shift**: implementer commits to one or the other. If byte-identical is unachievable after reasonable effort (e.g., the factor-rounding investigation in Step 6), the new baseline SHA is captured AND the F-word-only diff scope is verified (no path geometry change, no extrusion-amount change).
- **AC-9 — Workspace test gate**: this is the final batch ceremony. Dispatch the run to a sub-agent for FACT pass/fail + duration + count. Count delta vs P85 baseline within ±10 (allowing for migrations P86, P87, P88's golden tests).
- **AC-8 — Module-level test**: must use `#[module_test]` per post-P78 SDK convention; must NOT manually call `slicer_sdk::test_support::install_log_capture()` (the macro's `mock_host_setup` hook handles that).

## Verification Commands

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `test -d modules/core-modules/overhang-classifier-default && test -f modules/core-modules/overhang-classifier-default/Cargo.toml && test -f modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml && test -f modules/core-modules/overhang-classifier-default/src/lib.rs && test -d modules/core-modules/overhang-classifier-default/wit-guest && grep -qE '^slicer-sdk *=' modules/core-modules/overhang-classifier-default/Cargo.toml && ! grep -qE '^slicer-(core|runtime|wasm-host|scheduler|gcode|model-io) *=' modules/core-modules/overhang-classifier-default/Cargo.toml` | FACT pass/fail |
| AC-2 | `grep -qE 'impl.*FinalizationModule' modules/core-modules/overhang-classifier-default/src/lib.rs && grep -qE 'overhang_(1|2|3|4)_4_speed' modules/core-modules/overhang-classifier-default/src/lib.rs && ! rg -q 'slicer_core::' modules/core-modules/overhang-classifier-default/src/ && grep -rqE 'fn classify_layers\|LinesDistancer2D' modules/core-modules/overhang-classifier-default/src/` | FACT pass/fail |
| AC-3 | `! rg -q 'classify_layers' crates/slicer-gcode/src/ && ! rg -q 'overhang_quartile' crates/slicer-gcode/src/` | FACT pass/fail |
| AC-3.5 | `test ! -f crates/slicer-core/src/algos/overhang_classifier.rs && ! grep -qE 'overhang_classifier' crates/slicer-core/src/algos/mod.rs && ! grep -qE 'slicer_core::algos::overhang_classifier::classify_layers' crates/slicer-runtime/src/lib.rs && test ! -f crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` | FACT pass/fail |
| AC-4 | `cargo xtask build-guests && cargo xtask build-guests --check` | FACT pass/fail |
| AC-5 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode --instrument-stderr 2> /tmp/p88-stderr.log && grep -qE 'overhang-classifier-default|overhang_classifier_default' /tmp/p88-stderr.log` | FACT pass/fail |
| AC-6 | (Manual ceremony — temp module dir without overhang module via `--no-default-module-paths`; slice succeeds; SHA logged) | (not CI) |
| AC-7 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode && sha256sum /tmp/benchy-p88.gcode` (compare to P87 baseline `89a329ad…`; document byte-identical OR LSB-shift) | SNIPPET (SHA + verdict) |
| AC-8 | `cargo test -p overhang-classifier-default` | FACT pass/fail + count |
| AC-9 | `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace` (flags mandatory per P85 closure) | FACT pass/fail + duration + count |
| AC-N1 | `! rg -q 'classify_layers' crates/slicer-gcode/src/` | FACT pass/fail |
| AC-N2 | `! rg -q 'OVERHANG_CLASSIFICATION_PRODUCER|OverhangClassificationProducer' crates/slicer-runtime/src/ && [ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ]` | FACT pass/fail |
| AC-N3 | `git diff --name-only HEAD -- crates/slicer-schema/wit/ \| wc -l \| grep -qE '^0$'` (run BEFORE the closure commit, so HEAD is still pre-P88) | FACT pass/fail |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-3 | `cargo xtask build-guests --check` | FACT pass/fail |

## Step Completion Expectations

- Guest rebuild (`cargo xtask build-guests`) MUST run after the module source is created and BEFORE `cargo test --workspace` (the workspace gate would otherwise read a stale or missing `.wasm` artifact).
- The new module's `Cargo.toml` MUST inherit `wit-bindgen` from `[workspace.dependencies]` if that's how the other 20 modules do it; copy the pattern exactly.
- AC-7 outcome MUST be documented in the implementation log either as "byte-identical to P87 baseline" or as "new baseline SHA <X>; F-word-only diff scope verified".

## Packet-Specific Context Discipline

- `world-finalization.wit` is ~130 LOC and OK to load in full (it's the contract this packet implements).
- `modules/core-modules/finalization-default/src/lib.rs` is the template; read lines 1–80 only (imports + `impl FinalizationModule` skeleton). NEVER load the full file.
- `crates/slicer-core/src/algos/overhang_classifier.rs` (post-P84) is the kernel the module calls; read only its `pub fn classify_layers` signature and the docstring (lines 1–60).
- `crates/slicer-gcode/src/emit.rs` — grep for `classify_layers` to find the single call site to delete; ±10 lines around the match.
- `OrcaSlicerDocumented/` is irrelevant — the OrcaSlicer parity for overhang lives in the `slicer-core` kernel (preserved verbatim in P84).
