# Support Families and Anchored Entities — Completion Plan (v2)

Status: approved (2026-08-22, grill-with-docs session); supersedes the 2026-08-12 plan.

## How to consume this document (for LLM packet authors)

This plan is the single authority for completing the support-families work. Authority order
when documents disagree: **this plan > the gap register > packet 224's handoffs > the
2026-08-12 plan > the old remediation plans**. Every file path, symbol, and test name in the
per-packet briefs and the appendix was **verified against the working tree on 2026-08-22 at
commit `5d0e2350`** (branch `parity/support-planners-clean`). Symbols drift: re-ground every
load-bearing name against the live tree at packet-authoring time (Authoring Rule 3), but
**never invent replacements** — if a cited symbol is gone, stop and re-derive from this plan's
sources, do not substitute a plausible-sounding one. The Known Traps section lists the exact
mistakes previous agent sessions made; each is stated so it cannot be repeated.

Sources:

- `docs/adr/0059-support-families-and-anchored-entities.md`
- `docs/specs/support-parity-gap-register.md` (G-01..G-24)
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` (~20 rows,
  recorded-not-fixed)
- `docs/spec_packets/224-support-family-orca-closure/handoffs/HANDOFF-224-s6.md`
- `docs/spec_packets/224-support-family-orca-closure/parity-audit.md` (F-1..F-49)
- The 2026-08-12 plan (predecessor), `docs/specs/support-generation-remediation-plan.md`,
  `docs/specs/support-generation-defect-verified-findings.md`

## 1. Why this rewrite

The 2026-08-12 queue (packets 219–224) was implemented on branch `parity/support-planners-clean`,
but closure consumed ~80 remediation commits (squashed into 4 commits: `bab79c5c`, `55211648`,
`6b03d0b8` + docs `a644abee`). Packets 219–223 are `implemented`; packet 224 remains `draft`
with `TASK-335` unchecked. The remediation produced a precisely measured remainder: the gap
register (G-01..G-24), five unnumbered stubs under `docs/spec_packets/stubs/`, the divergence
record `orca-divergences.md`, six open deviations (DEV-141..DEV-146, renumbered from
DEV-135..140 after a mainline collision), and four never-implemented drafts (215/216/217/218).

This plan replaces the 2026-08-12 queue with a completion queue (236–242) authored as deltas
on the current branch code, promotes the five stubs into numbered packets, routes every known
gap/divergence/deviation to an explicit owner, and hardens evidence standards so the 224
failure modes cannot recur.

## 2. Current state (measured baseline)

- Branch `parity/support-planners-clean`, HEAD `5d0e2350`: 33 commits ahead of
  `origin/master` (rebased onto `8749a4af` 2026-08-21), no divergence, pushed. **All new work
  lands on this branch; merge to master only after 242's human gate is signed** (human
  decision, 2026-08-22).
- Suite state at last full measurement (HANDOFF-224-s6, `--no-fail-fast`): 3809 passed / 1
  failed / 7 ignored across 386 result lines. The one failure is
  `planner_emits_one_entry_per_region_in_region_map` in
  `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` (AC-8),
  deliberately red; resolved by Ruling 1.
- Measured vs Orca (post-AC-1-fix, artifacts in `target/review-224/`): tree 122
  `;TYPE:Support` blocks / 2 interface — matches the Orca reference exactly; traditional 121 /
  3 (count divergence = G-18). Layer counts 150 vs 452 (reference sliced at finer layer
  height). Post-fix path-length and deposited-material ratios are **unmeasured** — 236
  re-measures. The 1.58x material / 1.75x path deficit figures predate the AC-1 fix; do not
  requote them.
- Implementation inventory (all carry AGPLv3 porting headers per `docs/ORCASLICER_ATTRIBUTION.md`):
  - `modules/core-modules/tree-support-planner/src/lib.rs` (~5.9k lines; port of
    `TreeSupport.cpp`; key types `SupportPlanner`, `PlannedSupportNode`, `NodeArena`,
    `TreeVolumes`, `InterfaceRole` {Body/Roof/Floor}; functions cited throughout §12).
  - `modules/core-modules/traditional-support-planner/src/lib.rs` (~687 lines; port of
    `SupportMaterial.cpp` orchestration).
  - `modules/core-modules/tree-support/src/lib.rs` (636 lines) and
    `modules/core-modules/traditional-support/src/lib.rs` (622 lines) — the family renderers.
  - `modules/core-modules/support-surface-ironing/src/lib.rs` (277 lines).
  - Host side: `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`,
    `crates/slicer-runtime/src/builtins/support_geometry_producer.rs`,
    `crates/slicer-core/src/algos/support_geometry.rs`,
    `crates/slicer-wasm-host/src/support_aggregation.rs`.
- `OrcaSlicerDocumented/` **is on disk** at the repo root (gitignored; full OrcaSlicer history
  incl. the PrusaSlicer lineage). See Trap T1.
- Decisive fixture: `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`
  (tracked). `tmp/` currently contains (gitignored, disposable): `SupportTest.stl`,
  `SupportTest.3mf`, `SupportTest_Tree_Orca.gcode`, `SupportTest_Normal_Orca.gcode`, and the
  matched PnP profiles `tmp/support-family-config-tree-matched.json`,
  `tmp/support-family-config-normal-matched.json`, plus visual-debug requests
  `tmp/vd-geom-tree.json`, `tmp/vd-geom-normal.json`. Regenerate before relying on any of them.

## 3. Rulings made in this session (2026-08-22)

1. **AC-8: one plan entry per RegionMap region.** The host
   (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) mints
   `family_assignments` per RegionMap region, not per candidate; a region with no candidate
   receives a structured declined/empty entry, not silence. The test's count assertion stands;
   the implementation changes. Recorded as an amendment note in ADR-0059 when accepted (236).
   Root cause (verified in HANDOFF-224-s6): assignments were minted per candidate, candidates
   come from `SliceIR` regions, and `candidate_family` refuses to self-default — so a RegionMap
   region with no candidate got no family and was silently declined. **The `c3c1ed5a`
   mesh-path-gate hypothesis is DISPROVED — do not re-attempt it.**
2. **Parity bar for closure:** every gap-register row closed or human-waived in writing; the
   five behavioral axes hold (termination, coverage, collision freedom, interfaces, independent
   heights); matched-height visual/differential inspection signed off; quantitative deltas
   measured and recorded. Plus **every packet has a human validation gate** (§8).
3. **Tree styles** `smsTreeStrong`/`smsTreeHybrid` are implemented (238b), not deferred.
4. **DEV-128** (tree-planner f32 mm vs canonical scaled-integer `coord_t`): sized inside 238b;
   splits into its own packet if L. Not a closure blocker if consciously deferred with a
   recorded waiver at 238b authoring time.
5. **Feature-gap key plumbing:** intersecting keys fold into this queue (raft keys → 240;
   `bridge_no_support`/`max_bridge_length` + issue-20 support keys → 238a). Ironing keys
   (orca-feature-gap issue 22) and filament keys (issue 38) stay in the feature-gap track.
6. **Draft packets 215/216/217/218 are deleted** (inside 236); git history is the provenance.
   212-extra-perimeters-parity is not support scope and is untouched.
7. **AGG rasterizer is a port, not a research question.** Upstream history settles it:
   `OrcaSlicerDocumented` commit `fb7b995050` — *"Projection into a grid has been reworked to
   use the AGG rasterizer. This fixes #5209 and #6067… the raster is now being oversampled by
   maximum 8x8 samples and the supports are only allowed to expand inside the cell. This
   significantly reduces leakage of supports through object walls, which fixes #5054."* — and
   `a95607d7bf` fixed support columns "missing abruptly when going down" caused by
   grid-extraction contour filtering. Wall leakage is a collision-freedom defect; missing
   columns are coverage/termination defects. G-07's claim that the rasterizer does not affect
   termination, coverage, or collision freedom is **refuted**; correct it when consuming the row.
8. **Canonical behaviors land as config-selectable knobs.** PnP aims to offer more knobs than
   OrcaSlicer: where this queue replaces a *legitimate* existing PnP behavior (not a defect
   fix), canonical is the default and the prior behavior stays selectable. Instance: 241 adds
   `support_area_rasterizer` (working name, snake_case, manifest-declared), `agg` default vs
   the current semantic. Pure defect fixes (e.g. the `support_density` percent/fraction
   mis-scale) get no knob.

## 4. Domain model

No glossary changes: every term used here (support candidate, support demand, feasible support
envelope, support routing cell, support family, support body, anchored entity, sublayer,
Z-spanning print entity, degraded success, self-captured baseline, structural invariant) is
already canonical in `CONTEXT.md`.

## 5. Architecture decisions

The ten decisions of the 2026-08-12 plan stand and are embodied by packets 219–223 (global
layer as parallel work unit; anchored entities before support contracts; host analysis split
from family planning; normalized exact-Z host queries; universal structural `SupportPlanIR`;
planner+renderer selected as one family; per-region dispatch without negotiation;
validate-and-degrade before rendering; attributed rendering per anchored event; real tree and
traditional planners). Two amendments:

- **Per-region family assignment** (Ruling 1). ADR-0059 flips `proposed` → `accepted` with
  this amendment note in 236.
- **Human validation gate** is part of every packet's definition of done (§8).

## 6. Required invariants

Invariants 1–14 of the 2026-08-12 plan stand (body/nozzle-sweep disjointness from exact-Z
occupancy; accepted demands connected to eligible termination; structured decline reasons;
family-attributed emission only; pairing failure before slicing; same-family merge preserving
demand IDs; cross-family overlap drops both bodies; planar anchored output on declared Z;
Z-spanning atomicity; same-Z support in ordinary ordering; per-event optimization/accounting;
serial/parallel determinism; support-disabled emits nothing; SupportTest.stl reaches the plate
beneath the overhang for both families). Add:

15. Every RegionMap region has exactly one attributed plan entry; regions requiring no support
    carry a structured no-work/declined record (Ruling 1).
16. No acceptance command may match zero tests. Every verification command names explicit
    `--exact` test names or a filter whose matched count is asserted non-zero in the same run.
    (224 lesson: the filter token `support_family_closure` matched 0 of 306 tests, exited 0,
    and read green — the closure tests are bare `#[test] fn` wrappers registered in
    `crates/slicer-runtime/tests/integration/main.rs` delegating to
    `support_family_closure::*`; no test name carries the module prefix.)

