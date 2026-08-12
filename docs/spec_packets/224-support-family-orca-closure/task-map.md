# Task Map: support-family-orca-closure

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-335` | Steps 1-3 | `docs/specs/support-families-anchored-entities-plan.md` §§Visual And Differential Gates, Supersession | Closure integration tests mounted by `tests/integration/main.rs`; `[[test]] name = "integration" path = "tests/integration/main.rs"`; fixture/evidence requests; visual-debug inspection | `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; `SupportMaterial.cpp`; `SupportCommon.cpp` | M | Establishes authoritative behavioral closure and dispositions for superseded work. All closure commands use `cargo test -p slicer-runtime --test integration support_family_closure ...`. |
