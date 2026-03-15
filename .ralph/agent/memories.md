# Memories

## Patterns

### mem-1773548099-f554
> All IR struct tests use bincode for serde round-trip verification. Tests check struct construction, schema_version presence, and serialization/deserialization.
<!-- tags: testing, ir, serde | created: 2026-03-15 -->

### mem-1773548096-1970
> Coordinate system for Point2: 1 scaled integer unit = 100 nm = 10^-4 mm. Use Point2::from_mm() and units_to_mm() for conversion. Never use raw literals.
<!-- tags: coordinates, ir | created: 2026-03-15 -->

## Decisions

## Fixes

### mem-1773548102-d29a
> Workspace Cargo.toml needs only slicer-ir member during development if other crates don't exist yet. Use cargo test -p slicer-ir to test in isolation.
<!-- tags: workspace, cargo, testing | created: 2026-03-15 -->

## Context
