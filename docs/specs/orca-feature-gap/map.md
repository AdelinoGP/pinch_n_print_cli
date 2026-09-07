# Map: Close the OrcaSlicer FFF feature gap

Label: `wayfinder:map`

## Destination

Every FFF (non-SLA) OrcaSlicer *feature* Pinch 'n Print is still missing is
**closed** — each one either landed in the tree or consciously ruled out of
scope. Work is ordered **cheapest-first** (smallest diff before new geometry).

How a feature closes depends on its size, per the **Packets are for complex
implementation only** rule in Notes: small config-key work is **implemented
directly in its ticket's session**; only work needing new geometry, a new
IR/WIT field, a new module, or more than one session gets a **fully authored,
preflighted spec packet** under `docs/spec_packets/` (`packet.spec.md`,
`requirements.md`, `design.md`, `implementation-plan.md`, passing
`/spec-review --preflight`).

The config keys are the *inventory* of the gap, not the deliverable. A key is
"covered" only when the behaviour OrcaSlicer attaches to it exists in this
tree and the key drives it. A packet that declares keys in a manifest, pads
them into the CONFIG_BLOCK, and records the behaviour as a "gap" covers
nothing — see **Authoring rules** in Notes.

The map is done when every in-scope feature is either landed in the tree,
carried by an authored packet, or consciously ruled out of scope. Packet
implementation (`/swarm`) runs off-map, after; direct implementation does not.

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
  **Instance resolved by ticket 114 (2026-09-05):** the `ResolvedConfig`
  default moved to canonical `100.0` and the module-side
  `value / BASE_SPEED` factor mechanics were retired — sparse paths carry
  factor 1.0 and the host `FeedrateConfig` owns the role speed (the same
  shared raw key was previously multiplied twice, e.g. with gyroid/lightning
  as the fill holder a configured 200 emitted 800 mm/s before and 200 after).
  The hazard itself stands for the other plain-typed fields. **Consequence: every "default
  aligned to canonical" claim in tickets 99–107
  and packets 253–266 that was verified by reading a manifest is unverified for
  this class of key.** Not enumerated. Before asserting a default matches, check
  whether the key has a `ResolvedConfig` field and compare *that*
  (2026-09-01 grilling, Q11).
- **The scoped target is 410 queue keys** (03's 415 minus 04's 11 rulings plus
  07's 2 reclassified ironing keys plus 99's 2 fan-scale keys — minus ticket
  12's dead-in-canonical `brim_ears` ruling: **407**; the 406→407 step is
  ticket 105's re-adjudication of `resolution` out of the rename pool into the
  gap set; the 407→410 step is ticket 46's three source-missing
  `*_filament_id` siblings; per-key tier table in
  [`04-asset-tier-assignment.md`](issues/04-asset-tier-assignment.md), packet
  list in [`05-asset-packet-list.md`](issues/05-asset-packet-list.md). Size
  packets off those, never off the reference's ❌ column.
- **Execution override:** this map deliberately carries execution — packet
  *authoring* happens inside the map, not after it, and under the rule below so
  does the *implementation* of small config-key work. Only packeted
  implementation runs off-map.
- **Packets are for complex implementation only (user ruling, 2026-09-03).**
  A spec packet is ceremony worth paying for when the work is genuinely
  complex; it is dead weight on a key that needs declaring and wiring. So:
  - **Implement directly, in the ticket's own session** — no packet number, no
    `/spec-packet-generator`, no `/spec-review --preflight` — when the whole
    remaining work is declaring config keys in a manifest or `ResolvedConfig`
    and wiring them to a decision point that either already exists or is small
    enough to build in that same session. The ticket resolves with the commit;
    its answer names the keys, the decision points they now drive, and the
    tests that prove it.
  - **Author a packet** when the work needs new geometry, a new IR field, a WIT
    or schema change, a new module, a new claim seam, or is simply too large
    for one session. Roughly: the old Tier B/C work, not Tier A.
  - **The trigger is the code, not the key count.** A three-key ticket whose
    keys have no decision point anywhere in the tree can still need a packet
    (ticket 26 was sized "Tier A plumbing" and turned out to be three
    zero-occurrence keys); a twelve-key ticket that only declares can still go
    direct. Size the work at claim time, from the tree, not from the tier
    table.
  - **Re-derive the *owner* too, not just the size.** Ticket 27 found both P20
    keys assigned to `crates/slicer-gcode` on the strength of a neighbouring
    key's read site; `printer_structure` has no emitter behaviour whatever, and
    its real owner is `machine-gcode-emit`. Ticket 04's owner column was
    reviewed against canonical, not against *this tree's* module seams — check
    which module owns the decision point the key actually drives before
    declaring it anywhere.
  - **Authoring rules 1–6 below still bind direct work.** They govern what
    counts as *covering* a key, not what counts as a packet: no
    declaration-only keys, no `ORCA_CONFIG_PADDING` as evidence, the
    dead-in-canonical check per key, the PnP-way rule, and — standing in for
    the preflight gates — every key must end the session driving a
    behaviour-changing decision point with a test asserting that change at a
    non-default value.
  - **Retroactive on the open queue.** Tickets 26–98 are still titled "Author
    packet P<NN>"; read every one of them under this rule and re-size it at
    claim time. The title is a ledger fact that has rotted, not an
    instruction. Packets 253–274 already authored stay as they are.
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
       drifts); and a holder naming no loaded module must **fail validation**
       rather than yield a silently hollow part.
       `validate_startup_dag_with_configured_holders`
       (`crates/slicer-scheduler/src/validation.rs`) now checks the configured
       holder against the full loaded module set and emits a fatal structured
       error; a matching module that does not declare the selected claim has
       its own error variant.
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
- **The prime tower is a stub, and closing it is in scope at full canonical
  parity (user ruling, 2026-09-03, ticket 29).** This port's tower emits purge
  scan-lines only — no shell, brim, sparse infill, or idle-layer body — so every
  key that *selects* over that geometry (packet 255's ten with-gap keys, P02's
  framework / brim-width / infill-gap / flat-ironing keys, `wipe_tower_filament`)
  is blocked on building the body, not on config plumbing. Ticket 122 carries it.
  **Do not declare any of those keys anywhere in the meantime**, and do not
  re-derive the census — ticket 29 holds it, classified per body class.
  **Ticket 31 added `timelapse_type` to that set** (P24 dissolved): smooth mode
  is a tower-body selector — single-filament tower, a layer on every object
  layer, equalised depth, and an outer wall as the wipe target — so it lands
  with the body, not with config plumbing. Its one cross-module clause (the
  traditional-injection suppression in `DEV-168` (d)) lands there too.
- **A per-tool config axis already exists — do not build a second one.**
  Ticket 118 measured it: `tool_config:<tool_index>:<key>` on the raw config
  source yields a whole `ResolvedConfig` per tool
  (`resolve_per_tool_configs`, `crates/slicer-scheduler/src/config_resolution.rs`),
  covering every CLI-bound field plus module-manifest keys, at precedence
  `global < per_object < per_paint_semantic < per_tool`. Before treating any key
  as "blocked on there being no per-tool model", read
  [118's asset](issues/118-asset-per-tool-config-inventory.md) — the blockers are
  ingest, `overlay_resolved`'s 29-of-83 narrowing, and the paint-chain-only tool
  identity, not the absence of a mechanism. Two traps it corrects: the
  `@filament` / `@printer` scope markers are **GUI preset-routing labels with no
  runtime semantics** (`filament_diameter` is `@filament` *and* a scalar), and
  `tool-count()` **is** callable from a `PostPass` module — it returns 1 only
  because no core module declares `filament_density`, its sole source of truth.
- **Sequential printing (`print_sequence == ByObject`) is a missing *feature*,
  and its keys are scattered across three packets that cannot close alone.**
  Ticket 32 found `nozzle_height`'s only slicing-pipeline decision points inside
  canonical's `Print::sequential_print_clearance_valid` — which also reads all
  three `extruder_clearance_*` keys (P79 / ticket 86) and guards the mode
  `print_sequence` selects (P69 / ticket 76). This port has no sequential mode,
  no clearance model, and no per-object skirt grouping.
  [124 — Author packet — sequential printing (print-by-object) and toolhead
  clearance validation](issues/124-author-packet-sequential-printing-and-toolhead-clearance.md)
  owns the feature; tickets 76 and 86 carry a note to fold their keys in when
  claimed. **Its first act is a scope ruling from the human** — validation-only
  (which closes every key) versus real object-by-object emission (which reorders
  the layer loop) — on the ticket-29 precedent. Do not declare any of these keys
  in the meantime.
- **The queue is only as complete as its source, and that has not been checked.**
  Ticket 30 found two keys canonical's flush path reads
  (`flush_multiplier_fast`, `prime_volume_mode`) that appear **nowhere** in
  `docs/ORCA_CONFIG_REFERENCE.md`, and so nowhere in the queue. Ticket 01 audited
  the reference's ✅/❌ column, not its row set. [123 — Audit the gap source's key
  set for completeness against upstream](issues/123-audit-gap-source-key-set-completeness.md)
  measures it. Until it resolves, treat "the queue is closed" as weaker than "the
  destination is reached".
