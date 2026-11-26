# 15 — Author packet P08 — Strength / Infill — infill modules

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa4d14e5cffeJeUdJotuH6lNhM) — claimed 2026-09-01, resolved 2026-09-01
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P08 — Strength / Infill — infill modules** — 7 keys, Tier A plumbing, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P08 — Strength / Infill — infill modules):

`fill_multiline`, `gap_fill_target`, `internal_solid_infill_pattern`, `solid_infill_direction`, `solid_infill_rotate_template`, `sparse_infill_pattern`, `sparse_infill_rotate_template`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Resolved 2026-09-01 — packet `docs/spec_packets/262-infill-pattern-keys/` authored (`draft`), preflight PASS** (0 blockers, 0 high; report in the packet dir).

**Four keys wired (default-path identity), three declared-with-gap.** Authoring-time grounding re-derived every decision point from code (tier table not trusted):

- **Wired — `solid_infill_direction`** (float 45 [0,360]): the solid-role angle read in `RectilinearInfill::from_config` / `GyroidInfill::from_config` — solid roles (Top/Bottom/InternalSolid) use this angle, sparse keeps `infill_direction`; default 45 = 45, byte-identical.
- **Wired — `sparse_infill_rotate_template` / `solid_infill_rotate_template`** (string ""): per-layer angle from a comma-separated list cycled by layer index (canonical `Fill.cpp::calculate_infill_rotation_angle` list form); the metalanguage (joints/repeats/units) is declared-with-gap — metalanguage strings fall back to the base angle with a logged warn. Default "" = base angle, identity.
- **Wired — `fill_multiline`** (int 1 [1,10]): sparse-only multiline in rectilinear (canonical `Layer::make_fills` `erInternalInfill` branch + `multiline_fill` offset lists + `fill_surface_by_multilines` spacing); base scan spacing × N, N copies at perpendicular offsets of the sparse line width, clipped to the region. Default 1 = single line, identity. Gyroid/lightning declared-with-gap (curve/tree-segment offsetting is Tier B+).
- **Declared-with-gap — `sparse_infill_pattern`** (enum, 26 canonical values, default `crosshatch`): the port's pattern IS module identity (rectilinear/gyroid/lightning each implement one family; the host selects via `*_fill_holder`); the port-side decision point is host-side holder resolution, recorded. **Divergence recorded:** port default sparse = rectilinear (holder default) vs canonical `crosshatch`.
- **Declared-with-gap — `internal_solid_infill_pattern`** (enum, 8 top-fill values, default `monotonic`): same module-identity finding. **Divergence recorded:** port solid fill = rectilinear scan-line generator vs canonical `monotonic` filler class.
- **Declared-with-gap — `gap_fill_target`** (enum, everywhere/topbottom/nowhere, default `nowhere`): gates canonical's **fill-side** gap fill (`FillBase.cpp::Fill::_create_gap_fill`); the port has no fill-side gap fill — its gap fill is the perimeter-side `process_classic` mechanism (classic-perimeters/arachne-perimeters, `filter_out_gap_fill` gate), which canonical's `gap_fill_target` does not gate either.

**17 manifest tables** (rectilinear 7, gyroid 7, lightning 3 — the sparse keys only; the lightning omission of the 4 solid keys pinned by AC-N2). **One padding correction:** `ORCA_CONFIG_PADDING`'s `("sparse_infill_pattern", "grid")` → `"crosshatch"` (ticket 14 precedent — the padding value contradicted canonical); `("gap_fill_target", "nowhere")` matches and stays. **No deviation rows** (numeric defaults 1/45 match canonical; enum/string defaults never enter the numeric comparison map in `render_deviations` — block stays at 26). **No user rulings required** (every wired key is default-path identity; every declared key carries canonical defaults/bounds). ADR-0027 conformance stated (no holder-default change, no gyroid emission removal).
