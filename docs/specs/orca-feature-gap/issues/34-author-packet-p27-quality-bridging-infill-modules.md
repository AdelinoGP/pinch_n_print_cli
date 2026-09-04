# 34 — Author packet P27 — Quality / Bridging — infill modules

Type: task
Status: resolved
Assignee: Adelino Penedo (agent session)
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P27 — Quality / Bridging — infill modules** — 3 keys, Tier B new logic, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P27 — Quality / Bridging — infill modules):

`bridge_density`, `internal_bridge_density`, `thick_internal_bridges`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation — no packet**, under the map's "Packets are for
complex implementation only" rule. All three keys now drive live,
behaviour-changing decision points on both rectilinear-style bridge-fill holders,
each covered by a test that asserts the change at a non-default value.

The ticket was filed "Tier B new logic". Sized from the tree at claim time it is
not: the bridge decision point already existed in `rectilinear-infill`, and the
gap was one holder short of complete.

### What was already live

`RectilinearInfill::run_infill`
(`modules/core-modules/rectilinear-infill/src/lib.rs`) already selected the
internal/external twin of each key off `is_internal_bridge` and used them exactly
as canonical does: the density divides the bridge line spacing
(`bridge_spacing_mm / bridge_density`), and `thick_internal_bridges` picks
`canonical_bridging_flow`'s round-thread spacing over the flattened
`line_width_to_spacing`. All three keys were declared in
`rectilinear-infill.toml`. Only `internal_bridge_density` had a non-default test.

### What landed

- **`wave-overhangs` (the second `claim:bridge-fill` holder) read the external
  keys on internal bridges.** Its fallback rectilinear fill branches
  `region.is_internal_bridge()` for the emitted *role* but computed spacing and
  flow from `bridge_density` / `thick_bridges` / `bridge_flow` regardless. It now
  selects `internal_bridge_density` / `thick_internal_bridges` /
  `internal_bridge_flow` on that branch, mirroring canonical's `is_thick_bridge`
  and the `internal_bridge_density` override.
- **Three keys `wave-overhangs` read but never declared are now in its manifest**
  (`internal_bridge_density`, `internal_bridge_flow`, `thick_internal_bridges`),
  plus `thick_bridges`, whose read site carried a comment claiming printer
  profiles supply it. They cannot: `ConfigView::from_declared`
  (`crates/slicer-ir/src/slice_ir.rs`) whitelists by the module's own schema, so
  an undeclared key never reaches the guest and `cfg_bool(config, "thick_bridges",
  false)` always took its fallback. The comment is corrected in place.
- **`bridge_density`'s `max` was 120 in both manifests; canonical is 125.** The
  120 came from `docs/ORCA_CONFIG_REFERENCE.md`, whose row for this key is a
  stale upstream snapshot ("10-120%") — the oracle checkout's `PrintConfig.cpp`
  says `min = 10`, `max = 125` for `bridge_density` *and* `internal_bridge_density`.
  Aligned to the oracle. One more instance of the map's standing rule not to size
  anything off that reference.

### Canonical, read not assumed

`Fill::make_fills` (`Fill.cpp`) is the consumer of all three:
`params.density = bridge_density.get_abs_value(1.0)` for an external bridge
(guarded by `surface_fill.params.bridge && surface.is_external() &&
params.density > 99.0`), `params.density =
internal_bridge_density.get_abs_value(1.0)` for an internal one, both with
`dont_adjust = true`; and `is_thick_bridge = surface.is_bridge() &&
(surface.is_internal_bridge() ? thick_internal_bridges : thick_bridges)`, which
picks `layerm.bridging_flow`. `thick_internal_bridges` has a second canonical read
site in `Print::validate` (`allow_thin_bridge_width = thick_bridges &&
thick_internal_bridges` gates a "Line width too small" rejection of
`bridge_line_width`); that is config-range validation, not fill geometry, and
belongs with ticket 113 rather than here.

### Recorded divergences

- Canonical reads `internal_bridge_density` from the **object** config and
  `bridge_density` from the **region** config. This port resolves both per region
  (`slicer_sdk::config_resolution::resolve_float` + `region.config()`), a strict
  superset.
- Canonical's external-bridge override is gated on `surface.is_external() &&
  params.density > 99.0`; this port applies `bridge_density` to any non-internal
  bridge surface. The port has no sub-100% pre-bridge density to preserve, so the
  guard has nothing to protect here.

### Filed, not fixed here

- **[127 — Rule on gyroid-infill's density for solid, top/bottom and bridge
  surfaces](./127-gyroid-bridge-and-solid-fill-density.md).** `gyroid-infill`
  holds `claim:bridge-fill` (and top/bottom) and fills every role at
  `sparse_infill_density`, so none of P27's keys drive anything on that holder.
  Canonical never uses gyroid for a bridge or a solid surface. Fixing it means
  ruling on top/bottom density too, which is other packets' key set.
- **[128 — Fix the units mismatch on `percent` / `float_or_percent` config
  keys](./128-percent-key-numeric-spelling-units-mismatch.md).** Manifest bounds
  check a bare number as a percent; `get_abs_value` reads it as a fraction. For
  `bridge_density`, canonical's own spelling `100.0` passes bounds and reaches the
  guest as density 100. Class-wide, so it is not fixed under these three keys.

### Verification

`cargo test -p rectilinear-infill --test bridge_infill_emission_tdd` — 8 passed,
2 new:

- `bridge_density_spaces_external_bridge_lines` — 50% vs 100% roughly halves the
  external bridge line count
- `thick_internal_bridges_widens_internal_bridge_spacing` — thick emits strictly
  fewer internal bridge lines than thin at identical density

`cargo test -p wave-overhangs --test wave_overhangs_tdd` — 16 passed, 3 new:

- `internal_bridge_density_drives_internal_bridge_fallback_spacing` — halving the
  internal key spaces the lines; halving the **external** key changes nothing
- `bridge_density_drives_external_bridge_fallback_spacing` — the mirror
- `thick_internal_bridges_drives_internal_bridge_fallback_spacing` — thick vs
  thin on an internal bridge; `thick_bridges` must not move it

Each new test would fail if the key it names were ignored (the assertions compare
two runs that differ only in that key).

No-regression, narrow: `cargo test -p slicer-runtime --test e2e
calicat_internal_bridge` (2 passed), `--test e2e wave_overhang_bridge_fill`
(1 passed), `--test contract integrated_parity_wave_overhangs` (1 passed —
native/wasm parity, which is what the guest rebuild would break first).

Gates: `cargo clippy -p rectilinear-infill -p wave-overhangs --all-targets --
-D warnings` clean; `cargo xtask check-literals` 0 violations;
`cargo xtask build-guests` rebuilt both guests (stale the moment the modules
changed) and `cargo xtask build-guests --check` exits `0`.

Status: **P27 closed by direct implementation.** No packet authored.
