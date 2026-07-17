# Execution Manifest: 167-config-block-viewer-keys

## Packet metadata
- slug: 167-config-block-viewer-keys
- status: draft (user invoked implement explicitly; will not auto-flip)
- mode: implement
- aggregate context cost: S
- band: standard
- task IDs: TASK-273 (new — mint at closure)
- predecessor: none
- successor: 169-time-estimator-slice-stats

## Acceptance criteria registry
- AC-1: padding purged of speed/accel/jerk/machine_max classes (grep gate)
- AC-2: ≥80 `; key = value` lines in CONFIG_BLOCK with minimal raw_config
- AC-3: exactly one `; printer_model = Generic PNP Printer` line; no `Bambu` substring
- AC-4: "CONFIG_BLOCK viewer-key contract" subsection in docs/02_ir_schemas.md
- AC-N1: fork-supplied `machine_max_acceleration_extruding=20000` and `printer_model=MyFork Printer` appear exactly once with fork values
- AC-N2: pre-existing `gcode_header` tests (no-dup, ordering) still pass

## Step table
| Step | Cost | Files-read (range) | Files-edit | Verification |
|------|------|-------------------|-----------|--------------|
| 1: Write RED integration tests | S | gcode_header_thumbnail_config_blocks_tdd.rs lines 1-120, 420-500 | gcode_header_thumbnail_config_blocks_tdd.rs, closure-log.md | `cargo test ... config_block` (printer_model + no-shadow RED, count GREEN) |
| 2: Rework ORCA_CONFIG_PADDING | S | serialize.rs lines 200-480 | serialize.rs | AC-1 grep PASS; `config_block_meets_orca_minimum_key_gate` GREEN |
| 3: Synthesize printer_model | S | serialize.rs lines 283-400 | serialize.rs | All `config_block` tests GREEN |
| 4: Golden re-bless + invariants | S | target/test-output.log (grep-only) | precision_legacy_20mmbox.gcode (conditional), closure-log.md | `gcode_header` GREEN; golden binary GREEN |
| 5: Doc + crosswalk | S | docs/02_ir_schemas.md lines 1660-1720 | docs/02_ir_schemas.md, docs/07_implementation_status.md (via worker), closure-log.md | AC-4 grep PASS; clippy+check clean |

## Command registry
- Gating: AC-1 grep, AC-4 grep, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`
- Targeted: per-step `cargo test ... config_block`, `cargo test ... gcode_header`
- Packet-level: all 6 AC commands

## Step ledger (rolling)
(empty)

## File ownership
- serialize.rs: Step 2 + Step 3 (sequential, same file)
- test file: Step 1 only
- golden: Step 4 only (conditional)
- docs/02: Step 5 only
- docs/07: Step 5 only (via worker)
- closure-log: Steps 1, 4, 5 (append-only)

## Parallelism
None — steps serialize because Steps 2+3 share serialize.rs and Steps 2+3 feed AC-2's count assertion.

## Doc impact
- Add "CONFIG_BLOCK viewer-key contract" subsection to docs/02_ir_schemas.md
- Append TASK-273 row to docs/07_implementation_status.md
- packet status: leave `draft`; flag for user
