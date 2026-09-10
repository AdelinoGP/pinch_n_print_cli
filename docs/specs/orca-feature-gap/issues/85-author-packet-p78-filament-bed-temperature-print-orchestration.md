# 85 — Author packet P78 — Filament / Bed temperature — print-orchestration

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260910_P78) — claimed 2026-09-10, resolved 2026-09-10
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P78 — Filament / Bed temperature — print-orchestration** — 1 keys, Tier B new logic, owner print-orchestration. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P78 — Filament / Bed temperature — print-orchestration):

`support_multi_bed_types`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, re-filed, no packet, no code change** (the ticket-28/39/45 shape). `support_multi_bed_types` passes rule 3 — it is live in canonical's slicing pipeline (`PrintConfig.cpp` `coBool` default `false`; `Print::validate`'s filament-vs-plate compatibility arm, `Print.cpp`, gated `is_BBL_printer() || support_multi_bed_types` — verified against the oracle at claim time) — and is zero-occurrence in `crates/` + `modules/` + `xtask/` + `resources/` (verified by tree-wide `rg`; the one `serialize.rs` hit is the `s_IsBBLPrinter` CONFIG_BLOCK comment, not the key). But the key is a *gate over an absent selection domain*, so Tier B does not hold as ticketed:

- Canonical's arm reads `get_bed_temp_key(m_config.curr_bed_type)` — a whole-vector plate lookup (`ConfigOptionInts` per-filament plate vectors: `cool_plate_temp` / `eng_plate_temp` / `hot_plate_temp` / `textured_plate_temp` / `textured_cool_plate_temp` / `supertack_plate_temp` + `_initial_layer` twins — Tier D, zero tree occurrences — against six Tier D plate-temperature vector pairs) and rejects when the selected plate's temperature for a filament is `0`. The `BBL` side of the `||` is a vendor-identity predicate the port also lacks (`s_IsBBLPrinter` is only echoed back into the CONFIG_BLOCK as a synthesized `printer_model` string; ticket-03 class).
- The key's other consumers are all GUI surface (`Plater.cpp` bed-panel show/enable, `Tab.cpp` per-plate temp-line toggling) — no slicing geometry.
- Ticket 45's finding stands: the port's only bed-temperature value is the PnP-specific scalar `bed_temperature_initial_layer_single` (template-driven in `machine-gcode-emit`) with no plate axis and no per-filament vectors. A packet declaring `support_multi_bed_types` alone would wire a gate whose *subject* (`curr_bed_type` → plate vector → zero-check) does not exist — 100% declaration-only, prohibited by rule 1. The tier table's `print/orchestration` owner is additionally the ticket-27 hazard shape (this tree's validation seam is `run_slice` + `slicer-scheduler` validation, not a `Print::validate` analog — but the arm belongs with the plate-selection feature wherever it lands, not as a standalone validation packet).

**Re-filed as P78-carrying content on [138](138-author-packet-p38-bed-temperature-selection-refiled.md) (re-titled at claim time to carry it), which stays blocked on 125** — the 39→136 / 45→138 pattern; the subject (`curr_bed_type` plate selection over the Tier D vectors) and its gate (`support_multi_bed_types`) must author together or the gate has nothing to gate. 138's Question gains the key at claim time; 04/05 rows annotate the re-file. No packet number taken, no deviation rows, no `ORCA_CONFIG_PADDING` edit. Docs-only change: no build, clippy, or test gates affected.