## 7. Evidence standards (hard rules; each answers a measured failure)

- **E1. No vacuous assertions.** A test that computes a boolean and runs an empty `if`, asserts
  only artefact existence where the AC demands a judgement, or reads no fixture is a defect.
  (224's AC-2/AC-3/AC-6 were amended for exactly this; their test halves are now invariant-only.)
- **E2. Inspection ACs are inspection-only.** What requires a human/LLM to look at renders is
  satisfied by a written checklist naming source, layer, tap, and verdict — never by a test
  claiming to prove it. PNG existence, byte size, and manifest greps are not evidence.
- **E3. Self-captured baselines prove self-regression only** (ADR-0042). Golden reblessing
  requires classifying the drift first; silent regeneration is forbidden. Regeneration env
  gates: `SUPPORT_PLANNER_REGEN_GOLDEN=1` (tree planner goldens) /
  `SUPPORT_WEDGE_REGEN_GOLDEN=1` (runtime wedge). Tolerances are frozen: Hausdorff ≤ 0.5 mm,
  branch-count drift ≤ 10% — widening a tolerance is prohibited; widen the fixture's geometric
  margin or regenerate with justification recorded in `docs/DEVIATION_LOG.md`.
- **E4. Guest freshness before attribution.** `cargo xtask build-guests --check` (exit 0 fresh
  / 1 stale / 3 infra — do not grep for `STALE:`) before attributing any guest, dispatch, or
  parity failure. G-24: staleness presents as a *count divergence* (`native=128 wasm=126`),
  not an instantiation error. Parity harnesses must assert freshness before comparing (236).
- **E5. Workspace totals only from `--no-fail-fast` runs.** Fail-fast truncation twice produced
  false green totals in the 224 tail ("258 binaries / 2112 passed" skipped the whole e2e
  binary). Broad runs go through `cargo xtask test --summary` (it tees to
  `target/test-output.log`; read results from the file, never re-run to see more output).
- **E6. Feature-gated test blindness.** `slicer-core` carries 11 test targets with
  `required-features = ["host-algos"]` and most `arachne_*.rs` files gate on
  `#![cfg(feature = "host-algos")]`; a bare `cargo test -p slicer-core` compiles them to zero
  tests and prints `ok`. Use `cargo test -p slicer-core --features host-algos --no-fail-fast`;
  a binary-count drop between narrow and workspace runs means the narrow run was blind.
- **E7. OrcaSlicer reads are delegated** to a sub-agent (LOCATIONS/SUMMARY contract), cited by
  file + function, never line number. In-tree citations use symbol + crate-qualified path.
- **E8. Coordinate discipline.** 1 unit = 100 nm (mm × 10_000); divide OrcaSlicer constants by
  100. `Point2`/`Polygon`/`ExPolygon` are scaled-integer; `Point3`/`z`/layer heights are mm
  floats. 224's audit found no divide-by-100 defect in the support modules — keep it that way.
- **E9. Config keys are snake_case** everywhere (manifests already are; runtime key strings
  must match). Undeclared keys silently resolve to in-code defaults — the config view is
  filtered to the declared `config.schema` (G-16 mechanism).

## 8. Human validation gate (per packet, blocking)

Every packet's `packet.spec.md` carries a `## Human Validation Gate` section; a packet may not
flip to `status: implemented` without a sign-off line (date + verdict) recorded there. The
section names the exact artifact-producing commands, the checklist (termination, coverage,
collision freedom, interfaces, block counts vs Orca references), and artifact locations.
Minimum artifact set per geometry-touching packet: tree and traditional G-code of the tracked
`SupportTest.stl` fixture (use `tmp/support-family-config-tree-matched.json` /
`-normal-matched.json` as the matched profiles), plus visual-debug bundles for the packet's
own boundary. 239/240/242 additionally require the regenerated Orca references (§9).

## 9. Orca reference regeneration (human-owned)

The user regenerates reference G-code with their Orca install; this plan ships the settings.
Base profile: the 2026-08-18 reference profile in packet 224's `design.md` (§Orca reference
profile), under which `independent_support_layer_height` was disabled.

- **239**: tree + normal references with `independent_support_layer_height` **enabled**. The
  current references emit 150 print Z for 150 layers and cannot measure G-02; the "Orca 205 vs
  PnP 150" figure is void — never requote it.
- **240**: references with raft enabled (`raft_layers > 0`), tree + normal.
- **242**: re-confirm all references fresh. Human gates of 239/240/242 block until the named
  references exist under `tmp/`.

## 10. Supersession and disposition

- **219–223** stay `implemented`; their suites remain the regression net. As documents they
  are superseded by this queue and not re-executed.
