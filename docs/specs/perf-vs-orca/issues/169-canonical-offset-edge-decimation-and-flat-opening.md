# Port canonical v2.5.0 offset edge decimation and flat-path `offset2_ex` into `polygon_ops`

Type: task / measured candidate (canonical-parity wrapper port + standing gate)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t167 canonical ClipperUtils replay](167-canonical-clipperutils-replay.md)
([`evidence/t167-canonical-clipperutils-replay/FINDINGS.md`](../evidence/t167-canonical-clipperutils-replay/FINDINGS.md)),
which triggered the `>= 3x` branch on all hot offset sites and isolated edge decimation (`0.005 * |delta|`) as a `3.56x–8.27x` lever on internal-bridge offsets and `2.04x–2.10x` on `0.20 mm` openings.

## Question

**What is the paired 8-cell wall, CPU, and output/parity impact of porting canonical `v2.5.0` (`40ecc66f78`) `ClipperUtils.cpp`'s two offset-wrapper optimizations into PnP's `polygon_ops` (`crates/slicer-core/src/geom/polygon_ops.rs`):**
1. **`append_offset_path` pre-offset edge decimation** (`ClipperOffsetShortestEdgeFactor = 0.005`, dropping consecutive vertices closer than `0.005 * |delta|` before `ClipperOffset`, skipping collapsed rings `< 3` vertices); and
2. **Flat-path pass-1 `offset2_ex`** (running pass 1 `-r` into flat `Paths64` instead of constructing an intermediate `PolyTree64` and re-flattening `ExPolygon`s before pass 2 `+r`)?

## What is already known (t167)

- Across 233 unique `offset` cases (`base-prepass-t167` + `t69`'s `t68sites`), canonical `v2.5.0` `ClipperUtils.cpp` + Clipper2 2.0.1 is **4.96x–5.02x** faster single-thread (**5.16x–5.78x** under 12-way concurrent load) than `rust_pnp`, and **4.00x** faster than `t69`'s bare C++ Clipper2 1.5.2 twin.
- On `base-prepass-t167.corpus`, the t167 3-way ablation (`ablation_decimation.json`) measured the pure `append_offset_path` edge-decimation factor (`f3 = orca_v2_5_0_nodecim / orca_v2_5_0`) alone at:
  - **`IB_DEEP_NEG`** (`-1.6069 mm`, `8.03 µm` threshold): **8.27x**
  - **`IB_DEEP_POS`** (`+0.5356 mm`, `2.68 µm` threshold): **3.56x**
  - **`SH_OPEN_*_LEGACY`** (`0.20 mm`, `1.00 µm` threshold): **2.04x–2.10x**
  - **`SH_OPEN_*_CANON`** (`0.04 mm`, `0.20 µm` threshold): **1.26x–1.29x**
- **Output expectation (disclosed from t167 §6):**
  - Flat-path pass-1 `offset2_ex` should be near-exact or byte-identical; test it as its own separable arm first so any exact saving is cleanly attributed.
  - Edge decimation (`0.005 * |delta|`) is **not output-neutral**: on `IB_DEEP_NEG`, collapsing ~5 µm microscopic pinholes (instead of inflating them into 3.2 mm craters under `-1.6069 mm` erosion) raises median output area by **+6.44%** toward canonical OrcaSlicer (`0.0032%` median Δarea across all 233 offsets, `0.0365%` on `opening_ex`). This is a canonical-parity geometry change (same class as t166), not an exact refactor.
- **Note on supports-on & perimeters:** because `polygon_ops::offset` and `offset2_ex` serve **both** prepass (`ShellClassification`, `InternalBridges`, `SupportGeometry` occupancy/clip offsets, `SeamPlanning`) **and** per-layer (`Perimeters`, `Infill`, `TreeSupport`), this wrapper port hits both prepass and per-layer cells across all 8 cells of the matrix.

## Scope

1. **Arm 1 (flat-path pass-1 `offset2_ex` alone):** screen whether skipping intermediate `PolyTree64` construction on pass 1 of `offset2_ex` is byte-identical across 8/8 cells and measure its standalone wall/CPU effect.
2. **Arm 2 (canonical `append_offset_path` `0.005 * |delta|` decimation on `offset` / `offset2_ex`):** implement canonical `v2.5.0` `ClipperUtils.cpp`'s `append_offset_path` (including wrap-around trim `while high > 0 && same(path[high], path[0])`, `dist2 < shortest2`, and `< 3` vertex drop) in `polygon_ops`, matching Orca's 1 nm threshold scaled cleanly to PnP's 100 nm integer grid. Check whether guest modules call host `offset_polygons_batch` / `polygon_ops` so both host and guest paths benefit.
3. **Output & parity audit:** check all 8 cells against `t166` (or active) reference hashes, verify zero regressions in `layer_module_tdd`, `slicer-core` tests, `clippy`, `check-literals`, and adhesion/internal-bridge/shell parity metrics (quantifying section deltas and confirming microscopic pinhole cleanup rather than feature loss).
4. **Paired 8-cell campaign:** 5 interleaved pairs/cell on the ordinary arm (plus same-window null band and accelerated arm corroboration per t163), measuring both wall and process CPU, plus an instrumented prepass phase split to re-price the remaining prepass gap after the wrapper port.
5. **Re-price t129's "operand volume" shapes** on the post-decimation prepass profile so subsequent tickets only chase costs that survive canonical wrapper parity.

## Pre-declared gate

- Clears the 0.80 % wall floor and sits outside the measured null band across the matrix (process CPU corroborating in 8/8), with clean geometric parity audit and unit tests covering `append_offset_path` wrap-around, pinhole collapse, and flat-path `offset2_ex`.
- If output moves, record the new active `reference.json` per cell.

## Acceptance

- Production change + unit tests green (`cargo test`, `clippy --workspace --all-targets -D warnings`, `cargo xtask build-guests --check`).
- `evidence/t169-offset-decimation-flat-opening/` with paired campaign rows, Arm 1 vs Arm 2 decomposition, section/layer delta audit, `reference.json`, `FINDINGS.md`, and checkout-only `verify.py --self-test`.
- Map: destination-status block, Decisions-so-far line, updated active reference set, and re-priced prepass frontier.

## Answer

_(pending)_
