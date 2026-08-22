# HANDOFF-224-s5 — Packet 224 remediation session 5 (2026-08-20/21) [condensed]

Session 5 ran the packet-224 parity-audit remediation: adversarial audit of `d2a92e1e..71b5a08b`
(33 commits → `parity-audit.md`, commit `8c8c30ff`; 49 findings: 3 CRITICAL / 20 HIGH / 21 MED / 5 LOW;
also disproved the "deleted orca goldens hid a regression" suspicion — they were never Orca data), then
the human-approved fix plan. **18 commits landed, `8c8c30ff..e85e546a`.** Packet 224 carries the debt of
closing any issues from packets 219–223 (human decision); those five are marked `implemented` and left
32 workspace test failures behind (baseline: `target/audit-evidence/ws-baseline-32-failures.txt`).

## Mandate — decisions in force (human, do not re-litigate)

Drop only findings already routed to stub packets (F-8, F-9, F-10, F-17, F-30, F-39, F-40, F-42-part,
F-43 → G-rows), fix everything else ("this implementation must be complete so follow-up work on the stub
packets can continue"); clear ALL 32 workspace failures NOW without attribution bisect; overhang
threshold default canonical **30**, not 45; **keep AC-6's no-Orca-read gate as-is** (invariant tests are
the regression prevention; do not track Orca G-code or distilled fixtures); F-19 = auto/manual axis only,
register `erSupportTransition` as gap G-20, do not implement; F-23 golden rebless deferred until all else
proven correct; paperwork vehicle = inside draft-status packet 224 (amend ACs as fixes land, then mark
implemented); definition of done = workspace green + clippy/check-literals + human-reviewed SupportTest.stl
G-code for Tree AND Traditional + HTML slicer report; execution via swarm, commit per coherent fix; no ADR,
no CONTEXT.md glossary changes.

## The 18 commits (oldest→newest except where noted; each verified before commit)