- **224** is superseded by 242. Its amended ACs, the gap register, the parity audit, and
  `orca-divergences.md` are inherited here. `TASK-335` closes at 242.
- **215/216/217/218** are deleted in 236 (provenance: git history). Absorption mapping:
  - 215-raft-geometry → 240. Verified absent today: no `SlicedRegion.raft_fill` field, no
    `com.core.raft-default` module. Layer indices are `u32` and STAY `u32`
    (`GlobalLayer.index`, `ObjectLayerRef.local_layer_index`/`global_layer_index`,
    `SliceIR.global_layer_index`, `SupportIR.global_layer_index`; only
    `SupportPlanEntry.global_layer_index` is `i32`, retained for historical
    reasons); `LayerModule::run_infill` takes `u32`. The u32→i32 signed-index
    migration once planned here is WITHDRAWN — see the Banding decision note
    below. Note `claim:raft-fill` DOES already ship (`should_emit`,
    `crates/slicer-sdk/src/views.rs`); only the holding manifest is absent.
  - 216-support-interface-layers → behavior shipped by 220/224 (`InterfaceRole` end-to-end;
    code-1003 retired; `tree-support-planner/tests/diagnostics_tdd.rs` documents the
    replacement); residue (G-18 counts) → 238c. Do **not** resurrect the pre-families
    `SupportInterfacePlanEntry`/`SupportPlanIR.interface_plan` record design — it does not
    exist and the structural plan entries supersede it.
  - 217-support-type-variants → fully absorbed by 220 (`select_support_family` /
    `canonical_support_family` aliases in `crates/slicer-scheduler/src/execution_plan.rs` and
    `crates/slicer-ir/src/slice_ir.rs`) and 224 (F-19 auto/manual axis in
    `support_analysis_producer.rs`: `SupportType::is_auto`,
    `manual_support_type_emits_no_auto_detected_candidate`,
    `manual_support_type_emits_enforcer_driven_candidate`). Nothing remains.
  - 218-support-gcode-e2e → 242. The G-code-mode renderer exists
    (`crates/pnp-cli/src/visual_debug_gcode.rs`; tests
    `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`) and the emitter already maps
    `orca_type_label` `SupportMaterial → ";TYPE:Support"` /
    `SupportInterface → ";TYPE:Support interface"` (`crates/slicer-gcode/src/emit.rs`). The
    end-to-end evidence was never produced.
  - Same commit: update the queue rows in `docs/specs/support-generation-remediation-plan.md`
    (rows 3–6; the only live references — verified this session).
- **210a/210b/211/213/214** remain `superseded` provenance, untouched.
- **The five stubs** are promoted by this queue; each stub file is deleted as its packet is
  authored, and the gap register's destination column updated in the same commit.
- **DEV-145's premise is false** (`orca-divergences.md` squash 6): `support_bottom_interface_spacing`
  *is* canonical (`PrintConfig.cpp` declares it, default 0.5, min 0; used in
  `SupportParameters.hpp` and `TreeSupport::generate_toolpaths`). The real divergence is PnP's
  default −1.0 (mirror-top) vs canonical 0.5 mm. Correct the DEV row in 238c.
- **DEV-129** resolves in 238c: bottom-interface bands exist via `InterfaceRole::Floor`, yet
  the tree-planner manifest still says "Not yet implemented" and `diagnostics_tdd.rs` asserts
  the diagnostic. Verify current truth; close as implemented or finish — no third state.

## 11. Packet queue

Packet numbers and task IDs below are **provisional ledger facts**: re-derive at authoring
time (next free packet number via `ls docs/spec_packets/ | sort`; next free task ID in
`docs/07_implementation_status.md` — TASK-336 is taken by packet 225; do **not** reuse
TASK-324..328, which are historically claimed/collided). Numbers shown assume 236–242 are free.

| # | packet slug | goal (one sentence) | depends on | absorbs |
|---|-------------|---------------------|------------|---------|
| 236 | support-stabilization | Implement the AC-8 per-region ruling, rebless the G-23 tripwire with real collision/avoidance inputs, fix routed hygiene items, delete drafts 215/216/217/218, accept ADR-0059, gate the branch fully green. | — | handoff open-work; G-21, G-22, G-24 |
| 237 | support-analysis-parity | Make host support analysis canonical-faithful: real `needs_support` signal (G-17), enforcers under auto, the five missing `detect_overhangs` steps. | 236 | stub-support-eligibility-classification; divergences 5.2/5.3 |
| 238a | support-pattern-config-keys | Declare and wire the pattern/expansion/bottom-z/line-width config surface with canonical semantics and reconciled transports. | 236 | stub (patterns half); G-03, G-04, G-05, G-08, G-09, G-16; issue-20/37 keys |
| 238b | tree-planner-canonical-fidelity | Bring the tree planner's algorithms to canonical fidelity: top-Z gap, smoothing, role coexistence, circle fidelity, collision/avoidance keying, move semantics, tree styles; size DEV-128. | 238a | divergences 1.1–5.7/7.1–8.1; DEV-141, DEV-142, DEV-143, DEV-144 |
| 238c | support-renderer-flow-interfaces | Fix renderer flow/density/interface semantics: hollow tree walls, density scaling, over-extrusion, radius caps, roof/floor counts, base-interface role. | 238b | stub (renderer half); G-10, G-11, G-12, G-13, G-18; F-37 piece 2; DEV-129, DEV-145, DEV-146 |
| 239 | support-independent-layer-z | Implement support-layer Z independent of object-layer Z, against fresh enabled-feature Orca references. | 238c | stub; G-02 |
| 240 | support-raft | Implement raft geometry: `raft-default` synthesizer, `claim:raft-fill`, a **positive raft offset band** (raft at global layer indices `0..N-1`, model layers shifted to `N..`, matching canonical), raft keys. | 236 | stub; G-06; all of 215; issue-19/20 raft keys; DEV-124 upheld |
| 241 | support-agg-rasterizer | Port the canonical `SupportGridPattern` AGG rasterizer as a config-selectable mode, canonical by default (Rulings 7/8). | 238c | stub; G-07 |
| 242 | support-family-orca-closure | Close the sequence: register closure, invariant suite, matched-height inspection, e2e `;TYPE:` evidence, TASK-335/TASK-163b disposition, final human gate. | 237, 238a/b/c, 239, 240, 241 | supersedes 224; absorbs 218 |

Splitting rule: any packet whose implementation plan contains an L-sized step splits (238b is
the expected candidate per Ruling 4; further 238 splits are allowed — human decision
2026-08-22). 239, 240, 241 are mutually independent; order between them may vary.

## 12. Per-packet technical briefs

### 236-support-stabilization

Owned work:

