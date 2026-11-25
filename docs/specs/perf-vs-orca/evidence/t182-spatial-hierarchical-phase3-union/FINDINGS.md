# t182 findings: host-side spatial hierarchical Phase-3 union with convex-containment pruning

**Status: resolved 2026-11-22 — landed (`worker-k8`) by explicit human KEEP ruling advancing the active reference set ([`reference.json`](reference.json)) for the 4 moved supports-on hashes (zero part-geometry movement; zero `Support interface` movement; 4/4 supports-off byte-identical to `t175`).**

**Verdict: literal standing `t137` 4-cell planner gate PASSED (4/4 supports-on PASS, 4/4 supports-off INERT-PASS; 20/20 ordinary wall wins, 20/20 ordinary CPU wins, 12/12 accelerated wall wins, 11/12 accelerated CPU wins), with the stricter `t175` single-pair-null lens transparently disclosed (`base` 2/2 PASS at 8.2×–31.5× above `null` any-pair; `benchy` 2/2 `INSIDE-NULL-ANY-PAIR` on ordinary wall while clearing `null` median wall by 1.9×–3.7×, clearing `null` any-pair CPU by 7.1× on `benchy-classic-on`, and corroborating at `+4.58 % / +3.81 %` on accelerated).**

Implementing `slicer_core::polygon_ops::union_spatial_tree` (`crates/slicer-core/src/polygon_ops_spatial_union.rs`, selected winner `worker-k8`: `DEFAULT_SPATIAL_UNION_CHUNK_SIZE = 8`, `DEFAULT_SPATIAL_UNION_NESTED_PAR = false`) and wiring it strictly into batched `ClipOperation::Union` dispatch (`crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` and native fallback `crates/slicer-sdk/src/host_batch.rs::clip_polygons_batch` when `subject.len() + clip.len() > 8`) delivers:
- **`base-classic-on`:** **+13.85 %** ordinary wall (`28.073 → 24.241 s`, `5/5` wins, **17.31×** floor, **31.5×** `null` any-pair `0.44 %`) / **+23.07 %** CPU (`171.45 → 131.61 s`, `5/5`); accelerated **+14.64 %** wall (`3/3`) / **+24.20 %** CPU (`3/3`).
- **`base-arachne-on`:** **+14.00 %** ordinary wall (`28.547 → 24.549 s`, `5/5` wins, **17.51×** floor, **8.2×** `null` any-pair `1.71 %`) / **+22.62 %** CPU (`177.64 → 137.45 s`, `5/5`); accelerated **+13.57 %** wall (`3/3`) / **+23.39 %** CPU (`3/3`).
- **`benchy-classic-on`:** **+1.90 %** ordinary wall (`3.425 → 3.358 s`, `5/5` wins, **2.37×** floor, **3.7×** `null` median `0.51 %`, inside `null` any-pair `4.80 %`) / **+7.25 %** CPU (`15.73 → 14.59 s`, `5/5`, **7.1×** `null` any-pair CPU `1.01 %`); accelerated **+4.58 %** wall (`3/3`) / **+4.79 %** CPU (`3/3`).
- **`benchy-arachne-on`:** **+2.41 %** ordinary wall (`3.515 → 3.419 s`, `5/5` wins, **3.02×** floor, **1.9×** `null` median `1.25 %`, inside `null` any-pair `3.65 %`) / **+4.70 %** CPU (`16.97 → 15.83 s`, `5/5`); accelerated **+3.81 %** wall (`3/3`, above ordinary `null` any-pair) / **+3.73 %** CPU (`2/3`).
- **All 4 supports-off cells:** structurally inert (`enable_support: false` never calls `clip_polygons_batch`), **byte-identical to `t175`'s `reference.json`** across all runs on both ordinary and accelerated arms, and **INERT-PASS** (`−1.07 %` to `+1.64 %` ordinary wall, all inside noise).

Ticket: [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](../../issues/182-host-spatial-hierarchical-phase3-union.md). Graduated from [t178](../../issues/178-planner-phase3-union-attribution.md).

---

## 1. Design & implementation (host-only, zero WIT/guest edits)

