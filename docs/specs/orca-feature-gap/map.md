# Map: Close the OrcaSlicer FFF feature gap

Label: `wayfinder:map`

## Destination

A queue of **fully authored, preflighted spec packets** under `docs/spec_packets/` that
together cover every FFF (non-SLA) OrcaSlicer config feature Pinch 'n Print is
still missing — each packet complete with `packet.spec.md`, `requirements.md`,
`design.md`, `implementation-plan.md`, and passing `/spec-review --preflight`.
Packets are ordered **cheapest-first** (smallest diff before new geometry).

The map is done when every in-scope feature is either covered by an authored
packet or has been consciously ruled out of scope. Implementation (`/swarm`) runs
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
  `wipe_tower_speed` → `wipe_tower_max_purge_speed` — before any rename work
  may treat it as settled. It **gates the queue by owner** — each
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
  — packet `docs/spec_packets/253-part-cooling-fan-scale-and-cooling-keys/`
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
  — packet `docs/spec_packets/254-prime-tower-keys-wipe-tower/` authored
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
  — packet `docs/spec_packets/255-wipe-tower-geometry-keys/` authored
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
- [108 — Adjudicate `wipe_tower_speed` → `wipe_tower_max_purge_speed`](issues/108-adjudicate-wipe-tower-speed-alias.md) is
  still open (filed by ticket 10's authoring); the rename workstream's
  remaining members are 105–107, all unblocked (105 gates 107).
- [11 — Author packet P04 — Printer / Machine / Print volume — wipe-tower](issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md)
  — packet `docs/spec_packets/256-wipe-tower-bed-exclude-area/` authored
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
  — packet `docs/spec_packets/257-brim-type-and-brim-keys/` authored
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
  — packet `docs/spec_packets/258-skirt-type-and-draft-shield-keys/` authored
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
  — packet `docs/spec_packets/259-fuzzy-skin-keys/` authored (`draft`),
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
  — packet `docs/spec_packets/260-support-interface-keys/` authored (`draft`), preflight
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
  — packet `docs/spec_packets/261-raft-keys/` authored (`draft`), preflight **PASS**.
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
  — packet `docs/spec_packets/262-infill-pattern-keys/` authored (`draft`), preflight
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
  — packet `docs/spec_packets/263-infill-pattern-specific-keys/` authored (`draft`),
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
  — packet `docs/spec_packets/264-top-bottom-surface-keys/` authored (`draft`), preflight
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
