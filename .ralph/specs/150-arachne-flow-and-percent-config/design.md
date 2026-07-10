# Design: 150-arachne-flow-and-percent-config

## Controlling Code Paths

- Primary code paths:
  - Config type set: `crates/slicer-schema/src/lib.rs:347-355` (`VALID_CONFIG_TYPES`,
    currently not enforced live) → wire into
    `crates/slicer-scheduler/src/manifest.rs:1079` (`parse_config_field_entry`)
    and `crates/slicer-scheduler/src/config_resolution.rs:181-182`
    (`is_numeric_field_type`, note the pre-existing `int-list` inconsistency).
  - Percent value model (WIT-crossing): `crates/slicer-schema/wit/deps/config.wit:4-7`
    (`variant config-value`) gains `percent-val` / `float-or-percent-val`;
    `crates/slicer-macros/src/lib.rs:590-601` (`__slicer_adapt_config`) gains the
    matching 1:1 adapter arms; `crates/slicer-ir/src/slice_ir.rs:681-692`
    (`enum ConfigValue`) + accessors from `:798` gain `Percent`/`FloatOrPercent`
    and a `get_abs_value(key, base)` read-time resolver.
  - Flow math: `crates/slicer-core/src/flow.rs` — `line_width_to_spacing`
    (`:51`, already correct) and `bridging_flow` (`:88-94`, the 1.0 stub).
  - Module wiring: `modules/core-modules/arachne-perimeters/src/lib.rs` —
    `arachne_params_from_config` (`:108-225`, feed spacing into bead widths;
    read `layer_height`/`nozzle_diameter`) and the bridge call site (`:436-438`).
  - Manifests: `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
    (retype three keys; register `layer_height`, `nozzle_diameter`),
    `modules/core-modules/classic-perimeters/classic-perimeters.toml`
    (register `nozzle_diameter`).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/arachne_parity_gaps.rs`
  (G4/G5/G6 red tests — the arbiters; do not edit), `arachne_parity.rs` (14
  locks; do not edit), `crates/slicer-core/src/flow.rs` `#[cfg(test)] mod tests`,
  `crates/slicer-ir` config unit tests, `classic-perimeters` module tests.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference
  Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **WIT/Type Changes checklist (CLAUDE.md):** the `config-value` variant is read
  by both host (`bindgen!`) and guest (`wit_bindgen::generate!` per module). The
  new arms must be added to the WIT variant AND every exhaustive `match` on it
  (`slicer-macros __slicer_adapt_config`) or guests fail to compile. Type
  identity must match across the boundary; run `cargo build --tests` after the
  WIT edit and `cargo xtask build-guests` (not `--check`) to regenerate guests.

- **Unit-consistency hazard (the crux of AC-3):** widths in the module config
  are in slicer units (`optimal_width = mm_to_units(0.4) = 4000`) while
  `layer_height` arrives in mm (the G4 test passes `.float("layer_height", 0.2)`
  = 0.2 mm, not units). `line_width_to_spacing`'s formula `w − h·(1−π/4)` is
  linear, so it is scale-invariant ONLY if `w` and `h` share a unit. The module
  MUST convert `layer_height` to the same unit as the widths before subtracting
  (e.g. `mm_to_units(0.2) = 2000`), yielding `4000 − 2000·0.2146 ≈ 3571 units
  ≈ 0.3571 mm`. Feeding `4000 − 0.2·0.2146 ≈ 4000` (mixed units) leaves the gap
  at 0.4 mm and AC-3 fails. The red test is the falsifier — do not hand-wave the
  conversion.
- Percent resolution is **module-side at read time**, not host pre-resolution:
  Orca resolves each percent against a per-call-site base (nozzle diameter for
  `min_feature_size`, wall width for `min_width_top_surface`). The host cannot
  know the base, so `ConfigValue::Percent` survives into `ConfigView` and the
  module supplies the base at read time.

## Code Change Surface

- Selected approach: module-side read-time percent resolution + spacing wired at
  the point widths are handed to the beading pipeline + a nozzle/width/height-aware
  `bridging_flow`.