- **The canonical oracle is `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`
  — and only that checkout (user ruling, 2026-09-03, ticket 33).** Two other
  OrcaSlicer trees sit beside this repo and neither is usable for a
  dead-in-canonical check under Authoring rule 3:
  - `D:\slicerProject\Orca(pnp_gui)` is the **GUI-only PnP fork**. Its HEAD is
    `pnp B7/F13: FFF pipeline removal — M2 CLOSE (native-slicing rip-out
    complete)`, and **73 of its 215 `libslic3r` `.cpp` files are gone** — all of
    `Fill/`, all of `Arachne/`, `GCode.cpp` and `GCode/`, `Brim.cpp`,
    `Feature/Interlocking/`, `FuzzySkin.cpp`. Grepping it for a slicing key
    returns the *declaration only*, which reads exactly like a dead key. Ticket
    33 hit this: `calib_flowrate_topinfill_special_order` shows zero read sites
    there and two live ones in `Fill/FillBase.cpp` and `Fill/FillPlanePath.cpp`
    in the real oracle. It is also older (`02.06.00.51` vs `02.08.01.55`).
  - `pinch_n_print_cli_2/OrcaSlicerDocumented` is content-identical to the
    canonical one at the time of writing, but is not the designated copy.
  **Never rule a key dead against a checkout you have not confirmed still has
  the pipeline file the key would be read in.** Re-derive the path at point of
  use; do not trust this note's spelling of it if the tree has moved.
- **`order_lock` is a geometry contract, not just an ordering flag — and its shipped
  implementation is narrower than its own ADR.** Surfaced by ticket 33. ADR-0062 says the
  host remaps local tags to global tags **"at every output boundary"** and enforces the
  invariant "at every mutation point"; the shipped
  `remap_infill_order_locks_from` / `next_global_infill_tag` / `validate_infill_order_locks`
  (`crates/slicer-runtime/src/layer_executor.rs`) walk `InfillRegion::sparse_infill` **only**,
  so no lock emitted on `solid_infill`, `ironing`, or `internal_bridge_infill` survives.
  Packet 275 closes that as ADR conformance, not as an amendment — do not file an
  `ADR-AMENDED` deviation for it. ADR-0063 additionally makes locked paths **self-clipping**:
  the producer guarantees the whole swept footprint is in its legal domain, the linker
  neither clips nor links them, and it differences that footprint out of untagged fill of
  the same region. Any packet that emits a lock takes on both obligations; asserting them is
  not optional.
- **A passing module test does not prove a percent key reaches the guest in the
  right unit.** Ticket 34 measured it on `bridge_density`: manifest bounds check a
  bare number as a **percent** (`is_numeric_field_type` includes `percent` /
  `float_or_percent`, and `check_value` bounds the raw number), while
  `ConfigView::get_abs_value` reads a bare `Float` as a **fraction**. Canonical's
  own spelling `100.0` therefore passes `[10, 125]` and reaches the guest as
  density 100. `1.0` is rejected as below min. `100` (an `Int`) is unhandled and
  falls back. Only the percent **string** is correct, and strings skip bounds
  entirely. Module tests spell fractions, so they never see it.
  [128](issues/128-percent-key-numeric-spelling-units-mismatch.md) settles the
  unit and where the coercion lands; until then, do not read "the key is live" as
  "a user's profile value drives it".
- **A module cannot read a key its own manifest does not declare.**
  `ConfigView::from_declared` (`crates/slicer-ir/src/slice_ir.rs`) whitelists the
  raw source by the module's schema keys, so an undeclared key is filtered out and
  the guest's `unwrap_or` fallback always wins — silently, and a comment saying
  "profiles supply it" reads exactly like a working key. Found in `wave-overhangs`
  for `thick_bridges` (ticket 34). When checking whether a key is live, check the
  owner's manifest declares it, not just that the code reads it.
- **A queue ticket's keys may not belong in one packet — or in a new packet at
  all.** Ticket 35 re-derived P28's three owners and got three different seams, none
  of them ticket 04's `infill modules`. Two of the keys were *operators on decisions
  another already-authored packet was building*, so they were **folded into those
  packets** (262a, 262b) rather than carried as a fourth; the third was implemented
  directly. Before authoring, check whether an existing draft packet already owns the
  decision the key modifies — `ls docs/spec_packets/` and read the neighbouring
  packet's Goal. A fold is cheaper than a packet and keeps the decision in one place;
  packets 253–266 are being re-authored anyway, so amending one costs little.
- **This port has no internal-solid fill *domain*, and the object's rotation does
  not survive loading.** Two structural absences measured by ticket 35 that will
  bite any packet touching solid fill or fill angles:
  - `SlicedRegion::internal_solid_fill` is a **marker** (`top_solid_fill −
    top_solid_seed`), read by `arachne-perimeters` for the exposed top and by
    internal-bridge detection for what is not sparse. **Nothing fills it.** The
    `InternalSolidInfill` role comes from a **per-region** `top_shell_index` /
    `bottom_shell_index` ≥ 1 (`solid_fill_role`,
    `modules/core-modules/rectilinear-infill/src/lib.rs`). So (a) a polygon
    reclassified from sparse to solid has no dedicated vector to land in, and (b)
    canonical's per-polygon pattern overrides cannot be expressed as a second claim
    holder — the claim seam is per region. Do not assume `internal_solid_fill` is a
    fill domain because its name reads like one.
  - `ObjectMesh.transform` is `identity_transform()` for **every** loaded object:
    `resolve_object` (`crates/slicer-model-io/src/loader.rs`) composes the 3MF build
    item's transform, bakes it into the vertices, and discards it. Canonical's
    `object->trafo()` has no counterpart to read, so any key deriving from object
    orientation needs a model-io change first, not a config declaration.
- **The CONFIG_BLOCK is parsed by OrcaSlicer, and it silently corrects what it
  cannot parse — so a wrong *spelling* is invisible, not loud.** Measured by
  ticket 112. The G-code viewer calls canonical `ConfigBase::load_from_gcode_file`
  (`Config.cpp`) with `ForwardCompatibilitySubstitutionRule::EnableSilent`, and
  under that rule `ConfigBase::set_deserialize_raw`'s fallback rescues any value
  a `coBool` or `coEnum` rejects: bools go through
  `ConfigHelpers::enum_looks_like_true_value` (which matches only `"enabled"` /
  `"on"`), enums are reset to `optdef->default_value`. Nothing is thrown, nothing
  is logged, and the pair still counts toward the ≥80 floor. Consequences this
  port is living with today: `ConfigOptionBool::deserialize` accepts only `"1"` /
  `"0"`, so **every word-form `true` emitted by `emit_config_kv`
  (`crates/slicer-gcode/src/serialize.rs`) reads back in OrcaSlicer as `false`**;
  and `wall_generator`, emitted as `format!("{:?}")`, resets to canonical's
  default because the keywords are lowercase. `SupportType::as_canonical_str`
  (`crates/slicer-ir/src/slice_ir.rs`) is the same bug already fixed for one enum.
  **Any packet that adds a CONFIG_BLOCK key owes a canonical *value* spelling, not
  just a canonical key name** — and note that canonical's `key_value_pairs`
  counter increments only for keys it *accepted*, so an unrecognised key buys no
  margin. Ticket 132 carries the fix; do not spot-fix one key ahead of it.
