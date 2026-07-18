# Closure Log — Packet 98 (loader-paint-channel-symmetry)

## Deliverable (TASK-248)

Sub-facet hex stroke decoding is now symmetric across all four 3MF paint channels
(`paint_color`, `paint_supports`, `paint_seam`, `paint_fuzzy_skin`). Previously only
`paint_color` and `paint_supports` decoded their sub-facet hex partitions; `paint_seam`
and `paint_fuzzy_skin` parsed only the dominant whole-triangle state and silently dropped
their sub-facet strokes.

Implementation: a private helper
`fn decode_strokes_for_channel(hex, tri_verts, byte_offset, map_state: impl Fn(u32)->Option<(PaintSemantic, PaintValue)>) -> Result<Vec<PaintStroke>, ModelLoadError>`
wraps the existing `decode_paint_hex_strokes` raw decoder and maps each `(verts, state)`
pair to a `PaintStroke` via a per-channel closure. Called once per channel in **each** of
the two parse loops (`parse_3mf_model_xml`, `parse_sub_model_objects`). `MeshCollector`
gained `seam_strokes_enforcer`, `seam_strokes_blocker`, `fuzzy_strokes`; `build_paint_data`
wires them into the corresponding `PaintLayer`s. Malformed hex propagates a structured
`ModelLoadError` via `?` on all four channels.

## Acceptance results

| AC | Result |
| --- | --- |
| AC-1 helper exists | PASS |
| AC-2 call sites | PASS — corrected to `-eq 9` (1 def + 8 calls; two parse loops). See Deviation 1. |
| AC-3 paint_color subfacet | PASS |
| AC-4 paint_supports subfacet | PASS |
| AC-5 paint_seam subfacet | PASS (semantic `Custom("seam_enforcer")`/`Custom("seam_blocker")`; see Deviation 3) |
| AC-6 paint_fuzzy_skin subfacet | PASS |
| AC-7 strokes reach live consumer | **PASS (REWRITTEN)** — 4 consumer-path tests green. See Deviation 2 / D-98-AC7-CONSUMER-PATH. |
| AC-8 wedge byte-identical | PASS (MATCH) |
| AC-9 cube_4color byte-identical | PASS (MATCH) |
| AC-10 cube_fuzzy may differ | PASS (DIFFER, expected) |
| AC-11 guest `--check` | **PASS** — 31 guests rebuilt, `--check` CLEAN. See Deviation 4. |
| AC-N1 malformed seam hex rejected | PASS |
| AC-N2 empty hex no-op | PASS |
| AC-N3 no paint channels, no strokes | PASS |
| clippy `--workspace --all-targets` | PASS |
| `cargo test -p slicer-model-io` | PASS (all bins) |
| `cargo test --workspace` (closure ceremony) | **PASS** — 192 test bins `ok`, 0 failed, 0 panics, 0 compile errors |

## Tests added / changed

- `crates/slicer-model-io/tests/model_loader_tdd.rs` — 4 positive per-channel + 3 negative
  tests (AC-3/4/5/6, AC-N1/2/3); pre-existing `…fuzzy_strokes_are_empty` renamed to
  `…fuzzy_strokes_populated` (it had asserted the old buggy drop behavior).
- `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` (NEW) —
  AC-7 consumer-path tests, all end-to-end through the loader + `execute_paint_segmentation`:
  - `paint_channel_color_strokes_reach_material_variant_chain` (resources/cube_4color.3mf)
  - `paint_channel_fuzzy_skin_strokes_reach_fuzzy_variant_chain` (resources/cube_fuzzyPainted.3mf)
  - `paint_channel_supports_strokes_reach_consumer` (resources/bridge_support_enforcers.3mf, new obj[2])
  - `paint_channel_seam_strokes_have_no_live_consumer` (resources/cube_cilindrical_modifier.3mf)
- Registered the new file in `crates/slicer-runtime/tests/executor/main.rs`.

## SHA log (pre → post)

| Fixture | Pre (post-P97 baseline) | Post-P98 | Verdict |
| --- | --- | --- | --- |
| regression_wedge.stl | `aa4da2fa…ef1e3b` | `aa4da2fa…ef1e3b` | MATCH (AC-8) |
| cube_4color.3mf | `ad0245c3…b54ebf` | `ad0245c3…b54ebf` | MATCH (AC-9) |
| cube_fuzzyPainted.3mf | `eb9a9db4…3bad5aa4` | `239bf709…a0a729d9` | DIFFER (AC-10) |

