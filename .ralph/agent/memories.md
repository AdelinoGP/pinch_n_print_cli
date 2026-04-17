# Memories

## Patterns

### mem-1773548096-1970
> Coordinate system for Point2: 1 scaled integer unit = 100 nm = 10^-4 mm. Use Point2::from_mm() and units_to_mm() for conversion. Never use raw literals.
<!-- tags: coordinates, ir | created: 2026-03-15 -->

### mem-1773555812-e978
> clipper2-rust API: uses Point64 struct {x: i64, y: i64} for coordinates. Functions like union_64 take &Vec<Vec<Point64>>. inflate_paths_64 takes JoinType and EndType from clipper2_rust root, not ::core.
<!-- tags: clipper2, rust, ffi | created: 2026-03-15 -->

### mem-1773548099-f554
> All IR struct tests use bincode for serde round-trip verification. Tests check struct construction, schema_version presence, and serialization/deserialization.
<!-- tags: testing, ir, serde | created: 2026-03-15 -->

## Fixes

### mem-1773605720-f78c
> failure: cmd=ralph tools task ensure with chained creation using inline JSON parsing, exit=1, error=ralph tools task list --format json returned non-JSON/empty output in command substitution so --blocked-by received no value, next=create dependent runtime tasks with explicit task ids from prior command output or separate quiet/list calls
<!-- tags: tooling, tasks, error-handling | created: 2026-03-15 -->

### mem-1773558506-2e6b
> failure: ralph tools task ensure descriptions with backticks, exit=0-with-shell-errors, error=zsh evaluated backticks inside task descriptions causing command substitution and corrupted task text, next=reissue task ensure descriptions without backticks or shell-interpreted markdown literals
<!-- tags: tooling, tasks, error-handling | created: 2026-03-15 -->

### mem-1773558194-c741
> Telegram bot onboarding can be completed non-interactively with 'ralph bot onboard --token <token> --chat-id <id>'; the detected chat id is persisted in .ralph/telegram-state.json and unblocks 'ralph tools interact progress'.
<!-- tags: tooling, robot | created: 2026-03-15 -->

### mem-1773557494-6a4f
> Task dependency edges in ralph tools task must use task IDs, not stable keys; using a key in --blocked-by leaves downstream tasks open but never ready.
<!-- tags: tooling, tasks, error-handling | created: 2026-03-15 -->

## References

### mem-1773561044-01a7
> Manifest ingestion has no direct OrcaSlicer upstream artifacts to reference; OrcaSlicerDocumented only exposes an unrelated Windows app manifest at src/dev-utils/platform/msw/OrcaSlicer.manifest.in. Planner briefs for manifest/scheduler work should rely on docs/01, docs/03, and docs/04 instead.
<!-- tags: slicer-host, manifest, scheduler | created: 2026-03-15 -->

### mem-1773560253-92f6
> OrcaSlicer references for contour-plus-hole containment and paint-region geometry: OrcaSlicerDocumented/src/libslic3r/ExPolygon.cpp lines 182-205 (containment semantics), OrcaSlicerDocumented/src/libslic3r/AABBTreeIndirect.hpp lines 992-1019 (candidate filtering), OrcaSlicerDocumented/tests/libslic3r/test_polygon.cpp and test_geometry.cpp (polygon containment coverage).
<!-- tags: slicer-core, geometry, paint, orca-reference | created: 2026-03-15 -->

### mem-1773559386-ad53
> OrcaSlicer references for AABB tree / mesh-query work: OrcaSlicerDocumented/tests/libslic3r/test_aabbindirect.cpp, tests/libslic3r/test_indexed_triangle_set.cpp, src/libslic3r/AABBMesh.cpp, and src/libslic3r/AABBTreeIndirect.hpp.
<!-- tags: slicer-core, geometry, orca-reference | created: 2026-03-15 -->
## Context

### mem-1776412341-b749
> Packet 01_manifest-ir-access-and-config-schema: TASK-121 (ir-access, all 17 modules) and TASK-122 (config.schema) complete. TDD test core_module_ir_access_contract_tdd.rs passes all 3 tests. docs/07_implementation_status.md updated.
<!-- tags: manifest, ir-access, config-schema, TASK-121, TASK-122 | created: 2026-04-17 -->
