# Implementation Plan: 04_custom-payload-widening

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (TASK-149, TASK-150).
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Confirm WIT type representation (resolved)

- Task IDs: `TASK-149`
- Objective: wasmtime confirmed to support `list<tuple<string, paint-value>>` for `wall-feature-flag.custom`. No proof-of-concept needed — the representation is valid and Step 2 can proceed directly.
- Precondition: None (user confirmed wasmtime supports the representation)
- Postcondition: Step 2 is unblocked; no files changed
- Files expected to change: None
- Authoritative docs: User verification (wasmtime supports `list<tuple<string, paint-value>>`)
- OrcaSlicer refs: None
- Verification: N/A
- Exit condition: Confirmed; Step 2 unblocked

---

### Step 2: Update canonical WIT disk files with widened types

- Task IDs: `TASK-149`
- Objective: Apply the three WIT type changes to the canonical disk source:
  - `wit/deps/types.wit`: `enum extrusion-role { ..., custom }` → `variant extrusion-role { ..., custom(string) }`
  - `wit/deps/ir-types.wit`: `enum paint-semantic { ..., custom }` → `variant paint-semantic { ..., custom(string) }`
  - `wit/deps/ir-types.wit`: `record wall-feature-flag` → add `custom: list<tuple<string, paint-value>>` field (wasmtime supports tuple in list)
- Precondition: Step 1 confirmed WIT representation compiles
- Postcondition: `wit/deps/types.wit` and `wit/deps/ir-types.wit` on disk carry the widened types; all four `include_str!` consumers (macro and host) pick up the changes on next build
- Files expected to change:
  - `wit/deps/types.wit`
  - `wit/deps/ir-types.wit`
- Authoritative docs: `docs/03_wit_and_manifest.md` (`deps/types.wit`, `deps/ir-types.wit` sections)
- OrcaSlicer refs: None
- Verification: `grep "variant extrusion-role" wit/deps/types.wit` shows `custom(string)`; `grep "variant paint-semantic" wit/deps/ir-types.wit` shows `custom(string)`; `grep "custom:" wit/deps/ir-types.wit` shows the new field in `wall-feature-flag`
- Exit condition: All three WIT type changes present in canonical disk files

---

### Step 3: Update macro converters for widened types

- Task IDs: `TASK-150`
- Objective: Update the converter functions in `crates/slicer-macros/src/lib.rs` to handle the widened WIT types:
  - `__slicer_ir_role_to_wit` (~line 1774): update `Custom` match arm from arity-0 to arity-1, carrying the string
  - `__slicer_wit_semantic_to_ir` (~line 1619): update `Custom` match arm, carrying the string
  - `__slicer_ir_feature_to_wit` (~line 1822): add encoding for `custom` map field as `Vec<(String, PaintValue)>` sorted by key
  - `__slicer_wit_feature_to_ir` (~line 1589): add decoding for `custom` map field from wasmtime tuple `Vec<(String, PaintValue)>`
  - `ir_to_wit_paint_semantic` (~line 1406): add `Custom(s)` arm that routes through the WIT variant
- Precondition: Step 2 complete; disk WIT files updated
- Postcondition: Macro converters correctly handle the three widened types; `cargo build --package slicer-macros` succeeds
- Files expected to change:
  - `crates/slicer-macros/src/lib.rs`