1. **`crates/slicer-core/src/polygon_ops_spatial_union.rs` (re-exported from `crates/slicer-core/src/polygon_ops.rs`):**
   - **Canonical-winding guard (`has_canonical_winding`):** verifies exact `i128` doubled signed area (`contour > 0`, every hole `< 0`) on all inputs. If any input violates canonical Slic3r winding (e.g., opposite-wound rings relying on `FillRule::NonZero` hole punching), delegates immediately to monolithic `clip_polygons(subject, clip, ClipOperation::Union)`.
   - **Exact convex-containment pruning (`prune_convex_contained`):** certifies hole-free simple convex CCW containers in $O(V)$ (`is_convex_ccw`: rejects zero-length edges, negative turns, $180^\circ$ collinear foldbacks, and multi-wound star polygons via exact half-plane wrap count `wraps == 1`), sorts containers by descending exact `i128` net doubled area and candidates ascending (so identical convex siblings never drop each other), and prunes polygons strictly inside a single convex container using conservative $O(1)$ inscribed-disk fast-accept backed by exact `i128` $O(\log V)$ binary-search `point_in_convex_ccw` (exact even with collinear intermediate vertices on closing rays; checks both contour and hole vertices).
   - **Deterministic bbox-connected components (`bbox_connected_components`):** X-axis sweep + path-compressed union-find grouped via `BTreeMap` by smallest member index; canonical singletons bypass Clipper2.
   - **2D Morton Z-order spatial hierarchical reduction (`morton_sort_indices` + `union_spatial_tree_with_options`):** sorts multi-polygon components (`> k`) by `i128`-safe bbox-center Morton code and reduces in chunks of `k = 8` via head-tail `clip_polygons(&polys[..1], &polys[1..], ClipOperation::Union)`, with a strict termination guard forcing a single final union when a round fails to shrink a component or at `MAX_HIER_DEPTH = 5`.
2. **Surgical batch-only wiring (sole production caller is `tree-support-planner::run_clip_batches`):**
   - `crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` and `crates/slicer-sdk/src/host_batch.rs::clip_polygons_batch` (native fallback) dispatch `slicer_core::polygon_ops::union_spatial_tree` **only** when `op == ClipOperation::Union` and `subject.len() + clip.len() > DEFAULT_SPATIAL_UNION_CHUNK_SIZE` (`8`).
   - Serial `clip_polygons` and all `Difference`/`Intersection`/`Xor` calls are untouched. Zero WIT/schema or guest source edits (guests were rebuilt solely to refresh `slicer-core` path-dependency fingerprints so both freshness gates exit `0`).

---

## 2. Parameter sweep (`base-classic-on`, chosen: `worker-k8`)

1 warmup + 3 alternating rounds (`baseline → worker-k4 → worker-k8 → worker-k16 → nested-k4 → nested-k8 → nested-k16`, reversed on round 2) on a quiet box (sampled CPU load `4 % → 5 %`), staged with identical baseline module tree `96f3b3b0…` and differing only in `pnp_cli.exe` (`reduced/sweep.json`, `reduced/sweep_audit.json`):

| Arm | Wall med (s) | Wall spread (s) | Wall Δ | CPU med (s) | CPU Δ | SHA-256 (prefix) | Δ bytes / lines | `Support` Δ mm (`%` total) | Part lines moved |
|---|---:|---|---:|---:|---:|---|---:|---:|---:|
| `baseline` | 28.646 | 28.433–29.102 | — | 173.28 | — | `7b90e749` (`t175`) | 0 / 0 | 0.00000 (0.000000 %) | 0 |
| `worker-k4` | 24.434 | 24.381–24.586 | +14.70 % | 131.27 | +24.25 % | `9cd6c27f` | +87 / +3 | −1.19490 (−0.000153 %) | **0** |
| **`worker-k8` (chosen)** | **24.262** | **24.181–24.518** | **+15.30 %** | **132.39** | **+23.60 %** | **`7e115231`** | **−56 / −1** | **−0.24238 (−0.000031 %)** | **0** |
| `worker-k16` | 24.586 | 24.432–26.162 | +14.17 % | 133.31 | +23.07 % | `097b6dbd` | +61 / +2 | −0.11633 (−0.000015 %) | **0** |
| `nested-k4` | 24.205 | 24.097–25.404 | +15.50 % | 132.47 | +23.55 % | `9cd6c27f` | +87 / +3 | −1.19490 (−0.000153 %) | **0** |
| `nested-k8` | 24.411 | 24.230–24.438 | +14.78 % | 132.81 | +23.35 % | `7e115231` | −56 / −1 | −0.24238 (−0.000031 %) | **0** |
| `nested-k16` | 24.462 | 24.342–24.535 | +14.61 % | 133.86 | +22.75 % | `097b6dbd` | +61 / +2 | −0.11633 (−0.000015 %) | **0** |

