---
status: draft
packet: 88
task_ids: [TASK-238]
requires: [86, 87]
backlog_source: docs/07_implementation_status.md
---

# Packet 88 — Overhang Classification as a `FinalizationModule` Core-Module

## Goal

Ship `modules/core-modules/overhang-classifier-default/` — a guest WASM module implementing the existing `FinalizationModule` trait — that walks the layer collection, calls `slicer_core::classify_layers` (the kernel moved in P84), and emits per-wall-entity `modify-entity(entity_id, set-speed-factor(factor))` mutations through the `finalization-output-builder` already defined in `world-finalization@1.0.0`; delete `slicer_gcode`'s direct `slicer_core::classify_layers` call (added in P84 / P86) so overhang annotation becomes the module's exclusive responsibility; with the module shipped under the workspace's standard core-modules path, default `pnp_cli slice --module-dir modules/core-modules` invocations preserve current behavior, while users who curate a custom module dir without this module get NO overhang annotation (the explicit Q6-resolution from the deepening-plan grilling).

## Scope Boundaries

This packet completes the deepening batch by turning overhang classification into a real user-swappable seam — without inventing a new stage, without a WIT change, and without rebuilding guests for unrelated contract churn. The `world-finalization::run-finalization` export already provides `list<layer-collection-view>` input and `modify-entity(entity_id, set-speed-factor(f32))` output (verified at `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit:121`). The new module sits alongside the 20 existing core-modules; its presence is observed by `pnp_cli`'s module-search-path discovery (the same way `part-cooling`, `skirt-brim`, etc. are). The `slicer-gcode` crate keeps its g-code emit path; only its *direct call* to `classify_layers` is deleted — the emitter reads `set-speed-factor` annotations off entities the same way it already reads them for the existing `finalization-default` module. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 86 closed**: `gcode_emit` lives in `slicer-gcode`, imports `slicer_core::classify_layers` at the call site this packet deletes.
- **Requires packet 87 closed**: the `region_mapping` move stabilises the final `slicer-core` algo layout (the new module imports from `slicer_core::*`).
- **Workspace-test checkpoint packet** — the final gate for the deepening batch. `cargo test --workspace` MUST run green at close per the deviation policy recorded in P81.
- Closure requires `cargo xtask build-guests --check` clean. **This packet adds a new directory under `modules/core-modules/*` — in CLAUDE.md's guest-staleness path list.** Implementer MUST run `cargo xtask build-guests` (no `--check`) to compile the new guest, then `--check` to confirm clean.

## Acceptance Criteria

### AC-1 — `modules/core-modules/overhang-classifier-default/` exists with a manifest declaring `PostPass::LayerFinalization`

