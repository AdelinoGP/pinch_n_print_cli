# 06 — Settle packet numbering and how this queue interleaves with live work

Type: task
Status: resolved
Assignee: wayfinder session (ses_016603e7affem7g2wEEEmg12cw)
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

## Answer

### Disk state re-derived at resolution (2026-08-10) — re-derive at point of use, never trust this

- **Packets 200–205 are committed and tracked.** The map's "untracked, not yet
  merged" hazard note is stale — they landed via the spec-packets migration
  commit `a352c6b5` (along with the 194–199 series). The working tree carries
  uncommitted edits to `200/` and `201/` (`design.md`, `201/packet.spec.md`) —
  live implementation work in flight. So the "block on 200–205 merging" question
  is factually moot, and in general it's answered below.
- **Highest allocated numeric prefix from disk is 212** (next free = 213).
  `210a`/`210b` are a *split* of one allocated number (210; the merged 210
  re-split per its `requirements.md`), and 211 is `superseded` — neither
  advances the counter. Nothing in `xtask/` or CI consumes packet numbers; the
  number is purely the directory prefix + front-matter `packet:` field.

### Rule 1 — Allocate one number at a time, from disk, at authoring time. No reserved block.

A reserved block is precisely the frozen ledger fact CLAUDE.md and this ticket
warn about. The number is allocated by the **existence of the packet
directory**: derive, then write the directory; if the directory already exists
(a parallel session got there first), re-derive and take the next free.

Derivation command (the procedure — the number is the output of this, re-run
per authoring session):

```bash
ls -d docs/spec_packets/[0-9]*/ \
  | sed 's|docs/spec_packets/||; s|/||' \
  | grep -E '^[0-9]+' \
  | sed 's/^\([0-9]*\).*/\1/' \
  | sort -n | tail -1
```

Next free packet number = that output **+ 1**. Letter suffixes never appear in
a fresh allocation — they only exist when a *merged or existing* packet is
re-split (`210a`/`210b` precedent).

### Rule 2 — Numbering order follows the queue, naturally

Authoring sessions claim tickets in frontier order (one ticket per session),
so P01 gets the lowest free number, then P02, and the directory ordering
mirrors the cheapest-first queue from 05's packet list. No extra machinery; the
queue's identity lives in 05's list and the map, not in the numbers.

### Rule 3 — Author every packet as `status: draft`; activation is a `/swarm`-time act

Authoring never implies activation. The generator skill defaults to `draft` and
its activation gate (explicit request, no other active packet, no unresolved
blocker) matches README's "exactly one packet active at a time". Preflight runs
fine on drafts — precedent: all of 200–205 were preflighted while `draft`, and
the map's gate is `/spec-review <packet> --preflight` at authoring time. The
implementing session flips `status: active` when it starts a packet, per the
generator's step 10.

### Rule 4 — Never block on merge status; author in parallel with implementation

Authoring proceeds in parallel with any live packet work, including the
in-flight 200/201 edits. The only coupling between the queue and other efforts
is numbering, and Rule 1 decouples that. No blocking edges to packet dirs —
`06`'s own unblocking of the 91 authoring tickets is the only sequencing the
map needs.
