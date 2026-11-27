# 64 — Author packet P57 — Speed / Advanced (Speed) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260907_P57) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P57 — Speed / Advanced (Speed) — emitter** — 3 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P57 — Speed / Advanced (Speed) — emitter):

`extrusion_rate_smoothing_external_perimeter_only`, `max_volumetric_extrusion_rate_slope`, `max_volumetric_extrusion_rate_slope_segment_length`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/290-speed-advanced-emitter/`](../../../spec_packets/290-speed-advanced-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0 five files non-empty; S1 no satisfied-dep claim — no dep on any draft packet, arc fitting is 284's scope with tooltip-only coupling, spiral-279 ordering not borrowed; S2 `DEV-182` absent from the log, format-conformant, collision-free — LOG max DEV-171, drafts claim 172–181; S3 no schema versions; S4 no new ADRs; S5 all symbols resolve — `extract_float` / `extract_bool` / `declare_resolved_config!` in `resolved_config.rs`, `emit_gcode` + `with_resolved_config` post-loop tail in `emit.rs` (no smoother exists anywhere), `ExtrusionRole` bridge/ironing/outer variants + `Point3WithWidth.overhang_quartile` in `slice_ir.rs`, `ORCA_CONFIG_PADDING` + `serialize_config_block` in `serialize.rs` (zero twins), `gen-config-docs --check` gate spelling, test autodiscovery; S6 all role/quartile identifiers resolve, `smooth_extrusion_rates` net-new; S7 file-per-binary auto-discovered (15 files, no aggregator), guard self-contained within the 3-edit cap; S8 ADR-0062-conformant — F-retime + E-conserved splits are speed-side, direction preserved).

**Claim-time sizing: Tier B held, owner stands but the seam corrected, membership held 3-in — no code change.**
All three keys live in canonical (none dead, no alias) and zero-occurrence as behaviour here (verified by grep — no smoother, no marker blocks, no key spelling anywhere under `crates/`/`modules/`/`xtask/` outside map prose). Every canonical read site for the three is the emission-side `PressureEqualizer` text post-stage, so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep publishes placeholders, move retiming is the host emitter's job). Tier table owner text corrected from `estimator.rs` (time-math only) to the post-loop smoothing stage.
**Canonical scalarity IS held** — all three are scalar `GCodeConfig` fields, so unlike packets 276/277/279–289 there is no ticket-125 vector arm and DEV-182 carries no vector clause.

**Authoring-time grounding findings (recorded in the packet):**
- Canonical defaults/bounds grounded against the map oracle: `max_volumetric_extrusion_rate_slope` coFloat `0` min `0`; `max_volumetric_extrusion_rate_slope_segment_length` coFloat `3.0` min `0.5` max `5`; `extrusion_rate_smoothing_external_perimeter_only` coBool `false`; all scalar.
- Master gate: slope `> 0` constructs the equalizer, else the stage does not exist — so defaults ARE identity (byte-identical default stream, AC-2 pins it; the inverse of packet 289's emitting default).
- Port shape: IR-native post-stage over built `GCodeCommand::Move`s (E deltas already encode the cross-section — no text re-parse, no filament table, no one-layer lookbehind; the ×3600 is implicit in mm/min rate math); skip list exactly bridge + ironing + the external-only gate (`OuterWall` + `overhang_quartile`-marked points — `erOverhangPerimeter` folds into the quartile arm, the port's overhang representation); single-slope limiter (canonical's ctor assigns both directions from the one value); F-only rewrite with E conservation; segment splitting at the configured length with the trivial floor.
- No deps: arc fitting is 284's scope with tooltip-only coupling (no code edge — grep finds none); spiral-279 is not a dep (position borrowed, filter shape not); the estimator needs no change (it times the smoothed stream automatically); the `@`-block bounds flow through the existing ticket-113 `ConfigBoundsIndex::from_modules` seam.
- Packet number `289` → `290` derived from disk per ticket 06; DEV-182 first collision-free (LOG max 171, drafts 172–181).
- 04 tier rows corrected (estimator.rs → post-loop smoothing stage, packet 290); 05 P57 section annotated (3-in at 290); no queue-count change. No new fog graduated, nothing ruled out of scope.

### Gates

- `cargo check -p slicer-gcode --all-targets` green at authoring time (no code changed): proves the cited baseline the packet grounds against. Full packet verification (guard + clippy + gen-config-docs) belongs to the packet's own Verification list at swarm time.
