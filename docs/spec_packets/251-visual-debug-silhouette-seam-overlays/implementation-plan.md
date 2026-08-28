# Implementation Plan: 251-visual-debug-silhouette-seam-overlays

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: `OverlayEvent::Seam` gains optional `z` (serialization-compat locked)

- Task IDs: `TASK-455`
- Objective: add `z: Option<f32>` with `skip_serializing_if` to `OverlayEvent::Seam`, set `z: None` at both existing construction sites (`collect_overlay_events`'s `Perimeter` and `SeamPlan` arms), and add `..` to the two glyph-loop or-patterns; TDD the serialization shape (a `Seam { z: None }` serializes without a `z` key; `Some` serializes `"z"`).
- Precondition: compiling tree (independent of 247 — the enum and its sites exist today).
- Postcondition: workspace compiles; `visual_debug_overlays_tdd` green unchanged; serialization pins green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_style.rs` — `OverlayEvent` region only
  - `crates/slicer-runtime/src/visual_debug_render.rs` — `collect_overlay_events` + `draw_overlay_events` regions only (symbol-anchored)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_style.rs`
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs`; packet dirs 247–250
- Blast-radius discipline (mandatory — field addition to a public enum variant):
  - Dispatch a `LOCATIONS` worker for every `OverlayEvent::Seam` site in `crates/` before editing; the pre-authoring sweep (2026-08-27, pre-247 tree) found: constructions `visual_debug_render.rs` (`collect_overlay_events` ×2), patterns `visual_debug_render.rs` (`draw_overlay_events`), `visual_debug_gcode.rs` (glyph loop), `visual_debug_style.rs` (`kind`, `event_glyph` — already `{ .. }`); zero test literals. Packets 247–250 may have added sites — the fresh sweep is authoritative.
- Expected sub-agent dispatches:
  - Question: every `OverlayEvent::Seam` site; scope: `crates/`; return: `LOCATIONS ≤10`
  - Question: run `cargo test -p pnp-cli --test visual_debug_overlays_tdd` tee'd to `target/test-output.log`; return: `FACT pass/fail`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D18's `z` clause + §10 item 8 (ranged)
- OrcaSlicer refs: none (no parity).
- Verification:
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo test -p pnp-cli --test visual_debug_overlays_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
- Exit condition: workspace compiles; overlays suite green; serialization unit pins (no-`z`-key on `None`) pass. Falsifying exit: any existing overlays/gcode test asserts a changed byte — the field leaked a `Some` or lost the skip attribute; fix the code, never the test.

### Step 2: R9 validation matrix + pin retirements (TDD)

- Task IDs: `TASK-456`
- Objective: add `VisualizationOptions.composited_overlays`; implement every R9/R10 validation arm (seams-only on silhouette `overlays` and `composited_overlays`; silhouette-only; model-source-only with `OverlayUnsupportedOnGcode` for the gcode seam forms; 1.2.0-only incl. the 1.0.0 stray-key loop; non-empty; group-conflict rejection); retire 247's `composited_overlays_not_accepted_by_247`, retarget 248's `gcode_silhouette_overlay_rejections_unchanged` → `gcode_seam_overlay_forms_rejected`, and add `unknown_option_keys_still_rejected`.
- Precondition: Step 1 done; packet 247 implemented (silhouette validation exists — FORWARD-DEP); the two retirement greps are vacuously true until 247/248 are implemented (queue-order artifact).
- Postcondition: AC-N1..N7's validation tests green; no bundle-writing change yet (the legal forms parse and validate but Step 4 wires rendering — validation-only tests must not require a render).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — `VisualizationOptions` + `validate_request` overlays block + the 1.0.0 stray-key loop (symbol-anchored)
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` — the harness helpers + the two retired tests only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`; packet dirs 247–250
- Expected sub-agent dispatches:
  - Question: exact assertions of the two prior pins being retired; scope: `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`; return: `FACT`
  - Question: run the validation binary tee'd; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §6 R9/R10 + D18 validation clause (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `! rg -q 'composited_overlays_not_accepted_by_247' crates/ && ! rg -q 'gcode_silhouette_overlay_rejections_unchanged' crates/ && echo RETIRED` — FACT
- Exit condition: every R9 arm has a red-then-green test; both old pin names absent; unknown-key rejection still green. Falsifying exit: `composited_overlays` accepted under 1.1.0's strict parse (the field now deserializes there) without the schema check — AC-N4's test must catch it; if it does not, the test is wrong, not the gate.

### Step 3: Renderer seam entries (TDD)

- Task IDs: `TASK-455`
- Objective: implement `silhouette_seam_events` (source-order layer filter, per-view coords, `z: Some`), `render_silhouette_seam_overlay` (FAINT_BASE base + glyphs), and `render_silhouette_composite_seamed` (glyph pass over the colored base; existing styled entry delegates with no seams); re-export; TDD AC-2/AC-6 plus glyph-pixel assertions for both forms.
- Precondition: Steps 1–2 done; packet 247 implemented (composite internals + `visual_debug_silhouette_tdd` exist — FORWARD-DEP); if packet 250's `render_gcode_emit_silhouette` landed, its byte behavior without seams is untouched.
- Postcondition: renderer-level AC-2/AC-6 green; 247/249's silhouette suites green unchanged (delegation byte-equivalence).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/visual_debug_render.rs` — composite internals + `render_stage_capture_styled`'s `OverlayIsolated` arm (symbol-anchored)
  - `crates/slicer-ir/src/slice_ir.rs` — `SeamPlanIR`/`SeamPosition`/`RegionKey` definitions only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`; packet dirs 247–250
- Expected sub-agent dispatches:
  - Question: run the silhouette runtime suite tee'd; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
  - Question: `cargo xtask check-literals` exit code after fixtures; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D18 rendering forms + §7 determinism (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail (the whole binary — 247/249/250's tests must stay green)
- Exit condition: AC-2/AC-6 test names pass; the full silhouette binary green. Falsifying exit: any pre-existing composite test's PNG bytes change — the delegation is not byte-equivalent; fix the factoring.

### Step 4: Assembly wiring + bundle tests

- Task IDs: `TASK-456`
- Objective: wire the silhouette branch — seam plan read (`ctx.blackboard.seam_plan()`, fail closed when `None` and seams requested), `rendered_layers` set, isolated image per (tap, view) at `_overlay_seams`, composited base via the seamed entry, `ImageEntry.composited_overlays` field (+ `None` at every literal site), event mirrors — and author the new bundle binary (AC-1/3/4/5/7, N8), including the extracted-helper test for the missing-seam-plan path.
- Precondition: Steps 2–3 done; packets 247 and 249 implemented (silhouette branch + tool-carrying tap for AC-7 — FORWARD-DEP; queue order guarantees both).
- Postcondition: AC-1/3/4/5/7 and AC-N8 green end-to-end.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` — silhouette branch + `ImageEntry` + the isolated-overlay top-down arm (symbol-anchored)
  - `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs` — wedge harness helpers only
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_seam_overlay_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` (consumed via Step-3 exports); packet dirs 247–250
- Blast-radius discipline (mandatory — `ImageEntry` field addition):
  - Dispatch a `LOCATIONS` worker for every `ImageEntry { ... }` literal before editing (4 sites pre-247, all in `visual_debug.rs`; packets 247–250 added silhouette sites — the fresh sweep is authoritative); add `composited_overlays: None` at each; re-verify no test constructs the struct as a Rust literal.
- Expected sub-agent dispatches:
  - Question: every `ImageEntry` literal site; scope: `crates/pnp-cli/`; return: `LOCATIONS ≤15`
  - Question: `cargo xtask build-guests --check` exit code (0/1/3), then run the new binary tee'd; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
  - Question: `cargo xtask check-literals` exit code; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — §5 request/manifest sketch + D18 (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — FACT pass/fail
  - `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` — 247's bundle suite must stay green (no-overlay bundles byte-unchanged)
- Exit condition: both binaries green; the AC-5 legacy-serialization test passes against a real 1.1.0 bundle. Falsifying exit: 247's bundle suite reports a byte change on a no-overlay bundle — the seamed delegation or the `ImageEntry` field leaked into legacy output; fix before proceeding.

### Step 5: Docs

- Task IDs: `TASK-457`
- Objective: write `docs/19_visual_debug.md`'s "Seam overlays" silhouette subsection (AC-8 anchor `composited_overlays`): both forms' semantics, R9 rules, the `z` mirror and its 1.0/1.1 absence, `_overlay_seams` filename, glyph-over-palette visibility caveat, sub-pixel-band glyph note (D4 interaction).
- Precondition: Step 4 done (documented behavior exists).
- Postcondition: AC-8 grep passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` — silhouette + isolated-overlay sections only (heading-anchored)
- Files allowed to edit (at most 3):
  - `docs/19_visual_debug.md`
- Files explicitly out of bounds:
  - `docs/02_ir_schemas.md`; `docs/DEVIATION_LOG.md` (no deviation rows in this packet)
- Expected sub-agent dispatches: none (single bounded doc edit).
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-debug-silhouette-side-views-plan.md` — D18 + §8 exclusions (ranged)
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'composited_overlays' docs/19_visual_debug.md && echo PASS` — FACT
- Exit condition: the grep passes and the subsection names both forms, the `z` field, and the exclusion list. Falsifying exit: `composited_overlays` already present in the doc before this step wrote it (would make AC-8 vacuous) — verified absent 2026-08-27; re-verify before writing.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | additive field + pattern fixes + serialization pins |
| Step 2 | M | R9 matrix + two cross-packet retirements |
| Step 3 | M | renderer entries + delegation equivalence |
| Step 4 | M | assembly + bundle binary + blast radius |
| Step 5 | S | docs |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (glyph legibility over tool palettes; group-conflict strictness).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