- **AC-8 ruling (Ruling 1).** Change `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  so `family_assignments` are minted per RegionMap region; regions without candidates get a
  structured declined/empty entry. Target test:
  `cargo test -p slicer-runtime --test executor -- planner_emits_one_entry_per_region_in_region_map --exact`
  (file: `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs`;
  expects 2 entries for `(layer=2, object=obj-multi)` incl. region 42). The count assertion is
  not to be weakened. Do not re-attempt the disproved mesh-path-gate hypothesis.
- **G-23 tripwire.** `benchy_tree_support_regression_tripwire`
  (`modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`) currently runs with
  `SupportGeometryView { entries: vec![] }` and default occupancy — collision and avoidance
  ladders are empty at every layer, so the only tree-geometry golden exercises neither.
  Give the fixture real occupancy/avoidance/collision inputs; classify drift, then rebless
  `resources/golden/benchy_tree_support_regression_endpoints.txt` /
  `..._branch_count.txt` via `SUPPORT_PLANNER_REGEN_GOLDEN=1` (E3).
- **G-21 validator.** `validate_startup_dag` (`crates/slicer-scheduler/src/validation.rs`,
  `GlobalClaimConflicts`/`WriteConflicts`) still enforces the pre-221 single-holder rule; every
  full-directory slice emits four `ClaimConflict` advisories (`support-generator`,
  `support-planner`, `support-family:traditional`, `support-family:tree`) and two
  `WriteConflict` advisories (`SupportPlanIR`, `SupportIR`) — all expected post-221. Update the
  validator contract to recognize family-scoped multi-holder claims; silence the noise.
- **G-22 bounds.** Declare `support_threshold_angle` (and legacy alias `support_overhang_angle`)
  with `[0, 90]` in the appropriate manifest `[config.schema]`; `docs/config/host-keys.toml`
  records the range as documentation only, and `docs/15_config_keys_reference.md` was left
  stale by a past manifest deletion (`4d1848eb`) — regenerate config docs in this packet.
- **G-24 freshness assert.** The integrated-parity harnesses (slicer-runtime contract tests
  `integrated_parity_support_planner_tdd`, `integrated_parity_tree_support_tdd`,
  `integrated_parity_traditional_support_tdd`, `integrated_parity_support_surface_ironing_tdd`)
  compile native from the working tree but load wasm from on-disk artifacts; assert guest
  freshness (or invoke the `build-guests --check` fingerprint) before comparing.
- **Native/wasm view seam.** wasm builds layer views via `dispatch_layer_call` + guest shim;
  native via `build_native_layer_request` (`crates/slicer-wasm-host/src/marshal/native.rs`).
  An input added to one leg silently renders nothing (hit 3×: `85f1f889`, `ddf9dffe`,
  `with_slice_ir`). Share one construction path or add a per-stage view-equivalence test.
- **Paint fallback.** `execute_paint_segmentation`'s `matching_base.is_empty()` fallback
  (base regions were built from whole-layer all-object contours; inert on single-object
  layers, wrong in general).
- **Deletions.** Remove `docs/spec_packets/{215-raft-geometry,216-support-interface-layers,217-support-type-variants,218-support-gcode-e2e}/`;
  update rows 3–6 of `docs/specs/support-generation-remediation-plan.md` in the same commit.
- **ADR-0059** `proposed` → `accepted` with the Ruling-1 amendment note.
- **Re-measurement.** Post-fix tree/traditional path-length and deposited-material ratios vs
  the Orca references; record in the gap register (supersedes the stale pre-fix figures).
- **Green gate.** `cargo xtask test --summary --workspace --no-fail-fast` must be fully green
  (E5); plus `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo xtask check-literals` (G-15's 61 inherited violations across 34 files are pre-existing
  debt: this packet is neither blocked by nor credited with them — keep the count unchanged).
- **Human gate:** tree + traditional G-code on the tracked fixture, both inspected.

Known traps: T1, T2, T3, T4, T5, T8 (§13).

### 237-support-analysis-parity

Owned work (canonical reference throughout: `detect_overhangs`/`detect_contacts`,
`SupportMaterial.cpp`):

- **G-17 eligibility.** `classify_object` (`crates/slicer-core/src/algos/mesh_analysis.rs`)
  hardcodes `needs_support = true`; `SliceRegionView`'s `Default`/`from_ir`
  (`crates/slicer-sdk/src/views.rs`) hardcode it too — no producer ever sets false, so the
  flag has no signal. Producers set false where canonical declines; planners consume the flag.
  This reverses packet 224 decision 2's renderer-side inversion by design change (the deleted
  vacuous test `enforcer_overrides_needs_support_false` stays deleted; its replacement must
  assert real signal, per E1).
- **Divergence 5.2 (enforcers under auto).** `commit_support_analysis_builtin`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) routes through
  `enforcer_contacts` only when `!support_type.is_auto()`. Canonical `detect_contacts` runs the
  enforcer branch whenever `has_enforcer` (`annotations.enforcers_layers` non-empty), with no
  support-type gate; the `auto_normal_support` gate applies only to `detect_overhangs`'
  thresholded branch. Fix the routing and the contradicting `SupportType::NormalAuto` doc
  comment (`crates/slicer-ir/src/slice_ir.rs`).
- **Divergence 5.3 (five missing steps).** `detect_support_contacts`
  (`crates/slicer-core/src/algos/overhang_annotation.rs`) implements diff → expand-back →
  blockers → tiny-spot filter → XY expansion → union_ex and self-documents as "Not modelled":
  sharp-tail detection (`g_config_support_sharp_tails`), `buildplate_covered` subtraction
  (buildplate-only), `remove_bridges_from_contacts` under `bridge_no_support` (key from 238a),
  the post-union cantilever pass, and `enforce_support_layers` forcing `lower_layer_offset = 0`.
  Implement all five.
- Tests: `crates/slicer-core/tests/support_overhang_detection_tdd.rs` and the runtime producer
  tests — remember E6 (`--features host-algos`) for slicer-core.
- Human gate: enforcer-painted variant of the fixture if available, else a synthetic enforcer
  case documented in the packet.

Known traps: T5, T6, T8.

### 238a-support-pattern-config-keys

Owned work:

- **G-03** `support_base_pattern` / `support_base_pattern_spacing` pattern generators
  (canonical pattern stage, `SupportMaterial.cpp`; reference profile: `rectilinear`, spacing 2).
- **G-04** `support_expansion` (reference profile sets 0 — current references cannot exercise
  it; the packet must add a fixture/profile that does).
- **G-05** `support_bottom_z_distance` (reference profile 0.2; PnP honors only the top-Z
  distance — 224 design.md §RC-11).
- **G-08** `support_line_width` with canonical semantics: `coFloatOrPercent` over nozzle
  diameter, default 0 = auto (canonical derives via
  `Flow::auto_extrusion_width(frSupportMaterial, nozzle_diameter)`; PnP has no flow model —
  divergence 5.4 — so the packet decides the key-based mapping and records any deviation).
  Today: the tree planner declares a plain-mm `support_line_width` (default 0.35, min 0,
  max 2) and `crates/slicer-gcode/src/serialize.rs` carries a `support_line_width` G-code
  header field that feeds no extrusion geometry. Unify.
- **G-09 transport reconciliation.** `project_layer_plan_view`
  (`crates/slicer-wasm-host/src/marshal/in_.rs`) takes a **max** of `effective_layer_height`;
  `build_native_prepass_request` (`crates/slicer-wasm-host/src/marshal/native.rs`) takes a
  **first match** — the same run hands guests two different layer heights by transport. Pick
  one canonical rule. (224 design.md §RC-11 already prohibits dividing by the field; walk
  actual layer Z — that prohibition stands.) Also fix the `support_layer_height_mm: 0.0`
  hardcode in `crates/slicer-core/src/algos/support_geometry.rs`.
- **G-16 + divergence 3.1 key declarations.** Declare `support_branch_merge_distance_mm` and
  `support_max_branches_per_layer` in `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  (both read today but undeclared — E9's silent-default mechanism); declare `max_bridge_length`
  (canonical default 10.0, currently hardcoded `DEFAULT_MAX_BRIDGE_LENGTH_MM` in
  `sample_contact_points`); declare `support_style` (behavior lands in 238b).
- **Issue-20/37 intersecting keys:** `support_critical_regions_only`,
  `support_remove_small_overhang`, `support_threshold_overlap`, `support_object_first_layer_gap`,
  `enforce_support_layers` (behaviors consumed by 237/238b as they land); `bridge_no_support`
  (consumed by 237).
- Regenerate `docs/15_config_keys_reference.md` (gen-config-docs gate).
- Human gate: tree + traditional G-code with non-default pattern/expansion/bottom-z settings.

Known traps: T4, T5, T6, T8.

### 238b-tree-planner-canonical-fidelity

All PnP symbols live in `modules/core-modules/tree-support-planner/src/lib.rs` unless noted;
canonical citations are `TreeSupport.cpp` / `TreeSupportCommon.hpp` functions.

- **Top-Z gap (div. 1.1).** `contact_layer_after_top_gap` walks `LayerPlanViewEntry.z` in mm;
  canonical `generate_contact_points` computes `z_distance_top_layers = round_up_divide(scale_(z_distance_top), scale_(layer_height)) + 1`
  and inserts a virtual gap node (`distance_to_top = -1`); `TreeSupportSettings` uses
  `round(support_top_distance / layer_height)`. Adopt the layer-count mechanism; the mm walk
  diverges under variable layer heights.
- **Smoothing reinstatement decision (div. 2.1).** Canonical `generate_toolpaths` calls
  `smooth_nodes()` (100 iterations, max_move = `support_line_width/2`) immediately before
  `draw_circles()`. PnP removed the `smooth_branches(&mut entries_in_order, 100)` call from
  `run_support_geometry` ("do not smooth after exact-Z collision validation"); the port is
  production-dead (called only from `smooth_nodes_tdd.rs`). Decision point: reinstate smoothing
  **before** the emit-time collision gates so validation sees final geometry, or record a
  reasoned deviation. Includes **DEV-141** (`smooth_outward` vs canonical `clip_narrow_corner`)
  and **DEV-143** (canonical truncates to integer each of 100 passes; PnP relaxes in f64 and
  rounds once).
- **Role coexistence (div. 2.2).** `build_roles` runs
  `if !roof.is_empty() || !floor.is_empty() { carved.clear(); }` — a layer with any interface
  carries no body. Canonical `draw_circles` classifies each node's circle
  (`support_roof_layers_below` → roof_1st_layer/roof_base_areas/roof_areas/base_areas) and
  `diff_ex(base_areas, roofs)`; base and roof coexist, disjoint via diff.
- **Circle fidelity (div. 2.3).** `structural_body_regions` unions capsules and caps contours
  at `BRANCH_CIRCLE_SEGMENTS` (16) via `limit_contour_vertices`. Canonical:
  `CIRCLE_RESOLUTION = 100` (4 only under `SQUARE_SUPPORT` when `avg_node_per_layer > 200`);
  canonical never unions node circles into one body region.
- **Collision/avoidance keying (div. 4.1/4.2, 7.1).** Avoidance: canonical `move_nodes` calls
  `get_avoidance(next_radius, …)` with the per-node tapered radius; PnP's
  `volumes.ensure_avoidance(branch_radius)` uses one constant `tree_support_branch_diameter / 2`.
  Collision: canonical `get_collision(radius, l)` bakes radius into the volume + point-in test;
  PnP reads `get_collision(0.0, l)` and adds radius at test time via `body_intersects` /
  `body_overlaps_occupancy` (point-in plus distance-to-contour disc; documented as the F-13
  interim). Move to radius-baked volumes + point-in, and replicate
  `avoid_object_remove_extra_small_parts`'s largest-part selection in the carve (div. 7.1 —
  PnP currently keeps all surviving parts via `swallowed_by_collision`/`node_swallowed` +
  `build_roles` carve).
