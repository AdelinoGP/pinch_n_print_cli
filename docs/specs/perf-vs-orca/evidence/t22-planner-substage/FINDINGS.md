# Ticket 22 — Tree-planner substage attribution

Wayfinder ticket: [Tree-planner substage attribution](../../issues/22-tree-planner-substage-attribution.md).
Attribution only (no optimization authorized, no wall claims): every number
below is a probe-scoped measurement under `--instrument-stderr`, per the map's
recipe traps (§3.2/§3.3).

## Method and probe provenance

Temporary, `PERF-T22-PROBE`-tagged brackets, removed before any commit:

- **Guest side** — `modules/core-modules/tree-support-planner/src/lib.rs`
  (`T22Span`/`T22CallGuard`). `host::now_us()` is the host clock for the
  current module call, so deltas are call-local microseconds. The planner is
  dispatched once per slice (`module_complete` calls = 1), so the guest is
  single-threaded within the bracket and the sums are honest serial wall on
  that one call. Flushed as one `module_log` line
  (`t22-planner-split <name>=<us>us/<calls> ...`).
- **Host side** — `crates/slicer-wasm-host/src/perf_t22_probe.rs`, brackets in
  `host.rs` (`offset_polygons`, `offset_polygons_batch`, `clip_polygons`,
  `clip_polygons_batch`, `simplify_polygon_batch`), `exact_z_query.rs`
  (cache hits/misses + cross-section wall), and `support_aggregation.rs`
  (whole call + `validate_entry` / same-family union / territory clip /
  cross-family overlap sub-terms). Reset and flushed from
  `crates/slicer-runtime/src/prepass.rs` around exactly the
  `PrePass::SupportGeometry` stage. One JSON line per stage.

The singular `clip_polygons` bracket in `HostExecutionContext::clip_polygons`
(`crates/slicer-wasm-host/src/host.rs`) starts **after** WIT→IR conversion and
ends **before** IR→WIT conversion; its recorded wall is the core boolean call,
not the entire host import or the guest↔host round trip. Other brackets have
their own boundaries (the singular offset bracket includes conversions).

Units and scope: the guest figures are **wall inside one module call**
(microseconds), the host figures are **wall inside one stage's host-side
work** (milliseconds). They are different scopes and are never summed. Host
figures include every thread that runs inside the bracket; on the planner
stage that is the calling thread plus any rayon fan-out the batch services
perform.

The planner takes no `slicer-core` dependency (`modules/core-modules/tree-support-planner/Cargo.toml`;
`slicer-core` reaches the guest only through `slicer-sdk`, whose polygon ops
route to host imports on wasm32 since packet 200), so the controlled
accelerated build cannot change this stage's cost. Both fixtures use
`pnp-classic-supports-on.json`; the planner does not read `wall_generator`, so
one configuration per fixture is enough.

## Captures

| Capture | Fixture | Config | Notes |
| --- | --- | --- | --- |
| `benchy-final` | `tmp/3dbenchy.stl` | `pnp-classic-supports-on.json` | complete bracket set |
| `base-final` | `tmp/base.stl` | `pnp-classic-supports-on.json` | complete bracket set |
| `base` (earlier) | `tmp/base.stl` | same | pre-`p6d/e/f` bracket set; corroborates the shared terms |

Reproduction (stderr keeps the probe lines):

```bash
cargo xtask build-guests --check          # must exit 0
target/release/pnp_cli.exe slice --model <fixture> \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
  --module-dir modules/core-modules --output <scratch>/out.gcode \
  --instrument-stderr 2> <scratch>/events.jsonl
python docs/specs/perf-vs-orca/evidence/t22-planner-substage/t22_split.py <scratch>/events.jsonl
```

## The split (measured)

### Guest — `com.core.tree-support-planner`, one call per slice

Microseconds of wall inside the single module call.

