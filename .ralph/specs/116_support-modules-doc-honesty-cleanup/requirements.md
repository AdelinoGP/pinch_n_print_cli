# Requirements: support-modules-doc-honesty-cleanup

## Packet Metadata

- Grouped task IDs:
  - `TASK-250` — Doc-comment honesty across the three support modules (B1 from `docs/specs/support-modules-orca-port.md`)
  - `TASK-251` — `support_interface_bottom_layers` dead-state cleanup + `not_implemented` warning (B2)
  - `TASK-252` — `BASE_SPEED` documented as a project-wide normalization convention (B3)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The lead `//!` doc-comments of `tree-support`, `traditional-support`, and `support-planner` overpromise OrcaSlicer parity that the code does not deliver. `tree-support` describes itself as a "tree-style branching support generator" while its fallback is a per-layer 2-D grid MST; `support-planner` claims to be a "Port of OrcaSlicer's `TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes`" while shipping the algorithmic *shape* and not the numerical output. A future contributor reading these comments forms the wrong mental model and may chase phantom regressions.

Separately, `SupportPlanner::on_print_start` reads `support_interface_bottom_layers` from config, stores it on the struct, and never uses it. The dead state is harmless today but actively misleading: a user who sets the config key sees no effect and no signal.

Separately, `BASE_SPEED = 50.0` is hardcoded in four modules as the normalization base for `speed_factor`. The convention is undocumented; future readers cannot explain why 50 without reading the gcode-emit consumer.

This packet closes the three honesty gaps in one slice — all are doc-shaped or one-field-deletion-shaped fixes that share a common code review surface (the four module lib.rs files).

## In Scope

- Rewrite the lead `//!` block of `modules/core-modules/tree-support/src/lib.rs` to the language specified in `docs/specs/support-modules-orca-port.md` §B1.
- Rewrite the lead `//!` block of `modules/core-modules/traditional-support/src/lib.rs` to the language specified in §B1.
- Rewrite the lead `//!` block of `modules/core-modules/support-planner/src/lib.rs` to the language specified in §B1.
- Delete the `support_interface_bottom_layers: i32` field from the `SupportPlanner` struct.
- Delete the `support_interface_bottom_layers = match config.get(...)` parse block in `on_print_start`.
- Replace the parse block with: when `config.get("support_interface_bottom_layers")` returns `Some(v)` with `v != Int(-1)`, emit `log(LogLevel::Warn, "support-planner: support_interface_bottom_layers is not yet implemented; set to -1 (default) to suppress this warning")`. Emitted once per `on_print_start` call, never per-layer.
- Add a `# Speed normalization` doc-comment section to each of the four modules' lead `//!` blocks (`tree-support`, `traditional-support`, `support-planner`, `rectilinear-infill`) documenting the `speed_factor = configured_speed / BASE_SPEED` convention with `BASE_SPEED = 50.0`.
- Add a TOML comment `# Not yet implemented — see docs/specs/support-modules-orca-port.md §B2` next to the `support_interface_bottom_layers` config schema entry in `support-planner.toml`.
- Add the three negative-case unit tests called out in §AC-5, AC-N1, AC-N2.

## Out of Scope

- Real implementation of `support_interface_bottom_layers` (Orca-style bottom interface band).
- Removing the `support_interface_bottom_layers` key from `support-planner.toml [config.schema]`. The user-facing surface is preserved; only the dead Rust state is cleaned up.
- Changing `BASE_SPEED` to a configurable value.
- Replacing `BASE_SPEED` normalization with absolute-speed emission.
- Any change to `tapered_radius`, `inflate_polygon`, raft handling, paint policy, or MST propagation (covered by sibling Block B/C packets).

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` — read §B1, §B2, §B3, §D8, §D9 directly (under 100 lines combined). Carries the exact doc-comment text and the deletion+warn approach.
- `docs/01_system_architecture.md` — > 300 lines; delegate a SUMMARY of `Layer::Support` and `PrePass::SupportGeometry` stage descriptions if the implementer needs context for the new doc-comments. Return format: SUMMARY ≤ 200 words.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-6` from `packet.spec.md`.
  - Doc-comment text greps (`AC-1`, `AC-2`, `AC-3`, `AC-6`) are byte-exact assertions against `rg` matches.
  - `AC-4` enforces full removal of the dead field; the negative grep `! rg -q 'support_interface_bottom_layers'` MUST match zero lines in the Rust source.
  - `AC-5` exercises the new diagnostic path with a `ConfigView` carrying the config key set to a non-default value.
