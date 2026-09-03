---
status: superseded
packet: 239-support-independent-layer-z
superseded_by:
  - 239a-anchored-host-seams
  - 239b-anchored-wit-contract
  - 239c-support-layer-height-producer
depends_on: 238c-support-renderer-flow-interfaces
task_ids:
  - TASK-399
  - TASK-400
  - TASK-401
  - TASK-402
  - TASK-403
  - TASK-404
  - TASK-405
  - TASK-406
  - TASK-407
  - TASK-408
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 239-support-independent-layer-z

## SUPERSEDED (2026-08-28)

This packet is **superseded** by `239a-anchored-host-seams`, `239b-anchored-wit-contract`, and
`239c-support-layer-height-producer`. Do not implement it. The plan of record for the split,
with the nine verified findings that motivated it, is
`docs/specs/support-independent-layer-z-split-plan.md`.

Why it was split, in one paragraph: a `/swarm` run measured this packet's central premise as
false and its true scope as XL. The `requirements.md` claim that "the anchored-event substrate
(packets 219–223) already carries everything needed … but two verified blockers keep it out of
the production slice path" does not hold. Blocker 1 is **refuted** — `is_same_z_entity`'s two
consumers are exact complements, so the executor routing partition is already total and off-grid
entities do reach the anchored collection (recorded here as correction PC-1). Blocker 2 holds,
but it is not the whole gap: there is also no anchored input seam on `PipelineConfig`, no
emission representation for an anchored row, **no production producer of `AnchoredEntity`
anywhere**, and the anchored WIT records in `ir-types.wit` are orphaned — referenced by zero
interfaces, zero worlds, and zero lift/lower glue. Consequently this packet's Human Validation
Gate was unsatisfiable in substance, not merely blocked on missing reference files: a real slice
would have produced zero anchored entities, making the gate artifacts structurally identical to
today's output.

The content below is retained as history, including correction PC-1 and the `visual_debug.rs`
scope note added during that run. Its acceptance criteria live on, redistributed: AC-1..AC-4 and
AC-N1..AC-N3 in `239a-anchored-host-seams` (as integration-level truths), and AC-5's measure-first
`height_delta` protocol plus this gate in `239c-support-layer-height-producer`, where a producer
finally makes them checkable. Trap T11 still stands wherever it is quoted.

## Goal

Make support-layer Z independent of object-layer Z: support print rows at planes off the
object-layer grid are routed through the anchored-event substrate, executed by the
production pipeline (which today never invokes `execute_per_layer_with_anchored_events`),
emitted at their declared Z, and flow-scaled correctly — with the `height_delta` risk
measured before any emitter change is made.

## Scope Boundaries

Host-side execution and emission wiring only: exact-Z entity routing in
`crates/slicer-runtime/src/layer_executor.rs`, first production enablement of the anchored
executor in `crates/slicer-runtime/src/pipeline.rs`, synthesis of support-only print rows
from anchored collections, and the measure-first `height_delta` protocol in
`crates/slicer-gcode/src/emit.rs`. Planner/renderer fidelity is 238b/238c; raft geometry is
240; the AGG rasterizer is 241; no new config keys are declared here. The §9 enabled-feature
Orca references are HUMAN-owned regenerations — this packet gates on their existence under
`tmp/` and never generates them.

## Prerequisites and Blockers

- Depends on: `238c-support-renderer-flow-interfaces` — FORWARD DEPENDENCY: 238c is
  authored (status `draft`) ahead of this packet in the queue rooted at
  `236-support-stabilization`; this packet must not activate until 238c reaches
  `implemented`. Chain position: 236 → 237 → 238a → 238b → 238c → 239 (239/240/241 are
  mutually independent).
- Unblocks: `242-support-family-orca-closure`.
- Activation blockers: none beyond the dependency above; `[BLOCK]`-tagged questions live in
  `design.md` §Open Questions. The Human Validation Gate additionally cannot sign off until
  the §9 references (generated with `independent_support_layer_height` ENABLED) exist under
  `tmp/`.

## Acceptance Criteria

