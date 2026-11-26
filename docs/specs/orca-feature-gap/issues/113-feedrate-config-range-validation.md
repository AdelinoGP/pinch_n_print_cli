# 113 — Add range validation to `FeedrateConfig`

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-04)
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 22, from the 2026-09-01 grilling ruling **Q6(b)**: *"add range
validation"* to `FeedrateConfig`'s fields — *"Canonical declares `min = 10`
here; the struct has no bounds machinery at all."* The ruling explicitly flags
its own gap: **canonical min/max per field were not derived in that session.**
Deriving them is the bulk of this ticket.

Verified in-tree (2026-09-02), `crates/slicer-ir/src/feedrate.rs`:

- `FeedrateConfig` — every field is a bare `pub <name>_speed: f32`. No bounds,
  no validation fn.
- The file's only functions are `default`, `read_speed`, `as_number`, and
  `from_raw_config`. `read_speed` coerces `Float` / `Int` / non-percent
  `FloatOrPercent` and returns `None` otherwise — it does not reject zero,
  negative, or absurd values.
- The registration table is **`SPEED_KEYS`** (26 entries), *not* `FEEDRATE_KEYS`
  — the name the grilling row uses does not exist in this tree. Ticket 22's
  preflight caught the same fiction inside packet 267; treat other symbol names
  in `key-correction-inventory.md` as unverified until greped.

Decide and execute:

1. **Derive canonical min/max per feedrate key** from OrcaSlicer's
   `PrintConfig.cpp` declarations (cite by file + function, never line numbers;
   the checkout is the sibling `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`).
   26 keys is the working set; expect several to have a min and no max.
2. **Where validation runs.** `FeedrateConfig` is host-side and typed, so it does
   not go through `ConfigBoundsIndex` (which serves manifest-declared module
   keys). Decide whether feedrate bounds join that machinery, get their own
   check, or ride `from_raw_config`'s parse path — and what the error type is.
3. **What an out-of-range value does** — reject the slice, or clamp with a warn?
   Rejecting is consistent with the manifest-bounds behaviour; clamping is
   friendlier to imported profiles. Pick one and say why.
4. **Any key whose canonical bound the port cannot honour** becomes a recorded
   divergence with rationale, not a silently dropped bound.

Relationships (re-derive status at point of use, do not trust this line):

- **Ticket 108** (`wipe_tower_speed` → `wipe_tower_max_purge_speed`) is where this
  surfaced — canonical declares `min = 10` on that key and the port's typed arm
  cannot express it. Q6(a) ruled the rename; Q6(b) is this ticket. Whether 108
  waits on this or records the bound as deferred is 108's call.
- **Ticket 109** decides whether the renamed `support_ironing_speed` joins
  `SPEED_KEYS`; if it does, it inherits whatever this ticket builds.

Not a queue key; changes no queue count.

## Answer

**Resolved 2026-09-04 — decided and implemented directly, no packet. The premise needed correcting first: canonical declares these bounds but does not enforce them, so this port's enforcement is a deliberate divergence rather than parity.**

### The premise correction

Q6(b) reads "canonical declares `min = 10` here", which is true and misleading. Measured against the designated oracle this session: **`def->min` / `def->max` are GUI spinner hints with no runtime effect.** `ConfigBase::set_deserialize` / `set_deserialize_raw` (`Config.cpp`) never consult them; the only speed check in `Print::validate` (`Print.cpp`) compares against `machine_max_speed_x/y` and **the whole block is commented out**, with the note *"Orca: disable the speed check for now as we don't cap the speed"*; and the only consumer of those fields outside `PrintConfig.cpp` is the GUI spinner in `Field.cpp`. OrcaSlicer will load a negative speed from a file or CLI without complaint.

So this ticket is not closing a parity gap. It is choosing to be **stricter than canonical**, which the user ruled (2026-09-04) is the right call, and it is recorded as a divergence rather than dressed up as parity.

### Why strictness is warranted here

`read_speed` (`crates/slicer-ir/src/feedrate.rs`) coerces any `Float` / `Int` / non-percent `FloatOrPercent` and returns `None` otherwise — it rejects nothing. `DefaultGCodeEmitter::resolve_feedrate` (`crates/slicer-gcode/src/emit.rs`) then computes `base_speed * 60.0 * clamped_factor` with **no floor on `base_speed`** (the `clamp(0.05, 5.0)` guards the *factor*, not the speed). A configured `outer_wall_speed = 0` therefore emits `F0`, and a negative emits a negative feedrate, straight into the G-code.

