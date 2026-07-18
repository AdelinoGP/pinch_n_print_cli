# Packet 111 — Closure Log

Packet: `111_arachne-beading-strategy-stack`
Closed: 2026-07-03

This log records the accumulated implementation notes from Steps 2-7 (all
shipped and green prior to this closure step) plus Step 8's own findings.
Per the packet's "Packet Completion Gate", this documents decisions already
made during implementation — it does not re-decide anything.

## 1. No OrcaSlicer ground-truth fixtures exist

Confirmed by recon at packet start: no gtest suite, hardcoded assertions, or
worked numeric examples exist anywhere in `OrcaSlicerDocumented/` for any
`BeadingStrategy` class. ALL golden JSON fixtures across Steps 2, 3, 4, 6, 7
(`distributed_10_thickness.json`, `redistribute_outer_consistent.json`,
`widening_thin_wedge.json`, `limited_cap_boundary.json`,
`factory_orca_reference.json`) were derived analytically by hand from each
strategy's documented C++ algorithm (via per-step SUMMARY/LOCATIONS
sub-agent dispatches), independently and BEFORE the corresponding Rust
implementation was finalized (true TDD RED phase) — not transcribed from a
recorded OrcaSlicer run, and not captured by executing this codebase's own
not-yet-written code.

## 2. [SUPERSEDED — see §2a] Four deliberate, design.md-locked divergences from literal upstream C++ semantics — RESOLVED post-closure

This section originally recorded four accepted simplifications shipped at
first closure. Following an explicit user directive ("OrcaSlicer parity is
the current main goal, deferring features is not acceptable, fix all
deviations now"), three of the four were rewritten to faithfully match
upstream's exact algorithms, and the fourth was partially resolved. Original
text is struck through below for audit-trail purposes; see §2a for the
current, resolved state.

- ~~`RedistributeBeadingStrategy` (Step 3): uses post-hoc clamping instead of
  upstream's reduced-thickness recursive call.~~ **RESOLVED** — now ports
  upstream's exact mechanism.
- ~~`WideningBeadingStrategy` (Step 4): always emits a single FIXED
  `min_bead_width` bead below `min_input_width`.~~ **RESOLVED** — now ports
  upstream's exact 3-way branch (`thickness < optimal_width` gate, then
  `thickness >= min_input_width` sub-gate).
- ~~`OuterWallInsetBeadingStrategy` (Step 5): offsets BOTH
  `toolpath_locations[0]` and `toolpath_locations[last]`.~~ **RESOLVED** —
  now single-sided (`toolpath_locations[0]` only), clamped to `thickness/2`.
- ~~`BeadingStrategyFactory::create_stack` (Step 7): wraps all 5 layers
  UNCONDITIONALLY.~~ **PARTIALLY RESOLVED** — `OuterWallInsetBeadingStrategy`
  is now conditionally wrapped exactly as upstream (`outer_wall_offset !=
  0.0`). `WideningBeadingStrategy` remains unconditionally wrapped — see §2a
  for why this one specific gap could not be closed without expanding the
  packet's registered config-key surface.

## 2a. Post-closure OrcaSlicer parity pass — what changed and what remains

Rewritten to true upstream parity (verbatim C++ fetched and translated
exactly, not summarized):

- **`RedistributeBeadingStrategy`**: constructor now takes
  `(parent, optimal_width_outer: f64, minimum_variable_line_ratio: f64)`
  (was a single `optimal_width` param). `compute` now recurses into the
  parent at *reduced* thickness/bead-count
  (`thickness - 2*optimal_width_outer`, `bead_count - 2`, when both are
  positive) and prepends/appends fresh outer beads around that result,
  rather than clamping the parent's full-`bead_count` output post-hoc.
  `bead_count == 0` or `thickness < minimum_variable_line_ratio *
  optimal_width_outer` now correctly produces an EMPTY beading (matches
  upstream `RedistributeBeadingStrategy.cpp:99-141`).
