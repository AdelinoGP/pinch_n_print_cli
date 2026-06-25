# Implementation Plan: 125_voronoi-oom-hardening (rescoped)

## Execution Rules

- One atomic step at a time. TDD first (RED), then implement (GREEN), then narrow validation.
- No `docs/07` task IDs (bug-fix); steps trace to the WI labels in `requirements.md`.
- Locate by symbol — line numbers drift (tree has the WI-1 dumps + WIP).

## Steps

### Step 1: WI-1 — Diagnose + OOM tripwire — DONE
- Status: **complete** (recorded for provenance; do not redo).
- Delivered (in working tree): guarded `>1 GiB #[global_allocator]` in `crates/slicer-runtime/tests/executor/main.rs`; the confirmed chain + captured value (`region_id 0x3E8281949ECA9508`, `as u32` = 2,664,076,552 = `max_tool`, `vec![0.0f32; …]` = 9.924 GiB); bisect verdict = pre-existing committed bug (WIP innocent); temporary diagnostic dumps in `emit.rs` (removed in Step 6 — keep the allocator).
- Exit condition (met): crash site confirmed = `emit.rs` per-tool alloc fed by a leaked `region_id`.

### Step 2: WI-2 (B) — Safe tool fallback (crash-stop floor; TDD)
- Objective: a `region_id` identity can never enter the tool slot.
- Precondition: Step 1 complete.
- Postcondition: both `.unwrap_or(region.region_id)` fallbacks in `layer_executor.rs` (walls ~:743, paths ~:773) use a bounded default (`0` / named `DEFAULT_TOOL`); AC-1 green; `cube_fuzzy_painted_face_jitter` no longer OOMs.
- Files allowed to read: `layer_executor.rs` resolver chains (±40 lines); `mod.rs:169-178` (confirm what the identity is).
- Files allowed to edit (≤3): `crates/slicer-runtime/tests/integration/<new>_tdd.rs` (AC-1 RED first); `crates/slicer-runtime/src/layer_executor.rs`.
- Out-of-bounds: `emit.rs` (Step 4); `mod.rs` (read-only).
- Expected dispatches: "Run `cargo test -p slicer-runtime --test integration -- tool_fallback_never_leaks_region_identity`; FACT."
- Context cost: `S`
- Verification: AC-1 test green; `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter` no longer OOMs (may still fail its assertion until Step 5 — acceptable here).
- Exit condition: no tool slot can hold a `paint_variant_region_id` output; the OOM is gone.

### Step 3: WI-3 (A) — Restore correct paint→tool resolution (parity; TDD)
- Objective: painted entities resolve their real tool via `dominant_tool_index(&wl.feature_flags)`; the fallback never fires for painted geometry.
- Precondition: Step 2 complete; the (A) Open Question traced (where `feature_flags` should carry the tool).
- Postcondition: a painted entity's `paint_tool` is `Some(valid_tool)`; AC-2 green.
- Files allowed to read: `layer_executor.rs` `dominant_tool_index` + `feature_flags` population trace; `mod.rs` paint-seg propagation (range).
- Files allowed to edit (≤3): the traced (A) site (host paint-seg propagation OR a guest perimeter module — see Open Questions); a new/updated test for AC-2; (if guest) its `*.toml` is NOT needed unless a config key changes.
- Out-of-bounds: `emit.rs`.
- Expected dispatches: "Trace why `dominant_tool_index(&wl.feature_flags)` is `None` for a painted entity; LOCATIONS + ≤5-line FACT." ; "Run `cargo test -p slicer-runtime --test executor -- painted_entity_resolves_real_tool`; FACT."
- Context cost: `M`
- OrcaSlicer refs: none.
- Verification: AC-2 green. **If the (A) fix touches a guest perimeter module:** `cargo xtask build-guests --check` (rebuild if `STALE:`) before re-running.
- Exit condition: painted entities carry their real tool; the `.unwrap_or(0)` fallback is reached only by genuinely tool-less geometry.

