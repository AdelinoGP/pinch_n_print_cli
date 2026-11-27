---
status: draft
packet: brim-type-and-object-gap
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P05)
context_cost_estimate: M
---

# Packet Contract: brim-type-and-object-gap

## Goal

Replace `skirt-brim`'s single global bounding-box rectangle brim with per-object contour brim derived from the layer-0 outer-wall loops the module already receives, and make two keys drive it: `brim_type` selects outer / inner / both / none per object, and `brim_object_gap` offsets the brim band away from the object contour. Both keys are read nowhere in the tree today.

## Scope Boundaries

The packet touches the `skirt-brim` core module only — its manifest, `src/lib.rs`, and its test directory — plus one bounds arm in `slicer-scheduler`. Object outlines are reconstructed inside the module by grouping layer-0 entities on `PrintEntity.region_key.object_id`, keeping those whose `role` is `ExtrusionRole::OuterWall`, and unioning their closed loops through `slicer_sdk::host::clip_polygons` with `ClipOperation::Union`, which resolves contours and holes. No WIT type, no IR field, no `ResolvedConfig` field, and no new host input is added. The skirt path is untouched: it keeps the global bounding box, which is what a skirt is. Ear geometry is packet `257b-brim-ears`; `brim_use_efc_outline` is returned to the queue.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — `257a` derived from disk at authoring time under the 210a/210b letter-suffix precedent); ticket 05 (packet-list P05 membership); ticket 04 (tier rubric — Tier **B** re-derived in `design.md` § Tier Derivation, was Tier A).
- Unblocks: `257b-brim-ears`, which needs this packet's per-object contour and its `brim_type` dispatch before ear anchors can be detected.
- Activation blockers: none. No `[BLOCK]` is open — see `design.md` § Open Questions.

## Acceptance Criteria

- **AC-1 (`brim_type = no_brim`).** Given an enabled module with `brim_width = 5.0`, when `brim_type = "no_brim"` (canonical default `auto_brim`), then `run_finalization` pushes zero entities whose `role` is `ExtrusionRole::Brim`, while the skirt entities it pushes are unchanged in count. | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd no_brim_emits_no_brim_entities 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-2 (`brim_type = inner_only`).** Given one object whose layer-0 outline has an interior hole, when `brim_type = "inner_only"` (canonical default `auto_brim`), then every emitted brim loop lies inside that hole and none lies outside the object contour, whereas at `outer_only` every loop lies outside the contour and none inside the hole. | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd inner_only_emits_only_hole_brim 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-3 (`brim_type = outer_and_inner`).** Given the same holed object, when `brim_type = "outer_and_inner"`, then the emitted brim set is the union of the `outer_only` set and the `inner_only` set — strictly more loops than either alone. | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd outer_and_inner_is_the_union_of_both 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-4 (`brim_type` is per object).** Given two objects on the plate with different per-object `brim_type` values (`outer_only` and `no_brim`), then brim entities are emitted for the first object's `region_key.object_id` and none for the second's — the decision is per object, not global. | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd brim_type_is_resolved_per_object 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-5 (`brim_object_gap`).** Given `brim_type = "outer_only"` and a single square object, when `brim_object_gap = 1.0` (canonical default `0.0`), then the innermost emitted brim loop stands `1.0` mm further from the object contour than at `0.0`, and the loop count is reduced accordingly because the band width is unchanged. | `mkdir -p target && cargo test -p skirt-brim --test brim_object_gap_tdd gap_pushes_the_innermost_loop_off_the_contour 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-6 (contour brim, not bbox — the enabling behaviour).** Given a single L-shaped object, when `brim_type = "outer_only"` and `brim_width = 2.0`, then the emitted brim follows the L's concave corner — at least one emitted brim point lies strictly inside the object's axis-aligned bounding box while outside the object contour — which the previous rectangle brim could never produce. | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd brim_follows_concave_contour_not_bounding_box 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N1 (negative — unknown enum value rejected).** Given `brim_type = "gyroid"`, when the module is constructed, then `from_config` returns a `ModuleError` naming the key and the offending value rather than silently falling back to a default. | `mkdir -p target && cargo test -p skirt-brim --test brim_type_tdd unknown_brim_type_is_rejected 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N2 (negative — bounds enforcement).** Given the scheduler's manifest bounds layer, when `brim_object_gap = 3.0` (canonical max `2`), then resolution fails with the out-of-bounds error naming the key rather than clamping. | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N3 (negative — returned and deferred keys must not be stubbed).** Given the keys this packet does not build, then `skirt-brim.toml` declares no table for `brim_use_efc_outline`, `brim_ears_max_angle` or `brim_ears_detection_length`. | `grep -qE 'config.schema.(brim_use_efc_outline|brim_ears_max_angle|brim_ears_detection_length)' modules/core-modules/skirt-brim/skirt-brim.toml && echo FAIL || echo PASS`
- **AC-N4 (negative — no padding edits).** Given Authoring rule 2, then this packet's diff contains no change to `crates/slicer-gcode/src/serialize.rs`. | `git diff --stat -- crates/slicer-gcode/src/serialize.rs | grep -q . && echo FAIL || echo PASS`
- **AC-N5 (regression guard — skirt untouched).** Given `skirt_loops`, `skirt_distance` and `skirt_height` at any values, then the emitted skirt entities are identical before and after this packet: the skirt still uses the global bounding box. This is a guard, never a key's evidence. | `mkdir -p target && cargo test -p skirt-brim --test skirt_brim_tdd 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`

## Gate Commands

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (inspect the exit code; `skirt-brim/src/**` feeds the guest build)

## Docs Impact

- `docs/15_config_keys_reference.md` — regenerate; two keys move from unread to live.
- `docs/adr/` — no new ADR. The contour-derivation choice is recorded as DIV-1 in `design.md`, not as an architecture decision, because it introduces no seam and no cross-module contract.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `outer_inner_brim_area`, for the `has_outer_brim` / `has_inner_brim` derivation per `BrimType` value and for how `brim_object_gap` becomes `brim_offset` on the contour and on the reversed holes.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `PrintConfig.hpp` — the `BrimType` enum's declared value order and default, for the manifest's `values` list.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintObject::has_brim`, the gate this packet's `no_brim` arm reproduces.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_toolpaths`'s first-layer brim-avoidance block, cited as deliberately **not** ported (DIV-3).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
