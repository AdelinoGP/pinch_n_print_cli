# Ticket 32 — Emit-pass carve gate: representation safety + paired A/B

Wayfinder ticket: [Emit-pass carve gate: representation safety + paired A/B](../issues/32-emit-carve-gate-representation-and-ab.md).
Candidate source: [Tree-planner substage attribution](../issues/22-tree-planner-substage-attribution.md)
and `evidence/t22-planner-substage/FINDINGS.md`.

## 1. The gate

`carve_emitted_regions_with_bbox_gate` (`modules/core-modules/tree-support-planner/src/lib.rs`)
replaces the unconditional per-region
`host::clip_polygons(region, collision_polys, Difference)` in `build_roles`'
`with_areas` closure. A region whose bounding box is disjoint from the
collision set's bbox cannot intersect it, so the clip is a no-op *as a set* and
is skipped. `bboxes_disjoint` treats touching boxes as overlapping.

The collision bbox is computed once per `build_roles` call
(`expolygons_bbox(collision_polys)`) and shared across the four role carves.

## 2. Representation safety — the measured answer

The ticket-22 probe observed that a disjoint `Difference` returns the subject
set-equal but **not verbatim** (clipper normalizes ring order/winding). The
question this ticket had to answer: does anything downstream observe that
normalization? Measured in `t32_gate_*` tests
(`modules/core-modules/tree-support-planner/src/lib.rs`, module test block):

### 2.1 What the skipped clip actually returns

`t32_gate_disjoint_difference_observation`, six shapes against a far-away
collision rect, in-repo via the module's own `host::clip_polygons` (native
route: `slicer_sdk::host::clip_polygons` → `slicer_core::polygon_ops::clip_polygons`):

| shape | verbatim? | set-equal? | clip output is a fixed point? |
| --- | --- | --- | --- |
| rectangle (CCW) | no | yes | yes |
| L-shape (CCW) | no | yes | yes |
| rectangle with hole | no | yes | yes |
| rectangle, reversed winding (CW) | no | yes | **no** |
| rectangle, rotated start vertex (CCW) | no | yes | yes |
| rectangle with collinear vertex (CCW) | no | yes | yes |

`fixed_point` = `clip(clip(r, c, D), c, D) == clip(r, c, D)` byte-wise. The
single non-fixed-point case is the non-canonical (CW) input.

### 2.2 The chain after the carve erases representation for canonical input

`t32_gate_chain_gate_matches_unconditional` compares the full `with_areas`
chain (carve → `union_expolys` → optional `expolygons_simplify` →
final set-wide `Difference`) across **29 shapes × 3 configs = 87
combinations**, comparing the production gate against the unconditional
carve byte-for-byte:

- production gate: **0 mismatches**;
- naive gate (no winding guard, no lone-region repair): **6 mismatches**.

The naive gate's divergences are of two kinds:

1. **Non-canonical winding** (synthetic CW rectangle): the clip normalizes CW
   to CCW; returning CW verbatim changes the ring. Fix: the winding guard
   (`region_has_canonical_winding`) never gates a non-canonical region.
2. **Lone gated region under active simplify** (`single_reversed_cw` is a
   special case; the reachable instance is a single curved body disc):
   `simplify_ring` runs Douglas–Peucker anchored at `points[0]`, so a verbatim
   ring can select a different kept-vertex set than the clip-normalized one.
   Measured on a 16-gon disc at `avg_node_per_layer = 500`:
   area 50,000,000 → 49,989,460 units² (−0.021%), different start vertex.
   Fix: when the gate fired AND exactly one region survives AND
   `simplify_active`, re-clip that lone region (`carve_emitted_regions`), which
   reproduces today's exact bytes.

### 2.3 Every reachable producer emits canonical CCW rings

`t32_gate_real_input_windings_are_canonical` sweeps 60 combinations of radius
(0.05–2.0 mm), movement (six directions including large diagonals), resolution
(fine 100-gon / coarse 4-gon quad):

