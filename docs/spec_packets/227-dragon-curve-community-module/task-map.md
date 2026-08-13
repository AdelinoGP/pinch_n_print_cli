# Task Map: 227-dragon-curve-community-module

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. Skip it for a single-task packet unless another explicit mapping need requires it.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-338` | `Step 1` | `docs/14_submodule_programming_languages.md` §Community-module context | `modules/community-modules/dragon-curve/` (dir) | — | `S` | Read the 225 verdict; select Go vs Rust-fallback branch. |
| `TASK-338` | `Step 2` | `docs/specs/community-modules-dragon-curve-infill.md` §1/§5 | `modules/community-modules/dragon-curve/dragon-curve.toml` | — | `S` | Manifest + claims + compatibility + six config keys. |
| `TASK-338` | `Step 3` | `docs/specs/community-modules-dragon-curve-infill.md` §4 | `src/lib.rs` + `tests/dragon_{tiling,color_map}_tdd.rs` | — | `M` | Pure tiling + color-map wrap, TDD. |
| `TASK-338` | `Step 4` | `docs/specs/community-modules-dragon-curve-infill.md` §5 | `src/lib.rs` + `tests/dragon_config_override_tdd.rs` + `Cargo.toml` | — | `M` | `#[slicer_module]` wiring + per-region override. |
| `TASK-338` | `Step 5` | `docs/specs/community-modules-dragon-curve-infill.md` §1 | `Makefile` + `README.md` + `dragon-curve.wasm` + `tests/dragon_emission_tdd.rs` | — | `M` | Build script + committed wasm + banner + deferred 226 emission. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
