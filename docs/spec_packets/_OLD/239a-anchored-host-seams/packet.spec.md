---
status: implemented
packet: 239a-anchored-host-seams
supersedes: 239-support-independent-layer-z
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

# Packet Contract: 239a-anchored-host-seams

## Goal

Give the host an anchored-entity input seam, switch all three non-anchored
`execute_per_layer*` call sites to `execute_per_layer_with_committed_anchored_events`, and
lower `CommittedLayerEvent::Anchored` collections into ordinary `LayerCollectionIR` rows at
their declared Z — merged against object rows by the canonical `|dz| <= EPSILON` rule — so
off-grid support work survives finalization and postpass into G-code.

## Scope Boundaries

Host-side seam and row synthesis only, inside `crates/slicer-runtime` plus the one
`crates/pnp-cli/src/visual_debug.rs` call site. Rows are synthesized as ordinary
`LayerCollectionIR` and inserted at or before layer finalization, so the `GCodeEmitter` trait and
every `impl` of it are untouched. Measured 2026-08-28 (`rg -n 'impl GCodeEmitter for' crates/`,
`rg -c '\.emit_gcode\(' crates/ -g '*.rs'`): **14** `impl GCodeEmitter for` blocks workspace-wide
and **52** `.emit_gcode(` call sites. Exactly **one** impl lives in
`crates/slicer-gcode/src/emit.rs` (`impl GCodeEmitter for DefaultGCodeEmitter`, the production
one); the other **13 are distributed across test files** in `crates/slicer-runtime/tests/` and
`crates/pnp-cli/tests/` — they are not concentrated in `emit.rs` and cannot be surveyed there.
No WIT, IR, schema, SDK, manifest, or config-key changes; no module
edits; no `crates/slicer-gcode/src/emit.rs` edit. Guest-side transmission of anchored work is
`239b-anchored-wit-contract`; a production producer of `AnchoredEntity` and the
`height_delta` flow verdict are `239c-support-layer-height-producer`.

Also out of scope, stated explicitly so no step tries to build it: **pipeline-level parallel
determinism.** There is no `force_parallel` config key, env var, or `PipelineConfig` field.
`force_parallel` is a positional `bool` parameter of
`execute_anchored_event_collections_with_mode` (`crates/slicer-runtime/src/layer_executor.rs`),
reachable only by calling that executor function directly. AC-3 is scoped to that call, not to a
pipeline knob; threading a parallel-mode selector through `run_pipeline_core` / `PipelineConfig`
is not this packet's work.

## Prerequisites and Blockers

- Depends on: none. This packet is implementable against the tree as it stands.
- Unblocks: `239c-support-layer-height-producer` (host emission of off-grid rows).
- Activation blockers: none. `[FWD]` questions live in `design.md` §Open Questions.

Honest limitation, stated up front and repeated in `requirements.md`: no production code
constructs an `AnchoredEntity` today. Verified 2026-08-28 — the type appears in exactly four
production files (`crates/slicer-ir/src/lib.rs`, `crates/slicer-ir/src/slice_ir.rs`,
`crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-scheduler/src/execution_plan.rs`),
and `rg -n 'AnchoredEntity \{' crates/` minus the `pub struct` definition and the `-> AnchoredEntity {`
fn-signature lines yields **9 literal construction sites, every one of them in a test file**
(`crates/slicer-ir/tests/ir_validation_tdd.rs` ×2,
`crates/slicer-scheduler/tests/integration/capability_derived_anchor_closure.rs` ×2, and one each
in the five `crates/slicer-runtime/tests/integration/anchored_*.rs` files). Zero production
literals. Every acceptance criterion below is therefore an
**integration-level** truth driven by a hand-built `ExecutionPlan` plus an explicit
`PipelineConfig.anchored_entities` payload. No real slice exercises this path until
`239c-support-layer-height-producer` lands a producer. This packet must not claim otherwise,
and its closure must not rest on any fixture-slice artifact.

## Acceptance Criteria

**Test-name convention (load-bearing — read before writing any command below).**
`crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` is mounted by a bare
`mod offgrid_rows_tdd;` line and its functions carry `#[test]` in place (the `pipeline_tdd`
convention). libtest therefore names each of its tests **`offgrid_rows_tdd::<fn>`**, and each
`pipeline_tdd.rs` test **`pipeline_tdd::<fn>`**. Verified against a recorded run of the
`integration` binary, whose lines read `test pipeline_tdd::access_audits_live_path ... ok` and
`test support_family_routing::family_attribution ... ok`. A filter naming the bare function
without its module prefix matches **zero** tests, exits 0, and reads green — the exact failure
the plan doc's numbered invariant 16 was written for. Every filter below therefore carries its
module path. The two exceptions are `anchored_event_ordering` and `anchored_parallel_determinism`,
which are **top-level `#[test] fn` wrappers declared directly in
`crates/slicer-runtime/tests/integration/main.rs`** and so carry no prefix.

