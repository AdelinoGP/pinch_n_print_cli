# Memories

## Patterns

### mem-1773557828-c599
> TASK-012 coding green updated triangle mesh slicing to track endpoint topology (vertex vs edge), dedupe vertex-on-plane hits, and emit polygons only for topologically closed chains; commit 3868e87 makes the loop-chaining red tests pass.
<!-- tags: slicer-core, task-012, geometry | created: 2026-03-15 -->

### mem-1773557435-28a1
> TASK-012 QA red tests prove current slicer-core loop chaining mishandles three cases: unordered cube contours keep a duplicate closing point, open strip slices are emitted as closed polygons, and vertex-touching slices fail to form the expected closed triangle.
<!-- tags: slicer-core, testing, task-012 | created: 2026-03-15 -->

### mem-1773555812-e978
> clipper2-rust API: uses Point64 struct {x: i64, y: i64} for coordinates. Functions like union_64 take &Vec<Vec<Point64>>. inflate_paths_64 takes JoinType and EndType from clipper2_rust root, not ::core.
<!-- tags: clipper2, rust, ffi | created: 2026-03-15 -->

### mem-1773551740-3f68
> TASK-006 scaffolded crates/slicer-sdk with prelude IR re-exports, coords helpers (SCALING_FACTOR/mm_to_units/units_to_mm), and placeholder host service wrappers validated by smoke tests.
<!-- tags: sdk, phase-a, testing | created: 2026-03-15 -->

### mem-1773551410-3537
> TASK-005 scaffolded crates/slicer-test with minimal stable APIs: MockHost call/log assertions, ConfigViewBuilder, SliceRegionViewBuilder, InfillOutputCapture, and assert_paths_planar validated by smoke tests
<!-- tags: testing, phase-a, sdk | created: 2026-03-15 -->

### mem-1773550924-a3e7
> TASK-004 uses a minimal proc-macro skeleton: slicer_module/module_test attribute macros are pass-through placeholders, with smoke usage test at crates/slicer-macros/tests/smoke.rs.
<!-- tags: macros, testing, phase-a | created: 2026-03-15 -->

### mem-1773548099-f554
> All IR struct tests use bincode for serde round-trip verification. Tests check struct construction, schema_version presence, and serialization/deserialization.
<!-- tags: testing, ir, serde | created: 2026-03-15 -->

### mem-1773548096-1970
> Coordinate system for Point2: 1 scaled integer unit = 100 nm = 10^-4 mm. Use Point2::from_mm() and units_to_mm() for conversion. Never use raw literals.
<!-- tags: coordinates, ir | created: 2026-03-15 -->

## Decisions

## Fixes

### mem-1773558506-2e6b
> failure: cmd=/home/admin/.config/nvm/versions/node/v24.14.0/lib/node_modules/@ralph-orchestrator/ralph-cli/node_modules/.bin_real/ralph tools task ensure TASK-013..., exit=0-with-shell-errors, error=zsh evaluated backticks inside task descriptions causing command substitution and corrupted task text, next=reissue task ensure descriptions without backticks or shell-interpreted markdown literals
<!-- tags: tooling, tasks, error-handling | created: 2026-03-15 -->

### mem-1773558194-c741
> Telegram bot onboarding can be completed non-interactively with 'ralph bot onboard --token <token> --chat-id <id>'; the detected chat id is persisted in .ralph/telegram-state.json and unblocks 'ralph tools interact progress'.
<!-- tags: tooling, robot, error-handling | created: 2026-03-15 -->

### mem-1773557577-937a
> failure: cmd=/home/admin/.config/nvm/versions/node/v24.14.0/lib/node_modules/@ralph-orchestrator/ralph-cli/node_modules/.bin_real/ralph tools interact progress 'Starting TASK-012 coding green: implementing loop chaining against the new red tests and Orca references.', exit=1, error=No chat_id found. Run 'ralph bot onboard' to detect it, next=skip Telegram progress updates until bot onboarding is completed
<!-- tags: tooling, robot, error-handling | created: 2026-03-15 -->

### mem-1773557494-6a4f
> Task dependency edges in ralph tools task must use task IDs, not stable keys; using a key in --blocked-by leaves downstream tasks open but never ready.
<!-- tags: tooling, tasks, error-handling | created: 2026-03-15 -->

### mem-1773548102-d29a
> Workspace Cargo.toml needs only slicer-ir member during development if other crates don't exist yet. Use cargo test -p slicer-ir to test in isolation.
<!-- tags: workspace, cargo, testing | created: 2026-03-15 -->

## Context

### mem-1773557896-e363
> TASK-012 is complete once commit 3868e87 is verified by cargo test -p slicer-core and cargo test -p slicer-core --test triangle_mesh_slicer_tdd, then docs/07_implementation_status.md can mark the loop chaining item done.
<!-- tags: slicer-core, task-012, docs | created: 2026-03-15 -->
