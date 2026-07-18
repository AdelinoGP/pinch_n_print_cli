# Design: 151-arachne-winding-wallcount-dispatch

## Controlling Code Paths

- Primary code paths:
  - Wall-count wiring + params: `modules/core-modules/arachne-perimeters/src/lib.rs`
    `arachne_params_from_config` (`:108-225`) — currently reads `max_bead_count`
    (`:119-122`, falling back to `unwrap_or(defaults.max_bead_count)` = 9) with
    no `wall_count` read anywhere in the file. The key is registered
    (`arachne-perimeters.toml:159`, `default = 9`) but `ConfigView` never merges
    schema defaults, so unset ⇒ `get_int` → `None` — which is exactly what makes
    the precedence rule implementable: explicit `Some` wins, `None` →
    `max_bead_count = 2 × wall_count`. Also the home for G7 overhang reads (the
    overhang keys `overhang_reverse` / `overhang_reverse_internal_only` /
    `detect_overhang_wall` are registered — toml `:230-244` — but have ZERO
    reads in `lib.rs`; the nearby `let _ = only_one_wall_top;` discard at
    `:305-306` is packet 152's key, not an overhang key) and G9 tolerance
    wiring.
  - Winding (G1/G7): emission block `lib.rs:467-497` (sets `perimeter_index =
    line.inset_idx`, `path`); the `path` comes from
    `extrusion_line_to_extrusion_path3d` (`crates/slicer-ir/src/slice_ir.rs:1783-1792`),
    a direct junction-order copy with NO winding normalization. G1/G7 add a
    signed-area check + conditional `path.points` reversal (contour to requested
    winding; holes opposite). A comment at `lib.rs:~526` about closing even lines
    and unioning "to normalize winding" is worth the implementer confirming.
  - G9 tolerances: `crates/slicer-core/src/arachne/pipeline.rs` `ArachneParams`
    fields (`:145-159`) + `Default` (`:180-208`) —
    `smallest_line_segment_squared` (0.0025 mm²) /
    `allowed_error_distance_squared` (0.000025 mm²), both mm², currently sourced
    from `meshfix_*`. The module already reads `meshfix_maximum_resolution/deviation`
    (`lib.rs:190-198`); G9 adds `wall_maximum_*` reads and feeds the squared mm.
  - G8 dispatch: `crates/slicer-wasm-host/src/execution_plan_live.rs:208-219`
    (extracts `wall_generator` from `config_source` via
    `WALL_GENERATOR_CONFIG_KEY`, calls
    `dedup_same_claim_modules_with_wall_generator`) +
    `crates/slicer-scheduler/src/execution_plan.rs:246-271` (test wrapper
    `:246-251`, production entry `:259-265`, private impl `:267+`; classic id
    const `CLASSIC_PERIMETERS_MODULE_ID = "com.core.classic-perimeters"` at
    `:201`). Thread a `spiral_vase` bool the same way; force classic when true.
    The G8 red test's source probe (`arachne_parity_gaps.rs:491-509`) accepts
    "spiral" in EITHER `execution_plan.rs` OR `run.rs` (`||`), so the
    scheduler-side change alone satisfies it.
- Neighboring tests: `crates/slicer-runtime/tests/arachne_parity_gaps.rs` —
  existing test bodies are arbiters, never modify them; Step 1 APPENDS the
  packet-authored `arachne_parity_wall_count_wires_max_bead_count` test to this
  file (append-only). `arachne_parity.rs` (15 locks).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference
  Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **G9 unit exception:** `ArachneParams` stores mm² directly (existing defaults
  0.0025 = 0.05², 0.000025 = 0.005²). `wall_maximum_resolution` (0.5 mm) is
  squared to 0.25 mm² with NO ÷100 — the ÷100 rule applies to toolpath
  coordinates, not to these mm-based scalar params. Do not double-convert.
- **Winding has no existing helper** — the module must introduce a signed-area
  (shoelace) test + point-order reversal; there is no `make_ccw`/`reverse` to
  reuse in `slicer-ir` or `slicer-core`. Keep it local to the module's emission.
- **ADR-0011 conformance (wall sequencing):** ADR-0011 locks inter-wall
  *reordering* logic (e.g. InnerOuterInner grouping) into shared
  `slicer-perimeter-utils`. Winding is point-traversal *direction within one
  loop*, not inter-wall order, so the module-local shoelace normalization does
  not contradict it. If `classic-perimeters` later needs the same reversal
  (Orca applies `overhang_reverse` to both generators), promote the helper to
  `slicer-perimeter-utils` then — out of scope here.
- **ADR-0035 conformance (faithful emission):** the winding normalization is
  the port of OrcaSlicer's wall reorientation
  (`PerimeterGenerator.cpp:527-545` winding rule; `:58-98,422-429` odd-layer
  reversal) applied after junction emission — it is not a supplemental
  invention on top of the faithful `generateJunctions`/`connectJunctions`
  surface. Likewise the G9 rewiring only changes where
  `simplifyToolPaths`' tolerances come from (config vs `meshfix_*`), never the
  simplify algorithm itself; the replace-vs-supplement `[FWD]` dispatch must
  confirm the Orca sourcing before wiring.

## Code Change Surface

- Selected approach: all wall-count/winding/tolerance logic in the module's
  `arachne_params_from_config` + emission block; G8 in the scheduler/loader
  selection path (one bool threaded like `wall_generator`).
- Exact changes:
  - `arachne-perimeters/src/lib.rs`: read `wall_count` (→ `max_bead_count =
    2 × wall_count` when `max_bead_count` is unset; explicit `Some` wins),
    `wall_direction`, `only_one_wall_first_layer`, the overhang keys,
    `wall_maximum_resolution/deviation`; add winding normalization at `:467-497`;
    odd-layer reversal for G7. Also author a `#[cfg(test)] mod tests` containing
    `wall_maximum_resolution_wired` (AC-6b) — no in-file test module exists
    today, so the `--lib` filter would otherwise match nothing and false-pass.
  - `arachne-perimeters.toml`: register `wall_count`, `wall_direction` (enum),
    `only_one_wall_first_layer`, `overhang_reverse_threshold` (`float_or_percent`),
    `wall_maximum_resolution`, `wall_maximum_deviation`.
  - `crates/slicer-runtime/tests/arachne_parity_gaps.rs`: APPEND the
    packet-authored `arachne_parity_wall_count_wires_max_bead_count` test
    (AC-1); existing test bodies are immutable arbiters.
  - `execution_plan_live.rs`: extract `spiral_vase` from `config_source`; pass to
    dedup. `execution_plan.rs`: `dedup_same_claim_modules(..., spiral_vase)` forces
    `classic-perimeters` for the `perimeter-generator` claim when spiral is active.
    (Adding the literal string "spiral" to `execution_plan.rs` also satisfies the
    G8 red test's substring probe — but the real behavior is the point.)
  - `crates/slicer-scheduler/tests/contract/spiral_vase_arachne_dispatch_tdd.rs`
    (new) + `mod spiral_vase_arachne_dispatch_tdd;` registration in
    `tests/contract/main.rs` (AC-N1; the aggregated binary is
    `scheduler_contract` per `slicer-scheduler/Cargo.toml` `[[test]]`).
- Rejected alternatives: (a) putting spiral logic in the module (rejected — the
  audit and Orca both gate at selection time, before the module runs); (b) a new
  winding type in `slicer-ir` (rejected — a local shoelace test is enough and
  avoids a shared-type change + guest rebuild churn beyond the manifest).

## Files in Scope (read + edit)

Primary:

- `modules/core-modules/arachne-perimeters/src/lib.rs` — the four in-module gaps
  (wall_count, winding, first-layer, overhang) + G9 wiring.
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — six key
  registrations.
- `crates/slicer-scheduler/src/execution_plan.rs` + (secondary)
  `crates/slicer-wasm-host/src/execution_plan_live.rs` — G8 spiral threading.

Test files (packet-authored additions only):

- `crates/slicer-runtime/tests/arachne_parity_gaps.rs` — append the AC-1 test.
- `crates/slicer-scheduler/tests/contract/spiral_vase_arachne_dispatch_tdd.rs`
  (new) + `crates/slicer-scheduler/tests/contract/main.rs` (one `mod` line).

The packet exceeds ≤3 primary files because G8 legitimately lives in a different
crate from the module gaps; it is a small, isolated two-file change with no
overlap. The test additions are per-step (see implementation-plan) and stay
within each step's ≤3-edit cap.

## Read-Only Context

- `crates/slicer-runtime/tests/arachne_parity_gaps.rs` — the five target test
  bodies (G1 `:166-194`, G2 `:207-228`, G7 `:440-474`, G8 `:491-509`, G9
  `:527-544`) — purpose: exact assertions.
- `crates/slicer-core/src/arachne/pipeline.rs:145-208` — `ArachneParams` +
  defaults — purpose: G9 field names/units.
- `crates/slicer-scheduler/src/execution_plan.rs:180-275` — the `wall_generator`
  precedent — purpose: mirror it for spiral.
- `crates/slicer-runtime/tests/arachne_parity.rs` — grep only (>800 lines) for
  any lock asserting a wall COUNT — purpose: AC-7 baseline validation.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate.
- `crates/slicer-core/src/beading/**` — the `{0,1,2,3,4}` anomaly is fully
  explained by the unread `wall_count`; do not spelunk the beading engine.
- `target/`, `Cargo.lock`, generated `*/wit-guest/` bindings — never load.

## Expected Sub-Agent Dispatches

- "From `PerimeterGenerator.cpp:527-545`, how is contour vs hole winding decided
  under `wall_direction`? SUMMARY ≤200 words." — G1/G7 winding rule.
- "Run `cargo test -p slicer-runtime --test arachne_parity_gaps -- <name>
  --exact`; FACT pass/fail or SNIPPETS on fail." — per gap.
- "Run `cargo test -p slicer-runtime --test arachne_parity`; FACT pass/fail +
  failing test names." — AC-7.
- "Run `cargo xtask build-guests --check`; FACT clean/STALE." — after edits.

## Data and Contract Notes

- IR/manifest: six new manifest keys; `overhang_reverse_threshold` uses packet
  150's `float_or_percent` type (hard dependency).
- WIT boundary: none in this packet (no `config.wit`/`common.wit` edit; the
  percent TYPE arrived in 150). Manifest + module + scheduler only.
- Determinism/scheduler: G8 changes claim resolution — must stay deterministic
  (spiral bool is a pure config read; no ordering nondeterminism introduced).

## Locked Assumptions and Invariants

- `max_bead_count = 2 × wall_count` (Orca `WallToolPaths.cpp:525`) is the wiring
  contract; the emitted distinct-index count on a solid square equals `wall_count`.
- The 15 `arachne_parity.rs` locks are invariant (AC-7); wall-count shifts are
  validated, not rebaselined.
- Default `wall_direction = counter_clockwise` must reproduce the prior
  (pre-packet) winding so absent-key configs are unchanged (AC-N2).
- G8's spiral fallback fires ONLY when spiral is active (AC-N1) — it must not
  become an unconditional classic override.

## Risks and Tradeoffs

- **wall_count wiring changes emitted counts across many existing tests.** Highest
  blast radius; mitigated by AC-7 and the "validate, don't rebaseline" rule.
- **Winding reversal touching every contour** could interact with seam placement
  / downstream path optimization — verify no seam/regression lock flips (AC-7,
  AC-N2).
- **G8 in the live loader** is exercised only on the wasm path; the contract test
  (AC-N1) plus the substring-probe red test (AC-5) both must pass so the behavior
  is real, not just string-satisfying.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` — the module changes (four gaps in one file), kept at
  M by delegating cargo runs and the Orca winding query.
- Highest-risk dispatch: the Orca winding-rule query — must return SUMMARY, never
  the file.

## Open Questions

- `[FWD]` Does G9's `wall_maximum_resolution` REPLACE the `meshfix_*`-sourced
  tolerances, or supplement them (Orca has both keys)? Resolve from the
  `WallToolPaths.cpp:487-503,702-719` dispatch during the G9 step; the red test
  only needs registration, so this affects AC-6b's wiring, not AC-6.
- `[FWD]` Exact spiral-vase config key/source on the raw config path —
  `spiral_vase` is registered on the arachne manifest (toml `:288`), and the
  selection path reads the RAW `config_source` (the same map `wall_generator`
  is read from via `WALL_GENERATOR_CONFIG_KEY` at
  `execution_plan_live.rs:208-219`), so a user-set `spiral_vase` will be
  present there; absent ⇒ treat spiral as inactive. Remaining question for the
  G8 step: the raw `ConfigValue` variant to match (`Bool` vs `String`) —
  confirm against how the gap/contract tests build their config fixtures.
