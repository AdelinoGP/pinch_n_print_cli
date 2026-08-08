# Deviation Remediation — packets 206–212

Source of truth for every packet below is its row in `docs/DEVIATION_LOG.md`. Each
row already carries both-side evidence, canonical function+file citations, and
`Affected section:` pointers; authors read the row first and ground its claims
against the tree before designing.

These rows were filed by a deferred-task verification sweep on 2026-08-07 that
classified 30 carried-over items into CONFIRMED / STALE / NOT-A-DEVIATION /
UNVERIFIABLE. Only CONFIRMED items were logged. Four of the confirmed set were
fixed in the same session (DEV-124, DEV-125, DEV-130, and the region-split half
of DEV-123) and are **not** in this queue. DEV-131 is deliberately absent: its
impact is unmeasured and a packet before profiling would be guessing.

## Approved plan

**PACKET 1 (206) — Seam paint delivery.** Bundles DEV-123 (open half), DEV-133,
DEV-134.

- (a) Populate `PaintSemantic::Custom("seam_enforcer"/"seam_blocker")` into
  `SlicedRegion.segment_annotations`, so packet 108's existing reader
  (`seam_paint_boxes` → `apply_seam_paint_bias`, `modules/core-modules/classic-perimeters/src/lib.rs`)
  and `seam-planner-default`'s `paint_annotation_type` are actually fed. The
  writer is a sibling of `build_modifier_segment_annotations`
  (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`). Needs **no** IR or
  WIT change — `project_seam_planning_view` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
  already marshals `segment_annotations` verbatim.
- (b) DEV-133: replace `paint_marker`'s substring match on `"enforcer"` /
  `"blocker"` (`modules/core-modules/seam-planner-default/src/visibility.rs`)
  with exact semantic matching. Today `SupportEnforcer` / `SupportBlocker` are
  read as seam intent.
- (c) DEV-134: add the missing `apply_seam_paint_bias` call to
  `modules/core-modules/arachne-perimeters/src/lib.rs`.

These three MUST land together: (a) alone makes (b)'s leak user-visible, and (a)
without (c) leaves seam bias classic-only. Canonical: `gather_enforcers_blockers`
(`SeamPlacer.cpp`).

**PACKET 2 (207) — DEV-122 per-region shell config in paint segmentation.**
`execute_paint_segmentation` reads `region_map.configs.first()`, which in
production is a dead placeholder (`RegionMapIR::default()` pre-seeds `configs[0]`
with `ResolvedConfig::default()`; `run_slice` inserts `slice_has_paint` into
`extensions`, which `ResolvedConfig`'s `PartialEq` compares, so real configs
intern at `ConfigId(1+)`). Every painted slice therefore places shells with
`top=3, bottom=3, layer_height=0.2, width=0.45` regardless of user config.
Requires a `propagate_top_bottom` signature change (it currently takes scalar
shell params for a whole-layer-stack call) plus a decision on multi-object scenes,
where `painted_subsets` merges across objects. Precedent to follow:
`resolve_shell_counts` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`).
Also fix the `RoleWidthContext` hardcoded nozzle diameter / zero
`outer_wall_line_width` at the same site. **Ground against TASK-253 / packet 128
("Paint-segmentation shell-depth per-object propagation") before designing — that
work overlaps this territory and this is not greenfield.** High risk: moves
painted-model output.

**PACKET 3 (208) — DEV-126 wall-flag path clip.** Replace the PnP-invented
nearest-vertex reprojection in `build_wall_flags` / `nearest_original_vertex`
(`crates/slicer-core/src/perimeter_utils.rs`) with the canonical shape: clip
finished wall paths against the paint region's `ExPolygons`, mirroring
`apply_fuzzy_skin`, `group_region_by_fuzzify` and `Algorithm::split_line`
(`Feature/FuzzySkin/FuzzySkin.cpp`). Needs new Clipper-based line-split
infrastructure — `split_line` has zero occurrences in-tree — and touches shipped,
tested classic-perimeters behaviour.