- all 60 `node_ellipse` outputs have positive signed area (CCW);
- the `swept_region` convex-hull disc is CCW by construction (documented CCW
  monotone-chain hull).

`branch_areas` / `interface_areas` / `base_areas` / `floor_areas` receive only
`node_ellipse` output (`InterfaceRole` dispatch in the emit pass);
`structural_body_regions` receives only `swept_region` discs. So the winding
guard is a safety net, not a hot-path filter.

### 2.4 The lone-region repair is load-bearing

`t32_gate_coarse_single_disc_is_the_unsafe_case` builds the real `build_roles`
with `avg_node_per_layer = 500` and a single degenerate disc, then:

- unconditional replication == real `build_roles` body (model faithful);
- production gate == body (repair preserves output);
- naive gate != body (the repair is required).

`t32_gate_touching_bbox_is_not_gated` pins the shared-edge case.

### 2.5 Downstream observability (why the repair is the only one needed)

Traced consumers (crate-qualified symbols):

- `build_roles` returns `slicer_ir::SupportPlanRoleRegion` (`crates/slicer-ir/src/slice_ir.rs`:
  role + regions; no representation fields);
- host aggregation (`crates/slicer-wasm-host/src/support_aggregation.rs`,
  `try_aggregate_support_plans_with_policy` / `validate_entry` /
  `union_same_family_entries`) is set/area/bbox based — no vertex-sequence
  comparison. `validate_entry` uses `in_routing_cell` (per-body bbox extent)
  and `overlaps_any` (clipper intersection area vs tolerance);
- the tree renderer (`modules/core-modules/tree-support/src/lib.rs`) runs
  **every** role through `slicer_core::polygon_ops::union_ex` before any
  vertex is read (the `rendered` map, both the regularized and the
  `unwrap_or_else` branch); `render_polygon_with_wall_count` then derives paths
  from already-unioned polygons, and `skeleton_wall_count` is a point-in-poly
  test, not a sequence read;
- traditional-support (`modules/core-modules/traditional-support/src/lib.rs`)
  consumes only its own family's entries (a family mismatch is non-fatal
  error code 333), so the tree planner's regions never reach it.

The in-planner chain (2.2) is therefore the *last* place representation could
survive to an observer, and the lone-region repair closes it there.

## 3. Firing evidence (probe-instrumented run, 2026-09-25)

Temporary `t32-gate` counters (removed before commit) around the gate:
one `log_warn` line per module call.

| fixture | gated (regions skipped) | repaired (lone-region re-clips) | carved (regions still clipped) |
| --- | ---: | ---: | ---: |
| `tmp/3dbenchy.stl` | 11,018 | 0 | 10,694 |
| `tmp/base.stl` | 78,832 | 0 | 69,161 |

The gated counts match ticket 22's predicted bbox-disjoint carve classes
exactly (11,018 / 78,832), and `repaired=0` on both real fixtures: the repair
never fires on real geometry, it exists for the reachable-in-principle
coarse-simplify case pinned by the test.

Both probe runs: `degraded=false`, `fatal_error_count=0`,
`non_fatal_error_count=0`.

## 4. Paired A/B

Protocol: [`run_ab.ps1`](run_ab.ps1) (the in-session copy ran under
`target/t32-ab/`; interleaved baseline/candidate, arm order alternating by
repeat or explicitly chosen for short batches; uninstrumented; process
creation-to-exit wall and CPU via `GetProcessTimes`; per-sample cpu/wall
recorded). The consolidated rows were independently validated against stderr:
exactly one `slice_complete`, `status=ok`, `degraded=false`, zero fatal and
non-fatal counts, and G-code byte count matched the file on disk for every
retained sample. Raw G-code and stderr stay gitignored; the per-run hashes,
TYPE histograms, timings, and ratios are in `ab-rows.csv`.