- Negative cases: `AC-N1`, `AC-N2` from `packet.spec.md`.
- Cross-packet impact: `support-planner.toml` retains the config key (user-facing surface unchanged). Downstream sibling packet `118_support-planner-typed-diagnostics` (B4 + B7) will later migrate the `LogLevel::Warn` call to a typed `Diagnostic` channel; this packet ships the string-logged form deliberately so the diagnostic exists before its typed migration.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo build -p tree-support -p traditional-support -p support-planner -p rectilinear-infill` | Compile gate on the four modules. | FACT pass/fail |
| `cargo clippy -p tree-support -p traditional-support -p support-planner -p rectilinear-infill --all-targets -- -D warnings` | Doc-comment + dead-field changes do not regress lint. | FACT pass/fail |
| `cargo test -p support-planner 2>&1 \| tee target/test-output.log` | Includes AC-5, AC-N1, AC-N2 new tests; existing planner unit tests must still pass. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `rg -q 'Per-layer 2-D grid-MST infill with optional SupportPlanIR consumption' modules/core-modules/tree-support/src/lib.rs` | AC-1 substring. | FACT pass/fail |
| `rg -q 'Per-layer rectilinear scan-line filler for Layer::Support' modules/core-modules/traditional-support/src/lib.rs` | AC-2 substring. | FACT pass/fail |
| `rg -q 'Multi-layer support planner inspired by OrcaSlicer' modules/core-modules/support-planner/src/lib.rs` | AC-3 substring. | FACT pass/fail |
| `! rg -q 'support_interface_bottom_layers' modules/core-modules/support-planner/src/lib.rs` | AC-4 field/parse fully removed from Rust. | FACT pass/fail |
| `for m in tree-support traditional-support support-planner rectilinear-infill; do rg -q '# Speed normalization' modules/core-modules/$m/src/lib.rs \|\| { echo "MISSING: $m"; exit 1; }; done` | AC-6 section present in all four. | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM artifacts up to date after src/lib.rs edits (see Architecture Constraints). | FACT pass/fail |

## Step Completion Expectations

- The B1 doc-comment edits and the B3 `# Speed normalization` block share the same lead `//!` region of `support-planner/src/lib.rs`. Step 2 (doc-comment rewrite) and Step 4 (BASE_SPEED note) MUST not introduce conflicting edits to overlapping lines in that file; the implementation plan orders Step 4 immediately after Step 2 specifically to keep their edits sequential, not interleaved.
- No step may regress AC-N1 / AC-N2: a future change to the `on_print_start` parse logic that silently removes the absent-key short-circuit would break the negative cases. The Step 3 exit condition explicitly asserts both negative tests are present and passing.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `docs/specs/support-modules-orca-port.md` — read §B1, §B2, §B3, §D8, §D9 only (line ranges per the spec's section headings). Do NOT read the whole spec.
  - `docs/01_system_architecture.md` — delegate any cross-reference SUMMARY.
- Likely temptation reads (skip these):
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — NOT consulted by this packet; the doc-comments declare what these modules are NOT, no Orca behavior is being ported.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — historical packet 31b test; unaffected by this packet, do not open.
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo test -p support-planner` — sub-agent returns FACT pass/fail; on fail, SNIPPETS ≤ 20 lines with the failing assertion line.
  - `cargo xtask build-guests --check` — sub-agent returns FACT (`up to date` / `STALE: <which>`); never paste the full build log.