- **AC-1 (off-grid production emission).** Given a live execution plan carrying a
  same-z-support anchored entity whose declared planar Z (e.g. `3000` units = 0.3 mm) lies
  strictly between two global-layer rows (0.2 mm, 0.4 mm), **when** the production pipeline
  executes, **then** the emitted layer stream contains a print row whose `z` equals 0.3 mm
  ordered between the 0.2 mm and 0.4 mm rows, and the entity's paths appear exactly once in
  the stream, on that row. |
  `cargo test -p slicer-runtime --test integration -- offgrid_support_entity_emits_intermediate_print_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2 (exact-Z routing totality).** Given any same-z-support anchored entity,
  **when** the executor routes it, **then** exactly one route applies: appended into the
  anchor layer's ordinary `ordered_entities` if and only if
  `|z − mm_to_units(anchor.z)| ≤ AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`,
  otherwise carried as anchored work emitted at its declared Z — never dropped and never
  present on both routes. |
  `cargo test -p slicer-runtime --test integration -- every_same_z_support_entity_routes_exactly_once --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3 (off-grid determinism, invariant 12).** Given the same plan with off-grid entities
  at distinct intermediate planes, **when** execution runs serially and again with
  `force_parallel`, **then** both runs commit identical event order and an identical
  print-Z row sequence. |
  `cargo test -p slicer-runtime --test integration -- offgrid_interleaving_identical_serial_and_parallel --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4 (Z-spanning atomicity, invariant 9).** Given a `ZSpanning` same-z-support anchored
  entity spanning several object layers, **when** the production pipeline executes,
  **then** the entity emits as one atomic contiguous block covering its inclusive span —
  never split into per-object-layer fragments. |
  `cargo test -p slicer-runtime --test integration -- zspanning_support_entity_emits_atomic_single_block --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5 (measure-first `height_delta` verdict).** Given the Step 5 measurement dispatch
  over `crates/slicer-gcode/src/emit.rs`, **when** the verdict test runs, **then** the
  emitted extrusion `E` for an off-grid pass satisfies exactly the recorded branch —
  verdict `MISSCALE_FIXED`: `e == distance · point.width · (declared_plane_delta_z) ·
  point.flow_factor / filament_area`; verdict `CONSISTENT`: the current per-row formula
  asserted equal within `1e-6` on measured constants — and the verdict is recorded in
  `docs/07_implementation_status.md` under TASK-403 before the fix/no-fix decision. |
  `cargo test -p slicer-gcode --lib -- height_delta_verdict_matches_measured_behavior --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6 (guest freshness before slice-level evidence).** Given any fixture-slice evidence
  run (human-gate artifacts, differential comparisons), **when** it is produced, **then**
  `cargo xtask build-guests --check` exited `0` immediately beforehand (exit `1` = rebuild
  required, exit `3` = `wasm-tools` infrastructure error — never grep for `STALE:`). |
  `cargo xtask build-guests --check && echo FRESH`

Every AC names exact symbols, paths, counts, or output fragments and ends with its own
runnable command. Commands that dump more than 200 successful lines are tee'd to
`target/test-output.log` with a non-zero matched-count guard (invariant 16). No AC command
targets `slicer-core`, so E6 feature-gated blindness does not apply to this packet's suite;
the rule still governs any broad reconciliation run.

## Negative Test Cases

- **AC-N1 (invariant 6 preserved — on-grid ordinary ordering).** Given an on-grid
  same-z-support entity whose declared plane equals its anchor layer's Z within
  `COORDINATE_TOLERANCE_UNITS`, **when** the new routing executes, **then** the entity is
  still appended into the anchor layer's ordinary `ordered_entities` (model layer 0 first,
  anchored collection before its anchor's model event, same-z entity inside the anchor
  model event) and no separate intermediate print row is created for it. |
  `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2 (no grid collapse).** Given an off-grid same-z-support entity whose plane differs
  from every global-layer Z beyond tolerance, **when** the executor routes it, **then** it
  is rejected from ordinary merging — no layer's `ordered_entities` contains it — and it
  appears only on its declared-Z anchored row. |
  `cargo test -p slicer-runtime --test integration -- offgrid_entity_never_merged_into_grid_layers --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3 (support-disabled silence, invariant 13).** Given support disabled with the new
  pipeline path enabled, **when** a slice runs, **then** zero anchored support rows and
  zero `;TYPE:Support` output are produced. |
  `cargo test -p slicer-runtime --test integration -- support_disabled_pipeline_emits_nothing --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- Primary targeted proof: AC-1's command (production off-grid emission).
- Guest freshness gate before any slice-level evidence: `cargo xtask build-guests --check && echo FRESH`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - governing plan; §12 brief
  "239-support-independent-layer-z", §6 invariants (6, 8, 9, 12, 13, 15, 16), §7 E1–E9,
  §8 human gate, §9 reference regeneration, §13 traps T1/T4/T5/T11, §14 rules. Bounded
  ranged reads.
