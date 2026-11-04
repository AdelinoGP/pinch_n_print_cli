---
status: implemented
packet: layer-module-preparation_09_lightning-kernel-portability
task_ids:
  - TASK-576
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: lightning kernel portability (Gate B)

## Goal

Execute the module-owned lightning kernel **source** on the five Classic canonical planning inputs
through the real native entry and a test-only WASM guest, compare the planned graph and grounding
witnesses to the independent canonical replay record, establish `clip_lightning` semantics against
constructed final fills, and record a truthful `pass`/`fail`/`inconclusive` Gate B outcome.

The kernel under test is the already-migrated module source
(`modules/core-modules/lightning-infill/src/kernel/{generator,layer,tree_node,distance_field}.rs`),
included into the isolated `feasibility-kernel` crate by `#[path]` — byte-identical source, not a
copy and not a second implementation. The adapters that turn canonical planning layers into
`Generator` inputs and turn `Generator`'s tree graph into `KernelPlan` records are this packet's new
code and live outside the included files.

A mismatch is a completed investigation, not an incomplete packet. The canonical capture does not
contain every population this packet's schema can carry; those populations are recorded as **named**
non-comparable populations and never silently dropped.

## Scope Boundaries

This packet owns **Gate B only**. It adds: the `#[path]` kernel-include module, the canonical-input
and graph-extraction adapters, `plan_lightning`, `clip_lightning` with constructed-fill semantics, a
test-only guest using the existing `slicer:layer-infill` world, host dev wiring, the native/WASM
driver, four integration tests, one kernel crate test, and the B outcome.

It consumes packet 07's checked-in fixtures read-only and does not edit them, the recorder, or the
recorder patch. No production wiring, no migration, no new WIT/SDK surface, no preparation
integration, no production module edit.

Comparable populations (canonical data exists):

- the per-layer planned node multiset (point, parent point, root point, `is_root`),
- the complete directed edge-occurrence multiset,
- the isolated-root multiset,
- grounding witness **points** on root nodes.

Named non-comparable populations (the capture does not record them):

- per-node `origin_region_index`,
- the per-node boundary/source/contributor/operation table,
- `sampled_segments`,
- grounding witness `kind`,
- the entire canonical clipped-segment comparison (`reference.final_fills` is published `null`),
- final-fill contributor attribution for dispatches the manifest marks `unavailable`.

A case-level `pass` therefore asserts only the comparable populations and is stated as such in the
outcome. The clipped sub-claim is recorded `inconclusive` with the manifest's exact absent reason; it
blocks migration and is not converted into a pass.

## Prerequisites and Blockers

- Re-derive packet 07 and packet 08 status at activation; do not quote a status written here.
  Packet 07 published the fixtures and packet 08 migrated the kernel source this packet includes.
- Published fixture facts, verified against the checked-in files on 2026-10-23:
  - `manifest.cases[*].absent` names `b_input.input`, `reference.graph`, `reference.final_fills`,
    and `reference.policies` as `null` with a reason each.
  - `reference.replay.production.graph` is populated for all five cases; `reference.clipped` and
    `inputs.b_input.contributor_evidence` are populated.
  - Canonical node/edge/root populations are `27/16/11`, `46/32/14`, `41/27/14`, `41/25/16`,
    `48/30/18`; every edge resolves inside its own layer's node table.
  - Canonical clipped fills per case are `13/2/2/4/6`; every clipped dispatch token resolves in
    `contributor_evidence`.
  - `reference.planning_layers` records only global layer 1 (plus global layer 2 for
    `anchor_bridge_classic`, which has zero internal domains and two temporary anchors); replay
    nodes exist only on the recorded layer. Layers without a planning record are empty inputs.
  - `reference.parameters` is identical across cases: density `0.15`, extrusion width `0.45` mm,
    layer height `0.2` mm, multiline `1`, nozzle `0.4` mm, overhang `45°`, prune `5°`,
    straightening `5°`, supporting radius `3.0` mm, wall supporting radius `0.2` mm,
    prune length `0.017497` mm, straightening max distance `0.017497` mm.
- The included kernel files import only `slicer-ir`, `slicer-core` (`difference`, `offset`,
  `OffsetJoinType`, `geometry::closest_point_on_segment`, `polygon_ops::clip_polylines`), and
  `super::` siblings. The feasibility crate therefore adds `slicer-core` with default features; it
  must stay free of `host-algos`, `rayon`, `boostvoronoi`, `slicer-runtime`, and the production
  module.
