# Proposal — Fail Loudly on Negative Flow Spacing (D-162)

**Status: IMPLEMENTED 2026-07-17** (grilled and approved; implemented directly
rather than via a packet — user decision). Written 2026-07-16 at the close of
the D-160 fix session. Ledger row: `D-162-FLOW-NEGATIVE-SPACING-SILENT-ZERO`
(now Closed). Deltas from the proposal as written: the new e2e test uses
0.1mm walls at 1.0mm layer height (both inside manifest `[min,max]` ranges —
the proposed 2.0mm layer height exceeds the arachne manifest's 1.0 max and
would be rejected at config resolution before reaching the flow math); the
`top_bottom.rs` error surfaces as `PaintSegmentationError::NegativeSpacing`.
Every canonical claim below was verified against the local OrcaSlicer checkout
(`OrcaSlicerDocumented/`) during this session; citations are by file +
function per repo convention.

## The divergence

Canonical `Flow::rounded_rectangle_extrusion_spacing` (`Flow.cpp`):

```cpp
auto out = width - height * float(1. - 0.25 * PI);
if (out <= 0.f)
    throw FlowErrorNegativeSpacing();
return out;
```

One rejection rule, an exception, and no reference to the nozzle.
`FlowErrorNegativeSpacing` is a `FlowError`, itself a
`Slic3r::InvalidArgument` (`Flow.hpp`) — the slice aborts with a config
diagnosis.

PnP `slicer_core::flow::line_width_to_spacing` returns `0.0` where canonical
throws, and its callers treat `0.0` as "no usable spacing" and fall back —
`arachne_params_from_config` (`modules/core-modules/arachne-perimeters/src/lib.rs`)
has `spacing <= 0.0 → raw_width_mm` for **both** bead targets. That fallback
feeds a raw WIDTH into a SPACING-domain strategy stack: the exact defect class
of D-160 (a width smuggled into the spacing domain, over-stating every bead
target by `layer_height·(1 − π/4)`), one layer up, and *silent*.

**Still reachable in production.** The formula only goes non-positive when
`width ≤ layer_height·(1 − π/4) ≈ 0.2146·layer_height`, but the
classic-perimeters schema allows `layer_height` up to 2.0mm, so a 0.4mm width
at a 2.0mm layer height (threshold 0.429mm) hits it. (The former
`width < layer_height → 0.0` guard, which made the branch fire ~4.7× more
often, was already removed as fabricated — see the `fix(flow)` commit of
2026-07-16.)

## Proposed design

### 1. Error type: `Result`, not a sentinel

```rust
/// crates/slicer-core/src/flow.rs
#[derive(Debug, Clone, PartialEq)]
pub struct NegativeSpacingError {
    pub width_mm: f32,
    pub layer_height_mm: f32,
    /// The non-positive result, kept for the message: width − h·(1 − π/4).
    pub spacing_mm: f32,
}

pub fn line_width_to_spacing(
    width: f32,
    layer_height: f32,
) -> Result<f32, NegativeSpacingError>
```

- Mirrors canonical's contract exactly: error **iff** `out <= 0`. No other
  rejection rule.
- `Display` message should state the fix the user can act on, e.g.
  `"line width 0.40mm is too small for layer height 2.00mm: extrusion
  spacing would be -0.03mm (width must exceed layer_height·(1 − π/4) =
  0.43mm). Increase the wall line width or reduce layer height."`
- Rust has no exceptions, so `Result` is the canonical-throw analog. **Do not**
  keep a `0.0`-returning variant alongside it — a surviving sentinel path is
  how the current fallback would quietly reassemble itself.

### 2. Drop the vestigial `nozzle_diameter` parameter

Canonical's formula does not reference the nozzle. PnP's signature takes
`nozzle_diameter` solely for a `<= 0` sanity check (already documented as
vestigial in `flow.rs`). Since every call site must be touched to adopt the
`Result` anyway, this is the moment to remove the parameter — afterwards the
signature *cannot* re-grow a nozzle clamp unnoticed. The
`width <= 0 || layer_height <= 0` defensive guards (canonical relies on config
validation upstream instead) collapse naturally: a non-positive width or
height yields a non-positive spacing, so the single canonical rejection rule
already covers them — no separate guard, no separate error variant.

### 3. Surfacing across the WIT/module boundary

Guests cannot unwind across the component boundary. The existing, already-
plumbed channel is the WIT `module-error` record
(`crates/slicer-schema/wit/deps/common.wit`):

```wit
record module-error { code: u32, message: string, fatal: bool }
```