**Given** the new module,
**When** the workspace is inspected,
**Then** `test -d modules/core-modules/overhang-classifier-default && test -f modules/core-modules/overhang-classifier-default/Cargo.toml && test -f modules/core-modules/overhang-classifier-default/module.toml` (or whatever the manifest filename is — match the existing 20 modules' convention). The manifest declares a stage entry mapping the trait method `run_finalization` to the `slicer:world-finalization@1.0.0` world. The Cargo.toml declares `slicer-sdk` (with the `module` feature or whatever the SDK exposes for guest builds), `slicer-ir`, `slicer-core` (for `classify_layers`).

| `test -d modules/core-modules/overhang-classifier-default && test -f modules/core-modules/overhang-classifier-default/Cargo.toml && (test -f modules/core-modules/overhang-classifier-default/module.toml || test -f modules/core-modules/overhang-classifier-default/manifest.toml) && grep -qE '^slicer-sdk *=' modules/core-modules/overhang-classifier-default/Cargo.toml`

### AC-2 — Module source uses `#[slicer_module]` and implements `FinalizationModule::run_finalization` reading `FeedrateConfig` from `config-view`

**Given** the SDK-shaped guest pattern (`crates/slicer-sdk/src/`),
**When** `modules/core-modules/overhang-classifier-default/src/lib.rs` is read,
**Then** it contains a `#[slicer_module]` attribute on a struct that implements `FinalizationModule`. The `run_finalization` body reads each of the four overhang-speed fields from `config-view` (`overhang_1_4_speed`, `overhang_2_4_speed`, `overhang_3_4_speed`, `overhang_4_4_speed` — exact key names per `slicer-ir::FeedrateConfig`'s field names), short-circuits when all four are 0.0 (preserving the AC-2 baseline from `crates/slicer-runtime/src/overhang_classifier.rs` pre-P84), iterates `layers.ordered_entities()`, calls `slicer_core::classify_layers` (or the per-layer variant) to compute quartiles, then calls `output.modify_entity(layer_index, entity_id, EntityMutation::SetSpeedFactor(factor))` for each wall entity in a non-Q4 quartile.

| `grep -qE '#\[slicer_module\]\|slicer_sdk::slicer_module' modules/core-modules/overhang-classifier-default/src/lib.rs && grep -qE 'impl.*FinalizationModule' modules/core-modules/overhang-classifier-default/src/lib.rs && grep -qE 'overhang_(1\|2\|3\|4)_4_speed' modules/core-modules/overhang-classifier-default/src/lib.rs && grep -qE 'slicer_core::classify_layers\|slicer_core::algos::overhang_classifier' modules/core-modules/overhang-classifier-default/src/lib.rs && grep -qE 'SetSpeedFactor\|modify_entity' modules/core-modules/overhang-classifier-default/src/lib.rs`

### AC-3 — `slicer-gcode`'s direct `classify_layers` call is deleted; emit path consumes `set-speed-factor` annotations only

**Given** the seam,
**When** `crates/slicer-gcode/src/` is grepped,
**Then** no source file contains `classify_layers(` (the direct kernel call from P84/P86 is removed). The emit path still reads speed factors off the entity stream — that path is unchanged from pre-P86 (the existing `finalization-default` module already exercises `set-speed-factor` for non-overhang reasons; overhang now flows through the same mechanism).

| `! rg -q 'classify_layers\s*\(' crates/slicer-gcode/src/`

### AC-4 — `cargo xtask build-guests --check` is clean after rebuild; new guest `.wasm` artifact exists

**Given** the new module,
**When** `cargo xtask build-guests` runs (without `--check`),
**Then** it succeeds and produces `modules/core-modules/overhang-classifier-default/overhang_classifier_default_guest.wasm` (or equivalent — match the existing per-module guest-output filename convention). Subsequently, `cargo xtask build-guests --check` reports zero STALE entries — the new guest is registered correctly in xtask's discovery / build-list mechanism.

| `cargo xtask build-guests && cargo xtask build-guests --check`

### AC-5 — Default invocation (`pnp_cli slice --module-dir modules/core-modules ...`) loads `overhang-classifier-default` and applies overhang annotations

**Given** the standard module-dir convention,
**When** `pnp_cli slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode --report /tmp/p88-report.html` runs (and the report feature is enabled — default per P82),
**Then** the slicer report HTML (or the structured progress events emitted on stderr with `--instrument-stderr`) shows the `overhang-classifier-default` module loaded and producing at least one `modify-entity` mutation on a fixture with known overhang geometry (benchy has overhangs at the bow and stern). The implementation log records the mutation count.

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode --instrument-stderr 2> /tmp/p88-stderr.log && grep -qE 'overhang-classifier-default\|overhang_classifier_default' /tmp/p88-stderr.log`

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

### AC-9 — Workspace test gate passes (final checkpoint)

**Given** the deepening-batch policy (P83, P85, P88 are the three checkpoint packets),
**When** `cargo test --workspace` runs (dispatched to a sub-agent that returns FACT pass/fail per CLAUDE.md §Test Discipline),
**Then** the full suite passes with zero regressions vs the P85 baseline count. The count delta vs P85 should reflect: (a) tests added in P86's `slicer-gcode` golden test, P87's `slicer-core` region-mapping test, P88's `overhang-classifier-default` module test; (b) tests migrated between crates per P84/P86/P87. Net new tests ≥ 3 (one per AC-8-style golden in P86/P87/P88).

| `cargo test --workspace`

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

| `! rg -q 'OVERHANG_CLASSIFICATION_PRODUCER\|OverhangClassificationProducer' crates/slicer-runtime/src/ && [ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ]`

### AC-N3 — No WIT file under `crates/slicer-schema/wit/` was edited

**Given** the no-WIT-change invariant,
**When** `git diff --name-only HEAD~N -- crates/slicer-schema/wit/` runs (where `HEAD~N` is the pre-P88 commit),
**Then** the output is empty. (Structural confirmation that no contract change shipped — guests rebuild because of `modules/core-modules/overhang-classifier-default/` being NEW, not because of an existing guest's WIT contract changing.)

| `git diff --name-only HEAD -- crates/slicer-schema/wit/ | wc -l | grep -qE '^0$'`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests` (rebuild) then `cargo xtask build-guests --check` (clean — new guest registered)
4. `cargo test --workspace` (final checkpoint gate — dispatched to sub-agent)
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

- **ADR-0007 — Overhang annotation is a `FinalizationModule`, not a new stage.** Records: (a) why no new stage / WIT export was added (the existing `world-finalization::run-finalization` already provides the seam); (b) why no host fallback exists (the module ships in `modules/core-modules/`, so default invocations include it; users opt out by curating their module dir); (c) the byte-identical-or-LSB-shift trade-off documented in AC-7. Future architecture reviewers will likely ask why we didn't add a `PostPass::OverhangAnnotation` stage — this ADR explains.

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
