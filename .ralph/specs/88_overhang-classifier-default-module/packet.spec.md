---
status: implemented
packet: 88
task_ids: [TASK-238]
requires: [86, 87]
backlog_source: docs/07_implementation_status.md
---

# Packet 88 — Overhang Classification as a `FinalizationModule` Core-Module

## Goal

Ship `modules/core-modules/overhang-classifier-default/` — a guest WASM module implementing the existing `FinalizationModule` trait — that **owns the complete overhang-classification logic** (the 319-LOC `classify_layers` algorithm relocated from `slicer-core/src/algos/overhang_classifier.rs` plus the `LinesDistancer2D` primitive it consumes and any other helpers it pulls from `slicer-core`'s internals) and emits per-wall-entity `modify-entity(entity_id, set-speed-factor(factor))` mutations through the `finalization-output-builder` already defined in `world-finalization@1.0.0`; delete `slicer-gcode`'s direct `classify_layers` call site, delete `slicer-runtime/src/lib.rs:192`'s P84-era `pub use slicer_core::algos::overhang_classifier::classify_layers;` re-export, delete `crates/slicer-core/src/algos/overhang_classifier.rs` and the P84 golden test at `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` (or migrate the golden into the guest's tests); the guest is self-contained — no `slicer-core` dep — preventing the `host-algos` feature gate from contaminating the guest dep tree. Default `pnp_cli slice --module-dir modules/core-modules` invocations preserve current behavior modulo a possible LSB-precision shift in feedrate decimals per AC-7; users who curate a custom module dir without this module get NO overhang annotation (the explicit Q6-resolution from the deepening-plan grilling).

## Scope Boundaries

This packet completes the deepening batch by turning overhang classification into a real user-swappable seam — without inventing a new stage, without a WIT change, and without rebuilding guests for unrelated contract churn. The `world-finalization::run-finalization` export already provides `list<layer-collection-view>` input and `modify-entity(entity_id, set-speed-factor(f32))` output (verified at `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit:121`). The new module sits alongside the 20 existing core-modules; its presence is observed by `pnp_cli`'s module-search-path discovery (the same way `part-cooling`, `skirt-brim`, etc. are). The `slicer-gcode` crate keeps its g-code emit path; only its *direct call* to `classify_layers` is deleted — the emitter reads `set-speed-factor` annotations off entities the same way it already reads them for the existing `finalization-default` module. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 86 closed**: `gcode_emit` lives in `slicer-gcode`, imports `slicer_core::classify_layers` at the call site this packet deletes.
- **Requires packet 87 closed**: the `region_mapping` move stabilises the final `slicer-core` algo layout (the new module imports from `slicer_core::*`).
- **Workspace-test checkpoint packet** — the final gate for the deepening batch. `cargo test --workspace` MUST run green at close per the deviation policy recorded in P81.
- Closure requires `cargo xtask build-guests --check` clean. **This packet adds a new directory under `modules/core-modules/*` — in CLAUDE.md's guest-staleness path list.** Implementer MUST run `cargo xtask build-guests` (no `--check`) to compile the new guest, then `--check` to confirm clean.

## Acceptance Criteria

### AC-1 — `modules/core-modules/overhang-classifier-default/` exists with manifest declaring `PostPass::LayerFinalization`; module Cargo.toml does NOT depend on `slicer-core`

