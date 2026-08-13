# Task Map: 225-dragon-curve-feasibility-gate

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-336` | `Step 1` | `docs/specs/community-modules-dragon-curve-plan.md` (Grounding facts) | `Cargo.toml`, `crates/**/Cargo.toml`, `modules/**/Cargo.toml`, `crates/slicer-wasm-host/src/{host,dispatch,instance}.rs`, `crates/slicer-runtime/src/run.rs` | none | `S` | Discovery inventory bounds every later step |
| `TASK-336` | `Step 2` | `docs/specs/community-modules-dragon-curve-plan.md` (Grounding facts) | `Cargo.toml` | none | `S` | Root wasmtime 47.0.3 + wit-bindgen 0.60.0 pin |
| `TASK-336` | `Step 3` | Step 1 inventory | `crates/**/Cargo.toml`, `modules/**/Cargo.toml` (stale pins) | none | `S` | No stale 43.0.0/0.57.1 pins remain |
| `TASK-336` | `Step 4` | `docs/feasibility-probes/go-wasm.md` §8 | wasmtime-API surface (`slicer-wasm-host`, `slicer-runtime`), wit-bindgen consumers (`slicer-sdk`, `slicer-macros`, guests) | none | `M` | Compile+guest-freshness gate green |
| `TASK-336` | `Step 5` | `docs/feasibility-probes/go-wasm.md` (full brief) | scratchpad probe only (no tree edits) | none | `M` | Go re-run; MoonBit recorded not-re-run |
| `TASK-336` | `Step 6` | `docs/14_submodule_programming_languages.md` §Community-module context | docs only | none | `S` | Gate verdict recorded; unblocks TASK-338 (packet 227) |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M. Aggregate = M.
