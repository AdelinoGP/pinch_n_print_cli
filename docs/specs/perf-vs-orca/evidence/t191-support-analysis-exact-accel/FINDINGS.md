# Ticket 191 — Exact host-only `PrePass::SupportAnalysis` acceleration

- **Date:** 2026-11-24 (UTC)
- **Base commit:** `63e07300` (t190 landed)
- **Active reference currency (unchanged, zero reference change):** t182's set,
  [`../t182-spatial-hierarchical-phase3-union/reference.json`](../t182-spatial-hierarchical-phase3-union/reference.json).
  Every t191 run fails closed against it (`0/440` hash mismatches).
- **Strict host-only source isolation:** All production and test edits live strictly
  inside `crates/slicer-core` (`crates/slicer-core/src/triangle_mesh_slicer.rs`,
  `crates/slicer-core/src/lib.rs`, `crates/slicer-core/src/algos/overhang_annotation.rs`,
  `crates/slicer-core/tests/support_overhang_detection_tdd.rs`) and `crates/slicer-runtime`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`). Zero WIT, schema,
  guest WASM source, or `vendor/clipper2-rust` changes (`git diff -- modules/ vendor/ xtask/`
  is empty). `slicer_core::triangle_mesh_slicer::slice_mesh_ex` is untouched so supports-off
  (`PrePass::Slice`) remains 100 % structurally inert.
- **Guest freshness after candidate builds:** `cargo xtask build-guests --check` exit `0`;
  `cargo xtask build-guests --accelerated --check` exit `0`.
- **Arms (`reduced/output.json` manifest):**
  - `baseline-ordinary`: `pnp_cli` `e661baa5…`, modules tree `e7a59f5c…`
  - `candidate-ordinary`: `pnp_cli` `e27681cf…` (**changed**), modules tree `7f0fb2aa…` (rebuilt via `cargo xtask dist --edition developer` after `slicer-core` edit; 22/24 guest WASMs byte-identical, zero guest source changes, `--profile` guest fuel invariant within `0.003 %` WASI HashMap noise)
  - `baseline-accelerated`: `pnp_cli` `a7260880…`, modules tree `a2b7af7b…`
  - `candidate-accelerated`: `pnp_cli` `3ba6cebb…` (**changed**), modules tree `3ef2a5b7…` (rebuilt via `cargo xtask dist --edition developer --accelerated`; zero guest source changes)
- **Verifier:** `verify.py` → `VERIFY PASS`; `verify.py --self-test` → `20/20` mutations rejected.

## 1. Verdict

**LANDED.** The standing `t137` 4-cell planner gate passes **4/4 supports-on cells**
on the pooled two-window campaign (`10` interleaved pairs/cell ordinary, `6` null pairs/cell,
`6` accelerated pairs/cell), clearing the `0.80 %` wall floor by **`9.48×`–`13.32×`**
and clearing even the strict any-pair null lens on **4/4 supports-on cells**, with
**40/40** ordinary supports-on wall pairs won, ordinary process CPU lower in **4/4**, and
accelerated wall corroborating **4/4** (`+7.34 %` to `+10.71 %`, `23/24` pairs won).
All 4 supports-off cells are structurally inert (`INERT-PASS`, `PrePass::SupportAnalysis`
`0.0 → 0.0 ms`, every cell inside its null wall median). Zero output drift, zero geometry
exceptions, and zero gate waivers.

1. **Output parity (fail-closed).** All **440/440** measured runs exited `0` and
   reproduced `t182`'s active hashes byte for byte (`hash_mismatches = 0`):
   - `16` ordinary and `16` accelerated output runs (`ref` + `accel`);
   - `160` ordinary A/B (`ab` + `ab2`), `96` null (`null` + `null2`), `96` accelerated A/B
     (`accel` + `accel2`), `48` instrumented stage (`stage`), and `8` `--profile` runs.

   Every run had `0` non-zero exits, `0` degraded runs, `0` fatal or non-fatal errors,
   and `0` code-1200 diagnostics. `t182/reference.json` is unchanged.

2. **Standing `t137` 4-cell planner gate — supports-on (`10` interleaved pairs/cell ordinary, `6` null pairs/cell, `6` accelerated pairs/cell, `RAYON_NUM_THREADS=12`):**

   | cell | baseline / candidate median wall | wall saving | floor mult | wins | CPU saving | null wall med / max | accel wall / CPU | gate |
   |---|---:|---:|---:|---:|---:|---:|---:|---|
   | `benchy-classic-on` | 2.595 / 2.344 s | **+7.58 %** | 9.48× | 10/10 | +1.65 % (6/10) | 0.50 / 5.97 % | +7.44 % / −0.59 % | **PASS** |
   | `benchy-arachne-on` | 2.634 / 2.445 s | **+7.83 %** | 9.78× | 10/10 | +2.03 % (6/10) | 0.55 / 1.48 % | +7.34 % / −0.53 % | **PASS** |
   | `base-classic-on` | 18.943 / 17.148 s | **+10.65 %** | 13.32× | 10/10 | +7.49 % (10/10) | 3.27 / 8.66 % | +9.40 % / +6.26 % | **PASS** |
   | `base-arachne-on` | 19.506 / 17.466 s | **+10.18 %** | 12.73× | 10/10 | +6.48 % (10/10) | 0.93 / 4.23 % | +10.71 % / +6.48 % | **PASS** |

   - **Strict any-pair null lens (`_gate_strict_any_pair_null`):** **4/4 PASS** —
     every supports-on cell's pooled ordinary median wall saving exceeds its largest
     single same-binary null pair (`+7.58 > 5.97 %`, `+7.83 > 1.48 %`, `+10.65 > 8.66 %`,
     `+10.18 > 4.23 %`).
   - **Pairwise wins:** **40/40** ordinary supports-on wall pairs won (`base-classic-on`
     has strict non-overlapping separation on both wall and CPU; `base-arachne-on` has
     `10/10` wall and `10/10` CPU wins with strict CPU separation); **23/24** accelerated
     supports-on wall pairs won (`base-arachne-on` has strict separation on wall and CPU;
     `base-classic-on` won `6/6` wall and `6/6` CPU).
   - **Accelerated `benchy` CPU disclosure:** On `base`, `SA-4`'s cantilever certificate
     eliminates `7.66–8.85 s` of Clipper2 thread-time, so both ordinary (`+7.49 / +6.48 %`)
     and accelerated (`+6.26 / +6.48 %`) process CPU drop sharply (`12/12` accelerated wins).
     On `benchy`, `SA-4` eliminates `0.21–0.23 s` of Clipper2 thread-time while `SA-1` + `SA-5`
     parallelize `~170 ms` of previously serial single-thread work across 12 Rayon workers;
     ordinary process CPU is lower (`+1.65 / +2.03 %`, `6/10` wins each), while accelerated
     process CPU is flat (`−0.59 / −0.53 %`, `3/6` wins each, `~65 ms` out of `11–12 s`,
     well inside `benchy`'s `5.13 %` / `1.73 %` identical-binary null CPU medians).

3. **Supports-off (structurally inert, null-bounded non-regression `bound = max(0.80 %, null_wall_median)`):**

   | cell | ordinary wall | wins | CPU | null wall med / max | stage ms (base → cand) | accel wall / CPU | gate |
   |---|---:|---:|---:|---:|---:|---:|---|
   | `benchy-classic-off` | +0.07 % | 5/10 | +0.00 % | 1.58 / 4.95 % | 0.0 → 0.0 ms | −0.11 % / −0.22 % | **INERT-PASS** |
   | `benchy-arachne-off` | −0.45 % | 5/10 | −0.92 % | 1.73 / 11.64 % | 0.0 → 0.0 ms | −1.51 % / −0.61 % | **INERT-PASS** |
   | `base-classic-off` | −1.01 % | 3/10 | +0.16 % | 1.38 / 8.25 % | 0.0 → 0.0 ms | −3.34 % / −0.15 % | **INERT-PASS** |
   | `base-arachne-off` | +0.37 % | 6/10 | −0.13 % | 2.32 / 3.66 % | 0.0 → 0.0 ms | −1.86 % / −0.50 % | **INERT-PASS** |

   Inside `crates/slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`,
   `config.support_enabled == false` skips the entire analysis block before any `t191` code
   executes (pinned by unit test `t191_enable_support_false_is_structurally_inert`), and
   `slicer_core::triangle_mesh_slicer::slice_mesh_ex` is untouched (`PrePass::SupportAnalysis`
   is `0.0 → 0.0 ms` in `stage.json` on all four supports-off cells). Disclosed: `base-classic-off`
   pooled ordinary wall is `−1.01 %`, strictly inside its own `1.38 %` null wall median
   (`8.25 %` max), with lower process CPU (`+0.16 %`, `6/10` wins) and faster instrumented
   `slice_complete` (`8,735 → 8,706 ms` across 3/3 stage pairs).

4. **Instrumented `PrePass::SupportAnalysis` stage wall & deterministic `--profile` `host:support_analysis` calls/thread-time (attribution only):**

   | cell | `SupportAnalysis` wall (3-pair med) | speedup | `host:support_analysis` calls (`Δ`) | `host:support_analysis` thread-time | guest fuel `Δ` |
   |---|---:|---:|---:|---:|---:|
   | `benchy-classic-on` | 279.0 → 95.0 ms (**−65.95 %**) | **2.94×** | 1,366 → 1,046 (**−320 / −23.43 %**) | 0.909 → 0.684 s (**−24.82 %**) | +0.0031 % |
   | `benchy-arachne-on` | 277.0 → 87.0 ms (**−68.59 %**) | **3.18×** | 1,366 → 1,046 (**−320 / −23.43 %**) | 0.877 → 0.671 s (**−23.54 %**) | +0.0003 % |
   | `base-classic-on` | 2,864.0 → 1,044.0 ms (**−63.55 %**) | **2.74×** | 4,430 → 2,982 (**−1,448 / −32.69 %**) | 19.539 → 11.882 s (**−39.19 %**) | +0.0014 % |
   | `base-arachne-on` | 2,953.0 → 1,094.0 ms (**−62.95 %**) | **2.70×** | 4,430 → 2,982 (**−1,448 / −32.69 %**) | 19.120 → 10.268 s (**−46.30 %**) | +0.0004 % |

   - **Classic ≡ Arachne invariance:** `host:support_analysis` call reduction is exact to the
     single call between classic and arachne:
     - `benchy`: **`−320`** (`−241` `clip_polygons` + `−79` `offset`, certifying **79/79** false cantilever layers + skipping **162** empty modifier intersections).
     - `base`: **`−1,448`** (`−1,092` `clip_polygons` + `−356` `offset`, certifying **356/357** false cantilever layers — 4 more than `t188`'s `352/357` thanks to the narrow level-3 tile refinement — + skipping **736** empty modifier intersections).
   - **Zero guest behavior change:** Total guest fuel varies by only `0.0003 %`–`0.0031 %`
     (expected per-run WASI `HashMap` `RandomState` fluctuation across `18B–137B` fuel).

## 2. Route (ESTIMATE ONLY: `t174 × t175 × t182 × t183 × t184 × t185 × t190 × t191`; not a ratio of record)

`reduced/route.json` scales `t190`'s estimate by each supports-on cell's pooled `t191`
wall saving (carrying forward exact `t190` ratios on the four structurally inert
supports-off cells while disclosing their measured noise deltas separately):

| cell | ordinary post-t190 → post-t191 | accelerated post-t190 → post-t191 |
|---|---:|---:|
| `benchy-classic-on` | 1.0704x → **0.9892x** | 1.0227x → **0.9466x** |
| `benchy-arachne-on` | 1.0007x → **0.9224x** | 0.9838x → **0.9115x** |
| `base-classic-on` | 0.7688x → **0.6869x** | 0.7590x → **0.6877x** |
| `base-arachne-on` | 0.7012x → **0.6297x** | 0.6615x → **0.5906x** |

- **All 8/8 matrix cells now project `< 1.00x` on BOTH ordinary (`0.6263x–0.9892x`) and
  accelerated (`0.5906x–0.9466x`)** — `benchy-classic-on` (`0.9892x` ord / `0.9466x` acc)
  and `benchy-arachne-on` (`0.9224x` ord / `0.9115x` acc) both cross `< 1.00x` ahead of
  remaining frontier track [Exact Zero-WIT `PostPass` acceleration](../../issues/192-postpass-exact-acceleration.md)
  (screened at `+3.56…+9.51 %` across all 8 cells).
- Official running tally remains **2/8 provisionally won** pending a fresh single-HEAD
  `n >= 4` Orca closing scoreboard after `t192`.

## 3. Disclosures

- **Box load and pooled two-window design (`_meta`):** All `pnp_cli` campaign batches ran
  sequentially inside the standard command environment with `RAYON_NUM_THREADS=12` pinned
  inside `run_t191.ps1`. Instantaneous `Win32_Processor` load samples before/after each
  batch (the initial before-sample reflects `Write-Manifest` hashing right before warmup):
  `ab` `23.75 → 4.15 %`, `ab2` `27.60 → 6.41 %`, `null` `11.36 → 8.12 %`,
  `null2` `14.47 → 4.09 %`, `accel` `10.42 → 10.61 %`, `accel2` `16.67 → 4.38 %`,
  `stage` `17.40 → 5.32 %`. Window 1 alone passed all 4 supports-on cells on ordinary wall
  and CPU (`+7.13…+10.98 %`, `20/20` wins) while `benchy-arachne-off` landed at `−1.15 %`
  on a noisy pair-1 spike (`−6.64 %`); a 2×2 executable/module crossover confirmed zero
  regression in either binary or module tree, and Window 2 (`ab2` + `null2` + `accel2`)
  was run and pooled honestly with Window 1 (zero rows discarded).
- **Explicit non-goals honored:** `slicer_core::triangle_mesh_slicer::slice_mesh_ex` was
  not modified (`PrePass::Slice` remains unchanged), and `G_holes` / `G_win` / `G_island_win`
  / `G_cert` on `grown_offset` (falsified/rejected in `t188`) were excluded.

## 4. Production implementation

All edits live strictly inside `crates/slicer-core` and `crates/slicer-runtime`:

- **`SA-1` — Parallel print-plane slicing (`crates/slicer-core/src/triangle_mesh_slicer.rs::slice_mesh_ex_parallel`, re-exported in `crates/slicer-core/src/lib.rs`, called from `crates/slicer-runtime/src/builtins/support_analysis_producer.rs::collect_print_plane_occupancy`):**
  - Derived `Copy` on `IntersectionLine` and factored `slice_make_lines_for_indices`.
  - Added `#[cfg(feature = "host-algos")] pub fn slice_mesh_ex_parallel` partitioning
    `mesh.indices` into 16,384-triangle chunks (serial fallback below `32,768` triangles),
    merging per-layer line buffers in exact chunk order `0..P` in parallel, and chaining
    each layer with `into_par_iter().collect()`.