- Exact functions/manifests/tests expected to change:
  - `config.wit`: `variant config-value` gains `percent-val(f64)` /
    `float-or-percent-val(...)` cases.
  - `slicer-macros/src/lib.rs`: `__slicer_adapt_config` gains the two matching
    arms (1:1 with the WIT variant).
  - `slice_ir.rs`: `enum ConfigValue` gains `Percent(f64)` / `FloatOrPercent(f64,
    bool)` (or equivalent); new `ConfigView::get_abs_value(&self, key, base) ->
    Option<f64>`; unit tests for resolution + zero-base (AC-2, AC-N2).
  - `slicer-schema/src/lib.rs`: extend `VALID_CONFIG_TYPES`; ensure it is the
    enforced set.
  - `manifest.rs` `parse_config_field_entry`: accept the new types; parse
    `<n>%` defaults; reject malformed (AC-N1).
  - `config_resolution.rs` `is_numeric_field_type`: include the new types as
    numeric where appropriate.
  - `flow.rs` `bridging_flow`: new signature carrying nozzle diameter, bead
    width, layer height; return the round-section factor; update unit tests.
  - `arachne-perimeters/src/lib.rs`: read `layer_height`/`nozzle_diameter`;
    compute spacing and feed it as the bead width; pass bridge inputs to
    `bridging_flow`.
  - `arachne-perimeters.toml`: retype 3 keys, add 2 keys.
  - `classic-perimeters.toml`: add `nozzle_diameter`; `classic-perimeters` test
    for the read (AC-5).
- Rejected alternatives: (a) host-side percent pre-resolution — rejected, bases
  are per-call-site and often the wall width the host hasn't computed; (b) a
  new layer-kind/units wrapper type instead of reusing `line_width_to_spacing` —
  rejected, the helper already returns the correct value and has passing unit
  tests, the only gap is the caller.

## Files in Scope (read + edit)

Primary (the AC arbiters and their direct wiring):

- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: the module that
  must call `line_width_to_spacing` and the new `bridging_flow`; expected change:
  read `layer_height`/`nozzle_diameter`, feed spacing, pass bridge inputs.
- `crates/slicer-core/src/flow.rs` — role: owns `bridging_flow`; expected change:
  replace the 1.0 stub with the round-section factor + tests.
- `crates/slicer-ir/src/slice_ir.rs` — role: owns `ConfigValue`/`ConfigView`;
  expected change: percent variants + `get_abs_value` + tests.

Secondary (mechanical, small): `crates/slicer-schema/wit/deps/config.wit`,
`crates/slicer-macros/src/lib.rs` (adapter arm only), the two `.toml` manifests,
`slicer-schema/src/lib.rs`, `slicer-scheduler/src/{manifest.rs,config_resolution.rs}`,
`classic-perimeters` test. Each is a localized edit; the packet exceeds the ≤3
primary target because the percent type legitimately spans WIT + schema + IR +
macros + scheduler, but no single file carries more than one concern.

## Read-Only Context

- `crates/slicer-runtime/tests/arachne_parity_gaps.rs` — read the three target
  test bodies only (G4 `:290-325`, G5 `:344-382`, G6 `:400-422`) — purpose:
  exact assertions the fixes must satisfy.