- **`WideningBeadingStrategy`**: constructor now takes
  `(parent, optimal_width: f64, min_input_width: f64, min_output_width:
  f64)` — an explicit `optimal_width` field was added (upstream inherits
  this via C++ base-class copy-construction from `parent`; Rust has no
  equivalent, so it's an explicit constructor param instead) and the field
  formerly named `min_bead_width` is renamed `min_output_width` to match
  upstream's actual field name. `compute` is now a genuine 3-way branch:
  `thickness >= optimal_width` → full delegation; `min_input_width <=
  thickness < optimal_width` → single bead of `thickness.max(min_output_width)`
  (NOT a fixed width), `left_over = 0`; `thickness < min_input_width` →
  EMPTY `bead_widths`, `left_over = thickness`. `optimal_bead_count` and
  `get_transition_thickness` are also now ported exactly (matches upstream
  `WideningBeadingStrategy.cpp:48-91`). **Packet's own AC-4 wording was
  corrected** (see §7) — the original AC-4 literally required "NOT empty,"
  which directly contradicted true upstream behavior for `thickness <
  min_input_width`.
- **`OuterWallInsetBeadingStrategy`**: `compute` now offsets ONLY
  `toolpath_locations[0]`, clamped to `thickness / 2.0` — the opposite end
  is never touched (matches upstream `OuterWallInsetBeadingStrategy.cpp:
  69-92` exactly, including its non-zero-width-bead recount before the
  `< 2` early return). **Packet's own AC-5 wording was corrected** (see §7)
  — the original AC-5 literally required both ends to shift, which
  contradicted upstream's single-sided design.
- **`BeadingStrategyFactory::create_stack`**: `OuterWallInsetBeadingStrategy`
  is now wrapped conditionally (`if params.outer_wall_offset != 0.0`), an
  exact match for upstream's `if (outer_wall_offset != 0)` gate
  (`BeadingStrategyFactory.cpp:50-97`). Two tests now cover both branches:
  `factory_stack_composition_order` (nonzero offset → full 5-layer chain)
  and `factory_stack_composition_order_skips_outer_wall_inset_when_offset_zero`
  (zero offset → 4-layer chain, `OuterWallInset` absent).

**Two residual, DOCUMENTED scope gaps remain** (not fixed — closing them
requires expanding this packet's registered config-key surface beyond its
own T-218 scope, which is a packet-owner decision, not something a
mechanical parity fix can resolve on its own):

1. **`WideningBeadingStrategy` remains unconditionally wrapped.** Upstream
   gates it on a `print_thin_walls` boolean. This packet's T-218 scope
   registers 11 named keys and none of them is that boolean. Closing this
   would mean registering a 12th config key (new docs entry + new manifest
   `[config.schema.*]` block), which goes beyond "port the existing 11
   keys' consuming logic faithfully" into "add a new key" — left open for
   the packet owner to decide.
2. **`preferred_bead_width_outer`/`preferred_bead_width_inner` merged into
   one `optimal_width` key.** Upstream's factory takes two distinct width
   configs (selecting between them based on `max_bead_count <= 2`); this
   packet's T-218 scope registers only a single `optimal_width` key, so
   `BeadingFactoryParams::optimal_width` now serves both roles. Same
   reasoning as above — fixing this needs a new config key, out of a pure
   parity-fix's scope.

## 2b. Both residual scope gaps closed (second follow-up, explicit user directive)

§2a left two scope gaps open, both requiring new config keys beyond this
packet's original T-218 11-key surface. Per explicit follow-up user
direction ("close both gaps now, do not write them off"), both are now
closed with two NEW registered config keys (bringing the packet's total to
13):

- **`detect_thin_wall`** (bool, default `false`) — registered under
  upstream's REAL `PrintConfig.cpp` option name (not the internal Arachne
  parameter name `print_thin_walls`, which is what it's threaded through as
  inside `BeadingStrategyFactory::create_stack`), confirmed via a LOCATIONS
  dispatch: `coBool`, default `false`, label "Detect thin wall"
  (`PrintConfig.cpp:6299-6305`). `WideningBeadingStrategy` is now wrapped
  into the composition stack ONLY when this is `true` — closing the last
  unconditional-wrap gap. Default `false` means `Widening` is now correctly
  ABSENT from the stack by default, a genuine behavior change from every
  prior state of this packet (Widening was unconditionally present in both
  the original ship and the §2a parity-only fix).