**PACKET 4 (209) — DEV-127 scan-line duplication.** Two drifted copies:
`scan_expolygon` (`modules/core-modules/rectilinear-infill/src/lib.rs`) and
`SupportFiller::fill_expolygon` (`modules/core-modules/traditional-support/src/lib.rs`),
differing in rotation origin, vertex test (half-open vs strictly-between), and
scan start. Canonical shares one filler: `fill_expolygons_generate_paths`
(`SupportCommon.cpp`) → `FillRectilinear` (`FillRectilinear.cpp`). Real remedy is
WIT-interface pattern services. `docs/adr/0009-raft-as-layer-infill-role.md` is
stale — it names `fill_expolygon_multi`, points at a `docs/specs/_OLD/` path, and
its follow-up TASK-270 was reused for the visual-debug renderer (packet 160); the
packet should correct it.

**PACKET 5 (210) — DEV-128 support-planner f32 → coord_t.** `Pt { x: f32, y: f32 }`
and ~113 f32 sites in `modules/core-modules/support-planner/src/lib.rs`; canonical
`SupportNode::position` is `Point` (`coord_t`, i64). CRITICAL: PnP's unit is
100 nm, not canonical's 1 nm — divide canonical constants by 100
(`docs/08_coordinate_system.md`). Trigger is invariant-2 (collision-free) failures
on dense large-XY models.