### 1. Canonical min/max per key

Derived from `PrintConfigDef::init_fff_params` (`PrintConfig.cpp`), landed as `SPEED_BOUNDS` (`crates/slicer-ir/src/feedrate.rs`), positionally pinned to `SPEED_KEYS` by a const assertion.

**No canonical speed key declares a `max` — not one of the 26.** Mins are `1` for the ordinary speeds, `0` for the keys where canonical assigns zero a meaning, and `10` for `wipe_tower_max_purge_speed` (the bound ticket 108 recorded as inexpressible; it is now expressed).

Zero-as-sentinel keys, per canonical's own tooltips: `overhang_1_4_speed`..`overhang_4_4_speed` (0 = use the wall speed), `skirt_speed` (0 = default layer extrusion speed), `travel_speed_z` (0 = use `travel_speed`), and `wipe_speed` (canonical min 0). This port adds `filament_ironing_speed` (0 = use `ironing_speed`, per `resolve_feedrate`); canonical declares min 1 with an unset default, a shape this port cannot express, so the sentinel wins — recorded in item 4.

Three keys **have no canonical counterpart at all** — `thin_wall_speed`, `bottom_surface_speed`, `prime_tower_speed` appear nowhere in `PrintConfig.cpp`. Their bounds are this port's own choice, taken as `1.0` to match the family they sit in.

Six keys are `coFloatOrPercent` / `coFloatsOrPercents` with a `ratio_over` base: `internal_bridge_speed` over `bridge_speed`, `initial_layer_travel_speed` and `wipe_speed` over `travel_speed`, and the four `overhang_*_speed` over `outer_wall_speed`. See item 4.

### 2. Where validation runs

**It joins `ConfigBoundsIndex`** — because half of it was already there, inconsistently. Measured: **14 of the 26 `SPEED_KEYS` were already bounds-checked**, not by design but because some module manifest happened to declare a twin of the key; the other 12 (`bottom_surface_speed`, `support_interface_speed`, `skirt_speed`, `wipe_tower_max_purge_speed`, `prime_tower_speed`, `travel_speed`, `travel_speed_z`, `initial_layer_speed`, `initial_layer_infill_speed`, `initial_layer_travel_speed`, `wipe_speed`, `filament_ironing_speed`) were checked by nothing at all. Two sources of truth, one of them accidental.

`ConfigBoundsIndex::from_modules` (`crates/slicer-scheduler/src/config_resolution.rs`) now seeds itself from `slicer_ir::feedrate::speed_bounds()` before walking the module declarations, so a host speed is checked whether or not a module declares it, at the same point and with the same `ConfigResolutionError::OutOfRange` as a manifest key. Bounds still intersect, so a module declaring a stricter range continues to win. The contributor id for the seeded bounds is the pseudo-module `<host [speeds]>`, so the empty-intersection warning can still name where a bound came from.

This also resolves the "> 0 is inexpressible" problem for free: `NumericBounds` is inclusive-only, and canonical's mins are inclusive `1` / `0`, so adopting them needs no exclusivity machinery. `docs/config/host-keys.toml`'s prose `range = "> 0"` is superseded by the canonical `>= 1` for the ordinary speeds; its `skirt_speed = "> 0"` row was **wrong** against canonical, which allows the 0 sentinel.

### 3. Out-of-range behaviour: reject

**Reject the slice** (user ruling, 2026-09-04). Consistent with how this port already treats every module-manifest-declared key — and the alternative would have meant the same value being rejected as a module key and clamped as a host key, which is exactly the inconsistency this ticket exists to remove.

### 4. Bounds the port cannot honour — recorded divergences