- **`SA-2` — Parallel `object_layer_union` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`):**
  - Borrowed `Vec<&[ExPolygon]>` per `(object_id, layer_index)` (zero-clone on single-region
    layers) and ran `union_ex` in parallel via `into_par_iter().collect::<BTreeMap<_, _>>()`.
  - Also borrowed `&[ExPolygon]` in `object_tops` instead of cloning every layer's polygons.
- **`SA-3` — Empty `enforcers` / `blockers` guard (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`):**
  - Guarded `enforced` and `blocked` checks with `!enforcers.is_empty()` and `!blockers.is_empty()`
    before invoking `intersection_ex`.
- **`SA-4` — Exact `cantilever_surfaces` distance certificate (`crates/slicer-core/src/algos/overhang_annotation.rs::certify_covered_by_square_offset` + `detect_support_contacts_with_annotations`):**
  - Exact closed-ring decimation matching `OFFSET_SHORTEST_EDGE_FACTOR` (`0.005 * delta_units`),
    positive-area CCW contour + validated CW hole spatial CSR segment grid (`LowerSegGrid`),
    conservative `1,000`-unit (`0.10 mm`) safety margin (`r_in = delta_units - 1_000.0`),
    single-disk / interior-box fast path, vertex-screen fast reject, and coarse-to-fine
    (4×4 + narrow level-3 4×4) box cover, falling back to unchanged `offset(lower, 3.0, Square, 0.01)` +
    `difference_ex` on any uncertain, near-threshold, or malformed input.
