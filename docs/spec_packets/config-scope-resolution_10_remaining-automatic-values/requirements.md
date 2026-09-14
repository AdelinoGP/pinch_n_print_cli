# Requirements: remaining-automatic-values

## Packet Metadata

- Grouped task IDs: `TASK-571`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The emitter already computes deposited volume per millimetre from each move's width and flow factor plus the layer's height delta, but `resolve_feedrate` only sees role and scalar speed factor. A configured role speed of zero therefore remains zero instead of using the active filament's maximum volumetric throughput. Config-only automatic values and overhang percentages belong to packet 04; this packet closes only the remaining context-dependent emission seam and guards against silently overlooking future negative-default sentinels.

## In Scope

- Add `filament_max_volumetric_speed` as a typed positive per-filament host key carried by `ResolvedConfig`, using entry 0 globally and packet-05-resolved per-tool values at emission.
- Preserve explicit positive role speeds and packet-04-expanded percentages unchanged.
- At the extrusion-move emission site, compute `mm3_per_mm = point.width * height_delta * point.flow_factor`; when the selected role's configured base speed is exactly zero, use `filament_max_volumetric_speed / mm3_per_mm` as the automatic mm/s base, then pass it through ADR-0052's existing factor clamp and G-code mm/min conversion seam.
- Fail closed through `GCodeEmitError::Emit` when the zero-speed branch lacks finite positive maximum volumetric speed or finite positive move width, layer height, flow factor, or derived `mm3_per_mm`.
- Select the active tool's resolved maximum volumetric speed, falling back to the resolved global value only when that tool has no narrower value.
- Add an independently calculated emitter test with literal F values, a two-tool discriminator, an explicit-speed non-regression row, and invalid-input rows.
- Add a registry-derived negative-sentinel census over numeric defaults and lower bounds that fails on every unclassified negative-capable declaration. Current grounding found packet-04's config-only `support_interface_bottom_layers` (negative default/lower bound) and `support_bottom_interface_spacing` (nonnegative default, negative lower bound); no geometry-dependent `-1` declaration currently requires an implementation branch.
- Add the committed visual-debug request and validate its `PostPass::GCodeEmit` bundle.
- Supply that request's `source.config` as a committed companion JSON file parsed by `parse_cli_config_source`, with `outer_wall_speed = 0` and `filament_max_volumetric_speed = 8.0`.
- Document the Phase-C placement and regenerate the config-key reference.

## Out of Scope

- The four overhang speed percentages; packet 04 owns their `outer_wall_speed` base and absolute Phase-B expansion.
- `support_interface_bottom_layers = -1` and `support_bottom_interface_spacing = -1`; packet 04 owns both config-only mirrors.
- Applying a volumetric cap to explicit positive configured speeds; this row owns only the stated `0 = volumetric auto` fallback.
- Inventing support for Orca `-1` keys not declared by the live PnP registry.
- Changes to scope precedence, ingestion, `ConfigView`, WIT, IR schema versions, CLI JSON schemas, or manifest vocabulary.
- Any implementation before packets 04 and 05 land and their exports are reconciled.

The referenced `crates/slicer-config/tests/automatic_value_expansion_tdd.rs` target is itself a **FORWARD-DEP net-new file from draft packet 04**, not a pre-existing test in this packet's baseline; Step 1 must reconcile its exact landed name before Step 2 reads or edits it.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — ranged reads of RC-8, Expansion Phases B/C, queue rows 04/05/10, and cross-cutting tests.
- `docs/02_ir_schemas.md` — ranged reads of `ResolvedConfig`, numeric handling, and hashing/interner invariants.
- `docs/19_visual_debug.md` — ranged reads of request and G-code emit bundle contracts.
- `docs/22_test_quality.md` — ranged read of independent oracles, derived rosters, vacuity, and negative controls.
- `docs/11_operational_governance_and_acceptance_gate.md` — ranged read of compatibility policy; this packet adds a host config field but changes no public serialized IR/WIT/CLI shape.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — verify `GCode::_extrude`'s `filament_max_volumetric_speed / mm3_per_mm` zero-speed fallback, active-extruder selection, and invalid-flow treatment.

## Acceptance Summary

- Positive: `AC-1` through `AC-5` in `packet.spec.md` cover literal volumetric fallback, tool selection, explicit-speed preservation, the derived negative-default census, visual evidence, and docs.
- Negative: `AC-N1` proves invalid maximum/geometry cannot emit a zero or non-finite feedrate.
- Cross-packet impact: packet 04 remains sole owner of Phase-B percentages and config-only negative mirrors; packet 05 supplies fully resolved global/tool values under the normative precedence. This packet exports no API required by a later queue row.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd` | Literal single-/multi-tool success, explicit-speed preservation, and invalid-input rejection | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p slicer-config --all-targets --test automatic_value_expansion_tdd registry_negative_sentinel_census_has_no_unowned_phase_c_candidate -- --exact` | Derived negative-capable declaration ownership census in packet 04's FORWARD-DEP net-new, name-reconciled test target | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo check --workspace --all-targets` | Struct-field and cross-crate compile blast radius | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask check-literals` | Watched struct-literal gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test false-green report | FACT findings/no findings for touched files |
| `cargo xtask build-guests --check` | Required freshness verdict because `slicer-ir` feeds guest WASM | FACT exit 0/1/3 |
| `cargo xtask gen-config-docs --check` | Generated config-reference drift gate | FACT pass/fail |

## Step Completion Expectations

- Reconcile packet 04/05 exports and complete the derived sentinel census before adding a production branch.
- Add the `ResolvedConfig` key and all macro-generated equality/hash/map fallout before wiring the emitter.
- Keep the zero-speed branch at the move site; do not freeze width, height, flow, or tool context during Phase B.

## Context Discipline Notes

`crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-gcode/src/emit.rs`, and the authoritative docs are long; locate symbols first and read bounded ranges only. Delegate Orca inspection and every cargo command with the bounded return formats above.
