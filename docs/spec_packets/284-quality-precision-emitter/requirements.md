# Requirements: 284-quality-precision-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000` (queue packet, no backlog slice — wayfinder map ticket 58)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P51 (Quality / Precision, host emitter) is two Tier B keys with zero live occurrences in `crates/`, `modules/`, or `xtask/`. Canonical drives both from the print config into emission: `enable_arc_fitting` selects arc (`G2`/`G3`) versus linear (`G1`) extrusion output (`GCode.cpp` extrusion emission, `GCodeWriter` arc paths), and `resolution` is the generation-time global simplify tolerance (`PerimeterGenerator.cpp` `ex.simplify_p`, `Brim.cpp`, `Fill/Fill.cpp`, `Layer.cpp`, `PrintObjectSlice.cpp`, `Print.cpp`, `TreeSupport`) plus the emit-side arc density in `GCodeWriter.cpp`. This port emits extrusion as `G1` only (the `Move` rendering arm in `crates/slicer-gcode/src/serialize.rs` has no `G2`/`G3` path) and simplifies once at emission through per-role tolerances (`tolerance_for_role` in `crates/slicer-gcode/src/serialize.rs`: `gcode_resolution` / `infill_resolution` / `support_resolution`) that ticket 105 adjudicated as PnP-specific and distinct from the canonical global. The packet builds both decision points where the port already computes them — the emitter tolerance seam and the extrusion-move renderer — closing P51 without a new module, IR field, or WIT change.

## In Scope

- Declare `enable_arc_fitting` (`bool = false`, canonical `coBool` default `0`) and `resolution` (`f32 = 0.01`, canonical `coFloat` default `0.01` min `0` no max — no invented maximum per ticket 113) as scalar-global `ResolvedConfig` fields with `docs/config/host-keys.toml` `[resolved_config]` mirror rows and lock-test arms.
- `resolution` threads into `to_config_map` so the live `0.01` shadows the stale `ORCA_CONFIG_PADDING` `("resolution", "0.012")` row at runtime via `emit_config_kv` dedup (one intended default value change, count unchanged; the padding table itself is untouched — rule 2).
- `enable_arc_fitting` stays host-only (omitted from `to_config_map`, P35 `enable_pressure_advance` / P18 `disable_m73` precedent): the emitter reads the typed field directly; no CONFIG_BLOCK line is added and the canonical `0`/`1` spelling debt rides ticket 132 rather than a spot-fix here (DEV-176(d)).
- Effective tolerance in `tolerance_for_role` (`crates/slicer-gcode/src/serialize.rs`): arc off → `max(per_role_tol, resolution)` (defaults identity: every per-role default exceeds `0.01`); arc on → `min(per_role_tol, 0.2 * resolution)` (canonical `PerimeterGenerator` tightening, AC-5). Travel (`Custom`) stays `0.0`; `order_lock` entities bypass both simplify and arcs in `emit_gcode` (ADR-0063 self-clipping).
- Emitter-side arc fitting in `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`): post-simplify, pre-`Move` coalescing of arc-consistent XY extrusion runs into `G2`/`G3` `Raw` lines (XY-plane only, same-Z, travel-excluded, lock-excluded, E-conserving, F-carrying); arc off (default) keeps today's `G1`-only path byte-identical.
- Emitter-side validation: negative `resolution` rejects the slice with a stable error (canonical min is a GUI hint only — ticket-113 class, DEV-176(c)).
- Regenerate `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`; add DEV-176 row; no `ORCA_CONFIG_PADDING` edit (rule 2, AC-N3).
- One new auto-discovered test file `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs` carrying schema + behaviour + negative pins (no aggregator edit — `slicer-gcode` tests are auto-discovered).

## Out of Scope