- **`SA-5` + `SA-6` — Concurrent `rayon::join` + LPT contact scheduling + zero-clone `ContactWork<'a>` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`):**
  - Borrowed `polygons: &'a [ExPolygon]` in `ContactWork<'a>`.
  - Overlapped `collect_print_plane_occupancy` with `(object_layer_union → ModifierGeometry::slice → contacts_par)` via `rayon::join`.
  - Scheduled contacts in descending `(lower_verts + region_verts)` weight order (tie-broken
    by canonical index) with `.into_par_iter().with_max_len(1)`, then sorted surviving
    `(idx, Contact)` pairs by `idx` ascending for 100 % deterministic emission.

### Tests added

- `crates/slicer-core/src/triangle_mesh_slicer.rs`:
  `t191_slice_mesh_ex_parallel_matches_serial_bit_for_bit_across_chunks_holes_grazing_and_empty`
  (38,400-triangle multi-chunk hollow-box mesh with grazing planes, holes, and empty inputs).
- `crates/slicer-core/tests/support_overhang_detection_tdd.rs`:
  `t191_cantilever_certificate_matches_uncertified_oracle_across_geometry_classes`
  (covered rims, near-threshold `2.85 / 2.95 / 3.05 mm`, true cantilevers `4.0 mm`, C/L strips,
  covered hole rim + 20 mm hole-spanning cantilever, sub-decimation & collinear degenerate
  contours, empty lower, and CW/reversed rings).
- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`:
  `t191_multi_region_layer_union_prevents_spurious_cross_region_contact`,
  `t191_lpt_scheduling_preserves_canonical_candidate_order_and_empty_modifier_flags`,
  `t191_enable_support_false_is_structurally_inert`.

### Gates run

- `cargo test -p slicer-core --features host-algos --lib triangle_mesh_slicer::tests::t191_`: `1/1` pass (exit `0`).
- `cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd`: `19/19` pass (exit `0`).
- `cargo test -p slicer-runtime --lib support_analysis_producer`: `21/21` pass (exit `0`).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (exit `0`).
- `cargo xtask check-literals`: `0` violations across 188 watched types (exit `0`).
- `cargo xtask check-test-quality --report`: `0` findings (exit `0`).
- `cargo xtask build-guests --check`: exit `0`.
- `cargo xtask build-guests --accelerated --check`: exit `0`.
- `python docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/verify.py --self-test`: `20/20` rejected (exit `0`).
- `python docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/verify.py`: `VERIFY PASS` (exit `0`).