Arms within each mode differ in exactly one file: the
`tree-support-planner/tree-support-planner.wasm` in each isolated module dir;
`pnp_cli.exe`, manifests, and all 23 other external modules are identical.
Ordinary planner SHA-256: baseline
`4e6c2124301d0e3e6b04d81162b3dec536ae59b84ebfe601123d5b3a1066b38b`,
candidate `4d212e82c7a472d11b498f925ce7442d00f6555d0cea47f60c5d2a6f62dffa8e`;
accelerated: baseline
`712117f49b73df6f23899cef355c08c11c8f232600b6f1cfb427bb10d3123a7b`,
candidate `0efafa7799294a2853db7b258569cc62fb82f9ec899b437a93aeb44097e1839f`.
Full per-run data, including output hashes, are in [`ab-rows.csv`](ab-rows.csv).
The accelerated baseline
was rebuilt from a `git archive HEAD` source snapshot, separately freshness
checked; the candidate was rebuilt in the working tree. Both accelerated arms
were staged from the complete `target/dist-accelerated/developer/` developer
snapshot, then **only** its planner WASM was replaced for the baseline arm;
`module diagnose` passed with all 24 modules external in both arms. The old
saved accelerated baseline (`9ed76d76…`) was **not** used: it did not match
the fresh matched-source baseline.

Config `pnp-classic-supports-on.json`, `--module-dir` the respective arm dir,
12 threads, `tmp/3dbenchy.stl` and `tmp/base.stl`.

### Results

Machine condition: the ordinary benchy batch overlapped a **different
worktree's** integration-test session (`D:\slicerProject\pinch_n_print_cli`,
not this worktree), previously observed using ~8.5 of 12 logical cores.
Later batches had changing, unisolated external load. Quiet 12-thread runs
have cpu/wall ≈ 6.5 (§10.3); here per-sample ratios spanned 1.90–4.96.
Wall is therefore **load-qualified**. Process CPU is less exposed to
descheduling, and consistently corroborates the direction, but cannot prove a
quiet-machine wall improvement. The standing exclusion (`cpu/wall < 2.0`)
removes three ordinary benchy baseline samples and no other measured sample;
all completed pairs and every per-sample ratio are in `ab-rows.csv`.

An earlier benchy batch was discarded: it was measured while this session ran
cargo builds, adding self-inflicted load. Batch `ab-benchy-2` is clean of
self-inflicted load.

#### Ordinary benchy, 10 paired repeats (1 warmup/arm)

All 10 pairs, candidate − baseline:

| metric | median | mean | range | candidate faster |
| --- | ---: | ---: | ---: | ---: |
| process CPU | −1.71 s | −2.05 s | [−4.14, −0.59] | **10/10** |
| wall (load-qualified) | −6.28 s | −3.17 s | [−10.5, +11.3] | 8/10 |

CPU paired mean / standard error = **−5.09**, i.e. the candidate's CPU win is
well outside the pair noise.

Starvation note (cpu/wall < 2.0, §10.3): 3 of 10 baseline samples, 0 of 10
candidate samples. Because the exclusion is arm-asymmetric, the table above
uses all pairs for CPU (less descheduling-sensitive) and quotes the wall row from all
pairs as load-qualified; the starvation-filtered wall subset (7 pairs) gives
median −5.48 s, mean −1.14 s, 5/7.

Output disclosure: every accepted run `degraded=false`, `non_fatal=0`; G-code
bytes baseline 15,001,823–15,002,422 vs candidate 15,002,068–15,003,086.
`TYPE` counts vary only for `Inner wall` (same-binary nondeterminism); support,
interface, and all other marker counts are identical across arms. The arms'
byte ranges overlap and sit inside the known same-binary variance band (§3.6).

#### Accelerated benchy, 10 paired repeats (1 warmup/arm)

