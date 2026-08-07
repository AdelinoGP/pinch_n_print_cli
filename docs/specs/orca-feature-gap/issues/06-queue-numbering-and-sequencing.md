# 06 — Settle packet numbering and how this queue interleaves with live work

Type: task
Status: open
Blocked by: 05
Map: ../map.md

## Question

What numbers do this map's packets take, and how do they coexist with spec work
already in flight?

Concrete hazard, not hypothetical: packets `200-205` currently exist as
**untracked** directories in the working tree, and `CLAUDE.md` explicitly warns
that "the next free packet number" is a *ledger fact* that rots while you work —
a previous effort duplicated a committed row exactly this way.

Resolve:

- Re-derive the highest allocated packet number **from disk at the moment of
  resolution**, and state the derivation command rather than the number.
- Does this queue reserve a contiguous block, or allocate one number at a time at
  authoring time? A reserved block is convenient and is precisely the kind of
  frozen ledger fact the repo has been burned by.
- Only one packet may be `status: active` at a time. Does this map's queue author
  all packets as `status: draft` and let `/swarm` flip them, or does authoring
  imply activation?
- Does this queue block on `200-205` merging first, or can authoring proceed in
  parallel with their implementation?

Output: the numbering procedure (a command plus a rule), not a number.