**PACKET 6 (211) — DEV-129 support_interface_bottom_layers.** Currently warn-only
(code-1003 in `TreeSupportPlanner`'s prepass), pinned by
`modules/core-modules/support-planner/tests/diagnostics_tdd.rs` AC-6/AC-N3, which
MUST be rewritten to the new contract, never weakened. Blocker to solve in design:
`PlannedSupportNode` carries only `dist_to_top`; there is no `dist_to_bottom` and
no notion of where a branch lands on model geometry below, which is exactly what a
bottom-interface band requires. Canonical:
`number_of_support_interface_bottom_layers` (`SupportParameters.hpp`), plus
`SupportCommon.cpp`, `TreeSupport.cpp`, `TreeSupportCommon.hpp`.

**PACKET 7 (212) — DEV-132 extra_perimeters.** Two coupled divergences:
(a) `arachne_params_from_config` never reads `extra_perimeters` while classic
applies it every layer, so switching `wall_generator` silently drops bonus walls;
(b) PnP models it as a config key at all, whereas canonical carries
`surface.extra_perimeters` as a per-`Surface` member folded into `loop_number` in
**both** `process_classic` and `process_arachne` (`PerimeterGenerator.cpp`) — the
only `"extra_perimeters"` string canonically is `JSON_SURF_EXTRA_PERIMETERS` in
`Print.cpp`. Fixing (a) alone entrenches (b). Unit relation if wired: arachne wall
count enters via `max_bead_count = 2 * wall_count`, per canonical's
`max_bead_count = 2 * inset_count` in `WallToolPaths::generate`.

## Per-packet grounding obligations

Step-4 grounding is mandatory: treat this plan as claims, not evidence. Each
author verifies these against the tree **before** designing.

**206** — that `project_seam_planning_view` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
really marshals `segment_annotations` verbatim (i.e. no WIT change needed); the
exact shape of `build_modifier_segment_annotations`; that `seam_paint_boxes` /
`apply_seam_paint_bias` exist as described; the current `paint_marker` substring
logic. **The region-split half of DEV-123 is ALREADY FIXED** (`is_seam_paint_semantic`
in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`) — this packet owns
only the writer, plus DEV-133 and DEV-134. Do not re-specify the filter.

**207** — **TASK-253 / packet 128 "Paint-segmentation shell-depth per-object
propagation" already exists**; find it (`docs/07_implementation_status.md`,
`docs/spec_packets/`, `docs/spec_packets/_OLD/`) and establish what it did. This is not
greenfield. Also verify the `RegionMapIR.configs` interning claim (`config_for`,
`ConfigId`, `intern_config`) and the `resolve_shell_counts` precedent.

**208** — the real shape of `build_wall_flags` / `nearest_original_vertex` and both
perimeter-module call sites; that `split_line` genuinely has zero occurrences under
`crates/` and `modules/`; and `crates/slicer-core/tests/inner_wall_concave_reprojection_tdd.rs`
+ `inner_wall_material_boundary_tdd.rs`, which pin the CURRENT technique — the
packet must state how they are replaced and must NOT weaken them. Establish what
the repo's Clipper binding (`clipper2-rust`, `crates/slicer-core/src/polygon_ops.rs`)
can already do before specifying new line-split infrastructure.

**209** — confirm and COMPLETE the divergence list between the two scan-line
copies; deciding which behaviour is canonical IS the core design question. Whether
WIT pattern-services infrastructure exists at all is a real open question — record
a `[BLOCK]` if it needs a user decision.

**210** — re-derive the f32 site count (do not quote "~113"). Large blast radius:
the step that retypes `Pt` OWNS every compiling site and every test that hard-asserts
an old float value (SKILL struct-literal blast-radius discipline).

**211** — the core design work is the blocker itself: `PlannedSupportNode` has only
`dist_to_top`. Study the EXISTING top-interface densification in the same file
(driven by `dist_to_top` + `support_interface_top_layers`) — the bottom band is its
mirror and is the best available model. `diagnostics_tdd.rs` AC-6/AC-N3 pin the
warn-only contract and must be REWRITTEN to the new contract, never weakened.

**212** — the two divergences are coupled: fixing (a) entrenches (b). Decide with
evidence between (i) mirror the key and accept the modelling divergence, (ii) move
to a per-`Surface`/per-region model, or (iii) stage them; record a `[BLOCK]` rather
than guessing. Compose with the just-landed DEV-125 code (`base_wall_count + 1`
under a four-conjunct guard, after the `extra_perimeters` addition and before the
`only_one_wall_first_layer` clamp). `manifest_default_reconcile_tdd.rs` is
exhaustive-by-enumeration: any manifest key added or removed must be reflected in
its per-module fallback table, and that is part of the blast radius.

## Conventions binding every packet

- Cite in-tree code by symbol + crate-qualified path; a line number is a
  navigation hint, never an identifier.
- Cite OrcaSlicer by function + file, never by line number.
- Ledger facts (next free `DEV-###` / `TASK-###`, line counts, SHAs) are
  re-derived at point of use, never frozen into the packet.

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 206-seam-paint-delivery | Feed seam paint to the already-ported seam placer, fix the support-paint substring leak, and wire arachne's missing bias call. | TASK-322 | - | **generated** | `docs/spec_packets/206-seam-paint-delivery/` |

> **Row #1 PREFLIGHT PASS** (independent reviewer, round 3; 0 blockers, 0 high). Two
> fix rounds were needed. Round 1 fixed AC-7's import regex, which failed against
> rustfmt's real multi-line braced output; round 2 fixed AC-9, which had the same
> defect class against `docs/05_module_sdk.md`'s 76-char hard wrap, plus a
> mirror-image false-PASS in its `!`-negative clause. Final audit: 23 greps checked,
> 14 executed against the tree, constructed post-edit trees, and 5 mutated negatives;
> no defect classes remaining.
>
> Two LOW informational residuals, recorded not fixed: (a) AC-9 clause 1 still misses
> if the doc wraps immediately after the open paren (`-U` without
> `--multiline-dotall`) — judged an implausible wrap point; (b) `docs/05_module_sdk.md`
> also cites `seam_paint_boxes` as living in `classic-perimeters/src/lib.rs` and T-083
> names `classic-perimeters` alone — both go stale from this packet and no bullet or AC
> pins them.
| 2 | 207-paint-segmentation-per-region-shell-config | Resolve shell params per painted `variant_chain` instead of from the dead `configs[0]` placeholder. | TASK-323 | #1 | **generated** | `docs/spec_packets/207-paint-segmentation-per-region-shell-config/` |

> **Row #2 PREFLIGHT PASS** (independent reviewer, final round; 0 blockers, 0 high,
> no defect classes). Two fix rounds. Round 1 replaced a FALSIFIED granularity
> claim: the packet had rejected per-region resolution as "not implementable
> — painted facets carry no region identity", but `painted_subsets`' key
> `(sem_name, PaintValue)` IS one `RegionKey.variant_chain` element and the
> function already performs that lookup; resolving from BASE would have silently
> dropped `paint_config:<semantic>:top_shell_layers`, reproducing DEV-122's own
> failure mode one axis down. Round 2 fixed a STRUCTURAL defect — six unit ACs
> called module-private items from an external test crate, with both candidate
> fix sites forbidden by the packet's own step boundaries; they are now homed in
> an in-crate `#[cfg(test)] mod shell_config_resolver_tests`.
>
> Feature-gate trap confirmed by execution: `crates/slicer-core/src/lib.rs` gates
> `pub mod algos` on `host-algos`, so a bare `cargo test -p slicer-core --lib`
> compiles the module away (68 tests vs 181 with the feature). Every cargo AC in
> this packet carries `--features host-algos`.
>
> Two non-blocking notes: Step-3's three `slicer-runtime` sweep commands stop at
> `grep -E '^test result'` with no `-q` assertion (all three filters verified to
> resolve to real registered tests today, so no live vacuous pass); and ADR-0030
> pre-authorizes the same golden re-blessing this packet performs — a citation
> gap, not a contradiction.
| 3 | 208-wall-flag-path-clip | SUPERSEDED — deferred by user decision 2026-08-07; the mechanism is inert, revisit only after a Material/FuzzySkin `segment_annotations` writer exists (packets 206/207). | TASK-324 | - | superseded | - |
| 4 | 209-scanline-pattern-service | RE-SCOPED 2026-08-07: reconcile the **three** drifted scan-line copies in place so they agree on one canonical behaviour; **no shared kernel, no extraction**. Each copy keeps its own implementation; the duplication itself is left to a future WIT pattern-services packet. | TASK-325 | - | **generated** | `docs/spec_packets/209-scanline-pattern-service/` |

> **Row #4 PREFLIGHT PASS** (independent reviewer, decisive round; 0 blockers,
> 0 high; **19/19 ACs executed**; zero exit-status swallowing; no PCRE use; no
> defect classes). Reached after a full re-scope plus four fix rounds.
>
> **Slug caveat:** the directory is still `209-scanline-pattern-service`, but there
> is no service and no extraction — the packet reconciles three copies in place.
> Renaming was declined mid-flight to avoid breaking cross-references; treat the
> slug as historical.
>
> **The grid is MOVED to canonical, not deferred.** Measured: rectilinear today
> `floor(h/s)+1`; support AND ironing today `ceil(h/s)-1`; canonical `ceil(h/s)`.
> No copy matched canonical, so "preserve shipped behaviour" was preserving a third
> distinct wrong answer. Two fixtures are re-baselined as an owned step
> (`square_10mm_density_20_emits_n_raw_segments` 6→5; the very-small-polygon case
> 0→1). **This moves shipped `rectilinear-infill` output** — deliberate, per the
> repo rule that canonical parity outranks a green suite.
>
> **The real bug is the vertex test**, not the duplication: strictly-between
> (support + ironing) drops BOTH events at a true crossing, losing the crossing and
> inverting inside/outside for the rest of that scan row.
>
> **DEV-127 stays OPEN** — three copies remain, now agreeing on behaviour, so it
> cannot close as "duplication gone". Its target close (WIT pattern services) is
> unchanged. The row's stale `SupportFiller` type is corrected to
> `TraditionalSupport` and the third copy added.
>
> **A claim that mutated three times, now measured:** whether canonical's half-open
> bound is separable from `align_to_grid`. Stated first as coupled, then struck as
> "false", finally measured — `make_fill_lines` runs
> `bounding_box.merge(align_to_grid(...))` BEFORE reading `min.x()`, so
> `align_to_grid` sets the grid ORIGIN and is NOT refuted; only inseparability from
> the `full_infill()` inset is refuted. The corrected wording is what
> `D-209-HALF-OPEN-SCAN-GRID-ADOPTED` will carry.
>
> Two of nineteen ACs (AC-11, AC-12) are honestly labelled regression guards that
> are green today, with an explicit rule that neither may be cited as proof a step
> landed. Non-blocking nits recorded: the packet quotes `slicer_core::infill_ops`
> where ADR-0026 spells it `slicer-core::infill_ops`; design.md's "reproduces
> canonical's solid branch exactly" is loose by exactly the inset the packet
> separately files as unported.

