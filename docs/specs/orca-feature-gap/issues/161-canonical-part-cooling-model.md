# 161 — Port the canonical part-cooling model (`fan_min_speed`, layer-time interpolation, ramp)

Type: gap
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

`com.core.part-cooling` implements a reduced fan model: after
`close_fan_the_first_x_layers` it emits `fan_max_speed`, and on an overhang
event it emits `overhang_fan_speed × fan_max_speed / 100`. Canonical
`GCode/CoolingBuffer.cpp::apply_layer_cooldown` computes the target from the
same two bounds plus layer time and layer index:

- `fan_min_speed` is the base when `reduce_fan_stop_start_freq` is on, and the
  low end of the interpolation.
- `fan_max_speed` applies when `layer_time < slow_down_layer_time`.
- Between `slow_down_layer_time` and `fan_cooling_layer_time` the value is
  linearly interpolated from the ceiling to the floor.
- `full_fan_speed_layer` ramps the result in over the first layers after the
  close-layer window.
- At emission `GCodeWriter::set_fan` raises a non-zero speed to
  `part_cooling_fan_min_pwm`, then converts percent to the 0–255 PWM value.

Canonical declares both bounds as `coFloats` in percent (`0..100`, defaults 20
and 100). This port declares `fan_max_speed` as raw `int 0..255` — ticket 111's
percent conversion is still a draft packet (253) — and until this change
declared `fan_min_speed` as raw `int 51` (the canonical 20 % expressed in the
raw scale) but read it nowhere. The declaration was removed rather than shipped
as a key no code honours (map rule: a key is covered only when the behaviour
exists and the key drives it).

## Acceptance

- One fan-speed decision path in `com.core.part-cooling`, driven by config,
  implementing the base/ceiling/interpolation/ramp bands and the PWM floor.
- The per-extruder percent semantics adopted for both bounds, which also
  resolves the raw-0–255 divergence.
- Tests pin each band at its boundary: close layers, `slow_down_layer_time`,
  the interpolation midpoint, `fan_cooling_layer_time`, `full_fan_speed_layer`,
  and a non-zero `part_cooling_fan_min_pwm`.
- `fan_min_speed` is re-declared only when it drives a decision.

## Notes

- The overhang multiplier (`overhang_fan_speed × fan_max_speed / 100`) is a
  separate PnP path; check it against canonical's `change_extruder_set_fan`
  callers before folding it in.
- This is not required for the merge that removed the dead declaration.
