# Design: z-envelope-and-wit-boundary-gaps

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/scheduler/` (output commit validation), `crates/slicer-host/src/wit/` (boundary crossing)
- Neighboring tests or fixtures: `crates/slicer-host/tests/` (Z-envelope and WIT boundary tests to be added)
- OrcaSlicer comparison surface: None

## Architecture Constraints

- Z-envelope validation must occur at output-commit (not at call time) so that the full path Z array is available for checking.
- The enforcement must be proactive — catch the violation before it is committed to the IR, not after.
- Deep-copy behavior must be exercised on the live path, not just in unit tests that mock the IR.

## Proposed Changes

### TASK-127 — Z Envelope Enforcement

1. **Identify output-commit points**: For each stage's output builder, find the commit step where `ExtrusionPath3D` paths are finalized.
2. **Add Z-envelope validation**: At each commit, iterate all path points and verify `layer.z <= point.z <= layer.z + effective_layer_height`. If any point violates, abort with `LayerStageError::FatalModule` and emit required diagnostics.
3. **Add test harness**: A test that exercises a module writing out-of-envelope Z and confirms it is caught with correct diagnostics.

### TASK-129a — Postpass GCode Command List Coverage

4. **Audit `dispatch_postpass_gcode_call`**: Find the function and confirm it currently receives real GCode command lists or stub data.
5. **Wire real command content**: Ensure the actual `GCodeCommand` list from `PostPass::GCodeEmit` is passed to the postpass module, not a placeholder.
6. **Add boundary coverage test**: A test that verifies per-command content (move, retract, fan-speed, etc.) crosses the WIT boundary correctly.

### TASK-129b/129c — Deep-Copy Boundary Coverage

7. **Audit layer-world deep-copy paths**: Find where `LayerCollectionIR` or other per-layer IR is deep-copied outside native fallback code.
8. **Add live-path tests**: Tests that exercise the deep-copy on a real slice run and assert that all fields are preserved.
9. **Audit finalization-world deep-copy paths**: Find where `Vec<LayerCollectionIR>` is deep-copied during finalization.
10. **Add live-path tests for finalization**: Tests that exercise the finalization deep-copy on a real slice run.

## Data and Contract Notes

- Z-envelope violation diagnostics must include: module id, stage id, violating IR path, out-of-envelope value, layer.z, effective_layer_height.
- GCode command content must include all fields: x/y/z/e/f for moves, length/speed for retracts, value for fan-speed, etc.
- Deep-copy tests must compare field-by-field, not just check that the copy succeeded.

## Risks and Tradeoffs

- Z-envelope validation at commit time adds O(path_points) overhead per commit. Keep the check cheap — simple bounds comparison, not full IR traversal.
- Deep-copy coverage tests may require real slice data to be meaningful. Use the Benchy fixture if available.

## Open Questions

- Does Z-envelope enforcement already exist anywhere in the codebase, or is it completely missing? Check `crates/slicer-host/src/` for any Z validation.
- Does `dispatch_postpass_gcode_call` currently receive real or stub data? Check `crates/slicer-host/src/postpass/` or similar.
- Are there existing deep-copy tests that cover the layer-world and finalization-world paths?