- **Determinism proof:** `worker-k4` ≡ `nested-k4` (`9cd6c27f…`), `worker-k8` ≡ `nested-k8` (`7e115231…`), and `worker-k16` ≡ `nested-k16` (`097b6dbd…`) are **byte-identical** across all runs, confirming indexed Rayon collection preserves exact order.
- **Selection (`worker-k8`):** satisfies the pre-declared rule (`k = 8` preferred if within noise of `k = 4` for sub-micron Hausdorff): `worker-k8` (`24.262 s`, tight spread `0.337 s`) sits within `0.057 s` (`0.23 %`) of `nested-k4` (`24.205 s`, spread `1.307 s`), strictly beats `nested-k8` on both wall (`−149 ms`) and CPU (`−420 ms`), strictly beats `nested-k4` on CPU (`132.39 vs 132.47 s`), and cuts the `Support` path delta **4.9×** (`−0.242 mm` vs `−1.195 mm`; `0.949 µm` vs `2.309 µm` `t178` Hausdorff).

---

## 3. Standing `t137` gate: paired wall, CPU, null control, and accelerated corroboration

Four freshly staged distributions from HEAD `7b82f188` (`output.json::manifest`): ordinary `59ce295a… → dd668170…`, accelerated `ac969468… → 3c2bc193…`. Quiet box across all three batches (sampled CPU load: `ab` `6 % → 9 %`, `null` `2 % → 7 %`, `accel` `0 % → 3 %`). Positive = candidate faster.

### Ordinary arm (5 interleaved pairs/cell, `reduced/ab.json` + `reduced/null.json`)

| Cell | Wall s (base → cand) | Wall med (spread) | Wins | Floor× | Null wall (med / any) | CPU med (wins) | Null CPU (med / any) | Literal `t137` | Strict any-pair lens |
|---|---:|---|---:|---:|---:|---|---:|---|---|
| `benchy-classic-off` | 1.772 → 1.721 | +1.64 % (+0.38…+5.46) | 5/5 | 2.05× | 0.88 / 1.18 % | −0.73 % (1/5) | 2.49 / 2.63 % | **INERT-PASS** | **INERT-PASS** |
| **`benchy-classic-on`** | **3.425 → 3.358** | **+1.90 %** (+0.30…+3.14) | **5/5** | **2.37×** | 0.51 / 4.80 % | **+7.25 %** (5/5) | 0.61 / 1.01 % | **PASS** | `INSIDE-NULL-ANY-PAIR` |
| `benchy-arachne-off` | 1.788 → 1.807 | −1.07 % (−2.54…+6.73) | 2/5 | −1.33× | 0.48 / 6.07 % | +0.00 % (2/5) | 0.81 / 0.81 % | **INERT-PASS** | **INERT-PASS** |
| **`benchy-arachne-on`** | **3.515 → 3.419** | **+2.41 %** (+1.85…+3.73) | **5/5** | **3.02×** | 1.25 / 3.65 % | **+4.70 %** (5/5) | 4.65 / 8.83 % | **PASS** | `INSIDE-NULL-ANY-PAIR` |
| `base-classic-off` | 10.557 → 10.536 | −0.24 % (−4.83…+1.12) | 2/5 | −0.29× | 0.22 / 0.55 % | −0.11 % (1/5) | 0.18 / 0.50 % | **INERT-PASS** | **INERT-PASS** |
| **`base-classic-on`** | **28.073 → 24.241** | **+13.85 %** (+11.92…+14.83) | **5/5** | **17.31×** | 0.30 / 0.44 % | **+23.07 %** (5/5) | 0.79 / 1.29 % | **PASS** | **PASS** |
| `base-arachne-off` | 11.154 → 11.086 | +0.20 % (−0.87…+3.30) | 4/5 | 0.25× | 0.98 / 1.35 % | +0.13 % (3/5) | 0.28 / 0.74 % | **INERT-PASS** | **INERT-PASS** |
| **`base-arachne-on`** | **28.547 → 24.549** | **+14.00 %** (+13.72…+15.57) | **5/5** | **17.51×** | 0.55 / 1.71 % | **+22.62 %** (5/5) | 0.29 / 1.11 % | **PASS** | **PASS** |

*Disclosure on `benchy` supports-on vs `null` any-pair:* on a `~3.4–3.5 s` benchy cell, the entire clean Phase-3 union is only `154 ms` (`4.48 %` of cell, `t178`), so the projected ~`105 ms` (`~3 %`) gain is structurally smaller than a single `130–162 ms` Windows scheduling jitter outlier in `null` (`p2` `benchy-classic-on` `3.372 vs 3.534 s` = `4.80 %`; `p1` `benchy-arachne-on` `3.473 vs 3.600 s` = `3.65 %`). The effect is real and corroborated on every independent axis: **10/10** ordinary benchy wall wins, **10/10** ordinary benchy CPU wins (`benchy-classic-on` CPU `+7.25 %` is **7.1×** above `null` any-pair CPU `1.01 %`), both wall medians sit **1.9×–3.7×** above `null` median wall (`0.51 / 1.25 %`), and the 3-pair accelerated corroboration below wins **6/6** benchy pairs at **`+4.58 % / +3.81 %`** wall.

