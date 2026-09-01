# 17 — Author packet P10 — Strength / Top/bottom shells — infill modules

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa4ada734ffeh4QFhZNEOkxsuy) — claimed 2026-09-01, resolved 2026-09-01
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P10 — Strength / Top/bottom shells — infill modules** — 4 keys, Tier A plumbing, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P10 — Strength / Top/bottom shells — infill modules):

`bottom_surface_density`, `bottom_surface_pattern`, `top_surface_density`, `top_surface_pattern`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/264-top-bottom-surface-keys/` authored (`draft`), preflight
**PASS** (S0–S8 all green, report in the packet dir). **Two keys wired, two declared-with-gap.**
Canonical grounding (delegated reads) verified all four keys exist on `PrintRegionConfig`
and re-derived the dispositions: **`top_surface_density` / `bottom_surface_density`
(coPercent 100; top min 0, bottom min 10) are wired** into the rectilinear top/bottom solid
spacing decision points (`solid_spacing = line_width / SOLID_DENSITY` in
`rectilinear-infill/src/lib.rs`, `SOLID_DENSITY = 1.0` — canonical `FillLine.cpp`
`FillLine::_fill_surface_single`'s `line_spacing = flow.spacing() / density` shape),
exposed-surface-only (canonical `group_fills` gives `stInternalSolid` a fixed `100.f`),
with the canonical `density <= 0` skip wired as a `density > 0` gate on the top block
(bottom gate provably inert under min 10) — defaults 100 → fraction 1.0 → byte-identical
(AC-2), non-default values change spacing (AC-3). **`top_surface_pattern` /
`bottom_surface_pattern` (coEnum, 8 values; defaults `monotonicline` / `monotonic`) are
declared-with-gap** — filler selection is module identity (packet 262's finding, unchanged
for the surface roles); canonical's other pattern reads (extra-internal-solid-fill branch,
`GCode.cpp` `_needSAFC`/`retract`) and the density keys' surface-expansion gates
(`detect_surfaces_type`, `top_fill_replaces_inner_walls`) are recorded, not wired. **One
padding correction**: `("top_surface_pattern", "monotonic")` → `"monotonicline"` in
`ORCA_CONFIG_PADDING` (ticket-14/262 precedent; the bottom twin already matches). The 4
tables land in `rectilinear-infill.toml` only; gyroid's ADR-0027 opt-in solid path rides
the sparse density (recorded divergence, not wired — wiring would change gyroid solid at
defaults) and lightning is sparse-only; both omissions pinned (AC-N2). Guard is the
net-new `top_bottom_surface_config_schema_tdd.rs` (distinct from 262/263's guards — no
collision; `toml = "0.8"` dev-dep add-if-absent; shared-manifest append churn with 262/263
recorded as queue-order merge churn). Zero deviation rows (enum defaults never enter the
numeric comparison map; `100%` fails `parse::<f64>` — block stays at 26, re-measured
2026-09-01). No user rulings required. Ledger facts re-derived at authoring: next packet
number 264 (disk-derived), deviation rows 26 (measured 2026-09-01). Unblocks nothing
downstream; P13 (ticket 20) is the next unblocked queue head. Two fog items graduated to
the map's Not yet specified: the gyroid solid-density path and canonical's
extra-internal-solid-fill machinery.

## Answer
