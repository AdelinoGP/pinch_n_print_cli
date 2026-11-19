# Canonical-kernel replay: how fast are Orca's own ClipperUtils ops on PnP's prepass operands?

Type: task / measurement (canonical side, no PnP change)
Label: wayfinder:task
Status: resolved
Resolved: 2026-11-19 — Branch A fires (`canonical_ge_3x_faster_on_hot_offset_sites`): canonical `v2.5.0` (`40ecc66f78`) migrated off Clipper1 onto Clipper2 2.0.1 (`222c6a2df5`) and is **4.96x–5.02x** faster on `offset` (**5.16x–5.78x** 12-way loaded; `IB_DEEP_NEG` **12.28–12.76x**, `IB_DEEP_POS` **4.56–4.64x**, `base-t68sites` **3.92–3.97x** single / **4.87–5.12x** loaded) and **2.45x–2.59x** faster on `opening_ex` (**2.79x–3.26x** loaded on canonical `0.04 mm Miter 3`), while booleans remain at parity (**0.79x–1.41x**). 3-way ablation isolates `append_offset_path` edge decimation (`0.005 * |delta|`, **3.56x–8.27x** on `IB_DEEP_*`) plus Clipper2 2.0.1 / flat-path `offset2_ex` (**1.72x–1.76x** on `SH_OPEN_*_CANON`) and rules out Clipper1. Graduated [169](169-canonical-offset-edge-decimation-and-flat-opening.md) and [170](170-clipper2-2-0-1-kernel-port-screen.md); zero PnP production change.
Assignee: 2026-11-19 Antigravity wayfinder session — the human said "Continue the perf vs orca map" (no ticket named); 167 is first on the frontier.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the [t166 probe](../evidence/t166-shell-opening-parity/FINDINGS.md)
§4, which challenges the premise of the operand-volume program (t129 → t131).

## Question