> **Row #4 re-scoped — the original approach was forbidden by an ADR nobody had
> checked.** The packet proposed promoting a shared scan-line kernel plus
> `adjust_solid_spacing` into `crates/slicer-core/src/scanline_fill.rs`. That is the
> `slicer-core::infill_ops` proposal renamed — proposed at the 2026-07-01
> infill-parity grilling and **rejected by the project owner**, which is what
> `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` exists to record.
> Its §Consequences state `slicer-core` gains ONLY `clip_polylines` because that is
> generic geometry with no domain logic (a scan-line fill kernel is fill-pattern
> logic); its decisive argument is the multi-language module promise — a C++ or Zig
> infill component must not have to link a Rust helper; and its §Future-Reviewer
> Notes say "Do not re-suggest `slicer_core::infill_ops`." The packet cited
> ADR-0026 zero times. Caught by the S8 gate on the third review, after an author,
> two reviewers and two fix rounds had passed over it.
>
> **The ADR-0009 override approved on 2026-08-07 is now MOOT** — with no extraction
> there is nothing to amend, so it lapses unused. `D-209-ADR-0009-AMENDED` is
> dropped.
>
> The third copy (`fill_expolygon`, `modules/core-modules/support-surface-ironing/src/lib.rs`)
> is now IN scope: reconciling only two of three would leave the divergence
> half-fixed. The correctness driver is the vertex test — rectilinear uses a
> half-open `scan_y < lo \|\| scan_y >= hi` while support and ironing use
> strictly-between, which can drop or double-count a scan line at a vertex.
| 5 | 210a-support-planner-coord-t | Owns DEV-128: migrate support-planner tree-node geometry from f32 mm to scaled-integer coord_t, and — in the one and only rewrite of `smooth_branches` — extract the sub-chain gap walk into `split_column_into_chains` for #5b to consume. Re-split from the merged 210 (210+211) by user decision 2026-08-07 after the reviewer ruled SIZE: must decompose. | TASK-326 | - | **generated** | `docs/spec_packets/210a-support-planner-coord-t/` |

