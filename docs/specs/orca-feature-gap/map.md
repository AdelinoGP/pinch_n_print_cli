# Map: Close the OrcaSlicer FFF feature gap

Label: `wayfinder:map`

## Destination

A queue of **fully authored, preflighted spec packets** under `docs/spec_packets/` that
together **implement** every FFF (non-SLA) OrcaSlicer *feature* Pinch 'n Print is
still missing — each packet complete with `packet.spec.md`, `requirements.md`,
`design.md`, `implementation-plan.md`, and passing `/spec-review --preflight`.
Packets are ordered **cheapest-first** (smallest diff before new geometry).

The config keys are the *inventory* of the gap, not the deliverable. A key is
"covered" only when the behaviour OrcaSlicer attaches to it exists in this
tree and the key drives it. A packet that declares keys in a manifest, pads
them into the CONFIG_BLOCK, and records the behaviour as a "gap" covers
nothing — see **Authoring rules** in Notes.

The map is done when every in-scope feature is either implemented-by-packet
or has been consciously ruled out of scope. Implementation (`/swarm`) runs
off-map, after.

## Notes

- **Domain:** 3D-printing slicer config/feature parity with OrcaSlicer. The gap
  source is `docs/ORCA_CONFIG_REFERENCE.md` — an upstream snapshot whose
  ✅/❌ "In Codebase" column is **hand-maintained and measurably wrong** (only
  the `Default` column is machine-read, by `xtask/src/gen_config_docs.rs`).
  Ticket 01 measured it: wrong on 66 of 574 FFF keys. **Never size anything off
  that column** — use ticket 01's asset, or re-derive.
- **Pinch 'n Print renamed Orca's keys — now being standardised away.**
  Ticket 07's ruling: **standardise to Orca's names**, not document. The rename
  workstream is tickets **99–107** (24 keys after ticket 105's re-adjudication
  and ticket 107's closure: 21 exact rows + 2 duplicate collapses —
  `infill_density` → `sparse_infill_density` and `infill_speed` →
  `sparse_infill_speed`; `infill_overlap` was re-adjudicated a PnP-specific
  decision point, not a duplicate, and stays — + `ironing_spacing_mm` —
  `resolution`
  was re-judged a **gap**, not a rename: canonical applies it as a
  generation-time *global* simplify, the host's `gcode_resolution` is emit-time
  and per-role, so the two are different decision points; the key now rides
  queue packet P51 and `gcode_resolution` stays PnP-specific); ticket 108
  (filed by ticket 10's authoring) adjudicates a possible 25th —
  `wipe_tower_speed` → `wipe_tower_max_purge_speed` — resolved by ticket 108
  with the canonical purge-speed cap. It **gates the queue by owner** — each
  packet ticket is blocked by the rename tickets that touch *its* owner (wired
  in ticket 100 after the original wiring was found to gate nothing: 09–98 were
  blocked only by the already-resolved 06). 20 packets touch no renamed owner
  and carry no gate. `03-asset-scoped-gap.md` remains the historical
  adjudication; each workstream ticket updates its own rows there. The 34
  Pinch-specific keys and the `raft_layers` 1→3 split (a strict superset — not
  a gap) stay untouched. The two narrowed ironing enums were reclassified as
  **gaps**: P14 +`ironing_type`, P15 +`support_ironing` (see Decisions so far).
- **A rename is not automatically mechanical — check the *value* format too.**
  Ticket 100 found `bed_shape` → `printable_area` changes how the value is
  spelled, not just the key: Orca writes the bed as point strings
  (`["0x0","250x0",…]`), this port as an interleaved float list. Adopting the
  name alone broke 3MF ingestion. Before closing a rename, resolve a real Orca
  3MF through it, not just the unit tests.
- **The deviation gate compares booleans as of ticket 100.** It did not before
  (`num_of` returned `None` for `toml::Value::Boolean`), so any pre-100 claim
  that a boolean default "matches Orca" was never actually checked. Re-verify
  rather than trust those.
- **A manifest `default =` is DEAD for any plain-typed key that also exists as
  a `ResolvedConfig` field — checking the manifest proves nothing.**
  `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`)
  seeds from `ResolvedConfig::default()`, and its schema-default back-fill loop
  iterates `ConfigBoundsIndex::schema_defaults`, which holds **`percent` /
  `float_or_percent` fields only**. Module config is then built from
  `ResolvedConfig::to_config_map()`
  (`crates/slicer-wasm-host/src/marshal/in_.rs`, `marshal/native.rs`), so for a
  plain float/int/bool key the *macro* default reaches the module and the
  manifest's value is never consulted. Confirmed instance:
  `sparse_infill_speed` — three manifests declare `100.0` (aligned to canonical
  by ticket 107), every module actually receives `ResolvedConfig`'s `50.0`.
  **Consequence: every "default aligned to canonical" claim in tickets 99–107
  and packets 253–266 that was verified by reading a manifest is unverified for
  this class of key.** Not enumerated. Before asserting a default matches, check
  whether the key has a `ResolvedConfig` field and compare *that*
  (2026-09-01 grilling, Q11).
