# Packet 86 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- `gcode_emit.rs` is 1 914 LOC; NEVER load in full. Section-by-section grep + ±50-line reads only.
- The packet closure gate runs narrow per-crate tests, NOT `cargo test --workspace` (P86 is not a checkpoint; the last checkpoint was P85, the next is P88).
- P84 MUST be closed (Step 0 verifies): `FeedrateConfig` in `slicer-ir`, `classify_layers` in `slicer-core`.
- No guest-feeding path is edited; `cargo xtask build-guests --check` should stay clean throughout.

---

## Step 0 — Verify P84 closure + capture pre-packet g-code SHA baseline

**Objective.** Confirm prereq: `FeedrateConfig` lives in `slicer-ir`, `classify_layers` lives in `slicer-core`. Capture the current g-code SHA (carried forward from P85 closure if P85 is also done, otherwise from P84).

**Precondition.** P84 is `superseded`. Working tree clean.

**Postcondition.** Two log entries: P84-state verification, baseline SHA.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: confirm `rg -l 'pub struct FeedrateConfig' crates/ | grep -qE '^crates/slicer-ir/' && rg -l 'pub fn classify_layers' crates/ | grep -qE '^crates/slicer-core/'`. Return FACT pass/fail.
- Dispatch: g-code SHA. `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/p86-baseline.gcode && sha256sum /tmp/p86-baseline.gcode`. Return FACT `<hex>`.

**Context cost: S.**

**Narrow verification.** Both returns positive.

**Falsifying check / exit condition.** P84 verification fails → abort.

---

## Step 1 — Enumerate `EmitContext` trait surface, trait sigs, test consumers, OrcaSlicer refs

**Objective.** Surface every input needed to write the new types correctly.

**Precondition.** Step 0 green.

**Postcondition.** Five lists/SNIPPETS in the log per design.md dispatches #1, #2, #3, #4, and one OrcaSlicer parity dispatch (#5 / #6 combined).

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch #1: `Blackboard` methods `emit_gcode` calls — LOCATIONS.
- Dispatch #2: `pub trait GCodeEmitter` and `pub trait GCodeSerializer` verbatim signatures — SNIPPETS.
- Dispatch #3: test files referencing the moved serialization symbols — LOCATIONS.
- Dispatch #4: `base64` usage outside `gcode_emit.rs` — FACT yes/no.
- Dispatch #5+#6: OrcaSlicer references for `;TYPE:` placement, layer markers, thumbnail sentinels — LOCATIONS + FACT.

**Context cost: S.**

**Narrow verification.** Five returns populated.

**Falsifying check / exit condition.** Dispatch #1 returns > 15 `Blackboard` methods → consider whether `&Blackboard` actually IS the right interface and a different seam (e.g., owned snapshot type) might be cleaner. Surface to user before continuing.

---

## Step 2 — Scaffold `slicer-gcode` crate + `EmitContext` trait + `GCodeEmitError` enum

**Objective.** New crate compiles standalone with empty trait impls.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `cargo build -p slicer-gcode` green against an empty `lib.rs` plus the new `context.rs` containing `EmitContext` and `GCodeEmitError`.

