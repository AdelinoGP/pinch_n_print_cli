# 111 — Convert the part-cooling fan scale to percent 0–100, and make `overhang_fan_speed` absolute

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-04)
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling rulings **Q4(a)** (*"convert
port to percent 0–100"*, align-default) and **Q4(b)** (*"[`overhang_fan_speed`]
absolute, matching canonical"*, implement — folded into Q4(a)).

The port declares its fan keys on a raw PWM 0–255 scale; canonical declares
`fan_min_speed` / `fan_max_speed` min 0 / max 100. Verified in-tree (2026-09-02),
`modules/core-modules/part-cooling/part-cooling.toml`:

- `fan_min_speed` — `int`, default `51`, `min 0`, `max 255`
- `fan_max_speed` — `int`, default `255`, `min 0`, `max 255`
- `overhang_fan_speed` — `int`, default `100`, `min 0`, **`max 100`**

Two separate problems, and the second is not the one the audit first wrote down.

**(a) The unit divergence is real.** 51/255 = 20% and 255/255 = 100% are exactly
canonical's 20 and 100, so the *defaults* are physically identical — this is a
unit fix, not a value fix. What breaks is any non-default Orca input: a 3MF
setting `fan_max_speed = 100` means "full" upstream and lands as ~39% here. The
ruling is to convert the port to percent rather than convert at the config
boundary.

**(b) `overhang_fan_speed`'s divergence is semantic, not scalar.** Note the
correction already recorded in `key-correction-inventory.md` §"Corrections to
this document": the row claiming "100 means full in Orca and about 39% in the
port" is **wrong** — the key is already declared 0–100, and
`part-cooling/src/lib.rs` computes `(overhang_fan_speed * fan_max_speed) / 100`,
so at defaults it yields 255, i.e. full. The genuine divergence is that the port
treats it as a **percentage of `fan_max_speed`** while canonical assigns it
directly and compares it against the current speed. Q4(b) rules: make it
absolute.

Decide and execute:

1. The manifest tables' new types/bounds and the module-side arithmetic in
   `modules/core-modules/part-cooling/src/lib.rs`, including the `u8` PWM value
   the emitter ultimately needs — where does percent→PWM conversion happen, and
   once only?
2. `overhang_fan_speed`'s absolute semantics and the comparison-against-current
   behaviour canonical uses.
3. The back-compat break: existing user configs carrying 0–255 values will be
   read as percentages and clamp. Is that accepted (the ticket-107 /
   Q14(b) precedent) or does it need a migration?
4. Whether the sibling layer-time slowdown keys ride along —
   `slow_down_for_layer_cooling`, `slow_down_layer_time`, `slow_down_min_speed`
   are listed in the audit's "In-scope keys not ruled on" as STUBs that no
   question reached. Probably a separate ticket; say so either way.

**Binds packet 253** (`docs/spec_packets/253-part-cooling-fan-scale-and-cooling-keys/`,
authored by ticket 08 and since re-authored under the Authoring rules), which
covers these fan keys. Sequence this against that packet rather than duplicating
it; if the packet already specifies the conversion, this ticket's job is to
confirm and close, not to re-decide.

## Answer

**Resolved 2026-09-04 — confirm-and-amend. The conversion is packet 253's; this ticket ruled the open semantics, corrected three errors in the packet, and did not re-decide the conversion.**

Packet 253 (`docs/spec_packets/253-part-cooling-fan-scale-and-cooling-keys/`, still `status: draft`) already specifies the percent conversion in AC-1/AC-2/AC-2b/AC-3/AC-4 and negative case AC-N3. Per this ticket's own instruction, the job was to confirm and sequence against it, not duplicate it — so **no code changed in this session**; the deliverable is the packet's corrected text.

**Canonical grounding** (delegated read of the designated oracle at the ticket-33 path `pinch_n_print_cli\OrcaSlicerDocumented`, re-derived at point of use; cited by file + function only):

- `PrintConfigDef` (`PrintConfig.cpp`): `fan_min_speed` and `fan_max_speed` are `coFloats`, min 0 / max 100, defaults 20 / 100, sidetext `%`. `overhang_fan_speed` is `coInts`, min 0 / max 100, default 100, sidetext `%`. All percent — the port's 0–255 scale is the divergence, as the ticket stated.
- `apply_layer_cooldown`'s lambda `change_extruder_set_fan` (`CoolingBuffer.cpp`): all four role fan speeds are read as **raw absolute percents**, never multiplied by `fan_max_speed`.
- `GCodeWriter.cpp`: `set_fan` uses `static_cast<unsigned int>(255.5 * speed / 100.0)`; `set_additional_fan` uses `(int)(255.0 * speed / 100.0)`; `set_exhaust_fan` uses `(int)(speed / 100.0 * 255)`.

**The four questions:**

1. **Where percent→PWM happens, and how often — three converters, one per channel, not one shared helper.** `set_fan`'s `255.5` biases truncation into round-half-up; `set_additional_fan` and `set_exhaust_fan` both use `255.0` plain truncation (numerically equivalent to each other, different from `set_fan`). Conversion happens once per channel at the emission site inside the owning guest — `part-cooling` for `M106 S` and `M106 P2`, `machine-gcode-emit` for `M106 P3`; no host-side conversion. This matches packet AC-2, which `design.md` **contradicted** — see the corrections below.
2. **`overhang_fan_speed` is absolute, and its selection is gated by `>`, not by `max`.** Canonical sets `overhang_fan_control = overhang_fan_speed > fan_speed_new` against the base speed the curve just produced, and on true the overhang value *replaces* the current fan speed outright. So at packet-253 defaults (base 100, overhang 100) the gate is **false** and no bump is emitted — the byte stream is unchanged only because the two values coincide. The `full_fan_speed_layer` ramp scales it by the same `factor` as the base, clamped `[0, 255]` after conversion. `-1` fallbacks: `internal_bridge_fan_speed` inherits **both** `overhang_fan_speed` and its control flag; `support_material_interface_fan_speed` and `ironing_fan_speed` have `>= 0` control flags with no fallback substitution.
3. **The back-compat break is accepted, unmigrated (user ruling, 2026-09-04, on the ticket-107 precedent).** No alias, no migration, no version gate, no deprecation warning. An existing config carrying `fan_max_speed = 255` fails loudly at `ConfigBoundsIndex::check` (`crates/slicer-scheduler/src/config_resolution.rs`) as out of range; a value in 0–100 is silently reinterpreted on the new scale (today's default `51`, meaning 20%, would read as 51%). Accepted knowingly. Recorded in the packet's Architecture Constraints so an implementer cannot re-open it.
4. **The sibling slowdown keys do NOT need a separate ticket — packet 253 already owns them.** `slow_down_for_layer_cooling`, `slow_down_layer_time` and `slow_down_min_speed` are carried by AC-10 (the ported `CoolingBuffer::calculate_layer_slowdown` stage writing `EntityMutation::SetSpeedFactor`), and `dont_slow_down_outer_wall` by AC-11. The audit's "In-scope keys not ruled on" entry for them is stale; no ticket filed.

**Three errors corrected in packet 253** (all in text authored before this ticket's rulings; the packet is draft and unactivated, so the amendments are free):

- **`design.md` §Architecture Constraints said "The percent→S conversion is shared: implement exactly one helper … Both the primary channel and the `P2` auxiliary channel use it."** That directly contradicts the same packet's AC-2, which demands three distinct converters plus a test that fails loudly on a collapse, and contradicts `design.md`'s own change-surface steps 7 and 9, which name `percent_to_additional_fan_s` and `percent_to_exhaust_fan_s`. Replaced with the three-converter statement and the canonical formulas. Also noted: AC-2's "assert they disagree for at least one percent value" is satisfiable only for the `set_fan`-vs-others pair, since `P2` and `P3` are numerically equivalent — the three helpers are kept as three anyway, because the divergence is canonical's.
- **The same bullet said `overhang_fan_speed`'s "product with `fan_max_speed` must switch to percent×percent"** — i.e. it preserved the port's percentage-of-max semantics, which is exactly what Q4(b) ruled against, and contradicts the packet's own AC-4. Replaced with the absolute semantics and the `>` gate; AC-4 in `packet.spec.md` amended to state the gate explicitly.
- **`design.md`'s default-path note claimed a default-config overhang bump of `M106 S100`.** Wrong twice: today's module computes `(100 × 255) / 100` = S255 for that bump, and under absolute semantics the default gate is false so there is no bump. The default byte stream is unchanged either way, but not for the stated reason. Corrected.

**One fixture the packet under-scoped.** `design.md` said "fixture updates limited to restating raw-byte expectations in percent terms". `overhang_region_bumps_fan` (`modules/core-modules/part-cooling/tests/part_cooling_tdd.rs`) drives `fan_max_speed = 255`, `overhang_fan_speed = 40` and asserts a bump to `M106 S102` against a base of S255 — it pins an overhang speed *below* the base producing a bump. Under absolute semantics that configuration produces **no** bump (`40 > 100` is false), so the test's premise inverts and a units restatement will not save it. The packet now instructs re-authoring it with an overhang speed above the base (`fan_max_speed = 50` → base `trunc(255.5 × 50 / 100)` = S127, `overhang_fan_speed = 100` → bump S255) plus a companion no-bump test.

**Incidental finding — `fan_min_speed` is a declaration-only key today**, the disposition map Authoring rule 1 prohibits. Measured this session: the only non-doc occurrences in the tree are its `[config.schema]` row in `modules/core-modules/part-cooling/part-cooling.toml` and a schema-default assertion in `modules/core-modules/part-cooling/tests/cooling_config_schema_tdd.rs`; `modules/core-modules/part-cooling/src/lib.rs` never reads it (`PartCooling::from_config` reads four keys, and `fan_min_speed` is not among them). Packet 253's AC-3 gives it its first real consumer — the `S ≤ T < F` interpolation floor and the `reduce_fan_stop_start_freq` off-branch base. Not a new ticket: it closes with the packet.

**Sequencing:** packet 253 remains `draft` and is implemented off-map by `/swarm`, per the map's execution override (packet *authoring* is in-map, packeted *implementation* is not). This ticket adds no packet and writes no module code.

**Verification:** no code changed, so no build or test gate applies. Doc-only edits to `packet.spec.md` and `design.md`.