- **AC-1 (off-grid row reaches emission).** Given a `PipelineConfig` whose
  `anchored_entities` carries one `same-z-support` entity with
  `AnchoredGeometryContract::Planar { z: 3000 }` (= 0.3 mm at 1 unit = 100 nm) and global
  layers at 0.2 mm and 0.4 mm, **when** `run_pipeline_with_instrumentation` executes, **then**
  the captured `&[LayerCollectionIR]` seen by the emitter contains a row whose `z` equals
  0.3 mm within `1e-6`, ordered strictly between the 0.2 mm and 0.4 mm rows, and the entity's
  paths appear on exactly that one row. |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_support_row_emitted_at_declared_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2 (routing totality end to end).** Given a plan carrying both an on-grid
  `same-z-support` entity (plane within `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`
  = 10 units = 1e-3 mm of its anchor layer) and an off-grid one, **when** the pipeline
  executes, **then** each entity's paths appear exactly once across the whole captured row
  sequence — the on-grid one inside its anchor row's `ordered_entities`, the off-grid one on
  its own declared-Z row — with neither dropped nor duplicated. |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::every_same_z_support_entity_routes_exactly_once --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3 (off-grid determinism — the plan doc's "serial/parallel determinism" invariant).**
  Scoped at the **executor call**, not at the pipeline, because no pipeline-level parallel knob
  exists (see §Scope Boundaries). Given an `ExecutionPlan` with off-grid entities at three
  distinct intermediate planes, **when**
  `slicer_runtime::layer_executor::execute_anchored_event_collections_with_mode(&plan, &entities, false, &module)`
  and the same call with `true` are both run — the idiom
  `crates/slicer-runtime/tests/integration/anchored_parallel_determinism.rs` already uses — and
  each returned collection sequence is lowered through `synthesize_anchored_rows` against the
  same fixed `CommittedLayerEvent::Model` rows, **then** the two resulting row sequences are
  identical in their full `(z, global_layer_index)` pair sequence and in per-row entity ordering. |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_row_order_identical_serial_and_parallel --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4 (Z-spanning atomicity — the plan doc's "Z-spanning atomicity" invariant, under
  ADR-0059).** Scoped at the **executor call plus row synthesis**, not at the pipeline, for the
  same reason as AC-3: `PipelineConfig.anchored_entities` reaches the executor, but no pipeline
  entry point exposes the committed `Vec<CommittedLayerEvent>` in which a Z-spanning block's
  position inside its anchor row can be observed — by the time rows reach the emitter, the
  `Model` row and the block have already been merged into one `LayerCollectionIR`. The test
  therefore drives `execute_anchored_event_collections_with_mode` and lowers the result through
  `synthesize_anchored_rows`, which is the seam the placement rule actually lives on. Given a
  `AnchoredGeometryContract::ZSpanning` `same-z-support` entity spanning
  several object layers, **when** that path executes, **then** its paths appear as **one
  contiguous block inside its anchor layer's ordinary row** — that is, inside the
  `ordered_entities` of the `CommittedLayerEvent::Model` row for the anchor global layer, at that
  layer's normal position, on **no** separate synthesized row — and are never split into
  per-object-layer fragments. This is `docs/adr/0059-support-families-and-anchored-entities.md`'s
  normative clause: "A future atomic Z-spanning entity may extend outside its anchor layer's Z
  interval **while still executing at that layer's normal position**." Atomicity is preserved
  exactly as before; what the ADR fixes is *where* the block lives. |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::zspanning_support_entity_emits_atomic_single_block --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5 (canonical merge rule).** Given an anchored collection whose declared plane sits
  within the merge epsilon of an existing object row, **when** row synthesis runs, **then**
  the two merge into one row (no duplicate Z), and given a plane beyond that epsilon, **then**
  the lower Z emits its own solo row first — matching canonical `GCode::collect_layers_to_print`
  (`GCode.cpp`). |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_row_merge_matches_canonical_epsilon_rule --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6 (empty-collection equivalence).** Given a plan with zero anchored entities, **when**
  the pipeline executes on the new committed path, **then** the captured row sequence is
  element-wise identical in length, `global_layer_index`, and `z` to the sequence produced
  before the switch — the recorded pre-change baseline. |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::support_free_slice_row_sequence_unchanged --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

Every AC names exact fields, paths, counts, or output fragments and ends with its own runnable
command. Each command names one test with `--exact` and asserts a non-zero matched count, so a
subagent returns a FACT rather than a log. `crates/slicer-runtime` has features
`default = ["report"]` and no `required-features` on any test target, so the E6 feature-gated
blindness rule does not apply to this packet's suite.

## Negative Test Cases

- **AC-N1 (the plan doc's "same-Z support in ordinary ordering" invariant preserved).**
  Cited by phrase, not ordinal: in `docs/specs/support-families-anchored-entities-plan.md` §6,
  items 1–14 are an unnumbered semicolon-separated prose parenthetical, and positional item 6 is
  "same-family merge preserving demand IDs", which is a different rule. The invariant this AC
  guards is "same-Z support in ordinary ordering". Given an on-grid `same-z-support` entity whose plane
  equals its anchor layer's Z within `COORDINATE_TOLERANCE_UNITS`, **when** the new path
  executes, **then** it is still appended into the anchor layer's ordinary `ordered_entities`
  in the pre-existing order, and no separate synthesized row is created for it. |
  `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2 (no grid collapse).** Given an off-grid `same-z-support` entity whose plane differs
  from every global-layer Z by more than `COORDINATE_TOLERANCE_UNITS`, **when** the pipeline
  executes, **then** no grid row's `ordered_entities` contains its paths — they appear only on
  the synthesized declared-Z row. |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_entity_never_merged_into_grid_layers --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N3 (support-disabled silence — the plan doc's "support-disabled emits nothing" invariant).** Given `anchored_entities` empty and
  support disabled, **when** a slice runs on the new path, **then** zero synthesized rows are
  produced and the G-code contains no `;TYPE:Support` fragment. |
  `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::support_disabled_pipeline_emits_no_anchored_rows --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- Primary targeted proof: AC-1's command.

## Authoritative Docs

- `docs/specs/support-independent-layer-z-split-plan.md` - the split plan of record;
  findings F1-F4, F9 and the canonical `GCode::collect_layers_to_print` merge rule. Direct
  ranged read.
- `docs/adr/0059-support-families-and-anchored-entities.md` (`Status: accepted`) - the governing
  ADR for anchored-entity ordering and anchor attribution. Direct read of its second paragraph
  only; it is short. This packet conforms to it — see `design.md` §ADR Conformance.
- `docs/specs/support-families-anchored-entities-plan.md` - §6 invariants, cited **by quoted
  phrase, never by ordinal**: "same-Z support in ordinary ordering", "Z-spanning atomicity",
  "serial/parallel determinism", "support-disabled emits nothing", "planar anchored output on
  declared Z", plus the genuinely numbered list item 16 (non-zero matched test count). Items
  1–14 are an unnumbered prose parenthetical, so ordinals there are unresolvable and must not be
  used. Bounded ranged reads; never full-read.
- `docs/08_coordinate_system.md` - consulted via the coord-system constraint in `design.md`;
  do not full-read.

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` - `TASK-399`..`TASK-408` registered at packet-owned
  closure (Step 10). The grep MUST assert the **row form**, not a bare token: `rg -q 'TASK-399'`
  already exits 0 on the unmodified file today, matching the 2026-08-28 renumbering-note prose,
  so a bare-token grep is vacuous. Rows in that file have the shape `- [ ] TASK-NNN <text>` or
  `- [x] TASK-NNN <text>` (verified against the file's existing `TASK-121`..`TASK-331` rows), so
  the assertion is -
  `rg -q '^\s*- \[[ x]\] TASK-399 ' docs/07_implementation_status.md && rg -q '^\s*- \[[ x]\] TASK-408 ' docs/07_implementation_status.md`
  (leading `\s*` because at least one existing row, `TASK-303`, is indented by one space).
- `docs/specs/support-parity-gap-register.md` - row `G-02` destination re-pointed from
  `239-support-independent-layer-z` to this packet's slice, and a new row recording that the
  anchored substrate has no production producer (F5/F6/F7) - `rg -q '239a-anchored-host-seams' docs/specs/support-parity-gap-register.md`
- `docs/specs/support-independent-layer-z-split-plan.md` - queue row 1 gains this packet's
  directory in its `packet dir` column and `closed` in its `status` column. The grep MUST assert
  the **row form**, not a bare token: `rg -q '239a-anchored-host-seams' <that file>` already
  exits 0 on the unmodified tree because §Packet Queue row 1 already carries the slug in its
  `packet slug` column, so a bare-token grep is vacuous exactly as it was for `TASK-399` above.
  The queue table's columns are `| # | packet slug | goal | task ids | depends on | status |
  packet dir |`, and row 1 reads `... | TASK-399..TASK-408 | - | pending | - |` today
  (verified against the file), so the assertion is -
  `rg -q '^\| 1 \|.*\| closed \|.*docs/spec_packets/239a-anchored-host-seams' docs/specs/support-independent-layer-z-split-plan.md`
  which exits 1 on the current tree and can only pass once Step 10 sets both columns.
- No IR/WIT/schema/manifest/SDK contract change. `PipelineConfig` gains one additive host-side
  field; it is not an IR or wire type, carries no schema version, and crosses no component
  boundary. Step 1 owns its struct-literal blast radius in full.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `collect_layers_to_print`: the canonical
  object/support row-merge rule this packet's synthesis must match. A delegated dispatch
  during the 239 swarm run returned: two independent indices, `print_z_min = min(object_layer->print_z, support_layer->print_z)`,
  un-consume whichever side exceeds `print_z_min` by more than EPSILON, merge iff
  `|dz| <= EPSILON`, else the lower one emits solo. Re-verify by dispatch before implementing
  Step 5; do not treat this paragraph as a substitute for inspection.

Citation policy (E7): canonical behaviour is cited by file + function only, never line number,
and only what a delegated dispatch actually returned.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

## Implementation Deviations

Recorded at Step 10 (`TASK-408`). Each was measured during execution; none is an estimate.

**D1 — capability-complete committed ladder.** Steps 6/7/8 as written specified switching the
three non-anchored call sites directly to `execute_per_layer_with_committed_anchored_events`.
Measured during execution: that function is a composition layer whose body calls
`execute_per_layer_with_events`, which hardcodes `SupportToolSelection::default()`,
`&NoopInstrumentation`, and `cancel_flag = None`; no `execute_per_layer_*` variant accepted
`anchored_entities` together with any capability parameter. A literal switch would therefore have
silently dropped production support-tool selection, all pipeline instrumentation, and
cancellation. Step 4 instead added two additive rungs in
`crates/slicer-runtime/src/layer_executor.rs` —
`execute_per_layer_with_committed_anchored_events_and_support_tools` (`pub`) and
`execute_per_layer_with_committed_anchored_events_instrumented` (`pub(crate)`, the real body) —
with the pre-existing entry point kept signature-identical and delegating with the old defaults.
Every existing caller retains byte-identical behaviour. Verified by the
`run_pipeline_with_instrumentation` suite (4 passed) and the
`visual_debug_forwards_support_tool_selection` tripwire, both green after the switch.

**D2 — Step 8's exit condition restated.** As written ("no
`execute_per_layer_with_events_and_support_tools` or
`execute_per_layer_with_instrumentation_and_support_tools` call remains in any production file")
it became unsatisfiable once D1 landed, because the ladder's real body calls the latter by
design. Restated as: no non-anchored per-layer call remains at any pipeline or CLI **call site**,
i.e. outside `layer_executor.rs`'s own ladder. Verified: 11 sweep hits, 9 ladder-internal in
`layer_executor.rs`, 3 in one test file, zero production call sites elsewhere.

**D3 — out-of-scope literal-gate fix.** `cargo xtask check-literals` was already RED at HEAD
before any packet edit, at `crates/slicer-scheduler/src/manifest.rs` (a `ConfigFieldEntry`
literal in `#[cfg(test)] mod tests`, left by commit a50bfc28). Fixed with a single-line
`// exhaustive:` waiver under explicit user authorisation, so the packet's own gate could go
green. `..Default::default()` was rejected because it trips `clippy::needless_update`.

**D4 — guest freshness gate is unsatisfiable on this tree (NOT caused by this packet).**
`cargo xtask build-guests --check` returned exit 1 with 33 stale guests on the UNMODIFIED tree
before any edit. A full `cargo xtask build-guests` succeeded ("built 44 guest(s)", exit 0) and
`--check` still returned exit 1 (33 stale, all "fingerprint mismatch"; the count oscillated
33 -> 14 -> 33 across runs). Exit was 1 not 3, so `wasm-tools` is present and the WIT decode path
works — the defect is in the code-input fingerprint. This packet's edit list is host-only.
Flagged for separate investigation; it must not be used to license "unrelated to my changes"
claims. **Filed as `DEV-164` in `docs/DEVIATION_LOG.md` at closure review** (originally filed
as `DEV-158`; renumbered during the 2026-09-03 merge of this branch into `master` because
packet 247 had independently allocated that ID on mainline), with one added
observation this packet did not have: the `contract` binary's own
`guest_fixture_freshness_tdd::guest_components_are_not_stale` passes on the same tree, so the
artifact-decoding oracle disagrees with the fingerprint oracle and says clean.

**D5 — plane-plane coalescing.** Closure review found `synthesize_anchored_rows`
(`crates/slicer-runtime/src/anchored_rows.rs`) merged only object-vs-plane: planes are grouped
PER COLLECTION, so two anchored collections declaring the same off-grid Z produced two adjacent
synthesized rows at identical Z, contradicting AC-5's "merge iff `|dz| <= EPSILON`, no duplicate
Z". Unreachable in production today (no `AnchoredEntity` producer exists), but
`239c-support-layer-height-producer` would hit it first. Closed by an additive coalescing pass
after the global `(z_units, collection_ordinal)` sort: a run opens at the first plane and a
successor joins iff within `MERGE_EPSILON_UNITS` of the RUN ANCHOR (never of its predecessor,
which would let small steps chain-drift); the run keeps the anchor's Z and concatenates entities
in run order. Covered by `planes_within_epsilon_across_collections_merge_into_one_row`.

**D6 — AC-4 restated as executor-level; it was written pipeline-level and implemented
executor-level.** As originally written AC-4 said "when the pipeline executes", but
`zspanning_support_entity_emits_atomic_single_block` drives
`execute_anchored_event_collections_with_mode` plus `synthesize_anchored_rows`, exactly as AC-3
does. The mismatch was found by the closure review, not during execution, and the AC text has now
been corrected to match. The reason the pipeline level is not available: AC-4's claim is about
**where inside its anchor row** the block sits, and the pipeline only ever exposes the post-merge
`&[LayerCollectionIR]` to the emitter — by then the anchor `Model` row and the spanning block are
one row and the placement decision is no longer observable. `implementation-plan.md` Step 9 and
`task-map.md` `TASK-407` both already described AC-4 at this level; only `packet.spec.md`'s
Given/When/Then said otherwise. No test changed; the AC text did.

**D7 — a pre-existing red in this packet's own test binary, recorded not fixed.**
`runtime_wiring_tdd::config_schema_json_matches_documented_shape` fails in
`cargo test -p slicer-runtime --test integration` (`left: Some("1.1.0")`, `right: Some("1.0.0")`)
— the binary carrying eight of this packet's nine AC tests. It is **not** caused by 239a:
the test file is untouched by this packet's diff, and the only edit to
`crates/slicer-scheduler/src/manifest.rs` (which owns `build_config_schema_json`) is D3's
single-line `// exhaustive:` comment inside `#[cfg(test)] mod tests`. The cause is commit
a50bfc28's config-schema wire bump 1.0.0 -> 1.1.0, which updated the emitter but not this
assertion — the same commit that left `check-literals` red in D3. It went unnoticed during
execution because every AC command is an `--exact` single-test filter and no step runs the whole
binary. Full-binary state at closure: **331 passed, 1 failed**, that one failure. Fixing it
belongs to whoever owns a50bfc28's fallout, not to this host-seam packet.

**D8 — silent Z-spanning drop, found at closure review and fixed.** `synthesize_anchored_rows`
routed `ZSpanning` entities with an `if let Some(row) = ...find(anchor)` and **no `else`**, so an
entity whose `anchor_global_layer_index` matched no committed `Model` row had its paths discarded
with no error and no diagnostic — contradicting AC-2's "neither dropped nor duplicated". Reachable
by construction, not only in theory: `route_of` (`crates/slicer-runtime/src/layer_executor.rs`)
sends an entity whose anchor is absent from `plan.global_layers` down the `AnchoredCollection`
route, so such an entity arrives at synthesis. Unreachable in production today (no
`AnchoredEntity` producer), but `239c-support-layer-height-producer` would meet it first — the
same reasoning that justified fixing D5. Closed by returning
`Result<Vec<LayerCollectionIR>, LayerExecutionError>`: the unmatched-anchor branch now yields
`LayerExecutionError::AnchoredGeometry` naming the offending `local_id` and the missing anchor
layer. Both `pipeline.rs` call sites propagate with `?` through the existing
`From<LayerExecutionError> for PipelineError`; `visual_debug.rs` maps to
`VisualDebugError::CaptureFailed`. Covered by
`z_spanning_entity_with_no_anchor_row_is_an_error_not_a_silent_drop`.