| phase | benchy (µs) | base (µs) | what it is |
| --- | ---: | ---: | --- |
| `p0_volumes_new` | 42,616 | 520,925 | `TreeVolumes::new` — outline stack, foreign keepout, below-union ladder |
| `p2_contacts` | 319,211 | 2,178,219 | contact seeding (mesh + analysis candidates) |
| `p3_drop_nodes_loop` | 414,214 | 5,247,652 | the top-down per-layer drop/move/MST loop |
| `p3c_collision_ladder` | 136,430 / 27 | 756,924 / 49 | lazy per-radius collision ladders (batched host offsets) |
| `p3d_avoidance_ladder` | 277,876 / 16 | 3,510,262 / 38 | lazy per-radius avoidance recurrences (serial host clips) |
| `p4_f14_and_erase` | 292 | 1,855 | `unsupported_branch_leaves` drain + `erase_if(is_processed)` |
| `p5_smooth_nodes` | 18,058 | 121,685 | `smooth_nodes` |
| **`p6_emit_pass`** | **9,484,857** | **262,039,495** | **`draw_circles` emit pass** (sub-split below) |
| `p6d_inflate_occupancy` | 2,268,224 / 365 | 61,369,697 / 864 | model-occupancy inflation, one host offset per (layer, region) |
| `p6e_node_roles` | 91,468 / 186 | 1,497,055 / 433 | per-node floor point-in tests + roof ancestry walk |
| `p6f_node_ellipses` | 120,342 / 186 | 554,200 / 433 | per-node ellipse construction + swallow gate |
| `p6a_carve_per_region` | 4,249,304 / 716 | 122,753,911 / 1,724 | per-region collision carve (one host clip per region) |
| `p6c_union_simplify` | 2,430,656 / 716 | 72,764,193 / 1,724 | `union_expolys` + simplify of the carved regions |
| `p6b_final_difference` | 266,150 / 716 | 2,352,289 / 1,724 | post-simplify set-wide collision difference |
| `p7_stamp_and_interpolate` | 4,973 | 33,266 | template stamp + 239c/239d interpolation |
| `p8_push_entries` | 8 | 20 | push every entry into the output builder |
| `p1_plan_for_object` (encloses p2…p8) | 10,244,424 | 269,639,934 | the whole per-object planner |
| `module_complete` (whole dispatch) | 10,357 | 270,691 | bracket including WIT marshalling |

Carve classes counted in-guest over the same run (a bbox-disjoint pre-test's
two classes): benchy `carve_disjoint=11,018`, `carve_overlap=10,694`; base
`carve_disjoint=78,832`, `carve_overlap=69,161`.

### Host — `PrePass::SupportGeometry` stage

Host-side wall inside the stage bracket (milliseconds).

| term | benchy | base |
| --- | ---: | ---: |
| singular `offset_polygons` | 2,702.08 / 4,977 calls | 65,077.57 / 23,535 calls |
| batched `offset_polygons_batch` | 59.43 / 28 batches (27 parallel) | 509.12 / 50 batches (49 parallel) |
| **singular `clip_polygons`** | **6,631.30 / 29,000 calls** | **194,039.62 / 171,636 calls** |
| — bbox-disjoint class | 1,775.18 / 12,237 calls | 52,588.67 / 79,369 calls |
| — bbox-overlap class | 4,856.13 | 141,450.94 |
| batched `clip_polygons_batch` | 0 | 0 |
| `simplify_polygon_batch` | 0 | 0 |
| exact-Z cache hits / misses | 0 / 179 | 0 / 431 |
| exact-Z cross-section wall | 680.92 | 21,135.11 |
| aggregation (whole call) | 1,000.77 | 23,809.56 |
| — of which `validate_entry` | 993.77 | 23,767.46 |
| — union / territory / overlap | 0.07 / 0 / 0.05 | 0.37 / 0 / 0.49 |
| stage `stage_complete` | 11,419 | 295,032 |

Note on cross-capture drift: the two base captures (2026-09-24) differ
materially on host terms (`clip` 146.8 s vs 194.0 s; `exact_z` 27.4 s vs
21.1 s) with identical host-probe coverage and class shares (46.2% disjoint
calls in both; the in-guest carve counts are byte-identical at 78,832/69,161).
The final capture additionally brackets `p6d/e/f` in the guest, so these are
not identical instrumented builds. External load is a possible cause (§10.3/
§3.6), not a demonstrated explanation. Both tables quote the respective
`benchy-final` and `base-final` captures; absolute walls drift even at the
same call counts and should not be promoted to uninstrumented wall claims.

## Reading