- `cargo xtask build-guests --check` must exit 0 before any guest-execution failure is attributed to
  this packet. Exit 1 is stale/lock divergence; exit 3 is infrastructure and stops the run.
- Completion requires execution and a truthful record, not `pass`. `fail`/`inconclusive` blocks
  migration and returns the port for design review.

## Acceptance Criteria

Commands are non-interactive Bash. `set -euo pipefail` rejects Cargo, tee, and log-check failures;
every command creates `target` and removes a stale `target/test-output.log` before running. Each
exact filter executes exactly one test; zero tests fails. Read/archive the combined log before
overwrite; never rerun for truncated output.

- **AC-1. Given** the five Classic `planning_layers`, `parameters`, and `canonical_region_order` from
  the checked-in fixtures and the module kernel source included by `#[path]`, **when** both adapters
  execute `plan_lightning`, **then** every case returns a complete record set for every recorded
  planning layer: each node carries a `node_id` below `1 << 24`, a finite `point`, an
  `origin_region_index` below `canonical_region_order.len()`, and a `parent_id`/`root_id` that
  resolves inside its layer; each grounding witness resolves to a `BoundaryProvenance` entry whose
  `address.surface_id` indexes that layer's `source_catalog`; each layer carries the input
  provenance table unchanged; the planned arm passes `layers: []` and still produces a nonempty
  graph, proving planning does not read the B-only clipping transport; the four
  derived scalars (`supporting_radius`, `wall_supporting_radius`, `prune_length`,
  `straightening_max_distance`) equal the recorded `3.0/0.2/0.017497/0.017497` mm in PnP units for
  every case; a zero-density parameter set carries `supporting_radius_mm = None` and yields a zero
  supporting radius without a nonzero default, while a `Some`/zero-density or
  `None`/nonzero-density disagreement is a named error; and both arms report nonzero node/edge/root
  populations for all five cases. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::planned_graph_native_and_wasm -- --exact >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-2. Given** the executed planned records, **when** the independent comparator runs in the same
  test on a fresh planned-driver execution, **then** each case's recorded status equals the
  recomputed comparison over exactly the canonical populations that exist: a node multiset keyed by
  `(point, parent point or none, root point, is_root)` with multiplicity, the complete directed
  edge-occurrence multiset keyed by `(parent point, child point)`, the isolated-root multiset, and
  each canonical `last_grounding` point lying on the kernel's addressed boundary segment within the
  recorded transport tolerance — with a canonical `null` requiring no kernel witness; `pass` names
  no difference, `fail` names the first missing/extra/reversed/disconnected entry, `inconclusive`
  names an unavailable comparable population; local node/edge ids are never compared because they
  may be relabeled; per-node `origin_region_index`, the per-node provenance table,
  `sampled_segments`, and grounding `kind` are recorded as named non-comparable populations; and both
  adapters are compared independently to `reference.replay.production.graph`, never to each other. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::gate_b_decision_is_truthful -- --exact >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-3. Given** constructed `FinalFillInput`s — a real expolygon with a hole, a partial domain, and
  an empty domain — **when** `clip_lightning` executes, **then** every dispatch yields oriented
  two-point segments lying inside its final expolygon with holes preserved and no segment welding,
  reordering, or clipping to a planning domain; a missing or ambiguous dispatch token is a named
  `InvalidInput`; and the canonical clipped comparison is not claimed by this criterion (it is
  recorded `inconclusive` under AC-5). | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml clip_lightning_semantics_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-4. Given** deliberately canned, echoed, region-isolated, or host-fallback outputs and corrupt
  graph payloads (missing/extra/duplicate node or edge, reversed edge, disconnected parent, corrupt
  root witness, tag-3 byte error), **when** the driver and decoder run, **then** each is rejected
  with a named mismatch or missing-producer error, including identical corruption applied to both
  adapters; a valid nonempty case still passes the same decoder; no callable host lightning producer
  or fallback is installed. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::rejects_canned_echo_and_fallback -- --exact >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- **AC-5. Given** the executed cases, **when** the outcome writer validates
  `target/lightning-portability/outcome.json` in the same test on a fresh driver execution, the
  source-plan outcome section, and `TASK-576`, **then** the record carries `gate: "B"`,
  executed/freshness identities, the `#[path]` include list with a sha256 per included kernel source
  file, the guest artifact hash, the `build-guests --check` exit, native/WASM measured
  node/edge/root counts, per-case planned statuses, per-case clipped status `inconclusive` naming the
  manifest's exact `reference.final_fills` absent reason with the recorded fill counts `13/2/2/4/6`,
  the named non-comparable populations, commands/exits/log hashes, and named blockers;
  `TASK-576 Gate B outcome:` is one of `pass|fail|inconclusive`; and `migration_eligible` is false
  unless A passes, B passes, and framework acceptance is independently recorded. | `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd::outcome_records_gate_b -- --exact >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a missing/extra/duplicate node or edge, a reversed edge, a disconnected parent, a
  corrupt root witness, or a tag-3 byte error, **when** the decoder runs, **then** each is named and
  fails, including when both adapters return the same corrupt payload.
- **AC-N2. Given** an empty, echoed, cached, region-isolated positional pair, or host-fallback
  output, **when** the comparator runs, **then** it is rejected; a digest-only or same-object
  self-comparison never counts as a pass.
- **AC-N3. Given** a failed/inconclusive A or B record, unexecuted artifacts, or absent framework
  acceptance, **when** migration eligibility is evaluated, **then** `migration_eligible` is false
  with the named blocker; no approximation or defaulted parameter changes the status.

## Completion Semantics

This is a feasibility investigation. The packet is complete when every exact command above has
executed and the outcome record truthfully reflects the measured comparison. The tests assert
execution, strict decoding, and that the recorded status equals the independently recomputed
comparison; they do not assert that the comparison must be `pass`.

The canonical capture never recorded final-fill domains or policy, so the clipped comparison is
recorded `inconclusive` with the manifest's reason and the recorded fill counts. That is a measured
limitation, not a skipped test: the kernel's clip semantics are exercised by AC-3 on constructed
fills, and the canonical population is accounted for by count and named reason. A measured graph
mismatch is recorded as a case-level `fail` and blocks migration; it is never converted into a pass,
repaired from expectations, or skipped. `TASK-576` is not `[x]` unless both Gate A and Gate B pass.

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`
- `cargo test --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml`
- `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_portability_tdd:: -- --nocapture >target/lightning-portability-command.log 2>&1; rg -q '^test result: ok\. 4 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- `cargo xtask check-literals`
- `cargo xtask check-test-quality --report`

## Doc Impact Statement (Required)

- Future implementation appends the B subsection to `## Lightning geometry/portability gate outcomes`
  in `docs/specs/layer-module-preparation-plan.md`, preserving the recorded A subsection verbatim. If
  the heading is absent, create it and add both records only after A is recorded. Check:
  `rg -q '^## Lightning geometry/portability gate outcomes$' docs/specs/layer-module-preparation-plan.md`.
