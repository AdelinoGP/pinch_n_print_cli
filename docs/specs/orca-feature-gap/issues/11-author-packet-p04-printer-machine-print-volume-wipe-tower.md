# 11 — Author packet P04 — Printer / Machine / Print volume — wipe-tower

Type: task
Status: resolved
Assignee: wayfinder session (ses_fab6d601bffejnYq9di5vN51d8) — claimed 2026-08-30
Blocked by: 06, 100
Map: ../map.md

## Question

Author the spec packet for **P04 — Printer / Machine / Print volume — wipe-tower** — 1 keys, Tier A plumbing, owner wipe-tower. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P04 — Printer / Machine / Print volume — wipe-tower):

`bed_exclude_area`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/256-wipe-tower-bed-exclude-area/` authored (`draft`), preflight **PASS** (2026-08-30; S0–S8 + AC-command + Doc-Impact checks, zero blockers, zero highs).

Grounding findings that shaped the packet:

- **The key is a true gap** — zero occurrences in `crates/`, `modules/`, `xtask/`, `resources/`; only the reference snapshot and gap docs mention it.
- **Canonical reads it four ways and disagrees with itself**: `Print.cpp::Print::validate` (via `layered_print_cleareance_valid`/`sequential_print_clearance_valid`) intersects **object volume convex hulls** with the polygon and fails **fatally** (`"<object> is too close to exclusion area…"`); `PrintConfig.cpp::get_bed_excluded_area` builds **one polygon from all points** (no rectangle pairing) while `Model.cpp` groups 4-point rectangles and `GCode.cpp::get_path_of_change_filament` demands exactly 4 points; `GCodeProcessor.cpp::apply_config` copies it for the viewer; `TimelapsePosPicker.cpp` subtracts it. **The wipe tower itself is never tested against it in canonical.**
- **Tier placement re-derivation**: ticket 04's row reads "wipe-tower (bed_shape)" for the *polygon-value* decision point; the port's only live bed-validation decision point is the wipe-tower's `run_finalization` 4-corner check (the object-hull validation of `Print::validate` has no orchestration-stage counterpart in this tree). The packet wires the **tower-rectangle check** (corner inside exclusion polygon → fatal, message names the key) and records the object-hull gap; on-edge counts as inside, matching the existing bed check's conservatism.
- **Canonical's default is a degenerate single point** `(0,0)` that excludes nothing → the manifest declares **no default** (absent-key fallback = same semantics; renders `—` in doc-15 like the sibling `printable_area`, and produces **no** deviation-table row since `default_num_of` compares numerics/booleans only), and degenerate values (empty/odd/<6) decay to "no exclusion", never an error.
- **Value format is already solved**: the manifest delivers by exact key name (`bind_module_config_view` → `ConfigView::from_declared`, untransformed copies; loader coercion is fully key-agnostic), and the module's `float_list_from_config` already expands Orca 3MF point-string arrays (`["0x0","20x0",…]`) via `slicer_ir::parse_orca_point_string` — ticket 100's `bed_shape`→`printable_area` lesson needed no new adaptation here; AC-3 pins it for this key in `bed_bounds_tdd.rs` next to the `printable_area` regression.
- **No host-side change at all** — `advanced = true` (Orca `comAdvanced`) uses the existing parsed manifest field; guests rebuilt via the freshness gate since the manifest feeds the fingerprint.

The tier table's "wipe-tower (bed_shape) + crates/slicer-gcode (printable_height)" dual placement was resolved as wipe-tower-only: the gcode-side half belongs to the `printable_height` family (P18/P19 queue), not to `bed_exclude_area` — canonical's gcode-adjacent consumers are gap-recorded, not wired.
