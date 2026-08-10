# 02 — Set the canonical-parity evidence standard for gap packets

Type: grilling
Status: resolved
Assignee: wayfinder session (ses_0173d7c4cffeYnq9tFnN69J1Ci)
Blocked by: —
Map: ../map.md

## Question

What evidence must each packet in this queue carry that its implementation
matches canonical OrcaSlicer behaviour — and what is the fallback when that
evidence cannot be obtained?

`CLAUDE.md` makes canonical parity correctness "the highest priority" and
forbids weakening the canonical implementation to make tests pass. But
OrcaSlicer is **not vendored in this repo**, citations may not use line numbers,
and a queue of this size cannot afford a bespoke parity investigation per key.

Settle, with the human:

- Does a local OrcaSlicer checkout exist on this machine, and may packets assume
  the implementer has one? (If not, the standard must be checkout-free.)
- What tiers of evidence are acceptable: canonical function read + described
  behaviour, a recorded fixture, a golden G-code comparison, or documented
  behaviour only?
- For pure config-plumbing keys (a threshold that feeds an existing decision
  point), is "default matches upstream + key reaches the consumer" sufficient,
  or does every key need a behavioural test?
- When a key's canonical behaviour cannot be verified, is the correct move to
  file a `docs/DEVIATION_LOG.md` row, defer the key, or block the packet?

The answer becomes a boilerplate acceptance-criteria block reused by every
packet this map authors, so it must be concrete enough to paste.

This ticket is deliberately unblocked: it constrains packet *shape*, not packet
*contents*, so it does not need the inventory.

## Answer

All four decisions confirmed with the human in one grilling session.

### 1. The checkout is assumed available

`OrcaSlicerDocumented/` is a full in-tree OrcaSlicer checkout (own `.git`,
`src/`, `tests/fff_print/`, prebuilt `deps/`) — **readable, not runnable**: no
build directory exists and none is planned. Packets in this queue **may assume
the implementer has it** for verification. The binding rules stay:
`CLAUDE.md`'s cite-by-file+function (never line numbers), never assume a
*reviewer* has it, and the spec-packet-generator's `orca-delegation` snippet
governs how reads happen (sub-agent delegation, never loaded into the
implementer's own context).

### 2. Evidence standard: invariants, not goldens

There is no way to record golden outputs here — the checkout cannot be run — so
golden G-code comparison is **not part of the standard**. The standard is:

- **Canonical function read + described behaviour** — for each key, cite the
  canonical consumer (file + function) and describe its behaviour in
  `requirements.md`.
- **Invariant tests are the standard** — behaviour is pinned with
  invariant/property assertions (counts preserved, mappings hold, emitted
  values equal expected), not golden comparisons. This matches the repo's
  existing practice (e.g. packet 212's emitted-wall-count mapping, packet
  170's wall-preservation invariants).
- **Copying OrcaSlicer's own tests is acceptable evidence** — when
  `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its
  assertions into PnP's suite, with the standard porting header
  (`docs/ORCASLICER_ATTRIBUTION.md`) where logic is translated.

### 3. Plumbing keys: plumbing standard suffices

For a pure config-plumbing key (a threshold feeding an existing decision
point), evidence is: **(a) the default resolves to the canonical value, and
(b) a test proves the value reaches the consumer**. No behavioural test
required. A behavioural test for a threshold that feeds an existing decision
point adds nothing.

### 4. Unverifiable behaviour: human sign-off, then DEVIATION_LOG.md row

When a key's canonical behaviour cannot be verified (canonical code unreadable,
or behaviour depends on a subsystem PnP lacks), **the human is consulted
first** — the packet author must surface the unverifiable key and the reason
before anything is filed. Only with the human's sign-off does the packet file a
`docs/DEVIATION_LOG.md` row — the single source of truth, CI-checked by
`cargo xtask check-deviations` — and proceed with documented scope. Never
defer the key or block the packet on unverifiability alone, and never file a
row without the human having been asked.

### Boilerplate home

The boilerplate block lives as a **snippet in the spec-packet-generator skill**
(`.claude/skills/spec-packet-generator/references/snippets/parity-evidence.md`),
which the README names the single source of truth for packet boilerplate.
Every packet this map authors inherits it automatically; it composes with the
existing `orca-delegation` snippet.
