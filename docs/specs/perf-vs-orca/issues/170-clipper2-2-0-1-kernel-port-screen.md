# Clipper2 1.1.0 → 2.0.1 kernel upgrade screen for `vendor/clipper2-rust`

Type: task / attribution + screen (kernel upgrade, no blind port)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t167 canonical ClipperUtils replay](167-canonical-clipperutils-replay.md)
([`evidence/t167-canonical-clipperutils-replay/FINDINGS.md`](../evidence/t167-canonical-clipperutils-replay/FINDINGS.md)),
whose 3-way ablation isolated a `1.72x–1.76x` (`f2`) non-decimated speedup from Clipper2 `1.5.2 → 2.0.1` + wrapper on canonical `0.04 mm Miter 3` shell openings and a `1.20x–1.47x` (`f1`) Rust `1.1.0 → C++ 1.5.2` residual, widening under 12-way concurrent load.

## Question

**After edge decimation and flat-path `offset2_ex` ([t169](169-canonical-offset-edge-decimation-and-flat-opening.md)) are accounted for, what specific algorithmic and data-structure changes between `vendor/clipper2-rust` 1.1.0 and canonical `v2.5.0`'s C++ Clipper2 2.0.1 (`D:\slicerProject\Orca(pnp_gui)\deps_src\clipper2`, commit `222c6a2df5`) drive the remaining `~2.1x–2.5x` gap on canonical `0.04 mm Miter 3` shell openings (`f1 * f2 = 1.40 * 1.72 = 2.41x` bot, `1.41 * 1.76 = 2.48x` top) and the wider 12-way loaded slowdown (`1.54x–1.75x` Rust vs `1.27x–1.46x` canonical), and what is the cheapest exact port path?**

## What is already known (t167)

- **Clipper1 is ruled out:** canonical `v2.5.0` deleted Clipper1 in commit `222c6a2df5` (*"Improve performance by migrating to Clipper2 2.0.1 (#15969)"*), `orca_v2_5_0` beats `orca_clipper1` on `offset` (`0.78x` sum / `0.47x` median), `base-t68sites` (`0.45x`), `union` (`0.79x`), and canonical `SH_OPEN_*_CANON` (`2.45–2.57x` vs `2.02–2.06x`), and Clipper1 has a hairline-sliver inversion bug (`IB_DEEP_NEG` L133 & L493).
- **Non-decimated gap on canonical `0.04 mm Miter 3` shell openings:** in t167's 3-way ablation (`ablation_decimation.json`), setting `ShortestEdgeLength = 0.0` still leaves `rust_pnp / orca_v2_5_0_nodecim` at **2.41x** (`SH_OPEN_BOT_CANON`) and **2.48x** (`SH_OPEN_TOP_CANON`), splitting into:
  - `f1` (`rust_pnp` 1.1.0 → `cpp_c2_1_5_2`): **1.40x–1.41x**
  - `f2` (`cpp_c2_1_5_2` → `orca_v2_5_0_nodecim` 2.0.1 + `ClipperUtils`): **1.72x–1.76x**
  - Note: on `SH_OPEN_*_LEGACY` (`0.20 mm Round`), `f2` is `0.92x–0.98x` — the `1.72x–1.76x` jump is **specific to Miter `opening_ex` / `c2_offset_expolygons` / flat pass-1 `Paths64` / `PreserveCollinear(false)` / Clipper2 2.0.1 `ClipperOffset`**.
- **12-way concurrent scaling gap:** under 12-way concurrency, `rust_pnp` slows down `1.54x–1.75x` vs `1.27x–1.46x` for `orca_v2_5_0` on `base-t68sites` and `SH_OPEN_*_CANON`, suggesting larger allocation / memory-footprint working sets in `vendor/clipper2-rust` 1.1.0.

## Scope

1. **Diff & micro-ablate `f2` (`1.72x–1.76x`) on `SH_OPEN_*_CANON`:** separate (a) flat-path pass-1 `offset2_ex` (no intermediate `PolyTree64`), (b) `PreserveCollinear(false)` in `Clipper64` / `ClipperOffset`, (c) `Clipper2 1.5.2 → 2.0.1` diff in `clipper.offset.cpp` and `clipper.engine.cpp` (`git show 222c6a2df5 -- deps_src/clipper2/`), and (d) `c2_offset_expolygons` single-group offsetting.
2. **Audit `vendor/clipper2-rust` 1.1.0 vs Clipper2 2.0.1:** identify which 2.0.1 optimizations (e.g., `ClipperOffset` miter/group handling, active-edge/outpt arena layout, `PreserveCollinear`, redundant allocations) are missing from `vendor/clipper2-rust`, plus whether `clipper2-rust` has an upstream 2.x release or whether targeted patches to `vendor/clipper2-rust` capture ≥ 80% of `f1 * f2`.
3. **Price the ceiling on the post-t169 tree** (or on t167's replay harness + `ShellClassification` Pass 1) and graduate a concrete implementation ticket if the remaining ceiling clears the 0.80 % gate across the matrix (or close as sub-floor if t169's flat-path `offset2_ex` + decimation already captures the bulk of `f2`).

## Acceptance

- `evidence/t170-clipper2-201-screen/` with clean sub-factor replay attribution on `t167`'s `base-prepass-t167.corpus` + `base-t68sites.corpus`, source diff inventory of Clipper2 `1.1.0 → 2.0.1`, `FINDINGS.md`, and checkout-only `verify.py`.
- Decision: graduate targeted `vendor/clipper2-rust` port (with priced wall ceiling) OR close if t169 already absorbs `f2`.
- Map: destination-status block and Decisions-so-far line.

## Answer

_(pending)_