> **Row #5 PREFLIGHT PASS** (independent reviewer, decisive round; 0 blockers,
> 0 high; 17/17 ACs executed; zero exit-status swallowing; no defect classes).
>
> **The f32 round-trip envelope was wrong three times and is now measured.** The
> original design claimed `2^24`, the 210+211 merge "corrected" it to `2^23`, and
> both were false. Re-measured twice independently with `rustc -O` against the
> verbatim helper bodies: first failure `u = 5_120_004 → 5_120_005` (off by exactly
> one unit), largest contiguous exact envelope `|u| ≤ 5_120_003` (symmetric),
> analytic bound `2^22 = 4_194_304` units = 419.43 mm. **Failures above the bound
> are sparse, not monotone** — `5_120_007`, `8_388_608` and `16_777_216` all
> round-trip exactly, which is how a spot check kept confirming whichever bound was
> being tested. The false justification "covers every real build volume with
> margin" is retired (600 mm-class beds exceed it on one axis); the wire-format
> decision survives on three different grounds — graceful ±1-unit (100 nm) failure,
> that being 3–4 orders below every downstream tolerance, and widening being
> cross-crate/cross-WIT. AC-N7 pins the numbers in-tree so they cannot rot a fourth
> time.
>
> **A hole-handling near-miss was caught across the 210a/210b seam.** 210a retypes
> `point_in_any_expoly` while 210b consumes it; neither packet originally stated
> that holes matter, and 210b was calling the ring-only `point_in_polygon_units`
> against `Vec<ExPolygon>` (which does not type-check). A contour-only retype would
> have satisfied every AC-7 clause and placed support branches inside holes. Now
> pinned three ways: the composition written verbatim as a preserved semantic;
> net-new behavioural AC-N8 (verified RED today and genuinely discriminating); and
> a Step 0 gate in 210b telling the implementer to STOP, not adapt.
>
> Two LOW non-blocking notes: the §Acceptance Ceremony re-dispatch list omits AC-N8
> by name (its catch-all line still covers it); and AC-8 clause (c) would go blind
> to an already-committed regeneration if the work were done directly on `master`,
> though Step 4's ordering makes it fire while the tree is dirty.
| 5b | 210b-support-interface-bottom-layers | Owns DEV-129: on #5's migrated shape, implement bottom-interface (floor) bands replacing the warn-only code-1003 stub — canonical's `< 0 ⇒ use the top count` fallback, model-landing detection against the per-layer collision cache, and upward scan-line densification. Adds a second caller to #5's `split_column_into_chains`; never reopens `smooth_branches`. TASK-327 is revived (it was folded into TASK-326 by the 2026-08-07 merge, restored by the same day's re-split). | TASK-327 (revived) | #5 (must be **implemented and merged**, not merely generated) | **generated** | `docs/spec_packets/210b-support-interface-bottom-layers/` |