**The emit pass owns the planner.** On benchy, `p6_emit_pass` = 9.48 s of the
10.24 s per-object planner (92.6%); on base, 262.0 s of 269.6 s (97.2%). Every
named phase before it is under 0.5 s on benchy and under 5.3 s on base. This
replaces the 2026-09-05-era collision-cache lead (ADR-0049's 98.0%) as the
planner's cost centre: the ladders are still built (`p3c`/`p3d`, inside the
base drop loop's 5.2 s, 0.41 s on benchy), but the matched job's time is
elsewhere.

**Inside the emit pass, the closure is near-total:**

| sub-term | benchy (µs) | share | base (µs) | share |
| --- | ---: | ---: | ---: | ---: |
| `p6d_inflate_occupancy` | 2,268,224 | 23.9% | 61,369,697 | 23.4% |
| `p6a_carve_per_region` | 4,249,304 | 44.8% | 122,753,911 | 46.8% |
| `p6c_union_simplify` | 2,430,656 | 25.6% | 72,764,193 | 27.8% |
| `p6b_final_difference` | 266,150 | 2.8% | 2,352,289 | 0.9% |
| `p6e_node_roles` | 91,468 | 1.0% | 1,497,055 | 0.6% |
| `p6f_node_ellipses` | 120,342 | 1.3% | 554,200 | 0.2% |
| **sum** | **9,426,144** | **99.4%** | **261,291,345** | **99.7%** |

The terms are disjoint by construction (`p6d`/`p6e`/`p6f` are sibling blocks
in the emit loop; `p6a`/`p6c`/`p6b` are the three sub-blocks of one
`build_roles` call), so the ~0.3–0.6% residual is the loop scaffolding and
`build_roles`' body-difference chaining, not an unmeasured bucket.

**The carve is the single biggest sub-term, and many of its calls are
set-preserving.** The carve itself makes 21,712 singular `clip_polygons` calls
on benchy (147,993 on base); the whole stage makes 29,000 (171,636), including
other call sites. The in-guest bbox pre-test classifies 11,018 (50.7% of carve
calls; 38.0% of stage calls) on benchy and 78,832 (53.3% of carve calls; 45.9%
of stage calls) on base as disjoint. The stage-wide host probe counts 12,237
(42.2%) / 79,369 (46.2%) disjoint calls, including **1,219 / 537 outside the
carve**. Its 1.78 / 52.59 s disjoint-class clip wall (26.8% / 27.1% of the
stage's core boolean-call wall) is therefore **not** the carve gate's measured
wall saving. The probe does not separately time the disjoint carve calls; a
probe-free paired A/B must determine the net wall effect.

**The union/simplify that follows is the second sub-term** (`union_expolys`
then `role_simplify_tolerance`'s simplify): 2.43 s benchy / 72.8 s base — a
comparable share to the carve.

**The surrounding host validation is real but second to the clips.**
`validate_entry`'s exact-Z queries are essentially the whole aggregation call
(23.8 s of 23.8 s on base; 0.99 s of 1.00 s on benchy), and every query is a
miss (0 hits) because each `(object, region, z)` identity is asked once. The
`cross_section_at_z` rebuild inside those misses is 21.1 s of the base term.

**The accelerated perimeter cfg does not target this planner path.** The
planner has no direct `slicer-core` dependency
(`modules/core-modules/tree-support-planner/Cargo.toml`); on wasm32 the SDK's
polygon ops use host imports. The cfg described in
`docs/23_controlled_perimeter_builds.md` applies to audited canonical core
compilation, not the guest's emit-pass carve algorithm. This does **not**
establish equal timing across modes or replace the paired acceptance runs.

## Candidate (for the ticket's recommendation, not authorized)

**Candidate 1 — bbox gate in `carve_emitted_regions`.** The probe identifies
11,018 (benchy) / 78,832 (base) disjoint carve calls that a representation-safe
gate could avoid as a set operation. Its net wall benefit is **unmeasured**:
stage-wide disjoint-clip wall includes other call sites, the clip bracket
excludes conversions, and a gate has its own cost.

Constraint, measured rather than assumed: the probe's disjoint `Difference`
returns a contour **not verbatim**. Probe test
`crates/slicer-core/tests/t22_probe_diff_identity_tdd.rs` (temporary) shows one
disjoint rectangle difference preserving area while rewriting the contour. It
does not prove general set equivalence or which contour attribute changed. A
gate returning the subject verbatim can match the current output only if all
reachable consumers tolerate the representation difference; otherwise it must
reproduce the current normalized result or be rejected. The `max_by` pick
compares `contour.points` as a tie-break, so representation is observable there.

**Candidate 2 — merge the carve's four role calls.** `with_areas` runs the
whole carve + union + simplify chain four times per layer (body/roof/base/
floor). The four calls share `collision_polys`; whether their union admits a
single pass is a geometry question, not measured here.

## Gaps

- Wall figures are from instrumented runs and are attribution-only (§3.2).
  The base host terms differed between captures with the same host probes
  but different guest bracket sets; call counts and class shares held.
- The candidate's own A/B (uninstrumented, paired, ordinary + accelerated) is
  ticket work beyond this attribution deliverable; no keep/drop decision is
  made here. The probe-only identity test is preserved in `probe.patch`, not
  installed in the current test suite.
