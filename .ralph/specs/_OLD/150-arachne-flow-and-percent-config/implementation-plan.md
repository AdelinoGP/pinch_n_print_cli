# Implementation Plan: 150-arachne-flow-and-percent-config

## Execution Rules

- One atomic step at a time.
- Each step maps back to the audit gaps G4/G5/G6 (backlog source
  `docs/18_arachne_parity_audit.md`; no `docs/07` task IDs).
- TDD first (the red gap test already exists), then implementation, then the
  narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by
  `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are the
  budget contract for the step.

## Steps

### Step 1: Extend the config-value WIT variant + adapter (boundary already resolved)

- Gaps: G6.
- Objective: add `percent-val` / `float-or-percent-val` to `variant config-value`
  in `config.wit` and the matching 1:1 arms in `slicer-macros`
  `__slicer_adapt_config`; rebuild guests so bindings regenerate.
- Precondition: packet active. The boundary question is already RESOLVED (a WIT
  variant mirrors `ConfigValue`; see design.md Open Questions) — no re-scoping
  dispatch needed.
- Postcondition: workspace + guests build with the new variant arms present;
  no exhaustive-match compile errors.
- Files allowed to read: `crates/slicer-schema/wit/deps/config.wit:1-20`,
  `crates/slicer-macros/src/lib.rs:585-605`.
- Files allowed to edit (≤3): `crates/slicer-schema/wit/deps/config.wit`,
  `crates/slicer-macros/src/lib.rs`.
- Files out-of-bounds: generated bindgen output under `*/wit-guest/`; the module.
- Expected sub-agent dispatches:
  - "Run `cargo build --tests -p slicer-macros -p slicer-ir`; FACT pass/fail or
    SNIPPETS (compile error) on fail."
  - "Run `cargo xtask build-guests`; FACT success + any failing guest names."
- Context cost: `M`.
- Authoritative docs: `CLAUDE.md` §"WIT/Type Changes Checklist" (load directly).
- OrcaSlicer refs: none.
- Verification: `cargo build --tests` clean; `cargo xtask build-guests` succeeds.
- Exit condition: all guests rebuild clean with the extended variant.

### Step 2: Add the percent / float_or_percent config type (schema + IR)

- Gaps: G6.
- Objective: `ConfigValue` gains `Percent`/`FloatOrPercent`; `ConfigView` gains
  `get_abs_value(key, base)`; the type set accepts the new type strings.
- Precondition: Step 1 landed (WIT variant + adapter arms exist).
- Postcondition: AC-2 + AC-N2 unit tests pass; `get_float` on a percent value
  returns `None` (no coercion).
- Files allowed to read: `crates/slicer-ir/src/slice_ir.rs:681-850` (ConfigValue
  + accessors); `crates/slicer-schema/src/lib.rs:347-355`.
- Files allowed to edit (≤3): `crates/slicer-ir/src/slice_ir.rs`,
  `crates/slicer-schema/src/lib.rs`.
- Files out-of-bounds: the manifests (Step 3), the module (Step 4).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-ir --lib config -- percent`; FACT pass/fail or
    SNIPPETS on fail."
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md` (delegate ConfigValue section);
  `docs/03_wit_and_manifest.md` (delegate: is `VALID_CONFIG_TYPES` enforced?).
- OrcaSlicer refs: `PrintConfig.cpp` percent defaults — delegate; never load.
- Verification: `cargo test -p slicer-ir --lib config -- percent`.
- Exit condition: AC-2 and AC-N2 green.

### Step 3: Wire live manifest validation + retype the three keys

- Gaps: G6.
- Objective: `parse_config_field_entry` accepts `percent`/`float_or_percent` and
  parses `<n>%` defaults (rejecting malformed, AC-N1); `is_numeric_field_type`
  updated; `arachne-perimeters.toml` retypes `min_width_top_surface` (300%),
  `min_feature_size` (25%), `wall_transition_length` (100%).
- Precondition: Step 2 landed.
- Postcondition: AC-1 (G6 gap test) and AC-N1 green.
- Files allowed to read: `crates/slicer-scheduler/src/manifest.rs:1060-1160`,
  `crates/slicer-scheduler/src/config_resolution.rs:175-190`,
  `arachne-perimeters.toml:38-108,257-263`.
- Files allowed to edit (≤3): `crates/slicer-scheduler/src/manifest.rs`,
  `crates/slicer-scheduler/src/config_resolution.rs`,
  `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`.
- Files out-of-bounds: the module source (Step 4).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity_gaps --
    arachne_parity_pipeline_percent_config_type_for_arachne_keys --exact`; FACT
    pass/fail."
  - "Run `cargo test -p slicer-scheduler --test scheduler_contract -- config_percent_type`;
    FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/03_wit_and_manifest.md` (delegate config-type
  section).
- OrcaSlicer refs: `PrintConfig.cpp:1498-1511,7169-7178,7217-7226` — delegate.
- Verification: the two dispatches above.
- Exit condition: AC-1 + AC-N1 green.
- Before editing: audit `arachne_params_from_config` for existing `get_float`
  readers of the three retyped keys and migrate them to `get_abs_value` (Locked
  Assumption) — else they silently zero.

### Step 4: Register layer_height/nozzle_diameter and wire Flow spacing

- Gaps: G4.
- Objective: register `layer_height` (default 0.2) and `nozzle_diameter`
  (default 0.4 mm) in `arachne-perimeters.toml`; read both in the module; feed
  `line_width_to_spacing(width, layer_height, nozzle_diameter)` output as the
  bead width to the beading pipeline (unit-consistent — convert layer_height to
  the width's unit before the subtraction).
- Precondition: Step 3 landed (keys resolve).
- Postcondition: AC-3 green — perimeter_index 0↔1 gap ≈0.3571 mm ±0.02.
- Files allowed to read: `arachne-perimeters/src/lib.rs:108-225`,
  `crates/slicer-core/src/flow.rs:42-65`, `docs/08_coordinate_system.md`.
- Files allowed to edit (≤3): `modules/core-modules/arachne-perimeters/src/lib.rs`,
  `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`.
- Files out-of-bounds: `crates/slicer-core/src/beading/**` (delegate any trace
  if AC-3 fails).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity_gaps --
    arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact`; FACT
    pass/fail or SNIPPETS (observed gap value) on fail."
  - "Run `cargo xtask build-guests --check`; FACT clean/STALE."
- Context cost: `M`.
- Authoritative docs: `docs/08_coordinate_system.md` (load directly, short).
- OrcaSlicer refs: `PerimeterGenerator.cpp:2129,2172-2173` — delegate.
- Verification: the G4 gap test dispatch; then AC-6 lock dispatch immediately
  after (this step moves wall positions).
- Exit condition: AC-3 green AND all 14 `arachne_parity.rs` locks still green.

### Step 5: Replace the thick_bridges 1.0 stub with the round-section factor

- Gaps: G5.
- Objective: `bridging_flow` returns the OrcaSlicer round-cross-section factor
  (`π·dmr²/(4·w·h)`, `dmr` per the Orca formula) for `thick_bridges==true`;
  module call site passes nozzle diameter, bead width, layer height.
- Precondition: Step 4 landed (`nozzle_diameter` registered/read).
- Postcondition: AC-4 green — ≥1 `is_bridge` vertex flow_factor differs from 1.0
  by >0.05.
- Files allowed to read: `crates/slicer-core/src/flow.rs:80-138`,
  `arachne-perimeters/src/lib.rs:342-440`.
- Files allowed to edit (≤3): `crates/slicer-core/src/flow.rs`,
  `modules/core-modules/arachne-perimeters/src/lib.rs`.
- Files out-of-bounds: `OrcaSlicerDocumented/**` (delegate the formula).
- Expected sub-agent dispatches:
  - "From `OrcaSlicerDocumented/src/libslic3r/Flow.{hpp,cpp}`, give the exact
    `dmr` formula and thick-bridge flow-factor computation. SUMMARY ≤200 words
    or one ≤30-line SNIPPET."
  - "Run `cargo test -p slicer-runtime --test arachne_parity_gaps --
    arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one
    --exact`; FACT pass/fail."
  - "Run `cargo test -p slicer-core --lib flow`; FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: none beyond the Orca dispatch.
- OrcaSlicer refs: `Flow.hpp:106`, `Flow.cpp` bridging_flow, `LayerRegion.cpp:31-50,135`.
- Verification: the G5 gap test + `slicer-core flow` unit tests.
- Exit condition: AC-4 green; existing flow unit tests updated + green.

### Step 6: Register nozzle_diameter on classic-perimeters (adjacent dead-read fix)

- Gaps: none (adjacent, shares nozzle plumbing).
- Objective: add `[config.schema.nozzle_diameter]` to `classic-perimeters.toml`
  so the existing `src/lib.rs:183-186` read receives the value instead of always
  falling back to `inner_wall_line_width`.
- Precondition: Step 5 landed (keeps the packet's nozzle work contiguous).
- Postcondition: AC-5 green.
- Files allowed to read: `classic-perimeters/src/lib.rs:175-190`,
  `classic-perimeters.toml` (config.schema section).
- Files allowed to edit (≤3): `modules/core-modules/classic-perimeters/classic-perimeters.toml`,
  `modules/core-modules/classic-perimeters/src/lib.rs` (test only, if the lock
  test lives in-module).
- Files out-of-bounds: the arachne module.
- Expected sub-agent dispatches:
  - "Run `cargo test -p classic-perimeters --lib -- nozzle_diameter`; FACT
    pass/fail."
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-5 dispatch. Flag the behavior-change risk (profiles where
  nozzle ≠ line width now behave differently) in the packet notes.
- Exit condition: AC-5 green; behavior-change risk recorded.

### Step 7: Docs, deviation closure, and guest freshness

- Gaps: G4/G5/G6 bookkeeping.
- Objective: update docs/03 (new type), docs/15 (retyped + new keys), docs/18
  (mark G4/G5/G6 closed), close D-105/D-104g/D-104h in DEVIATION_LOG.md.
- Precondition: Steps 2–6 green.
- Postcondition: every Doc Impact grep in `packet.spec.md` returns a hit.
- Files allowed to read: the four docs (ranged / delegated).
- Files allowed to edit (≤3 per touch; docs only): `docs/03_wit_and_manifest.md`,
  `docs/15_config_keys_reference.md`, `docs/18_arachne_parity_audit.md`,
  `docs/DEVIATION_LOG.md` (edit sequentially, not all at once).
- Files out-of-bounds: none new.
- Expected sub-agent dispatches:
  - "Run each Doc Impact grep from packet.spec.md; FACT all-hit / list misses."
- Context cost: `S`.
- Authoritative docs: the four above.
- OrcaSlicer refs: none.
- Verification: the Doc Impact grep suite.
- Exit condition: all Doc Impact greps hit; `cargo xtask build-guests --check`
  clean.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | config.wit variant + adapter arm + guest rebuild |
| Step 2 | M | percent type across IR + schema |
| Step 3 | M | live validation + manifest retype |
| Step 4 | M | flow spacing; moves wall positions (AC-6 watch) |
| Step 5 | M | bridging factor + Orca dmr dispatch |
| Step 6 | S | classic manifest one-liner + test |
| Step 7 | S | docs + deviation closure |

Aggregate: `M` (concurrent peak is one M step at a time; no L step).

## Packet Completion Gate

- All 7 steps complete; every exit condition met.
- AC-1..AC-6 and AC-N1/AC-N2 dispatched and PASS.
- 14 `arachne_parity.rs` locks green; G4/G5/G6 gap tests green; the other 7 gap
  tests still red (unchanged — they belong to packets 151/152).
- `cargo check --workspace --all-targets` and
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` clean.
- Doc Impact greps all hit; D-105/D-104g/D-104h marked closed.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Run the workspace gate via a sub-agent:
  `cargo xtask test --summary --workspace` — FACT PASS/FAIL + failing-test list
  only (never absorb full output).
- Record the classic-perimeters behavior-change risk (AC-5) and any shifted
  self-captured baseline explicitly before `status: implemented`.
- Confirm implementer peak context stayed < 70%; log if not.