- **Miter limits (div. 3.3/4.3).** `sample_contact_points` erodes with
  `OffsetJoinType::Miter, 0.0`; TreeVolumes offsets route through `slicer_core::polygon_ops::offset`
  → `inflate_once` at miter 2.0 (Clipper2 default). Canonical uses `offset_ex` defaults
  (`jtMiter`, `DefaultMiterLimit = 3.0`) at both sites. The host offset path
  (`host::offset_polygons`) exposes no miter-limit parameter — add one.
- **TreeVolumes ctor (div. 4.4/4.5).** `TreeVolumes::new` stores raw `SupportGeometryView`
  outlines; canonical simplifies each layer's `lslices` at `scale_(m_radius_sample_resolution)`
  and builds `m_layer_outlines_below` from the simplified outlines. `expolygons_simplify` drops
  canonical `ExPolygon::simplify`'s final `union_ex` (which can merge a hole into the contour
  or split an expolygon).
- **`to_buildplate` inflation (div. 4.6/5.6).** `push_contact_with_demand` /
  `push_analysis_contact` and the branch-A merged-node path test raw outlines
  (`point_in_any_expoly(volumes.outlines_at(...))`); canonical uses
  `!is_inside_ex(get_collision(0, obj_layer_nr), position)` (xy-distance-inflated). **Exception,
  do not "fix":** the F-14 per-descendant recompute correctly uses raw outlines — canonical's
  move pass uses `m_layer_outlines` there.
- **`move_out_expolys` (div. 5.1).** PnP projects onto the ORIGINAL ring, steps analytically,
  and aborts (returns the original point) on budget exceed. Canonical computes
  `polys_dilated = union_ex(offset_ex(polygons, scale_(distance)))`, projects onto the DILATED
  ring, and clamps to `pt_max = from + normal(outward_dir, scale_(max_move_distance))`,
  returning bool. The in-tree comment claiming canonical restores `from0` is false — fix the
  code and the comment. Affects the branch-A group-0 push-out, the STUDIO-4252 retry, and the
  F-13 move-pass escape.
- **STUDIO-4252 retry args (div. 7.2).** The F-13 move pass calls
  `move_out_expolys(&collision_next, pos, RADIUS_SAMPLE_RESOLUTION_MM + CANONICAL_EPSILON_MM, max_move + RADIUS_SAMPLE_RESOLUTION_MM + CANONICAL_EPSILON_MM)`;
  canonical `drop_nodes` computes `max_move_between_samples = max_move_distance + radius_sample_resolution + EPSILON`
  and passes it as BOTH the dilation and the max-distance argument.
- **Mesh-path overhang shim (div. 3.2).** `plan_for_object` builds per-layer `ExPolygon`s by
  projecting overhang triangles downward (self-acknowledged legacy shim for coplanar-plate
  fixtures); canonical samples `layer->loverhangs` (host-computed per-layer overhang polygons).
  Replace with host-computed polygons where the host contract allows; otherwise record the
  boundary precisely.
- **Branch-A roof counter (div. 5.5).** PnP seeds the merged node's `support_roof_layers_below`
  with `max(id, nid)` minus decrement. Canonical `drop_nodes` branch A uses the PARENT's
  counter (`node_parent->support_roof_layers_below - (node_parent->distance_to_top >= 0 ? 1 : 0)`);
  `insert_dropped_node`'s max-merge is the same-position dedup path, not branch A.
- **Tree styles (div. 5.7, Ruling 3).** Implement `is_strong` (unweighted neighbor sums;
  `movement = direction_to_outer + move_to_neighbor_center` with dot-product gate) and hybrid
  (`TreeNodeType::Polygon` minting — never minted today — with its own merge/move handling),
  keyed by the `support_style` manifest key from 238a.
