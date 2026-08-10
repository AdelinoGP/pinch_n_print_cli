# 07 — Document the Orca→Pinch alias map and retire the hand-maintained ❌ column

Type: grilling
Status: resolved
Assignee: wayfinder session (ses_016603e7affem7g2wEEEmg12cw)
Blocked by: — (03 resolved)
Map: ../map.md

## Question

Should the Orca→Pinch key alias map be written down and machine-checked, and
should the hand-maintained "In Codebase" column of `docs/ORCA_CONFIG_REFERENCE.md`
be replaced by generated output?

Ticket 01 established both halves of the problem with numbers: the column is
wrong on 66 of 574 FFF keys (11.5%, in both directions), and 62 declared Pinch
keys have no exact Orca counterpart because the project silently renamed the
vocabulary (`wall_count`/`wall_loops`, `first_layer_height`/
`initial_layer_print_height`, `wipe_tower_*`/`prime_tower_*`, and more). The
alias map exists only implicitly, scattered across module manifests.

This matters to the destination, not just to tidiness: **without a checked alias
map, every future packet in this queue re-litigates whether a key is a genuine
gap or a rename** — which is exactly the 57 false-gap keys 01 caught, and which
would otherwise have been specced as work.

Settle:

- Where does the alias map live — a new TOML beside `docs/config/host-keys.toml`,
  a column in `docs/15_config_keys_reference.md`, or module manifest metadata?
- Is it generated, hand-written, or hand-written-and-checked?
- Does `cargo xtask gen-config-docs --check` gain a gate that fails CI when the
  reference's presence flags disagree with the live registries? (It already
  parses that file for the `Default` column, so the plumbing exists.)
- Is retiring the column in scope for *this* map — it is tooling, not a feature
  packet — or is it a prerequisite deliverable that earns its place by
  unblocking the packet queue's correctness?

That last bullet is a scoping question: if the answer is "out of scope", close
this ticket into the map's **Out of scope** section rather than resolving it.

## Answer

Grilling session (2026-08-10). The alias map is not documented-and-maintained —
it is **eliminated**: Pinch 'n Print's vocabulary is standardised to Orca's
names, so the map shrinks to a historical migration record in 03's asset.

### Decisions

1. **Column retirement: OUT OF SCOPE.** The hand-maintained ❌ column of
   `docs/ORCA_CONFIG_REFERENCE.md` stays as-is; no generated-presence gate is
   added. The queue never sizes off it (map Notes + ticket 01's asset
   neutralise its 66-key error), so retirement is tooling hygiene for future
   efforts, not a prerequisite — it returns only if the destination is
   redrawn. Ruled into the map's Out of scope section.
2. **Standardise to Orca, not document.** The rename layer is the cost: 22
   mechanical renames (exact / word-order / unit-suffix) + 3 duplicate
   collapses + 1 rename 03's table missed (`ironing_spacing_mm` → `ironing_spacing`,
   top-surface-ironing — the map Notes' "four spellings, one concept" line) =
   **26 mechanical renames**, executed as a workstream of task tickets
   (99–107) on this map. After they land, the alias problem evaporates.
3. **Rename scope = mechanical only** (user ruling). The three shape-change
   rows from 03's 25-table are **not** renames:
   - `raft_layers` (1 Orca key → 3 Pinch keys): **not a missing feature** —
     Orca derives its base/interface split internally from one count
     (`OrcaSlicerDocumented/src/libslic3r/Slicing.cpp:194-196`); Pinch exposes
     all three counts, a strict superset. Renaming would merge keys and lose
     granularity. Recorded as a documented divergence, no gap, no rename.
   - `ironing_type` / `support_ironing`: **genuinely missing features, and
     they fell out of the queue.** Both modules declare the *same* shared
     `ironing_enabled` bool — Orca's enum modes (no ironing/top/topmost/solid)
     are unexpressible **and** top-surface vs support-interface ironing cannot
     be toggled independently. Neither key is in the 04/05 queue (03 classified
     them "already implemented (narrowed)" — the map's own Notes flagged them
     as hidden parity gaps). Reclassified as gap work: **P14 gains
     `ironing_type` (Tier B — mode-selection logic), P15 gains `support_ironing`
     (Tier A plumbing — independent bool)**. Assets and tickets amended; queue
     is now 405 keys, 356 in packets.
4. **Sequencing: renames first, blocking** (user ruling). The workstream
   (99–107) lands before the queue starts: ticket 08 (P01) is blocked by all
   nine; sessions take frontier tickets in order, so no queue ticket is
   claimable before the renames. Queue key lists are unaffected (they are
   Orca-named already — verified: every P01–P91 key list uses Orca spellings).
5. **Execution vehicle: task tickets on this map** (user ruling), one per
   owner group, each with: manifests + all call sites + tests + guest/wasm
   rebuilds + `cargo xtask gen-config-docs` regen (`--check` must pass — the
   rename lights up newly-matching deviation rows, which get triaged) +
   `03-asset-scoped-gap.md` row updates. The 34 Pinch-specific keys and the
   raft split stay untouched; `ironing_enabled` is *not* renamed by the
   workstream (its widening is P14/P15's packet work).

### New tickets cut from this resolution

99–107 (rename workstream, task type, created in a second pass — see the
issues dir). Queue amendments: 04's tier table +2 rows (A 118→119, B 223→224,
403→405), 05's packet list P14 3→4 keys / P15 1→2 keys (354→356; packet tiers
20 A→19 A / 65 B→66 B), tickets 21 and 22 re-keyed, 03's asset carries an
"Update after 07" note.

## Update after 03

The alias map's content now exists: 25 adjudicated renames and 34
Pinch-specific keys, in [`03-asset-scoped-gap.md`](./03-asset-scoped-gap.md).
This ticket is unblocked, and 03 widened it — the problem is not only
Orca↔Pinch drift but **internal** inconsistency:

- `modules/core-modules/fuzzy-skin` declares bare `thickness`,
  `point_distance`, `apply_to_all`. No declared key anywhere in the tree
  contains a dot, so there is no namespacing convention protecting a name that
  generic in a shared config space.
- The two ironing modules disagree with each other: `top-surface-ironing` uses
  `ironing_flow` + `ironing_spacing_mm`, `support-surface-ironing` uses
  `ironing_flow_rate` + `ironing_spacing`. Four spellings, one concept.
- `infill_density`, `infill_speed` and `infill_overlap` are declared *alongside*
  the Orca-named `sparse_infill_density`, `sparse_infill_speed` and
  `infill_wall_overlap`, all live. Two spellings of the same setting.

So the ruling this ticket needs is wider than first written: does the effort
also standardise Pinch's own key vocabulary, or only document the mapping to
Orca's? Standardising is a rename with blast radius across manifests, guest
rebuilds, and `docs/15_config_keys_reference.md`.
