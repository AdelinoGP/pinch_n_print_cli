# Design: 281-machine-motion-limits-emitter

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) → 267's envelope builder (same crate) → `GCodeCommand::Raw` machine-limit lines prepended ahead of `M73`/start block; `EstimatorLimits::from_config` (`crates/slicer-gcode/src/estimator.rs`) → `estimate_event_time` / `estimate_command_deltas` clamping.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (267's `machine_envelope*` tests — extend, do not rename), `crates/slicer-gcode/tests/estimator.rs` (`EstimatorLimits::from_config` fallback test at ~line 190, `estimate_print_with_elapsed` harness at ~line 240), `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` (field-default pins).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Host-emitter ownership stands (ticket-27 re-derivation hazard): all eight keys are emission-time/estimation-time decisions in `crates/slicer-gcode`. No manifest row in `machine-gcode-emit.toml` (verified: zero `machine_*` rows today), no module code reads these keys, no WIT/IR field beyond `ResolvedConfig`.
- Canonical `coFloats` stride-2 pairs collapse to scalar-global `Option<f32>` with `extract_float_or_first` ingest (ticket-140 fix for `enable_pressure_advance`/`filament_flush_*` is the exact precedent — same file, same macro arm shape). First filament/extruder entry wins; full vector ingest stays with ticket 125. Recorded as DEV-173, not a gap.
- Canonical minima (`min = 0` on all eight) are enforced as bounds rows; canonical maxima (including JD's `0.3`) are GUI hints per the ticket-113 ruling and are NOT enforced — a `max` on any of these fields would be a PnP invention.
- The `M205 J` line reuses the existing `GcodeFlavor::set_junction_deviation` helper (`crates/slicer-gcode/src/flavor.rs`) — no new flavor formatting code. RRF ×60 applies to `M203`/`M566` values only, never to `M201`.
- Default-path identity: all eight fields default `None`; `None` omits its envelope component and falls back to `EstimatorLimits::default()` (accel 1500/1500, min rates 0.0/disabled). Canonical's concrete defaults (accel x/y 1000, JD 0.01) are NOT adopted as port defaults — same stance as the ten live siblings — so the choice is consistent, not a new divergence.

## Code Change Surface

- Selected approach: extend, don't fork. The eight fields join the existing ten in `ResolvedConfig` + `to_config_map`; the envelope builder gains three conditional components (`M201`, `M204 R`, `M205 J`) in canonical position; the estimator gains two clamp fields. Total new decision logic is ~40 lines plus tests.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs`: 8 `cli_opt @printer` field declarations + 8 `to_config_map` arms + scheduler bounds rows (via `ConfigBoundsIndex` seeding or the speed-bounds seam — follow whichever of the two ticket-113 left; do not invent a third).
  - `crates/slicer-gcode/src/emit.rs` (or 267's envelope submodule if 267 factored one out — check at claim time): `M201` emission (all-four-`Some` gate, `int(v + 0.5)` rounding), `M204 R` merge into 267's `M204` format arms, `M205 J` via `set_junction_deviation`.
  - `crates/slicer-gcode/src/estimator.rs`: `EstimatorLimits::{min_extruding_rate, min_travel_rate}` fields + `from_config` arms + clamp in the feedrate-resolution path (`max(programmed, min)` per class when min > 0).
  - Tests: `gcode_emit_tdd.rs` (+6 tests: m201, m204r incl. legacy fallback, JD emit + omit, flavor gate, RRF factor, default identity), `estimator.rs` (+2: extruding clamp, travel clamp), `resolved_config_defaults_tdd.rs` (+1: eight-field defaults + map surface), `tests/integration/config_bounds_enforcement_tdd.rs` (+1: negative rejection, already aggregated).
  - `docs/DEVIATION_LOG.md`: one row DEV-173 (scalar-vs-stride-2, stealth unmodeled, min-rate estimator-only, JD-max-not-enforced).
  - Generated reference regen (`docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs` or its current name — verify at close-out).
- Rejected alternatives and reasons:
  - Manifest-declared module keys: rejected — no module consumes machine limits; declaration would be rule-1 violation (declared-with-gap).
  - Adopting canonical numeric defaults as `Some(...)`: rejected — the ten siblings are `None`-default and 267's envelope is `Some`-gated; concrete defaults would newly emit envelope lines on every default slice (default-path break).
  - Modelling the stealth variant (`silent_mode`): rejected — ticket 117 owns the per-variant model; a scalar `silent_mode` bool driving nothing would be declaration-only.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` - role: eight field declarations + map arms + bounds; expected change: +8 `cli_opt` arms, +8 map arms, bounds rows.
- `crates/slicer-gcode/src/estimator.rs` - role: min-rate fields + clamp; expected change: +2 struct fields, +2 `from_config` arms, clamp at feedrate resolution.
- `crates/slicer-gcode/src/emit.rs` (+ 267's envelope site if factored out) - role: three envelope components; expected change: M201 gate + M204 R merge + M205 J call in canonical order.
- `crates/slicer-gcode/tests/gcode_emit_tdd.rs`, `crates/slicer-gcode/tests/estimator.rs`, `crates/slicer-ir/tests/resolved_config_defaults_tdd.rs` - role: AC proofs; expected change: +9 tests total (test files are a fourth surface only because ACs demand them; each is a small append).

## Read-Only Context

- `crates/slicer-gcode/src/flavor.rs` - lines 74–120 only - purpose: `set_acceleration` / `set_travel_acceleration` / `set_jerk_xy` / `set_junction_deviation` signatures to reuse.
- `crates/slicer-gcode/src/estimator.rs` - lines 27–100 only - purpose: `EstimatorLimits` struct + `Default` + `from_config` arm shape to extend.
- `docs/spec_packets/267-printer-machine-power-recovery-emitter/packet.spec.md` - full (short file) - purpose: envelope seam contract + P47 exclusion list (the dependency surface).
- `docs/specs/orca-feature-gap/issues/117-silent-mode-per-variant-machine-limit-model.md` - full (small file) - purpose: variant-model boundary this packet must not cross.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/core-modules/machine-gcode-emit/*` - out of scope (no manifest/placeholder work); delegate symbol lookups only
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - never touch (Authoring rule 2)
- Packets 267/278/279/280 internals beyond `packet.spec.md` Goal/Scope - do not open `design.md` or `implementation-plan.md` of other packets

## Expected Sub-Agent Dispatches

- Question: enumerate every `ResolvedConfig {` struct-literal site that must compile after adding 8 fields; scope: `crates/ modules/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 blast radius.
- Question: confirm 267's envelope builder location + function name + gate shape on disk today; scope: `crates/slicer-gcode/src/`; return: `FACT` (≤5 lines); purpose: Step 2 precondition (absent → `[BLOCK]`).
- Question: confirm the bounds-seam entry point ticket-113 left for non-speed host keys; scope: `crates/slicer-scheduler/src/ crates/slicer-ir/src/`; return: `LOCATIONS` (≤10 entries); purpose: Step 1 bounds rows.
- Question: re-derive the next free `DEV-###` at implementation time; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (one number); purpose: Step 4 (assumed DEV-173, must re-verify — ledger facts rot).

## Data and Contract Notes

- IR/manifest contracts: no IR change, no manifest change, no schema-version bump. `to_config_map` gains 8 arms following the sibling `("key", self.field)` shape; `None` omits (verify the sibling omit-vs-null spelling at implementation time and match it exactly).
- WIT boundary: untouched — guests never see these keys (host-only emission/estimation).
- Determinism/scheduler constraints: envelope lines are deterministic in field values + flavor; estimator clamp is a pure function of limits + programmed feedrate. No claim, no ordering, no scheduler-rule interaction.

## Locked Assumptions and Invariants

- With all eight fields `None`, output is byte-identical to pre-packet (AC-7 pins it; any future default adoption must revisit this invariant explicitly).
- `M201` requires all four accel fields; partial configuration emits no `M201` (canonical has no partial form — `MAX_LIMIT` reads all four unconditionally).
- `M205 J` emits only when JD is `Some(v)` with `v > 0` (canonical `set_junction_deviation` gate); `Some(0.0)` and `None` both omit.
- Scalar ingest takes vector index 0 (first filament wins); a scalar spelling behaves identically (same extractor, ticket-140 shape).

## Risks and Tradeoffs

- 267-drift: if 267's implementation deviates from its spec (different builder location, different gate), Step 2 must adapt the extension point without re-specifying 267 — one bounded reconciliation, not a redesign. If 267 is absent, `[BLOCK]`, no fallback.
- Over-ceiling perception: 18 scalars vs the B ≤ 12 ceiling — counted as 9 family entries per the 05 grouping (`x/y/z/e` rows are one entry each), same counting the queue used to size P47. Stated here so review does not re-litigate.
- Estimator-clamp observability: min-rate clamps change only time estimates, never emitted feedrates — AC-4 asserts on `PrintEstimate`, not on G-code text; a reviewer grepping G-code for the clamp will find nothing by design.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 envelope + estimator clamp + 8 tests)
- Highest-risk dispatch and required return format: 267-builder location check — `FACT`, ≤5 lines (a wrong location assumption breaks every Step 2 edit).

## Open Questions

- None. `[BLOCK]` activates only if packet 267's envelope builder is absent from the tree at implementation time (then Step 2 stops and the packet stays `draft`).