**If PnP's captured prepass operands run through canonical's *actual*
implementation (`ClipperUtils.cpp` on ClipperLib/Clipper1, with
`offset_expolygon_inner`'s per-contour offsetting and `ShortestEdgeLength`), how
does the per-op time compare with PnP's Rust Clipper2 on the same operands with
the same parameters?**

The map has read "the gap is operand volume" since t131. That reading rests on
t70's claim that the kernel is at parity. But t69/t70's twin was **C++ Clipper2**
(`driver.cpp` includes `clipper2/clipper.h`), and canonical's hot path is not
Clipper2: `ClipperUtils.cpp` uses `ClipperLib::Clipper` / `ClipperLib::ClipperOffset`,
and only a few sites call `Clipper2Utils`. Canonical's operand sizes were also
inferred, never measured. The t166 probe measured 10–11 µs per input point on
the shell opening, and the cost there comes from operand shape (inverted
sliver erosion), not volume. If canonical's kernel and wrapper are several times
faster on identical operands, the route is a kernel/wrapper port, not a hunt for
smaller operands.

## Scope

1. **Build a canonical driver** against the local OrcaSlicer checkout's
   `ClipperUtils.cpp` + its ClipperLib (the t69 harness pattern: one process per
   case, warmup + 5 timed repeats, and parse/flatten outside the timer). Call
   `offset_ex`, `offset2_ex`/`opening_ex`, `diff_ex`, `intersection_ex` and
   `union_ex` exactly as canonical does. Scale coordinates ×100 to Orca's 1 nm
   units.
2. **Corpus**: t69's existing `t68sites` + `t55` cases, plus a new capture of the
   ShellClassification Pass 1 openings (`apply_opening` operands, both PnP's and
   canonical's parameters) and the `gate_internal_bridge_sites` offsets on base
   classic-off.
3. **Three columns per case**: PnP Rust Clipper2 (current wrapper), C++ Clipper2
   (t69's twin, for continuity) and canonical ClipperUtils. Report the medians
   per op class and per site, with output area/vertex deltas so that a fast but
   wrong kernel is visible.
4. **Single-thread and 12-thread loaded**: the stages run at 8–12x parallelism,
   so also run the corpus under a 12-way concurrent load to expose
   memory-bandwidth effects.
5. **No PnP change.** This verifies the premise; adoption is a separate ruling.

## Decision this unlocks

- Canonical is **≥ 3x faster** per op on the hot sites: graduate a
  kernel/wrapper port (per-contour offsetting, edge decimation, or a Clipper1
  port) as the prepass route, and re-price t129's "operand volume" shapes.
- Parity (within 1.5x): t131's reading stands. The remaining lever is work and
  operand shape, and parameter-parity audits (like t166) are the productive
  search.

## Acceptance

- `evidence/t167-canonical-clipperutils-replay/` with the driver source,
  provenance (Orca checkout commit, compiler, flags), the per-case results and a
  `verify.py`.
- A map destination-status block stating which branch fired.

## Answer

**Branch A fires (`canonical_ge_3x_faster_on_hot_offset_sites`).** Full evidence, driver sources, provenance, reduced per-case JSONs, and checkout-only verifier (`verify.py --self-test` PASS, 6/6 negative controls rejected) live in [`evidence/t167-canonical-clipperutils-replay/FINDINGS.md`](../evidence/t167-canonical-clipperutils-replay/FINDINGS.md).

1. **Premise correction at the canonical `v2.5.0` pin (`40ecc66f78`, 2026-11-07):** OrcaSlicer **migrated `ClipperUtils.cpp` completely off Clipper1 onto Clipper2 2.0.1** on 2026-10-02 (`222c6a2df5`, `#15969`, deleting `deps_src/clipper/`). Ticket 167's question was written against the older `OrcaSlicerDocumented` checkout (`3e1daccd7c`, ClipperLib 6.2.6). We therefore replayed **four** engines across all 413 cases (295 from `t69` + 118 newly captured `ShellClassification` Pass 1 `apply_opening` and `gate_internal_bridge_sites` operands from `base classic-off`, output-verified byte-identical to `t166`'s `0a1ee3df…` reference): `rust_pnp` (`vendor/clipper2-rust` 1.1.0 + `mimalloc`), `cpp_c2_1_5_2` (t69 continuity twin), `orca_v2_5_0` (canonical `v2.5.0` `ClipperUtils.cpp` + Clipper2 2.0.1), and `orca_clipper1` (pre-2.5.0 `ClipperUtils.cpp` + ClipperLib 6.2.6).
2. **Headline single-thread replay (fresh process/case, 1 warmup + 5 repeats, forward / reverse, 405 unique cases):**
   - **Booleans remain at parity** (`rust_pnp / orca_v2_5_0`): `difference` **0.79x / 0.78x** (Rust faster), `intersection` **1.24x / 1.24x**, `union` **1.39x / 1.41x** (`cpp_c2_1_5_2 / orca_v2_5_0` = **0.95x–1.00x**).
   - **Offsets are 3.0x–12.8x faster in canonical `orca_v2_5_0`:** across all 233 unique `offset` cases **4.96x / 5.02x** (`cpp_c2_1_5_2 / orca_v2_5_0` = **4.00x**); `IB_DEEP_NEG` (`-1.6069 mm`) **12.76x / 12.28x**, `IB_DEEP_POS` (`+0.5356 mm`) **4.56x / 4.64x**, `t69`'s `base-t68sites.corpus` **3.92x / 3.97x** (every individual offset site `A0`–`K` is **3.02x–6.08x**), `benchy-t68sites.corpus` **3.11x / 3.13x**. This explains why `t69`/`t70` missed the gap: `t69` compared Rust 1.1.0 only against *bare* C++ Clipper2 1.5.2 (`1.24x–1.27x`).
   - **Shell openings (`opening_ex`):** **2.49x / 2.55x** overall (`SH_OPEN_TOP_CANON` **2.45x / 2.57x**, `SH_OPEN_BOT_CANON` **2.45x / 2.56x**, `SH_OPEN_*_LEGACY` **2.43x–2.59x**).
3. **12-way concurrent loaded replay (320 base cases × 12 workers, cyclically staggered, forward / reverse):**
   - Whole-batch wall: `rust_pnp` **77.13 / 64.36 s** vs `orca_v2_5_0` **23.09 / 21.75 s** (**3.34x / 2.96x**; `rust_pnp` forward started at 57% host CPU load, reverse at 1% load).
   - Load widens the offset and opening gap in canonical's favor because `rust_pnp` degrades **1.54x–1.75x** vs **1.27x–1.46x** for `orca_v2_5_0`: `opcode/offset` **5.78x / 5.16x**, `base-t68sites` **5.12x / 4.87x**, `SH_OPEN_TOP_CANON` **3.26x / 2.91x**, `SH_OPEN_BOT_CANON` **3.00x / 2.79x**, `IB_DEEP_POS` **5.19x / 4.49x**, `IB_DEEP_NEG` **12.62x / 11.36x**.
   - **Allocator disclosure:** `rust_pnp` uses `mimalloc` whereas the C++ shims use MinGW CRT `malloc` (not Orca's `tbbmalloc`), so all measured C++ speedups are conservative lower bounds.
4. **3-way mechanism ablation (`ablation_decimation.json`, 118 prepass cases, 10 repeats/case):**
   - **Edge decimation (`append_offset_path`, `0.005 * |delta|`)** alone delivers **8.27x** on `IB_DEEP_NEG`, **3.56x** on `IB_DEEP_POS`, **2.04x–2.10x** on `0.20 mm` Round openings, and **1.26x–1.29x** on `0.04 mm` Miter openings.
   - **Clipper2 `1.5.2 → 2.0.1` + flat-path `offset2_ex` pass 1 (zero decimation)** alone delivers **1.72x–1.76x** on `SH_OPEN_*_CANON` (`0.04 mm Miter 3`) and **1.13x–1.29x** on `IB_*`.
   - **Rust 1.1.0 → C++ 1.5.2** accounts for **1.09x–1.47x**.
5. **Geometry-delta audit & Clipper1 ruled out:**
   - `orca_v2_5_0` vs `rust_pnp` has **0 empty mismatches** (28/28 match) and **0.0044%** median relative area delta across all 405 unique cases (`0.0032%` on `offset`, `0.0365%` on `opening_ex`). The one systematic area delta — `IB_DEEP_NEG` `+6.44%` median (`max 49.21%`) — comes from `append_offset_path` collapsing microscopic ~5 µm pinholes (`0.000025 mm²`) instead of inflating them into 3.2 mm craters under `-1.6069 mm` erosion (without decimation, `orca_v2_5_0_nodecim` matches `rust_pnp` to `0.0047%` median area).
   - **Clipper1 (`orca_clipper1`) is ruled out:** canonical itself deleted Clipper1 (`222c6a2df5`), `orca_v2_5_0` beats `orca_clipper1` on `offset` (`0.78x` sum / `0.47x` median), `base-t68sites` (`0.45x`), `union` (`0.79x`), and canonical `SH_OPEN_*_CANON`, and Clipper1 exhibits a degenerate hairline-sliver inversion bug on `IB_DEEP_NEG` L133 & L493 (`2` empty mismatches, inflating a 0.1 µm sliver into 10.85 / 4.81 mm² blobs).
6. **Graduated tickets (zero PnP production change in t167; `git diff crates/` empty, capture hook preserved as `probe-t167-dump.patch`):**
   - [169 — Port canonical v2.5.0 offset edge decimation and flat-path `offset2_ex` into `polygon_ops`](169-canonical-offset-edge-decimation-and-flat-opening.md) (AFK, unblocked).
   - [170 — Clipper2 1.1.0 → 2.0.1 kernel upgrade screen for `vendor/clipper2-rust`](170-clipper2-2-0-1-kernel-port-screen.md) (AFK, unblocked).