| metric (candidate − baseline) | paired median | paired mean | range | candidate faster |
| --- | ---: | ---: | ---: | ---: |
| process CPU | −1.48 s | −1.95 s | [−4.91, −0.84] | **10/10** |
| wall (load-qualified) | −1.77 s | −1.31 s | [−3.40, +1.96] | 8/10 |

CPU/wall: baseline 3.82–4.36, candidate 3.94–4.50; no ratio <2.0.
G-code bytes baseline 15,001,571–15,002,950, candidate
15,001,852–15,002,848; `TYPE` histograms vary only in `Inner wall`, and
both arms' aggregate marker counts match exactly. All runs completed clean.

#### Ordinary base, 4 completed pairs across interrupted and short batches

| metric (candidate − baseline) | paired median | paired mean | range | candidate faster |
| --- | ---: | ---: | ---: | ---: |
| process CPU | −35.35 s | −41.58 s | [−66.48, −29.14] | **4/4** |
| wall (load-qualified) | −38.95 s | −68.89 s | [−159.05, −38.62] | 4/4 |

Batch `ab-base-1` stopped before candidate repeat 3; batch `ab-base-2`
stopped midway through repeat 2. Only their completed pairs were retained,
then a complete opposite-order pair was added in `ab-base-3`. The 159 s
baseline wall excess in `ab-base-2` had cpu/wall 3.24 vs 4.04 in its
candidate, so the wall **mean** is especially load-confounded; the CPU
direction remains consistent. Ratios overall: baseline 3.24–4.65,
candidate 3.99–4.96; no sample <2.0. G-code bytes baseline
54,478,327–54,478,850 vs candidate 54,478,179–54,478,389;
`TYPE` histograms vary only in `Inner wall`. Every retained pair completed
clean (`degraded=false`, zero non-fatals).

#### Accelerated base, 3 complete pairs in alternating-order short batches

| metric (candidate − baseline) | paired median | paired mean | range | candidate faster |
| --- | ---: | ---: | ---: | ---: |
| process CPU | −38.39 s | −40.92 s | [−51.72, −32.64] | **3/3** |
| wall (load-qualified) | −33.13 s | −38.91 s | [−53.45, −30.14] | 3/3 |

CPU/wall: baseline 4.26–4.42, candidate 4.65–4.67; no sample <2.0.
G-code bytes baseline 54,478,175–54,478,892 vs candidate
54,478,597–54,478,759; `TYPE` histograms vary only in `Inner wall`.
All runs completed clean. The planner's accelerated artifact is **not**
byte-identical to the ordinary one at matched source states, so this was an
independent A/B, not an inherited verdict.

**Human decision: KEEP, accepted 2026-09-25.** The candidate was not
auto-committed; the explicit acceptance authorizes this commit. Process CPU decreased on all
completed pairs in both modes and fixtures, with lower paired median wall in
each. Under the standing *median uninstrumented wall* rule, this is a
load-qualified win, not a quiet-machine confirmation. For an unqualified wall
claim, repeat the paired runs on a quiet machine (cpu/wall near §10.3's
reference), especially ordinary benchy with asymmetric <2.0 exclusions.

## 5. Gates

- `cargo test -p tree-support-planner` — 144 tests passed (61 lib + 9
  integration binaries), including the seven `t32_gate_*` tests; rerun after
  the final test-only edits.
- `cargo clippy -p tree-support-planner --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations.
- `cargo xtask check-test-quality --report` — 12 existing findings in other
  files, none in the planner file touched here.
- `cargo xtask build-guests --check` and
  `cargo xtask build-guests --accelerated --check` returned 0 after rebuild;
  the archived baseline accelerated build passed its own freshness check.
  Rebuilding after the final regression-test-only edit produced the same
  ordinary and accelerated candidate WASM SHA-256 as the measured artifacts.
- Output disclosure on the smoke pair: identical `;TYPE:` counts (1,177
  markers, same histogram), `degraded=false`, `non_fatal=0`.
