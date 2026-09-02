# 113 — Add range validation to `FeedrateConfig`

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q6(b)**: *"add range
validation"* to `FeedrateConfig`'s fields — *"Canonical declares `min = 10`
here; the struct has no bounds machinery at all."* The ruling explicitly flags
its own gap: **canonical min/max per field were not derived in that session.**
Deriving them is the bulk of this ticket.

Verified in-tree (2026-09-02), `crates/slicer-ir/src/feedrate.rs`:

- `FeedrateConfig` — every field is a bare `pub <name>_speed: f32`. No bounds,
  no validation fn.
- The file's only functions are `default`, `read_speed`, `as_number`, and
  `from_raw_config`. `read_speed` coerces `Float` / `Int` / non-percent
  `FloatOrPercent` and returns `None` otherwise — it does not reject zero,
  negative, or absurd values.
- The registration table is **`SPEED_KEYS`** (26 entries), *not* `FEEDRATE_KEYS`
  — the name the grilling row uses does not exist in this tree. Ticket 22's
  preflight caught the same fiction inside packet 267; treat other symbol names
  in `key-correction-inventory.md` as unverified until greped.

Decide and execute:

1. **Derive canonical min/max per feedrate key** from OrcaSlicer's
   `PrintConfig.cpp` declarations (cite by file + function, never line numbers;
   the checkout is the sibling `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`).
   26 keys is the working set; expect several to have a min and no max.
2. **Where validation runs.** `FeedrateConfig` is host-side and typed, so it does
   not go through `ConfigBoundsIndex` (which serves manifest-declared module
   keys). Decide whether feedrate bounds join that machinery, get their own
   check, or ride `from_raw_config`'s parse path — and what the error type is.
3. **What an out-of-range value does** — reject the slice, or clamp with a warn?
   Rejecting is consistent with the manifest-bounds behaviour; clamping is
   friendlier to imported profiles. Pick one and say why.
4. **Any key whose canonical bound the port cannot honour** becomes a recorded
   divergence with rationale, not a silently dropped bound.

Relationships (re-derive status at point of use, do not trust this line):

- **Ticket 108** (`wipe_tower_speed` → `wipe_tower_max_purge_speed`) is where this
  surfaced — canonical declares `min = 10` on that key and the port's typed arm
  cannot express it. Q6(a) ruled the rename; Q6(b) is this ticket. Whether 108
  waits on this or records the bound as deferred is 108's call.
- **Ticket 109** decides whether the renamed `support_ironing_speed` joins
  `SPEED_KEYS`; if it does, it inherits whatever this ticket builds.

Not a queue key; changes no queue count.

## Answer
