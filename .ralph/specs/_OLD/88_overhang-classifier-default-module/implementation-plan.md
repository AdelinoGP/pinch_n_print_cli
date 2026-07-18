# Packet 88 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- Large existing files (`finalization-default/src/lib.rs` if > 200 LOC; `overhang_classifier.rs` from P84) are NEVER loaded in full. Line-range reads only.
- P86 and P87 MUST be closed (Step 0 verifies).
- This is the FINAL checkpoint packet of the deepening batch; `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace` runs at close.
- The new module under `modules/core-modules/` is in CLAUDE.md's guest-staleness path list — Step 5 rebuilds guests; Step 6 confirms `--check` clean.

---

## Step 0 — Verify P86/P87 closure + capture P87 g-code SHA baseline

**Objective.** Confirm `slicer-gcode/src/emit.rs` calls `slicer_core::classify_layers` (will be deleted) and `slicer-core/src/algos/region_mapping.rs` exists. Capture g-code SHA.

**Precondition.** P86 and P87 are both `implemented`. Working tree clean. Carried-forward g-code SHA is `89a329ad3a4c1b7febca839edfca8b6302e562d8d2a390ee144252fd54e65a2b` per the P81→P87 byte-identical streak.

**Postcondition.** Two log entries: prereq-state verification + baseline SHA from P87 closure.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `grep -rqE 'slicer_core::classify_layers' crates/slicer-gcode/src/ && test -f crates/slicer-core/src/algos/region_mapping.rs`. Return FACT pass/fail.
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/p88-baseline.gcode && sha256sum /tmp/p88-baseline.gcode`. Return FACT `<hex>`.
- Dispatch: pre-packet workspace test count. `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace 2>&1 | tail -5`. Return SNIPPET.

**Context cost: S.**

**Narrow verification.** Both positive.

**Falsifying check / exit condition.** Either prereq fails → abort.

---

## Step 1 — Survey the existing FinalizationModule core-module template + xtask module-discovery + SDK trait shape

**Objective.** Build the exact knowledge needed to scaffold the new module correctly.

**Precondition.** Step 0 green.

**Postcondition.** Four log entries per design.md dispatches #1, #2, #3, #4.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.** Dispatches #1, #2, #3, #4 from design.md.

**Context cost: S.**

**Narrow verification.** Four returns populated.

**Falsifying check / exit condition.** If dispatch #4 returns "no `speed_factor` consumption in `slicer-gcode/src/`," the multiplicative path doesn't exist yet — the packet needs to ADD the consumption logic to `resolve_feedrate`, which is a much larger scope. STOP and re-grill the user.

---

## Step 2 — Scaffold the new module directory + manifest + empty lib.rs

**Objective.** Module directory exists with valid structure that `cargo xtask build-guests` recognises.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `cargo xtask build-guests --check` reports the new module as STALE (i.e., it's discovered but not yet built). Workspace builds.

**Files allowed to read.** The template from Step 1 dispatch #1.
**Files allowed to edit.**
1. `modules/core-modules/overhang-classifier-default/Cargo.toml` — CREATE per dispatch #1's template.
2. `modules/core-modules/overhang-classifier-default/module.toml` (or `manifest.toml`) — CREATE per template.
3. `modules/core-modules/overhang-classifier-default/src/lib.rs` — CREATE with a stub `#[slicer_module]` impl (body returns `Ok(())` immediately — populated in Step 3).
4. `modules/core-modules/overhang-classifier-default/wit-guest/...` — CREATE per template (likely a small per-guest WIT include).
5. Workspace `Cargo.toml` — add `"modules/core-modules/overhang-classifier-default"` to `members` IF dispatch #2 reported members-driven discovery; OR `xtask/src/main.rs` (or wherever) gets the new module entry IF list-driven.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.
- Dispatch: `cargo xtask build-guests --check`. Return FACT (expected: STALE for the new module).

**Context cost: S.**

**Narrow verification.** Workspace builds; new module appears in `--check` STALE output.

**Falsifying check / exit condition.** New module not discovered → check dispatch #2's discovery mechanism; add to the right list.

---

## Step 3 — Implement the module body: read config, classify, emit set-speed-factor mutations

**Objective.** The module's `run_finalization` body is complete and correctly translates overhang classification to `set-speed-factor` mutations.

**Precondition.** Step 2 green.

**Postcondition.** `modules/core-modules/overhang-classifier-default/src/lib.rs` contains the full body per design.md §"Selected Approach" (also see `requirements.md` §In Scope item 3).

