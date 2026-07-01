# ADR-0027 — PnP Gyroid-Infill Is Multi-Role (Can Fill Top/Bottom/Bridge); OrcaSlicer's Is Sparse-Only

## Status

Proposed (lands with the gyroid-infill parity rewrite).

## Context

OrcaSlicer uses gyroid for **sparse infill only** (`src/libslic3r/Fill/Fill.cpp:926`).
Top, bottom, and internal-solid shells use a separate pattern (default
`ipMonotonic` or `ipRectilinear`, selected via `top_surface_pattern` /
`bottom_surface_pattern` / `internal_solid_infill_pattern` at `Fill.cpp:941-959`).
Bridge areas use bridge-flow rectilinear. The gyroid wave geometry is not
applied to solid shells.

The PnP `gyroid-infill` module's `src/lib.rs` (lines 180-210) contains an
`emit_polys` block that emits gyroid waves for **all four** fill roles: sparse,
top solid, bottom solid, and bridge. However, the module's manifest
(`gyroid-infill.toml:21`) declares **only `claim:sparse-fill`**. Because
`SliceRegionView::should_emit(role)` (crates/slicer-sdk/src/views.rs:466-482)
gates emission on the held-claim set, and the manifest only claims sparse, the
top/bottom/bridge emission code is **dead** — `should_emit(TopSolidInfill)` etc.
return `false` at runtime. The host's per-role fill-holder dispatch
(`ResolvedConfig.{top,bottom,bridge}_fill_holder`, all defaulting to
`"rectilinear-infill"` per `crates/slicer-ir/src/resolved_config.rs:624-630`)
routes solid shells to rectilinear-infill, never to gyroid.

So as of 2026-07-01, gyroid-infill emits only sparse — matching OrcaSlicer's
behavior — but the code contains dormant multi-role emission that cannot fire.

A grilling session (2026-07-01) surfaced this contradiction. The project owner
chose to make the multi-role emission **real**: add the three solid claims to
the manifest so a user who sets `top_fill_holder = "gyroid-infill"` (etc.)
actually gets gyroid waves for solid shells. This is a deliberate user-option
divergence from OrcaSlicer, not a port bug.

## Decision

1. **`gyroid-infill.toml` gains three claims** in addition to `claim:sparse-fill`:
   - `claim:top-fill`
   - `claim:bottom-fill`
   - `claim:bridge-fill`
   
   The manifest's `claims.holds` list becomes all four. This makes the existing
   top/bottom/bridge emission code in `src/lib.rs:180-210` actually fire when
   the user configures the module as the holder for those roles.

2. **The user opts in via fill-holder config.** The default config keeps
   `top_fill_holder` / `bottom_fill_holder` / `bridge_fill_holder` =
   `"rectilinear-infill"`, so default behavior is unchanged (gyroid sparse
   only, matching OrcaSlicer). A user who wants gyroid solid shells sets:
   ```json
   { "top_fill_holder": "gyroid-infill",
     "bottom_fill_holder": "gyroid-infill",
     "bridge_fill_holder": "gyroid-infill" }
   ```
   and the module's emission code fires for those roles.

3. **The existing `emit_polys` block stays** (it is already correct for the
   multi-role case; it was just unreachable). No new emission code is needed.
   The `solid_fill_role` mapping (depth-0 = exposed Top/BottomSolidInfill,
   deeper = InternalSolidInfill) already handles the shell-depth distinction.

4. **A DEVIATION_LOG entry (DEV-082) records this as a deliberate divergence**
   from OrcaSlicer. The divergence is opt-in: default config matches OrcaSlicer
   (gyroid sparse-only); the user must explicitly configure multi-role to
   activate it.

## Consequences

**Positive**:
- Users gain an opt-in "gyroid-for-solid-shells" option. Some aesthetic and
  mechanical use cases (organic prints, flexible filaments) benefit from gyroid
  solid shells over monotonic lines.
- The dormant emission code becomes live and tested, rather than dead and
  untested. The existing `emit_polys` block gains real test coverage.
- Default behavior is unchanged — no existing print regresses. The divergence
  is purely additive (new claims on the manifest; no removal).

**Negative**:
- Diverges from OrcaSlicer parity. A user comparing PnP gyroid output to
  OrcaSlicer gyroid output under the same density/angle will see different
  solid-shell geometry if multi-role is configured. Documented in
  DEV-082 so the divergence is not mistaken for a bug.
- Gyroid solid shells are not 100% dense (the wave pattern leaves gaps at the
  surface). For top/bottom surfaces where optical finish matters, this
  produces a wavy surface instead of a smooth one. The user is responsible for
  knowing this when they opt in.
- The module now holds four claims, so the dispatcher's per-region claim
  resolution can route any of the four roles to it. The
  `docs/04_host_scheduler.md:378` rule that "a single module may hold multiple
  fill-role claims" already permits this; no scheduler change is needed.

**Trade-offs we explicitly accept**:
- Gyroid solid shells are geometrically valid but surface-finish-suboptimal.
  This is the user's choice, not the slicer's default. The slicer's job is to
  produce the geometry the user asked for, not to refuse a valid configuration.
- The divergence is opt-in only. Default config matches OrcaSlicer. We do not
  ship gyroid-for-solid-shells as the default.

## Future-Reviewer Notes

- **Do not remove the top/bottom/bridge emission from gyroid-infill "to match
  OrcaSlicer."** It is a deliberate opt-in user option (ADR-0027 + DEV-082).
  Removing it closes the option. If OrcaSlicer adds a similar option later, the
  divergence closes naturally; until then, PnP offers it.
- **Do not change the default fill-holder config** to point solid roles at
  gyroid. The default stays rectilinear for solid shells.
- **The `solid_fill_role` mapping is shared** between rectilinear and gyroid
  (both have a copy). If it diverges, the divergence is a separate concern from
  this ADR.

## References

- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` — Architecture A (modules emit raw; the gyroid rewrite follows this).
- `docs/DEVIATION_LOG.md` — DEV-082 (this divergence).
- `modules/core-modules/gyroid-infill/gyroid-infill.toml:21` — current single claim.
- `modules/core-modules/gyroid-infill/src/lib.rs:180-210` — existing multi-role emission (currently dead).
- `crates/slicer-sdk/src/views.rs:466-482` — `should_emit` held-claim gate.
- `crates/slicer-ir/src/resolved_config.rs:624-630` — default fill-holder config.
- `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` — fill-holder CLI binding tests.
- OrcaSlicer `src/libslic3r/Fill/Fill.cpp:926-959` — OrcaSlicer's sparse-only gyroid + separate solid pattern selection.
- `docs/04_host_scheduler.md:378` — "a single module may hold multiple fill-role claims."