# Remaining whole-print collection drop inside G-code emission

Type: task
Label: wayfinder:task
Status: resolved (2026-10-29; Phase-1 screen — **measured-dead, no candidate,
no campaign, probe reverted.** The bracket is **93-94 % drop**; the parallel
free is a wash (**−0.32 / +0.20 s**); the deferred-overlap absolute ceiling is
the **1.37-1.60 s** post-emission tail = **0.82-0.97 %** of the cell, i.e. **at**
the 0.80 % floor before any overlap loss. See ## Answer)
Assignee: 2026-10-29 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named); the frontier re-derivation found no open
child, and the human selected this fog patch as the next route from five
measured options, with the **strict four-cell wall** gate
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Single-ownership layer collection through finalization and G-code emission](90-postpass-layer-collection-ownership.md)

## Question

Can the one surviving whole-print `Vec<LayerCollectionIR>` drop — now inside
`host:gcode_emit` at **3,616 ms classic / 2,961 ms arachne** on base supports-on
— be made cheaper without changing emitted bytes or skipping the free, and
does the resulting wall win pass the standing strict four-cell gate?

The t90 take deleted three of the four whole-print operations but left this
one. The bracket contains the emission walk as well, and t88 measured that walk
at ~0.19 s, so most of the bracket is very likely the drop — but t90 did not
split walk from drop in the candidate arm. **That split is this ticket's Phase
1, and no candidate is built before it is measured.**

## Measured premise

t90's candidate-arm attribution (instrumented, ordinary mode, attribution only
— never a wall claim):

| term | classic base | classic cand | arachne base | arachne cand |
| --- | ---: | ---: | ---: | ---: |
| `host:gcode_emit` bracket (walk + one drop) | 5,179 ms | **3,616 ms** | 5,642 ms | **2,961 ms** |
| emission walk (t88, pre-t90) | ~190 ms | ~190 ms | ~190 ms | ~190 ms |
| `PostPass::GCodeSerialize` | 703 ms | 685 ms | 807 ms | 704 ms |
| postpass stage-bracket sum | 12,738 ms | 5,772 ms | 13,438 ms | 5,088 ms |

The current source state, by symbol:

- `execute_postpass_with_owned_capture`
  (`crates/slicer-runtime/src/postpass.rs`) reconciles travel on the owned
  vector and calls `GCodeEmitter::emit_gcode_owned`.
- `DefaultGCodeEmitter::emit_gcode_owned`
  (`crates/slicer-gcode/src/emit.rs`) rotates in place, borrows the vector for
  `emit_from_reconciled_layers`, and lets `layer_irs` drop at the end of the
  function — i.e. inside the `host:gcode_emit` bracket.
- `emit_gcode` is the borrowing trait entry point; `DefaultGCodeEmitter`
  implements it as `emit_gcode_owned(layer_irs.to_vec())`, so the borrowing path
  pays a whole-print clone and stays off the production path.

The drop is the allocation free of the whole `LayerCollectionIR` tree; t74
measured the IR-only `support_entity_identities` side table at **23,751 rows
supports-on vs 0 supports-off**, which is why the free is seconds there and
milliseconds on supports-off. The collection is a `Vec<LayerCollectionIR>`; each
layer is independent after emission, so the frees are order-free.

## Phase 1 — split and price (mandatory)

Screening cell: **ordinary base classic supports-on**, the larger of the two
t90 brackets, with one arachne corroboration because the two drops differ
(3,616 vs 2,961 ms). Instrumented wall is attribution only; share is
wall-equivalent (accumulated ÷ the run's own cpu/wall) over the current cell
wall, the map's established method (t89 → t91 → t92 → t93).

Three numbers, two probe arms (the t93 pattern — one arm serial, one arm
parallel, so neither order biases the other):

1. **Serial split.** Time the rotation, `emit_from_reconciled_layers`, and the
   serial drop of the owned vector separately. The last one is the premise.
2. **Parallel free (counterfactual).** On the arm where the collection is not
   dropped serially, free it with `into_par_iter().for_each(drop)` (or
   `par_drain`) and time that. This is the whole-collection scaling number the
   candidate depends on; it changes no emitted byte.
3. **Overlap headroom.** From the same run, measure the postpass work that
   still executes after emission (`GCodePostProcess` modules, `GCodeSerialize`,
   text post-process). A drop deferred to a scoped task can hide only inside
   that work before it must join.

**Floor: 0.80 % of the cell wall, the map's established screening floor**
(t83/t84/t92/t93). At t90's candidate arm walls (175.44 s classic, 175.29 s
arachne) that is **~1.40 s**; re-derive it against the cell wall at take time.

Shapes the screen closes on measurement, and honest priors to state in the
answer:

- **Layer-by-layer consumption** — make the walk own the vector and drop each
  layer as it is emitted. Emission is serial and totals ~0.19 s, so this
  interleaves frees with that work rather than overlapping them; prior is a
  near-zero wall win, and only the measured split may keep it alive.
- **Parallel free** — above, if it clears the floor.
- **Deferred free overlapping the rest of the postpass** — only if the serial
  and parallel numbers show headroom the overlap could beat. Joined before
  `execute_postpass_with_owned_capture` returns; the collection must be freed,
  not leaked, and must not move outside the measured slice process.
- **Skipping or leaking the drop, or exiting before destructors run** —
  **out of bounds.** The destination is uninstrumented whole-process wall; a
  freed-after-measurement or leaked collection is not a performance change.

## Candidate shape (Phase 2, only if a screen shape clears the floor)

Whichever screen shape survives: the production path stays
`execute_postpass_with_owned_capture` → one owned collection → emitted bytes
identical; the free is complete before the function returns; the borrowing
`emit_gcode` path and every other `GCodeEmitter` implementation keep their
current contract. A `rayon` dependency in `slicer-gcode`, if needed, is
host-only and does not touch guest fingerprints.