**Files allowed to read.**
- `crates/slicer-core/src/algos/overhang_classifier.rs` (post-P84) — confirm `classify_layers` signature; lines 1–60 only.
- `crates/slicer-runtime/src/gcode_emit.rs` (post-P86 location: `crates/slicer-gcode/src/emit.rs`) — find the `base_speed_for_role` lookup logic and the role→base-speed table; copy the relevant subset into the module.
- `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` — confirm the `EntityMutation::SetSpeedFactor(f32)` variant name (in Rust per the bindgen output).
- The template module's lib.rs (≤ 80 lines).

**Files allowed to edit.**
1. `modules/core-modules/overhang-classifier-default/src/lib.rs` — fill in the body.

**The body skeleton (illustrative):**
```rust
#[slicer_module]
impl FinalizationModule for OverhangClassifierDefault {
    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let overhang_speeds: [f32; 4] = [
            config.get_float("overhang_1_4_speed")?.unwrap_or(0.0) as f32,
            config.get_float("overhang_2_4_speed")?.unwrap_or(0.0) as f32,
            config.get_float("overhang_3_4_speed")?.unwrap_or(0.0) as f32,
            config.get_float("overhang_4_4_speed")?.unwrap_or(0.0) as f32,
        ];
        if overhang_speeds.iter().all(|&s| s == 0.0) {
            return Ok(()); // AC-2 short-circuit
        }
        // ... per-layer classification via slicer_core::classify_layers (or per-layer variant)
        // ... per-entity factor computation and output.modify_entity(...)
        Ok(())
    }
}
```

**Expected sub-agent dispatch.**
- Dispatch: `cargo xtask build-guests`. Return FACT pass/fail + duration.

**Context cost: M.**

**Narrow verification.** Guest builds. `cargo xtask build-guests --check` reports clean for the new module.

**Falsifying check / exit condition.** Guest build fails on missing SDK method → check dispatch #3's trait sig; align method names.

---

## Step 4 — Add the AC-8 module-level test

**Objective.** A `#[module_test]` test exercises the module body against a two-layer fixture.

**Precondition.** Step 3 complete.

**Postcondition.** `cargo test -p overhang-classifier-default` passes.

**Files allowed to read.** Existing `#[module_test]`-using test files (any module under `modules/core-modules/*/tests/`) for the test-shape template.
**Files allowed to edit.**
1. `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs` — CREATE.

The test:
- Uses `#[module_test]` per post-P78 SDK convention.
- Constructs `LayerCollectionView` fixtures for two layers: layer 0 with one supported wall, layer 1 with one wall overhanging the layer-0 wall (signed distance places it in Q1 or Q2).
- Sets the four `overhang_*_4_speed` config values to non-zero values.
- Runs `run_finalization`.
- Asserts the second layer's wall entity received a `modify_entity` call with `SetSpeedFactor(f)` where `f < 1.0`.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test -p overhang-classifier-default`. Return FACT pass/fail + count.

**Context cost: S.**

**Narrow verification.** Test passes.

---

## Step 5 — Delete `classify_layers` from `slicer-gcode/src/emit.rs`; remove obsolete `overhang_quartile` resolve_feedrate branch

**Objective.** The seam is cut: g-code emission no longer drives overhang annotation; the module is the sole annotator. The dead overhang-quartile feedrate-lookup branch in `resolve_feedrate` is deleted (CLAUDE.md: "no dead code").

**Precondition.** Step 4 green.