- **Emit simplify gating (div. 8.1 = DEV-142).** `build_roles` runs `expolygons_simplify` at
  `DRAW_CIRCLES_RESOLUTION_MM` (0.0125 mm) on every role region (body, roof, floor) before the
  carve. Canonical `draw_circles` simplifies only `base_areas`, only under `SQUARE_SUPPORT`
  (`avg_node_per_layer > 200`), at `scale_(line_width / 2)` (~0.2 mm); the later
  `diff_ex(base_areas, trimming)` is the bottom-Z clearance trim via `get_trim_support_regions`,
  not a collision re-diff. In the normal case canonical emits unsimplified 100-vertex circles.
  Gate the simplify to the canonical condition (and correct the in-tree comment's canonical
  justification, which does not match the local checkout).
- **DEV-144.** `need_extra_wall` is computed per node but degrades to a per-layer capability
  string — add per-node transport through `SupportPlanIR` so the extra wall is printed.
- **DEV-128 sizing (Ruling 4).** Size the f32→`coord_t` retype of this ~5.9k-line planner
  (canonical `SupportNode::position` is `Point`/`coord_t`); split into its own packet if L;
  record a waiver if deferred.
- Tests: `modules/core-modules/tree-support-planner/tests/` — `tree_family_tdd.rs`,
  `smooth_nodes_tdd.rs`, `multi_neighbour_mst_tdd.rs`, `to_buildplate_tdd.rs`,
  `wall_clearance_tdd.rs`, `diagnostics_tdd.rs`, plus the strengthened `orca_parity_tdd.rs`
  tripwire (236). Run `cargo xtask build-guests --check` first (T4); the planner is a guest
  WASM, so crate-suite green can hide real-geometry regressions (the G-23 lesson: 76/0 green
  coexisted with a planner producing no usable support on real meshes — real-mesh validation
  is mandatory, not optional).
- Human gate: tree G-code on the fixture plus one non-coplanar real-mesh case.

Known traps: T1, T4, T5, T6, T7, T8.

### 238c-support-renderer-flow-interfaces

- **G-10/G-11 density and flow.** `render_polygon` (`modules/core-modules/tree-support/src/lib.rs`)
  renders branch bodies **filled**, with `spacing = line_width / density.min(1.0)` from the
  `support_density` key — which arrives as `20.0` (percent) and is consumed as a fraction, so
  the clamp forces 100% solid above 1. Canonical `TreeSupport::generate_toolpaths` renders
  **hollow concentric walls** and derives body density from
  `min(1., support_flow.spacing() / support_spacing)` with
  `support_spacing = support_base_pattern_spacing + support_flow.spacing()`, and interface
  density analogously — no `support_density` percentage key exists for tree support. Measured
  over-extrusion: PnP flow per path-mm is 1.107x Orca's (G-11). Fix both the geometry model
  and the scale; the density-spacings keys come from 238a.
- **G-12** `MAX_BRANCH_RADIUS_MM = 6.0` → canonical 10.0 (`tree-support-planner`).
- **G-13** missing canonical raise-to-`base_radius` when `support_interface_top_layers > 0`
  (reference profile sets 2 — active in every current reference).
- **G-18 roof/floor counts.** At `support_interface_top_layers = 2` /
  `support_interface_bottom_layers = 2`, PnP traditional emits 2 `;TYPE:Support interface`
  blocks vs Orca's 3 (placement correct in both; pinned by `interface_layer_count_follows_config`,
  commit `ee27ac94`). Implement canonical roof/floor band structure
  (`SupportParameters.hpp` `number_of_support_interface_bottom_layers`;
  `TreeSupport.cpp::draw_circles` floor block).
- **F-37 piece 2 (base-interface role).** `num_top_base_interface_layers` role: ~10 files,
  2 WIT edits, 1 schema bump, new `ExtrusionRole` + `;TYPE:` marker decision; canonical
  derivation recorded in commit `050d5c3a`. WIT discipline: edit canonical sources at
  `crates/slicer-schema/wit/` (both host `bindgen!` and guest `include_str!` read them); the
  three support worlds are `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`,
  `.../layer-support-postprocess/layer-support-postprocess.wit`,
  `.../prepass-support-geometry/prepass-support-geometry.wit`; `cargo build --tests` after WIT
  changes, then rebuild guests.
- **`interface_regularize` consolidation.** Two copies:
  `modules/core-modules/tree-support/src/interface_regularize.rs` and
  `modules/core-modules/traditional-support/src/interface_regularize.rs` → one shared
  implementation. (DEV-127 records the broader three-copy scan-line drift including
  `rectilinear-infill` — this packet owns only the support-side pair.)
- **DEV-145 correction.** `support_bottom_interface_spacing` is canonical (see §10); change
  the default from −1.0 (mirror-top) to canonical 0.5 mm and correct the DEV row. Manifests:
  `modules/core-modules/traditional-support/traditional-support.toml`,
  `modules/core-modules/tree-support/tree-support.toml`.
- **DEV-146.** Interface pitch derives from generic `line_width`; canonical uses the interface
  flow width. Latent — no interface-width key exists; this packet adds it (coordinate with
  238a's `support_line_width` semantics).
- **DEV-129 resolution.** See §10 — verify, then close or finish; no third state.
- Renderers under test: `tree-support/src/lib.rs` (636 lines), `traditional-support/src/lib.rs`
  (622 lines); suites `modules/core-modules/tree-support/tests/tree_support_tdd.rs`,
  `modules/core-modules/traditional-support/tests/{traditional_support_tdd,support_fill_geometry_tdd}.rs`.
- Human gate: tree + traditional G-code; verify interface block counts against the references.

Known traps: T4, T5, T6, T8.

### 239-support-independent-layer-z

- **G-02.** Blockers (both verified): `is_same_z_entity`'s on-grid filter
  (`crates/slicer-runtime/src/layer_executor.rs`) excludes off-grid entities, and
  `crates/slicer-runtime/src/pipeline.rs` never calls
  `execute_per_layer_with_anchored_events`. Unverified risk: `height_delta`
  (`crates/slicer-gcode/src/emit.rs`) is computed per layer and may mis-scale flow for
  off-grid entities — **measure first**, then fix if real.
- The 219 anchored substrate stands: regression test
  `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact`.
- Requires the enabled-feature Orca references (§9) before the human gate.
- Human gate: matched-height comparison of an enabled-feature slice vs the fresh references.

Known traps: T1, T4, T5.

### 240-support-raft (split into 240a + 240b)

> Split at preflight. The substrate half (the `GlobalLayer.is_raft` marker and
> its WIT marking, the positive raft band emission, the object-bottom predicate
> audit, the `SlicedRegion.raft_fill` carrier, the `raft-plan` read accessor) is
> `docs/spec_packets/240a-support-raft-substrate`; the consumer half
> (`com.core.raft-default`, the `generate_raft_base` port, the raft keys,
> the ADR-0009 amendment, the human gate) is
> `docs/spec_packets/240b-support-raft-module`. 240b hard-depends on 240a.

- **All of 215's scope + G-06** (canonical reference `SupportCommon.cpp::generate_raft_base`;
  rafts occupy a **positive offset band** — global layer indices `0..N-1` where
  `N = support_raft_layers`, with model layers shifted to `N..` — and are never
  anchored entities):

  > **Banding decision, corrected 2026-09-04.** Earlier revisions of this plan
  > specified *signed negative* layer indices (`-N..-1`) for the raft band and
  > attributed that contract to ADR-0009. Both were wrong. ADR-0009 concerns
  > where raft *pattern algorithms* live and mentions no layer index or
  > signedness; its Status is `Proposed`. And canonical does the opposite of
  > what was claimed: `generate_support_layers` (`SupportCommon.cpp`) appends
  > raft layers at strictly positive print_z in `[0, object_print_z_min]`,
  > sorts by print_z, and assigns a dense non-negative counter, while object
  > `Layer` ids start at `slicing_parameters().raft_layers()` in `new_layers`
  > (`PrintObjectSlice.cpp`). The positive band therefore matches canonical,
  > needs no `u32`->`i32` migration of the layer-index surface, and upholds
  > DEV-124's shipped `layer_index == support_raft_layers` clamp instead of
  > reopening it. The consequence to respect everywhere: **the first printed
  > model layer is `support_raft_layers`, not `0`.**
  - New module `com.core.raft-default` (`Layer::Infill` synthesizer) holding `claim:raft-fill`;
    reads `SupportPlanIR.raft_plan`, `SliceIR`, `LayerPlanIR`; writes the new
    `SlicedRegion.raft_fill`; deterministic rectilinear rendering.
  - Raft marker: `GlobalLayer.is_raft: bool` (`#[serde(default)]`,
    `crates/slicer-ir/src/slice_ir.rs`) set from a new WIT
    `layer-proposal.is-raft-prefix: bool`, plus an `is-raft` accessor on
    `paint-region-layer-view` so a `Layer::Infill` guest can see it. Layer
    indices stay `u32`; no signed-index migration.
  - Existing transport to reuse: `RaftPlan` (`crates/slicer-ir/src/slice_ir.rs`) is produced by
    the tree planner (`push_raft_plan`) and already flows through SDK
    (`crates/slicer-sdk/src/prepass_builders.rs`), the macro (`crates/slicer-macros/src/lib.rs`),
    host (`crates/slicer-wasm-host/src/host.rs`), marshal (`in_.rs`, `native.rs`), and
    blackboard merge (`crates/slicer-runtime/src/blackboard.rs` `raft_plan_min`).
