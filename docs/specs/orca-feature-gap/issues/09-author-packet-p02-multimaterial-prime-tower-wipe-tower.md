# 9 — Author packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower

Type: task
Status: resolved
Assignee: wayfinder session (ses_fb4f7b5a6ffe8gFxrBv0CIT36X)
Blocked by: 06, 100 (both resolved at claim time)
Map: ../map.md

## Question

Author the spec packet for **P02 — Multimaterial / Prime tower (1/2) — wipe-tower** — 13 keys, Tier A plumbing, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower):

`enable_filament_ramming`, `enable_tower_interface_cooldown_during_tower`, `enable_tower_interface_features`, `filament_tower_interface_pre_extrusion_dist`, `filament_tower_interface_pre_extrusion_length`, `filament_tower_interface_print_temp`, `filament_tower_interface_purge_volume`, `filament_tower_ironing_area`, `prime_tower_brim_width`, `prime_tower_enable_framework`, `prime_tower_flat_ironing`, `prime_tower_infill_gap`, `prime_tower_skip_points`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Packet `docs/spec_packets/254-prime-tower-keys-wipe-tower/`** — authored per this
ticket, `/spec-review --preflight` **PASS** (reviewer re-verified the three
P2/P3 findings fixed: context-discipline marker, AC-N2 no-existing-arm
phrasing, gate-list duplication). Status `draft` per ticket 06's rule; the
packet number was derived from disk at authoring time (253 was the highest
committed directory; 253 itself is this map's P01 packet from ticket 08, still
uncommitted at claim time — the queue's numbering rule tolerates that).

**Key set honored:** all 13 keys from `05-asset-packet-list.md`'s P02 row,
declared in `modules/core-modules/wipe-tower/wipe-tower.toml` with Orca-parity
defaults/bounds (AC-1 pins the 21-key manifest).

**Authoring-time grounding that reshaped dispositions** (the tier table's
mechanical proxy was re-derived in code, per this ticket's obligation):

- **One live decision point, not 13.** In-tree survey: none of the 13 keys is
  declared or read anywhere today; only `prime_tower_infill_gap` has a
  decision point the port's simplified tower actually possesses — its
  scan-line advance is hardcoded `y += line_width` (`generate_purge_paths`). The
  packet wires exactly that one (canonical `WipeTower.cpp` ctor +
  `align_perimeter`: pitch = `(gap/100) × m_perimeter_width`) and records
  decision-point gaps for the other 12 (packet 253's
  `dont_slow_down_outer_wall` disposition pattern): the interface-feature
  cluster (`WipeTower2::set_extruder`, `toolchange_ChangeExtruder`), the
  ramming unload (`toolchange_Unload`), framework walls (ctor
  `m_tower_framework` → `generate_wipe_tower_blocks`), first-layer brim with
  `get_auto_brim_by_height` Auto resolution, flat-ironing passes
  (gap-wall-conditional), and `prime_tower_skip_points` travel-avoid — which
  canonical declares a **plain bool**, not a point list (verified against the
  def line; the name is misleading).
- **Unlike P01, this packet is output-changing at defaults:** pitch
  `line_width` → `(150/100) × line_width` = 0.6 mm at the default 0.4; AC-2
  owns the baseline fallout inside the module crate. The pitch-basis
  divergence (port pitches off `line_width`; canonical off
  `nozzle_diameter × Width_To_Nozzle_Ratio`) is recorded in `design.md` as
  deliberate — no nozzle→perimeter-width pipeline exists at this stage.
- **Per-filament vectors go scalar-global:** six keys are canonical
  `coFloats`/`coInts`; the queue's ticket-04 ruling (11 filament keys are
  global) applies, and the build-out defers to the map's Tier-D fog. Named in
  `requirements.md` §Out of Scope.
- **Transport verified, not assumed:** percent-typed schema defaults thread
  into `ResolvedConfig.extensions` (packet-185 machinery,
  `ConfigBoundsIndex::schema_defaults()` → `resolve_global_config`), so the
  one percent key rides every CONFIG_BLOCK; user-supplied values of all 13
  declared keys ride the extensions bucket. Non-percent defaults stay
  manifest-side with module-read fallbacks (pinned as an architecture
  constraint: do not widen the threading).

**Grounding cost note:** no `[BLOCK]` questions; no deviation rows filed (both
divergences are behavioral and recorded, and neither is unverifiable — the
ticket-02 standard consumed no human sign-off).

**Gates at authoring time:** `cargo xtask build-guests`/`--check` NOT run —
packet authoring touched no guest-affecting path; the packet's own
Verification list (Step 1's freshness gate onward) governs its swarm. No
crate/manifest/code edit happened in this session; nothing to compile.

### Canonical-path note (re-derived, not quoted from ticket 08)

The OrcaSlicerDocumented checkout for this clone is the **sibling**
`F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-verified to
exist at first dispatch in this session. Future tickets must re-derive, not
quote this line.
