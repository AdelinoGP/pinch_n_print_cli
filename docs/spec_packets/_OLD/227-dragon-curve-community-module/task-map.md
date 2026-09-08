# Task Map: 227-dragon-curve-community-module

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. Skip it for a single-task packet unless another explicit mapping need requires it.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
Code-surface paths below are relative to `modules/community-modules/dragon-curve/` and all exist on disk.

| `TASK-338` | `Step 1` | `docs/14_submodule_programming_languages.md` §"Re-measurement under the accommodating host — packet 225a (2026-08-13)" | `modules/community-modules/dragon-curve/` (dir) | — | `S` | Read the recorded verdict and select the language. Answer: Go `NOT_LOADABLE_OR_CORRECT (terminal)`, MoonBit `LOADABLE_AND_CORRECT` → **MoonBit**. |
| `TASK-338` | `Step 2` | `docs/specs/community-modules-dragon-curve-infill.md` §1/§5 | `dragon-curve.toml` | — | `S` | Manifest + claims + compatibility + six config keys (`color_map` is `float-list`). |
| `TASK-338` | `Step 3` | `docs/specs/community-modules-dragon-curve-infill.md` §4 | `src/dragon/{dragon.mbt,dragon_test.mbt,moon.pkg.json}` | — | `M` | Pure tiling + colour-map wrap, TDD. Import-free package so it tests without bindings. Colour keys on `Seg.tile`, not the segment ordinal. |
| `TASK-338` | `Step 4` | `docs/specs/community-modules-dragon-curve-infill.md` §5 | `src/glue/{main.mbt.in,moon.pkg.json.in}` + `src/dragon/dragon.mbt` | — | `M` | WIT glue `run` + hand-rolled `holds_sparse_fill` / `pick_tiling_depth` (no `#[slicer_module]`, no `slicer-sdk`) + unconditional `tool_index`. |
| `TASK-338` | `Step 5` | `docs/specs/community-modules-dragon-curve-infill.md` §1 | `wit/layer-infill.wit` + `wit/deps/{common,config,ir-types,types}/*.wit` + `moon.mod.json` + `Makefile` + `README.md` + `.gitignore` + `dragon-curve.wasm` | — | `M` | Frozen WIT closure + bindgen/`moon build`/`wasm-tools embed --encoding utf16`/`component new` + committed wasm + banner. |

**Language corrected 2026-08-14.** Rows 3-5 previously named `src/lib.rs`, `Cargo.toml`, and four `tests/dragon_*_tdd.rs` files, from the packet's original Go-vs-Rust branch. Packet 225a superseded that branch (see the Step 1 row); **none of those files was ever created.** Test runner is `moon test --target wasm -p slicer/layer-infill/src/dragon`, not `cargo test`.

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