- `docs/specs/support-parity-gap-register.md` - row G-02 (destination
  **239-support-independent-layer-z**); direct range read around G-02 only.
- `docs/08_coordinate_system.md` - porting checklist; consult via the coord-system
  constraint in `design.md`, do not full-read.

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` - TASK-399..TASK-408 registered at packet-owned
  closure (Step 10) - `rg -q 'TASK-399' docs/07_implementation_status.md && rg -q 'TASK-408' docs/07_implementation_status.md`
- `docs/specs/support-parity-gap-register.md` - G-02 row flipped to closed/implemented with
  destination `239-support-independent-layer-z` at closure - `rg -q 'G-02' docs/specs/support-parity-gap-register.md && rg -q '239-support-independent-layer-z' docs/specs/support-parity-gap-register.md`
- No IR/WIT/schema/manifest/SDK contract changes: `AnchoredEntity`,
  `OrderedEventCollection`, `LayerCollectionIR`, and `ExecutionPlan` shapes are consumed
  as-is. If a conditional step must add a struct field, THAT step owns the struct-literal
  blast radius and the matching `docs/02_ir_schemas.md` section edit plus grep in the same
  step — recorded as a step-level obligation in `implementation-plan.md`, not packet-level
  doc debt.

## Human Validation Gate

Blocking per plan §8. Sign-off is impossible until BOTH artifact sets exist.

Precondition — fresh §9 references (HUMAN-generated with `independent_support_layer_height`
ENABLED; this packet never generates them):

- `tmp/p239-orca-ref-tree-independent.gcode`
- `tmp/p239-orca-ref-normal-independent.gcode`

Existence gate: `test -f tmp/p239-orca-ref-tree-independent.gcode && test -f tmp/p239-orca-ref-normal-independent.gcode && echo REFS-PRESENT`

Trap T11 stands: the current references were sliced with the feature DISABLED, so they
cannot measure G-02; the "Orca 205 vs PnP 150 print-Z" figure is VOID and is never
requoted anywhere in this packet.

Packet artifacts (regenerate before inspection; freshness gate AC-6 precedes each):

- `tmp/p239-support-indep-tree.gcode` — `cargo run --bin pnp_cli --release -- slice
  --input crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --output
  tmp/p239-support-indep-tree.gcode` with `tmp/support-family-config-tree-matched.json`.
- `tmp/p239-support-indep-normal.gcode` — same fixture with
  `tmp/support-family-config-normal-matched.json`.
- Visual-debug bundle for this packet's boundary: intermediate support print rows +
  ordinary rows side by side (`pnp_cli visual-debug`), stored under `tmp/vd-p239/`.

Checklist (each answered with layer, tap, verdict in writing; E2 — inspection only):

- [ ] Termination: support reaches the plate/model beneath its overhangs on both families.
- [ ] Coverage: demanded overhang regions carry support on the fixture.
- [ ] Collision freedom: no support intersects model walls at any print row, including new
      intermediate rows.
- [ ] Interfaces: roofs/floors sit carved out of the body at interface pitch on their rows.
- [ ] Matched-height comparison (REQUIRED for this packet): the enabled-feature PnP slice
      vs the fresh references — distinct print-Z rows exceed the object-layer count where
      finer support pitch is demanded; Z sequences interleave monotonically; row placement
      differences vs the reference are recorded as measured deltas (behavioral parity bar;
      exact toolpath identity is out of scope per plan §15).
- [ ] Block counts vs fresh references: `;TYPE:Support` / `;TYPE:Support interface` counts
      recorded for both families against `tmp/p239-orca-ref-*-independent.gcode`.

Sign-off: `_date_ _verdict_` (packet may not flip to `status: implemented` without it).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `generate_support_layers`: inspect (do NOT assume) how support-only print Z rows are produced when `independent_support_layer_height` is enabled; return the insertion predicate and the Z-value source for support rows between object layers.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — support-layer insertion points gated on `independent_support_layer_height`: confirm where enabled-feature support layers enter the print Z sequence relative to object layers.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — confirm the `independent_support_layer_height` declaration (type and default) as ground truth for the matched profiles used at the human gate.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `_extrude` (the width × height × length flow product feeding E) as the comparison target for the AC-5 measurement verdict.

Citation policy (E7): canonical behaviour is cited by file + function only, never line
number, and only what a delegated dispatch actually returned. The current references were
sliced with `independent_support_layer_height` DISABLED; no canonical behaviour may be
asserted from them, and the "Orca 205 vs PnP 150 print-Z" figure is VOID (trap T11) —
never requote it.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
