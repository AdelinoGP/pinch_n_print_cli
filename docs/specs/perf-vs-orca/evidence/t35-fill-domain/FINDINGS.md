# Benchy classic sparse-fill domain at middle layers (2026-09-29)

Ticket: [Benchy classic sparse-fill domain at middle layers](../../issues/35-benchy-classic-sparse-fill-domain.md).
This is a **localization and root-cause** finding, not a geometry fix or speed A/B.

## Question and answer

Why does matched Benchy/classic/supports-off emit a large sparse-infill path
where PNP Arachne emits almost none and the retained Orca captures emit none?

**Root cause: the `Layer::InfillPostProcess` empty-output protocol resurrects the
raw, unclipped gyroid waves.** The infill linker's own verdict on those layers is
*"nothing survives the per-role clip"* — and the host reads a linker invocation
that emitted no paths as "committed nothing" and preserves the prior `InfillIR`
verbatim, i.e. the raw wave envelope the clip was supposed to remove. The G-code
then prints a bbox-expanded gyroid envelope that reaches well outside the part
cross-section.

## Reproduction (unmodified committed tree)

`cargo xtask build-guests --check` exits 0; release host rebuilt. Then:

```
pnp_cli slice --model tmp/3dbenchy.stl \
  --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json \
  --module-dir modules/core-modules --output <file>
```

Layer index 104 (z = 21.0 mm), measured from the raw G-code:

| | PNP classic | PNP Arachne | Orca classic (retained) | Orca Arachne (retained) |
| --- | ---: | ---: | ---: | ---: |
| Sparse printed XY segments | 2,562 | 1 | 0 | 0 |
| Sparse printed XY mm | 1,741.2 | 1.7 | 0.0 | 0.0 |
| Sparse XY bbox (mm) | 100.2–159.2 × 97.4–150.5 | — | — | — |
| Wall-only bbox (mm) | 117.0–153.9 × 109.9–140.1 | — | — | — |
| Sparse claim area (mm²) | 17.7 | 46.9 | — | — |

The printed sparse path spans **59.0 × 53.1 mm** while the layer's walls span
**36.8 × 30.2 mm**: the sparse envelope reaches 16.8 mm past the wall bbox on −X
and 12.5 mm past on −Y. In the part's own frame (bed offset +124.18, +125.00)
the `SliceIR` cross-section is 37.2 × 30.6 mm and the revived sparse envelope is
67.6 × 57.2 mm.

Measuring the revived path against the layer's own `SliceIR` cross-section
(polygon test at each segment midpoint): of 1,741.0 mm of revived sparse path at
layer 104, **1,545.0 mm (88.7%) lies outside the part cross-section** and only
196.1 mm (11.3%) is inside.

The same layer's sparse claim area is **17.7 mm²**. Even at 100% density a
0.4 mm-wide path can cover at most `17.7 / 0.4 = 44.2 mm`; the printed 1,741.2 mm
is **39× that absolute upper bound** (197× the 20%-density bound of 8.8 mm). The
printed length is therefore arithmetically impossible inside the claim, which is
what makes this a containment defect rather than a fill-pattern disagreement.
(Independent cross-check: the layer's `PerimeterIR.infill_areas` — the wall
inset the claim is cut from — is 17.56 mm², giving the same ≈40× result.)

`sparse_layers.py` on the same files puts the classic-vs-Arachne sparse segment
ratio at 2,562 : 1 for this layer; the retained Orca captures record zero.

The full 240-layer census (`benchy-sparse-layers.csv`, produced by the same
script on the ticket-28 captures and reproduced byte-for-byte by the fresh
unmodified run) shows the classic burst is concentrated in layers 104–108 and
111 (2,171–2,741 segments each), with neighbouring layers at single digits.

## Mechanism (code read, probe-verified)

The chain, in pipeline order:

1. **Gyroid emits raw.** `GyroidInfill::fill_expolygon`
   (`modules/core-modules/gyroid-infill/src/lib.rs`) builds the wave over the
   gyroid bbox aligned to the wave grid, **expanded by `10 × spacing_mm`** on
   every side, and closes with the explicit comment *"Emit raw — no clipping
   (ADR-0025 degraded-not-failed)"*. Overshoot is by design: the linker owns
   clipping.
2. **The linker is supposed to remove the overshoot.** `RoleBoundaries::for_role`
   (`modules/core-modules/infill-linker/src/orchestrate.rs`) resolves the
   sparse role's own host-partitioned polygon; `link_paths_without_offset` clips
   every raw wave against the overlap-offset boundary and then drops paths under
   `0.8 × spacing_mm` (`remove_short_polylines`).
3. **On the burst layers, nothing survives the clip.** A temporary in-guest probe
   (see `probe.patch`, removed before this ticket closed) measured, for classic
   layer 104, **56 raw waves / 1,741 mm in**, and **0 paths kept** against both
   the offset boundary and the raw fallback. Arachne layer 104 took 56 raw waves
   / 1,782 mm in and kept **1 path / 2 mm**.
