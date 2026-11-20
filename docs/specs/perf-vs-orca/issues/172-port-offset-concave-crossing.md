# Port Orca's `OffsetConcaveCrossing` into `vendor/clipper2-rust`

Type: task / measured candidate (canonical-parity port + standing gate; needs a
human t81-style ruling before landing)
Label: wayfinder:task
Status: open — **first step is the human's ruling**, then the paired gate
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t170 Clipper2 2.0.1 kernel screen](170-clipper2-2-0-1-kernel-port-screen.md)
([`evidence/t170-clipper2-201-screen/FINDINGS.md`](../evidence/t170-clipper2-201-screen/FINDINGS.md)),
which isolated the remaining offset gap to one missing Orca fork patch, measured
it on three corpora, and priced it.

## Question

**What are the paired 8-cell wall, CPU and output/parity effects of porting
canonical `v2.5.0`'s `OffsetConcaveCrossing` (`clipper.offset.cpp`, an Orca fork
patch not present in upstream Clipper2 2.0.1 or `clipper2-rust` 1.2.0) into
`vendor/clipper2-rust/src/offset.rs`, and does it win `base classic-off` and
`base arachne-off` as the t170 estimate predicts?**

## What is already known (t170)

- **The patch is the whole `f2` factor.** Applying the same ~30-line patch to
  C++ Clipper2 1.5.2 reproduces canonical 2.0.1 **byte-for-byte on 87/87**
  IB + `0.04 mm Miter 3` opening cases. Without it, 2.0.1 is **0.78–0.87x** of
  1.5.2 on those openings. The kernel version contributes nothing.
- **The Rust port without the patch** is **2.38–2.70x** canonical on the two
  canonical shell openings, **2.95–4.57x** on the classic-perimeter offsets
  (`A0/A1/B/C/E/F/G/K`), and **2.57/1.94x** on `IB_DEEP_NEG/POS`. With the patch:
  **1.04–1.30x**, **1.07–1.27x**, and **1.28/1.11x**; vertex totals converge on
  canonical (e.g. `A0` 1.082 → 1.007, `A1` 1.073 → 0.998). The scratch crate
  passes **403/403** of its own tests.
- **The patch text is frozen**:
  [`offset_concave_crossing.patch`](../evidence/t170-clipper2-201-screen/offset_concave_crossing.patch).
  It adds `offset_concave_crossing` (guards: `cos_a <= -0.999`, `sin_a * delta >= 0`,
  the two `4x²` distance tests, `delta_callback` bypass, `path[j] == path[k]`
  bypass) and calls it from `offset_polygon`; the crossing point must be
  **rounded**, not truncated, because C++ `Point64`'s constructor rounds.
- **Whole-cell estimate (post-t169 tree):** central **+16.4 %** on
  `base classic-off` (required 3.9 %), **+15.3 %** on `base arachne-off`
  (required 10.3 %), 5.2–11.4 % on the other six; conservative host-only clears
  the 0.80 % floor in **8/8** (by 5.2x–16.3x). These are two-step estimates, not
  a campaign.
- **The kernel route cannot be the whole route.** Even at zero cost for every
  host and guest offset, six cells stay short of their required cut
  (`base classic-on` 23.3 % vs 52.7 %; `base arachne-on` 21.6 % vs 50.8 %; the
  four benchy cells 10.7–26.1 % vs 39.0–58.5 %). Winning those needs a second
  phase, not a bigger kernel patch.
- **Upstream has no route.** `clipper2-rust` 1.2.0 is still a 1.5.4 port and does
  not contain the patch; there is no 2.x release. A vendor patch is the only
  exact path.
- **Geometry moves.** The patch removes the 3-point concave loops canonical does
  not emit. It is canonical's own algorithm, but it is **not output-neutral**,
  so it needs the human's t81-style exception (t169's fifth exception is the
  precedent; this would be the sixth) and a fresh reference set.

## Scope

1. **Ruling first.** Ask the human for a t81-style exception scoped to
   `vendor/clipper2-rust`'s offset path and its two consumers
   (`classic-perimeters`, `arachne-perimeters`), with the measured corpus
   numbers and the 87/87 identity as the argument. Do not edit production before
   the ruling.
2. **Port.** Apply `offset_concave_crossing.patch` to
   `vendor/clipper2-rust/src/offset.rs`, including the `.round()` conversion.
   Run the crate's own suite and PnP's `slicer-core` offset tests. Add a unit test
   that falsifies the patch: a dense concave inward offset whose output loses
   the 3-point loops and whose vertex count matches the canonical rule; a
   negative control with the patch disabled must fail it.
3. **Output.** Record the 8-cell reference set on both arms; the baseline must
   reproduce t169's set 8/8, and the candidate's deltas must be audited section
   by section (expect the `Internal Bridge` / `Support` directions t169 saw).
4. **Paired gate.** Standing 8-cell protocol, 5 interleaved pairs/cell ordinary
   plus 3/cell accelerated, process CPU corroborating, null band in the same
   window. Pre-declared gate: clears the 0.80 % wall floor in 8/8; no
   supports-off regression; `base classic-off` and `base arachne-off` must beat
   their required cuts if the t170 estimate is right. If either does not, record
   the measured price and hand back — do not weaken the gate.
5. **Disclose the residual.** t170 leaves **1.04–1.30x** per-call Rust-vs-C++
   overhead on the hot sites (rounding/arena differences), and the `DoRound`
   `std::round`-vs-`ceil` Orca patch is **not** included. Neither is required for
   the speed claim; state them.

## Pre-declared gate

- Wall floor 0.80 % in all eight cells, outside the same-window null band,
  process CPU corroborating (never contradicting); **8/8 byte-identical** on the
  accelerated arm vs the ordinary candidate; baseline reproduces t169's active
  set 8/8.
- A new reference set if output moves (it will), recorded per cell.
- The patch is applied only to `vendor/clipper2-rust`; no `polygon_ops` wrapper
  changes other than what the vendor patch requires.

## Acceptance

- Production change + tests green (`cargo test -p slicer-core --features
  host-algos`, the two perimeter modules, `clippy --workspace --all-targets -D
  warnings`, `check-literals`, `build-guests --check` both modes).
- `evidence/t172-offset-concave-crossing/` with the campaign rows, output
  hashes, delta audit, `reference.json`, `FINDINGS.md`, and a checkout-only
  `verify.py --self-test`.
- Map: destination-status block, Decisions-so-far line, updated reference set and
  route arithmetic, and an explicit statement of whether the kernel route is now
  exhausted for the six non-winning cells.

## Answer

_(pending)_
