# 13 — Author packet P06 — Others / Skirt — skirt-brim

Type: task
Status: resolved
Assignee: wayfinder session (ses_faac2acf8ffeQ94yALejPHW5W7) — claimed 2026-08-30
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P06 — Others / Skirt — skirt-brim** — 5 keys, Tier A plumbing, owner skirt-brim. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P06 — Others / Skirt — skirt-brim):

`draft_shield`, `min_skirt_length`, `single_loop_draft_shield`, `skirt_start_angle`, `skirt_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/258-skirt-type-and-draft-shield-keys/` authored (`draft`), preflight **PASS** (2026-08-30; S0–S8 + AC-command + Doc-Impact checks, zero blockers, zero highs; report persisted at `preflight-report.md` in the packet dir).

Scope: **5 keys** as listed (no scope-changing rulings; all five are true zero-occurrence gaps — authoring-time whole-tree grep found hits only in docs, no near-variant spellings). Three keys wired (decision points re-derived in code, not trusted from the tier table), two declared-with-gap:

- **Wired (3):** `draft_shield` (`Print::has_infinite_skirt` → the port's skirt layer span extends to the full layer set when `enabled`; `skirt_height` unchanged in the disabled arm), `single_loop_draft_shield` (`GCode::generate_skirt`'s `!first_layer` single-wall condition → exactly the innermost rect loop on `global_layer_index > 0`), and `skirt_start_angle` (`Skirt::find_start_point` mirrored as corner-nearest ring rotation of the first-layer first-emitted loop; the in-tree observability chain — gcode emitter never rotates closed loops, path-optimization permutes whole entities only — was verified at authoring, honoring ticket 100's value-reachability lesson). Default −135° selects the loop's existing start corner, so **default output is byte-identical** (AC-4 identity clause).
- **Declared-with-gap (2):** `skirt_type` (canonical consumers in `Print::_make_skirt`/`GCode::generate_object_skirt_group` need per-object skirt grouping — none exists in-tree; default `combined` matches today) and `min_skirt_length` (`Print::_make_skirt`'s extruded-length loop expansion needs a per-filament e_per_mm model — Tier-D fog territory; default 0 = disabled matches today). AC-N1 pins both as non-perturbing.
- **Recorded divergences (not fixed, packet 257's class):** the port's skirt loops are emitted innermost-first with no reversal (canonical exports outermost-first), so canonical's rotated-start condition — first-*emitted* = spatially **outermost** wall there, **innermost** here — lands on a different wall despite the same emission-order decision rule; corner-nearest start selection instead of canonical's mid-edge seating (rect loops have only corner vertices).
- Manifest exactness: enum tables in the in-tree `type = "enum"` + `values` form (`seam-planner-default.toml` precedent); defaults all canonical-identical (`skirt_start_angle` −135 [−180,180]; `min_skirt_length` 0 min-0-no-max; both enums in canonical value order) — **no deviation rows, no human sign-off consumed** (none of the 5 keys appears in `docs/DEVIATION_LOG.md`).
- CONFIG_BLOCK honesty: `ORCA_CONFIG_PADDING` (70 entries verified) gains **no twins** for the five keys (packet 254/255/257 precedent — non-percent manifest defaults do not thread into raw config); AC-6 pins single-emission for explicit values and honest absence at defaults.
- Preflight correction (recorded in `preflight-report.md`): the Doc-Impact grep initially targeted a nonexistent `## skirt-brim` per-module heading in the generated doc; a disk probe showed modules are table rows with an owner column, so verification is key-presence (matching 257's corrected form). Packet number 258 derived from disk per ticket 06's rule; queue packet carries `task_ids: []`; implementation order note: packet 257 (same owner) implements first.