4. **Empty output means "committed nothing".** `deconstruct_layer_ctx`
   (`crates/slicer-wasm-host/src/dispatch.rs`) returns `Ok(None)` when the
   invocation's sparse/solid/ironing/raft buckets are all empty, and
   `layer_executor.rs`'s apply treats `None` as "the invocation committed
   nothing" — so the arena's prior `InfillIR` from `Layer::Infill` is left in
   place. That preservation rule is deliberate and documented (ADR-0028
   §Amendment *Change 3*: "a stage with zero registered modules never produces a
   commit — the prior `InfillIR` is preserved").
5. **The preserved prior `InfillIR` is the raw gyroid.** So the layer prints the
   unclipped envelope.

Steps 4–5 are a *collision between two intended behaviours*: the linker
correctly clips everything away, and the host correctly preserves the prior IR
when a stage emits nothing — but "emitted nothing after clipping" and "was not
run" are indistinguishable at this boundary, and only the second should mean
"preserve".

### Correlation

Probe outcome against the printed G-code, per layer:

- **Classic:** 60 layers where every linker call returned zero paths. Those layers
  carry **54,324 of the 79,940 printed sparse mm (68.0%)** and 72,135 of the
  106,442 printed sparse segments. Raw length fed to the linker on those layers:
  54,332 mm. All 37 G-code layers with >1,000 sparse segments are in the
  empty-output set (37/37).
- **Arachne:** 16 such layers, **7,566 of 36,219 mm (20.9%)**. All 5 of its
  >1,000-segment layers are in the set (5/5).
- **Base fixture:** classic and Arachne each log 492/493 linker layers with
  **zero** all-empty layers, and the base classic file prints *fewer* sparse mm
  than base Arachne (164,905 vs 174,242) — matching ticket 28's "base's classic
  excess is mostly wall bytes, sparse is smaller than Arachne". The mechanism
  therefore explains the fixture divergence: base's cross-sections rarely make
  the sparse clip vanish entirely, Benchy's do at these layers.

## Why "empty after clipping" is the wrong signal

The mechanism makes a **verdict** look like an **absence**:

- If the linker clipped a role's paths to nothing because the host partitioned
  that role an empty polygon (`Some(empty)` in `for_role`'s terms), the correct
  printed result is *no sparse on this layer* — the raw IR carries geometry the
  boundary forbids.
- The host cannot tell that from "the linker never ran", so it prints the raw
  geometry instead.

This is a **containment hole**, the same class ADR-0025's 2026-07-24 amendment
closed three times over in the linker itself (`for_role`'s plural boundary, the
connector-along-contour rule, per-role re-clip). The amendment states the
contract precisely: *"A known-empty role boundary is not an unknown one.
`for_role` returns `Option`: `None` means no boundary could be resolved and the
paths pass through untouched; `Some(empty)` means … the role's paths have nowhere
legal to go and clip away."* The linker honours that distinction; the
empty-output protocol then overrides it one stage later.

## Scope and non-claims

- **This is a real defect, not a benchmark artifact.** It reproduces on the
  unmodified committed tree (`verify-classic.gcode`, byte-identical segment
  counts to the ticket-28 capture), and it is visible in final G-code, not just
  in IR captures.
- **A fix is not authorized here.** The ticket asks for the first divergent
  ownership boundary; changing the empty-output/preservation protocol is a
  runtime-contract change that needs its own packet, ADR review (ADR-0028
  §Change 3 and ADR-0025's containment contract are both in scope), and the
  map's ordinary+accelerated paired A/B plus output-disclosure gates. The
  natural fix shape — distinguish "emitted nothing because every role clipped to
  nothing" from "invocation did not run", e.g. by having the linker always
  commit the (possibly empty) replacement set rather than signalling absence —
  is stated here as a **candidate**, not a recommendation, and it must preserve
  the ADR-0028 preservation rule for genuinely absent modules.
- **Geometry correctness is not adjudicated.** This finding says PNP classic
  prints a raw envelope its own linker rejected. It does *not* prove PNP Arachne
  or Orca are canonical on these layers; ticket 28's caution against presuming
  the shorter generator correct stands.
- **No wall/CPU claim.** No timing run was made; the byte and segment counts are
  an output census only. The 54,324 mm of sparse path in the classic Benchy file
  is a *work-volume* signal consistent with ticket 28's CPU-per-output-byte
  observation, not a measured speed opportunity.
- `ERR_MALFORMED_LAYER_MARKER` warnings from `machine-gcode-emit` were present
  in the ticket-28 logs and were not investigated here.

## Assets

- `probe.patch` — the temporary in-guest linker probe (`lib.rs`,
  `orchestrate.rs`), saved for reproduction and removed from the tree before
  close. It logs one line per `link_paths_without_offset` call:
  `T35-PROBE link_paths layer=… bucket=… pathsn=… in_units=… clip_units=…
  kept_n=… kept_units=… out_n=… out_units=…`.
- Raw captures, probe logs, and the verification slice live under
  `target/matched-pair/t35-fill-domain/` (gitignored; `tmp/3dbenchy.stl` and
  `tmp/base.stl` are user-supplied and licence-encumbered).
- `sparse_layers.py` and `measure_sections.py` are reused unchanged from
  `evidence/t28-output-volume/`.
