# Requirements: 239-support-independent-layer-z

## Packet Metadata

- Grouped task IDs: `TASK-399` through `TASK-408`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

G-02 (`docs/specs/support-parity-gap-register.md`, destination
**239-support-independent-layer-z**): PnP has no support-layer Z independent of
object-layer Z. The anchored-event substrate (packets 219–223) already carries everything
needed — planar and Z-spanning entity contracts, deterministic committed event ordering,
and a dedicated executor entry point — but the feature is still absent from the production
slice path. There is exactly ONE blocker, not two:

1. **CORRECTED (re-verified live 2026-08-28; the 2026-08-22 characterization is REFUTED
   and is preserved here only as history).** The original claim was that
   `is_same_z_entity` (`crates/slicer-runtime/src/layer_executor.rs`) "matches nothing, so
   [an off-grid entity] is silently excluded from ordinary merging" — i.e. that off-grid
   entities fall through a routing gap. **That is wrong.** Direct read of
   `crates/slicer-runtime/src/layer_executor.rs` finds exactly three references to
   `is_same_z_entity`: its definition, a positive filter inside `append_same_z_entities`,
   and a negated filter (`!is_same_z_entity`) inside
   `execute_anchored_event_collections`. Those two filters are EXACT COMPLEMENTS over a
   single predicate, so the routing partition is ALREADY TOTAL: an on-grid same-z-support
   entity (tolerance match against `mm_to_units(anchor.z)` within
   `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`) takes the ordinary model-layer
   route, and an off-grid entity fails that match, is rejected by the ordinary route, and
   is therefore CAUGHT by the negated filter and lands in the anchored collection. It does
   not vanish at this filter. No routing gap exists to close; what remains here is a
   behavior-neutral clarity refactor (one shared named helper so the two filters cannot
   drift apart) — see `design.md` §Approach 1.
2. `crates/slicer-runtime/src/pipeline.rs` calls only the non-anchored per-layer variants
   (`execute_per_layer_with_events_and_support_tools`,
   `execute_per_layer_with_instrumentation_and_support_tools`);
   `execute_per_layer_with_anchored_events` and
   `execute_per_layer_with_committed_anchored_events` are exercised solely by tests
   (verified 2026-08-22; **re-verified live 2026-08-28 — HOLDS**). A third non-anchored
   call site was found in the same live re-verification:
   `crates/pnp-cli/src/visual_debug.rs` also calls
   `execute_per_layer_with_events_and_support_tools`, so visual-debug output shares the
   same blind spot as the slice path.

   **This is the entire mechanism of the observable defect.** The off-grid same-z support
   entity does reach the anchored collection; that collection is simply never executed,
   because no production call site invokes an anchored executor entry point. Blocker 2
   alone explains "off-grid support never prints"; blocker 1 explains nothing.

A stated-but-unmeasured risk sits in emission: `height_delta`
(`crates/slicer-gcode/src/emit.rs`) is computed per emitted row from neighbouring row Zs
and feeds volumetric E (`distance · width · height_delta · flow_factor / filament_area`);
an off-grid support pass may inherit a wrong height term. The gap register records this as
"stated, not measured" — this packet treats measurement as a gate, not a premise.

The current Orca references were regenerated with `independent_support_layer_height`
DISABLED, so G-02 is a missing canonical feature, not a measurable divergence against them;
the "Orca 205 vs PnP 150 print-Z" figure is VOID (trap T11) and never requoted. Fresh
enabled-feature references are human-owned (plan §9); this packet gates on their existence.

This is one coherent slice because both blockers plus the emission question live on one
path — plan execution → anchored routing → row synthesis → E computation — and partial
fixes produce off-grid entities that are routed but never printed.

## In Scope

- Behavior-neutral consolidation of the same-z route decision in
  `crates/slicer-runtime/src/layer_executor.rs`: on-tolerance planes merge into the anchor
  layer's ordinary entities (invariant 6 preserved); all other same-z-support entities take
  the anchored route and emit at their declared Z. This is already true today (the two
  filters are exact complements); the packet only extracts the decision into one named
  helper consulted by `append_same_z_entities` and `execute_anchored_event_collections` so
  they cannot drift apart. Totality is asserted, not created.
- First production enablement of `execute_per_layer_with_anchored_events` /
  `execute_per_layer_with_committed_anchored_events` inside
  `crates/slicer-runtime/src/pipeline.rs` (both instrumented and non-instrumented paths)
  and `crates/pnp-cli/src/visual_debug.rs` (the third non-anchored call site, so
  visual-debug output matches sliced output), including synthesis of support-only
  intermediate print rows from committed anchored collections so they survive
  finalization/postpass into G-code.
- Measure-first protocol for `height_delta` in `crates/slicer-gcode/src/emit.rs`: a
  dispatched measurement with a recorded verdict (MISSCALE_FIXED / CONSISTENT), then a
  conditional fix ONLY if the verdict demands it.
- Regression protection for the existing substrate:
  `anchored_event_ordering` (bare wrapper in `crates/slicer-runtime/tests/integration/main.rs`),
  accounting, parallel-determinism, and Z-span validation suites stay green throughout.
- New integration tests in `crates/slicer-runtime/tests/integration/` for off-grid
  production emission, exact-once routing, determinism, atomicity, grid-collapse rejection,
  and support-disabled silence; one verdict-locking unit test in `crates/slicer-gcode`.
