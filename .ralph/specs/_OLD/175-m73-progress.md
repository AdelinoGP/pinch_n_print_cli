---
status: implemented
packet: 175-m73-progress
task_ids:
  - TASK-279
---

# 175-m73-progress

## Goal

Emit `M73 P<pct> R<remaining_min>` plus stealth `M73 Q<pct> S<remaining_min>` progress lines (same estimate for both masks) at stream start, every layer boundary, and stream end, and append the `; filament used [mm]/[cm3]/[g]` + `; estimated printing time` comment block — all driven by packet 169's trapezoidal estimator, gated by a new `disable_m73` config key (bool, default false).

## Problem Statement

The fork's print hosts and firmware read `M73` remaining-time lines and the `; filament used` / `; estimated printing time` comment block to drive progress bars and material accounting; PNP emits neither (verified: zero `M73` occurrences under `crates/`). Packet 169 builds the trapezoidal estimator but explicitly excludes M73 and names this packet as its wave-2 unblock. This is one coherent slice: everything here is a pure consumer of 169's `PrintEstimate`, layered onto the already-emitted `GCodeIR` command stream.

## Architecture Constraints

- The layer-boundary marker in the emitted stream is `GCodeCommand::Raw { text: ";LAYER_CHANGE" }` (pushed in `emit.rs` around line 331; `Raw` because the serializer's `Comment` arm prepends `"; "`). `inject_m73` detects boundaries by exact `Raw` text match `";LAYER_CHANGE"` — never by `Comment` variant.
- The serializer's `DefaultGCodeSerializer.filament_density_g_cm3` (default `1.24`, `serialize.rs:97`) is HEADER-BLOCK-only. Grams in the comment block come exclusively from the resolved-config `filament_density: Option<f32>` (`resolved_config.rs:792`); absent density ⇒ omit the `[g]` line (mirrors 169's `gcode_weight_grams` omission semantics).
- Injection happens on `GCodeIR.commands` (as `Raw` entries), not on serialized text — so `ThumbnailAwareSerializer` and any `GCodePostProcess` module see the M73 lines, and the injection is testable without a serializer.
- This packet's change surface includes `crates/slicer-ir/src/resolved_config.rs`, and `crates/slicer-ir/**` is a universal guest dependency — the wasm-staleness constraint below applies (triggered by Step 3's edit).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- IR/manifest contracts: no schema change — `Raw` is an existing `GCodeCommand` variant; `GCodeIR.schema_version` untouched. `metadata.estimated_print_time_s` stays 169's fill.
- WIT boundary: none.
- Determinism/scheduler constraints: injection is a pure function of `(commands, elapsed_s, config)` — byte-identical output across runs; no scheduler interaction.

## Locked Assumptions and Invariants

- `M73 Q<p> S<r>` always carries values identical to its adjacent `M73 P<p> R<r>` (single-estimate contract; a future stealth estimator would amend this packet's tests).
- `disable_m73 = true` suppresses M73 only; the comment block is unconditional.
- Layer-boundary granularity (not per-move) is a locked, documented deviation from Orca.

## Risks and Tradeoffs

- 169's packet is still `draft` even though its estimator code is in the working tree; its remaining closure work could still rename or move the exports/call site — Step 1's precondition FACT catches drift; reconcile before proceeding, never fork a second estimator.
- Print hosts that require strictly per-move M73 density would see coarse steps; acceptable per plan (fork samples at layer cadence).
- Inserting into `Vec<GCodeCommand>` mid-stream is O(n²) if done naively per boundary; build a new Vec in one pass.
