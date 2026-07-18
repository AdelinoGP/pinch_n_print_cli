# Visual Pipeline Debug Packet Plan

This approved plan implements the proposed Visual Pipeline Debug design in five
dependency-ordered packets. It preserves ordinary `pnp_cli slice` behavior and
keeps typed intermediate rendering separate from final G-code rendering.

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | visual-debug-request-bundle-contract | Define the `pnp_cli visual-debug` request, validation, output-bundle lifecycle, overwrite policy, and manifest model without taps or rendering. | TASK-267 | ADR-0039 | generated | packet 157 (archived) |
| 2 | visual-debug-typed-tap-capture | Capture requested typed stage outputs after execution and run only their scheduler dependency closure. | TASK-268 | 157-visual-debug-request-bundle-contract; ADR-0037 | generated | packet 158 (archived) |
| 3 | visual-debug-intermediate-renderer | Render captured typed geometry into deterministic PNGs using the shared viewport and fixed semantic palette. | TASK-269 | 158-visual-debug-typed-tap-capture | generated | packet 159 (archived) |
| 4 | visual-debug-gcode-renderer | Render final PnP-subset G-code into PNGs while preserving unclassified extrusion and reporting unsupported constructs. | TASK-270 | 157-visual-debug-request-bundle-contract | generated | packet 160 (archived) |
| 5 | visual-debug-agent-verification | Add the agent workflow and verify contract coverage, determinism, and zero ordinary-slice overhead. | TASK-271 | 159-visual-debug-intermediate-renderer; 160-visual-debug-gcode-renderer; ADR-0038 | generated | .ralph/specs/161-visual-debug-agent-verification/ |