**Given** the new module,
**When** the workspace is inspected,
**Then** `modules/core-modules/overhang-classifier-default/` exists with `Cargo.toml`, `overhang-classifier-default.toml` (manifest — match the existing modules' `<name>.toml` convention verified at `seam-planner-default/` and 19 others), `src/lib.rs`, and `wit-guest/` directory. The Cargo.toml declares `slicer-sdk`, `slicer-schema`, `slicer-ir` as path deps plus the wasm32-only `wit-bindgen` workspace dep (mirroring the verified pattern at `modules/core-modules/seam-planner-default/Cargo.toml`). It does **NOT** depend on `slicer-core`, `slicer-runtime`, `slicer-wasm-host`, `slicer-scheduler`, `slicer-gcode`, `slicer-model-io`, or `wasmtime` — the guest must be self-contained because `slicer-core`'s `host-algos` feature gate would otherwise contaminate the guest dep tree (P84 lesson).

| `test -d modules/core-modules/overhang-classifier-default && test -f modules/core-modules/overhang-classifier-default/Cargo.toml && test -f modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml && test -f modules/core-modules/overhang-classifier-default/src/lib.rs && test -d modules/core-modules/overhang-classifier-default/wit-guest && grep -qE '^slicer-sdk *=' modules/core-modules/overhang-classifier-default/Cargo.toml && ! grep -qE '^slicer-(core|runtime|wasm-host|scheduler|gcode|model-io) *=' modules/core-modules/overhang-classifier-default/Cargo.toml`

### AC-2 — Module owns the complete overhang-classification logic in `src/`; no import from `slicer_core::algos::overhang_classifier`; reads `FeedrateConfig` from `config-view`

**Given** the self-contained-guest invariant,
**When** `modules/core-modules/overhang-classifier-default/src/` is read,
**Then** it contains a `#[slicer_module]` attribute on a struct that implements `FinalizationModule`. The complete `classify_layers` algorithm (~319 LOC relocated from `crates/slicer-core/src/algos/overhang_classifier.rs`) lives inside the guest's `src/` tree, along with the `LinesDistancer2D` primitive (currently `crates/slicer-core/src/aabb_lines_2d.rs`) and any other helpers the kernel transitively depends on. No source file under the guest imports from `slicer_core::*` (zero references). The `run_finalization` body reads the four overhang-speed fields from `config-view` (`overhang_1_4_speed`, `overhang_2_4_speed`, `overhang_3_4_speed`, `overhang_4_4_speed` — exact key names per `slicer-ir::FeedrateConfig`'s field names), short-circuits when all four are 0.0 (preserving the pre-P84 byte-identical baseline for unconfigured printers), iterates the per-layer entity stream, runs the internal classification kernel, then calls `output.modify_entity(layer_index, entity_id, EntityMutation::SetSpeedFactor(factor))` for each wall entity in a non-Q4 quartile.

| `grep -qE '#\[slicer_module\]|slicer_sdk::slicer_module' modules/core-modules/overhang-classifier-default/src/lib.rs && grep -qE 'impl.*FinalizationModule' modules/core-modules/overhang-classifier-default/src/lib.rs && grep -qE 'overhang_(1|2|3|4)_4_speed' modules/core-modules/overhang-classifier-default/src/lib.rs && ! rg -q 'slicer_core::' modules/core-modules/overhang-classifier-default/src/ && grep -rqE 'fn classify_layers|LinesDistancer2D' modules/core-modules/overhang-classifier-default/src/ && grep -qE 'SetSpeedFactor|modify_entity' modules/core-modules/overhang-classifier-default/src/lib.rs`

### AC-3 — All references to `classify_layers` and the obsolete overhang-quartile feedrate-lookup branch are removed from `slicer-gcode/src/`