### Accelerated corroboration (3 interleaved pairs/cell, `reduced/accel.json`)

| Cell | Wall s (base → cand) | Wall med (wins) | CPU med (wins) |
|---|---:|---:|---:|
| `benchy-classic-off` | 1.748 → 1.745 | −0.11 % (1/3) | +2.58 % (2/3) |
| **`benchy-classic-on`** | **3.450 → 3.312** | **+4.58 %** (3/3) | **+4.79 %** (3/3) |
| `benchy-arachne-off` | 1.766 → 1.778 | −0.02 % (1/3) | −0.83 % (1/3) |
| **`benchy-arachne-on`** | **3.578 → 3.431** | **+3.81 %** (3/3) | **+3.73 %** (2/3) |
| `base-classic-off` | 10.468 → 10.738 | −2.79 % (1/3) | +0.14 % (2/3) |
| **`base-classic-on`** | **27.968 → 23.934** | **+14.64 %** (3/3) | **+24.20 %** (3/3) |
| `base-arachne-off` | 10.964 → 10.814 | +1.37 % (2/3) | +0.28 % (2/3) |
| **`base-arachne-on`** | **28.596 → 24.509** | **+13.57 %** (3/3) | **+23.39 %** (3/3) |

---

## 4. Output currency & full 8-cell G-code section audit (`reduced/output.json`, `reduced/audit.json`)

Across all **229** verified runs (`32` output + `80` A/B + `48` null + `48` accelerated + `21` sweep): **0** non-zero exits, **0** degraded runs, **0** fatal errors, **0** non-fatal errors, **0** code-1200 diagnostics. Baseline reproduces `t175`'s active `reference.json` **16/16** (`8` ordinary + `8` accelerated), and candidate ordinary is byte-identical to candidate accelerated **8/8** (**16/16** total).

| Cell | `t175` (baseline) | `t182` (candidate) | Identical? | Δ bytes / lines | Total extruding Δ mm | Non-support G1 lines checked (XY diffs / max `\|dE\|`) | Moved sections (`layers more/less/eq`) |
|---|---|---|---|---:|---:|---:|---|
| `benchy-classic-off` | `4c67ef0b…` | `4c67ef0b…` | **YES** | 0 / 0 | 0.00000 | 99,657 (`0` / 0.00000 mm) | none |
| **`benchy-classic-on`** | `fdca4e2d…` | **`a8c93535…`** | NO | +3 / 0 | **−0.01777** (−0.0000097 %) | **99,657 (`0` / 0.00025 mm)** | `Support` only: −0.01777 mm (−0.000022 %, `38/35/127` of 200) |
| `benchy-arachne-off` | `fd631384…` | `fd631384…` | **YES** | 0 / 0 | 0.00000 | 102,801 (`0` / 0.00000 mm) | none |
| **`benchy-arachne-on`** | `2869186c…` | **`7a8d87ae…`** | NO | +3 / 0 | **−0.01777** (−0.0000097 %) | **102,801 (`0` / 0.00000 mm)** | `Support` only: −0.01777 mm (−0.000022 %, `38/35/127` of 200) |
| `base-classic-off` | `02ffa2d6…` | `02ffa2d6…` | **YES** | 0 / 0 | 0.00000 | 514,432 (`0` / 0.00000 mm) | none |
| **`base-classic-on`** | `7b90e749…` | **`7e115231…`** | NO | −56 / −1 | **−0.24238** (−0.0000311 %) | **514,432 (`0` / 0.00000 mm)** | `Support` only: −0.24238 mm (−0.000062 %, `145/194/93` of 432) |
| `base-arachne-off` | `e806296f…` | `e806296f…` | **YES** | 0 / 0 | 0.00000 | 509,474 (`0` / 0.00000 mm) | none |
| **`base-arachne-on`** | `f77633af…` | **`da1e41d4…`** | NO | −56 / −1 | **−0.24238** (−0.0000311 %) | **509,474 (`0` / 0.00048 mm)** | `Support` only: −0.24238 mm (−0.000062 %, `145/194/93` of 432) |

