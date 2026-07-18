# Task Map: 157-visual-debug-request-bundle-contract

This explicit crosswalk is required by packet preflight for the single backlog task owned by this packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-267` | `Steps 1-4` | `docs/07_implementation_status.md` (TASK-267 row); `docs/specs/visual-pipeline-debug.md`; `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` | `crates/pnp-cli/src/**`; `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` | None; parity does not apply | `M` | Defines the opt-in `pnp_cli visual-debug` request validation, atomic bundle lifecycle, explicit overwrite behavior, and versioned manifest model; excludes stage taps, scheduler dependency closure, rendering, and final G-code parsing. |