**Given** the seam,
**When** `crates/slicer-gcode/src/` is grepped,
**Then** (a) no source file contains the call `classify_layers(`, (b) no source file contains the import `use slicer_core::.*classify_layers`, (c) `resolve_feedrate` (at `emit.rs:106-123` pre-P88) no longer branches on `overhang_quartile: Option<u8>` because the guest emits `set-speed-factor` mutations directly — the obsolete table-lookup branch (`overhang_1_4_speed` through `overhang_4_4_speed` indexed by quartile) is removed. The multiplicative path that applies `speed * factor` from existing `set-speed-factor` mutations is preserved (it's what the existing finalization-implementing modules already use — Step 1 dispatch #4 confirms). The change deletes dead code, not preserves it.

| `! rg -q 'classify_layers' crates/slicer-gcode/src/ && ! rg -q 'overhang_quartile' crates/slicer-gcode/src/`

### AC-3.5 — `slicer-core/src/algos/overhang_classifier.rs` and its mod declaration are GONE; `slicer-runtime/src/lib.rs:192`'s P84 re-export shim is GONE

**Given** the move into the guest,
**When** the workspace is grepped,
**Then** `crates/slicer-core/src/algos/overhang_classifier.rs` does NOT exist; `crates/slicer-core/src/algos/mod.rs` does NOT declare `pub mod overhang_classifier;` and does NOT re-export `classify_layers`; `crates/slicer-runtime/src/lib.rs` does NOT contain `pub use slicer_core::algos::overhang_classifier::classify_layers` (the L192 P84-era compat shim). The P84 golden test `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` is either DELETED or MIGRATED into `modules/core-modules/overhang-classifier-default/tests/` (preferred — the test's invariants are still valuable, just at the guest layer).

| `test ! -f crates/slicer-core/src/algos/overhang_classifier.rs && ! grep -qE 'overhang_classifier' crates/slicer-core/src/algos/mod.rs && ! grep -qE 'slicer_core::algos::overhang_classifier::classify_layers' crates/slicer-runtime/src/lib.rs && test ! -f crates/slicer-core/tests/algo_overhang_classifier_tdd.rs`

### AC-4 — `cargo xtask build-guests --check` is clean after rebuild; new guest `.wasm` artifact exists

**Given** the new module,
**When** `cargo xtask build-guests` runs (without `--check`),
**Then** it succeeds and produces `modules/core-modules/overhang-classifier-default/overhang_classifier_default_guest.wasm` (or equivalent — match the existing per-module guest-output filename convention). Subsequently, `cargo xtask build-guests --check` reports zero STALE entries — the new guest is registered correctly in xtask's discovery / build-list mechanism.

| `cargo xtask build-guests && cargo xtask build-guests --check`

### AC-5 — Default invocation (`pnp_cli slice --module-dir modules/core-modules ...`) loads `overhang-classifier-default` and applies overhang annotations

**Given** the standard module-dir convention,
**When** `pnp_cli slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode --report /tmp/p88-report.html` runs (and the report feature is enabled — default per P82),
**Then** the slicer report HTML (or the structured progress events emitted on stderr with `--instrument-stderr`) shows the `overhang-classifier-default` module loaded and producing at least one `modify-entity` mutation on a fixture with known overhang geometry (benchy has overhangs at the bow and stern). The implementation log records the mutation count.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode --instrument-stderr 2> /tmp/p88-stderr.log && grep -qE 'overhang-classifier-default|overhang_classifier_default' /tmp/p88-stderr.log`

### AC-6 — Custom invocation without the module (`--module-dir <empty>`) succeeds without crashing; no overhang annotation present

**Given** the opt-out shape (Q6 from grilling: ship the module; no host fallback),
**When** a slice runs against a module dir that does NOT include `overhang-classifier-default` (e.g., the other 20 modules curated into a temp directory),
**Then** the slice completes successfully (exit 0). The resulting g-code does NOT contain feedrate variations attributable to overhang annotation (i.e., wall paths over overhangs run at the base wall feedrate). The implementation log documents the test command and the SHA of the resulting g-code (this SHA will differ from the default-invocation SHA, by design — that's the user opt-out).

| `mkdir -p /tmp/p88-noverhang && for m in modules/core-modules/*/; do test "$(basename $m)" = overhang-classifier-default || cp -r "$m" /tmp/p88-noverhang/; done && cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir /tmp/p88-noverhang --output /tmp/benchy-p88-noverhang.gcode --no-default-module-paths && sha256sum /tmp/benchy-p88-noverhang.gcode`

### AC-7 — End-to-end g-code on default invocation produces output consistent with the P87 baseline (byte-identical OR documented LSB-precision shift)

**Given** the migration from `overhang_quartile` annotations + `resolve_feedrate` lookup to `set-speed-factor(factor)` flowing through the existing finalization mutation path,
**When** the default `pnp_cli slice` is compared to the P87 closure SHA,
**Then** **EITHER**:
- (a) The SHA matches byte-for-byte — the engineering choice for `factor = overhang_speed / base_speed_for_role` preserves f32 rounding such that `base_speed * factor` re-derives `overhang_speed` exactly for all wall paths in benchy.
- **OR** (b) The SHA differs in feedrate decimal digits ONLY (verified by `diff -u /tmp/p87-baseline.gcode /tmp/benchy-p88.gcode | grep '^[+-]F'` showing only F-word differences and only in the 3rd–6th decimal). The implementation log captures both SHAs, documents the LSB-precision rationale, and updates the post-batch baseline.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode && sha256sum /tmp/benchy-p88.gcode`

### AC-8 — Module-level test passes (`#[module_test]` harness)

**Given** the SDK test harness convention (post-P78 fold),
**When** `cargo test -p overhang-classifier-default` runs,
**Then** at least one `#[module_test]` test passes. The test constructs a tiny two-layer `LayerCollectionIR` fixture (one layer fully supported, one layer with a wall overhanging the previous layer's wall), runs the module's `run_finalization` body, and asserts that the second layer's wall entity received a `SetSpeedFactor` mutation with a factor < 1.0 (i.e., the overhang slowed it down).

| `cargo test -p overhang-classifier-default`

### AC-9 — Workspace test gate passes with full feature flag set (final checkpoint)

**Given** the deepening-batch policy (P83, P85, P88 are the three checkpoint packets) AND the P85 lesson that bare `cargo test --workspace` silently masks regressions via fail-fast + feature-gated test target skip,
**When** `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace` runs (dispatched to a sub-agent that returns FACT pass/fail per CLAUDE.md §Test Discipline),
**Then** the full suite passes with zero regressions vs the Step 0 baseline (which captures the post-P87 count, ≈ 2067 from P85's corrected baseline plus P86's slicer-gcode golden test +1 plus P87's region_mapping tests +4 ≈ 2072). The count delta is non-negative; the only expected subtraction is the P84 golden `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` (-4 to -8 tests) if it's deleted rather than migrated into the guest. Net delta documented in the implementation log.

| `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace`

## Negative Test Cases

### AC-N1 — `slicer-gcode/src/` does NOT import `classify_layers` after this packet

**Given** the deletion,
**When** `crates/slicer-gcode/src/` is grepped,
**Then** no source file imports `classify_layers` from `slicer_core` or anywhere else. (Positive form: AC-3. Negative form: structural assertion that the seam was cleanly cut.)

| `! rg -q 'classify_layers' crates/slicer-gcode/src/`

### AC-N2 — `slicer-runtime/src/` and `crates/slicer-runtime/Cargo.toml` do NOT regain a host builtin for overhang annotation

**Given** the Q6 resolution (ship the module; no host fallback),
**When** the runtime is inspected,
**Then** no `OVERHANG_CLASSIFICATION_PRODUCER` static appears anywhere under `crates/slicer-runtime/src/` (would indicate a regression to a host-fallback shape). The `runtime_builtins()` count stays at 8 (unchanged from P84/P87).

| `! rg -q 'OVERHANG_CLASSIFICATION_PRODUCER|OverhangClassificationProducer' crates/slicer-runtime/src/ && [ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ]`

### AC-N3 — No WIT file under `crates/slicer-schema/wit/` was edited

**Given** the no-WIT-change invariant,
**When** `git diff --name-only HEAD~N -- crates/slicer-schema/wit/` runs (where `HEAD~N` is the pre-P88 commit),
**Then** the output is empty. (Structural confirmation that no contract change shipped — guests rebuild because of `modules/core-modules/overhang-classifier-default/` being NEW, not because of an existing guest's WIT contract changing.)

| `git diff --name-only HEAD -- crates/slicer-schema/wit/ | wc -l | grep -qE '^0$'`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests` (rebuild) then `cargo xtask build-guests --check` (clean — new guest registered)
4. `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace` (final checkpoint gate — dispatched to sub-agent; flags mandatory per P85 closure)
5. `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode` (succeeds; SHA documented per AC-7)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` — the existing WIT contract this packet exercises. NO edit. Read in full (≤ 130 LOC) to confirm `run-finalization`, `modify-entity`, `entity-mutation::set-speed-factor`, `list<layer-collection-view>` shapes.
- `docs/05_module_sdk.md` — `FinalizationModule` trait, `#[slicer_module]` macro, manifest TOML schema.
- `docs/04_host_scheduler.md` — `PostPass::LayerFinalization` stage placement. Confirms multiple FinalizationModules can run in the stage (the existing `finalization-default` already does — `overhang-classifier-default` joins it).
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — preserved (no built-in is being added or removed).
- `modules/core-modules/finalization-default/` (or any existing finalization-implementing module — check `find modules/core-modules -name 'manifest.toml' -exec grep -l finalization {} \;`) — the template / pattern this packet follows. NO content change.

## Doc Impact Statement

One ADR planned at packet close:

- **ADR-0008 — Overhang annotation is a `FinalizationModule`, not a new stage.** Records: (a) why no new stage / WIT export was added (the existing `world-finalization::run-finalization` already provides the seam); (b) why no host fallback exists (the module ships in `modules/core-modules/`, so default invocations include it; users opt out by curating their module dir); (c) the byte-identical-or-LSB-shift trade-off documented in AC-7. Future architecture reviewers will likely ask why we didn't add a `PostPass::OverhangAnnotation` stage — this ADR explains.

`docs/15_config_keys_reference.md` may grow a note that the four `overhang_*_4_speed` keys are now consumed by `overhang-classifier-default` rather than by the host's emit path. Deferred to the deepening-batch doc-sweep packet.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by this edit.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- [AC-2 / design.md] — Specified: classify_layers operates on `&mut [LayerCollectionIR]` (in-place) | Implemented: operates on `&[LayerCollectionView]` (read-only), returns `HashMap`, mutations via `output.modify_entity()` | Reason: Guest receives `LayerCollectionView` from WIT boundary; SDK `FinalizationOutputBuilder` is the mutation channel.
- [AC-2 / requirements.md] — Specified: module reads only 4 overhang config keys | Implemented: reads 7 keys (4 overhang + 3 base wall speeds) | Reason: `factor = overhang_speed / base_speed` requires per-role base speed lookup. Note: this introduces config-key duplication — both the guest module and `slicer-gcode::resolve_feedrate` now read `outer_wall_speed`/`inner_wall_speed`/`thin_wall_speed` for different purposes (factor computation vs. base-line feedrate). Functionally correct; flagged for future config-consumption audit.
- [design.md manifest] — Specified: config schema declares 4 keys | Implemented: declares 7 keys + `overridable-per-region` + `overridable-per-layer` sections | Reason: Manifest loader requires the two overridable sections; module needs base speeds.
- [AC-5] — Specified: module name appears in `--instrument-stderr` | Implemented: required adding `execute_layer_finalization_with_instrumentation` to `slicer-runtime` | Reason: Original `execute_layer_finalization` lacked per-module instrumentation events.
- [AC-6] — Specified: SHA differs from default invocation | Implemented: SHA is byte-identical with default config | Reason: Default config has all overhang speeds at 0.0 → short-circuit → no mutations emitted with or without module. Verified behavioral difference with non-zero config: default SHA `7D3AF220…` vs no-overhang SHA `A9C9DEC0…`.
- [AC-9] — Specified: test count delta within ±10 of 2072 | Implemented: 2051 (delta -21) | Reason: `overhang_speed_tdd.rs` deletion (6 tests) + P84 golden deletion (~15 tests) + 1 new test.
