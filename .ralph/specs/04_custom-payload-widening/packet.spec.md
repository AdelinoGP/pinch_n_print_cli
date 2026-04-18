---
status: draft
packet: 04_custom-payload-widening
task_ids:
  - TASK-149
  - TASK-150
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 04_custom-payload-widening

## Goal

Widen the WIT boundary types so `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, and `WallFeatureFlags.custom` can cross the WIT boundary losslessly. Update macro, host, and guest converters to preserve the widened custom payloads. Add round-trip WIT regression tests. Continues DEV-016.

## Scope Boundaries

- In scope:
  - TASK-149: Change WIT `deps/types.wit` `enum extrusion-role { ..., custom }` → `variant extrusion-role { ..., custom(string) }` so `ExtrusionRole::Custom(String)` payload crosses the boundary
  - TASK-149: Change WIT `deps/ir-types.wit` `enum paint-semantic { ..., custom }` → `variant paint-semantic { ..., custom(string) }` so `PaintSemantic::Custom(String)` payload crosses the boundary
  - TASK-149: Change WIT `deps/ir-types.wit` `record wall-feature-flag { ..., custom: list<tuple<string, paint-value>> }` so `WallFeatureFlags.custom: HashMap<String, PaintValue>` crosses the boundary (WIT records cannot use `hashmap`; use `list<tuple<string, paint-value>>` as the closest equivalent)
  - TASK-150: Update macro converters in `crates/slicer-macros/src/lib.rs` at `__slicer_adapt_extrusion_role` (line ~828), `__slicer_adapt_paint_semantic` (line ~1692), and `__slicer_adapt_wall_feature_flags` (line ~1662) to round-trip the widened payloads
  - TASK-150: Update host converters in `crates/slicer-host/src/wit_host.rs` to preserve custom payloads (search for `String::new()` synthesization of custom variants)
  - TASK-150: Add round-trip WIT regression tests proving `ExtrusionRole::Custom("my-role@1")` survives macro→WIT→host round-trip with payload intact
  - TASK-150: Add round-trip WIT regression tests proving `PaintSemantic::Custom("com.example/texture@1")` survives round-trip with payload intact
  - TASK-150: Add round-trip WIT regression tests proving `WallFeatureFlags.custom` with entries `{"key": PaintValue::Scalar(0.5)}` survives round-trip with payload intact
  - TASK-150: Verify that `deps/ir-types.wit` on disk (canonical from Packet A) is updated with these type changes

- Out of scope:
  - TASK-144/145/146 (WIT canonical source — separate packet `03_wit-canonical-source-and-validation`)
  - Changes to `slicer:world-prepass.wit` or any world file other than what's needed to propagate the widened types
  - Changes to IR serialization (serde) — IR already carries `Custom(String)` correctly; only the WIT conversion layer needs updating
  - Changes to GCode emission or downstream consumers that treat custom roles/semantics specially (those are separate concerns)

## Prerequisites and Blockers

- Depends on:
  - `03_wit-canonical-source-and-validation` (TASK-144/145/146) must be at least `draft` so the canonical WIT source location is established before this packet modifies it
  - This packet modifies `wit/deps/types.wit` and `wit/deps/ir-types.wit` — the same files consolidated in Packet A
- Unblocks:
  - Nothing (this is the last Workstream 1 packet)
- Activation blockers:
  - Packet A should be at least draft before activating this packet, since both modify the same canonical WIT files
  - The `list<tuple<string, paint-value>>` representation for `WallFeatureFlags.custom` must be confirmed to work with wasmtime's component model (WIT does not have `tuple` — use `record { key: string, value: paint-value }` instead)

## Acceptance Criteria

- **Given** `wit/deps/types.wit` defines `variant extrusion-role { ..., custom(string) }` (arity-1 custom variant carrying a string), **when** a macro-authored module emits `ExtrusionRole::Custom("bridge-style-a@1")` and the host commits the output, **then** the host-side IR contains `ExtrusionRole::Custom("bridge-style-a@1")` with the string payload preserved (not empty string, not `None`). | `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- extrusion_role_custom_payload --nocapture`

- **Given** `wit/deps/ir-types.wit` defines `variant paint-semantic { ..., custom(string) }`, **when** a module emits `PaintSemantic::Custom("com.example/texture@1")` and the host commits the output, **then** the host-side IR contains `PaintSemantic::Custom("com.example/texture@1")` with the string payload preserved. | `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- paint_semantic_custom_payload --nocapture`

- **Given** `wit/deps/ir-types.wit` defines `wall-feature-flag` with `custom: list<record { key: string, value: paint-value }>` (or equivalent non-hashmap representation), **when** a perimeter module emits `WallFeatureFlags { custom: {"my-key": PaintValue::Scalar(0.42)} }`, **then** the host-side `WallFeatureFlags.custom` contains exactly `{"my-key": PaintValue::Scalar(0.42)}` after round-trip. | `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- wall_feature_flags_custom_payload --nocapture`

- **Given** the macro converter for `ExtrusionRole` is updated, **when** `cargo build --package slicer-macros` succeeds, **then** the macro-generated WIT glue correctly handles `custom(string)` as a variant with payload. | `cargo build --package slicer-macros 2>&1 | grep -i error || echo "build OK"`

- **Given** the host converter for `ExtrusionRole`, `PaintSemantic`, and `WallFeatureFlags` are updated, **when** `cargo build --package slicer-host` succeeds, **then** the host's WIT bindings compile with the widened types. | `cargo build --package slicer-host 2>&1 | grep -i error || echo "build OK"`

- **Given** all three round-trip tests exist and pass, **when** the full workspace build and clippy run, **then** there are zero warnings and zero errors. | `cargo build --workspace && cargo clippy --workspace -- -D warnings`

## Negative Test Cases

- **Given** the WIT type is still `enum extrusion-role { ..., custom }` (arity-0) and the converter sends `Custom("my-string")`, **when** the WIT encoder tries to encode the string payload into an arity-0 variant, **then** the conversion fails at the macro boundary with a `FatalModule` diagnostic or the string is silently dropped. | The round-trip test for `ExtrusionRole::Custom` FAILS with payload mismatch before the type is widened

- **Given** `WallFeatureFlags.custom` is represented as `list<record { key: string, value: paint-value }>` in WIT, **when** a module sends a map with 1000 entries, **then** all 1000 entries survive the round-trip (WIT `list` has no entry limit; test should use a meaningful count like 10 entries to prove the pattern works). | `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- wall_feature_flags_custom_multiple_entries --nocapture`

- **Given** a `PaintSemantic::Custom` with an empty string `""` is emitted, **when** the host receives it, **then** the IR contains `PaintSemantic::Custom("")` (empty string preserved, not dropped). | Verify the converter does not treat empty custom strings as `None` or skip them

## Verification

- `cargo build --package slicer-macros`
- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — `deps/types.wit` (`extrusion-role` enum), `deps/ir-types.wit` (`paint-semantic` enum, `wall-feature-flag` record)
- `docs/02_ir_schemas.md` — `ExtrusionRole::Custom(String)`, `PaintSemantic::Custom(String)`, `WallFeatureFlags.custom: HashMap<String, PaintValue>`
- `crates/slicer-macros/src/lib.rs` — `__slicer_adapt_extrusion_role`, `__slicer_adapt_paint_semantic`, `__slicer_adapt_wall_feature_flags` converter functions
- `crates/slicer-host/src/wit_host.rs` — host-side WIT converters
- DEV-016 deviation log entry for exact loss-point locations

## OrcaSlicer Reference Obligations

None. This is an internal WIT type-widening task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
