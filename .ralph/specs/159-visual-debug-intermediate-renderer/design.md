# Design: 159-visual-debug-intermediate-renderer

## Controlling Code Paths

- Primary code path: packet-158's renderer-owned typed capture handoff (a `slicer-runtime` capture entry point called from `crates/pnp-cli/src/visual_debug.rs::run_visual_debug`) into a new `slicer-runtime` intermediate renderer (viewport planner, fixed palette, overlay compositor, PNG encoder) whose output `crates/pnp-cli/src/visual_debug.rs` assembles into `ImageEntry`/`Manifest`.
- Grounded fact: `slicer-runtime` cannot import `crates/pnp-cli/src/visual_debug.rs`'s `Manifest`/`ImageEntry` types (dependency direction is `pnp-cli -> slicer-runtime`), so the renderer itself must be a pure function returning runtime-owned PNG bytes/metadata, with `pnp-cli` writing the file and populating `ImageEntry`.
- Neighboring tests/fixtures: `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` (new, follows `visual_debug_request_bundle_tdd.rs`'s convention) and the smallest deterministic typed-capture fixtures exported by packet 158.
- Final G-code comparison: explicitly out of scope; packet 160 owns the independent serialized final-G-code renderer.

## Architecture Constraints

- The renderer consumes typed, post-stage, post-host-hook, renderer-owned values only. It does not create scheduler edges, invoke modules, read Blackboard state, or retain `LayerArena` data.
- The renderer must preserve the documented source distinction: direct `ExPolygon` areas render directly; typed paths use `Point3WithWidth.width` for filled sweeps; diagnostic diagrams use documented trace fields rather than fabricated model geometry.
- Every image in a bundle uses one model-wide XY viewport, documented fixed margin, fixed v1 semantic palette, fixed legend version, and the requested raster scale. Palette values are implementation constants, not request options.
- PNG output is deterministic in pixel traversal, primitive ordering, alpha handling, compression configuration, path naming, and manifest image-entry ordering. A failure cannot be reported as a successful partial bundle.
- `png` is pure Rust; the implementation must record enabled features and license review before closure.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a renderer-owned intermediate pipeline that normalizes packet-158 capture records into render primitives, computes the bundle viewport once, applies fixed semantic colors and legend metadata, composites optional overlays, and writes deterministic PNGs through the pure-Rust encoder.
- Exact functions, traits, manifests, tests, and fixtures: packet 158's exported typed capture is `slicer_runtime::layer_executor::{StageCapture, CapturedIr, CaptureOutput}` (`crates/slicer-runtime/src/layer_executor.rs:591-679`); the image-entry handoff is the existing `ImageEntry.png_path`/`typed_capture` fields and the `run_model_source` call site (`crates/pnp-cli/src/visual_debug.rs:351-454`, capture call at 420-426, `ImageEntry` construction at 437-454); a new `slicer-runtime` visual-debug renderer module (exact submodule path is an implementation-time choice, not a forward contract); `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` (new, no file of this name exists yet); and focused deterministic typed-capture fixtures built directly against `PerimeterIR`/`InfillIR`/`SupportIR`/`LayerCollectionIR` since packet 158's own test file (`crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs`, 601 lines) exposes no reusable typed-IR fixture constructors — it asserts only on serialized `typed_capture` JSON.
- Rejected alternatives and reasons: rendering from serialized G-code is packet 160 and cannot localize intermediate defects; a debug WASM module violates ADR-0037 ownership; per-image viewports prevent stage comparison; inferred bead width loses typed geometry semantics; external image tools violate release-time portability and determinism.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/` - role: intermediate renderer, viewport/palette/overlay composition, and PNG encoding as a pure function of typed capture data; expected change: add the renderer without changing scheduler capture and without depending on `pnp-cli` types.
- `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` - role: focused renderer contract coverage (new file); expected change: add positive, determinism, and failure-path assertions driven through `run_visual_debug`/the CLI request path, matching `visual_debug_request_bundle_tdd.rs`'s convention.
- `crates/pnp-cli/src/visual_debug.rs` identified by `[FWD-158-2]` - role: additive `ImageEntry` handoff; expected change: only the fields required to attach renderer-produced PNG metadata and paths returned by the new `slicer-runtime` renderer call, if packet 158 does not already wire this.

The two concrete runtime/test surfaces and the `crates/pnp-cli/src/visual_debug.rs` seam are the only edit categories; implementation must not broaden them without a new packet.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 112-163 and 180-213 only - bundle image contract, visualization semantics, intermediate ownership, and exact tap source fields.
- `docs/19_visual_debug.md` - lines 18-50 only - request scale, shared viewport, filled-area semantics, warnings, and failure behavior.
- `docs/01_system_architecture.md` - lines 246-387, 460-635, and 638-665 only - typed IR outputs, postpass ownership, data ownership, and arena lifetime.
- `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - complete 44-line decision record - typed tap rationale and synthetic-diagram boundary.
- `docs/11_operational_governance_and_acceptance_gate.md` - complete 179-line governance contract - determinism, recoverability, resource, and coupling obligations.
- `.ralph/specs/158-visual-debug-typed-tap-capture/**` - bounded export/contract lookup only - exact renderer-owned capture and manifest handoff names.

## Out-of-Bounds Files

- `crates/pnp-cli/src/visual_debug.rs`'s command parsing (`validate_request`, `VisualDebugRequest`), source-mode selection, output lifecycle, overwrite behavior, and base manifest semantics - packet 157 owns them; only the additive `ImageEntry`-attachment call site may be consumed/edited.
- Scheduler/executor capture paths and layer-retention logic - packet 158 owns them.
- Final G-code parser/renderer surfaces - packet 160 owns them.
- `crates/slicer-schema/wit/`, module manifests, IR schema definitions, `modules/`, guest artifacts, and WASM build inputs - no contract or guest changes are permitted.
- `.claude/skills/`, `docs/17_agent_debugging.md`, HTML report code, and agent-facing documentation - later packet or existing tooling owns them.
- `target/`, `Cargo.lock`, generated code, and vendored dependencies - never load or edit.
- `OrcaSlicerDocumented/` - no parity scope applies; do not load.

## Expected Sub-Agent Dispatches

- Question: What exact public packet-158 capture records, tap/layer ordering, and renderer handoff symbols are available? Scope: `.ralph/specs/158-visual-debug-typed-tap-capture/**`; return: `LOCATIONS` at most 20 entries; purpose: resolve `[FWD-158-1]` and `[FWD-158-2]` without inventing APIs.
- Question: Which existing `slicer-runtime` module is the narrowest seam for a new pure renderer invocation (typed capture in, PNG bytes/metadata out), given that `ImageEntry`/`Manifest` are `pnp-cli`-owned and unreachable from `slicer-runtime`? Scope: `crates/slicer-runtime/src/**`; return: `LOCATIONS` at most 20 entries; purpose: avoid moving packet-157/158 ownership.
- Question: Which exact typed fields and fixture constructors are available for polygon, `Point3WithWidth`, overlay, and synthetic diagram inputs? Scope: packet-158 export plus targeted `crates/slicer-ir/src/**` and `crates/pnp-cli/tests/**`; return: `SNIPPETS` at most 3 snippets, 30 lines each; purpose: build compile-pinned renderer tests against real types.
- Question: Which `png` crate version/features and license evidence are acceptable in the workspace? Scope: manifests and dependency policy only; return: `FACT` in 5 lines or fewer; purpose: close the pure-Rust encoder decision without browsing lockfiles.

## Data and Contract Notes

- IR/manifest contracts: the renderer consumes packet-158 typed captures and records image path, source mode, tap, layer/Z where applicable, visualization, viewport, legend version, source schema version, and warnings in the existing manifest image-entry seam.
- WIT boundary: unchanged. No capture or render data crosses into a module and no module receives a new access capability.
- Determinism/scheduler constraints: scheduler timing and capture ordering are inherited from packet 158; renderer ordering, viewport, palette, raster scale, PNG bytes, and manifest fields must be stable for identical inputs.
- `[FWD-158-1]` Capture records must preserve all documented source fields needed by this packet, including `Point3WithWidth.width` and overlay fields, in renderer-owned values.
- `[FWD-158-2]` The packet-158 handoff must permit image entries to be appended without duplicating request parsing, bundle lifecycle, or base manifest semantics.
- `[FWD-158-3]` Non-geometry typed captures must expose trace-relevant fields sufficient for a stage-specific synthetic diagram or be rejected as unsupported rather than fabricated.

## Locked Assumptions and Invariants

- Packet 158 is `implemented` (commit `68b10706`) and its capture code is merged; the three FWD handoff statements above are grounded against real types, not the spec text (see Open Questions).
- The v1 palette, legend version, fixed margin, and `1024 x 1024` base raster are implementation-owned constants derived from the documented contract, not user-configurable request fields.
- A successful render has no dangling source borrows and no successful partial image set.
- Ordinary slice execution has no renderer allocation or invocation because this packet only consumes the existing opt-in visual-debug path.

## Risks and Tradeoffs

- Packet 158 may expose a capture shape that cannot carry one documented source field; the correct response is a draft blocker or scope change, not field guessing.
- Rasterization of integer geometry and width sweeps can introduce edge ambiguity; deterministic tie-breaking and pixel-center rules must be tested on fixture boundaries.
- Overlay text can vary by font backend; use a deterministic in-process glyph/label representation or a documented fixed raster strategy, never an OS font dependency.
- PNG compression settings affect byte determinism; pin encoder settings and test repeated byte output.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: packet-158 export/handoff lookup; `LOCATIONS` at most 20 entries.

## Open Questions

All three forward contracts below were resolved by a grounded read-only inventory of packet 158's merged implementation (commit `68b10706`) on 2026-07-15. No open questions remain.

- **[RESOLVED]** Capture payload type: `slicer_runtime::layer_executor::StageCapture` (`crates/slicer-runtime/src/layer_executor.rs:619-628`) — fields `stage_id` (tap identity), `layer_index`, `layer_z`, `ir: CapturedIr`. `CapturedIr` (lines 591-615) is a tagged enum over `PerimeterIR`, `InfillIR`, `SupportIR`, `LayerCollectionIR` (`crates/slicer-ir/src/slice_ir.rs:1943,1983,2008,2140`), each with its own `schema_version: SemVer` exposed via `CapturedIr::schema_version_string()`. Ordering is guaranteed: `CaptureOutput.captures: Vec<StageCapture>` (lines 662-679) is documented as ordered by `(STAGE_ORDER position, layer_index)`. **`warnings` is absent from the capture types** — `ImageEntry.warnings` stays hardcoded `Vec::new()` in `crates/pnp-cli/src/visual_debug.rs:451`; this packet's determinism obligation (AC-5) is satisfied trivially since the field is always empty, not because it round-trips real warnings. Image-entry handoff: `ImageEntry.png_path: String` and `ImageEntry.typed_capture: Option<serde_json::Value>` already exist (`crates/pnp-cli/src/visual_debug.rs:269-288`); `png_path` is left `String::new()` by packet 158 with a doc comment explicitly deferring rendering to this packet. No new `ImageEntry`/`Manifest` field is required — this packet fills `png_path` additively on the existing row.
- **[RESOLVED]** No synthetic-diagram payload exists. `CapturedIr` has exactly four variants and all four are geometry-bearing (`Perimeter`, `Infill`, `Support`, `LayerCollection`); there is no diagnostic-only/trace-only capture kind. Consequence for scope: `diagnostic_overlay` is always a composite over one of the four geometry views, never a standalone synthetic diagram — this matches the design's existing "compose overlays with a geometry visualization" approach and requires no renderer-local projection type. Overlay field availability is **conditional on capture variant**: seam coordinates (`SeamPosition`/`SeamCandidate` on `PerimeterRegion`) and region/object identifiers (`object_id`/`region_id`) are available on `Perimeter`/`Infill`/`Support`; travel anchors (`TravelMove`/`TravelRetract`) and execution annotations (`LayerAnnotation`) exist **only** on `LayerCollectionIR` captures. `layer_bounds` is not a capture field at all — where used as an overlay, the renderer must compute it as an XY bounding box of the already-captured geometry, not read it from a source field; this is a derived render-time value, not fabricated geometry, and does not violate the "documented fields only" constraint.
- **[RESOLVED]** Pure-Rust `png` dependency: absent from the workspace today (zero matches across the workspace root, `crates/slicer-runtime/Cargo.toml`, and `crates/pnp-cli/Cargo.toml` as of this grounding). It must be added fresh, owned by `crates/slicer-runtime` (matches this design's placement of the renderer as a pure function in `slicer-runtime`). Implementation must pick default/pure-Rust-only features (avoiding any C `zlib` backend) and record the exact enabled feature set plus MIT/Apache-2.0 license note in Step 3's evidence — this remains an implementation-time recording obligation, not an activation blocker.