- **Issue-19/20 raft keys:** `raft_contact_distance`, `raft_expansion`,
  `raft_first_layer_expansion`; wire the existing dead raft keys in the four support modules
  or record why each stays dead.
- **DEV-124 check** (`only_one_wall_first_layer` fires on the wrong layer under a raft) while
  the raft path is open.
- Requires raft-enabled Orca references (§9). Human gate: raft slice inspection + reference
  comparison.

Known traps: T1, T4, T5, T8.

### 241-support-agg-rasterizer

- Port `SupportGridPattern` (`SupportMaterial.cpp`): AGG antialiased scanline rasterizer over a
  byte grid, ≤8x8 oversampling with expansion restricted inside the cell, 4-direction seed
  fill, marching-squares contour extraction. Target: the traditional planner's area
  propagation, replacing-as-default the current propagate-without-growth semantic (trim per
  layer at `support_object_xy_distance`).
- **Ruling 8 knob:** `support_area_rasterizer` (working name, snake_case) in the
  `traditional-support-planner` manifest: `agg` (canonical, default) vs the legacy semantic;
  both paths tested; parity evidence runs the default.
- Justification: Ruling 7's upstream history. Correct G-07's premise in the register and
  consume the stub in the same commit.
- Acceptance evidence: before/after wall-leakage (collision freedom) and column-continuity
  (coverage) measurement against the Orca references — measurement as gate.
- Human gate: traditional G-code on the fixture, both rasterizer modes.

Known traps: T1, T4, T5, T7.

### 242-support-family-orca-closure

- **Inherited 224 ACs** (as amended 2026-08-17/18): fixture invariants; matched-height
  evidence (artefact-presence precondition only — the inspection itself is recorded in writing
  per E2); differential evidence (PnP-side structural invariants + recorded inspection);
  final G-code roles; supersession records; TASK-163b disposition. Verification form (E1 +
  invariant 16): `cargo test -p slicer-runtime --test integration -- fixture_invariants family_reaches_region_routing invalid_geometry_fails matched_height_evidence differential_evidence final_gcode_roles supersedes_packet_213_and_task_329 task_163b_disposition --exact`
  and confirm a non-zero pass count. The `support_test_path` resolver panic contract (AC-N2)
  is the fixture-absence gate — no dedicated missing-fixture test (it was deleted for asserting
  `std::fs` behavior; do not recreate it).
- **218 absorption:** e2e G-code-mode visual-debug evidence that `;TYPE:Support` /
  `;TYPE:Support interface` markers coexist with final-G-code layer images, using
  `crates/pnp-cli/src/visual_debug_gcode.rs`; extend
  `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` (which today uses only inline
  Outer wall/Solid infill fixtures) with a support-marked case.
- **Register closure:** every G-row closed or human-waived in writing; DEV-141..146 closed or
  carried with corrected premises; every `orca-divergences.md` row dispositioned.
- **Supersession records:** 213/`TASK-329`, the deleted drafts 215/216/217/218, and 224 itself.
  `TASK-335` closes here, and only here.
- **Final human gate:** full differential inspection of both families against fresh references
  (§9), plus the whole-suite green run.

Known traps: all of §13.

## 13. Known traps (each one already happened — do not repeat)

- **T1. "The Orca checkout doesn't exist."** `OrcaSlicerDocumented/` is gitignored, so glob
  tooling and `git ls-files` miss it. Verify by direct listing
  (`ls OrcaSlicerDocumented/src/libslic3r/Support/`). A shallow search once told five workers
  no checkout existed; three defects trace to that false premise (F-33 wrong `smooth_nodes`
  kernel, F-37 arc-fillet resampler, dual contact seeding missing canonical's `layer_nr - 1`
  shift). The same gitignore blindness applies to `tmp/` — the Orca reference G-code and
  matched-config JSONs exist there even when globs report nothing.
- **T2. Zero-test green.** A filter that matches no test exits 0 and prints `ok`
  (`support_family_closure` matched 0 of 306). Always `--exact` names or assert the matched
  count; confirm `N passed` is non-zero before believing any run (invariant 16).
- **T3. Fail-fast totals.** `cargo test --workspace` without `--no-fail-fast` truncates;
  reported totals were wrong twice. Broad runs: `cargo xtask test --summary --workspace -- --no-fail-fast`-style
  invocations; read `target/test-output.log`, never re-run for more output.
- **T4. Stale guests.** Guest `.wasm` artifacts are not rebuilt by `cargo build`/`cargo test`.
  Run `cargo xtask build-guests --check` (exit codes 0/1/3) before attributing failures.
  Staleness can present as a geometry count divergence (G-24: `native=128 wasm=126`), not an
  instantiation error. Note: `modules/core-modules/tree-support-planner/` carries two wasm
  artifacts (`support-planner.wasm` legacy + `tree-support-planner.wasm`).
- **T5. Feature-gated blindness.** Bare `cargo test -p slicer-core` skips the `host-algos`
  suite silently (11 targets + `#![cfg]` files). Use `--features host-algos`; reconcile binary
  counts against the workspace run.
- **T6. Vacuous ACs / self-captured oracles.** Tests asserting artefact existence where a
  judgement is required; booleans feeding empty `if`s; goldens renamed `*_orca_*` that never
  contained Orca data. E1/E2/E3 apply. The current goldens are honestly named
  (`benchy_tree_support_regression_*`); keep it that way.
- **T7. Planner green ≠ real-mesh correctness.** The tree planner's crate suite was 76/0 green
  while emitting empty/near-empty plans on real meshes (G-23 mechanism). Every planner packet
  validates on the tracked fixture AND at least one non-coplanar real mesh.
- **T8. Silent config defaults.** A module's config view is filtered to its declared
  `config.schema`; undeclared keys silently resolve to in-code defaults (G-16). When adding a
  key: manifest `[config.schema]` entry + regenerate `docs/15_config_keys_reference.md` in the
  same commit (a past deletion, `4d1848eb`, left the doc stale).