### Step 4: WI-4 (Guard) — Emit bound-check (defense; TDD)
- Objective: `slicer-gcode/src/emit.rs` never sizes a dense per-tool `Vec` by an unvalidated id.
- Precondition: Step 2 complete (independent of Step 3).
- Postcondition: before `vec![0.0f32; max_tool + 1]`, an out-of-range `max_tool` (> extruder count) is rejected with a typed error or clamped; AC-N1 green.
- Files allowed to read: `emit.rs` around `filament_per_tool.keys().max()` + the `as u32` read (±40 lines).
- Files allowed to edit (≤3): `crates/slicer-gcode/tests/<new>_tdd.rs` (AC-N1 RED first); `crates/slicer-gcode/src/emit.rs`.
- Out-of-bounds: `layer_executor.rs`.
- Expected dispatches: "Run `cargo test -p slicer-gcode -- emit_rejects_out_of_range_tool_id`; FACT."
- Context cost: `S`
- Verification: AC-N1 green; the synthetic 2,664,076,552 id does not allocate > 1 GiB.
- Exit condition: emit refuses to OOM on a garbage tool id even if a future leak reappears.

### Step 5: WI-5 — Non-vacuous `cube_fuzzy_painted_face_jitter`
- Objective: a green run proves partition + fuzzy + tool actually worked.
- Precondition: Step 2 complete (no OOM); Step 3 in for the colour-region assertion to be meaningful.
- Postcondition: the two `eprintln!+return` escape hatches become failures conditioned on a successful slice (require `pts` non-empty AND both face bins > 0 so `painted > 2×` always runs); add a ≥2-distinct-`PaintValue`-colour-regions assertion; AC-4 green.
- Files allowed to read: the test body (escape hatches + bin computation).
- Files allowed to edit (≤3): `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs`.
- Out-of-bounds: production crates.
- Expected dispatches: "Run `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter`; FACT + assertion on fail."
- Context cost: `S`
- Verification: AC-4 green and non-vacuous (no `return` escape on the degraded path).
- Exit condition: AC-4 cannot pass vacuously.

### Step 6: WI-6 — Validate + cleanup
- Objective: full acceptance set green; remove temporary instrumentation, keep the tripwire.
- Precondition: Steps 2–5 complete.
- Postcondition: AC-3, AC-5, AC-6 green; the WI-1 diagnostic dumps removed from `emit.rs`; the guarded allocator retained; `executor` bucket green ×10.
- Files allowed to read: the WI-1 dump sites in `emit.rs`.
- Files allowed to edit (≤3): `crates/slicer-gcode/src/emit.rs` (remove dumps); `crates/slicer-runtime/tests/executor/` (the AC-5 repeat test if not already added).
- Out-of-bounds: the allocator (`main.rs`) — keep it.
- Expected dispatches: "Run `cargo test -p slicer-runtime --test executor`; FACT bucket pass/fail + names; SNIPPETS ≤20 on fail." ; "Run `cargo test -p slicer-runtime --test executor cube_4color_paint`; FACT 12/12."
- Context cost: `M`
- Verification: AC-3 (`T<n>` set matches painted tools), AC-5 (tripwire green ×10), AC-6 (cube_4color_paint 12/12); `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Exit condition: all ACs green; no temporary dumps remain; allocator retained.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 (WI-1) | — | DONE |
| 2 (WI-2 fallback) | S | local, two call sites |
| 3 (WI-3 paint→tool) | M | trace-then-fix; possible guest rebuild |
| 4 (WI-4 emit guard) | S | local |
| 5 (WI-5 non-vacuous test) | S | one test file |
| 6 (WI-6 validate + cleanup) | M | bucket run via FACT dispatch |

Aggregate `M`. No step `L`.

## Packet Completion Gate

- AC-1..AC-6 + AC-N1 each dispatched PASS.
- `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo xtask build-guests --check` clean (relevant only if Step 3 touched a guest).
- WI-1 diagnostic dumps removed; guarded allocator retained.
- The separate `fpv.is_finite()` painted-path panics logged as a follow-up (not fixed here); the
  `boostvoronoi::discretize` cap logged as separate optional hardening.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`; confirm PASS.
- Run the full `executor` bucket ×10 under the tripwire (FACT dispatch) — green.
- `cargo test --workspace` once at closure as a single FACT dispatch only, after all narrower commands pass.
- Record residual risk (the `region_id`/tool overload remains; the deferred fpv + discretize items) before `status: implemented`.
