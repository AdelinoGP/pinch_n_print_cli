# 111 — Convert the part-cooling fan scale to percent 0–100, and make `overhang_fan_speed` absolute

Type: task
Status: open
Assignee: —
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
