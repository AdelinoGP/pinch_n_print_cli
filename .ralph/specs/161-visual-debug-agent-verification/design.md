# Design: 161-visual-debug-agent-verification

## Controlling Code Paths

- Primary code path: independent `.claude/skills/visual-debug/SKILL.md` workflow and examples, paired with focused contract/determinism/overhead test targets that invoke the packet-157/159/160 seams.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/visual_debug_agent_contract_tdd.rs` for intermediate taps, `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` for the owning final-renderer seam, `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs`, and `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs`; use the smallest existing deterministic visual-debug fixtures.
- Renderer ownership: packet 159 owns typed rendering and packet 160 owns final-G-code rendering; this packet asserts their published outputs and does not add a second renderer abstraction.

## Architecture Constraints

- Visual debugging is a separate opt-in evidence surface. Ordinary `pnp_cli slice` must not capture, allocate, serialize, render, spawn a visual-debug process, or write visual-debug artifacts.
- The skill is independent of `.claude/skills/debug-pipeline/SKILL.md`: geometry localization may start with visual-debug; timing, DAG, and manifest diagnosis remains with `debug-pipeline`.
- Packet 159 and packet 160 are generated/draft. Their exact exports, test seams, parser/schema version fields, and manifest attachment points are `[FWD]` contracts and must be confirmed before test implementation.
- No WIT, IR, scheduler, manifest ownership, module, guest, or WASM build surface is changed.
- No coordinate conversion or geometry construction is introduced; the tests consume renderer-produced artifacts and documented fixture data only.
- No OrcaSlicer parity or source translation applies; the guide must state the documented Pinch 'n Print subset rather than imply full preview parity.

## Code Change Surface

- Selected approach: add a small independent skill with two guide examples, then add four focused test targets that pin source-field/manifest contracts, byte determinism, request validation, and ordinary-slice opt-out behavior.
- Exact functions, traits, manifests, tests, and fixtures: skill entrypoint and example Markdown; packet-157 visual-debug invocation seam; packet-159 typed capture/image-entry seam; packet-160 final-G-code/image-entry seam; the four named test files; existing deterministic fixtures only.
- Rejected alternatives and reasons: extending `debug-pipeline` violates ADR-0038; testing only PNG existence misses schema drift and warning/order nondeterminism; measuring ordinary-slice overhead by changing `slice` violates the no-overhead contract; implementing missing renderer exports is owned by packets 159/160.

## Files in Scope (read + edit)

- `.claude/skills/visual-debug/SKILL.md` - role: independent agent workflow; expected change: add source-selection, request, inspection, warning, cost, failure, and cross-link guidance.
- `.claude/skills/visual-debug/examples/*.md` - role: guide examples; expected change: add model-backed and standalone-G-code examples with exact commands and negative cases.
- `crates/slicer-runtime/tests/visual_debug_agent_contract_tdd.rs` - role: typed tap/manifest contract tests; expected change: assert every documented tap field and named `[FWD]` seams.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - role: owning final-renderer contract seam; expected change: assert exact final output, layer/tap, and manifest fields plus validation failure.
- `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs` - role: complete-bundle determinism; expected change: compare manifests, PNG bytes, ordering, paths, and warnings for both source modes.
- `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs` - role: ordinary-slice opt-out proof; expected change: assert no visual-debug path/artifact/measurement signal when not requested.

These six edit categories are the only implementation/test surfaces. Example file names may be finalized by the worker within the examples directory without changing scope.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 20-35, 41-59, 99-131, 143-178, 180-221, and 223-235 - exact feature boundaries, tap inventory, agent pairing, determinism, and no-overhead contract.
- `docs/19_visual_debug.md` - lines 9-58 - usage, manifest-first inspection, scale, warnings, and failure behavior.
- `docs/17_agent_debugging.md` - lines 7-19, 21-55, 103-132 - independent debug-pipeline evidence boundary and commands.
- `docs/adr/0038-visual-debug-skill-pairs-with-debug-pipeline.md` - complete 34-line decision.
- `docs/01_system_architecture.md` - lines 65-109, 246-387, 460-497, 621-665 - scheduler/IR/postpass/ownership boundaries.
- `docs/11_operational_governance_and_acceptance_gate.md` - complete governance contract.
- `docs/07_implementation_status.md` - line 243 only - TASK-271 ownership.
- `.ralph/specs/159-visual-debug-intermediate-renderer/**` and `.ralph/specs/160-visual-debug-gcode-renderer/**` - packet contracts and implementation seams only; no broad source inference.

## Out-of-Bounds Files

- Packet 159/160 renderer, parser, capture, PNG, and bundle implementation files - consume their exports; do not implement or alter them.
- Packet 157 request validation, CLI command contract, source selection, lifecycle, overwrite, and manifest ownership - consume only its published seam.
- `crates/slicer-schema/wit/`, `crates/slicer-ir/`, scheduler/executor production code, module manifests, `modules/`, guest artifacts, and WASM build inputs - no contract or implementation changes.
- Ordinary `pnp_cli slice` production path - no instrumentation or behavior change.
- Coordinate-system docs or conversion helpers - no geometry conversion is in scope.
- `OrcaSlicerDocumented/` - no parity scope; do not load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and broad test output - never load or edit.

## Expected Sub-Agent Dispatches

- Question: What exact packet-159 typed capture, tap ordering, image-entry, and test symbols are published? Scope: `.ralph/specs/159-visual-debug-intermediate-renderer/**` and named implementation seam; return: `LOCATIONS` at most 20 entries; purpose: resolve `[FWD-159-1]`.
- Question: What exact packet-160 final-G-code invocation, parser-version, warning, image-entry, and test symbols are published? Scope: `.ralph/specs/160-visual-debug-gcode-renderer/**` and named implementation seam; return: `LOCATIONS` at most 20 entries; purpose: resolve `[FWD-160-1]`.
- Question: What existing deterministic fixtures and narrow ordinary-slice observability can the tests reuse? Scope: named test directories and visual-debug invocation symbols only; return: `LOCATIONS` at most 20 entries; purpose: avoid new geometry, coordinate, or runtime instrumentation.
- Question: Do the focused contract, determinism, overhead tests, all-target check, and clippy pass? Scope: repository commands only; return: `FACT` in 5 lines or fewer; purpose: bounded closure evidence.

## Data and Contract Notes

- IR/manifest contracts: runtime tests assert packet-159 typed source fields; the owning `pnp-cli` final-renderer tests assert packet-160 output, layer/tap, parser, and image-entry fields through the packet-157 manifest shape; this packet owns no schema.
- WIT boundary: unchanged; no module receives visual-debug access, and no WASM artifact is built or modified.
- Determinism/scheduler constraints: compare complete artifact bytes and ordered metadata; visual-debug taps do not create scheduler edges or module-visible access; ordinary slice remains opt-out.
- `[FWD-159-1]` Packet 159 must expose the exact typed capture/test seam and all documented source fields needed for contract assertions.
- `[FWD-160-1]` Packet 160 must expose the exact standalone renderer/test seam, parser-version field, warning representation/order, and final image entries.
- `[FWD-157-1]` Packet 157 must expose validated request input and complete bundle commit/output seams for both source modes.

## Locked Assumptions and Invariants

- The skill never requires timing/DAG diagnosis before visual debugging.
- The guide does not claim OrcaSlicer preview parity and does not introduce coordinate-system guidance beyond directing readers to existing documentation when needed.
- Missing forward exports fail the contract suite loudly; no guessed adapter or compatibility layer is added.
- Determinism includes manifest bytes, PNG bytes, image order, warning order, paths, viewport, legend, and source version metadata.
- No ordinary slice visual-debug artifact or runtime signal exists when the command is not invoked.

## Risks and Tradeoffs

- Draft prerequisite seams may change before implementation; named `[FWD]` diagnostics keep this packet draft and prevent silent coupling.
- A subprocess-based overhead proof can be platform-sensitive; prefer existing test harness observability and compare artifact/path/process evidence, not arbitrary wall-clock thresholds.
- Contract tests spanning runtime and CLI crates may require separate fixtures; keep assertions field-specific and delegated rather than broad.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: exact packet-159/160 seam inventory; `LOCATIONS` at most 20 entries per packet.

## Open Questions

- [FWD] What exact packet-159 export and test names expose every documented typed tap field and image-entry metadata? Confirm before implementation; absence blocks activation.
- [FWD] What exact packet-160 export and test names expose parser version, warnings, and final image entries? Confirm before implementation; absence blocks activation.
- [FWD] What packet-157 lifecycle seam permits clean repeated output comparison without reimplementing request validation or bundle commit? Confirm before implementation; absence blocks activation.