- Authoritative docs: `crates/slicer-macros/src/lib.rs` (converter locations from DEV-016)
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-macros && cargo test --package slicer-macros --lib -- --nocapture` (run macro unit tests if any exist)
- Exit condition: All converter functions updated; macro crate builds without errors

---

### Step 4: Update host converters for widened types

- Task IDs: `TASK-150`
- Objective: Update host-side converters in `crates/slicer-host/src/wit_host.rs` to decode the widened WIT types back to IR:
  - Decode WIT `custom(string)` → IR `ExtrusionRole::Custom(string)`
  - Decode WIT `custom(string)` → IR `PaintSemantic::Custom(string)`
  - Decode WIT `wall-feature-flag.custom: list<tuple<string, paint-value>>` → IR `HashMap::from_iter(entries)` (wasmtime generates tuples as Rust `(K, V)` pairs)
- Precondition: Step 3 complete; macro converters updated
- Postcondition: Host converters correctly round-trip the three custom types; `cargo build --package slicer-host` succeeds
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
- Authoritative docs: `crates/slicer-host/src/wit_host.rs`
- OrcaSlicer refs: None
- Verification: `cargo build --package slicer-host` — all 5 known `String::new()` synthesis sites for custom variants updated, and `convert_wall_feature_flag` updated to handle the new `custom` field
- Exit condition: Host converters updated; host crate builds without errors

---

### Step 5: Add round-trip WIT regression tests

- Task IDs: `TASK-150`
- Objective: Add three round-trip test cases proving custom payloads survive the WIT boundary:
  - `extrusion_role_custom_payload_roundtrip`: create `ExtrusionRole::Custom("test-role@1")`, convert IR→WIT→IR, assert payload = "test-role@1"
  - `paint_semantic_custom_payload_roundtrip`: create `PaintSemantic::Custom("com.example/texture@1")`, round-trip, assert payload preserved
  - `wall_feature_flags_custom_payload_roundtrip`: create `WallFeatureFlags { custom: HashMap::from_iter([("key", PaintValue::Scalar(0.5))]) }`, round-trip, assert map has exactly one entry with key="key" and value=Scalar(0.5)
- `wall_feature_flags_custom_multiple_entries_roundtrip`: create `WallFeatureFlags { custom: HashMap::from_iter([("a", PaintValue::Scalar(0.1)), ("b", PaintValue::Flag(true)), ("c", PaintValue::ToolIndex(2))]) }`, round-trip, assert all 3 entries survive with correct values
- `paint_semantic_custom_empty_string_roundtrip`: create `PaintSemantic::Custom("")`, round-trip, assert payload is empty string (not `None` or dropped)
- Precondition: Steps 3 and 4 complete; both macro and host converters updated
- Postcondition: Three new test cases in `macro_all_worlds_roundtrip_tdd.rs` (or new dedicated file `custom_payload_roundtrip_tdd.rs`) that pass
- Files expected to change:
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (add new test cases) OR
  - `crates/slicer-host/tests/custom_payload_roundtrip_tdd.rs` (new file)
- Authoritative docs: `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (existing round-trip test structure)
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- --nocapture` (all three new cases pass)
- Exit condition: All three round-trip tests assert payload survival and pass

---

### Step 6: Update drift detection test (Packet A test)

- Task IDs: `TASK-150` (the drift detection test from Packet A must reflect the new WIT types)
- Objective: Ensure `wit_drift_detection_tdd.rs` (from Packet A step 7) still passes after the WIT type changes. If the test compares WIT type definitions, update the expected values to match the new widened types.
- Precondition: Steps 2-5 complete; Packet A step 7 implemented
- Postcondition: Drift detection test passes with the new widened WIT types
- Files expected to change:
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs` (may need expected-value updates)
- Authoritative docs: Packet A `implementation-plan.md` step 7
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
- Exit condition: Drift detection test passes after WIT type changes

---

### Step 7: Verify workspace build and clippy

- Task IDs: `TASK-149`, `TASK-150`
- Objective: Run full workspace build and clippy to confirm no regressions from the type changes and converter updates.
- Precondition: Steps 1-6 complete
- Postcondition: `cargo build --workspace` succeeds; `cargo clippy --workspace -- -D warnings` passes with zero warnings
- Files expected to change: None (verification only)
- Authoritative docs: None
- OrcaSlicer refs: None
- Verification: `cargo build --workspace && cargo clippy --workspace -- -D warnings`
- Exit condition: Full workspace build and clippy pass with zero warnings

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- `cargo build --workspace` passes.
- `cargo clippy --workspace -- -D warnings` passes with zero warnings.
- `cargo test --package slicer-host --test macro_all_worlds_roundtrip_tdd -- --nocapture` passes with all three custom payload test cases.
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture` passes.
- All acceptance criteria from `packet.spec.md` are verified by the pipe-suffixed commands.
- `docs/07_implementation_status.md` updated: TASK-149, TASK-150 marked complete.
- `packet.spec.md` status updated to `implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm full workspace build and clippy are green.
- Confirm all three custom payload round-trip tests assert non-empty (or expected empty string) payload and pass.
- Confirm drift detection test passes with the widened WIT types.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
