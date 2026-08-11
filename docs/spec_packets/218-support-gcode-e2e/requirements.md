# Requirements: support-gcode-e2e

## Packet Metadata

- Grouped task IDs: `TASK-327`
- Backlog source: `docs/07_implementation_status.md` (approved plan queue row 6)
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

Geometry fixes are not meaningful if support paths disappear or their source markers are absent before final output. The visual-debug G-code renderer already accepts `final_gcode`; this packet closes the missing real-artifact verification path without claiming typed role capture in standalone G-code mode.

## In Scope

- Add one targeted e2e test or equivalent test harness in `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` using `tmp/SupportTest_Normal_Orca.gcode`.
- Exercise both exact request shapes represented by `tmp/visual-debug-gcode.json` (layer `30`) and `tmp/visual-debug-gcode2.json` (layers `31` through `34`) with `final_gcode` and `gcode_line_width_mm: 0.4`.
- Assert `manifest.json` source, tap, layer, and image counts, and assert the input artifact contains both canonical support labels `;TYPE:Support` and `;TYPE:Support interface`.
- Preserve the existing negative validation for `filled_areas` without `gcode_line_width_mm`.
- Treat `TASK-322`, `TASK-323`, `TASK-324`, and `TASK-325` as activation-blocking FORWARD-DEPs: respectively consume their support-planner, fallback/marshalling, raft, and interface geometry outputs before running this end-to-end verification.
- Require the gitignored on-disk fixtures `tmp/SupportTest_Normal_Orca.gcode`, `tmp/visual-debug-gcode.json`, and `tmp/visual-debug-gcode2.json` to be present before running the commands.

## Out of Scope

- Any edit to `crates/slicer-gcode/src/emit.rs`, `crates/slicer-gcode/src/serialize.rs`, `ExtrusionRole`, support geometry, or G-code parser behavior.
- Regenerating Orca G-code or adding an Orca binary dependency.
- Numeric parity beyond the existing artifact's role labels and selected layers.

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct queue/dependency read.
- `docs/specs/support-generation-defect-verified-findings.md` - direct ranges `178-231,246-265`.
- `docs/19_visual_debug.md` - direct ranges `17-46,158-180`.
- `crates/pnp-cli/src/visual_debug.rs` - direct ranges `125-197,686-774`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionEntity.cpp` — confirm the raw `;TYPE:Support` and `;TYPE:Support interface` marker spelling used by the artifact check.

## Acceptance Summary

- Positive: `AC-1` and `AC-2` in `packet.spec.md`.
- Negative: `AC-N1` in `packet.spec.md`.
- Cross-packet impact: activation-blocking FORWARD-DEPs `TASK-322` through `TASK-325` produce the fixed support-planner, fallback/marshalling, raft, and interface outputs this packet verifies; exports no new production symbol.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd` | Run targeted G-code renderer and new support artifact check | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-gcode.json --output target/vd-gcode-e2e --overwrite` | Verify layer 30 manifest | FACT bounded JSON assertion |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-gcode2.json --output target/vd-gcode-e2e-roles --overwrite` | Verify layers 31-34 and marker-bearing artifact | FACT bounded JSON plus bounded `rg` |
| `cargo check --workspace --all-targets` | Compile verification surface | FACT pass/fail |

## Step Completion Expectations

- The test must run after prerequisites `TASK-322` through `TASK-325`, not substitute a synthetic G-code string for the named reproduction.
- `manifest.json` is read before PNGs; role evidence comes from exact input labels and the successful `final_gcode` renderer path.
- No production source changes are permitted in this packet.

## Context Discipline Notes

Keep G-code evidence bounded to the two named requests, manifest entries, and exact `;TYPE:` lines. Do not load full generated bundles or unrelated parser tests.