- **Enforcement itself is the divergence.** Canonical does not enforce; this port does. Rationale above, recorded in `SPEED_BOUNDS`' doc comment.
- **`max = 300.0` was a PnP invention and is retired.** Twelve module-manifest rows across eight manifests carried it on `SPEED_KEYS` keys with no canonical basis; canonical declares no maximum on any speed. It would have rejected legitimate high-speed profiles — an existing test already exercises `travel_speed = 500.0`. Removed, and pinned by `no_speed_key_carries_an_upper_bound`.
  **Three identical rows were deliberately left alone** — `internal_solid_infill_speed` (`rectilinear-infill`), `support_ironing_speed` (`support-surface-ironing`), `wave_overhang_print_speed` (`wave-overhangs`). They are module-owned speeds outside `FeedrateConfig`, and this session derived no canonical bound for them; changing them on the strength of a neighbouring key's finding is the exact mistake the map's ticket-27 note warns about. Filed as [133](133-retire-invented-speed-maxima-on-module-owned-speeds.md).
- **`filament_ironing_speed` min 0, not canonical's 1.** Canonical declares min 1 with an *unset* default and treats "unset" as the fallback signal; this port has no unset state on an `f32` and uses `0` as the sentinel in `resolve_feedrate`. Adopting min 1 would make the port's own default unrepresentable.
- **Percent-form values for the six `ratio_over` keys are silently ignored, and this ticket does not fix it.** `read_speed` matches `FloatOrPercent { is_percent: false }` only, so a canonical profile spelling `internal_bridge_speed = 150%` falls through to the host default. That default (37.5) happens to equal 150% of `bridge_speed`'s 25, so the two agree by coincidence today and drift the moment `bridge_speed` moves. This is the same family as ticket 128's percent-spelling mismatch and belongs with it, not here — noted on 128 rather than fixed blind.

### Also found

- **`docs/15_config_keys_reference.md` was already stale on HEAD**, independent of this work: `bridge_density`'s max reads `120.0` in the doc and `125.0` in `rectilinear-infill.toml`. Verified by running `gen-config-docs --check` against a stashed tree. The regen in this ticket absorbs that drift; it was not introduced here.
- **A duplicated doc-comment block** sits above `FeedrateConfig::from_raw_config` (the same four paragraphs appear twice). Cosmetic, left alone.
- **Host and module defaults disagree for two speed keys** — `outer_wall_speed` (host 60.0, `classic-perimeters` 30.0) and `top_surface_speed` (host 100.0, `rectilinear-infill` 60.0), plus `inner_wall_speed` declared 45.0 by `classic-perimeters` and 60.0 by `overhang-classifier-default`. That is a defaults question, not a bounds question; not touched here, and not fixed silently.

### Verification

- `cargo clippy -p slicer-ir -p slicer-scheduler --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations.
- `cargo test -p slicer-scheduler --test scheduler_integration` — 80 passed, 10 failed. **All 10 failures are pre-existing** and unrelated: `dag_cli_integration` aborts on a stale `pnp_cli` binary (`pnp-cli-locator`'s staleness guard). Confirmed by stashing this ticket's changes and re-running: the same 10 fail on the baseline tree, with 74 passing instead of 80 — i.e. this ticket adds 6 passing tests and breaks nothing.
- `cargo test -p slicer-ir` — all binaries green.
- Module suites for all eight edited manifests (`classic-perimeters`, `gyroid-infill`, `lightning-infill`, `rectilinear-infill`, `top-surface-ironing`, `traditional-support`, `tree-support`, `wave-overhangs`) — zero failing binaries.
- `cargo xtask gen-config-docs` — regenerated (269 module keys, 55 host keys, 26 Orca deviations).
- `cargo xtask build-guests --check` reported stale after the edit, which is correct and expected: `slicer-ir` sits in every guest's dependency closure. Rebuilt with `cargo xtask build-guests`.

New tests, all in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`: `host_speed_keys_are_bounds_checked_without_any_module_declaring_them`, `zero_is_rejected_for_a_speed_that_has_no_zero_meaning`, `zero_is_accepted_for_the_sentinel_speeds`, `wipe_tower_max_purge_speed_carries_canonical_min_ten`, `no_speed_key_carries_an_upper_bound`, `every_speed_key_is_bounds_checked`.

### Relationships

- **Ticket 108** can now express canonical's `min = 10` on `wipe_tower_max_purge_speed`; it is live in `SPEED_BOUNDS` and pinned by its own test.
- **Ticket 109**: if `support_ironing_speed` joins `SPEED_KEYS`, it inherits this machinery automatically — add a row to `SPEED_BOUNDS` and the const assertion forces it.
- **Ticket 128** gains the percent-form finding above.