- **Zero part-geometry movement (`1,226,364` non-support G1 motion lines checked across the 4 supports-on cells → `0` XY mismatches; max cumulative `|dE|` ≤ 0.00048 mm):** `Outer wall`, `Inner wall`, `Top/Bottom surface`, `Bridge`, `Internal bridge`, `Sparse/Solid/Gap infill`, `Skirt`, `Brim`, and even **`Support interface`** are 100 % unmoved.
- **Mechanism provenance (`t178`):** `t178` exhaustively proved `147/147` (`base`) and `27/27` (`benchy`) of component-split diffs are `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` distant-scanbeam clamping, and `k = 8` hierarchical union carries worst-case boundary Hausdorff `0.949 µm` (`base`) and `0.400 µm` (`benchy`, relative XOR area `1.64 × 10⁻⁸` and `4.93 × 10⁻⁹`), matching the `−0.000062 %` / `−0.000022 %` `Support` path delta observed here.

---

## 5. Route arithmetic (`ESTIMATE ONLY`, `reduced/route.json`)

Cross-window composition of `t174` quiet-board official ratios × `t175` paired savings × `t182` paired savings (`ESTIMATE ONLY`, never a three-tool scoreboard ratio of record):

| Cell | Est. post-`t175` (ord / acc) | `t182` paired Δ (ord / acc) | **Est. post-`t182` (ord / acc)** | Share of post-`t175` cut closed (ord) | Est. stacked with `t185` host tail (ord) |
|---|---:|---:|---:|---:|---:|
| `benchy-classic-on` (binding) | 1.5700x / 1.6016x | +1.90 % / +4.58 % | **~1.540x / ~1.528x** | 5.2 % | ~1.445x |
| `benchy-arachne-on` | 1.3885x / 1.4091x | +2.41 % / +3.81 % | **~1.355x / ~1.355x** | 8.6 % | ~1.273x |
| **`base-classic-on`** | 1.2206x / 1.1692x | **+13.85 % / +14.64 %** | **~1.052x / ~0.998x (`< 1.0x` acc!)** | **76.6 %** | **~0.970x (`< 1.0x`!)** |
| **`base-arachne-on`** | 1.2012x / 1.1533x | **+14.00 % / +13.57 %** | **~1.033x / ~0.997x (`< 1.0x` acc!)** | **83.6 %** | **~0.961x (`< 1.0x`!)** |

Both `base` supports-on cells cross `< 1.0x` on the accelerated arm (`~0.998x / ~0.997x`, estimate only) and close **76.6 % / 83.6 %** of their remaining binding ordinary gap (`~1.052x / ~1.033x`), leaving only `~3.2–4.9 %` before landing [t185](../../issues/185-support-geometry-post-planner-host-tail-acceleration.md) (`−7.75 % / −6.97 %` replay-verified host tail), [t183](../../issues/183-per-layer-tree-support-and-infill-linker-exact-acceleration.md), and [t184](../../issues/184-shell-classification-exact-acceleration.md).

---

## 6. Pre-commit gates & verification

- `cargo test -p slicer-core --features host-algos --no-fail-fast`: **PASS** (all binaries green, including 10 new `polygon_ops_spatial_union` unit/property/determinism tests).
- `cargo test -p slicer-sdk --features test --lib host_batch`: **PASS** (3/3, including new `clip_polygons_batch_large_union_and_difference_preserve_order_and_geometry`).
- `cargo test -p slicer-wasm-host`: **PASS** (all binaries green, including 33/33 unit tests with new `batch_call_audit_tdd::clip_polygons_batch_large_union_and_difference_preserve_order_and_geometry`).
- `cargo test --manifest-path modules/core-modules/tree-support-planner/Cargo.toml --no-fail-fast`: **71/71** lib + **84/84** integration across 8 targets green; only the pre-existing `t148` `slicer_module_binding_tdd::binding_surface_matches_manifest` (`@2.1.0` vs `@2.0.0`, documented in `t175/FINDINGS.md` §7) fails — untouched.
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (exit 0).
- `cargo xtask check-literals`: **PASS** (`0` violations in `0` files).
- `cargo xtask check-test-quality --report`: **PASS** (`0` findings in touched files).
- `cargo xtask build-guests --check`: **PASS** (exit `0`).
- `cargo xtask build-guests --accelerated --check`: **PASS** (exit `0`).
- Checkout-only verification: `python audit_t182.py --self-test` (**7/7** rejected), `python verify.py --self-test` (**15/15** rejected), `python verify.py` (**PASS**, including 8/8 `reference.json` hash and byte-size reconciliation against `reduced/output.json`).
