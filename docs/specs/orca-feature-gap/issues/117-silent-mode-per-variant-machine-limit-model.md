# 117 — `silent_mode`: needs a per-variant machine-limit model

Type: task
Status: open
Assignee: —
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
