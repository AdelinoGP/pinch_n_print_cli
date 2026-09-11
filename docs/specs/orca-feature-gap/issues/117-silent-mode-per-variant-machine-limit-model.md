# 117 — `silent_mode`: needs a per-variant machine-limit model

Type: task
Status: resolved
Assignee: wayfinder session (ses_f717e4784ffeBzgoAQj4JoT66G) — claimed 2026-09-11, resolved 2026-09-11
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 25 (P18 authoring), which returned `silent_mode` to the queue
as unimplemented. Canonical declares `silent_mode` as a `coBool` (default
`false`, `comDevelop`-gated) and reads every `machine_max_*` key through
`printer_options_with_variant_2`, so each value is a stride-2 array of
(normal, stealth) pairs; `silent_mode` selects which variant the machine
envelope (`GCode::print_machine_envelope`) and the estimator consume.

PnP's ten machine-limit fields (`machine_max_acceleration_extruding`,
`machine_max_acceleration_travel`, `machine_max_speed_x/y/z/e`,
`machine_max_jerk_x/y/z/e` in `crates/slicer-ir/src/resolved_config.rs`) are
scalar `Option<f32>` values with no variant dimension, and
`EstimatorLimits::from_config` (`crates/slicer-gcode/src/estimator.rs`) reads
them directly. Declaring `silent_mode` would be a declaration-only key under
the map's Authoring rule 1: there is no decision point it can drive.

Decide and execute:

1. **The per-variant model.** Widen the `machine_max_*` fields to carry a
   normal/stealth pair (or a `silent_mode`-selected variant index) so the
   envelope and the estimator can select the variant. This is P47-family work:
   the motion-limits packet (ticket 54) owns the `machine_max_*` keys and
   packet 267's envelope (M203/M204/M205) records the missing groups as
   divergences that this model would also feed.
2. **The consumer wiring.** `silent_mode = true` must change the emitted
   envelope values and the estimator limits to the stealth variant, with
   invariant tests at both seams.
3. **The queue records.** When the model lands, the tier row in
   `04-asset-tier-assignment.md` and the P18 entry in
   `05-asset-packet-list.md` graduate `silent_mode` from "returned to queue" to
   its owning packet.

Related but separately ruled, do **not** fold in: the P47 missing fields
(`machine_max_acceleration_x/y/z/e`, `machine_max_acceleration_retracting`,
`machine_max_junction_deviation`, `machine_min_extruding_rate`,
`machine_min_travel_rate`) — those belong to the motion-limits packet itself,
and packet 267 records their absence as divergences rather than inventing
values.

## Answer

Resolved 2026-09-11 by direct implementation (no packet — the map's
"Packets are for complex implementation only" rule; the decision point
already existed). Grilled Q1–Q4 with the user; all four took the recommended
option.

**1. The per-variant model.** The ten `machine_max_*` fields
(`machine_max_acceleration_extruding|travel`, `machine_max_speed_x/y/z/e`,
`machine_max_jerk_x/y/z/e` in `crates/slicer-ir/src/resolved_config.rs`) are
now `Option<MachineLimitPair>` (`{ normal, stealth }` with a
`select(silent_mode)` accessor) instead of scalar `Option<f32>`. The new
`extract_machine_limit_pair` keeps both stride-2 entries: scalar → both,
one-element list → both, `[n, s, ...]` → the first pair (trailing printer
variants dropped — single-extruder scalar-subset, DEV-169 (c) precedent).
Unparseable elements and empty lists stay hard errors. `PartialEq`/`Hash`
compare/hash both entries bit-exact; `overlay_onto` covers the new
`silent_mode` field and the retyped fields automatically (macro arms), proven
by the existing drift guard. Wire type is `float-list`; the overlay sentinel
(single-element list) is accepted by the new extractor.

**2. The consumer wiring — estimator only (Q1 correction).** Re-derived
against the designated oracle: `GCode::print_machine_envelope` always reads
the normal variant (`values[extruder * stride]`), and only the time estimator
switches (`init_gcode_processor` + `apply_config` enable the stealth
estimator). The ticket's "envelope must change" clause was factually wrong
and was not built — the envelope keeps the normal entry, which is what draft
packet 267 already specifies. `EstimatorLimits::from_config`
(`crates/slicer-gcode/src/estimator.rs`) now selects
`pair.select(cfg.silent_mode)` per field, falling back per-field to the
estimator defaults when absent (so `silent_mode` with no configured limits
is identity). Selection is flavor-agnostic (Q3) — canonical gates on
Marlin/Marlin2 but this seam carries no flavor — recorded as **DEV-200**.
`silent_mode` itself (canonical `coBool` default `false`) is a
`ResolvedConfig` bool, host-only and omitted from the CONFIG_BLOCK (the
`disable_m73`/P35 `pressure_advance` precedent). CONFIG_BLOCK spelling for
the pairs is conditional: equal variants keep the historical bare-value
spelling (byte-stable — the fork-keys test passes unchanged), distinct
variants emit the canonical comma-separated pair.

**3. The queue records.** 04's `silent_mode` row graduates from "returned to
queue" to Tier B live; 05's P18 entry records the direct landing (packet
stays 3 keys). Packet 281's eight scalar-global fields are untouched per the
ticket's scope fence (their first-wins stealth gap stays DEV-173's).

Tests: 9 `machine_limit_config_tests` (ingest both-modes, scalar/1-list/n-list
duplication, first-pair-wins, pair emission, silent parse, rejections) +
`silent_mode` default/omission pins; 4 new estimator tests (normal select,
stealth select, absent-limits identity, end-to-end slower-under-stealth).
Gates: `slicer-ir` 22 binaries + `slicer-gcode` 17 binaries + `slicer-scheduler`
9 binaries + runtime header/unit/doc-lock + e2e 147/147 green; workspace
clippy + check-literals clean; doc 15 regenerated (59 host keys);
`check-deviations --check` green (73 open); 36 guests rebuilt, `--check`
clean.

Observations (not fixed — out of scope): `disable_m73` is missing from the
manual `PartialEq`/`Hash` impls (declared line 1989, compared nowhere) —
same drift class ticket 126 fixed for the overlay; silent_mode was added to
both. The P47 missing fields (`machine_max_acceleration_x/y/z/e`,
`machine_max_acceleration_retracting`, `machine_max_junction_deviation`,
both minimum rates) still need the same pair treatment when packet 281
activates — 281's "stealth stays with 117" note now means "extend the
`MachineLimitPair` pattern", not "invent a model".
