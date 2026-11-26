---
status: draft
packet: brim-ears
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P05)
context_cost_estimate: M
---

# Packet Contract: brim-ears

# Goal

Ship `brim_type = brim_ears` as a real generator: detect ear anchors on the per-object contour that packet `257a` builds, and make both ear keys drive them — `brim_ears_detection_length` as the Douglas-Peucker decimation tolerance applied before detection, and `brim_ears_max_angle` as the corner-sharpness threshold that decides which vertices become ears. Neither key is read anywhere in the tree today; `brim_ears_max_angle` and `brim_ears_detection_length` have zero occurrences outside documentation.

## Scope Boundaries

The packet touches the `skirt-brim` core module only — its manifest, `src/lib.rs`, and its test directory — plus one bounds arm in `slicer-scheduler`. It adds a contour-decimation helper and a corner-detection helper, and dispatches `brim_type`'s `brim_ears` value (declared but rejected by `257a`) onto them. It adds no WIT type, no IR field, no `ResolvedConfig` field, and no module. It does not implement `brim_type = painted` (no point-valued paint carrier exists) and does not implement `brim_ears_outer_only`, which is not a ticket-12 key.

## Prerequisites and Blockers

- **Hard dependency: `257a-brim-type-and-object-gap`** (`status: draft` at authoring time — this is a FORWARD-DEP, not a satisfied prerequisite). `257a` creates the per-object layer-0 contour that ear detection runs on, and the `brim_type` mode dispatch this packet extends. Ear anchors are vertices of that contour; without it there is nothing to detect corners on, because the pre-`257a` brim is a bounding-box rectangle whose only convex vertices are its four corners.
- Depends on: wayfinder ticket 06 (packet numbering — `257b` derived from disk at authoring time under the 210a/210b letter-suffix precedent); ticket 05 (packet-list P05 membership); ticket 04 (tier rubric — Tier **B**, see `design.md` § Tier Derivation).
- Activation blockers: `257a` must be implemented first. No `[BLOCK]` is open — see `design.md` § Open Questions.

## Acceptance Criteria

- **AC-1 (`brim_type = brim_ears` ships).** Given an object whose layer-0 contour is a plus/cross shape with four sharp convex corners, when `brim_type = "brim_ears"` (canonical default `auto_brim`), then brim entities are emitted only in the neighbourhood of those corners — the total emitted brim path length is strictly less than the same object's `outer_only` brim, and at least one corner has brim within one `line_width` of it. | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd brim_ears_emits_only_at_convex_corners 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-2 (`brim_ears_max_angle` changes which corners qualify).** Given an object with one 90-degree corner and one 150-degree corner, when `brim_ears_max_angle = 100` (canonical default `125`), then only the 90-degree corner produces an ear; at the default `125` both do. The emitted ear count differs between the two runs. | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd max_angle_selects_which_corners_become_ears 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-3 (`brim_ears_detection_length` suppresses noise corners).** Given a contour whose straight edge carries a shallow zig-zag of many near-collinear vertices, when `brim_ears_detection_length = 3.0` (canonical default `1.0`), then the decimation removes the zig-zag and no ear is emitted along that edge, whereas at `0.0` — canonical's decimation-disabled value — ears are emitted along it. The ear counts differ. | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd detection_length_decimates_noise_before_detection 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-4 (`brim_ears_detection_length = 0` disables decimation).** Given the same noisy contour, when `brim_ears_detection_length = 0.0` (canonical's documented disable value, non-default), then no decimation runs and the ear set is exactly the set detected on the raw contour. | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd zero_detection_length_disables_decimation 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-5 (ear geometry is annular, not a disc).** Given one detected ear, then the emitted geometry is the ear polygon minus the gap-offset object island — no emitted ear point lies inside the object contour offset by `brim_object_gap`. | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd ear_geometry_excludes_the_object_island 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N1 (negative — decimation floor).** Given a contour of exactly 4 points and a `brim_ears_detection_length` large enough to decimate it below 4 points, then decimation is skipped and the original contour is used — canonical's documented guard — rather than yielding a degenerate contour. | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd decimation_is_skipped_below_four_points 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N2 (negative — bounds enforcement).** Given the scheduler's manifest bounds layer, when `brim_ears_max_angle = 200` (canonical max `180`) or `brim_ears_detection_length = -1.0` (canonical min `0`), then resolution fails with the out-of-bounds error naming the key rather than clamping. | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N3 (negative — `painted` still rejected).** Given `brim_type = "painted"`, then `from_config` still returns a `ModuleError` naming the missing point-valued paint carrier. This packet ships `brim_ears` and does not quietly enable `painted` alongside it. | `mkdir -p target && cargo test -p skirt-brim --test brim_ears_tdd painted_remains_rejected 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N4 (negative — no padding edits).** Given Authoring rule 2, then this packet's diff contains no change to `crates/slicer-gcode/src/serialize.rs`. | `git diff --stat -- crates/slicer-gcode/src/serialize.rs | grep -q . && echo FAIL || echo PASS`
- **AC-N5 (regression guard — `257a`'s modes unchanged).** Given `brim_type` at `outer_only`, `inner_only`, `outer_and_inner` or `no_brim`, then the emitted brim is identical to `257a`'s output; the ear keys are inert outside `brim_ears`. This is a guard, never a key's evidence. | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`

## Gate Commands

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (inspect the exit code; `skirt-brim/src/**` feeds the guest build)

## Docs Impact

- `docs/15_config_keys_reference.md` — regenerate; two keys move from absent to live.
- `docs/adr/` — no new ADR. Ear detection is a pass inside one module; it introduces no seam and no cross-module contract.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `make_brim_ears_auto`, the whole algorithm: the `_douglas_peucker` decimation gated on `ear_detection_length > 0` and skipped below four points, the `angle_threshold = (180 - max_angle) * PI / 180` conversion, the `convex_points` / `concave_points` selection, and the per-ear regular polygon of `POLY_SIDE_COUNT` sides at radius `size_ear`.
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `outer_inner_brim_area`, for how `size_ear` is computed by the caller (`brim_width_mod - brim_offset - flow.scaled_spacing()`) and how the gap-offset object island is subtracted to leave the annulus.
- `OrcaSlicerDocumented/src/libslic3r/Polygon.cpp` — `Polygon::convex_points` and `Polygon::concave_points`, for the exact angle convention the threshold is compared against.
- `OrcaSlicerDocumented/src/libslic3r/MultiPoint.cpp` — `MultiPoint::_douglas_peucker`, for the decimation semantics `brim_ears_detection_length` parameterises.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