## Pre-declared gate

**Strict wall in all four pre-declared cells** — ordinary/accelerated ×
classic/arachne, all base supports-on — paired median wall **below baseline in
every cell**, process CPU **≤ baseline + 3.0 %** (the map's work-preserving
ceiling, t78/t90 precedent), 6 alternating pairs per cell, 12 threads, zero
starvation exclusions, every retained pair byte-identical.

The human settled this clause before the take (t93's lesson: a strict four-cell
wall clause can fail on a real work removal, and the ruling is made before the
campaign, never after). This is a **strict wall** take, not a
CPU-corroborated-wall-neutral one.

## Proof gate

- 16-cell output-identity matrix against the current post-t81 references, both
  modes; ordinary ≡ accelerated bytes.
- `host:gcode_emit`'s `GCodeIR` and serialized bytes unchanged; the existing
  `gcode_emit_owned_parity_tdd` suite stays green.
- Tool rotation and `;TYPE:`/entity order unchanged on the multi-tool fixture
  (the negative control that pins the no-op rotation on the matched job).
- A unit test pinning that the collection is freed before
  `execute_postpass_with_owned_capture` returns (no leak), and that a small /
  empty collection still takes the same path.
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The Phase 1 split and the per-shape verdicts.
2. If a shape clears the floor: the candidate, proof matrix and the four-cell
   paired campaign with a measured recommendation to the human. No auto-commit.
3. If no shape clears: a measured-dead resolution with the probe reverted and
   the fog updated — no campaign.

## Answer

**Resolved 2026-10-29 as a Phase-1 screen — measured-dead, no candidate, no
campaign, no production edit.** Probe reverted (stashed; diff preserved as
[`probe.patch`](../evidence/t94-remaining-drop/probe.patch)). Both arms produce
the post-t81 references byte-for-byte (`22200671…` classic, `0b29e1bf…`
arachne); both guest freshness gates exit 0 at close (ordinary before and after
the take, accelerated at close).

### The split the ticket demanded

One probe slice per arm per cell, ordinary base supports-on, `SystemTime`
marks, attribution only (never a wall claim):

| cell | arm | rotate | walk | **drop** | bracket | tail after emission |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| classic-on | serial | 6 µs | 252.8 ms | **3,558.1 ms** | 3,811 ms | **1,603 ms** |
| classic-on | parallel free | 7 µs | 176.4 ms | **3,237.4 ms** | 3,415 ms | 1,370 ms |
| arachne-on | serial | 7 µs | 195.3 ms | **3,242.4 ms** | 3,438 ms | **1,367 ms** |
| arachne-on | parallel free | 7 µs | 192.7 ms | **3,442.1 ms** | 3,636 ms | 1,395 ms |

The bracket is **93-94 % drop**. Rotation is the no-op t90 recorded; the walk
is 176-253 ms; the free is 3.24-3.56 s. The only post-emission work a deferred
free could hide inside is the **1.37-1.60 s** tail (`machine-gcode-emit`
0.53-0.60 s + `GCodeSerialize` 0.72-0.92 s + dispatch gaps).

### Shape verdicts

- **Layer-by-layer consumption — measured-dead (structural zero).** The walk is
  serial and totals 176-253 ms; freeing each layer as it is emitted interleaves
  the same serial frees with that work rather than overlapping them, and no
  other thread runs during the walk.
- **Parallel free (`into_par_iter().for_each(drop)`) — measured-dead.** Classic
  **−0.321 s (+0.19 %** of the cell), arachne **+0.200 s (−0.12 %)**. A wash in
  both directions, against the **1.32-1.34 s** floor: the free does not scale
  (12 threads buy 9 % in one cell and lose 6 % in the other), so Windows'
  process-heap serialisation, not thread count, sets the cost.
- **Deferred free overlapping the post-emission tail — at the floor, no
  margin.** Absolute ceiling = the tail: **1.603 s classic (0.97 %)** and
  **1.367 s arachne (0.82 %)** of the cell, against a **1.318 / 1.335 s** floor
  (**1.22x / 1.02x**). That is the *ceiling* before any overlap loss, wake-up
  or allocator contention with the concurrent 42 MB serialisation — it cannot
  win more than the tail, and cannot win the whole tail. Under the pre-declared
  **strict four-cell wall** gate this does not justify a campaign; building it
  would be a coin flip at 0.8-1.0 % on cells whose repeat spread is several
  percent.

### What the screen leaves

The free is **not a drop-strategy problem, it is an allocation-volume
problem**: ~5.3 GB peak working set is measured on this cell class (t79), and
3.2-3.6 s of serial heap-walk free is its price. A shape that could move it is
**fewer/larger allocations — an arena or bulk-free layer collection** — which
is a structural IR/allocator change, recorded in the map's fog. Parallel free,
deferred free and layer-by-layer consumption are all measured at or below the
floor.

### Disclosures

Machine load was **8 %** before the first run (not a quiet campaign); Phase 1 is
attribution and the internal split is stable (drop spread ±5 % over four runs),
but no wall claim is made from the instrumented runs. The arachne serial and
parallel **walls** differ by 4.8 % in the *opposite* direction of the 0.20 s
drop difference, which is why single-run walls are not used. The arachne drop
(3.24 s) is larger than t90's bracket-minus-walk estimate (2.77 s): t90 never
split its bracket, and today's bracket is 3.438 s. No paired campaign was
started.

Evidence:
[`evidence/t94-remaining-drop/`](../evidence/t94-remaining-drop/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, **8/8** negative controls
rejected; `extract.py` re-derives `summary.json` from the raw capture tree).