- **The scoped target is 407 queue keys** (03's 415 minus 04's 11 rulings plus
  07's 2 reclassified ironing keys plus 99's 2 fan-scale keys — minus ticket
  12's dead-in-canonical `brim_ears` ruling: **407**; the 406→407 step is
  ticket 105's re-adjudication of `resolution` out of the rename pool into the
  gap set; per-key tier table in
  [`04-asset-tier-assignment.md`](issues/04-asset-tier-assignment.md), packet
  list in [`05-asset-packet-list.md`](issues/05-asset-packet-list.md). Size
  packets off those, never off the reference's ❌ column.
- **Execution override:** this map deliberately carries execution — packet
  *authoring* happens inside the map, not after it. Implementation does not.
- **Authoring rules — binding on every packet ticket (08–98), supersede
  anything earlier in this map or in ticket 02/04/05 that reads otherwise.**
  Adopted after review of packets 253–265 found most of them declaring keys
  as manifest stubs ("declared-with-gap") to satisfy a parity count: 263 has
  zero module reads for 10 keys, 261 zero for 2, 254 one live key of 13, 255
  one of 12, 257 one of 5. That is not what this map is for.
  1. **No declaration-only keys.** Every key in a packet must, by the end of
     the packet, drive a behaviour-changing decision point that the packet
     either builds or proves already live. The dispositions
     "declared-with-gap", "decision-point gap recorded", "declare + record
     the consumer", and any AC whose only evidence is default-path identity
     are **prohibited**. If the decision point does not exist, the packet
     builds it (and is re-tiered B/C in its ticket) — or the key is **left out
     of the packet** and returned to the queue as *unimplemented*, with the
     missing feature named in the tier table. A packet never counts a key it
     did not make work.
  2. **CONFIG_BLOCK padding is not parity.** `ORCA_CONFIG_PADDING`
     (`crates/slicer-gcode/src/serialize.rs`) is **not evidence**; adding or
     "correcting" a padding twin is never a packet deliverable, an AC, or
     evidence that a key is covered. Packets emit a key into the CONFIG_BLOCK
     only as a side effect of the key being live.
     **The table is load-bearing, not cosmetic — do not delete it.**
     Canonical `ConfigBase::load_from_gcode_file` (`Config.cpp`) *throws*
     `Slic3r::RuntimeError` when a CONFIG_BLOCK yields fewer than 80
     key-value pairs, on the same delimited path this port emits; the
     `emitted.len() >= 96` break in `serialize.rs` is a deliberate margin
     over that floor. An earlier wording here called the table "cosmetic",
     which is false: padding fires only for keys the host config map does
     *not* emit — which is every module-manifest-owned key — so for those the
     hardcoded padding value is the only value a viewer or re-slicer sees.
     Ruling (2026-09-01 grilling, Q5): the table is **derived mechanically
     from the resolved config** rather than hardcoded, so a twin cannot drift
     from what the slicer did, while still clearing the 80-pair floor.
  3. **Dead-in-canonical keys are out of scope, checked per key at
     authoring.** A key must have a read site inside OrcaSlicer's *slicing
     pipeline* (`libslic3r/`, not `ConfigManipulation.cpp`, GUI tooltips,
     preset plumbing, or an `IGNORE`/legacy-alias set). Keys that fail this
     go to **Out of scope** with the ticket-04/12 `brim_ears` precedent and
     shrink the queue count; they are never declared "for parity".
     **The precedent is narrower than it reads.** Ticket 12 ruled the
     `brim_ears` *bool* dead, and it still is. The ears *feature* is live —
     `Brim.cpp::make_brim_ears_auto`, reached through `brim_type ==
     btBrimEars` rather than the retired bool — and `brim_ears_max_angle` /
     `brim_ears_detection_length` are live with it. Rule out the **key** you
     verified dead, never the feature it used to reach: check what else
     reaches the behaviour before shrinking the count (2026-09-01 grilling,
     Q14(c)).
  4. **Implement the PnP way, not the Orca way.** The packet's design must
     satisfy the project goals in `docs/00_project_overview.md` (modular
     pipeline, community extensibility, config robustness) and use the
     mechanics this tree already has — in particular:
     - **Alternative behaviours become modules holding claims**, selected by
       the existing claim-holder keys and region overrides
       (`docs/03_wit_and_manifest.md` § Known claim IDs;
       `docs/01_system_architecture.md` § Claim System). An Orca enum whose
       values are different algorithms (`sparse_infill_pattern`,
       `top_surface_pattern`, `support_interface_pattern`,
       `fuzzy_skin_noise_type`, …) is *not* an enum to declare on one module
       and mark with-gap — it is a set of `claim:*` holders, one per shipped
       value, resolved through `*_fill_holder` / `module_overrides`. A packet
       may ship a subset of values; the unshipped values are unimplemented
       (rule 1), not declared.
       **Trigger test (2026-09-01 grilling, Q8):** this rule fires on
       *cross-module* algorithm selection — where the alternatives are
       separate implementations that must live in separate modules and be
       resolved through the claim seam. It does **not** fire on a module
       branching internally over a mode it implements itself. `seam_position`,
       `support_style`, `wall_sequence`, `retract_mode` and
       `wave_overhang_pattern` are the latter and stay as they are; they are
       not refactor targets, and rules 1–6 bind packet tickets, not
       already-merged tree code.
       **Holder-only, always (Q3):** the Orca enum is *never* declared as an
       input key — not even as a host-side alias mapping its string onto a
       holder name. Selection is by `*_fill_holder` / `module_overrides`
       alone. Consequences accepted with the ruling: an Orca 3MF setting
       `sparse_infill_pattern = gyroid` is silently dropped (the port has no
       opinion on keys it does not implement, and a "recognised but
       unimplemented" reject list is itself a form of declaration that
       drifts); and a holder naming a module no manifest matches must **fail
       validation** rather than yield a silently hollow part, which today it
       does — `resolve_held_claims`
       (`crates/slicer-scheduler/src/validation.rs`) returns empty for every
       module and no `SchedulerError` variant covers it.
     - New decision points go where the architecture puts them (prepass IR,
       `SliceRegionView` metadata, `PostPass` claims, manifests + SDK) — not
       as host-side special cases or hardcoded module constants.
     - Where the port's architecture allows a *better* answer than canonical
       (a cleaner seam, a per-region override Orca lacks, a bug Orca carries),
       the packet takes it and records it as a **recorded divergence with
       rationale**, not as a gap. Improving on OrcaSlicer is in scope;
       reproducing its coupling is not.
  5. **Ticket 02's plumbing-key exemption is narrowed.** "Default matches +
     value reaches the consumer" is sufficient evidence *only* when the
     consumer is a live, behaviour-changing decision point. It is never a
     licence to add a consumer that does nothing.
  6. **Preflight adds two gates for this map:** (a) the packet's disposition
     table lists zero declaration-only keys; (b) every key has at least one AC
     asserting a behaviour change at a non-default value, verified by test.
     `/spec-review --preflight` PASS on a packet violating (a) or (b) is not a
     PASS for this map.
  7. **Retroactive.** Packets 253–266 were authored before these rules and
     must be re-authored to them **before any of them merges or activates**;
     per-packet findings are marked on their Decisions entries below with
     ⚠. Open packet tickets keep their key lists but their "Work: declare in
     the owner's manifest + wire" line is read under rules 1–6.
- **Skills every session should consult:** `/grilling` and `/domain-modeling`
  for decision tickets; `/spec-packet-generator` for authoring; `/spec-review
  <packet> --preflight` as the authoring gate.
- **Repo rules that bind this effort** (`CLAUDE.md`): in-tree citations by
  symbol name + crate-qualified path, never bare line numbers; OrcaSlicer
  citations by file + function, never line numbers; ledger facts (next free
  packet number, next `DEV-###`, line counts) must be **re-derived at point of
  use**, never frozen into a ticket or packet.
- **Live ledger note:** the map's original 200–205 "untracked" hazard resolved
  itself — those packets are committed (spec-packets migration `a352c6b5`);
  206–212 also exist, with live uncommitted edits on 200/201. Numbering is fully
  decoupled from all of it by ticket 06's Rule 1: one number at a time, derived
  from disk at authoring time.

## Decisions so far

<!-- one line per resolved ticket: gist + link -->

- [01 — Build a mechanically verified FFF gap inventory](issues/01-verified-gap-inventory.md)
  — the real FFF gap is **419–481 keys, not ~640**; the hand-maintained ✅/❌
  column is wrong on 66 of 574 keys, and Pinch 'n Print uses an undocumented
  renamed key vocabulary (62 declared keys have no Orca counterpart), which is
  what makes the count a band rather than a number.
- [03 — Triage which verified-missing keys are not applicable at all](issues/03-nonapplicable-keys-triage.md)
  — **the queue must cover 414 keys** (405 after ticket 04's 11 additional
  out-of-scope rulings and ticket 07's 2 reclassified ironing keys). 42 ruled out of scope (print-host/preset,
  non-physical filament metadata, Bambu-proprietary, pellet, plater/GUI state);
  25 of the 62-key rename pool are genuine renames whose Orca key was a false
  gap, 34 are Pinch-specific, and 3 are *duplicate spellings of live keys*.
  Auto-set flags stay in scope as pipeline **outputs**; MMU toolchange physics
  stays in with no special sequencing.
- [02 — Set the canonical-parity evidence standard for gap packets](issues/02-parity-evidence-standard.md)
  — packets may assume the in-tree `OrcaSlicerDocumented/` checkout (readable,
  not runnable). Evidence is canonical function-read + described behaviour
  pinned by **invariant tests** — goldens are impossible here; porting
  OrcaSlicer's own `tests/fff_print/` assertions is acceptable with the
  attribution header. Plumbing keys need only default-matches-upstream +
  reaches-the-consumer. Unverifiable behaviour is surfaced to the human first
  and only filed as a `DEVIATION_LOG.md` row with their sign-off; never
  blocks. Boilerplate lives as the `parity-evidence` snippet in the
  spec-packet-generator skill.
- [04 — Define the cost rubric that makes "cheapest-first" decidable](issues/04-cost-tiering-rubric.md)
  — **A=119, B=226, C=15, D=47, X=11 — 407 keys in scope** (403 at 04's
  closure; +2 reclassified by ticket 07, +2 by ticket 99). Tier A = plumbing
  into an existing decision point (owner + decision point exist); B = new
  logic in an existing owner; C = new granular module at a new seam; D =
  deferred (per-filament config model); X = out of scope. Owners were
  **verified in code and adversarially reviewed against canonical OrcaSlicer
  five times until convergence** (~90 corrections): flow ratios, spiral,
  seam clipping and retraction are emission-time (host emitter
  `crates/slicer-gcode`); shell thickness is object-level planning;
  toolchange keys are emission-time not wipe-tower; 11 filament keys are
  global not per-filament; 11 keys ruled out of scope (dead-in-canonical 8,
  preset-management 3). 5 ResolvedConfig-only keys are Tier A
  manifest-declaration work. Tie-breaker: owning module. Full per-key table
  in [`04-asset-tier-assignment.md`](issues/04-asset-tier-assignment.md).
- [05 — Decide packet granularity and grouping](issues/05-packet-granularity.md)
  — **the queue is 91 packets (18 A, 67 B, 6 C; 358 keys)** (354 at 05's
  closure; P14 +`ironing_type` and P01 +`fan_max_speed`/`fan_min_speed` become
  mixed A/B — tickets 07/99 — P15 +`support_ironing` stays A). Grouping: owning
  module, then Orca UI section; tier is a purity check + queue-order key.
  Ceilings by tier: A ≤ 25, B ≤ 12, C ≤ 4 (split by sub-theme: Prime tower
  13+13, Retraction 10+10, Seam 8+8, Walls 9+9, interlocking 3+3). No merging
  of small groups (36 packets are ≤2 keys — packet 212 precedent). ADRs only
  for interlocking + mmu-segmented-region, authored inside the packet ticket.
  Full list in
  [`05-asset-packet-list.md`](issues/05-asset-packet-list.md) — the 91
  authoring tickets are cut from it. 47 D + 2 fog-blocked A keys
  (`filament_density`, `filament_diameter`) not packetized.
- [06 — Settle packet numbering and how this queue interleaves with live work](issues/06-queue-numbering-and-sequencing.md)
  — **one packet number at a time, allocated by directory existence, derived
  from disk at authoring time; no reserved block.** Derivation command:
  `ls -d docs/spec_packets/[0-9]*/ | sed ... | sort -n | tail -1`; next free =
  +1; letter suffixes only for re-splits (210a/b precedent). All packets born
  `status: draft`; activation is a `/swarm`-time act. Authoring proceeds in
  parallel with live packet work (200–205 are committed; 200/201 have in-flight
  edits) — no merge blocking; numbering decouples via Rule 1.
- [07 — Document the Orca→Pinch alias map and retire the hand-maintained ❌ column](issues/07-alias-map-and-column-retirement.md)
  — **standardise to Orca's names, don't document; the alias map is
  eliminated, not maintained.** 26 mechanical renames (22 exact rows + 3
  duplicate collapses + `ironing_spacing_mm`) executed as workstream tickets
  **99–107**, gating the queue (08 blocked by all nine). Shape changes stay
  out of the rename: `raft_layers` 1→3 split is a strict superset (recorded
  divergence, no gap); the two narrowed enums are **gaps** — the shared
  `ironing_enabled` bool can't express `ironing_type`'s modes nor toggle the
  two Orca features independently → P14 +`ironing_type` (B), P15
  +`support_ironing` (A). 34 Pinch-specific keys untouched. ❌ column
  retirement ruled **out of scope** (tooling hygiene; queue never reads it).
- [99 — Rename part-cooling keys to Orca names](issues/99-rename-part-cooling-keys.md)
  — four renames merged, tree green on all gates. The rename **exposed a
  scale deviation**: Orca's `fan_max_speed`/`fan_min_speed` are percent
  (0–100) while Pinch's were raw 0–255, and `fan_min_speed` was declared but
  never read → reclassified as gap work, **P01 +`fan_max_speed`/`fan_min_speed`
  (Tier B)** — queue is now 407 keys, 358 in packets (18 A / 67 B / 6 C).
  Known pre-existing condition reported: guests appearing stale on a clean
  tree (unrelated to renames). **Explained by ticket 100:** the parity
  harness calls an artifact stale when the newest source mtime exceeds the
  artifact mtime, so any operation that rewrites sources without changing
  them (`git stash push`/`pop`, a branch switch) trips it while
  `build-guests --check`, which uses a different criterion, still passes.
  Rebuilding the guests clears it.
- [100 — Rename wipe-tower keys to Orca names](issues/100-rename-wipe-tower-keys.md)
  — four renames merged, but **the rename was not mechanical**.
  `bed_shape` → `printable_area` is a *value-format* divergence: Orca
  serialises the bed as point strings (`["0x0","250x0",…]`), this port as an
  interleaved float list, so adopting the name alone broke 3MF ingestion
  (`expected Float value, got String`). Resolved in-ticket with an input
  adapter (`slicer_ir::parse_orca_point_string`), not a representation change.
  Defaults aligned to Orca: `prime_volume` 10.0 → **45.0**,
  `enable_prime_tower` true → **false**. The rename also exposed that
  `gen-config-docs`' deviation gate **never compared any boolean default in the
  tree** (`num_of` returned `None` for bools); fixing it surfaced 8 bool
  deviations — 6 aligned (`enable_support` ×4 owners → false,
  `detect_thin_wall` → false, `slowdown_for_curled_perimeters` → true), and
  `precise_outer_wall` held at `false` as **DEV-158** because default-on
  reorders classic-perimeters' walls (a defect, not a spacing difference).
  Map wiring corrected: the queue gate the Notes claimed did not exist, so 67
  packet tickets were re-wired to gate on the rename tickets touching their
  owner; **P01 (ticket 08) is now the unblocked queue head**.
- [08 — Author packet P01 — Cooling / Notes — part-cooling](issues/08-author-packet-p01-cooling-notes-part-cooling.md)
  — ⚠ **Correction required (Authoring rules):** `dont_slow_down_outer_wall` is declared+emitted with no slowdown stage — build the stage or drop the key; re-verify the 4 header/footer co-declarations are consumed by templates, not padding. Packet `docs/spec_packets/253-part-cooling-fan-scale-and-cooling-keys/`
  authored (`draft`), preflight **PASS**: percent-normalizes the two fan-scale
  keys, ports the canonical fan curve + role-fan/±1/threshold/re-timing
  semantics, co-declares the 4 header/footer keys into `machine-gcode-emit`
  for placeholder reachability, and records honest dispositions —
  `dont_slow_down_outer_wall` has no in-tree slowdown decision point (declared +
  emitted, gap recorded; the stage is future work). Grounding also found the
  snapshot's `overhang_fan_threshold` default (50%) contradicts a fresh
  canonical read (`95%`, `Overhang_threshold_bridge`) — packet follows the fresh
  read. Ledger fact: `OrcaSlicerDocumented/` is the **sibling**
  `..\pinch_n_print_cli\OrcaSlicerDocumented` in this clone, not in-tree; future
  tickets/packets must pin that path.
- [09 — Author packet P02 — Multimaterial / Prime tower (1/2) — wipe-tower](issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md)
  — ⚠ **Correction required (Authoring rules):** 12 of 13 keys declared-with-gap; the packet must implement the interface / ramming / framework / brim / travel-avoid behaviours or shed those keys as unimplemented. Packet `docs/spec_packets/254-prime-tower-keys-wipe-tower/` authored
  (`draft`), preflight **PASS**. Grounding found **only one of the 13 keys has
  a live decision point** (`prime_tower_infill_gap` → the tower's scan-line
  pitch, hardcoded `y += line_width` today); the packet wires that one
  (output-changing at defaults: 0.4 → 0.6 mm pitch) and records
  decision-point gaps for the other 12 (interface cluster, ramming,
  framework, brim/Auto, flat-ironing, skip-points travel-avoid — the last
  canonically a **plain bool**, not a point list). Six canonical coFloats/coInts
  keys declared scalar-global per ticket 04's ruling; per-filament model
  stays with the Tier-D fog. Percent-default threading into the CONFIG_BLOCK
  (packet-185 machinery) verified in code, not assumed. No deviation rows; no
  human sign-off consumed.
- [10 — Author packet P03 — Multimaterial / Prime tower (2/2) — wipe-tower](issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md)
  — ⚠ **Correction required (Authoring rules):** 10 of 12 keys declared-with-gap and the one wired key is identity at defaults; implement the cone/rib/fillet/rotation/wall-type geometry or shed. Packet `docs/spec_packets/255-wipe-tower-geometry-keys/` authored
  (`draft`), preflight **PASS**. Grounding found one live decision point:
  `wipe_tower_extra_flow` wires to the purge scan-lines' hardcoded
  `flow_factor: 1.0` (identity at defaults); 10 keys declared-with-gaps
  (cone/rib/fillet/bridging/rotation/wall-type/flush/ramming/sparse — all
  canonically scalar, the Tier-D fog is *not* engaged); and one **alias
  finding**: host key `wipe_tower_speed` already implements canonical
  `wipe_tower_max_purge_speed` (defaults both 90) — excluded from the packet
  as a duplicate-spelling and filed as
  [108 — Adjudicate `wipe_tower_speed` → `wipe_tower_max_purge_speed`](issues/108-adjudicate-wipe-tower-speed-alias.md).
  P03 therefore covers 12 keys, not 13. Output change at defaults is exactly
  +2 CONFIG_BLOCK lines (the two percent defaults thread via packet-185);
  geometry byte-identical. No deviation rows; no human sign-off consumed.
- [101 — Rename path-optimization keys to Orca names](issues/101-rename-path-optimization-keys.md)
  — three renames merged (`retract_length` → `retraction_length`,
  `retract_speed` → `retraction_speed`, `travel_z_hop` → `z_hop`), tree green
  on all gates. **User ruling: align both mismatching defaults** —
  `retraction_speed` 25.0 → 30.0, `z_hop` 0.0 → 0.4 with canonical range
  [0, 5] adopted; deviation table stays at 27 rows (no new deviations).
  The wipe-tower-owned `retract_length` (host typed arm, 2.0, consumed by
  `retract_length_for_tool`) is a different key — canonical's toolchange
  retract is `retract_length_toolchange` (Tier B queue) — and stays.
  **Guest-artifact correction to ticket 99's note:** guest WASMs *do* embed
  config key names, so renames must rebuild guests (proven by byte-search).
  Two pre-existing reds repaired in-ticket: the core-module count test
  (22 → 23, packet 246's wave-overhangs) and the wire-version pin
  (1.0.0 → `CONFIG_SCHEMA_WIRE_VERSION` 1.1.0) plus the last
  `check-literals` violation; `slicer-sdk --doc` remains red at HEAD
  (13 doc examples missing `ExtrusionPath3D.order_lock`) — flagged to the map.
- [102 — Rename classic-perimeters and seam keys to Orca names](issues/102-rename-classic-perimeters-seam-keys.md)
  — three renames merged (`wall_count` → `wall_loops` across
  classic/arachne/wave + the host typed field,
  `smaller_perimeter_threshold_mm` → `small_perimeter_threshold`,
  `seam_mode` → `seam_position` on both seam modules), **defaults aligned to
  Orca by user ruling**: `wall_loops` 3 → 2 (host `ResolvedConfig` was
  already 2 — the tree was internally inconsistent at HEAD) and
  `small_perimeter_threshold` 0.8 → 0.0; deviation table stays at 27 rows.
  The rename **surfaced a pre-existing latent defect, fixed in-ticket**: the
  wasm dispatch escalated every module error — including the WIT contract's
  `fatal=false` "logs and continues" — into a layer-fatal, so activating the
  real aligned-seam path (the 3MF's `seam_position` finally reaching the
  placer under its new name) aborted every painted slice on the seam
  placer's designed code-6 degraded fallback. Non-fatal now logs-and-continues;
  the three test-guests' intentional-error witness channel flipped to
  `fatal` to keep the macro round-trip assertions meaningful under the
  corrected contract. Four test baselines updated to the Orca-aliged
  defaults with measured justification. Three follow-ups flagged to the
  fog: the seam plan never covering painted-variant regions (per-layer
  degraded fallbacks on any aligned painted slice — non-fatal today), the
  degraded warn not surfacing in slice degraded stats, and the persistent
  `slicer-sdk --doc` red. Gates green; guests rebuilt fresh.
- [108 — Adjudicate `wipe_tower_speed` → `wipe_tower_max_purge_speed`](issues/108-adjudicate-wipe-tower-speed-alias.md)
  — Q6(a) was implemented: the host key uses the canonical name and
  `DefaultGCodeEmitter::resolve_feedrate` caps wipe-tower paths at the lower of
  the configured maximum and `sparse_infill_speed`; canonical min-10 validation
  is deferred to ticket 113.
- [11 — Author packet P04 — Printer / Machine / Print volume — wipe-tower](issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md)
  — ⚠ **Correction required (Authoring rules):** wires only the wipe-tower corner check; canonical's feature is object-footprint validation (`Print::validate`), recorded here as a gap — implement it at the port's validation seam. Packet `docs/spec_packets/256-wipe-tower-bed-exclude-area/` authored
  (`draft`), preflight **PASS**. Grounding found `bed_exclude_area` is a true
  zero-occurrence gap whose canonical consumers **disagree on the value's
  geometry** (one polygon in `get_bed_excluded_area`, 4-point rectangles in
  `Model.cpp`, exactly-4 in `get_path_of_change_filament`) and that the wipe
  tower itself is never validated against it — the packet follows the validation
  consumer (`Print::validate`, fatal collision message) translated to the port's
  only live bed-validation decision point: the wipe-tower `run_finalization`
  4-corner check. Canonical's degenerate single-point default → **no manifest
  default** (no doc-15 deviation row, no CONFIG_BLOCK line at defaults);
  degenerate values decay to no-exclusion. Orca 3MF point-string ingest rides
  the ticket-100 adapter unchanged (`slicer_ir::parse_orca_point_string`) —
  zero host-side changes. The object-hull validation (canonical's fuller
  semantics) is recorded as a Tier-B/C gap, and the tier row's gcode-side half
  is deferred to the `printable_height` P18/P19 family.
- [12 — Author packet P05 — Others / Brim — skirt-brim](issues/12-author-packet-p05-others-brim-skirt-brim.md)
  — ⚠ **Correction required (Authoring rules):** 4 of 5 keys declared-with-gap (`brim_type` modes beyond `no_brim`, ears, efc outline); implement in `skirt-brim` or shed. Packet `docs/spec_packets/257-brim-type-and-brim-keys/` authored
  (`draft`), preflight **PASS**. Scope ruling (user-confirmed): P05 covers
  **5 keys, not 6** — canonical's `brim_ears` bool is dead (declared in
  `PrintConfig.cpp`, no reads, no typed-struct member; ear physics live in
  `brim_type` modes `brim_ears`/`painted`) → dead-in-canonical out-of-scope;
  **queue 407 → 406**. Exactly one live decision point exists in-tree (the
  on/off gate in `SkirtBrim`); the packet wires `brim_type = "no_brim"`
  suppression (default-path identity + `brim_width` precedence pinned by
  invariant tests) and declares the other four keys with-gap, each with its
  canonical consumer pinned (`Brim.cpp::outer_inner_brim_area`,
  `make_brim_ears_auto`, `use_brim_efc_outline`). Manifest-declared defaults
  are canonical-identical — no deviation rows. CONFIG_BLOCK padding twins
  stay (module bool/int/float/enum manifest defaults don't thread into raw
  config; packet-254/255 precedent); explicit values reach the block once via
  `emit_config_kv` dedup (AC-5).
- [13 — Author packet P06 — Others / Skirt — skirt-brim](issues/13-author-packet-p06-others-skirt-skirt-brim.md)
  — ⚠ **Correction required (Authoring rules):** `skirt_type` (per-object skirt) and `min_skirt_length` declared-with-gap; implement per-object grouping, and either build the e-per-mm input or shed `min_skirt_length` to Tier D. Packet `docs/spec_packets/258-skirt-type-and-draft-shield-keys/` authored
  (`draft`), preflight **PASS**. All 5 keys verified true zero-occurrence gaps.
  **Three wired** (decision points re-derived in code): `draft_shield` →
  skirt layer span extends to the full layer set (`Print::has_infinite_skirt`
  semantics), `single_loop_draft_shield` → innermost loop only on
  `global_layer_index > 0` (`GCode::generate_skirt`'s `!first_layer`), and
  `skirt_start_angle` → corner-nearest ring rotation of the first-layer
  first-emitted loop, with the start point's reachability to final G-code
  verified at authoring (ticket 100's lesson) — default −135° selects the
  existing corner, so default output is byte-identical. **Two
  declared-with-gap:** `skirt_type` (needs per-object skirt grouping;
  default `combined` matches today) and `min_skirt_length` (needs a
  per-filament e_per_mm model — Tier-D fog; default 0 = disabled). Two
  recorded divergences (packet-257 class): the port emits skirt loops
  innermost-first (canonical exports outermost-first), so canonical's
  rotated-start condition lands on the outermost wall there vs the innermost
  here; corner-nearest selection instead of mid-edge seating. No deviation
  rows; no `ORCA_CONFIG_PADDING` twins (AC-6 pins honest absence). Preflight
  corrected the Doc-Impact grep against a disk probe (the generated doc has
  no per-module headings — key-presence verification, 257's corrected form).
- [103 — Rename fuzzy-skin keys to Orca names](issues/103-rename-fuzzy-skin-keys.md)
  — adopted canonical `fuzzy_skin_thickness` / `fuzzy_skin_point_distance`
  without aliases, aligned defaults to 0.2 / 0.3 by user ruling, regenerated
  config docs with no new deviation rows, and left the next fuzzy-skin packet
  authoring ticket unblocked.
- [104 — Rename support/layer-planner keys to Orca names](issues/104-rename-support-layer-planner-keys.md)
  — `support_top_z_distance_mm` → `support_top_z_distance` (traditional +
  tree manifests, guests, host `SupportGeometryIR` field, prepass) and
  `first_layer_height` → `initial_layer_print_height` (layer-planner-default
  manifest + guest, host `ResolvedConfig` cli field + `to_config_map`,
  `region_mapping` overlay, emitter, stats, 13 fixture JSONs). The support
  rename reconnected two plumbing-disconnected spellings: the Orca name
  already existed host-side, and the module-view filter had kept it from
  reaching the planners — one explicit value now feeds both prepass and
  planner. Deviation triage (user ruling): manifest `first_layer_height`
  default 0.3 → **0.2** (canonical + host + live behaviour were already 0.2;
  doc-only change). Deviation count stays 27. Unblocks the 13 packet tickets
  gated on 104 (P11–P13, P29–P31, P68–P70, P72, P81–P83).
- [14 — Author packet P07 — Others / Fuzzy Skin — fuzzy-skin](issues/14-author-packet-p07-others-fuzzy-skin-fuzzy-skin.md)
  — ⚠ **Correction required (Authoring rules):** 5 of 7 keys declared-with-gap (`fuzzy_skin_mode`, noise type/octaves/persistence/scale); noise types are alternative algorithms → module/claim shape per rule 4; the padding edit is not a deliverable. Packet `docs/spec_packets/259-fuzzy-skin-keys/` authored (`draft`),
  preflight **PASS**. Canonical read corrected the snapshot: `fuzzy_skin` is
  an **enum** (`none/external/hole/all/allwalls/disabled_fuzzy`, default
  `disabled_fuzzy`), not a bool — the master loop-selection switch
  (`should_fuzzify`'s `fuzzify_contours`/`fuzzify_holes`). **Two wired**:
  `fuzzy_skin` → the module's loop-selection gate (`external`/`all` → outer
  contour, `allwalls` → every loop, `none` → the per-vertex flag path,
  `hole` → inert) and `fuzzy_skin_first_layer` → the layer-0 pass-through
  gate. **Five declared-with-gap**: `fuzzy_skin_mode` (Arachne
  extrusion-line-only width semantics — the port is a `fuzzy_polyline`
  Polygon-path port), `fuzzy_skin_noise_type`/`octaves`/`persistence`/`scale`
  (libnoise coherent modules; the port's xorshift RNG is the `UniformNoise`
  (classic) analogue, so defaults are behaviorally faithful). Recorded
  divergence: the IR has no `LoopType::Hole` (hole boundaries are
  `LoopType::Outer` at `perimeter_index 0`), so `hole` is inert and `all`
  degrades to `external`. **Padding correction**: the preflight sweep found
  `fuzzy_skin`/`fuzzy_skin_mode` already in `ORCA_CONFIG_PADDING`
  (`crates/slicer-gcode/src/serialize.rs`); the `fuzzy_skin` value `"none"`
  contradicted the canonical default and is corrected to `"disabled_fuzzy"`
  (no entries gained/lost). Behavior changes (canonical-alignment, test
  fallout pre-baked): default `disabled_fuzzy` is inert (apply_to_all alone
  no longer fuzzes) and layer 0 passes through at default. No deviation rows;
  no human sign-off consumed.
- [105 — Rename host and infill-angle keys to Orca names](issues/105-rename-host-infill-angle-keys.md)
  — **one rename merged, one adjudication corrected** (both on this ticket).
  `infill_angle` → `infill_direction` verified exact against canonical
  (`Fill/Fill.cpp` `calculate_infill_rotation_angle`) and renamed across
  gyroid-infill + rectilinear-infill, host `ResolvedConfig` field/key,
  `region_mapping` overlay + lightning consumers, tests, and the dragon-curve
  community example (mirrors rectilinear's spellings by design) — defaults
  byte-identical 45.0, zero deviation rows, zero sign-off consumed. **The
  `gcode_resolution` → `resolution` row was re-adjudicated (human challenge,
  verified against canonical): a gap, not a rename** — canonical `resolution`
  is a generation-time **global** simplify (`PerimeterGenerator.cpp`
  `ex.simplify_p`, `Brim.cpp`, `Fill.cpp`, `Layer.cpp`, `PrintObjectSlice.cpp`,
  `Print.cpp`, `TreeSupport`) plus emit-side arc density (`GCodeWriter.cpp`);
  the host's `gcode_resolution` is emit-time and per-role
  (`tolerance_for_role`), so "exact" claimed parity the host doesn't implement
  (ironing-class finding). Records: 03 reclassified (rename pool 25 → 24;
  gap set 414 → **415**), tier **B** in 04, packet **P51** gains `resolution`
  in 05 — queue target **406 → 407**; `gcode_resolution` stays PnP-specific,
  unrenamed, deviation table stays 27 rows. Gates: gen-config-docs/check-literals/
  check/clippy clean; slicer-ir 20 binaries, slicer-core host-algos 599 tests,
  modules + slicer-gcode green, runtime e2e 136/136; all 44 guests rebuilt
  (slicer-ir sits in every guest's dependency closure).
- [106 — Rename ironing keys to Orca names](issues/106-rename-ironing-keys.md)
  — three renames merged, order respected so the two `ironing_spacing` spellings
  never crossed wires: `ironing_flow_rate` → `support_ironing_flow` and
  `ironing_spacing` → `support_ironing_spacing` (support-surface-ironing),
  `ironing_spacing_mm` → `ironing_spacing` (top-surface-ironing; manifest, module
  read sites, tests, benchy fixture + e2e embedded copies). **The rename surfaced
  a value-format deviation (user ruling — align):** canonical `support_ironing_flow`
  is coPercent default **10%** (`ConfigDef.cpp`), the port's 100.0 was consumed
  as a raw `flow_factor` multiplier (`emit.rs` — "1.0 normally; e.g. ~0.1 for
  ironing") → 100× nominal flow at defaults; the deviation gate is blind to it
  (`orca_defaults` parses the Default column with `parse::<f64>`, so `"10%"` never
  enters the comparison map). Aligned default → **0.10**, range [0.01, 1.0]
  (mirrors `ironing_flow`); parity-test config value updated to match. Deviation
  table stays 27 rows. All 44 guests rebuilt (the two ironing guests were the only
  stale ones). **Unblocks P14/P15 (tickets 21, 22).**
- [18 — Author packet P11 — Support / Interface — support-planner](issues/18-author-packet-p11-support-interface-support-planner.md)
  — ⚠ **Correction required (Authoring rules):** two keys were already live (no packet work) and two are declared-with-gap (`support_interface_pattern` dispatch, `support_interface_loop_pattern` contact loops); implement the dispatch as claim-held interface fillers and the loop pass, or shed. Packet `docs/spec_packets/260-support-interface-keys/` authored (`draft`), preflight
  **PASS**. Grounding re-derived the tier picture: the tier-table owner `support-planner`
  (a claim held by the two planner modules) is a mis-attribution — the four keys'
  decision points live in `tree-support` + `traditional-support`, and the packet declares
  there (owner correction rides the closure). **Two keys wired + verified**: the two
  spacing keys are already declared + consumed in both modules (density formula
  canonically exact vs `SupportParameters`). Canonical read overturned the top default:
  Orca's `support_interface_spacing` is **0.5**, not 0.4 (port comment mis-derived; 238c
  had fixed bottom already) — **user ruling: align 0.4 → 0.5**, removing the two known
  doc-15 deviation rows (27 → 25, re-measured). Canonical has **no -1 sentinel** on
  `support_bottom_interface_spacing` (that sentinel belongs to
  `support_interface_bottom_layers`) — the port's negative-mirrors-top branch is a PnP
  extension; **user ruling: keep as recorded divergence** (AC-3 witness + AC-4
  `-1.0`-legal bounds arm). **Two keys zero-occurrence, re-adjudicated declared-with-gap**:
  `support_interface_pattern` (canonical `contact_fill_pattern` branch order pinned;
  sparse-density default resolves to `ipSupportBase`, a `FillSupportBase : FillRectilinear`
  filler at `spacing/density` — same rectilinear family as the port's scan-line, so
  default behavior is structurally faithful) and `support_interface_loop_pattern`
  (**coBool** default false — canonical type correction; `LoopInterfaceProcessor`
  `n_contact_loops` absent in-tree). No deviation rows; no CONFIG_BLOCK twins (none of the
  four keys in `SUPPORT_CONFIG_DEFAULTS`/`ORCA_CONFIG_PADDING`); both modules need the
  `toml` dev-dep for the guard tests.
- [19 — Author packet P12 — Support / Raft — support-planner](issues/19-author-packet-p12-support-raft-support-planner.md)
  — ⚠ **Correction required (Authoring rules):** both keys declared-with-gap on a raft generator that does not exist; fold into / sequence after packet 240's raft geometry, or return the keys to the queue. Packet `docs/spec_packets/261-raft-keys/` authored (`draft`), preflight **PASS**.
  Both keys (`raft_contact_distance` 0.1, `raft_expansion` 1.5) zero-occurrence,
  re-adjudicated **declared-with-gap**: no raft geometry generator exists in-tree (draft
  packet 240-support-raft's `com.core.raft-default` is unimplemented; `RaftPlan` carries
  only layer counts). Canonical consumers pinned: `SlicingParameters::SlicingParameters`
  (raft Z-gap → `gap_raft_object` → `object_print_z_min`; forced to 0 when
  `raft_z_gap == 0.0 || zero_topZ_contact`), `SupportMaterial::generate_contact_polygons`
  (layer_id==0 XY expansion), `TreeSupport3D::generate_raft_contact` /
  `finalize_raft_contact`, `GCode.cpp` `_print_z` warning; the "ignored for soluble
  interface" tooltip is **GUI-only** (`ConfigManipulation.cpp`), not a slicing branch.
  **Owner confirmed, narrowed**: `support-planner` is right, but only `tree-support-planner`
  has raft surface (raft config cluster + `RaftPlan` emission); `traditional-support-planner`
  has none — the packet declares in `tree-support-planner.toml` (canonical defaults +
  bounds, no deviation rows) and pins the traditional omission (AC-N2). No user rulings
  required. Packet-240 relationship recorded (its AC-5 wire-or-record input), not
  deferred. No CONFIG_BLOCK twins (`("raft_layers", "0")` in the padding list is the
  canonical layer-count key, not these two).
- [107 — Collapse infill duplicate spellings to Orca names](issues/107-collapse-infill-duplicate-spellings.md)
  — **two collapses merged, one pair re-adjudicated (user rulings).**
  `infill_density` → `sparse_infill_density` **canonical-percent everywhere**
  (20.0 [0,100] in all five manifests; modules divide by 100; `ResolvedConfig`
  field/key renamed with a new `extract_percent_float` input adapter so Orca
  3MF percent strings resolve — ticket-100 precedent; `get_abs_value` now
  resolves percent strings; new `resolve_percent_float` SDK helper; loader
  part-metadata preserves percent strings; the M3 fixture's 15%/40% overrides
  finally reach the modules). `infill_speed` → `sparse_infill_speed` with
  manifests aligned to canonical 100 (user ruling; live factor stays 1.0 =
  canonical 100 mm/s); host `FeedrateConfig.sparse_infill_speed` untouched;
  deviation table 27 → **26** (measured). **`infill_overlap` re-adjudicated
  NOT a duplicate** — canonical `infill_wall_overlap` (coPercent 15,
  `PerimeterGenerator.cpp` `inset -= infill_peri_overlap`) is already ported
  in classic-perimeters; the linker's 0.45 fraction-of-spacing post-pass is a
  PnP-invented second mechanism; kept live, 03's row updated, collapse count
  3 → 2, rename pool 25 → 24. Also: the persistent `slicer-sdk --doc` red
  (13 `order_lock` doctests) repaired in-ticket (fog cleared); one latent
  prepass bug fixed (0.999 → 99.9 at the `BridgeDepthLayer` thresholds);
  dragon-curve example renamed + wasm rebuilt; all gates green incl. full e2e
  136/136 and 44 guests rebuilt twice.

- [15 — Author packet P08 — Strength / Infill — infill modules](issues/15-author-packet-p08-strength-infill-infill-modules.md)
  — ⚠ **Correction required (Authoring rules):** `sparse_infill_pattern` / `internal_solid_infill_pattern` declared-with-gap as "module identity" — that *is* the claim-holder mechanism; map pattern values to `claim:sparse-fill`/`claim:top-fill` holders (shipping at least the canonical defaults `crosshatch`/`monotonic` as modules), and `gap_fill_target` must gate a real fill-side gap fill or be shed. The padding edit is not a deliverable. Packet `docs/spec_packets/262-infill-pattern-keys/` authored (`draft`), preflight
  **PASS**. **Four keys wired (default-path identity), three declared-with-gap.**
  `solid_infill_direction` → the solid-role angle read in rectilinear + gyroid (sparse
  keeps `infill_direction`; 45 = 45); `sparse_infill_rotate_template` /
  `solid_infill_rotate_template` → per-layer angle from a comma-separated list cycled by
  layer index (canonical `calculate_infill_rotation_angle` list form; the metalanguage
  is declared-with-gap — falls back to base angle with a logged warn); `fill_multiline`
  → sparse-only multiline in rectilinear (canonical `multiline_fill` offset lists; base
  spacing × N, N copies at line-width offsets; gyroid/lightning with-gap — curve
  offsetting is Tier B+). **Pattern keys re-adjudicated with-gap**: the port's pattern
  IS module identity (3 of 26 canonical patterns; host selects via `*_fill_holder`) —
  `sparse_infill_pattern` (26 values, default crosshatch) and
  `internal_solid_infill_pattern` (8 top-fill values, default monotonic) declared
  with-gap, with two recorded behavior divergences at defaults (port rectilinear vs
  canonical crosshatch/monotonic). `gap_fill_target` with-gap: gates canonical's
  **fill-side** gap fill (`_create_gap_fill`), which the port lacks — its gap fill is
  the perimeter-side `process_classic` mechanism, which canonical's key does not gate
  either. 17 manifest tables (rectilinear 7, gyroid 7, lightning 3 — solid-key omission
  pinned AC-N2). **Padding correction**: `("sparse_infill_pattern", "grid")` →
  `"crosshatch"` (ticket-14 precedent). No deviation rows (block stays 26); no user
  rulings; ADR-0027 conformance stated. Unblocks P09/P10 (tickets 16/17) — same owner,
  different keys.

- [16 — Author packet P09 — Strength / Infill pattern-specific — infill modules](issues/16-author-packet-p09-strength-infill-pattern-specific-infill-modules.md)
  — ⚠ **Correction required (Authoring rules):** a pure-declaration packet (10 keys, zero reads). Re-author as the locked-zag / lateral-lattice / lateral-honeycomb pattern modules (claim holders) that consume these keys, or return all 10 to the queue. Packet `docs/spec_packets/263-infill-pattern-specific-keys/` authored (`draft`),
  preflight **PASS**. **All 10 keys re-adjudicated declared-with-gap — a pure-declaration
  packet (zero module-source reads, zero behavior change at any value).** Canonical
  grounding pinned every decision point: six keys (`infill_lock_depth`, both densities,
  both widths, `skin_infill_depth`) are consumed only by `FillLockedZag::fill_surface_locked_zag`,
  `lateral_lattice_angle_1`/`2` only by `FillLateralLattice::fill_surface`,
  `infill_overhang_angle` only by `FillLateralHoneycomb::fill_surface` — all unshipped
  patterns — and `symmetric_infill_y_axis`, the one key with a live in-port decision point
  (the rectilinear scan-line generator), is canonical-activated only when the sparse pattern
  is zigzag/crosszag/lockedzag (`Fill.cpp` `Layer::make_fills` gate, verified verbatim;
  never `ipRectilinear`): wiring it would implement behavior canonical never activates for
  the port's patterns; the zigzag-family re-open condition rides the key's disposition. The
  10 tables land in `rectilinear-infill.toml` (canonical defaults/bounds; percent forms per
  107, width forms per the in-tree convention, bool for the symmetric key); guard is the
  net-new `infill_pattern_specific_config_schema_tdd.rs`, distinct from 262's guard (no file
  collision; shared-manifest append churn recorded as queue-order merge churn; `toml`
  dev-dep add-if-absent). Zero deviation rows (5 parseable float defaults match; `25%`/`100%`
  never enter the numeric comparison map; bool matches under the ticket-100 comparison —
  block stays at 26, re-measured); zero CONFIG_BLOCK padding twins (honest absence pinned by
  AC-4). No user rulings. **P10 (ticket 17) is now the unblocked queue head.**

- [17 — Author packet P10 — Strength / Top/bottom shells — infill modules](issues/17-author-packet-p10-strength-top-bottom-shells-infill-modules.md)
  — ⚠ **Correction required (Authoring rules):** `top_surface_pattern` / `bottom_surface_pattern` declared-with-gap → claim-holder mapping per rule 4 (ship `monotonicline`/`monotonic` fillers); the padding edit is not a deliverable. Density keys stand. Packet `docs/spec_packets/264-top-bottom-surface-keys/` authored (`draft`), preflight
  **PASS** (S0–S8 all green). **Two keys wired, two declared-with-gap.** Canonical
  grounding verified all four keys exist on `PrintRegionConfig` and re-derived the
  dispositions: **`top_surface_density` / `bottom_surface_density` (coPercent 100; top min
  0, bottom min 10) wired** into the rectilinear top/bottom solid spacing decision points
  (`solid_spacing = line_width / SOLID_DENSITY`, `SOLID_DENSITY = 1.0` — canonical
  `FillLine.cpp` `FillLine::_fill_surface_single`'s `line_spacing = flow.spacing() /
  density` shape), exposed-surface-only (canonical `group_fills` gives `stInternalSolid` a
  fixed `100.f`), with the canonical `density <= 0` skip wired as a `density > 0` gate on
  the top block (bottom gate provably inert under min 10) — defaults 100 → fraction 1.0 →
  byte-identical (AC-2), non-default values change spacing (AC-3). **`top_surface_pattern`
  / `bottom_surface_pattern` (coEnum, 8 values; defaults `monotonicline` / `monotonic`)
  declared-with-gap** — filler selection is module identity (packet 262's finding, unchanged
  for the surface roles); canonical's other pattern reads (extra-internal-solid-fill branch,
  `GCode.cpp` `_needSAFC`/`retract`) and the density keys' surface-expansion gates
  (`detect_surfaces_type`, `top_fill_replaces_inner_walls`) recorded, not wired. **One
  padding correction**: `("top_surface_pattern", "monotonic")` → `"monotonicline"` in
  `ORCA_CONFIG_PADDING` (ticket-14/262 precedent; the bottom twin already matches). The 4
  tables land in `rectilinear-infill.toml` only; gyroid's ADR-0027 opt-in solid path rides
  the sparse density (recorded divergence, not wired — wiring would change gyroid solid at
  defaults) and lightning is sparse-only; both omissions pinned (AC-N2). Guard is the
  net-new `top_bottom_surface_config_schema_tdd.rs` (distinct from 262/263's guards; `toml`
  dev-dep add-if-absent; shared-manifest append churn with 262/263 recorded as queue-order
  merge churn). Zero deviation rows (enum defaults never enter the numeric comparison map;
  `100%` fails `parse::<f64>` — block stays at 26, re-measured). No user rulings. Unblocks
  nothing downstream; **P13 (ticket 20) is the next unblocked queue head**. Two fog items
  graduated to Not yet specified (gyroid solid-density path; extra-internal-solid-fill
  machinery).

- [20 — Author packet P13 — Support / Support — support-planner](issues/20-author-packet-p13-support-support-support-planner.md)
  — ⚠ **Correction required (Authoring rules):** five keys declared-with-gap (`raft_first_layer_expansion`, `support_bottom_z_distance`, `support_critical_regions_only`, `support_object_first_layer_gap`, `support_remove_small_overhang`) and six were already live; implement the five or shrink to `enforce_support_layers` + the type corrections. Packet `docs/spec_packets/265-support-support-keys/` authored (`draft`), preflight
  **PASS** (S0–S8 + AC + Doc-Impact green). Re-derivation split the 12 Tier-A keys into
  four states: **six already wired + canonical-faithful** (`support_object_xy_distance`
  0.35/[0,10] in both planner manifests + both clearance reads; `support_threshold_angle`
  30.0 host-typed + traditional-side declaration + alias, tree-side asymmetry recorded;
  `support_style` — tree-side 7-value enum with the **traditional-side string → enum type
  correction**; `support_type` — the family selector, functional via raw config but
  manifest-less, **now declared as the canonical 4-value enum in both planners** (global
  path enum-enforced; per-object tolerant fallback recorded); `support_expansion` 0.0
  wired; `support_threshold_overlap` 50% percent wired) — pinned, not changed; **one wired
  by the packet**: `enforce_support_layers` — decision point existed (`force_support =
  layer_id < enforce` branch; slicer-core arms already pinned the geometry) but
  `resolve_contact_params` hardcoded `0`; now reads the typed CLI field (default 0 →
  identity; tree-family `-0.15 × extrusion_width` nuance recorded); **five re-adjudicated
  declared-with-gap**: `raft_first_layer_expansion` (zero-occurrence; canonical default
  **2.0**, not the 3.0 of the older BBS comment — fresh read; tree planner only, AC-N2
  pins the traditional absence) + `support_bottom_z_distance` /
  `support_critical_regions_only` / `support_object_first_layer_gap` /
  `support_remove_small_overhang` (host-declared canonical defaults, zero read sites).
  Declare homes: 9 tables `tree-support-planner.toml`, 8 `traditional-support-planner.toml`
  (raft cluster, P12 precedent); owner correction recorded (decision points span host
  analysis + scheduler + planners); guards net-new ×3 + non-perturbation harness ×1
  (avoiding 253/260/261's planned filenames); integration arms in existing binaries.
  Deviation block stays 26 (all defaults canonical, re-measured); zero CONFIG_BLOCK twins;
  zero user rulings. No new fog graduated (traditional-raft fog's P13 reference now
  resolves to the declaration; the traditional-raft-handling question stays open).
   Unblocks nothing downstream; **P14 was the next unblocked queue head and is resolved below**.

- [21 — Author packet P14 — Quality / Ironing — top-surface-ironing](issues/21-author-packet-p14-quality-ironing-top-surface-ironing.md)
  — ⚠ **Review under Authoring rules:** modes and inset are real; the relative-angle fallback substitutes a layer-index turn for canonical's solid-infill-direction base — carry the direction through `SliceRegionView` (rule 4) or record it as a divergence with rationale, not fog. Packet `docs/spec_packets/266-top-surface-ironing-keys/` authored as `draft`,
  preflight **PASS** (S0-S8, AC-command, and Doc Impact checks). Canonical
  grounding corrected the ticket's both-manifest premise: all four P14 keys are
  consumed only by top-surface ironing, so the packet replaces the top module's
  gate and leaves support-surface-ironing for P15. Exact canonical relative-angle
  parity is recorded as fog because the current region view has no solid-infill
  direction metadata; the packet specifies a deterministic layer-index fallback.
- [22 — Author packet P15 — Support / Support ironing — support-surface-ironing](issues/22-author-packet-p15-support-support-ironing-support-surface-ironing.md)
  — the generated packet was discarded and its single atomic change was
  implemented directly in session. **P15 covers one key, not two.** `support_ironing`
  (canonical `coBool` false, `SupportParameters`' ctor →
  `generate_support_toolpaths`' `support_params.ironing && !top_contact_layer.empty()`
  gate) is **wired** by the support manifest and
  `SupportSurfaceIroning::from_config`: it replaces the PnP `ironing_enabled`
  bool that both ironing modules declare independently, per Q10(b). That bool is a
  two-way reachability bug, not just a name — an Orca config setting
  `support_ironing = 1` cannot enable support ironing at all, and a user
  enabling top-surface ironing silently gets support ironing too. Default
  `false` = current default = absent-key behaviour, so the default path is
  byte-identical; the change is at `true`, and reachability through the real
  host path is pinned by the support integrated-parity contract test.
  `support_ironing_pattern` is **returned to the queue as unimplemented**:
  a `coEnum` over `InfillPattern` feeding
  `Fill::new_from_type(support_params.ironing_pattern)` — holder-only under
  rule 4 / Q3(a), and this port has no support-ironing claim, no holder key,
  and no concentric filler (Tier C, not the Tier A it was tiered at). Records
  updated in 04 and 05; missing feature named. Two pre-existing divergences
  recorded, neither created here: **the port irons a different subject than
  canonical** (canonical irons the support top *contact* layer; this module
  gets only `&[SliceRegionView]` at `Layer::SupportPostProcess` and scan-fills
  every region polygon) and canonical's `top_interfaces` precondition is
  unexpressible — the first is graduated to the fog below. Q11(a)'s
  `ironing_speed` → `support_ironing_speed` rename was flagged, not folded in,
  and is filed as ticket 109. No user rulings, no deviation rows, no
  `ORCA_CONFIG_PADDING` edit.
- [23 — Author packet P16 — Quality / Wall generator — Arachne — arachne-perimeters](issues/23-author-packet-p16-quality-wall-generator-arachne-arachne-perimeters.md)
  — user chose direct closure over packet authoring because the production path
  was already live: `min_feature_size` is a canonical `percent` resolved against
  `nozzle_diameter` and passed to the widening strategy. Added
  `percent_min_feature_size_reaches_widening_threshold`, proving a `0.15 mm`
  strip emits at `25%` and is rejected at `50%` of a `0.4 mm` nozzle.
- [Key correction inventory — grilling rulings](issues/key-correction-inventory.md)
  — 26 rulings over the 140 in-scope rows of the 212-row key audit, in that
  file's `## Decisions — 2026-09-01` section. Five are map-level and are folded
  into the rules above: **Q3** holder-only (rule 4) removes ten
  algorithm-selecting enums from the declared-key set permanently and reshapes
  packets 262/264; **Q8** supplies rule 4's missing trigger test (cross-module
  selection, not in-module mode branching) and confirms rules 1–6 bind packet
  tickets, not merged tree code; **Q5** corrects rule 2's premise — the padding
  table is load-bearing, not cosmetic, because canonical *throws* below 80
  CONFIG_BLOCK pairs — and rules it derived rather than hardcoded; **Q14(c)**
  narrows the ticket-04/12 `brim_ears` precedent rule 3 cites, returning the
  ears feature to scope via `brim_type`; and the **dead-manifest-defaults**
  hazard (Q11) invalidates every manifest-verified default-alignment claim in
  tickets 99–107 and packets 253–266 for plain-typed keys.
  Key-level rulings of note: part-cooling converts to percent 0–100, fixing a
  live Orca-3MF ingestion bug (**Q4**); `slowdown_for_curled_perimeters` reverts
  to `false` — ticket 100 aligned it backwards (**Q9**); skirt/brim defaults
  align to 1 / 2 / 0 and `skirt_brim_enabled` retires (**Q14**); `apply_to_all`
  and `ironing_enabled` retire into `fuzzy_skin` / `ironing_type` +
  `support_ironing` (**Q10**); `wipe_tower_speed` renames and adopts canonical's
  cap semantic, closing ticket 108 (**Q6**). Eight in-scope key groups were left
  unruled and are listed in that section, as are four factual corrections to the
  audit itself.
  **Its eight "follow-up ticket needed: yes" rulings were never filed** — they
  sat as table rows inside an already-resolved ticket, on nobody's frontier.
  Filed 2026-09-02 (ticket 22's session) as **109–116**: 109 Q11(a)
  `support_ironing_speed` + `SPEED_KEYS` membership; 110 Q3(b) unmatched
  `*_fill_holder` must fail validation; 111 Q4(a)/(b) fan scale → percent and
  `overhang_fan_speed` absolute; 112 Q5 derive the CONFIG_BLOCK padding from the
  resolved config; 113 Q6(b) `FeedrateConfig` range validation; 114 Q11(b)
  `sparse_infill_speed` resolved default + `speed_factor` base; 115 Q13 retire
  `support_sharp_tails`; 116 Q15(a) document the `_mm` marker convention.
  **Verifying them against the tree corrected three of the inventory's own
  rationales**, so read that document's symbol claims as unverified until
  greped: (a) `resolve_held_claims`
  (`crates/slicer-scheduler/src/validation.rs`) does **not** "yield empty for
  every module" — it returns non-empty when the configured holder matches; the
  real gap is that nothing detects a holder naming a module no manifest matches;
  (b) Q11(b)'s "touches 3 infill modules" is not three of a kind —
  `rectilinear-infill` has **no** `BASE_SPEED` (gyroid and lightning do), so a
  two-module re-base would silently miss it; (c) the feedrate table is
  `SPEED_KEYS`, never `FEEDRATE_KEYS`. Ticket 110 is the load-bearing one: Q3(a)
  makes `*_fill_holder` the *only* selection channel for ten enums, and it
  currently has no safety net.

## Not yet specified

- **The prepass seam plan never covers painted-variant regions.** Surfaced
  by ticket 102: `PrePass::SeamPlanning` runs before
  `PrePass::PaintSegmentation`, so its plan keys are `(global_layer_index
  ≥ 1, region_id 0, chain [])` only — painted-variant `PerimeterIR`
  regions (material-chain ids 1/2/3/…) never match, and the aligned
  `seam-placer` takes its code-6 degraded fallback once per painted
  region per layer (`seam_degraded_fallback_tdd.rs` semantics: walls
  preserved, local candidate chosen). Non-fatal since the dispatch fix,
  but every aligned painted slice now emits one warn per region per
  layer (~125 on cube_4color). The fix shape (plan per painted variant,
  or teach the placer to key the base region's plan entry) is geometry
  work — queue-sized, not a rename follow-up. Fog until a packet picks
  it up.
- **Degraded module errors don't surface in slice stats.** The wasm
  dispatch's `fatal=false` warn is a log line only; `SliceEventCollector`
  never hears it, so a slice carried entirely on degraded fallbacks still
  reports `degraded: false` / `non_fatal_error_count: 0`. Observability
  gap only (docs/09 §Required Events); pair it with the seam-plan fix
  above when that packets.

- **Object-footprint validation against `bed_exclude_area`.** Surfaced by
  ticket 11: canonical `Print::validate` intersects each model volume's 2D
  convex hull with the exclusion polygon (fatal, `Print.cpp`); the port's
  packet 256 wires the wipe-tower rectangle instead (the only live bed
  decision point) and records the object-hull check as a gap. Whether to
  build the object-side check — and where it lives in this tree's
  orchestration — depends on whether the print-orchestration packets
  (P18/P19, tickets 86/87) stand up a `Print::validate`-level stage. Fog
  until those packets' grounding decides.
- **Where filament-level config even lives.** 47 keys (Tier D) are deferred
  on this question: does Pinch 'n Print have a per-filament config model at
  all, or do these keys imply a new subsystem? 11 filament keys were found
  to be global (not per-filament) and are assignable now (ticket 04).
  Revisit once the queue reaches Tier D. Graduating with it: 2 fog-blocked
  Tier A keys (`filament_density`, `filament_diameter` — declare-in-manifest
  work whose manifest home depends on the model).
- **Hole-loop identification in the wall IR.** Surfaced by ticket 14's
  authoring: canonical `fuzzy_skin = "hole"` / `"all"` (contour+hole) cannot
  be wired because `LoopType` has no `Hole` variant and classic-perimeters
  emits hole boundaries as `LoopType::Outer` at `perimeter_index 0` —
  indistinguishable from the contour. Packet 259 records `hole` as inert and
  `all` as degrading to `external`. Whether the IR gains a `LoopType::Hole`
  variant (or hole metadata on `WallLoop`) — and which consumers beyond
  fuzzy-skin would use it — is queue-sized IR work, not a queue packet;
  fog until a packet or IR effort picks it up.
- **Support-interface pattern dispatch and angle specialization.** Surfaced by
  ticket 18's authoring: canonical `support_interface_pattern` selects the
  interface filler through `SupportParameters`' `contact_fill_pattern` branch
  order (grid→`FillGrid`, interlaced→`FillRectilinear` with ±45° alternation,
  auto-with-zero-gap→`FillConcentric`, density>0.95→`FillRectilinear`, else
  `ipSupportBase` — a `FillSupportBase : FillRectilinear` filler) plus the
  per-pattern angles in `support_interface_angle()` (snug −45°, interlaced
  ±45°, grid = `base_angle`, auto/concentric = `interface_angle`). The port's
  interface generator is a single scan-line path with a universal 90°-per-layer
  alternation; packet 260 declares the enum with-gap. Building the dispatch
  (concentric/grid/interlaced generators + angle semantics) is Tier B+ geometry
  work in the support modules — fog until a queue packet or the
  support-interface closure packets pick it up.
- **Contact-loop interface generation (`support_interface_loop_pattern`).**
  Surfaced by ticket 18's authoring: canonical's `LoopInterfaceProcessor`
  (`n_contact_loops = value ? 1 : 0` in `generate_support_toolpaths`,
  `SupportMaterial::has_contact_loops`) prints the top contact layer of
  supports as concentric loops; the port has no contact-loop generator at all
  (packet 260 declares the coBool with-gap, default false). Wiring it is new
  geometry (a loop-filling pass over the interface plan regions) — queue-sized,
  Tier B+; fog until picked up.
- **Traditional-family raft handling is absent.** Surfaced by ticket 19's
  authoring: `traditional-support-planner` declares no raft keys and emits no
  `RaftPlan`, and `traditional-support` has no raft handling — raft is
  tree-family-only in this port (canonical supports raft for both families).
  Packet 261 declares the raft keys in `tree-support-planner.toml` and pins
  the traditional omission (AC-N2). Whether the traditional family gains raft
  handling — and where the keys would be declared if it does — is a port-state
  question for the raft geometry work (draft packet 240) and for P13
  (`raft_first_layer_expansion`); fog until one of them picks it up.
- **Gyroid solid-fill density semantics.** Surfaced by ticket 17's authoring:
  gyroid's ADR-0027 opt-in solid emission (top/bottom roles when the user
  sets `*_fill_holder = "gyroid-infill"`) rides the module's single
  `self.density` read from `sparse_infill_density` — a pre-existing divergence
  (gyroid solid at sparse density, e.g. 20% at defaults). Packet 264 declares
  the P10 density keys in `rectilinear-infill.toml` only and pins the gyroid
  omission (AC-N2). Whether gyroid's solid roles should consume
  `top_surface_density` / `bottom_surface_density` (per-role density in
  `emit_polys`) — and whether that changes the ADR-0027 opt-in contract — is a
  port-state question for a future gyroid-solid packet; fog until picked up.
- **Extra-internal-solid-fill machinery (`infill_only_where_needed`).** Surfaced
  by ticket 17's authoring: canonical `group_fills` produces an extra internal
  solid fill when internal voids exist and no `stInternalSolid` fill absorbed
  them, reading `top_surface_pattern` (monotonic/monotonicline → that pattern,
  else rectilinear) at fixed density 100. The port has no such pass (no
  `infill_only_where_needed` / `infill_every_layers` machinery). Whether the
  port gains the pass — and where — is queue-sized; fog until a packet picks it
  up.

- **Support ironing irons the wrong subject.** Surfaced by ticket 22's
  authoring: canonical irons the support **top contact (interface) layer**'s
  polygons — `generate_support_toolpaths` captures
  `top_contact_layer.polygons_to_extrude()` inside the `top_interfaces` arm and
  fills them at `erIroning`. The port's `support-surface-ironing` module runs at
  `Layer::SupportPostProcess`, and `LayerModule::run_support_postprocess`
  (`crates/slicer-sdk/src/traits.rs`) hands it only `&[SliceRegionView]` — there
  is no support-interface geometry at that seam to select — so it scan-fills
  every slice-region polygon it receives and pushes the result as support paths.
  Packet 267 records this (`DIV-267-A`/`DIV-267-B`) rather than changing it: it
  moves the gate's *key*, not the gate's *subject*, and rewriting the subject
  would change output for everyone already using the feature under a packet
  whose claim is default-path identity. Closing it means carrying support
  contact/interface polygons across the WIT boundary to a `SupportPostProcess`
  module — an IR field plus a WIT accessor, so queue-sized geometry/contract
  work. Note this makes P15 a **key-parity** packet only; nobody should read the
  queue's `support_ironing` coverage as geometry parity. Fog until a packet
  picks it up; the neighbouring support-interface work
  (packet `260b-support-interface-fill-claim-holders`) is the natural host, and
  the returned-to-queue `support_ironing_pattern` claim seam should be scoped
  with it.
- **Exact canonical relative ironing-angle parity.** The P14 packet's top module
   has no solid-infill direction or rotation-template metadata in `SliceRegionView`,
   so it uses a deterministic zero-degree base plus a layer-index turn. Whether the
   IR gains the canonical base direction, and which other ironing consumers use it,
   is future IR/geometry work; fog until a packet picks it up.

## Out of scope

- **SLA printing** — the whole `## SLA Printing` section (Support/Material/Pad/
  Display/Exposure/Hollowing/Faded-layers). A different pipeline entirely, not an FFF feature
  gap. Ruled out at charting time by the effort's scope decision; returns only
  if the destination is redrawn.
- **42 FFF keys ruled out by class** in
  [03](issues/03-nonapplicable-keys-triage.md) — print-host / preset management
  (17), non-physical filament metadata (9), Bambu-proprietary hardware (8),
  pellet-extruder hardware (2), plater / GUI state (6). Per-key list and class
  assignment in [`03-asset-scoped-gap.md`](issues/03-asset-scoped-gap.md).
  *Physical* filament keys and auto-set flags were explicitly kept in scope.
- **11 more keys ruled out by ticket 04's adversarial reviews** (user
  ruling, dead-in-canonical / preset-management classes) —
  dead-in-canonical (OrcaSlicer never reads them in the pipeline):
  `enable_timelapse` (superseded by `timelapse_type`), `allow_mix_temp`,
  `wiping_volumes_extruders`, `tree_support_with_infill` (obsolete in
  canonical's IGNORE set), `first_layer_sequence_choice` /
  `other_layers_sequence_choice` (dead alternate spellings),
  `support_chamber_temp_control` (GUI-only); preset-management (matching 03's
  class): `printer_technology`, `printer_variant`, `flush_volumes_vector`,
  `default_bed_type`. Per-key rows in
  [`04-asset-tier-assignment.md`](issues/04-asset-tier-assignment.md).
- **Retiring the hand-maintained ❌ column of `docs/ORCA_CONFIG_REFERENCE.md`**
  (07 ruling) — replacing it with generated presence flags + a `--check` gate
  is tooling hygiene, not a queue prerequisite: the queue never reads the
  column (ticket 01's asset and the map Notes neutralise its 66-key error).
  Returns only if the destination is redrawn. The standardisation workstream
  (99–107) makes the vocabulary converge meanwhile, shrinking the column's
  remaining drift surface.
- **Standardising the 34 Pinch-specific keys or the `raft_layers` 1→3 split**
  (07 ruling) — the Pinch-specific keys have no Orca counterpart, and the raft
  split is a strict superset of Orca's single count; neither is a gap or a
  rename. The raft divergence is recorded in
  [`03-asset-scoped-gap.md`](issues/03-asset-scoped-gap.md)'s 07 update.