- Future implementation updates only the existing `TASK-576` item under
  `## Perimeter Modules — OrcaSlicer Parity Roadmap` / `### M2 — Real Arachne` in
  `docs/07_implementation_status.md`, adding `  - TASK-576 Gate B outcome: (pass|fail|inconclusive)`.
  `[x]` requires both verified passes and packet acceptance; fail/inconclusive keeps `[~]`/`[ ]`.
  Placement checks: `rg -q '^- \[[ x~]\] \*\*TASK-576\*\*' docs/07_implementation_status.md`;
  `rg -q '^  - TASK-576 Gate B outcome: (pass|fail|inconclusive)\b' docs/07_implementation_status.md`.
- No backlog or source-plan edit occurs during authoring.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: row 07 Gate B sub-gate, Q13/Q14, row-08 entry.
- `docs/adr/0066-private-layer-preparation-capability.md`: migration waits for canonical strategy and
  portability proof, read with its `## Amendment — 2026-10-20` section.
- `docs/adr/0074-lightning-migration-on-recorded-equivalence.md`: packet 08's ownership move is not a
  parity claim; Gate B remains forward work.
- `docs/03_wit_and_manifest.md`: existing `slicer:layer-infill` world; no new WIT.
- `docs/05_module_sdk.md`: generated native entry, per-call reconstruction, test feature.
- `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md`,
  `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `generateTrees`, `generateInitialInternalOverhangs`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` — `getBestGroundingLocation`, `attach`, `reconnectRoots`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp` — `convertToPolylines`, propagation/straightening.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` — `_fill_surface_single` final clipping.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills` caller identity.

Citations use file plus function, never canonical line numbers. Source inspection is not execution evidence.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