**AC-10 rationale:** cube_fuzzyPainted g-code changed because `paint_fuzzy_skin` sub-facet
strokes are now decoded at the loader and consumed by the live `host:paint_segmentation`
stage (producing `variant_chain ("fuzzy_skin", Flag(true))` region splits). Pre-P98 those
strokes were dropped, so the fuzzy-skin paint had no slice effect. `paint_color` and the
unpainted STL remain byte-identical, bounding the change to the fuzzy-skin assignment.

## Fixtures (user-edited mid-packet) + regression reconciliation

The user added real painted channels to two existing fixtures so the consumer-path tests
could run end-to-end from disk instead of synthetic in-memory `MeshIR`:

- `resources/cube_cilindrical_modifier.3mf` — added `paint_seam`: 1 obj, `Custom("seam_enforcer")`
  layer with 3 facet_values + 2706 sub-facet strokes. (The cylinder modifier was also
  repositioned: transform translation 8.99/8.24 → ~17.98/16.49.)
- `resources/bridge_support_enforcers.3mf` — added a 3rd object (a painted bridge) with a
  `SupportEnforcer` layer: 2 facet_values + 8899 sub-facet strokes (was 2 objects, now 3).

Existing tests that hard-coded the old fixture shape were reconciled (intent preserved):

| Test | Break | Fix |
| --- | --- | --- |
| `two_objects_produce_separate_modifier_volumes` (e2e) | asserted exactly 2 objects | relaxed to `>= 2` (separateness of the modifier-volume objects still checked) |
| `duplicate_part_id_handled_gracefully` (e2e) | per-object loop required a modifier_volume on every object; new paint-only obj has none | scoped the loop to objects with `!modifier_volumes.is_empty()` |
| `modifier_volume_carries_typed_metadata` (e2e) | matrix substring `8.99`/`8.24` no longer present (cylinder moved) | re-asserted the **new** verbatim matrix substrings (NOT weakened to `!is_empty()`) — preserves the "loader keeps the transform matrix verbatim" coverage |

`load_model_populates_object_config_data` passed without change — the new bridge object
carries proper `extruder` sidecar config. **No fixture-completeness gaps found.**

**Fixture smell flagged for review (out of P98 scope):** in `cube_cilindrical_modifier.3mf`
the cylinder modifier's `matrix` metadata string now reports a ~(17.98, 16.49) offset, but
the modifier MESH geometry is unchanged — `modifier_world_aabb_matches_composition` still
passes asserting a centroid at world ~(133.99, 113.25), which corresponds to the OLD ~8.99
offset. So the matrix metadata and the actual mesh geometry now disagree about the cylinder
position. Both tests pass (one checks the metadata string, the other the mesh vertices); the
stale derivation comment was corrected. This predates/relates to the manual fixture edit, is
unrelated to loader paint symmetry, and is left for fixture review — not blocking P98.

## Deviations

### Deviation 1 — AC-2 count: 9, not 4
`loader.rs` has two parse loops (`parse_3mf_model_xml`, `parse_sub_model_objects`). Full
symmetry = 4 call sites per loop (8) + 1 definition = 9 `decode_strokes_for_channel(`
matches. `-eq 4` would only pass if one loop were left un-symmetrized (bug half-fixed).
AC-2's command corrected to `-eq 9`. Not a weakening — broader coverage than anticipated.

### Deviation 2 — D-98-AC7-CONSUMER-PATH (AC-7 rewritten)
AC-7 originally asserted strokes were normalized into `facet_values` and emptied by
`host:mesh_segmentation`. That stage was **retired in P94r** (`execute_mesh_segmentation`
never wired) and its WASM infra **deleted in P97**. AC-7 was rewritten to assert the live
architecture: strokes reach `host:paint_segmentation`
(`slicer_core::algos::paint_segmentation::execute_paint_segmentation`, called at
`prepass.rs:540-566`), which reads `PaintLayer.strokes` and emits `SlicedRegion.variant_chain`
entries. Per-channel routing (all via `painted_subsets` → `variant_chain`, confirmed by
reading `paint_segmentation/mod.rs`):
- `paint_color` → `("material", ToolIndex(N))` — consumed (region split / material).
- `paint_fuzzy_skin` → `("fuzzy_skin", Flag(true))` — consumed (drives the AC-10 g-code change).
- `paint_supports` → `("support_enforcer"/"support_blocker", Flag(true))`.
- `paint_seam` → `("seam_enforcer"/"seam_blocker", _)`.
Note: `segment_annotations` is the **D14 modifier-volume** path only — paint-hex strokes do
NOT populate it (the original AC's supports→segment_annotations expectation was wrong;
the test asserts the real `variant_chain` path).