- **`preferred_bead_width_outer`** (float, default `4000`, slicer units) —
  the width `RedistributeBeadingStrategy`'s `optimal_width_outer` parameter
  uses UNCONDITIONALLY, and the base width `DistributedBeadingStrategy`/
  `WideningBeadingStrategy` use INSTEAD OF the existing `optimal_width` key
  when `max_bead_count <= 2` (`effective_optimal_width = max_bead_count <=
  2 ? preferred_bead_width_outer : optimal_width`, matching upstream's
  `preferred_bead_width_outer`/`preferred_bead_width_inner` split exactly —
  `optimal_width`'s docs entry was updated to clarify its refined role as
  upstream's `preferred_bead_width_inner`).

Four new tests prove the closure is real (not just documentation):
`factory_stack_composition_order` (both flags set → full 5-layer chain),
`factory_stack_composition_order_default_skips_both_optional_layers`
(`Default()` → 3-layer chain, `Widening` AND `OuterWallInset` both absent —
note this is a NEW default composition, narrower than §2a's 4-layer
default), `factory_stack_composition_order_widening_only_when_thin_walls_true`
(`detect_thin_wall=true` alone → 4-layer chain with `Widening` present, no
`OuterWallInset`), `factory_max_bead_count_le_2_selects_preferred_bead_width_outer`
(`max_bead_count=2`, distinct `optimal_width`/`preferred_bead_width_outer`
values → proves the conditional selection actually flows through to
`Redistribute`'s output, not just that it compiles).

`factory_orca_reference.json`'s existing expected values were NOT changed —
the fixture's `max_bead_count=3` is not `<= 2`, so `effective_optimal_width`
still resolves to the (unchanged) `optimal_width=4000`; the fixture's
`params` object gained the two new required fields
(`print_thin_walls: true`, `preferred_bead_width_outer: 4000.0`, chosen
equal to `optimal_width` specifically to keep the already-verified
numeric expectations valid without re-derivation) but the arithmetic is
byte-identical to before.

**No scope gaps remain.** All four strategies plus the factory's
composition logic (both conditional layers, the outer/inner width split)
now faithfully port upstream OrcaSlicer's `Arachne::BeadingStrategy` stack.

Also fixed as part of this pass (not upstream-parity issues, but real bugs
found during a subsequent audit):

- **`BeadingFactoryParams::default()`'s stale values.** Confirmed by
  independent review: `min_input_width`/`min_output_width`/
  `distribution_count`/`default_transition_length` had silently diverged
  from this packet's OWN already-registered `docs/15_config_keys_reference.md`
  defaults (Step 8 corrected the docs; `factory.rs`'s `Default` impl was
  never updated to match, since Step 8 wasn't permitted to touch
  `factory.rs`). Now aligned exactly: `min_input_width=1000.0` (was
  `340.0`), `min_output_width=4000.0` (was `200.0`, and the field itself
  renamed from `min_bead_width`), `distribution_count=1` (was `3`),
  `default_transition_length=4000.0` (was `5000.0`). A new field,
  `minimum_variable_line_ratio: f64 = 0.5`, was added (no registered config
  key backs it — same "internal Arachne parameter, not yet exposed"
  treatment as `default_transition_length`/`transition_filter_dist`).
- **`BeadingFactoryParams` now derives `serde` directly** (§4 below,
  superseded).

## 3. `LimitedBeadingStrategy`'s sentinel-count generalization

Upstream OrcaSlicer's `optimal_bead_count` only ever clamps overflow to
exactly `max_bead_count + 1` (a single sentinel pair). This port generalizes
to `sentinel_count = bead_count - max_bead_count` for arbitrary excess,
reducing to upstream's exact shape when `sentinel_count == 1`. This is a
faithful extension, not a behavioral divergence.

## 4. [RESOLVED] `BeadingFactoryParams` now derives `serde` directly

Originally, `design.md`'s resolved Open Question called for
`#[derive(Serialize, Deserialize)]` on `BeadingFactoryParams`, but `serde`
was only a `[dev-dependencies]` entry, so Step 7 shipped a test-local
`ParamsFixture` mirror struct + `From` conversion as a workaround instead.

Per the user's explicit "fix all deviations now" directive, this was
resolved properly rather than left as a permanent workaround: `serde` was
promoted from `[dev-dependencies]` to `[dependencies]` in
`crates/slicer-core/Cargo.toml`, and `BeadingFactoryParams` now derives
`Serialize, Deserialize` directly alongside its existing `Debug, Clone,
PartialEq`. The `ParamsFixture`/`From` workaround was deleted from
`tests/beading/factory.rs`; the fixture's `params` object deserializes
straight into `BeadingFactoryParams`.

## 5. Config key defaults — PrintConfig.cpp LOCATIONS dispatch findings

The Step 8 OrcaSlicer LOCATIONS dispatch found 8 of the 11 keys registered
in `PrintConfig.cpp`, of which 6 are `coPercent` (percentage of nozzle
diameter) rather than fixed-length values. This required correcting 4 of
the packet's originally-suggested slicer-unit defaults, which had mistaken
the raw percentage number for a slicer-unit value:

| Key | Packet-suggested | Corrected | Basis |
|---|---|---|---|
| `min_feature_size` | 25 | **1000** | OrcaSlicer `coPercent` default 25% × 0.4mm nozzle = 0.1mm = 1000 units |
| `min_bead_width` | 200 | **4000** | OrcaSlicer `coPercent` default 100% × 0.4mm nozzle = 0.4mm = 4000 units |
| `wall_transition_filter_deviation` | 200 | **1000** | OrcaSlicer `coPercent` default 25% × 0.4mm nozzle = 0.1mm = 1000 units |
| `initial_layer_min_bead_width` | 850 | **3400** | OrcaSlicer `coPercent` default 85% × 0.4mm nozzle = 0.34mm = 3400 units |

The remaining 7 keys matched the packet's original suggestion or had no
literal upstream constant to correct against:

- `wall_transition_length` = 4000 (OrcaSlicer `coPercent` 100% × 0.4mm = 4000 units — matches).
- `wall_transition_angle` = 10.0 degrees (OrcaSlicer `coFloat` default 10.0 — matches exactly).
- `wall_distribution_count` = 1 (OrcaSlicer `coInt` default 1 — matches exactly).
- `min_length_factor` = 0.5 (dimensionless ratio) — kept as-is; the sub-agent flagged that a
  `PrintConfig.cpp` key found under this exact name registers as a `coFloat` in mm rather than a
  ratio, which may be a distinct UI-facing option sharing the name rather than the internal Arachne
  algorithm parameter T-227 targets (`removeSmallLines`'s `min_length_factor * min_width`). Kept the
  packet's ratio-semantics default pending T-227's own confirmation.
- `outer_wall_offset` = 0 — not a `PrintConfig.cpp` option at all (internal Arachne `coord_t` param); 0 (disabled) kept as-is.
- `max_bead_count` = 9 — not a `PrintConfig.cpp` option (upstream computes it internally as `2 * inset_count`, capped, in `WallToolPaths.cpp`); no literal constant to correct against, kept as-is.
- `optimal_width` = 4000 (0.4mm) — not a `PrintConfig.cpp` option (upstream sets it internally from `preferred_bead_width_outer`/`preferred_bead_width_inner`); matches this codebase's common `line_width` default, kept as-is.

The 4 corrected defaults and their derivation (against an assumed 0.4mm
reference nozzle diameter, matching this codebase's existing `line_width`
default and the packet's own `optimal_width = 4000` suggestion) are recorded
in `docs/15_config_keys_reference.md` "Arachne beading strategy stack
(packet 111)" and mirrored in `arachne-perimeters.toml`.

**`min_feature_size` / `min_input_width` naming-mapping conclusion:**
confirmed via the OrcaSlicer tooltip text ("Minimum thickness of thin
features; thinner is not printed, thicker is widened to min wall width")
that `min_feature_size` maps to `WideningBeadingStrategy`'s internal
`min_input_width` field — both are the sub-threshold-detection cutoff below
which a region is too narrow for the wrapped strategy's normal bead
distribution and gets widened instead. This mapping is noted in
`min_feature_size`'s docs entry for P112's benefit.

Additional field mappings established for P112's wiring convenience (not
explicitly requested by the packet digest but derived from the same
sub-agent dispatch and the `distributed.rs`/`widening.rs` field reads):

- `optimal_width` → `DistributedBeadingStrategy::optimal_width` (verbatim name match); also now threaded into `WideningBeadingStrategy::optimal_width` and `RedistributeBeadingStrategy::optimal_width_outer` (see §2a).
- `wall_transition_length` → `DistributedBeadingStrategy::default_transition_length` (reserved field, not yet read by `compute`).
- `wall_transition_filter_deviation` → `DistributedBeadingStrategy::transition_filter_dist` (reserved field, not yet read by `compute`).
- `wall_distribution_count` → `DistributedBeadingStrategy::distribution_count`.
- `min_bead_width` → `WideningBeadingStrategy::min_output_width` (renamed from `min_bead_width` during the §2a parity pass to match upstream's actual field name — verbatim match now).

**`BeadingFactoryParams::default()` alignment (post-closure fix, §2a):** the
struct's `Default` impl originally used stale placeholder values that
silently diverged from the table above's registered defaults
(`min_input_width=340.0` vs. registered `min_feature_size=1000`,
`min_output_width=200.0` vs. registered `min_bead_width=4000`,
`distribution_count=3` vs. registered `wall_distribution_count=1`,
`default_transition_length=5000.0` vs. registered `wall_transition_length=
4000`) — a real bug, not a documented deviation, since nothing intended
this mismatch. Now corrected to match this table exactly.

## 6. D-9 closure rationale (condensed)

`LimitedBeadingStrategy::compute` retains zero-width sentinel beads
internally — an OrcaSlicer bookkeeping mechanism that downstream centrality
propagation reads to keep bead-index alignment — but they are stripped
before external output via a separate `compute_and_strip` entry point.
This is necessary because this codebase's `WallLoop` WIT-boundary type
invariant requires `bead_widths.iter().all(|&w| w > 0.0)`, and stripping
avoids a contract change at that boundary. `compute` (raw, unstripped)
remains available for invariant testing (AC-6/AC-N2); `compute_and_strip` is
the entry point P112's wire-up (T-230) is expected to call. Full rationale
recorded as `D-111-ARACHNE-SENTINEL-STRIP` in `docs/DEVIATION_LOG.md`;
D-9 itself (the original roadmap-level decision to strip rather than
coordinate zero-width sentinels with downstream infill modules) remains a
roadmap-only ID in `docs/specs/perimeter-modules-orca-parity-roadmap.md`,
not duplicated into the deviation log.