- `crates/slicer-runtime/tests/arachne_parity.rs` — do NOT read in full (>800
  lines); grep for `precise_outer_wall` (`:518`) only — purpose: confirm it
  asserts a relative delta and survives the spacing change.
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml:38-108,257-293`
  — purpose: current key declarations to retype/extend.
- `docs/08_coordinate_system.md` — mm↔unit rule (short).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate all parity checks; never load.
- `crates/slicer-core/src/beading/**`, `skeletal_trapezoidation/**` — the
  beading engine is downstream of the width change; delegate any trace.
- `target/`, `Cargo.lock`, generated bindgen under `*/wit-guest/` — never load.
- `crates/slicer-runtime/tests/arachne_parity.rs` in full — grep only.

## Expected Sub-Agent Dispatches

- "Delegate: from `OrcaSlicerDocumented/src/libslic3r/Flow.{hpp,cpp}`, what is the
  exact `dmr` (thread diameter) formula in `bridging_flow` and how is the flow
  factor computed for `thick_bridges`? Return SUMMARY ≤200 words or one ≤30-line
  SNIPPET." — purpose: fixes AC-4's signature + factor.
- "Run `cargo test -p slicer-runtime --test arachne_parity_gaps -- <name> --exact`;
  return FACT pass/fail or SNIPPETS (assertion + ≤20 lines) on fail." — per G4/G5/G6.
- "Run `cargo test -p slicer-runtime --test arachne_parity`; return FACT
  pass/fail + list of any failing test names." — AC-6 regression lock.
- "Summarize the config-type/validation section of `docs/03_wit_and_manifest.md`;
  return FACT: is `VALID_CONFIG_TYPES` the enforced set or advisory?" — confirms
  where to wire live validation.
- "Run `cargo xtask build-guests --check`; return FACT clean/STALE + stale
  guest names." — after module/IR/schema edits.

## Data and Contract Notes

- IR/manifest contracts touched: `ConfigValue` gains variants (additive; existing
  `get_float`/`get_int` must keep returning `None` for percent values, not
  coerce); manifest config-type set widens.
- WIT boundary: **yes** — `config.wit`'s `variant config-value` gains
  `percent-val`/`float-or-percent-val` (confirmed the deciding boundary via a
  read-only dispatch: `config.wit:4-7` + adapter `slicer-macros/src/lib.rs:590-601`).
  The variant crosses host→guest per module; both the `bindgen!` host side and
  every guest's `wit_bindgen::generate!` regenerate from it, so all guests
  rebuild. `common.wit` is NOT touched (that boundary is packet 152).
- Determinism: spacing is a pure function of width/height; no scheduler impact.

## Locked Assumptions and Invariants

- The `config-value` WIT variant and `slicer_ir::ConfigValue` must stay 1:1;
  the `slicer-macros __slicer_adapt_config` match is exhaustive, so a new WIT
  arm without a matching Rust arm (or vice versa) is a compile error by
  construction — do not add a catch-all `_ =>` arm that would hide a future drift.
- `line_width_to_spacing` stays the single source of the spacing formula; the
  module must not inline a second copy.
- The 14 `arachne_parity.rs` locks are invariant (AC-6). The
  `precise_outer_wall` lock's relative-delta assertion is the reason the spacing
  change is safe; if any lock asserts an absolute wall position it must be
  surfaced, not silently rebaselined.
- `get_float("min_feature_size")` on a now-percent key returns `None` (type
  mismatch), so any existing reader that used `get_float` on these three keys
  must migrate to `get_abs_value` — audit the module for such readers before
  retyping (the three keys' current readers are in `arachne_params_from_config`).

## Risks and Tradeoffs

- **Spacing change moves every Arachne wall.** Highest regression risk; mitigated
  by AC-6 and the relative-delta nature of the surviving lock. Self-captured e2e
  baselines (D-109) may shift — re-verify, and if a baseline moves, confirm the
  new value equals the spacing-correct position before rebaselining.
- **Retyping keys that a reader still consumes via `get_float`** would silently
  zero them (get_float → None → unwrap_or(default)). The Locked-Assumptions
  audit prevents this.
- **Percent variant crossing the SDK config boundary** is the one unknown that
  could force a larger change (see Open Questions [BLOCK]).

## Context Cost Estimate

- Aggregate: `M` (schema+IR+scheduler percent work is the bulk; flow/module
  wiring is small and localized).
- Largest single step: `M` — the percent config type (spans 4 files across 3
  crates). Kept at M by delegating all cargo runs and the doc/Orca reads.
- Highest-risk dispatch: the OrcaSlicer `dmr` formula query — must return
  SUMMARY/one SNIPPET, never the file, or it blows budget.

## Open Questions

- ~~`[BLOCK]` Do the percent variants cross the module boundary?~~ **RESOLVED
  2026-07-09:** `ConfigValue` has a 1:1 WIT mirror at `config.wit:4-7`; percent
  variants require extending that WIT variant + the `slicer-macros` adapter +
  rebuilding all guests. User decision (future-proofing): keep the module-side
  read-time `get_abs_value` model and accept the WIT change on `config.wit`.
  This is now in scope, not a blocker.
- `[FWD]` Exact `bridging_flow` signature (which of nozzle diameter / bead width
  / layer height are parameters vs. read from a passed-in context) — resolve
  from the Orca `dmr` dispatch during Step 5; the red test only fixes the
  observable factor, not the signature.
