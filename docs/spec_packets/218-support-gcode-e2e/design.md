# Design: support-gcode-e2e

## Controlling Code Paths

- Primary code path: `crates/pnp-cli/src/visual_debug.rs:127-197` deserializes `VisualDebugRequest` and G-code source; `crates/pnp-cli/src/visual_debug.rs:686-774` defines manifest/image fields; `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs:48-72,136-194,522-544` provides the working request helper, manifest assertions, and negative validation.
- Neighboring tests/fixtures: `tmp/visual-debug-gcode.json`, `tmp/visual-debug-gcode2.json`, `tmp/SupportTest_Normal_Orca.gcode`, and `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Verification-only: do not edit `crates/slicer-gcode/src/emit.rs:157-158,237-238` or `crates/slicer-gcode/src/serialize.rs:44`; those existing paths already support the roles.
- Standalone G-code mode exposes raw source `;TYPE:` lines and manifest image entries only; assert the exact serialized labels `;TYPE:Support` and `;TYPE:Support interface` plus the rendered `final_gcode` entries, not typed role semantics.
- G-code mode is standalone: its source shape is `VisualDebugSource::Gcode { path, model }`; it does not run the slicer pipeline or use a model config.

## Code Change Surface

- Selected approach: extend the existing `visual_debug_gcode_renderer_tdd.rs` integration test with the real support artifact, invoke `run_visual_debug`, read `manifest.json`, and assert the selected final-G-code entries and canonical support markers.
- Exact functions, traits, manifests, tests, and fixtures: `gcode_request`, `manifest_at`, `run_visual_debug`, `VisualDebugSource::Gcode`, `Manifest::source`, `Manifest::images`, `ImageEntry::tap/source/layer_index`, and the existing `ac_n1_rejects_filled_areas_without_line_width` test.
- Rejected alternatives and reasons: changing emission is forbidden by the approved plan; a synthetic G-code string cannot prove the named support artifact reaches final G-code; a PNG-only assertion cannot prove role labels.

## Files in Scope (read + edit)

- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - role: targeted e2e test; expected change: add real-artifact request/manifest/marker assertions.

## Read-Only Context

- `crates/pnp-cli/src/visual_debug.rs` lines `125-197,686-774` only - request and manifest contract.
- `crates/slicer-gcode/src/emit.rs` lines `195-249` only - existing canonical role labels.
- `crates/slicer-gcode/src/serialize.rs` lines `30-45` only - existing support resolution.
- `docs/19_visual_debug.md` lines `17-46,158-180` only - request and manifest workflow.

## Out-of-Bounds Files

- All production G-code emission, serialization, parser, IR, and role files - read-only.
- `tmp/SupportTest_Normal_Orca.gcode` and request JSON files - input artifacts, never edit.
- `target/`, generated bundles, lockfiles, and `OrcaSlicerDocumented/` - never load directly.

## Expected Sub-Agent Dispatches

- Question: confirm the existing G-code integration test binary can drive `run_visual_debug` with a filesystem artifact; scope: `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`; return: `LOCATIONS`.
- Question: confirm canonical Orca support role labels; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionEntity.cpp`; return: `LOCATIONS`.

## Data and Contract Notes

- IR/manifest contracts: assert only existing manifest keys `source.kind`, `images[*].tap`, `images[*].source`, and `images[*].layer_index`; no schema change.
- WIT boundary: none.
- Determinism/scheduler constraints: G-code mode has no scheduler closure; selected layer order must remain `[30]` or `[31,32,33,34]`.

## Locked Assumptions and Invariants

- The source artifact contains the exact lines `;TYPE:Support` and `;TYPE:Support interface`; these raw markers are observed, not reinterpreted as typed manifest roles.
- Existing role emission is outside this packet and is not modified.
- `tmp/SupportTest_Normal_Orca.gcode` is the authoritative reproduction named by the verified findings.

## Risks and Tradeoffs

- The artifact must exist in the workspace or the test should report a precise missing-fixture failure; silently falling back to a synthetic fixture invalidates the e2e claim.
- G-code-mode manifest captures do not expose typed IR `ordered_entities`; role proof therefore combines exact source labels with successful rendered final-G-code entries.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: existing test-driver check, `LOCATIONS`.

## Open Questions

None.