> **Row #5b PREFLIGHT PASS** (independent reviewer, decisive round; 0 blockers,
> 0 high; 17/17 ACs executed; zero exit-status swallowing; no defect classes).
>
> **AC-12b was the one real catch** — the only criterion separating "predicate
> defined and unit-tested" from "`plan_for_object` actually calls it". Its original
> regex was satisfied by the DECLARATION, so a re-inlined call site printed
> `AC12b-PASS`. Now body-scoped to `plan_for_object` (measured: spans lines 390–869,
> tempered dot terminates at the fn's closing brace). Verified on two purpose-built
> harnesses: wired → PASS/exit 0; re-inlined → no output/exit 1; the old clause
> passed BOTH. This is the same shape as `apply_seam_paint_bias` (reader with no
> writer) and DEV-094's phantom bridge — a helper that exists and passes its own
> tests while no production path invokes it.
>
> **ADVISORY — PCRE portability.** AC-12b is the ONLY `rg -P` user in the entire
> `docs/spec_packets/` tree. Verified available here (ripgrep 14.1.0, `+pcre2`,
> PCRE2 10.42) and `.github/workflows/ci.yml` never invokes `rg`, so CI cannot hit
> it. The risk is fail-closed — an `rg` built without PCRE2 (default
> `cargo install ripgrep`, some musl/Alpine packages) exits non-zero with "PCRE2 is
> not available in this build", a false RED, never a false GREEN. Worth a one-line
> note in AC-12b telling the runner to check `rg --version | rg pcre2` before
> treating a red as a code defect. Not applied — the packet had already passed and
> further edits would need another review round.
| 6 | 211-support-interface-bottom-layers | SUPERSEDED — absorbed into #5 by user decision 2026-08-07. Both packets rewrote `smooth_branches` (`modules/core-modules/support-planner/src/lib.rs`) and neither planned for the other's edit: #5 retyped its sub-chain gap walk to an integer Laplacian, #6 extracted that same walk into `split_column_into_chains`. Directory retained for provenance; do not implement or delete. | TASK-327 (folded into TASK-326 by the merge, then REVIVED for #5b by the re-split — register it against #5b, not here) | #5 | superseded | `docs/spec_packets/211-support-interface-bottom-layers/` (frozen) |
| 7 | 212-extra-perimeters-parity | Reconcile extra_perimeters across both generators and against canonical's per-Surface model. | TASK-328 | - | generated | `docs/spec_packets/212-extra-perimeters-parity/` |

> **Row #7 is DONE — do not regenerate or overwrite `docs/spec_packets/212-extra-perimeters-parity/`.**
> All five files written (`packet.spec.md`, `requirements.md`, `design.md`,
> `implementation-plan.md`, `task-map.md`); preflight S0–S8 PASS.
>
> Three grounding corrections it made against the brief, which later packets should
> not re-inherit as facts:
> 1. The pinning test lives at `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs`,
>    **not** under `modules/core-modules/classic-perimeters/tests/`.
> 2. `TASK-328` does not yet exist in `docs/07_implementation_status.md` (highest
>    present is `TASK-315`), so the packet creates it. The same is true of
>    TASK-322–327 — each packet creates its own row.
> 3. **Canonical `Surface::extra_perimeters` has a dead sole writer** —
>    `PrintObject::make_perimeters`, short-circuited by the BBS patch. That is what
>    decides packet 212's core design question against building per-`Surface`
>    plumbing in PnP.
>
> Open `[FWD]` questions for the user: FWD-1 risk level (Low vs Medium) for the
> newly-filed half-(b) deviation row; FWD-2 optional correction of the rotted
> "vestigial, unread `wall_count`" comment in
> `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs`;
> FWD-3 (informational) `crates/slicer-gcode/src/serialize.rs`'s
> `("extra_perimeters", "0")` CONFIG_BLOCK entry is generator-agnostic and
> correctly needs no change.

Statuses: `pending` (not generated) · `generated` (files written and
`PREFLIGHT PASS`) · `blocked` (unanswerable `[BLOCK]` or gate failure after two
fix rounds) · `superseded`.

**Commit this plan file and the packet directories together.**
