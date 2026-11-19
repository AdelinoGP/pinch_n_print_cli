# t166 pricing probe: ShellClassification Pass 1 `apply_opening` vs canonical

Take: 2026-11-18, HEAD `2f6f1635` + [`probe.patch`](probe.patch) (env-gated,
reverted after the take). Box idle at the start (12 logical cores). This is an
attribution probe: every number is from an `--instrument-stderr` run, so **it is
never a wall claim**, and n = 2 per arm.

## Code facts

Canonical side read from the local `OrcaSlicerDocumented` checkout (`3e1daccd7c`, a nightly build), **not** the v2.5.0 pin (`40ecc66f78`) or the installed 2.5.1. Re-verify at the pin before relying on them. ClipperLib is pulled in via `#include <clipper/clipper.cpp>` (`src/libslic3r/clipper.cpp`) from deps that are not in this checkout.


| | canonical `PrintObject::detect_surfaces_type` (`PrintObject.cpp`) | PnP `apply_opening` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) |
| --- | --- | --- |
| operand | `diff_ex(layerm_slices, upper/lower lslices, ApplySafetyOffset::Yes)` | `difference(r_polys, upper/lower region polys)` |
| radius | `flow(frExternalPerimeter).scaled_width() / 10` | `line_width × 0.5` (`resolve_opening_radius`) = **0.2 mm** on the matched job |
| join | `opening_ex` → `offset2_ex`, `DefaultJoinType = jtMiter`, `DefaultMiterLimit = 3` (`ClipperUtils.hpp`) | **Round**, arc tolerance **0** (Clipper2's default is computed in 100 nm units) |
| kernel | ClipperLib (Clipper1), per-contour `ClipperOffset` with `ShortestEdgeLength = |delta| × 0.005` (`offset_expolygon_inner`, `ClipperUtils.cpp`) | Clipper2 (Rust), one whole-layer `ClipperOffset` per call (`inflate_once`, `crates/slicer-core/src/polygon_ops.rs`) |

So PnP's radius is about 5x canonical's (0.2 mm vs ≈0.04 mm), and its join differs. The
divergence was first recorded in t27 (2026-10-09) as "geometry-parity question,
small prize" and has been in the map's fog since.

## Probe arms

- **base**: the current code.
- **canon**: `offset2_ex(polys, -rr, +rr, Miter, 3.0)` with `rr = 2r/10` = 0.04 mm
  (`line_width / 10`. Canonical uses the *external-perimeter* width, so the exact
  radius must come from the matched job's resolved config. The probe value is an
  approximation.)

Cell: base classic-off (`tmp/base.stl`, `pnp-classic-supports-off.json`),
`RAYON_NUM_THREADS=12`, interleaved base, canon, base, canon.

| run | sha256 | `apply_opening` thread-time | calls | Pass 1+2 wall | `host:shell_classification` | prepass phase | slice |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base1 | `9bb18f0e` (= t151 active ref) | 53,427 ms | 974 | 5,415 ms | 11,569 ms | 16,664 ms | 20,344 ms |
| canon1 | `0a1ee3df` | 14,333 ms | 974 | 3,044 ms | 10,999 ms | 16,333 ms | 20,270 ms |
| base2 | `9bb18f0e` | 59,330 ms | 974 | 5,738 ms | 11,834 ms | 17,387 ms | 20,690 ms |
| canon2 | `0a1ee3df` | 12,598 ms | 974 | 2,736 ms | 9,699 ms | 14,549 ms | 17,890 ms |

Points through `apply_opening` (base arm): **5,201,451 in, 240,447 after the
erosion**. The erosion deletes 95.4 % of its input. The calls cost 55–61 ms each
on about 5.3k input points, which is 10–11 µs per input point.

## Readings

1. **`apply_opening` is a large share of the stage's thread-time.** 53–59 s against
   t163 §7's 104.2 s `host:shell_classification` accumulated thread-time on this
   cell. That is a cross-window comparison, so it is indicative only.
2. **Canonical parameters cut it 3.7x–4.7x** (thread-time), and the Pass 1+2 wall
   drops by 2.4–3.0 s.
3. **The stage and cell effect is not established.** The stage bracket moves
   −0.6…−2.1 s and the instrumented slice moves −0.1…−2.8 s. With canonical's
   smaller radius more top/bottom area survives, so the downstream
   internal-bridge gate and the per-layer fill do more work (canon1's remainder
   grew). n = 2 cannot separate that from noise. A paired uninstrumented campaign
   is required. That is [t166](../../issues/166-shell-opening-canonical-parity.md).
4. **The per-op cost is operand *shape*, not operand *volume*.** Even the canon
   arm is about 13 ms per call (about 2.5 µs per input point) for an opening that
   mostly deletes thin slivers. Eroding slivers past their width makes the offset
   self-invert, and the sweep pays for resolving that. Canonical runs the same
   erosion, but through ClipperLib with per-contour offsets and
   `ShortestEdgeLength`. Nobody has timed that kernel on these operands. t69/t70's
   "kernel at parity" compared Rust Clipper2 with **C++ Clipper2**, not with
   canonical's ClipperLib path. That is
   [t167](../../issues/167-canonical-clipperutils-replay.md).

## Restore

The probe is reverted (`git checkout -- crates/slicer-runtime/src/slice_postprocess_prepass.rs`).
`target/release/pnp_cli.exe` was rebuilt **with** the probe and must be rebuilt
before any campaign. `cargo xtask build-guests --check` exited 0 before the take.
The raw streams are in `target/t166-probe/` (gitignored).

## Scale

At best this is a tranche on the two base supports-off cells, the same size class as t132/t133/t158. Even the full Pass 1+2 wall delta (2.4–3.0 s, instrumented) is under half of base classic-off's required cut. It does not touch the supports-on cells, which carry the binding cut.