- **T9. Native/wasm leg skew.** An input added to only one of the two layer-view construction
  paths (wasm `dispatch_layer_call` + guest shim vs native `build_native_layer_request`)
  silently renders nothing. Hit 3× in the 224 tail (`85f1f889`, `ddf9dffe`, `with_slice_ir`).
- **T10. Pre-existing noise is not your defect.** `ERR_MALFORMED_LAYER_MARKER` fires ~110×/run
  from `machine-gcode-emit` with support disabled (G-14); the 61 `check-literals` violations
  across 34 files predate this work (G-15). Do not re-diagnose either as a support defect; do
  not claim credit for fixing them.
- **T11. Disproved hypotheses.** The AC-8 mesh-path-gate hypothesis (`c3c1ed5a`) is disproved.
  DEV-145's premise (`support_bottom_interface_spacing` is PnP-invented) is false. The "Orca
  205 vs PnP 150 print-Z" figure is void (references had the feature disabled). The pre-AC-1-fix
  1.58x/1.75x deficit figures are stale. Do not resurrect any of these.

## 14. Packet authoring rules (for the fresh session)

1. Use the spec-packet generator's Batch Protocol; author in dependency order (236 → 237 →
   238a → 238b → 238c → 239 → 240 → 241 → 242; 239/240/241 mutually independent).
2. Allocate fresh task IDs in `docs/07_implementation_status.md` at authoring time (re-derive
   the next free ID; never reuse TASK-324..328 or unrelated closed IDs).
3. Re-ground every load-bearing IR/WIT/stage/scheduler/raft/visual-debug symbol against the
   live tree before authoring each packet; names above describe approved contracts and were
   verified 2026-08-22, not guaranteed to exist unchanged.
4. Delegate all OrcaSlicer source inspection (LOCATIONS/SUMMARY contract); verify
   `OrcaSlicerDocumented/` by direct listing first (T1).
5. Apply §7 evidence standards to every packet; every verification command satisfies invariant
   16 (non-zero matched tests); every geometry-touching packet includes model-backed
   visual-debug taps for its own new boundary.
6. Consume each stub file as its packet is authored; update the gap register destinations in
   the same commit.
7. Doc hygiene per packet: this file's queue table, `docs/specs/support-generation-remediation-plan.md`,
   `docs/DEVIATION_LOG.md` (corrections like DEV-145; run `cargo xtask check-deviations`),
   `docs/07_implementation_status.md`, and `docs/15_config_keys_reference.md` when manifests
   change. `cargo xtask check-literals` before every closure (T10: the 61 inherited violations
   are not yours).
8. Keep every commit on `parity/support-planners-clean`; merge to master only after 242's
   human gate is signed.
9. Each packet gets: the standard four files + `task-map.md`, a `## Human Validation Gate`
   section (§8), a context-discipline note, and an OrcaSlicer-reference obligations snippet
   naming the canonical files/functions it will inspect.

## 15. Out of scope

- Ironing keys (orca-feature-gap issue 22) and filament keys (issue 38) — feature-gap track.
- Nonlinear perimeters, non-planar walls, milling, inspection, or other future anchored-entity
  producers; a global cross-layer scheduler; planner-to-planner negotiation.
- Exact Orca toolpath identity. Behavioral parity, measured deltas, and collision-safe
  printable geometry are the bar.
- Replacing raft-prefix layers with anchored entities (the raft band stays a
  positive `0..N-1` global-layer range; anchored entities remain prohibited for it).
- Silent clipping of invalid family geometry, emergency fallback fillers, or allowing model
  collisions to preserve support coverage.
- 212-extra-perimeters-parity (not support).
- G-14 (`ERR_MALFORMED_LAYER_MARKER` noise) and G-15 (inherited literal debt) — recorded so
  they are never re-diagnosed as support defects (T10).
- G-20 (`erSupportTransition`): register-only per prior human decision; revisit only if a
  producer appears. Note interaction: 238c's F-37 piece 2 adds a *different* new
  `ExtrusionRole` (base-interface) — do not conflate the two.

## Packet Queue

Live tracking table for the spec-packet-generator Batch Protocol. Goals abbreviated — §11
and the §12 briefs remain authoritative. Task IDs are allocated fresh from
`docs/07_implementation_status.md` at each packet's authoring time (never TASK-324..328);
the column records what was allocated.

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | support-stabilization | AC-8 per-region ruling, G-23 tripwire rebless, G-21/G-22/G-24 hygiene, delete drafts 215–218, accept ADR-0059, branch fully green. | TASK-344..TASK-352 | - | generated | docs/spec_packets/236-support-stabilization |
| 2 | support-analysis-parity | Canonical-faithful host analysis: real `needs_support` signal (G-17), enforcers under auto, five missing `detect_overhangs` steps. | TASK-353..TASK-362 | #1 | generated | docs/spec_packets/237-support-analysis-parity |
| 3 | support-pattern-config-keys | Declare/wire pattern/expansion/bottom-z/line-width config surface with canonical semantics and reconciled transports. | TASK-363..TASK-368 | #1 | generated | docs/spec_packets/238a-support-pattern-config-keys |
| 4 | tree-planner-canonical-fidelity | Tree planner algorithms to canonical fidelity (top-Z gap, smoothing, roles, circles, keying, moves, styles); size DEV-128. | TASK-369..TASK-380 | #3 | generated | docs/spec_packets/238b-tree-planner-canonical-fidelity |
| 5 | support-renderer-flow-interfaces | Renderer flow/density/interface semantics: hollow walls, density scale, radius caps, roof/floor counts, base-interface role. | TASK-381..TASK-398 | #4 | generated | docs/spec_packets/238c-support-renderer-flow-interfaces |
| 6 | support-independent-layer-z | Support-layer Z independent of object-layer Z, against fresh enabled-feature Orca references. | TASK-399..TASK-408 | #5 | generated | docs/spec_packets/239-support-independent-layer-z |
| 7a | support-raft-substrate | `GlobalLayer.is_raft` marker + WIT marking, positive raft band emission, object-bottom predicate audit, `raft_fill` carrier, `raft-plan` + `is-raft` read accessors. | TASK-409..TASK-413, TASK-533..TASK-536 | #1 | generated | docs/spec_packets/240a-support-raft-substrate |
| 7b | support-raft-module | `raft-default` synthesizer, `claim:raft-fill`, `generate_raft_base` port, raft keys, ADR-0009 amendment, human gate. | TASK-414..TASK-418, TASK-537 | #7a | generated | docs/spec_packets/240b-support-raft-module |
| 8 | support-agg-rasterizer | Port the canonical AGG rasterizer as config-selectable mode, canonical by default (Rulings 7/8). | TASK-419..TASK-428 | #5 | generated | docs/spec_packets/241-support-agg-rasterizer |
| 9 | support-family-orca-closure | Close the sequence: register closure, invariant suite, matched-height inspection, e2e `;TYPE:` evidence, TASK-335 disposition, final human gate. | TASK-429..TASK-440 | #2,#3,#4,#5,#6,#7,#8 | generated | docs/spec_packets/242-support-family-orca-closure |
| 10 | support-plan-ownership-seam | Region ownership enforced at the host support-plan merge point: declared-identity union key, default-deny check against `family_assignments` + producer claim, arrival order deleted, DEV-167 closed, packet-239 tests restored. | TASK-531 | #9, packet 241 | generated (implemented) | docs/spec_packets/241b-support-plan-ownership-seam |