- Human Validation Gate artifacts and matched-height inspection against fresh §9
  references under `tmp/p239-*`.

## Out of Scope

- Raft geometry (signed negative prefix layers, `com.core.raft-default`, `claim:raft-fill`)
  — packet 240.
- AGG rasterizer port and area-propagation semantics — packet 241.
- Planner algorithm fidelity (smoothing, collision/avoidance keying, tree styles) — packet
  238b; renderer flow/density/interface semantics — packet 238c.
- Declaring new config keys or manifests; `independent_support_layer_height` itself is not
  added here unless a conditional step's measured verdict requires a carrier field (that
  step owns the full blast radius; see `design.md`).
- Generating Orca reference G-code — human-owned (plan §9); this packet only gates on the
  files' existence.
- G-14 (`ERR_MALFORMED_LAYER_MARKER` noise) and G-15 (inherited literal debt) — recorded
  noise, never re-diagnosed (trap T10).
- Exact Orca toolpath identity (plan §15): behavioral parity with measured deltas is the bar.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - governing queue/brief;
  §12 "239-support-independent-layer-z", §6 invariants, §7 evidence standards, §8 gate,
  §9 references, §13 traps T1/T4/T5/T11. Bounded ranged reads (~755 lines total file;
  read sections, never full).
- `docs/specs/support-parity-gap-register.md` - G-02 row; direct range read around G-02.
- `docs/spec_packets/238c-support-renderer-flow-interfaces/packet.spec.md` - predecessor
  contract; delegated SUMMARY of its exports ledger only.
- `.agents/doc-index.md` - doc map when an unfamiliar doc must be located; never full-read
  canonical docs speculatively.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`: how enabled-feature support-only print Z rows are produced between object layers (predicate + Z source), inspected at implementation time.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — where support layers gated on `independent_support_layer_height` enter the print Z sequence relative to object layers.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the `independent_support_layer_height` declaration (type/default) behind the fresh reference profiles.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_extrude` flow product as the comparison target for the AC-5 measurement verdict.

No canonical behaviour is asserted as fact anywhere in this packet beyond what these
delegated dispatches return; current references were sliced with the feature DISABLED and
prove nothing about this feature (trap T11).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (off-grid production emission between grid rows), `AC-2` (exact-once
  routing totality), `AC-3` (serial/parallel determinism of the interleaved stream),
  `AC-4` (Z-spanning atomicity through production), `AC-5` (measure-first `height_delta`
  verdict locked by test), `AC-6` (guest freshness precedes slice-level evidence).
- Negative: `AC-N1` (on-grid entities still merge ordinarily — invariant 6 regression),
  `AC-N2` (off-grid entities never collapse onto grid layers), `AC-N3` (support disabled ⇒
  zero anchored rows/output — invariant 13).
- Cross-packet impact: enables `242-support-family-orca-closure`'s matched-height axis;
  consumes 238c's renderer fidelity outputs once implemented; independent of 240/241.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test integration -- offgrid_support_entity_emits_intermediate_print_z --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1 primary contract | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p slicer-runtime --test integration -- every_same_z_support_entity_routes_exactly_once --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 routing totality | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_interleaving_identical_serial_and_parallel --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 determinism | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- zspanning_support_entity_emits_atomic_single_block --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 atomicity | FACT pass/fail |
| `cargo test -p slicer-gcode --lib -- height_delta_verdict_matches_measured_behavior --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 verdict lock | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1 invariant-6 regression net (pre-existing suite) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_entity_never_merged_into_grid_layers --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2 no grid collapse | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- support_disabled_pipeline_emits_nothing --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3 invariant 13 | FACT pass/fail |
| `cargo xtask build-guests --check && echo FRESH` | Guest freshness gate before Steps 3, 4, 7, and 9 evidence commands (exit 0 fresh / 1 stale / 3 infra; never grep for `STALE:`) | exit code + FRESH token |
| `cargo check --workspace --all-targets` | type gate across all targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | exit code |

Every command satisfies invariant 16: explicit `--exact` names or an asserted non-zero
matched count in the same run. No `--workspace` test run is required for closure; if one
becomes required, it goes through `cargo xtask test --summary`.

## Step Completion Expectations

- Steps are strictly ordered: measurement (Step 5) precedes any emitter edit decision
  (Step 6); pipeline enablement (Steps 3–4) precedes fixture-slice evidence.
- The TASK-403 verdict string (`MISSCALE_FIXED` or `CONSISTENT`) recorded under
  `docs/07_implementation_status.md` is the single source of truth for whether Step 6 runs
  the fix branch or the assert-only branch; later steps read that record, never re-derive.
- Guest freshness (AC-6) is re-checked immediately before every fixture-slice artifact run;
  a stale result invalidates prior evidence.

## Context Discipline Notes

- `crates/slicer-runtime/src/layer_executor.rs` is very long (thousands of lines): ranged
  reads only, around the named symbols (`is_same_z_entity`, `append_same_z_entities`,
  `execute_per_layer_with_committed_anchored_events`); never full-load.
- `OrcaSlicerDocumented/` reads are delegation-only (snippet above). Verify the checkout by
  direct listing first — gitignored trees vanish from globs (trap T1).
- Never re-run a test invocation to see more output; read `target/test-output.log`.
