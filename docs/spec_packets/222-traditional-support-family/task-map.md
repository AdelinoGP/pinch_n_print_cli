# Task Map: traditional-support-family

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-333` | Steps 1-5 | `docs/specs/support-families-anchored-entities-plan.md` §§5-10 | `modules/core-modules/traditional-support-planner/`; `modules/core-modules/traditional-support/`; targeted runtime tests | `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:374`, `:2095`, `:2592`, `:1451`, `:2760`, `:2953`, `:3068`/`:3070`, `:3074`, `:3106`, `:3208`, `:480`, `:2735`, `:523`, `:555`, `:1980`, `:487`; `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47` | `S | M` | Traditional planner-renderer family; activation waits for TASK-331 blockers. |
