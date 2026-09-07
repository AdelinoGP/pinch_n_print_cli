# 53 — Author packet P46 — Printer / Machine / Bed mesh — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f85995a9effeoAh5BDD73KhZ7J) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P46 — Printer / Machine / Bed mesh — emitter** — 4 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P46 — Printer / Machine / Bed mesh — emitter):

`adaptive_bed_mesh_margin`, `bed_mesh_max`, `bed_mesh_min`, `bed_mesh_probe_distance`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 280** (`docs/spec_packets/280-bed-mesh-adaptive-placeholders/`,
`draft`), preflight **PASS** (no blockers; one S8 high converted to
`D-280-ADR-0050-AMENDED` with ADR carve-out). Claim-time re-derivation kept
Tier B sizing and all **4 keys in, none shed, none returned, no code change.**

- Tree grounding: all four keys zero-occurrence under `crates/`/`modules/`
  (no `bed_mesh`/`G29` logic anywhere); `machine-gcode-emit` owns the only
  live template-substitution seam (`substitute_placeholders` single-pass
  `[snake_case_key]` plus `config.keys()` sweep, `modules/core-modules/machine-gcode-emit/src/lib.rs`)
  with site-variable precedent (`layer_num`/`layer_z`/`max_layer_z` via
  `site_lookup`). `GCodeCommand::Move` carries `Option<f32>` x/y
  (`crates/slicer-ir/src/slice_ir.rs`), so an all-Move XY bbox is computable
  at `PostPass::GCodePostProcess`. `gcode_flavor` lives host-side
  (`crates/slicer-gcode/src/flavor.rs`) and does not reach PostPass modules.
- Canonical grounding (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`,
  sibling checkout): all four pass rule 3 and stay in scope —
  `PrintConfig.cpp` declares `bed_mesh_min`/`max`/`probe_distance` (`coPoint`,
  defaults `(-99999,-99999)`/`(99999,99999)`/`(50,50)`) and
  `adaptive_bed_mesh_margin` (`coFloat`, `0`); `GCode.cpp`'s placeholder setup
  computes `adaptive_bed_mesh_min`/`max`, `bed_mesh_probe_count`, `bed_mesh_algo`
  from the first-layer convex-hull bbox clamped by margin/limits (probe floor
  `1.0`, per-axis `max(3, ceil(size/dist)+1)`, `lagrange` iff product `<= 6`,
  Klipper per-axis floor `4`), consumed only by user `machine_start_gcode`
  templates (Klipper `BED_MESH_CALIBRATE`, Elegoo/Snapmaker/WonderMaker/OpenEYE
  variants). Raw inputs are never published as placeholders.
- **Owner correction** `crates/slicer-gcode` → `machine-gcode-emit`
  (ticket-47/39/27 placeholder-publication precedent): the feature is
  emission-time template computation, not host serialization. 04/05 annotation
  rides the packet's close-out step; no queue-count change.
- Packet shape: 4 manifest inputs (`float-list`/`float`, canonical defaults)
  resolve via the existing sweep; 7 derived scalars
  (`adaptive_bed_mesh_min_x/_y`, `max_x/_y`, `probe_count_x/_y`, `bed_mesh_algo`)
  join the site-variable map (no vector-index grammar change, no host-injected
  key — packet-256's alternative rejected as the wrong seam for emission-time
  values). Moves-bbox stands in for the first-layer hull (recorded
  divergence: wider on custom moves, never narrower on extrusions);
  scalar spellings stand in for canonical `{key[0]}` indexing (recorded
  migration obligation); absent `gcode_flavor` falls back to Marlin behavior.
  Every retained key has a behaviour-changing AC at non-default with a narrow
  `bed_mesh_adaptive_tdd` command; no padding/serialize edit (AC-N2).
- Preflight: S0 PASS (5 files incl. `task-map.md`), S1–S4 PASS, S5 PASS
  (all five symbols resolve), S6 PASS, S7 PASS (per-file `--test` binary, no
  aggregator), S8 carried as `D-280-ADR-0050-AMENDED` (quotes the §2
  "exactly" sentence; the `layer_num` seam already escapes the domain).