**Files allowed to read.** Workspace `Cargo.toml`.
**Files allowed to edit.**
1. Workspace `Cargo.toml` — add `"crates/slicer-gcode"` to `members`.
2. `crates/slicer-gcode/Cargo.toml` — CREATE per design.md.
3. `crates/slicer-gcode/src/lib.rs` — CREATE with module declarations (`pub mod emit; pub mod serialize; pub mod thumbnail; pub mod context;`).
4. `crates/slicer-gcode/src/context.rs` — CREATE with the `pub trait EmitContext { ... }` (methods enumerated by Step 1 dispatch #1) and `pub enum GCodeEmitError { ... }` (variants mirroring the failure modes in `gcode_emit.rs`).
5. The other three submodule files (`emit.rs`, `serialize.rs`, `thumbnail.rs`) — CREATE as empty placeholders.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-gcode`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Crate builds.

**Falsifying check / exit condition.** Build fails → likely a workspace inheritance issue or a missing `slicer-core` re-export.

---

## Step 3 — Move `gcode_emit.rs` content into `slicer-gcode/src/`; relocate the two trait defs

**Objective.** All pure-serialization code lives in `slicer-gcode/src/{emit,serialize,thumbnail}.rs`. The `GCodeEmitter` and `GCodeSerializer` trait defs land in `slicer-gcode/src/lib.rs` (or wherever `serialize.rs` declares them). The `&Blackboard` parameter on `emit_gcode` is rewritten to `&dyn EmitContext`; the `PostpassError` return becomes `GCodeEmitError`.

**Precondition.** Step 2 complete.

**Postcondition.** `cargo build -p slicer-gcode` green with the moved content. `crates/slicer-runtime/src/gcode_emit.rs` deleted. `crates/slicer-runtime/src/postpass.rs` has the trait defs removed but does NOT yet build (lib.rs still has stale `pub mod gcode_emit;`).

**Files allowed to read.** `crates/slicer-runtime/src/gcode_emit.rs` (line ranges only — never full file); `crates/slicer-runtime/src/postpass.rs` L130–180.
**Files allowed to edit.**
1. `crates/slicer-gcode/src/emit.rs` — fill with `DefaultGCodeEmitter` + the `GCodeEmitter` trait def. Replace `&Blackboard` with `&dyn EmitContext` in `emit_gcode`'s sig. Replace `PostpassError` returns with `GCodeEmitError`.
2. `crates/slicer-gcode/src/serialize.rs` — fill with `DefaultGCodeSerializer`, `ThumbnailAwareSerializer`, `tolerance_for_role`, `GCodeSerializer` trait def. Same error-type rewrite.
3. `crates/slicer-gcode/src/thumbnail.rs` — fill with `serialize_thumbnail_block`.
4. `crates/slicer-gcode/src/lib.rs` — re-export the public surface.
5. Delete `crates/slicer-runtime/src/gcode_emit.rs`.
6. `crates/slicer-runtime/src/postpass.rs` — delete the two trait defs (L144–163); add `use slicer_gcode::{GCodeEmitter, GCodeSerializer};`.

**Inside `emit_gcode`'s body**: every call to `blackboard.X(...)` becomes `ctx.X(...)` where `X` is a method on `EmitContext`. The call to `classify_layers(&mut layers, &feedrate_config)` is now `slicer_core::classify_layers(&mut layers, &feedrate_config)` (per AC-6). The `FeedrateConfig` import becomes `use slicer_ir::FeedrateConfig;`.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-gcode`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** `slicer-gcode` builds. `rg -l 'pub trait GCodeEmitter' crates/` → exactly `crates/slicer-gcode/`.

**Falsifying check / exit condition.** Build fails on missing `EmitContext` method → return to Step 2's `context.rs` and add the missing method per dispatch #1.

---

## Step 4 — Create the `GCodeEmitProducer` wrapper in `slicer-runtime/src/builtins/`

**Objective.** The runtime-side glue exists. It impls `BuiltinProducer` for `GCODE_EMIT_PRODUCER` and commits to `Blackboard`.

**Precondition.** Step 3 complete.

**Postcondition.** `crates/slicer-runtime/src/builtins/gcode_emit_producer.rs` exists, ≤ 80 LOC, declares the `pub static GCODE_EMIT_PRODUCER`.

**Files allowed to read.** `crates/slicer-runtime/src/dag.rs` (for `BuiltinProducer` trait shape, now imported from `slicer-scheduler` post-P85 if applicable, else still `crate::dag` if P85 hasn't shipped); `crates/slicer-runtime/src/blackboard.rs` (to find the right `replace_gcode_ir` / commit method).
**Files allowed to edit.**
1. `crates/slicer-runtime/src/builtins/gcode_emit_producer.rs` — CREATE the wrapper.
2. `crates/slicer-runtime/src/builtins/mod.rs` — add `pub mod gcode_emit_producer;` + re-export `GCODE_EMIT_PRODUCER`.

**Wrapper body shape (illustrative — exact API depends on `BuiltinProducer` trait):**
```rust
pub static GCODE_EMIT_PRODUCER: BuiltinProducer = BuiltinProducer { /* same stage_id/world_id/claim as before */ };

impl BuiltinProducer {
    pub fn run_gcode_emit(bb: &mut Blackboard, /* args */) -> Result<(), PostpassError> {
        let emitter = DefaultGCodeEmitter::new(/* read config from bb */);
        let gcode_ir = emitter.emit_gcode(&bb.layers(), bb as &dyn EmitContext)
            .map_err(PostpassError::from)?;
        bb.replace_gcode_ir(gcode_ir);
        Ok(())
    }
}
```

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-runtime`. Return FACT pass/fail. **EXPECTED FAIL** — lib.rs still has stale `pub mod gcode_emit;` and `runtime_builtins()` references; Step 5 fixes them.

**Context cost: S.**

**Narrow verification.** The wrapper file exists; its content is ≤ 80 LOC and contains the `GCODE_EMIT_PRODUCER` static.

**Falsifying check / exit condition.** None at this step — the runtime build fails by design until Step 5.

---

## Step 5 — Rewire `slicer-runtime/src/lib.rs`, Cargo.toml, postpass.rs, `runtime_builtins()`, and `Blackboard`'s `EmitContext` impl

**Objective.** Workspace builds; ADR-0001 preserved.

**Precondition.** Step 4 complete.

**Postcondition.** `cargo build --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` green.

**Files allowed to read.** `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/Cargo.toml`, `crates/slicer-runtime/src/blackboard.rs`, `crates/slicer-runtime/src/postpass.rs`.
**Files allowed to edit.**
1. `crates/slicer-runtime/src/lib.rs` — drop `pub mod gcode_emit;`. Rewrite or drop `pub use postpass::{GCodeEmitter, GCodeSerializer};` (rewrite to `pub use slicer_gcode::{GCodeEmitter, GCodeSerializer};` for transitional source compat). Rewrite the `gcode_emit::*` re-exports analogously. Update `runtime_builtins()` to reference `builtins::gcode_emit_producer::GCODE_EMIT_PRODUCER`.
2. `crates/slicer-runtime/Cargo.toml` — add `slicer-gcode = { path = "../slicer-gcode" }`.
3. `crates/slicer-runtime/src/blackboard.rs` — add `impl slicer_gcode::EmitContext for Blackboard { ... }` covering the trait's method set. Each impl method delegates to an existing `Blackboard` method.
4. `crates/slicer-runtime/src/postpass.rs` — already edited in Step 3; verify clean state.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail + first failing crate.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** Both green.

**Falsifying check / exit condition.** Build fails → most likely an `EmitContext` method missing from `Blackboard` impl; check dispatch #1's list and add.

---

## Step 6 — Migrate or rewire tests per dispatch #3

**Objective.** Per-crate test gates green.

**Precondition.** Step 5 complete.

**Postcondition.** `cargo test -p slicer-gcode -p slicer-runtime -p pnp-cli` green.

**Files allowed to read.** Test files from Step 1 dispatch #3.
**Files allowed to edit.**
1. Move test files whose SUT is serialization (`DefaultGCodeSerializer`, `format_*`, `tolerance_for_role`, `serialize_thumbnail_block`) → `crates/slicer-gcode/tests/`. Imports rewrite to `slicer_gcode::*`.
2. Tests whose SUT is `GCODE_EMIT_PRODUCER` end-to-end → stay; imports rewrite from `slicer_runtime::DefaultGCodeEmitter` to `slicer_gcode::DefaultGCodeEmitter` (or via the transitional re-export in lib.rs).
3. `crates/slicer-runtime/tests/{integration,executor}/main.rs` aggregators — drop `mod` declarations for moved tests.

**Expected sub-agent dispatches.**
- Dispatch: `cargo test -p slicer-gcode`. Return FACT pass/fail + count.
- Dispatch: `cargo test -p slicer-runtime`. Return FACT pass/fail + count + delta.
- Dispatch: `cargo test -p pnp-cli`. Return FACT pass/fail + count.

**Context cost: M.**

**Narrow verification.** All three green.

**Falsifying check / exit condition.** A test fails on import → check whether the transitional re-export in `slicer-runtime/src/lib.rs` is missing the named symbol; add it.

---

## Step 7 — Add the AC-8 golden test under `slicer-gcode/tests/`

**Objective.** A golden test exercises the serializer end-to-end without `slicer-runtime`.

**Precondition.** Step 6 green.

**Postcondition.** `cargo test -p slicer-gcode` passes with at least one new golden test asserting documented sentinel substrings.

**Files allowed to read.** None.
**Files allowed to edit.**
1. `crates/slicer-gcode/tests/golden_emit_tdd.rs` — CREATE.

The test:
- Constructs a tiny `GCodeIR` with one wall path, one infill path, one travel move. Optionally embeds a thumbnail.
- Calls `DefaultGCodeSerializer::default().serialize_gcode(&gcode_ir).unwrap()`.
- Asserts the resulting string contains `;TYPE:WALL_OUTER`, `;LAYER:0`, the OrcaSlicer-parity sentinels from Step 1 dispatch #5/#6.
- Imports zero `slicer_runtime::*` types.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test -p slicer-gcode --test golden_emit_tdd`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Test passes.

**Falsifying check / exit condition.** Test fails → the asserted sentinel string differs from what `DefaultGCodeSerializer` emits; check Step 1 dispatch #5's findings vs the actual emitted string.

---

## Step 8 — Confirm guest WASMs stay clean

**Objective.** No guest-feeding path was inadvertently edited.

**Precondition.** Step 7 green.

**Postcondition.** `cargo xtask build-guests --check` clean.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo xtask build-guests --check`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Clean.

**Falsifying check / exit condition.** STALE → investigate.

---

## Step 9 — AC-7 g-code SHA parity

**Objective.** Confirm byte-identical g-code output vs the Step 0 baseline.

**Precondition.** Step 8 green.

**Postcondition.** Post-packet SHA = Step 0 baseline SHA.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p86.gcode && sha256sum /tmp/benchy-p86.gcode`. Return FACT `<hex>`. Compare to Step 0 baseline.

**Context cost: S.**

**Narrow verification.** SHAs match.

**Falsifying check / exit condition.** SHA divergence → bisect; the most likely culprit is the `EmitContext` impl on `Blackboard` returning slightly different data than the pre-move `&Blackboard` access would have.

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 P84 verify + baseline | S |
| 1 Enumerate trait surface, test consumers, OrcaSlicer refs | S |
| 2 Scaffold slicer-gcode + EmitContext + GCodeEmitError | S |
| 3 Move gcode_emit.rs + traits; rewire emit_gcode sig | M |
| 4 Create wrapper in slicer-runtime/src/builtins/ | S |
| 5 Runtime rewire + Blackboard impl EmitContext | M |
| 6 Test migration / rewires | M |
| 7 Golden test | S |
| 8 Guest --check clean | S |
| 9 g-code SHA parity | S |

Aggregate: **M.** No L step. Total step count: 10.

## Packet Completion Gate

Narrow-only gates per the deepening-batch policy.

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo xtask build-guests --check` — clean.
4. `cargo test -p slicer-gcode -p slicer-runtime -p pnp-cli` — green; counts as expected.
5. AC-7 post-packet SHA = Step 0 baseline.

## Acceptance Ceremony

- All 9 ACs (AC-1 .. AC-9) and 3 negative cases (AC-N1, AC-N2, AC-N3) gate green per the inline verification commands in `packet.spec.md`.
- No ADR follow-up.
- Implementation log records: Step 0 baseline SHA, Step 9 post-packet SHA, `EmitContext` trait surface (final method list), list of moved tests, list of transitional re-exports added to `slicer-runtime/src/lib.rs`.
- `status: draft` → `status: superseded` after gate green AND user confirms closure.
