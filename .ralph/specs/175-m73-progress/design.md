# Design: 175-m73-progress

## Controlling Code Paths

- Primary code path: `crates/slicer-gcode/src/estimator.rs` (169's module, already in tree — extended) → new `crates/slicer-gcode/src/m73.rs` → wiring inside `DefaultGCodeEmitter::emit_gcode` at the existing estimator call site (`crates/slicer-gcode/src/emit.rs:757-758`, where `EstimatorLimits::from_config(&self.resolved_config)` and `tool_diameters` are already built and `metadata.estimated_print_time_s` is filled). `crates/slicer-runtime/src/postpass.rs:49-51` only stashes the already-filled `GCodeIR` — it is NOT part of this packet's edit surface.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/` is per-file test binaries (no aggregator — `estimator.rs`, `gcode_emit_tdd.rs`, etc.), so new `tests/m73.rs` needs no `mod` registration; same for `crates/pnp-cli/tests/m73_progress_tdd.rs` (pattern: `slice_progress_events_default_tdd.rs`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The layer-boundary marker in the emitted stream is `GCodeCommand::Raw { text: ";LAYER_CHANGE" }` (pushed in `emit.rs` around line 331; `Raw` because the serializer's `Comment` arm prepends `"; "`). `inject_m73` detects boundaries by exact `Raw` text match `";LAYER_CHANGE"` — never by `Comment` variant.
- The serializer's `DefaultGCodeSerializer.filament_density_g_cm3` (default `1.24`, `serialize.rs:97`) is HEADER-BLOCK-only. Grams in the comment block come exclusively from the resolved-config `filament_density: Option<f32>` (`resolved_config.rs:792`); absent density ⇒ omit the `[g]` line (mirrors 169's `gcode_weight_grams` omission semantics).
- Injection happens on `GCodeIR.commands` (as `Raw` entries), not on serialized text — so `ThumbnailAwareSerializer` and any `GCodePostProcess` module see the M73 lines, and the injection is testable without a serializer.
- This packet's change surface includes `crates/slicer-ir/src/resolved_config.rs`, and `crates/slicer-ir/**` is a universal guest dependency — the wasm-staleness constraint below applies (triggered by Step 3's edit).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: post-emit injection pass over `GCodeIR.commands`, downstream of 169's estimator, upstream of serialization.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-gcode/src/estimator.rs` (169's file): add `pub fn estimate_print_with_elapsed(gcode_ir: &GCodeIR, limits: &EstimatorLimits, tool_diameters: &BTreeMap<u32, f32>) -> (PrintEstimate, Vec<f64>)` where the vector is cumulative elapsed seconds *after* each command index (`len == commands.len()`); refactor `estimate_print` to delegate to it (identical physics — the existing 169 tests must stay green unmodified).
  - New `crates/slicer-gcode/src/m73.rs`:
    - `pub fn inject_m73(gcode_ir: &mut GCodeIR, elapsed_s: &[f64])` — inserts `Raw` pairs: index 0 gets `M73 P0 R<round(total/60)>` + `M73 Q0 S<same>`; after each `";LAYER_CHANGE"` `Raw`, `pct = ((elapsed/total)*100).round() as u8`, `min = ((total-elapsed)/60).round() as u32`, emitted only when `(pct, min)` differs from the last emitted pair (Orca `process_line_move` dedup); stream end gets `M73 P100 R0` + `M73 Q100 S0`. `total = *elapsed_s.last()`; empty/zero-total streams are a no-op.
    - `pub fn filament_stats_comment_block(estimate: &PrintEstimate, filament_density: Option<f32>) -> Vec<GCodeCommand>` — `Raw` lines `; filament used [mm] = <per-tool "%.2f", ", "-joined in ascending tool order>`, `; filament used [cm3] = <volume/1000, %.2f>`, `; filament used [g] = <cm3 × density, %.2f>` (only when `filament_density.is_some()`), `; estimated printing time (normal mode) = <format_time_dhms(total_time_s)>`; plus private `fn format_time_dhms(s: f64) -> String` (`1d 2h 3m 4s`, zero-leading units omitted, seconds always present — Orca `get_time_dhms` behavior).
  - `crates/slicer-gcode/src/lib.rs`: `pub mod m73;` + re-exports.
  - `crates/slicer-ir/src/resolved_config.rs`: one macro line `cli "disable_m73" disable_m73: bool = false => extract_bool;` (mirror `cli "support_enabled" ... => extract_bool;` at ~line 723; the macro derives `apply_cli_key` and `Default`; check whether the key-list array around line 985 and `to_config_map` need the companion entries the neighboring bool keys have, and mirror exactly).
  - `crates/slicer-gcode/src/emit.rs`: at the existing estimator call site (`emit.rs:757-758`) — switch `estimate_print` to `estimate_print_with_elapsed`, then `if !self.resolved_config.disable_m73 { inject_m73(&mut gcode_ir, &elapsed) }`, then append `filament_stats_comment_block(&estimate, self.resolved_config.filament_density)` to `gcode_ir.commands` before `Ok(gcode_ir)`. Because this runs inside `emit_gcode`, the M73/comment lines are automatically visible to every `GCodePostProcess` module and to serialization — no runtime-crate edit needed.
  - Tests: new `crates/slicer-gcode/tests/m73.rs` (AC-1, AC-2, AC-3, AC-N2 — synthetic `GCodeIR`, no WASM); new `crates/pnp-cli/tests/m73_progress_tdd.rs` (AC-4, AC-N1 — fixture slice via `run_slice`, mirroring `slice_progress_events_default_tdd.rs`'s harness).
- Rejected alternatives and reasons:
  - Serializer-side text injection (string post-process): invisible to `GCodePostProcess` modules and to `GCodeIR` consumers; harder to test; rejected.
  - Per-G1-line M73 (full Orca `process_line_move` parity): O(commands) inserted lines for negligible fork value (hosts sample at seconds granularity); layer-boundary emission is the plan-approved deviation, documented here.
  - A second stealth estimate (Orca's dual time-machine): plan explicitly mandates the same estimate under both masks; rejected as out of scope.

## Files in Scope (read + edit)

- `crates/slicer-gcode/src/estimator.rs` — role: 169's estimator (in tree); expected change: add `estimate_print_with_elapsed`, delegate `estimate_print`.
- `crates/slicer-gcode/src/m73.rs` — role: new injection + comment-block module; expected change: created.
- `crates/slicer-gcode/src/emit.rs` — role: wiring at the estimator site (:757-758); expected change: ~10 lines.
- (Extras, justified — one-line each, split across steps): `crates/slicer-gcode/src/lib.rs` (module export), `crates/slicer-ir/src/resolved_config.rs` (one macro line + companions), two new test files.

## Read-Only Context

- `crates/slicer-gcode/src/emit.rs` — lines `315-345` (marker shape) and `735-760` (estimator site) only; the file is an edit target but only these ranges may be read.
- `crates/slicer-gcode/src/serialize.rs` — lines `560-750` only — purpose: `Raw` passthrough arm and `format_coord`; confirm `Raw` lines serialize verbatim.
- `crates/slicer-ir/src/slice_ir.rs` — lines `2195-2295` only — purpose: `GCodeCommand` variants, `PrintMetadata`, `GCodeIR`.
- `crates/slicer-ir/src/resolved_config.rs` — lines `715-800` and `975-1010` only — purpose: `cli`/`cli_opt` macro line patterns and key-list companions.
- `.ralph/specs/169-time-estimator-slice-stats/design.md` — via SUMMARY dispatch only — purpose: reconfirm export names at implementation time.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load
- `.ralph/specs/169-time-estimator-slice-stats/**` — read-only predecessor; NEVER edit; consume via SUMMARY dispatch
- `crates/slicer-runtime/src/postpass.rs` and `run.rs` — NOT edit targets (postpass only stashes the already-filled IR; run.rs is 169's event surface); `crates/slicer-gcode/src/emit.rs` beyond the stated ranges; CONFIG_BLOCK tables in `serialize.rs`

## Expected Sub-Agent Dispatches

- Question: reconfirm the estimator exports at implementation time (`estimate_print` :168, `PrintEstimate` :91, `EstimatorLimits` :24) and the `emit.rs:757-758` call-site shape are unchanged; scope: `crates/slicer-gcode/src/estimator.rs` + `emit.rs` lines 735-760; return: `FACT`; purpose: Step 1/Step 4 precondition (169 is still draft; its closure work could move them).
- Question: exact Orca dedup + first/last M73 semantics and `get_time_dhms` formatting; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp`; return: `SUMMARY`; purpose: Step 2.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, m73.rs + its unit tests)
- Highest-risk dispatch and required return format: 169 export-name confirmation — `FACT` (mismatch blocks the packet).

## Open Questions

- `[FWD]` Whether `to_config_map`/the key-list array in `resolved_config.rs` need companion entries for `disable_m73`: mirror whatever the nearest `bool` key (`support_enabled`) does — resolvable by the Step 3 read.
- `[FWD]` Exact insertion index for the start pair (absolute index 0 vs after the leading `ExtrusionMode` command at index 0): either is firmware-valid; pick index 0 and keep tests order-agnostic beyond "first M73 line is P0".
- `[BLOCK]` (activation only, not design) Packet 169 must be `implemented` first; this packet must not be swarmed before 169's ceremony.