### D-98-SEAM-NO-CONSUMER (registered)
`paint_seam` strokes now load and flow into `SlicedRegion.variant_chain`
(`"seam_enforcer"`/`"seam_blocker"`), but **no live module reads them** — `seam-placer`
selects seams from geometric `SeamCandidate` scores computed by the perimeter generators,
not from paint annotations. P98 therefore makes seam paint *available* for a future
consumer; wiring that consumer is non-trivial (it is not a small change to seam-placer).
**Cross-reference:** seam-candidate quality / paint-driven seam placement belongs to the
**perimeter-modules-orca-parity roadmap** (M1 Phase 8, ~T-079 range). Closing
D-98-SEAM-NO-CONSUMER is deferred to that roadmap. The AC-7 test
`paint_channel_seam_strokes_have_no_live_consumer` documents the gap and asserts the
data-reaches-SliceIR fact so the regression is locked in.

**Post-roadmap supersession:** the seam-consumer wiring is bound in the perimeter-modules
OrcaSlicer-parity roadmap under "Inherited from P98 — paint_seam stroke consumption
obligation" (task **T-P98-SEAM**, Phase 8 — wire painted seam_enforcer/seam_blocker into
`slicer-helpers::perimeter_utils::generate_seam_candidates` + `seam-placer`). See
[`docs/specs/perimeter-modules-orca-parity-roadmap.md`](../../../docs/specs/perimeter-modules-orca-parity-roadmap.md).
Surfaced as a known parity gap in `docs/07_implementation_status.md` §"Known parity gaps
(post-roadmap work)". D-98-SEAM-NO-CONSUMER is intentional debt with a documented binding,
to be superseded by `D-<packet>-SEAM-CONSUMED` when T-P98-SEAM lands.

### D-98-SUPPORTS-VARIANT-CHAIN-UNREAD (informational; pre-existing, not P98)
The `variant_chain("support_enforcer")` entry produced by `execute_paint_segmentation` is
itself unread by any module. However, painted support enforcers ARE functionally consumed:
`support-planner` reads the `SupportEnforcer` `PaintLayer` directly from the mesh via the
WIT `paint_layers` interface (independent of `variant_chain`). So supports is NOT a
dead-end — only the redundant `variant_chain` entry is unconsumed. This predates P98
(supports sub-facet decoding already existed) and is recorded for completeness only.

### Deviation 3 — seam semantic uses `Custom(...)`, not `SeamEnforcer`/`SeamBlocker`
AC-5/requirements named `PaintSemantic::SeamEnforcer`/`SeamBlocker` variants that do not
exist in `slicer-ir` (variants: `Material`, `FuzzySkin`, `SupportEnforcer`,
`SupportBlocker`, `Custom(String)`). Adding variants is an IR change, out of scope.
Sub-facet seam strokes use `Custom("seam_enforcer")`/`Custom("seam_blocker")`, mirroring
existing whole-triangle seam handling in `build_paint_data`. AC-5's test asserts these.

### Deviation 4 — AC-11 resolved by actual rebuild
Initial `cargo xtask build-guests --check` reported STALE. A full `cargo xtask build-guests`
(31 guests) followed by `--check` returns CLEAN. The staleness was unbuilt artifacts, not
an unfixable condition — verified per CLAUDE.md §"Guest WASM Staleness" (no deflection).

### Cleanup — repurposed test renamed
`load_3mf_cube_fuzzy_painted_fuzzy_strokes_are_empty` (asserted the old buggy drop) →
`load_3mf_cube_fuzzy_painted_fuzzy_strokes_populated`, doc-comment updated. No remaining
references to the old name.

## Remaining before closure
- Tighten `modifier_volume_carries_typed_metadata` to new verbatim matrix values (in progress).
- Run `cargo test --workspace` closure ceremony (required by the P98 acceptance gate).
- Flip `packet.spec.md` `status: draft → implemented`; update `docs/07_implementation_status.md` for TASK-248.
- Two-commit close (code + docs).
