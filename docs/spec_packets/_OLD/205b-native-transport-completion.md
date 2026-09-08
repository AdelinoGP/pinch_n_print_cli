---
status: implemented
packet: 205b-native-transport-completion
task_ids:
  - ADR-0056
  - ADR-0057
---

# 205b-native-transport-completion

## Goal

Complete the two native dispatch transports packet 202 left as fatal errors — `Layer::PathOptimization` output commit and postpass gcode-command application — then integrate the two modules that depend on them (`path-optimization-default`, `machine-gcode-emit`) behind per-module cargo features with dual-dispatch parity gates, so that `cargo xtask dist --edition integrated` finally builds (every core module integrated, nothing staged externally).

## Problem Statement

Packet 202 left the `Layer::PathOptimization` output commit and postpass gcode-command application as fatal native transport errors. Packet 205a integrates every other committable module, so the two remaining modules cannot enter the Integrated edition until these transports are complete and proven equivalent to wasm.

## Architecture Constraints

- Native output must be committed into the existing IR types; do not bypass the stage runner or weaken a parity comparator.
- Parity compares structural invariants and measured coordinate tolerance, never floating-point byte equality.
- The two modules remain behind off-by-default per-module features. `integrated_registrations()` and `native_entries()` must be enabled by the same feature set.
- Preserve external-module precedence: an external module selected by the normal binding plan must not acquire an integrated native entry.
- No hardcoded module count may be introduced. Registry tests derive expected coverage from the registered set and existing contract conventions.
- No geometry call sites, WIT schema, dispatch routing, macro emission, CLI surface, `dist/editions.toml`, `docs/07_implementation_status.md`, or `docs/07` content is edited.
- Native module logic remains single-threaded; neither module may add `rayon` or parallel iterator usage.

## Locked Assumptions and Invariants

- `path-optimization-default` declares `Layer::PathOptimization`; its native commit must return `Ok(Some(..))` or `Ok(None)` consistently with the wasm path.
- `machine-gcode-emit` declares `PostPass::GCodePostProcess`; every supported collected command must be applied in order and unsupported commands must fail explicitly.
- Integrated feature names equal module directory names, and passthrough feature bodies delegate to the matching registry feature.
- Existing external override behavior and edition membership remain unchanged.

## Risks and Tradeoffs

- Path optimization may expose output fields not represented by the existing layer converter. Resolve by reusing the closest committed layer representation and add a structural parity assertion; do not silently drop paths.
- Gcode command variants may not all have accumulator equivalents. Fail with the command kind rather than treating emitted commands as success.
- Guest artifacts can become stale after manifest or feature changes; run the freshness gate before parity tests.