**Postcondition.** `crates/slicer-gcode/src/emit.rs` no longer contains `classify_layers(...)`, `use slicer_core::classify_layers;`, or any branch in `resolve_feedrate` that reads `overhang_quartile`. The multiplicative `set-speed-factor` consumption path (already used by existing finalization-implementing modules — Step 1 dispatch #4 confirmed) handles all entity feedrate factoring. Workspace still builds.

**Files allowed to read.** `crates/slicer-gcode/src/emit.rs` — the `classify_layers` call site (currently L226), the `resolve_feedrate` function (currently L106-123), the `overhang_quartile` field reads (currently L114, L120-123, L446, L875).
**Files allowed to edit.**
1. `crates/slicer-gcode/src/emit.rs` — delete the import and call site (L21, L226); delete the `overhang_quartile`-indexed feedrate-lookup branch in `resolve_feedrate` (L106-123 — keep the multiplicative speed-factor application path); update the `feedrate_config: FeedrateConfig` field reads: the four `overhang_*_4_speed` fields become unused; either drop them from the struct OR document the remaining base-speed reads. If `feedrate_config` becomes entirely unused, drop the struct field and any constructor params that referenced it.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.

**Context cost: M.** (Bigger than the prior framing because resolve_feedrate cleanup is real work, not a one-line deletion.)

**Narrow verification.** Build green. `! rg -q 'classify_layers' crates/slicer-gcode/src/` AND `! rg -q 'overhang_quartile' crates/slicer-gcode/src/`.

**Falsifying check / exit condition.** Build fails on unused field → drop the field. Build fails on a downstream consumer expecting `overhang_quartile` annotations → that consumer needs to be migrated to the `set-speed-factor` path (likely a test fixture; surface and fix). If a fixture genuinely cannot be migrated (the test exercises the pre-P88 path), it gets DELETED — the pre-P88 path is gone.

---

## Step 5.5 — Delete the slicer-core kernel and its compat shims (the user's central catch)

**Objective.** Now that the guest owns the complete algorithm, slicer-core's `overhang_classifier` module and its P84-era compat shims are dead. Delete them.

**Precondition.** Step 5 green (slicer-gcode no longer imports from slicer-core's overhang module).

**Postcondition.**
- `crates/slicer-core/src/algos/overhang_classifier.rs` deleted.
- `crates/slicer-core/src/aabb_lines_2d.rs` either (a) deleted if no other slicer-core algo consumes it, OR (b) kept with a comment noting which algo consumes it. Step 5.5 dispatch verifies.
- `crates/slicer-core/src/algos/mod.rs` no longer declares `pub mod overhang_classifier;` and no longer re-exports `classify_layers`.
- `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` deleted (the P84 golden — its invariants live on in the guest's tests per AC-8).
- `crates/slicer-runtime/src/lib.rs:192`'s `pub use slicer_core::algos::overhang_classifier::classify_layers;` deleted.

**Files allowed to read.** `crates/slicer-core/src/algos/mod.rs` (grep for overhang_classifier references); `crates/slicer-core/src/lib.rs` (grep for aabb_lines_2d consumers); `crates/slicer-runtime/src/lib.rs:185-200`.
**Files allowed to edit.**
1. Delete `crates/slicer-core/src/algos/overhang_classifier.rs`.
2. Delete `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs`.
3. Edit `crates/slicer-core/src/algos/mod.rs` — drop the `pub mod overhang_classifier;` line and any `pub use` re-export naming `classify_layers`.
4. Edit `crates/slicer-runtime/src/lib.rs` — drop the L192 `pub use slicer_core::algos::overhang_classifier::classify_layers;` line.
5. Conditionally delete `crates/slicer-core/src/aabb_lines_2d.rs` IF dispatch confirms no remaining consumer.

**Expected sub-agent dispatches.**
- Dispatch: `rg -l 'aabb_lines_2d\|LinesDistancer2D' crates/slicer-core/src/`. Return LOCATIONS — determines whether aabb_lines_2d.rs has other consumers in slicer-core after the deletion.
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Build + clippy green; the AC-3.5 grep passes.

**Falsifying check / exit condition.** Build fails on a consumer of `slicer_core::algos::overhang_classifier::*` we missed → grep workspace-wide for the symbol; rewire or surface.

---

## Step 6 — Confirm guest `--check` clean and AC-5 (module loads on default invocation)

**Objective.** Default `pnp_cli slice` invocation loads the new module and processes overhang.

**Precondition.** Step 5 green.

**Postcondition.** `cargo xtask build-guests --check` clean. Default slice invocation logs the new module name to stderr / progress events.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `cargo xtask build-guests --check`. Return FACT pass/fail.
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p88.gcode --instrument-stderr 2> /tmp/p88-stderr.log && grep -qE 'overhang-classifier-default\|overhang_classifier_default' /tmp/p88-stderr.log`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Both green.

---

## Step 7 — AC-7 SHA verdict + AC-6 manual ceremony

**Objective.** Post-packet g-code SHA documented (byte-identical or LSB-shifted with rationale). AC-6 (custom invocation without the module) succeeds with no overhang annotation.

**Precondition.** Step 6 green.

**Postcondition.** Three log entries: post-packet default SHA, P87 baseline SHA, F-word-diff verdict; AC-6 alternate SHA.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `sha256sum /tmp/benchy-p88.gcode` (from Step 6). Return FACT `<hex>`.
- Dispatch: compare to Step 0 baseline; if mismatch, `diff -u /tmp/p88-baseline.gcode /tmp/benchy-p88.gcode | grep '^[+-]F' | head -20`. Return SNIPPET (≤ 20 lines).
- Dispatch (AC-6 ceremony): construct `/tmp/p88-noverhang` as the curated module dir, run slice, hash output. Return FACT `<alt-hex>`.

**Context cost: S.**

**Narrow verification.** Either byte-identical (preferred) OR diff is F-word-only AND within decimal-3-to-6 precision shift. AC-6 alt-SHA captured.

**Falsifying check / exit condition.** Diff shows non-F-word changes (e.g., path coordinates, extrusion amounts) → the module is introducing geometry changes it shouldn't. Bisect by reverting Step 3's body and re-running.

---

## Step 8 — Workspace test gate (final batch checkpoint)

**Objective.** `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace` green; the deepening batch's final ceremony.

**Precondition.** Step 7 green.

**Postcondition.** Full suite passes; count delta vs Step 0 baseline documented.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace 2>&1 | tail -5`. Return SNIPPET. Then FACT pass/fail + count + duration.

**Context cost: M.**

**Narrow verification.** Pass. Count delta within +10/-10 vs Step 0 baseline (allowing for the three new golden tests in P86/P87/P88 plus any incidental shifts).

**Falsifying check / exit condition.** Any failure → triage by test name. Most likely causes: (a) the new module's manifest claims a role that conflicts with `finalization-default`'s; (b) a host-integration test depends on `overhang_quartile` annotations existing on entities (now they don't — only `set-speed-factor` mutations); (c) an SDK test relies on the SHA staying constant.

---

## Step 9 — Draft ADR-0008 + acceptance ceremony

**Objective.** ADR drafted; status flip.

**Precondition.** Step 8 green.

**Postcondition.** `docs/adr/0008-overhang-as-finalization-module.md` exists with the rationale. Packet ready to flip to `implemented`.

**Files allowed to read.** None.
**Files allowed to edit.**
1. `docs/adr/0008-overhang-as-finalization-module.md` — CREATE.

The ADR records:
- Decision: overhang annotation is implemented by a `FinalizationModule` core-module owning the complete algorithm (relocated from `slicer-core`); no new stage; no host fallback.
- Why: Q3+Q6 grilling resolved that the existing `world-finalization::run-finalization` provides the seam; adding a stage = WIT contract change = unnecessary scope. The guest owns the complete algorithm (no `slicer-core` dep) because the `host-algos` feature gate on slicer-core would contaminate the guest dep tree (P84 lesson).
- Consequences: users opt out by curating their module dir; the AC-7 LSB-precision shift is the price of routing through `set-speed-factor`. Future overhang-classifier authors implement from scratch (or by forking the default).
- Future architecture reviewers should not re-suggest a dedicated stage and should not propose re-locating the algorithm back to slicer-core.

**Expected sub-agent dispatch.**
- (None — ADR drafting is implementer-side.)

**Context cost: S.**

**Narrow verification.** ADR file exists, ≤ 80 LOC, all three sections present.

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 Verify P86/P87 + baselines | S |
| 1 Survey template + SDK + xtask | S |
| 2 Scaffold module dir | S |
| 3 Module body | M |
| 4 AC-8 module test | S |
| 5 Delete classify_layers call | S |
| 6 Guest --check + AC-5 | S |
| 7 AC-7 SHA verdict + AC-6 ceremony | S |
| 8 Workspace test gate | M |
| 9 ADR-0008 | S |

Aggregate: **M.** No L step. Total step count: 10.

## Packet Completion Gate

Final batch checkpoint — workspace tests run.

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo xtask build-guests` (rebuild) green, then `cargo xtask build-guests --check` clean.
4. `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace` — green; count delta within ±10 vs Step 0 baseline.
5. AC-5 default invocation loads the new module (stderr log confirms).
6. AC-7 SHA verdict documented (byte-identical OR F-word-only shift).
7. AC-6 alternate SHA captured (different from default — confirms user opt-out works).
8. ADR-0008 committed.

## Acceptance Ceremony

- All 9 ACs (AC-1 .. AC-9) and 3 negative cases (AC-N1, AC-N2, AC-N3) gate green per the inline verification commands in `packet.spec.md`.
- ADR-0008 (`docs/adr/0007-overhang-as-finalization-module.md`) committed.
- Implementation log records: Step 0 baseline SHA, Step 7 post-packet SHA, AC-7 verdict (byte-identical or LSB-shift), Step 7 AC-6 alt-SHA, Step 8 workspace test count + duration.
- `status: draft` → `status: implemented` after gate green AND ADR in place AND user confirms closure. (`superseded` is reserved for packets replaced by a later spec.)
- **Batch closure**: P88's `implemented` flip closes the architecture-deepening batch (P81–P88). The deviation log entry from P81 (workspace tests at checkpoints only) is the audit trail for the batch.
