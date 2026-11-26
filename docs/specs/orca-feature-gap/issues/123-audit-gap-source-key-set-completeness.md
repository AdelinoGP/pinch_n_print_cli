# 123 — Audit the gap source's key set for completeness against upstream

Type: research
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**Is `docs/ORCA_CONFIG_REFERENCE.md`'s row set complete against upstream
OrcaSlicer, and if not, what is missing?**

Filed by ticket 30. Canonical's flush path reads two keys this map has never
heard of — `flush_multiplier_fast` and `prime_volume_mode`, both consumed by
`ToolOrdering::prepare_flush_matrices` on the `prime_volume_mode == pvmFast`
branch — and **neither appears anywhere in `docs/ORCA_CONFIG_REFERENCE.md`**.
They are therefore absent from ticket 03's scoped gap, from
[04's tier table](./04-asset-tier-assignment.md), and from every packet in
[05](./05-asset-packet-list.md).

This is a different defect from the one ticket 01 measured. Ticket 01 audited the
reference's hand-maintained ✅/❌ **"In Codebase" column** against this tree and
found it wrong on 66 of 574 FFF keys. It did not ask whether the **row set
itself** is complete against upstream — a key that is missing from the snapshot
has no column to be wrong.

**Why it matters to the destination.** The map's destination is that every FFF
OrcaSlicer feature this port is missing is closed. The queue is derived from the
snapshot. If the snapshot under-reports upstream, closing every remaining queue
ticket does not reach the destination, and nothing in the current map would
reveal that.

### What to do

Re-derive the key set from an OrcaSlicer checkout — `PrintConfigDef::init_fff_params`
(`PrintConfig.cpp`) is the enumeration, one `this->add("<key>", co<Type>)` per key —
and diff it against the snapshot's rows. Note that a local checkout may itself be
a different revision from whatever the snapshot was taken against; say which
revision the diff was taken at, and treat that revision as a ledger fact rather
than freezing it into the tier table.

For each key the diff surfaces, the answer should record:

- the key, its canonical type and default;
- whether it has a read site inside `libslic3r/`'s slicing pipeline — **authoring
  rule 3** applies unchanged, and a key that fails it is out of scope, never
  declared "for parity";
- which existing owner/packet it would belong to, if it survives rule 3.

Both confirmed instances so far are canonical's fast-purge branch, whose own
in-tree comment calls `flush_multiplier_fast` inert for the shipping fleet and
keeps it out of the g-code config block. The missing rows may skew toward keys
that are dead anyway — **or may not**; do not assume the sample generalises.

### Scoping

The *size* of this is unknown until the diff is run, and that is the point of
running it. Resolve this ticket with the diff and the per-key triage. If the
surviving set is large enough to need its own packets or its own queue segment,
say so and file those as separate tickets rather than absorbing them here — and
if the count moves, remember the scoped queue target (currently 407) is a
**ledger fact**: re-derive it, do not quote it.


### Four more confirmed absentees (ticket 33, 2026-09-03)

Ticket 33 hit the same gap from the other direction: it could not close
`calib_flowrate_topinfill_special_order` without a key that is not in the queue.
All four below are defined in canonical `PrintConfig.cpp` and read in the slicing
pipeline (so they pass authoring rule 3), and none appears anywhere in
`docs/ORCA_CONFIG_REFERENCE.md`:

| key | canonical read site | note |
| --- | --- | --- |
| `top_surface_fill_order` | `Fill/Fill.cpp`, gated on pattern ∈ {`ipConcentric`, `ipArchimedeanChords`, `ipOctagramSpiral`}; invalidation set in `PrintObject.cpp` | `coEnum` `default`/`outward`/`inward`. **Now implemented by packet 275** |
| `bottom_surface_fill_order` | same | same. **Now implemented by packet 275** |
| `separated_infills` | 8 read sites outside `PrintConfig`/`Preset` | untriaged |
| `center_of_surface_pattern` | 12 read sites outside `PrintConfig`/`Preset` | untriaged |

This raises the confirmed-absent count from ticket 30's two to six, and — unlike
the fast-purge pair — these are **not** dead-in-canonical, which weakens the
"the missing rows may skew toward keys that are dead anyway" hypothesis above.
Two of the six were only found because a ticket tripped over them; nobody diffed
for them. Treat that as evidence the row-set diff is worth running, not as the
result of one.

**Do not re-derive the two fill-order keys as new queue work** — packet 275 owns
them. The remaining two are untriaged and belong to this ticket's diff.

## Answer