- **A canonical `min` / `max` is a GUI hint, not a validation rule — never cite
  one as evidence that canonical rejects a value.** Measured by ticket 113:
  `ConfigBase::set_deserialize` / `set_deserialize_raw` (`Config.cpp`) never
  consult `def->min` / `def->max`, the speed check in `Print::validate`
  (`Print.cpp`) is commented out ("Orca: disable the speed check for now as we
  don't cap the speed"), and the only consumer outside `PrintConfig.cpp` is the
  GUI spinner in `Field.cpp`. OrcaSlicer will load a negative speed from a file
  or CLI without complaint. This port **does** enforce bounds, so adopting a
  canonical range is a deliberate divergence to be recorded as one, not parity.
  Corollary from the same measurement: **no canonical speed key declares a
  `max`** — a `max` on a speed in a module manifest is a PnP invention, and
  twelve such rows were retired by ticket 113 ([133](issues/133-retire-invented-speed-maxima-on-module-owned-speeds.md)
  carries three more on module-owned speeds).
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
  `precise_outer_wall` held at `false` as **DEV-166** because default-on
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
  parity is recorded as a bounded divergence (`DIV-266-B`): the base is the
  shared `infill_direction` input, exact for rectilinear-filled regions and off
  by `CORRECTION_ANGLE_DEG` for gyroid-filled ones; the earlier layer-index
  fallback was withdrawn (`DIV-266-A`).
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
- [24 — Author packet P17 — Quality / Seam — seam-placer](issues/24-author-packet-p17-quality-seam-seam-placer.md)
  — user chose direct closure over packet authoring for the single
  `staggered_inner_seams` key. The seam-placer manifest and
  `SeamPlacer::from_config` now expose the canonical `false` default, and
  `run_wall_postprocess` shifts associated inner closed loops forward while
  preserving the outer seam, wall metadata, and unrelated contours. Focused
  geometry regressions cover interpolation, width clamping, wraparound,
  closure metadata, winding, and disjoint/nested wall ownership. Canonical
  candidate-local angle metadata is unavailable in the current IR, so the
  implementation records a deterministic outer-geometry approximation.
- [25 — Author packet P18 — Printer / Machine / Power / recovery — emitter](issues/25-author-packet-p18-printer-machine-power-recovery-emitter.md)
  — packet `docs/spec_packets/267-printer-machine-power-recovery-emitter/`
  authored as `draft`, preflight **PASS**. Re-derivation split the four scoped
  keys three ways: **`disable_m73` is Tier A plumbing** — the decision point is
  already live (`DefaultGCodeEmitter::emit_gcode`'s `if !self.resolved_config.disable_m73`
  gate around `crate::m73::inject_m73`, proven end-to-end by
  `crates/pnp-cli/tests/m73_progress_tdd.rs`); the packet declares it in
  `machine-gcode-emit.toml`, closing ticket 04's ResolvedConfig-only contract
  violation. **`emit_machine_limits_to_gcode` and `enable_power_loss_recovery`
  are Tier B** — both true zero-occurrence gaps; the packet builds the canonical
  machine envelope (`GCode::print_machine_envelope`) as the PnP scalar subset
  (M203 from `machine_max_speed_x/y/z/e` with RRF × 60, M204 P/T from
  `machine_max_acceleration_extruding`/`machine_max_acceleration_travel` with
  Marlin-legacy T = extruding, M205 from `machine_max_jerk_x/y/z/e` with RRF
  M566 × 60; flavor-gated to Marlin/Marlin2/RepRapFirmware; prepended ahead of
  `machine_start_gcode` via a postpass PrintStart rule change) and the recovery
  emission (`GCodeWriter::enable_power_loss_recovery`: `enable` → M413 S1 at the
  second emitted layer + M413 S0 at the end; `disable` → M413 S0 at the second
  emitted layer; `printer_configuration` → nothing; Marlin2 only). Missing
  envelope groups (M201, M204 R, M205 J, M593) and the Bambu M1003 form are
  recorded as divergences — the P47 fields and a Bambu flavor do not exist and
  are not invented. **`silent_mode` is returned to the queue as unimplemented**:
  canonical reads stride-2 normal/stealth `machine_max_*` pairs and PnP's scalar
  `Option<f32>` fields have no variant dimension; the missing per-variant
  machine-limit model is named in the tier table and filed as ticket 117. P18
  is now **mixed A/B, 3 keys**; the 04/05 assets are updated. No user rulings,
  no deviation rows, no `ORCA_CONFIG_PADDING` edit (AC-N3).
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
  Ticket 110 adds the holder-resolution safety net.

- [109 — Rename `ironing_speed` → `support_ironing_speed` and decide `SPEED_KEYS` membership](issues/109-rename-support-ironing-speed-and-decide-speed-keys-membership.md)
  — renamed the support module's PnP-specific normalization key, aligned its
  absent-key fallback to `30.0`, and kept it out of `SPEED_KEYS`: canonical has
  one global `ironing_speed`, while the support module owns its `speed_factor`.
- [110 — An unmatched `*_fill_holder` must fail validation](issues/110-unmatched-fill-holder-must-fail-validation.md)
  — startup validation now rejects a configured holder that matches no loaded
  module, and separately rejects a matching module that does not declare the
  selected claim; diagnostics include deterministic candidate module IDs.

- [26 — Close P19 — Printer / Machine / Print volume — emitter](issues/26-author-packet-p19-printer-machine-print-volume-emitter.md)
  — **first ticket closed by direct implementation instead of a packet**, under
  the new "Packets are for complex implementation only" rule. The ticket's "3
  keys, Tier A plumbing" sizing was wrong: all three keys had zero read sites,
  and the tree had no build-volume validation at all. `printable_height` is now
  live — `ResolvedConfig::printable_height` → `validate_printable_height`
  (`crates/slicer-model-io/src/loader.rs`) → called per object from `run_slice`,
  rejecting with the stable `EXCEEDS_PRINTABLE_HEIGHT` code, and emitted from the
  resolved config so it shadows the frozen `ORCA_CONFIG_PADDING` literal without
  editing the padding table. Default **250.0 deviates from canonical's 100.0**
  (user ruling, **DEV-167**): canonical's value is a preset fallback, and adopting
  it with a new hard rejection would newly fail every model over 100 mm tall.
  `extruder_printable_area` / `extruder_printable_height` were **returned to the
  queue as unimplemented** — inert single-extruder, their only canonical
  behaviour paths are multi-extruder wipe-tower ones — re-tiered A → B and
  pointed at ticket 28. Required promoting `slicer-model-io` to a real dependency
  of `slicer-runtime`. Verified: 8 validator tests, 2 slice-path wiring tests,
  `slicer-gcode` 16/16 green (CONFIG_BLOCK byte-stable), clippy + check-literals
  clean.

- [27 — Close P20 — Printer / Machine / Printer identity — emitter](issues/27-author-packet-p20-printer-machine-printer-identity-emitter.md)
  — **both keys closed with no packet; the tier table's owner for one of them was
  wrong.** `printer_structure` has no emitter behaviour at all: its only non-GUI,
  non-Bambu canonical footprint is
  `need_insert_timelapse_gcode_for_traditional` in `GCode::process_layer`, so it
  is declared as an enum on **`machine-gcode-emit`** (which owns this port's
  `time_lapse_gcode` injection site) and now suppresses time-lapse injection on
  non-i3 single-tool prints, verified end-to-end through the real guest. Three
  divergences filed as **DEV-168**: the `undefine` default does not suppress
  (canonical would — same "don't silently drop existing users' output" ruling as
  ticket 26), the `!spiral_vase` clause is unwirable (no spiral mode in this
  port), and `is_multi_extruder` is approximated by "the print performs a
  toolchange" because no extruder-count key reaches a `PostPass` module.
  `printer_model` needed **no code change** — a user value already beats the
  `Generic PNP Printer` synthesis in `serialize_config_block`, with both arms
  already pinned by tests; canonical's other reads are Bambu/Elegoo vendor
  branches in ticket 03's out-of-scope class. Also fixed a harness defect that
  would have blocked any future enum key on this module: the schema sweep in
  `machine_start_end_gcode_emission_tdd.rs` seeded enums with an empty-string
  sentinel, which is not a legal enum value.

- [28 — Author packet P21 — Extruder / Nozzle / MMU Hardware — wipe-tower](issues/28-author-packet-p21-extruder-nozzle-mmu-hardware-wipe-tower.md)
  — **not authored: P21 is Tier B+ feature work blocked on the per-tool config
  model.** All five MMU keys pass rule 3 (live in canonical) but only on the
  Type2 path: in the BBS `WipeTower` the constructor's assignment block and both
  `toolchange_Unload` / `toolchange_Load` bodies are `#if 0`, delegated to
  `change_filament_gcode` — a hook this port already has on `machine-gcode-emit`,
  so the port is at **Type1-equivalent parity today**. The live reads are
  `WipeTower2::toolchange_Unload` / `toolchange_Load`, selected by
  `Print::wipe_tower_type` (non-BBL default `type2`), gated on
  `single_extruder_multi_material` and `enable_filament_ramming` — **neither key
  exists in this tree** (`single_extruder_multi_material` is an
  `ORCA_CONFIG_PADDING` row only, which rule 2 rejects as evidence). Every use
  site multiplies a P21 key by a **Tier D** per-filament ramming value
  (`filament_cooling_moves` = 0 alone disables the only use of
  `cooling_tube_length`), so a packet today would be 100% declaration-only —
  prohibited by rule 1. Re-filed as
  [119 — Author packet P21 (re-filed)](issues/119-author-packet-p21-mmu-hardware-wipe-tower-refiled.md),
  which also **adopts** `extruder_printable_area` / `extruder_printable_height`
  (ticket 26's returned keys — the same missing subsystem seen from the extruder
  side: `nozzle_diameter` is a scalar `f32` in `ResolvedConfig`, not canonical's
  per-extruder vector). Both blocked on the new
  [118 — Inventory the port's per-tool config mechanism](issues/118-inventory-per-tool-config-mechanism.md).
  Owner hazard confirmed again (ticket 27's): the feature is wipe-tower's, the
  **seam is not** — `wipe-tower.toml` is `PostPass::LayerFinalization` geometry,
  and choreography is writer output interleaved with tower moves.

- [29 — Author packet P22 — Multimaterial / Filament for Features — wipe-tower](issues/29-author-packet-p22-multimaterial-filament-for-features-wipe-tower.md)
  — **P22 dissolved: the key's subject does not exist, so the map ruled the
  subject in scope instead.** `wipe_tower_filament` (1-based, `0` = auto) never
  touches the purge; it forces which filament prints the tower's **finish
  extrusions** — sparse infill, wall, brim — through
  `ToolOrdering::insert_wipe_tower_extruder` and
  `WipeTower2::first_toolchange_to_nonsoluble_nonsupport`. This port's tower is
  **purge-only**: `WipeTowerModule::generate_purge_paths` emits travel + purge
  lines + a prime entity per `ToolChange`, both module paths skip a layer whose
  `tool_changes` is empty, and every path is stamped `tool_index = tc.to_tool`.
  Distinct failure from ticket 28's — the Tier D per-filament fog is **not**
  engaged, because the forced branch short-circuits `filament_soluble` via the
  `set_extruder` masking. The ticket then took the **census once for every
  dependent packet** (body classes shell / infill / brim / idle layer, per-key
  table in the ticket), pinned canonical's body to `WipeTower2::finish_layer` +
  `plan_tower`'s top-down depth propagation, and established that **the port's
  seam can already carry it**: `run_finalization` receives all layers,
  `push_entity_with_priority` needs no toolchange anchor and takes an explicit
  `tool_index`, and no WIT/schema/IR change is implied — the single constraint
  being that intra-layer tool changes come only from `layer.tool_changes`
  (`crates/slicer-gcode/src/emit.rs`). **User ruling: the port grows a real tower
  body at full canonical parity**; purge-only is rejected as a design and no
  census key goes out of scope. All of it, including `wipe_tower_filament`, is
  carried by [122 — Author packet — prime tower body parity](issues/122-author-packet-prime-tower-body-parity.md).

- [30 — Author packet P23 — Multimaterial / Flush options — wipe-tower](issues/30-author-packet-p23-multimaterial-flush-options-wipe-tower.md)
  — **closed by direct implementation, no packet.** The "Tier B new logic" sizing
  did not survive the tree: `WipeTower::generate_purge_paths` already converted a
  purge volume into the scan-line box depth and the prime entity's length — the
  same arithmetic as canonical's `get_wipe_depth` — and ignored which tool change
  it served. `flush_volumes_matrix` (flat row-major `N*N`, indexed
  `[from_tool][to_tool]`, per canonical `WipeTower2::extract_wipe_volumes`) and
  `flush_multiplier` are now declared on `wipe-tower` and consumed by the new
  `WipeTower::purge_volume_for`. Unset, the matrix falls back to the flat
  `prime_volume`, so no existing print changes. Four divergences in `DEV-169` —
  the fallback (canonical zeroes the matrix unless `purge_in_prime_tower &&
  single_extruder_multi_material`, **neither key exists here**; both are P02), the
  multiplier not scaling the fallback, one scalar multiplier where canonical has a
  per-extruder `coFloats` (blocked on ticket 118), and no
  `filament_minimal_purge_on_wipe_tower` clamp (Tier D). 7 new tests.

- [31 — Author packet P24 — Others / Special mode — wipe-tower](issues/31-author-packet-p24-others-special-mode-wipe-tower.md)
  — **P24 dissolved; `timelapse_type` folded into ticket 122.** Third ticket in a
  row (28, 29, 31) whose keys turn out to be prime-tower-body work. Smooth mode
  (`tlSmooth`) is a body selector, not a timelapse toggle: it makes the tower
  exist for a single filament (`Print::has_wipe_tower` via
  `Print::enable_timelapse_print`), puts a tower layer on every object layer
  (`ToolOrdering`), floors and equalises every layer's depth to layer 0's
  (`WipeTower::plan_tower`), and makes `only_generate_out_wall` the per-layer
  deliverable (`WipeTower::generate`) — the wall *is* the surface the nozzle wipes
  on before each snapshot. All four are ticket 122's. The fifth read is the
  tempting one and was **deliberately not wired**: canonical suppresses the
  traditional injection when a tower has smooth timelapse enabled
  (`GCode::process_layer`'s outer `(!m_wipe_tower ||
  !m_wipe_tower->enable_timelapse_print())`), and doing that here — where no
  smooth wall exists and `machine-gcode-emit` cannot even observe that the tower
  module ran — would leave a smooth print with *no* timelapse mechanism at all.
  Recorded as clause (d) of `DEV-168` with ticket 122 as owner; no code change,
  no key declared, no packet number taken.

- [32 — Author packet P25 — Extruder / Nozzle / Nozzle — skirt-brim](issues/32-author-packet-p25-extruder-nozzle-nozzle-skirt-brim.md)
  — **P25 dissolved; `nozzle_height` folded into ticket 124.** Two findings. First,
  the owner is wrong: canonical's only non-GUI read outside
  `Print::is_all_objects_are_short` sits in a function *called*
  `Print::object_skirt_offset`, but that value never reaches skirt generation —
  neither `Print::_make_skirt` nor `_make_brim` calls it. Its `libslic3r/` caller
  is `Print::sequential_print_clearance_valid`; the rest are the GUI arranger. It
  is a clearance computation that accounts for per-object skirts, not a skirt
  computation. The ticket-27 lesson again. Second, both decision points
  `nozzle_height` drives sit inside a validator guarding
  `print_sequence == ByObject`, and this port has **neither**: `nozzle_height`,
  `extruder_clearance_radius`, `extruder_clearance_height_to_rod`, `skirt_type`
  and `draft_shield` have zero occurrences under `crates/`/`modules/`/`xtask/`,
  and `print_sequence` appears only as an `ORCA_CONFIG_PADDING` row (rule 2: not
  evidence). The key passes rule 3 — it is live in `libslic3r/` — so it stays in
  the queue, unimplemented. No key declared, no packet number taken, no code
  change, no deviation.

- [118 — Inventory the port's per-tool (filament / extruder) config mechanism](issues/118-inventory-per-tool-config-mechanism.md)
  — **A general per-tool mechanism already exists: `tool_config:<idx>:<key>`.**
  `resolve_per_tool_configs` (`crates/slicer-scheduler/src/config_resolution.rs`)
  yields a whole `ResolvedConfig` per tool, reaching all CLI-bound fields plus any
  module-manifest key, at precedence
  `global < per_object < per_paint_semantic < per_tool`. Per-stage inventory in
  [118's asset](issues/118-asset-per-tool-config-inventory.md). Four gaps, none of
  them the mechanism: nothing ingests an Orca `coFloats` vector *onto* that axis
  (and `extract_float_or_first` silently keeps element 0, so a two-filament 3MF
  loses filament 2's `filament_diameter`); `overlay_resolved` is a 29-of-83
  hand-written allowlist that silently drops the rest; the tool axis reaches
  geometry only via a `("material", ToolIndex(n))` paint chain; and no core module
  declares `filament_density`, so `tool-count()` is 1 everywhere. Three beliefs
  corrected — `@filament`/`@printer` are GUI preset labels with no runtime meaning,
  `tool-count` *is* reachable from a `PostPass` module (`host-services` is imported
  by `world gcode-postprocess-module`), and `nozzle_diameter` is an `extensions`
  scalar, not a `ResolvedConfig` field. Filed tickets 125 (the ruling) and 126 (the
  narrowing + a suspected precedence defect, read from code and not yet
  reproduced). No key declared, no code change.

- [33 — Author packet P26 — Calibration / Flow / Pressure advance calibration — infill modules](issues/33-author-packet-p26-calibration-flow-pressure-advance-calibration-infill-modules.md)
  — sizing rotted: `calib_flowrate_topinfill_special_order` is a **rider on a pattern
  family the port lacks**, not a declare-and-wire key. Packet 264 already ships
  `archimedean-chords-infill`; per user ruling the module stays with 264 and everything
  else went to **packet 275** (`docs/spec_packets/275-top-fill-order-and-calibration-order/`,
  draft, `PREFLIGHT PASS`): an SDK ordering kernel, the `order_lock` emission, the host
  widening, and `top_surface_fill_order` / `bottom_surface_fill_order` — two live keys
  absent from the gap source. Three corrections: the `Orca(pnp_gui)` checkout would have
  produced a **false dead-key ruling** (see the canonical-oracle Notes bullet); 3MF ingest
  is **not** a blocker (`parse_project_settings_json` ingests every key generically, no
  allowlist); and `order_lock` carries **geometry** semantics, not just ordering.

- [34 — Author packet P27 — Quality / Bridging — infill modules](issues/34-author-packet-p27-quality-bridging-infill-modules.md)
  — **closed by direct implementation, no packet.** `bridge_density`,
  `internal_bridge_density` and `thick_internal_bridges` were already live in
  `rectilinear-infill`; the gap was the second `claim:bridge-fill` holder,
  `wave-overhangs`, which read the **external** keys on internal bridges and read
  four bridge keys its manifest never declared — including
  `thick_bridges`, whose read could never fire because `ConfigView::from_declared`
  whitelists by the module's own schema. Five new tests, each a two-run comparison
  differing only in the key it names. Two findings filed rather than fixed:
  `gyroid-infill` fills bridges and solid surfaces at `sparse_infill_density`
  (ticket 127), and percent-typed keys are misread when spelled as a bare number
  (ticket 128).

- [35 — Author packet P28 — Strength / Advanced (Strength) — infill modules](issues/35-author-packet-p28-strength-advanced-strength-infill-modules.md)
  — **no new packet: two folds and one direct implementation.**
  `align_infill_direction_to_model` folded into packet **262a** (it adds the object
  rotation *after* the direction key and the rotate template that packet builds — and
  it first needs the rotation to survive loading at all);
  `detect_narrow_internal_solid_infill` folded into packet **262b** (it overrides the
  `internal_solid_infill_pattern` selection 262b builds, and must live inside the
  `claim:top-fill` holder because this port's claim seam is per region, not per
  polygon); `minimum_sparse_infill_area` **implemented in the ticket's session** in the
  host `PrePass::ShellClassification` pass, conservative against canonical because the
  port measures the sparse zone before the wall inset. Ticket 04's `infill modules`
  owner was wrong for two of the three.
  — **Follow-up (2026-09-04, user ruling):** the first landing rode
  `bottom_solid_fill` + a `bottom_shell_index` stamp; the same session then gave
  internal solid infill its own classification domain — the fill-stage partition
  now runs five-way precedence `bridge > bottom > top > internal > sparse`, the
  `claim:top-fill` holders emit the `internal_solid_fill` bucket as
  `InternalSolidInfill`, the linker boundary and `perimeter-region-view` gained
  `internal-solid-fill`, and the stamp (with its mixed-region divergence) is
  retired. Finding 2's "no internal-solid fill domain" is no longer true of the
  tree; 262b's DIV-8 seam statement survives (the claim seam is still per region,
  so the narrow split still lives inside the holder module).
- [111 — Convert the part-cooling fan scale to percent 0–100, and make `overhang_fan_speed` absolute](issues/111-convert-fan-scale-to-percent.md) — **confirm-and-amend on packet 253, which already owns the conversion; no code written.** Canonical declares all fan speeds percent 0–100, so the port's 0–255 scale is the divergence. Three rulings: percent→PWM converts **once per channel with three different formulas** (`set_fan` biases with 255.5, `set_additional_fan`/`set_exhaust_fan` truncate with 255.0), never one shared helper; `overhang_fan_speed` is **absolute**, gated by `overhang_fan_speed > base` (a `>` test that then replaces the base outright — not a `max` merge, not a fraction of `fan_max_speed`), so at the packet's own defaults the overhang branch does not even engage; and the 0–255→percent break is **accepted unmigrated** (user ruling, ticket-107 precedent) — >100 fails loudly at bounds check, 0–100 is silently reinterpreted. The sibling `slow_down_*` keys need no ticket: packet 253's AC-10/AC-11 already carry them. Corrected three contradictions in packet 253 (its `design.md` mandated a single shared converter against its own AC-2, preserved the percentage-of-max semantics the ruling rejected, and mis-stated the default overhang byte) and found one fixture whose premise inverts under absolute semantics rather than merely restating in percent. **Trap for any later fan work:** `fan_min_speed` was declaration-only — declared and schema-tested, never read by the module.
- [112 — Derive the CONFIG_BLOCK padding table from the resolved config](issues/112-derive-config-block-padding-from-resolved-config.md) — **decided, not executed; the premise inverted and the scope grew, so it filed a packet ticket instead.** The hardcoded padding table turned out to be the *correctly spelled* part; the resolved-config emission that shadows it is where values are silently corrupted (see the CONFIG_BLOCK Note above). Rulings: derive from **module manifest `[config.schema]` defaults** via `ConfigFieldEntry.default` (a string for every type, unlike the percent-only `parsed_default`) — and the map's "manifest default is dead" hazard **inverts** here, because a key with a `ResolvedConfig` field is already emitted and padding never fires for it; keep an explicit canonically-grounded floor list for the keys no module declares, pinned by a test that each entry is live in `PrintConfigDef` and declared by no module (that, not "nothing hardcoded", is the anti-drift property Q5 wanted); delete the canonical-unknown rows (`top_fill_pattern`, `support_material`, `outer_wall_direction`, `infill_first`, `extra_perimeters` — none of them a legacy alias, all silently dropped by canonical). Measured taxonomy of the 69 rows: **9 dead** (shadowed by `to_config_map`), **24 live and derivable**, **36 live with no in-tree owner**. **The ≥80 floor is not currently guaranteed:** canonical counts only *accepted* pairs, and the port's integration test counts *lines*, so it asserts the wrong quantity; the true accepted count is **unmeasured**. **Two ledger facts in the ticket body re-verified false:** `skirt_loops` / `skirt_distance` / `brim_width` were *not* realigned by Q14(a) (padding still reads 1/2/0 against manifests 6/3.0/8.0), and `support_raft_layers` is not in the table at all. Implementation handed to [132 — Author packet — the CONFIG_BLOCK is a contract with OrcaSlicer's reader](issues/132-author-packet-config-block-reader-contract.md).
- [113 — Add range validation to `FeedrateConfig`](issues/113-feedrate-config-range-validation.md) — **implemented directly; bounds now enforced for all 26 host speed keys.** The premise needed correcting first: canonical *declares* these bounds but never enforces them (see the GUI-hint Note above), so this is a deliberate divergence, ruled by the user as reject-the-slice. It was warranted — `read_speed` rejects nothing and `resolve_feedrate` applies no floor, so a configured `0` emitted `F0` and a negative emitted a negative feedrate. **14 of the 26 keys were already checked by accident** (a module manifest happened to declare a twin) and the other 12 by nothing at all; `ConfigBoundsIndex::from_modules` now seeds `slicer_ir::feedrate::speed_bounds()` so all 26 go through one seam with one error. Canonical's inclusive mins (`1`, `0` for the sentinel keys, `10` for `wipe_tower_max_purge_speed` — the bound ticket 108 called inexpressible) fit `NumericBounds` as-is, so the "`> 0` is inexpressible" problem dissolved. **Retired the invented `max = 300.0`** from 12 manifest rows. Three keys have no canonical counterpart (`thin_wall_speed`, `bottom_surface_speed`, `prime_tower_speed`). Six percent-form `ratio_over` keys are silently ignored by `read_speed` — handed to ticket 128, not fixed blind. Also found: `docs/15_config_keys_reference.md` was **already stale on HEAD** (`bridge_density` 120 vs 125), absorbed by this ticket's regen.
  - [36 — Author packet P29 — Quality / Line width — support-planner](issues/36-author-packet-p29-quality-line-width-support-planner.md) — **closed by direct implementation, no packet.** The tier table's `support-planner` owner was narrow: the key's decision points span the planner smoothing, both support renderers, and the host resolver. Both renderers read the key without declaring it — dead on the production path (`bind_module_config_view` whitelist), the ticket-34 shape — so the fix declares it in both manifests and aligns every site to the canonical auto chain (`support_material_flow` + `auto_extrusion_width`: value → `line_width` → nozzle; the `1.125×` factor is wall roles only). Retired the three misapplications of the wall auto to this key (renderer 0.45, planner 0.35, host 0-to-nozzle shortcut); default support width is now 0.4 everywhere, and the tree renderer's pitch test now pins the same Orca-measured 0.757 mm as the traditional one. No new deviation (canonical default is identically 0.0). Evidence: manifest guards, two geometry two-run comparisons, auto-chain unit tests at all four sites; contract 296/296, integration 345/345, e2e 144/144 green.
  - [37 — Author packet P30 — Support / Advanced (Support) — support-planner](issues/37-author-packet-p30-support-advanced-support-support-planner.md) — **closed by direct implementation, no packet**, after claim-time re-sizing. `bridge_no_support` now reaches `resolve_contact_params` from the typed `ResolvedConfig` field, and each `SlicedRegion::bridge_areas` reaches the existing bridge-removal decision point in `detect_support_contacts`; the producer regression compares the same bridge geometry with the flag off and on. `independent_support_layer_height`, `max_bridge_length`, and `support_base_pattern_spacing` were already covered by their existing packet work and needed no duplicate change. Q3's holder-only ruling removed the dead `support_base_pattern` enum declaration, object-metadata alias, planner field, and `traditional-base-pattern` capability label; the actual holder/module implementation is returned to [135](issues/135-author-packet-support-base-pattern-holder.md). Focused producer, core support-contact, model-I/O, and traditional-planner tests pass; workspace check, clippy, generated config docs, check-literals, and rebuilt guest freshness all pass.
  - [38 — Author packet P31 — Support / Support filament — support-planner](issues/38-author-packet-p31-support-support-filament-support-planner.md) — **closed without a new packet**: packet 172 already implemented `support_filament` / `support_interface_filament` parsing, 1-based-to-0-based rebasing, global runtime support/interface routing, 3MF metadata handling, and real-fixture G-code coverage. Claim-time grounding corrected the owner from support-planner to runtime entity assembly and the tier from new logic to already-live decision points; `TASK-210`/`TASK-211` are already recorded done.
  - [39 — Author packet P32 — Extruder / Nozzle / Extruder geometry / mapping — emitter](issues/39-author-packet-p32-extruder-nozzle-extruder-geometry-mapping-emitter.md) — **re-sized at claim time: not authorable now, one of seven keys already covered.** `extruder_colour` is **covered**: canonical's only pipeline read is the CONFIG_BLOCK alias to `filament_colour` (`GCode::append_full_config`); the port emits the directive in HEADER_BLOCK + CONFIG_BLOCK with the authored palette when supplied, pinned by `cube_4color_gcode_output_tdd`. The other six (`extruder_offset`, `extruder_type`, `master_extruder_id`, `physical_extruder_map`, `printer_extruder_id`, `printer_extruder_variant`) have **zero occurrences in the tree**, and their canonical decision points are per-extruder vectors or whole absent features (`GCode::point_to_gcode` offset subtraction + `WipeTowerIntegration::post_process_wipe_tower_moves` toolchange bridge moves; `ToolOrdering.cpp::build_filament_group_context` bowden/direct preferencing; the `FilamentGroup.cpp` grouping algorithm (`FilamentGroup::calc_group_by_kmedoids`); the physical T-map in `GCode::_do_export`'s reorder, placeholder seeds and `WipeTower`'s M104/M109 targets; `get_index_for_extruder_parameter` variant-array shape) — none of which exists here, so a packet would have to build the per-extruder config model itself, which is exactly what 125/126 rule on. **Re-filed as [136](issues/136-author-packet-p32-per-extruder-keys-refiled.md), blocked on 125** (which is blocked on 126) — the 28→119 pattern. Tier table + packet-list P32 rows annotate the re-file; no queue-count change (covered keys don't shrink the count, per P31's precedent).
  - [40 — Author packet P33 — Extruder / Nozzle / MMU Hardware — emitter](issues/40-author-packet-p33-extruder-nozzle-mmu-hardware-emitter.md) — **closed by direct implementation of one key; the other returned to the queue** (the ticket-22 shape). `grab_length` is **live**: the tier table's `crates/slicer-gcode (toolchange)` owner was wrong for this tree — the port computes the purge volume once, in `WipeTower::purge_volume_for` (`modules/core-modules/wipe-tower/src/lib.rs`, ticket 30's decision point), and the emitter only emits what the module produced. Declared on `wipe-tower.toml` (float, default 0, min 0 — canonical `coFloats` min 0 default `{0}`), and `purge_volume_for` now subtracts `grab_length × 2.4` (the `(diameter/2)^2*PI` cross-section both canonical read sites hardcode — `GCode.cpp` toolchange path, `Print.cpp` wipe-tower planning) clamped at 0, matching canonical's `std::max(0.f, wipe_volume - grab_purge_volume)`. 4 new tests (default identity, fallback + matrix reduction, zero clamp, emitted-geometry shrink); **DEV-170** records scalar-vs-per-extruder `coFloats` (the `flush_multiplier` precedent) and the fallback application. `start_end_points` is **returned to the queue as unimplemented**: canonical's only read site is `get_path_of_change_filament` (`GCode.cpp`), which computes the `travel_point_*` placeholders for `change_filament_gcode` from `start_end_points` + `bed_exclude_area` + object bounding boxes — and returns the safe default path when `bed_exclude_area.size() != 4`. `bed_exclude_area` is packet 256's scope (authored, **not implemented**), so wiring this key alone would be declaration-only (rule 1); the path computation also needs object bounding boxes at a seam reaching the postpass substitution (new `ResolvedConfig` fields or extensions + `travel_point_*` schema on `machine-gcode-emit`). Missing feature named in the tier table; re-file when 256 lands. P33 now covers 1 key. Gates: wipe-tower 36/36, `cube_4color_gcode_output_tdd` 9/9, clippy + check-literals + gen-config-docs clean, 46 guests rebuilt fresh.

  - [114 — `sparse_infill_speed`: align the `ResolvedConfig` default and re-base `speed_factor`](issues/114-sparse-infill-speed-resolved-default-and-speed-factor-base.md) — **resolved by direct implementation; the resolved default is now canonical 100 and the module-side factor base is retired, not re-based** (scope correction with measurement: the literal `value/100` reading double-counts, because `FeedrateConfig::from_raw_config` reads the same raw key the modules divide — gyroid/lightning with a configured 120 emitted 288 mm/s; only factor 1.0 emits the configured value). `ResolvedConfig.sparse_infill_speed` `50.0 → 100.0` (one value across manifests/ResolvedConfig/FeedrateConfig, and the CONFIG_BLOCK spelling stops being wrong-by-coincidence at 50); gyroid + lightning deleted `BASE_SPEED` and emit sparse paths at factor 1.0 (rectilinear's already-shipped host-owned contract); the lightning 1.6 (80/50) assertion re-pinned at 1.0; new pins: gyroid factor 1.0 at speed 200, slicer-ir default + `to_config_map` 100.0. **Step-4 proof: before/after default slice of the 20 mm box differs by exactly one line** (`; sparse_infill_speed = 50` → `= 100`), 686 sparse moves byte-identical at F6000. **Step-5 decision: retire `factor = module-key / private-constant` where the host reads the same key; keep ratio-of-two-keys factors (wave-overhangs self-cancels) and per-path modifiers.** The one same-shape survivor — `classic-perimeters`' outer/inner wall + gap-fill factors, whose own test pins 30/50 = 0.6 (a configured 30 emits 18 mm/s today) — filed as [134](issues/134-retire-classic-perimeters-speed-factor-bases.md). Gates: clippy + check-literals clean, slicer-ir + modules green, e2e 144/144, all 46 guests rebuilt. No deviation rows (default now matches canonical).

  - [41 — Author packet P34 — Extruder / Nozzle / Nozzle — emitter](issues/41-author-packet-p34-extruder-nozzle-nozzle-emitter.md) — **re-sized at claim time: not authorable now, all four keys re-filed, no packet, no code change** (the ticket-28/39 shape). All four keys zero-occurrence in the tree; canonical grounding puts every read site in `GCodeProcessor`, not slicing geometry — the HRC trio feeds the non-fatal `NOZZLE_HRC_CHECKER` post-export warning, and `nozzle_volume` feeds only Elegoo-`M6211` flush attribution. All four live in canonical (in scope); the tier table's emitter owner is wrong (no warning-list seam, no Elegoo seam, no per-tool vector ingestion in tree). **Re-filed as [137](issues/137-author-packet-p34-nozzle-keys-refiled.md), blocked on 125** (itself gated on 126); warning-vs-fatal divergence and Elegoo vendor-scope rulings ride the re-file. No queue-count change.

  - [42 — Author packet P35 — Extruder / Nozzle / Pressure advance — emitter](issues/42-author-packet-p35-extruder-nozzle-pressure-advance-emitter.md) — **closed by direct implementation of two keys; four returned to the queue** (the ticket-40/22 shape). `enable_pressure_advance` + `pressure_advance` are **live**: `ResolvedConfig` fields (canonical defaults, omitted from `to_config_map` — host-only emission control, P18/P96-AC-8 precedent) + `machine-gcode-emit.toml` declarations + host-emitter start/toolchange emission via the existing `GcodeFlavor::set_pressure_advance` helper (all five flavors; `pa < 0` guard; per-tool via the `tool_config:<idx>:` axis, vector ingest rides 125); 9 new tests, default path byte-identical, deviations stay 26, 46 guests rebuilt. The four `adaptive_*` keys are **unimplemented**: they need the AdaptivePAProcessor-style per-feature prediction (interpolators + `process_layer` post-pass + bridge/overhang arms + model validation) — missing feature named in the tier table, no packet number taken. P35 now covers 2 keys. Gates: workspace clippy + check-literals clean, `slicer-gcode` 17 binaries + `machine-gcode-emit` green, start/end + flavor CONFIG_BLOCK integration green. Pre-existing red noted: `check-deviations --check` (doc 07) fails on the clean tree too.

  - [43 — Author packet P36 — Extruder / Nozzle / Retraction (1/2) — emitter](issues/43-author-packet-p36-extruder-nozzle-retraction-emitter.md) — **authored as packet 276** (`docs/spec_packets/276-retraction-toolchange-restart-lift-emitter/`, `draft`), preflight **PASS** after one BLOCKED round (wrong 04 link, fictional bool placeholder arm, fictional second unretract site, nonexistent schema binary, multi-filter test commands — all fixed and re-verified). Tier B sizing survived (all ten keys zero-occurrence) but membership did not: **8 keys in, `retract_before_wipe` shed to P37** (ticket 44 — needs wipe moves), **`long_retractions_when_ec` returned to the queue** (no geometric decision point); scalar-global with DEV-171, vector model stays with ticket 125. Seven keys wire into `DefaultGCodeEmitter::emit_gcode` (toolchange length with per-tool override — one intended default change 2.0 → 10.0, restart extras via a toolchange-boundary set, deretraction `mm/s * 60`, lift bound/surface gating with strict-parse rejection); `long_retractions_when_cut` publishes via the module placeholder seam with a key-specific `1`/`0` guarantee (ADR-0050). 04/05 rows annotated (P36 → 8, P37 → 11). No code change.

  - [44 — Author packet P37 — Extruder / Nozzle / Retraction (2/2) — emitter](issues/44-author-packet-p37-extruder-nozzle-retraction-emitter.md) — **authored as packet 277** (`docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/`, `draft`), preflight **PASS** after two S7 rounds (new-guard-binary home, then the Step 2/3 split the second round forced — both fixed and re-verified). Tier B sizing survived and membership held: **all 11 keys in, none shed, none returned** (all zero-occurrence as config-driven behaviour); the wipe trio lands as one new emission site (no wipe path exists today), and the `_cut`/`_ec` distances publish via the module placeholder seam (276's `_cut`-bool precedent, float spelling); scalar-global with DEV-172, vector model stays with ticket 125. Nine keys wire into `DefaultGCodeEmitter::emit_gcode` (travel gate — one intended default change, short travels newly skip retracts; layer-change gate; wipe emission + percent split; firmware `G10`/`G11` mode; hop-style arms with slope math + strict-parse rejection; Z-offset shift — second intended default change, slope-shaped default lift). 04 `_cut`/`_ec` rows narrowed to the placeholder seam, 05 P37 → packet 277. No code change. Preflight lesson recorded on the ticket: existing-binary guard cases need an edit-list home — splitting beats rationalizing.

  - [45 — Author packet P38 — Filament / Bed temperature — emitter](issues/45-author-packet-p38-filament-bed-temperature-emitter.md) — **re-sized at claim time: not authorable now, both keys re-filed, no packet, no code change** (the ticket-28/39 shape). Both keys zero-occurrence; canonical grounding puts every read site in `GCode` selection over absent vectors — the formula (`GCode::_print_first_layer_bed_temperature`, `GCode::process_layer`, `GCode::_do_export`) over per-filament `bed_temperature` vectors, the bed type (`GCode::get_highest_bed_temperature` plus the same three) over six Tier D plate-temperature vector pairs — so a packet today would be 100% declaration-only (rule 1). The tier table's emitter owner is additionally wrong for this tree (ticket-27 hazard): live bed-temperature emission is template-driven in `machine-gcode-emit` (`bed_temperature_initial_layer_single`), not in the host emitter. **Re-filed as [138](issues/138-author-packet-p38-bed-temperature-selection-refiled.md), blocked on 125** (itself gated on 126). Tier table + packet-list P38 rows annotate the re-file; no queue-count change.

  - [46 — Author packet P39 — Multimaterial / Filament for Features — emitter](issues/46-author-packet-p39-multimaterial-filament-for-features-emitter.md) — **closed by direct implementation of six keys; no packet** (human-grilled Q1–Q5: canonical `_id` names, full family, direct, paint wins, global only). Claim-time re-derivation: the 3 queued names are legacy aliases in the oracle (`PrintConfig.cpp` `handle_legacy` → `sparse_infill_filament_id` / `internal_solid_filament_id` / `outer_wall_filament_id`, coInt 0 = Default/inherit); the other 3 live members (`top/bottom_surface_filament_id`, `inner_wall_filament_id`) were never queued — absent from source, queue, and tree — so the **queue grows 407 → 410** (04/05/Notes updated). The tier-table emitter owner was wrong (ticket-27 hazard): all six are **live** in runtime entity assembly (`FeatureFilamentSelection` on `SupportToolSelection`, role→tool map porting `LayerTools::extruder`, resolved below every paint-derived source; runtime-only, no manifest/CONFIG_BLOCK work). 9 new tests (parser rebase/inherit/clamp + per-role routing + default identity + paint-wins); gates: lib 107/107, contract authored 4/4, integration support-identity 1/1, clippy + check-literals clean. No deviation rows, no packet number. Named limitations: legacy ingest spellings unused (no loader remap seam), out-of-range clamps per canonical (not the support reject), ThinWall/GapFill→outer and bridges→sparse are branch analogies.

  - [47 — Author packet P40 — Multimaterial / Flush options — emitter](issues/47-author-packet-p40-multimaterial-flush-options-emitter.md) — **closed by direct implementation of two keys; no packet** (claim-time re-sizing: both keys placeholder-only in canonical with zero tree occurrences, and the substitution seam already exists — a packet would be dead-weight ceremony). `filament_flush_temp` (coInts 0, max 1500) + `filament_flush_volumetric_speed` (coFloats 0.0, max 200) are **live** as scalar-global `ResolvedConfig` fields + `machine-gcode-emit` manifest rows, published as-is through the module's generic `[key]` sweep (no key-specific arm; int/float render canonically). Owner re-derived `crates/slicer-gcode` → `machine-gcode-emit` (ticket-27 hazard — canonical's reads are all `GCode.cpp` placeholder publication). 6 new tests (manifest guard, placeholder-render at ToolChange, ResolvedConfig default + round-trip); **DEV-171** (scalar-not-vector, 0-with-no-Tier-D-fallback, raw-not-plural names). Gates: machine-gcode-emit + slicer-ir + slicer-gcode (17 binaries) green, workspace clippy + check-literals + gen-config-docs clean, 46 guests rebuilt. Fast-purge branch stays out of the queue (05 P23 note).

  - [48 — Author packet P41 — Multimaterial / Multimaterial advanced — emitter](issues/48-author-packet-p41-multimaterial-multimaterial-advanced-emitter.md) — **re-sized at claim time: not authorable now, returned to the queue as unimplemented, no packet, no code change** (the ticket-28/39 shape). `support_object_skip_flush` is zero-occurrence in the tree and both canonical reads (`GCode.cpp` sequential-toolchange + by-layer extrusion loop) are riders on the exclude-object seam (`m_enable_exclude_object` + `M624` label codes — P44 / ticket 51 scope, still open), so wiring the bool alone would be declaration-only (rule 1). Passes rule 3 (live in `libslic3r/`, stays in scope). 04/05 rows annotate the return; sequences after (or folds into) P44 when ticket 51 lands (6→7 keys stays under the B ceiling). No new ticket (the `start_end_points` shape: blocker is one packet, not the per-tool model). No queue-count change.

  - [49 — Author packet P42 — Multimaterial / Ooze prevention — emitter](issues/49-author-packet-p42-multimaterial-ooze-prevention-emitter.md) — **re-sized at claim time: not authorable now, all four keys re-filed, no packet, no code change** (the ticket-28/39 shape). All four keys zero-occurrence as behaviour (the one `ORCA_CONFIG_PADDING` row is not evidence, rule 2); all pass rule 3 (live in `GCode.cpp` / `GCodeProcessor`, stay in scope). The ooze pair (`ooze_prevention` + `standby_temperature_delta`) needs per-filament nozzle-temp vectors the host cannot see — its only temp field is the unrelated `filament_flush_temp`, real nozzle temps are module-side, and the host emitter never emits `Temperature` commands; the preheat pair (`preheat_time` + `preheat_steps`) needs a `GCodeProcessor`-style backtrace injector with no port seam (no usage-block builder, no XL concept, no filament count). Owner stands as tiered (`crates/slicer-gcode`, emission-time). **Re-filed as [139](issues/139-author-packet-p42-ooze-prevention-refiled.md), blocked on 125** (itself gated on 126). Tier table + packet-list P42 rows annotate the re-file; no queue-count change.

  - [50 — Author packet P43 — Multimaterial / Prime tower — emitter](issues/50-author-packet-p43-multimaterial-prime-tower-emitter.md) — **split at claim time: one key live by direct implementation, one returned to the queue** (the ticket-40/42 shape, no packet). `manual_filament_change` is **live**: `ResolvedConfig` bool (canonical default false) + `to_config_map` arm for module visibility, serializer `; MANUAL_TOOL_CHANGE T<n>` tag line verbatim (`GCodeWriter::toolchange_prefix`), and `machine-gcode-emit` skipping the `FilamentChange` injection at 1-based count 1 only (`GCode.cpp` `m_toolchange_count == 1`); 6 new tests across three binaries plus a manifest guard; CONFIG_BLOCK gains exactly one default-false line (word-form-bool spelling rides ticket 132). The `GCodeProcessor` comment-analysis arm is analysis-only — noted, not deviated. `single_extruder_multi_material_priming` is **unimplemented**: all four canonical reads sit in Type2 priming-tower flows with no port subject (purge-only tower, ticket-29 census) — sequences after ticket 122, the same seat as `wipe_tower_filament`. 04/05 rows annotated; P43 now covers 1 key. Verification also surfaced a pre-existing HEAD regression (ticket 42's strict PA ingest rejects real 3MF vectors — 5 e2e reds), filed as [140](issues/140-e2e-pressure-advance-vector-ingest-regression.md), blocked on 125. No queue-count change.

  - [126 — Close `overlay_resolved`'s 29-of-83 field narrowing, and prove the precedence defect](issues/126-overlay-resolved-field-narrowing.md) — **closed by direct implementation, no packet; both parts proven by test then fixed outright.** Part 2 reproduces exactly through the full production stack (global 0.6 / paint 0.5 / tool silent on the key → region came out 0.6): the fix compares each overlay against its **resolution origin** (the global config), not against `ResolvedConfig::default()` and not against the base being overlaid (the ticket's alternative is wrong — the base already carries the paint value when the tool overlay composes, so it would still clobber). Same move fixes "override back to default" and the `extensions` blanket-merge clobber of per-object overrides. Part 1 closed structurally: `ResolvedConfig::overlay_onto` (`crates/slicer-ir/src/resolved_config.rs`) is emitted by the `declare_resolved_config!` macro itself (new `__drc_overlay_arms!`), one arm per declared field (73+3, re-derived), so no allowlist can drift again; the kernel threads the origin to all five composition sites. 6 new tests (4 kernel through the real scheduler resolvers, 2 macro drift guards incl. whole-struct `PartialEq` over every declared field). **Unblocks 125.** Incidental finding recorded on ticket 140: the failing set already moved — ticket 47's `filament_flush_*` strict scalars reject the same fixtures' `coFloats` lists before the PA keys do. Gates: slicer-ir + slicer-core (host-algos) + scheduler + runtime unit (89) + runtime integration (345) green, workspace clippy + check-literals clean, 46 guests rebuilt with `--check` exit 0. No key declared, no packet number.

  - [51 — Author packet P44 — Others / G-code output — emitter](issues/51-author-packet-p44-others-g-code-output-emitter.md) — **authored as packet 278** (`docs/spec_packets/278-gcode-output-emitter-modes/`, `draft`), preflight **PASS**. Claim-time re-derivation kept Tier B but changed membership and two owners: **5 keys in, `filename_format` returned to the queue, no code change.** Five keys zero-occurrence as behaviour (`exclude_object`, `gcode_comments`, `gcode_label_objects`, `reduce_infill_retraction` — the one hit is the padding row, rule 2); `gcode_flavor` only parsed + echoed, no flavor-dependent syntax. `filename_format` owner-corrected to host-export/CLI (an emitter never chooses filesystem paths; missing feature is placeholder-based output naming with no `--output`; natural host is open P84/P85, tickets 91/92). `reduce_infill_retraction` owner-corrected to `path-optimization-default` (packet-15/TASK-120d1 retract-policy precedent). `support_object_skip_flush` sequenced behind this packet's exclude-object seam, not folded (needs Bambu flavor + M624 + flush carrier; ticket 48's re-entry call). P44 now covers 5 keys; no queue-count change.

  - [52 — Author packet P45 — Others / Special mode — emitter](issues/52-author-packet-p45-others-special-mode-emitter.md) — **authored as packet 279** (`docs/spec_packets/279-spiral-vase-modes/`, `draft`), preflight **PASS** (S0–S8 clean). Claim-time re-derivation kept Tier B with all **5 keys in and one spelling adoption, no code change.** The four SpiralVase keys are zero-occurrence; `spiral_mode` is padding-only — but the tree already holds the same decision point under PnP spelling `spiral_vase` (scheduler classic-forcing dispatch + both perimeter manifests), so the packet adopts `spiral_mode` canonically with `spiral_vase` as fallback alias rather than building a second mechanism. Builds the emitter SpiralVase stage (Z-ramp, smoothing under cap, flow ramps, tiny-move removal), orchestration validation (copies/materials/relative-only), and the map's time-lapse `!spiral` fog obligation. Slicing beyond classic-forcing is a recorded non-borrow with an `[FWD]` re-check. P45 still covers 5 keys; no queue-count change.

## Not yet specified

- **The time-lapse gate has two open clauses waiting on other work.** Surfaced by
  ticket 27, recorded as `DEV-168`. `machine-gcode-emit` now gates
  `time_lapse_gcode` on `printer_structure`, but canonical's gate is
  `(is_i3 && !spiral_vase) || is_multi_extruder` and this port can express
  neither trailing term: it has no spiral mode (`spiral_mode` is an
  unimplemented queue key, owner `crates/slicer-gcode` + orchestration), and no
  extruder-count key reaches a `PostPass` module, so `is_multi_extruder` is
  approximated by "this print performs a toolchange". Whichever packet lands
  spiral mode inherits the obligation to extend the gate; whether a printer-level
  extruder count should reach the postpass seam is a config-surface question for
  the extruder/nozzle packets. Fog until one of them picks it up.

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
- **How far the per-tool config axis must reach into geometry.** The
  *inventory* and the *ruling* halves of this patch have both graduated —
  [118](issues/118-inventory-per-tool-config-mechanism.md) measured the mechanism,
  and the ruling is now
  [125 — Rule on the port's per-tool config model](issues/125-rule-per-tool-config-model.md),
  unblocked 2026-09-06 by
  [126](issues/126-overlay-resolved-field-narrowing.md)'s resolution
  (overlay composition is origin-aware over every declared field; see the
  Decisions entry).
  What stays fog is what 125's answer implies downstream: a region's tool identity
  exists today only inside a `("material", ToolIndex(n))` paint variant chain, so
  "this object prints with tool 2" has no representation anywhere in the prepass
  IR. Whether that needs one — and whether an *extruder* axis is distinct from the
  *tool* axis at all — is not phraseable as a ticket until 125 picks a model. The
  deferred Tier D keys, ticket 119's keys, and ticket 136's six keys (P32's
  per-extruder identity/geometry/tool-map re-file) hang on 125; take their counts from
  [04's tier table](issues/04-asset-tier-assignment.md) at the point of use.
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
  Ticket 22's answer records this (`DIV-267-A`/`DIV-267-B`) rather than changing
  it: the direct implementation moves the gate's *key*, not the gate's
  *subject*, and rewriting the subject would change output for everyone already
  using the feature under a change whose claim is default-path identity. Closing
  it means carrying support contact/interface polygons across the WIT boundary
  to a `SupportPostProcess` module — an IR field plus a WIT accessor, so
  queue-sized geometry/contract work. Note this makes P15 a **key-parity**
  packet only; nobody should read the queue's `support_ironing` coverage as
  geometry parity. Fog until a packet picks it up; the neighbouring
  support-interface work (packet `260b-support-interface-fill-claim-holders`)
  is the natural host, and the returned-to-queue `support_ironing_pattern`
  claim seam should be scoped with it.
- **Exact canonical relative ironing-angle parity.** The P14 packet's top module
   reads the shared `infill_direction` input as its base angle (packet 266,
   `DIV-266-B`), so the residual gap is the per-region rotation template
   canonical folds into the solid fill's own direction — the region view has no
   solid-infill direction or rotation-template metadata. Whether the IR gains
   the canonical base direction, and which other ironing consumers use it, is
   future IR/geometry work; fog until a packet picks it up.
- **The filament-change travel path (`start_end_points`).** Surfaced by ticket
  40's authoring: canonical's `get_path_of_change_filament` (`GCode.cpp`)
  computes the three `travel_point_*` placeholders for `change_filament_gcode`
  from `start_end_points` + `bed_exclude_area` + object bounding boxes, and
  returns the safe default path when `bed_exclude_area.size() != 4`. The key
  was returned to the queue as unimplemented (tier-table row annotated):
  wiring it alone would be declaration-only until packet 256's
  `bed_exclude_area` implementation lands, and the path computation needs
  object bounding boxes at a seam that reaches the postpass substitution (new
  `ResolvedConfig` fields or an extensions mechanism + `travel_point_*` schema
  on `machine-gcode-emit`). Fog until packet 256's implementation lands; the
  re-file should fold into or sequence after it.

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