mirrored by `slicer_sdk::error::ModuleError`. Every stage entry point
(`run-perimeters`, `on-print-start`, …) already returns
`result<_, module-error>` (`world-layer.wit`), and the host aborts the slice
when `fatal: true`.

So no WIT change is needed: the module converts the flow error into a fatal
`ModuleError` at its own boundary —

```rust
let spacing = line_width_to_spacing(w, h)
    .map_err(|e| ModuleError::fatal(ERR_NEGATIVE_SPACING, e.to_string()))?;
```

with `ERR_NEGATIVE_SPACING` a module-level error-code constant (codes are
module-specific per the `ModuleError` docs; pick the module's next free code
and document it in the manifest's comment block). `fatal: true` is correct:
canonical aborts the slice, and a non-fatal "log and continue" would just be
the silent fallback with extra steps.

### 4. The two production call sites

- **`arachne_params_from_config`**
  (`modules/core-modules/arachne-perimeters/src/lib.rs`) — delete both
  `spacing <= 0.0 → raw_width_mm` fallback branches and propagate the error as
  a fatal `ModuleError` (the function must become fallible, or perform the two
  conversions in `run_perimeters`/`on_print_start` where `Result` already
  flows). This kills the width-into-spacing-domain smuggling outright.
- **`propagate_top_bottom`**
  (`crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs`) — computes
  `shell_step = spacing + width` for multi-material shell insetting, calling
  `line_width_to_spacing(width, layer_height_mm, width)` (note: it currently
  passes `width` as the vestigial nozzle argument — further evidence for §2).
  This is host-side code (PrePass paint segmentation), so no WIT boundary is
  involved: propagate the error up through the algo's `Result` to the runtime,
  which reports it as a slice-fatal config error. Under the current sentinel
  it would compute `shell_step = 0 + width` and silently produce wrong shell
  depths — same silence, different symptom.

Test call sites (flow.rs unit tests, `flow_tdd.rs`, the parity-test derived
expectations) update mechanically to `.unwrap()` on known-good inputs, plus
new tests pinning the error boundary at exactly
`width == layer_height·(1 − π/4)` (the existing
`spacing_is_zero_only_at_canonicals_actual_threshold` test becomes
`..._errors_at_canonicals_actual_threshold`).

### 5. `flow_to_width` is out of scope

The inverse (`spacing + h·(1 − π/4)`) cannot go non-positive for positive
inputs and canonical's `rounded_rectangle_extrusion_width_from_spacing` has no
throw; leave it alone.

## Open question resolved this session: `min_bead_width` in the spacing domain

Once the beading domain is genuinely spacing, what do `min_bead_width` /
`initial_layer_min_bead_width` mean? They are config WIDTHS (percent of
nozzle diameter upstream) fed raw into the spacing-domain strategy stack.

**Verified: canonical does the same.** `WallToolPaths`'s param intake resolves
`min_bead_width` / `initial_layer_min_bead_width` as `value · 0.01 ·
min_nozzle_diameter` (a width, never spacing-converted), and
`WallToolPaths::generate` passes `min_bead_width` raw into
`BeadingStrategyFactory::makeStrategy` alongside the spacing-domain
`bead_width_0` / `bead_width_x` — plus width-domain threshold ratios derived
from it (`wall_split_middle_threshold`, `wall_add_middle_threshold`, computed
against `external_perimeter_extrusion_width` / `perimeter_extrusion_width`).
So mixing a width-derived minimum into spacing-domain targets is **faithful**;
the semantic wrinkle is upstream's, not PnP's. **No change proposed** — but
the CONTEXT.md glossary entry ("Wall line width vs. bead width vs. flow
spacing") should be the reference if anyone is tempted to "fix" it, and any
future re-derivation of the split/add thresholds must reproduce canonical's
width-domain arithmetic, not a spacing-domain "correction" of it.

## Blast radius and verification sketch (for the implementing packet)

- No fixture should move: the error path is unreachable at every fixture's
  config (all now at 0.2mm layer height, widths ≥ 0.34mm). Assert this —
  `perimeter_parity` byte-identical is the acceptance gate.
- New e2e RED test: slice with `layer_height = 2.0`, `wall_line_width = 0.4`
  and assert the slice FAILS with the negative-spacing message (today it
  silently produces over-wide beads).
- `line_width_to_spacing` has ~5 call sites total (2 production, the rest
  tests); the signature change is mechanical but touches guest code — guests
  must be rebuilt and the freshness gate run before believing any result.