- Per-tool vectors for either key — canonical declares both scalar, so there is nothing to vectorise; ticket 125 is not engaged.
- Generation-time per-stage plumbing of `resolution` through perimeter / infill / support / slice modules — canonical's cross-cutting global is deliberately ported once at emission (DEV-176(a)); reproducing its coupling is out of scope per Authoring rule 4's better-seam clause.
- Canonical's spiral-travel density path, wipe-tower arc propagation, and tree-support `config.resolution` fan-out — verified live in canonical, not borrowed; the port's tower/support seams stay untouched (DEV-176(b)).
- The canonical first-change / same-extruder arc conditionals beyond the uniform on/off gate — no extruder model exists in tree to condition on (ticket-39/136 territory); the packet gates uniformly on the bool.
- `ORCA_CONFIG_PADDING` twins, module-manifest declarations, `FeedrateConfig`/`SPEED_KEYS` membership (these are emission-shape additions, not role base speeds), and any change to travel `G0` selection or relative-E accounting.
- Per-tool overrides beyond the automatic `tool_config:<idx>:` composition the `declare_resolved_config!` macro already threads (ticket-126 precedent: overlay arms are generated per field, no allowlist edit).

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (emitter as the modular-pipeline emission seam)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system non-applicability: arc on/off is in-module mode branching per the Q8 trigger test, not cross-module algorithm selection — rule 4 does not fire)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - direct read of P51 rows (tier B × 2, owner `crates/slicer-gcode`)
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - delegated SUMMARY (canonical function-read + invariant-test standard)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: both declarations and bounds (already grounded at authoring)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::apply_print_config` and arc-density paths: `m_resolution` storage and the off-path segment density
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — extrusion emission: `G1`-vs-arc selection and `apply_print_config` scaled store
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic` / `process_arachne`: the `0.2 * resolution` tightening when fitting is on

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6`; AC-3 pins the global-floor behaviour at large `resolution`, AC-4 pins `G2`/`G3` emission with E conservation and travel exclusion, AC-5 pins the `0.2 *` tightening; AC-2 pins default-geometry identity plus the single intended CONFIG_BLOCK value change (`0.012` → `0.01`); AC-6 pins docs.
- Negative: `AC-N1` (negative `resolution` rejected), `AC-N2` (travel + locked paths never arc / never simplify), `AC-N3` (no padding-table edit — honest absence/shadowing).
- Cross-packet impact: geometry byte-identical at defaults (global floors above `0.01`, arc off); CONFIG_BLOCK gains no line and loses none — exactly one value changes. DEV-176 is behaviour-only, not a default mismatch, so the deviation gate shows no new default row.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test quality_precision_arc_resolution_tdd 2>&1 \| tail -5` | schema + behaviour + negatives in one auto-discovered binary | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-gcode --test gcode_emit_per_role_tolerance_tdd 2>&1 \| tail -3` | no-regression on existing per-role tolerance pins | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 \| tail -3` | no-regression on general emission | FACT pass/fail |
| `cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 \| tail -3` | no-regression on golden emission bytes | FACT pass/fail |
| `cargo check --workspace --all-targets 2>&1 \| tail -3` | struct-literal blast radius (new ResolvedConfig fields) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -3` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate (test code FRU/`exhaustive` waiver) | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step order is declaration → tolerance + resolution behaviour (Step 2) → arc + validation (Step 3) → docs/deviation (Step 4); Steps 2–3 must not land before the schema guard proves both keys resolve (otherwise the tolerance/arc reads fallbacks).
- `host-keys.toml`, the lock-test arms, the `to_config_map` resolution arm, and the `gen-config-docs` regen land with the DEV-176 row in Step 4 so the lock test and the deviation gate observe one coherent tree — except the schema guard's own TOML rows, which land in Step 1 with the fields.
- No `run_slice` e2e driver is required: tolerance selection and arc coalescing are pure functions of (paths, two config values) pinnable at unit level (the `gcode_emit_per_role_tolerance_tdd` precedent constructs entities directly).

## Context Discipline Notes

- Tempting large reads to skip: `crates/slicer-ir/src/resolved_config.rs` (macro invocation — read only the precision/resolution window plus the `to_config_map` host-key insert region); `crates/slicer-gcode/src/emit.rs` (read only the simplify + `Move`-construction window); `crates/slicer-gcode/src/serialize.rs` (read only `tolerance_for_role` + the `Move` rendering arm + the padding row); `OrcaSlicerDocumented/` (delegate always).
- `target/*.gcode` echoes of these keys are generated output, not evidence of liveness — never cite them as reads.
