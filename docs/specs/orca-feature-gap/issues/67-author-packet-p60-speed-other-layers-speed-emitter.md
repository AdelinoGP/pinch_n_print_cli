# 67 — Author packet P60 — Speed / Other layers speed — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260908_P60) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P60 — Speed / Other layers speed — emitter** — 2 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P60 — Speed / Other layers speed — emitter):

`internal_solid_infill_speed`, `small_perimeter_speed`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/293-speed-other-layers-emitter/`](../../../spec_packets/293-speed-other-layers-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0 five files non-empty; S1 no satisfied-dep claim — 291 is anchor-awareness not a dep, 289 position-adjacent with no shared helper and no ordering edge; S2 `DEV-185` absent from the log and format-conformant, collision-free — LOG max DEV-171, drafts claim DEV-172–DEV-184; S3 no schema versions; S4 no new ADRs; S5 all symbols resolve — `DefaultGCodeEmitter::resolve_feedrate` InternalSolidInfill→sparse arm + comment in `emit.rs`, `is_loop`/`is_closed` in `slice_ir.rs`, `SPEED_KEYS`/`SPEED_META`/`SPEED_BOUNDS`/`SPEED_KEY_COUNT`/`Default` in `feedrate.rs`, `declare_resolved_config!` + `extract_float_or_first` + `support_threshold_overlap` `ResolvedFloatOrPercent` row + sparse `to_config_map` arm in `resolved_config.rs`, classic `small_perimeter_threshold` toml block + `classify_narrow_island` use with verified-zero `small_perimeter_speed`, rectilinear dead tuple + `speed_factor 1.0`, `run.rs` table+twin composition, `pipeline.rs` resolved-map baseline, `serialize.rs` zero twins; S6 all role spellings resolve, no WIT claim; S7 guard auto-discovered per-file binary, `--test` form matches the 289/291/292 precedent; S8 ADR-0052 cited as precedent only, no normative edit — plus AC-command and Doc-Impact green).

**Claim-time sizing: Tier B held, owner stands with no seam correction, membership held 2-in — no code change.**
Both keys are live in canonical (not dead, no alias) and zero-occurrence as behaviour here (verified by grep — `small_perimeter_speed` nowhere under `crates/`/`modules/`/`xtask/` outside map prose; `internal_solid_infill_speed` parse-only dead in `rectilinear-infill`). Every canonical read site is emission-time (`GCode` speed selection + `extrude_loop` gate, `Fill.cpp` `role_speed` cited as the no-dual-implementation evidence), so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep publishes placeholders, per-move speeds are the host emitter's job; the rectilinear parse is dead input, not a decision point). Tier table owner text narrowed from `feedrate.rs` (table only) to the reseat + gate sites in `emit.rs`.
**Canonical scalarity NOT held** — `internal_solid_infill_speed` is a `coFloats` per-nozzle nullable vector and `small_perimeter_speed` is `coFloatsOrPercents` per-nozzle, so unlike packet 291 there IS a ticket-125 vector arm and DEV-185(b) records the scalar-global simplification with first-wins ingest (ticket-140 precedent).

**Authoring-time grounding findings (recorded in the packet):**
- Canonical declarations grounded against the map oracle: `internal_solid_infill_speed` coFloats `100` min 1; `small_perimeter_speed` coFloatsOrPercents `50%`-percent `ratio_over outer_wall_speed` min 1; `small_perimeter_threshold` coFloats `0` min 0 (all per-nozzle nullable vectors).
- Port shape: one-line reseat (`InternalSolidInfill` → new table field) plus a small-perimeter gate helper at the per-entity `F` site wrapping 291's blended base (never composed — ADR-0052), qualifying on `threshold > 0` + `role.is_loop()` + `is_closed()` + `planar_length <= threshold * 2 * PI` (the `SMALL_PERIMETER_LENGTH` circumference conversion, scaling not borrowed); three value arms (`0` → `outer * 0.5` auto, percent → live outer base, absolute → as-is); wall-loops-only by construction (DEV-185(c)).
- The host `small_perimeter_threshold` twin is deliberately separate from the module width-classification twin (same canonical name family, different inputs — neither reads the other, unification out of scope); the rectilinear parsed tuple stays dead (a second implementation would reintroduce ticket-114's retired double-count); `speed == -1` has no port analogue (recorded, not stubbed); the minimum-cross-section guard and the support-side sibling pair are named non-borrows.
- Defaults ARE near-identity: F stream byte-identical (100 == shadowed sparse 100; threshold 0 silences the gate), CONFIG_BLOCK +1 line (the internal-solid twin via the resolved map; the small-perimeter pair stays host-only omitted, ticket-42 precedent — AC-2 pins both halves).
- Packet number `292` → `293` derived from disk per ticket 06; DEV-185 first collision-free (LOG max 171, drafts 172–184).
- 04 tier rows narrowed (feedrate.rs → reseat + gate sites in emit.rs, packet 293); 05 P60 section annotated (2-in at 293); no queue-count change. No new fog graduated, nothing ruled out of scope.

### Gates

- `cargo check -p slicer-gcode --all-targets` green at authoring time (no code changed): proves the cited baseline the packet grounds against. Full packet verification (guard + clippy + gen-config-docs) belongs to the packet's own Verification list at swarm time.