| # | SHA | What |
|---|---|---|
| 1 | `8c8c30ff` | The parity-audit report itself (read-only). |
| 2 | `1906ddce` | **G-15 cleared**: 61 `// exhaustive:` waivers across 34 files, 0 FRU; check-literals exits 0. Waivers-over-FRU rationale: 55 sites have no `Default`; 6 `SupportEntry` sites are identity contracts where FRU would weaken. |
| 3 | `f84cb555` | **F-6/F-27/F-45** (wasm-host): restored exact-Z occupancy rejection verbatim from `ed62090d^` (reason string `"body rejected: exact-Z occupancy"` load-bearing — `same_family_union` asserts `"body rejected: routing-cell collision"`); deleted post-union re-validation sweep (category error; canonical `union_` has no size cap); snapshot `RoutingCell` per group against mid-loop merge drift. |
| 4 | `98dd612a` | Parity-comparator hole closed: `compare_support_plan_ir` compares per-field and reaches geometry (roles → expolygons → points, skeleton points, exact integer compare for scaled coords) — previously a dropped skeleton point returned `Ok(())`. Exposed native/wasm entry-count divergence, resolved as guest staleness. |
| 5 | `6db44032` | **F-28**: untagged (`!any_tagged`) marshal emits up to four correctly-roled entries instead of collapsing support+interface+raft into one `SupportBody`; tool selection falls back to `path.role`; `support_tool_selection_assigns_entities` [1,1,1,1]→[1,2,1,2], fixtures unmodified. |
| 6 | `646c5ab1` | Aggregation degrade: `FamilyConflictPolicy { Degrade, Fail }` (cross-family clash no longer aborts the whole prepass). Worse half found and fixed: an arbitrary `if entry.family_id != "traditional" { continue; }` gate silently deleted the second entry for every non-traditional family at repeated identity. Same-family repeats union with no diagnostic; ordering asymmetry documented; `mismatched_family_fatal` stayed green unedited. |
| 7 | `85f1f889` | **Native leg dropped the plan — fixed**: `commit_native_layer_response` never received `support_plan` (identifier absent from marshal/native.rs) → natively-dispatched support could render none; threaded through dispatch.rs mirroring the wasm arm. "native=128 wasm=126" scare resolved as guest-artifact build skew, not transport. |
| 8 | `b5c305b7` | Stale contracts repaired (220/221/222/224 drift): wedge fixture was wrong family (4 tree tests asserted skeleton data that could not exist; also un-vacuumed `map_or(true, ..)` / `count > 0`); emission-order assumption → per-column contiguity; renderer fixtures now commit a plan (renderers stopped self-generating in 220/222); `ignores_support_plan_ir` replaced by inverse + renamed; `points.len()==2` snapshot → wall/fill structural checks; macro delegation accepts `_with_analysis`. Two parity tests went green only after #7. |
| 9 | `6ae49fc5` | Claim-dedup contract rewritten against post-221 behaviour (per-region family dispatch; both family candidates retained at startup); both rewrites strictly stronger. Explains defect survival: unit tests in execution_plan.rs updated, e2e elsewhere weren't (narrow-run blindness). Wedge root cause pinned: code 1201 `support demand 'demand-1' declined: NoRoute`. |
| 10 | `df6b75cd` | "Re-arm the support gates" (chronologically #10; see git log). |
| 11 | `5a725e2e` | **F-2**: `support_threshold_angle` typed field (default 30, canonical), `support_overhang_angle` alias via table-driven `CONFIG_KEY_ALIASES`, both-spellings rejected (HashMap order nondeterminism); producer reads typed field only; `support_angle` fallback (pattern-rotation key!) deleted. Proof: `orca-matched-config.json` set 30.0 while the producer used 45. Golden `precision_legacy_20mmbox.gcode` regenerated (one config-block line, zero toolpath bytes). Doc-lock test ties 30.0 to `ResolvedConfig::default()`. |
| 12 | `84bae156` | **Tree step 1/7 — F-16 volumes**: `TreeVolumes` replaces `LayerCollisionCache` (raw outlines ZERO inflation; avoidance inflated by `branch_distance/2` = canonical contact point_spread not clearance; iterative bottom-up ladder where canonical is recursive); guest-side iterative Douglas-Peucker (host `simplify_polygon` ignores tolerance on BOTH paths); manifest gains `support_object_xy_distance` (0.35). Step-5 call-site decision: gates read `get_collision(0.0, l)` + per-node predicate radius to avoid double-count. Found **G-23**: tripwire golden runs an empty `SupportGeometryView` exercising neither volume. |
| 13 | `a40f971a` | **Tree step 2/7 — F-1/F-34/F-35 + arena**: `NodeArena`/`NodeId` with cross-layer parent/child/parents mutation; per-node `support_roof_layers_below` replaces per-object band (which gave every second, lower overhang NO top interface); virtual gap node at `layer_nr − 1` with `distance_to_top = −gap_layers` (Z-walk deleted); rotated-bbox lattice span (corner contacts never generated). 3 expectations updated, each canonical; F-1 moved three red-by-design tests PAST the roof-band assertion onto the F-3 assertion (intended signal). |
| 14 | `cf4f7e62` | **Tree steps 3–4/7 — F-12/F-11**: per-part spanning trees (`nodes_per_part` off `outlines_below`, Prim per group) replace single global MST; canonical merge replaces `if d < merge_distance { drop[max_index] }` (leaf-degree test, midpoint node below, `dist_mm_to_top` parent selection, STUDIO-6326 multi-neighbour absorption, `move_out_expolys` in group 0); `support_branch_merge_distance_mm` deleted (never declared — G-16 partially self-resolves); new `support_line_width` manifest key (0.35) for `get_max_move_dist`'s cap; `move_out_expolys` documented deviation (projects on original ring, steps out analytically vs offsetting). |
| 15 | `0c32799d` | **F-24/F-25/F-26 canonical overhang detection**: layer-major whole-lower-layer UNION replaces per-region series (multi-region objects got spurious full-area supports); zero-angle is the overlap branch (`fw − overlap`); `+1` bump and 89° clamp; expand-back, tiny-spot filter (`−0.1·fw`), `support_expansion`, blockers subtraction, `jtSquare` everywhere. Producer fixtures rebuilt at mm scale (`GlobalLayer::default()` z=0 → offset 0 meant thresholded path NEVER exercised by its own tests). Determinism: work items sorted before par_iter. |
| 16 | `0d0c8d4a` | **F-19 auto/manual axis**: `SupportType` → NormalAuto/TreeAuto/NormalManual/TreeManual (canonical serde spellings), `is_auto()`/`is_tree()`/`family_claim()`; schema bumps region-map IR 2.0.0→3.0.0, analysis IR 1.0.0→1.1.0, NO WIT change (verified). `enforced`/`blocked` now REAL via per-object `slice_modifier_volumes` (deliberately per-object — shared bucket would let object A force support on B) and blockers fed to `detect_support_contacts` (was `&[]`). Manual mode skips thresholded branch, enforcer-only contacts. Latent bug fixed: `to_config_map`'s `format!("{:?}")` emitted `"Tree"` which case-sensitive prefix match failed → silent round-trip degrade to traditional. Residual gaps stated (100%-overlap edge; auto-mode enforcer append). |
| 17 | `659ac131` | Gap register: G-19 attribution CORRECTED (bisect baseline `ed62090d` is itself inside the audited range and deleted `overlaps_any`; "0 attributable to 224" was false; re-triaged 220 owns 5 groups, 221 owns 3, 223 owns 2, 222 owns 1, 224 owns 1). New rows: G-20 (`erSupportTransition` absent), G-21 (startup DAG still enforces pre-221 single-holder rule → 6 advisory conflicts/slice), G-22 (`support_threshold_angle` unbounded — user can set 200°), G-23 (tripwire exercises neither collision nor avoidance), G-24 (harness reports spurious mismatches from gitignored-guest staleness). |
| 18 | `e85e546a` | **Tree step 5/7 — F-13/F-14**: canonical move pass (full-length `get_max_move_dist` steps; `DO_NOT_MOVER_UNDER_MM`=5; `max_converge_distance`; cached `is_line_cut_by_contour`; `projection_onto`; STUDIO-4252 collision retry; sharp-tail skin follow; STUDIO-7883 radius clamp; `neighbours_of` skips `!valid`) replaces fractional-cap-then-clamp and deletes the code-1002 escape budget; `to_buildplate` seeded TRUE unconditionally, recomputed per descendant against RAW outlines; `unsupported_branch_leaves` deque with canonical parent-walk pruning; `get_collision/get_avoidance` return owned PolySet with `&self` ensure (two-bucket restriction lifted); 1002 assertions rewired to pruning assertions. Worker died at session limit mid-verification; coordinator completed verification (52/57, clippy, literals, 43 guests) and committed. |

## Disposition — the 49 audit findings

Closed with code: F-1, F-2, F-6, F-11, F-12, F-13, F-14, F-16, F-19, F-21, F-22, F-24, F-25, F-26, F-27,
F-28, F-34, F-35, F-45, F-4 (gates armed), F-44 (PENDING verification — diagnosed but no closing commit
found; verify before claiming done), plus F-5's cause.
Red-by-design awaiting tree step 7 (F-3 `carved.clear()` is the last tree defect): `raft_and_interface_layers_emit_expected_entry_count`,
`tree_family_tdd::distributed_contacts`, `tree_family_tdd::anchored_heights_and_termination` — fail on exactly
"layer N carries a TopInterface but no SupportBody" at layers 5/6/7.
Routed out by decision: F-8→G-18, F-9→G-12, F-10→G-13, F-17→G-10, F-30→G-16 (partially self-resolved),
F-39→G-05, F-40→G-07, F-42-part/F-43→G-17, F-29→G-20, F-23→rebless at end.
Still open module wave (not started): F-7 (interface spacing omits flow term, BOTH renderers; tree has no interface-spacing key), F-32 (orphaned `#[allow(dead_code)]` on `support_speed`), F-36 (bottom interface = whole layer), F-37 (no `closing`/`smooth_outward` regularization — needs non-gated slicer-core port), F-38 (attribution header missing on traditional-support-planner/src/lib.rs), F-46 (line-pinned Orca citation), F-47 (docs/15 drift), F-48 (stale doc comments), F-49 (top-band excludes plate layer), plus support-surface-ironing `begin_region` (sole remaining cause of `integrated_parity_support_surface_ironing`).

## Disposition — the 32 inherited workspace failures

Cleared (17): support_plan_validation ×3, same_family_union ×2, invalid_body_degraded ×2,
parity_comparator_* ×2, support_tool_selection_assigns_entities,
support_geometry_aggregates_family_outputs_before_one_final_commit,
integrated_parity_support_planner_native_matches_wasm, live_layer_support_tdd ×3,
integrated_parity_traditional_support, integrated_parity_tree_support, wedge invariants ×4,
macro_prepass_stages_each_delegate_to_their_sdk_trait_method.
Rewritten to post-X contracts (2): tree_support_active_holder, support_type_tree_config_selects_tree_support_holder.
Red-by-design → step 7 (3): the F-3 gates above.
Gated on tree geometry (5): wedge_support_marker_present, wedge_gcode_contains_support_feature_evidence,
slice_feature_evidence_failures_name_the_missing_family, mm_support_filament_real_fixture,
visual_debug_forwards_support_tool_selection — all trace to code 1201 NoRoute; re-measure after steps 6–7
with a FRESH pnp-cli build. tree-support-planner suite: **52/57** (3 red-by-design + tripwire).

## Defects found DURING remediation (not in any audit; meta-finding: each invisible because a gate above it was too coarse)

(1) tree-family duplicate entries silently deleted (`!= "traditional"` gate) — uncovered; (2) native dispatch leg dropped the support plan entirely — parity tests compared two fallback fillers; (3) parity fixture picked the wrong family making the native/wasm comparison vacuous; (4) painted enforcers/blockers had NO effect (`enforced`/`blocked` hardcoded false, blockers param empty); (5) `support_type` round-trip silently degraded (Debug format vs case-sensitive prefix match); (6) threshold-angle unbounded (G-22); (7) startup DAG pre-221 single-holder rule → 6 advisory conflicts/slice (G-21); (8) tripwire golden exercises neither collision nor avoidance (G-23). False alarm retracted: "native=128 wasm=126 transport gap" was guest-artifact staleness (G-24).

## Remaining work

A. Tree steps 6–7 sequential (briefs in step-2 commit message + design): step 6 = F-33 `smooth_nodes`
(consume `_contact_stats`, `skin_direction`, `need_extra_wall`; movement = (pts[i+1]−pts[i−1])/2) +
`draw_circles` ellipse-per-node-per-layer + `CIRCLE_RESOLUTION` (4 if avg_node_per_layer > 200 else 100)
+ distance-tolerance simplify — MUST land together (smooth is the only producer of final movement, the
ellipse matrix its only consumer); step 7 = F-3 `build_roles` carve (delete `carved.clear()`, keep
remainder, map roof_areas+roof_1st_layer→TopInterface, roof_gap_areas→emit nothing) flips the three
red-by-design tests green; then rebless golden (SUPPORT_PLANNER_REGEN_GOLDEN=1) and CHECK THE 40 DUPLICATE
ENDPOINTS ARE GONE — surviving duplicates = new defect, not baseline.
B. Module wave (parallel, guests rebuilt after): F-7 both renderers (`line_width_to_spacing(width,
layer_height)`, layer height from `SliceRegionView::effective_layer_height()`, rename key to canonical
`support_interface_spacing`, add `support_bottom_interface_spacing`), F-36/F-37/F-49, non-gated
`slicer_core` `smooth_outward` port, F-32/F-38 hygiene, support-surface-ironing `begin_region`, F-44
verification, F-46/F-47/F-48 docs, `cargo xtask gen-config-docs` re-run.
C. Closure: re-measure the 5 NoRoute-gated e2e tests (fresh pnp-cli build!), workspace tests → 0 failures,
clippy + check-literals green, re-measure design.md figures, amend packet ACs, mark implemented, produce
review artifacts: SupportTest.stl G-code for tree(auto) AND normal(auto) (orca-matched config,
--module-dir modules/core-modules) + --report HTML.

## Operational rules learned (next sessions)

Never `git stash` in a shared tree (one worker's stash was popped by another this session; workers read
baselines via `git show HEAD:<path>` or cp). Serialize guest rebuilds when lanes touch slicer-ir/slicer-core/
modules sources — one coordinator-run build after both land; workers verify native-only. Always
`cargo build -p pnp-cli` before reading an e2e result (47/128 e2e failures were once purely binary
staleness). `cargo xtask test -p <crate>` does NOT pass `-p` through (runs the wrong crate with a filter;
use plain `cargo test -p ...`). slicer-core tests need `--features host-algos` or compile to zero tests.
Workers must quote canonical rules inline in briefs (one worker burned its entire budget re-deriving
TreeSupport.cpp and shipped nothing).

## Evidence locations

Audit report: `parity-audit.md` in this packet dir. Gap register (G-01..G-24):
`docs/specs/support-parity-gap-register.md`. Baseline failures + check-literals before-state:
`target/audit-evidence/`. Per-step logs: `target/tree-step*.log`, `target/f2-*.log`, `target/agg-*.log`,
`target/overhang-*.log`, `target/rearm-*.log`, `target/f19-*.log`, `target/tree-step5-verify.log`.
Session commit range: `8c8c30ff..e85e546a` on `parity/support-planners`.
