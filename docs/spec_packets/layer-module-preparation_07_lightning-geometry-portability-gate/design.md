# Design: lightning canonical input oracle (Gate A)

## Selected Approach

Gate A is one bounded experiment: take the already-recorded canonical planning data for five
Classic families, publish it as a portable oracle, rebuild the same planning domains from the
restricted earlier-source description, and record whether the reconstruction matches. Gate B is a
separate packet and consumes the same fixtures. The split removes the previous packet's
all-or-nothing coupling: a real A mismatch is a result, not a reason the packet cannot close.

The recorder is frozen. No new C++ instrumentation, no `combine_infill` lineage, no true
rebuild-and-reslice replay. The fixtures are derived from the current 36-variant capture
(`target/lightning-gate-current`), which is reproducible with the checked-in patch.

## Fixture Model

Three checked-in files under `crates/slicer-wasm-host/tests/fixtures/lightning_gate/`:

- `inputs.json`: strict `GateInputsFile { schema_version: u32, cases: Vec<GateInputCase> }`;
  `GateInputCase { case_id, wall_strategy: Classic, a_source: GateASourceInput, b_input: Option<BInputEnvelope> }`.
  `BInputEnvelope { input: KernelInput, contributor_evidence: Vec<FinalFillEvidence> }`, where
  `FinalFillEvidence { dispatch_token: String, status: Complete|Unavailable, reason: Option<String> }`
  is keyed by dispatch token, never vector position. A selects and serializes only `a_source`; it
  never parses `b_input`.
- `canonical.json`: strict `GateOracleFile { schema_version, cases: Vec<GateOracleCase> }`;
  `GateOracleCase { case_id, wall_strategy: Classic, reference: GateReference }`.
  `GateReference` carries the A reference (`planning_layers`, `policies`, `parameters`,
  `scalar_source_region`, `replay`) and the B reference (`graph`, `clipped`, `final_fills`).
  A parses only its fields.
- `manifest.json`: the gate manifest. It records `schema_version`, `canonical_revision`,
  `canonical_build_revision`, `canonical_build_dirty` (expected true: recorder patch),
  `recorder_patch_sha256`, tool versions, canonical/PnP unit and transport tolerance, the five
  `case_id`s, per-case `a_source_sha256`, `a_raw_config_sha256`, `a_earlier_provenance_sha256`,
  `b_input_sha256`, `reference_sha256`, and a `contributor_evidence` summary. Missing hashes are
  explicit `"absent"`, never omitted.

`contributor_evidence` is an envelope field on each final-fill record, not a kernel DTO:
`complete | unavailable`, plus a named reason. It exists because the recorder cannot yet attribute
dispatches whose surfaces are created by `combine_infill`. Gate A ignores it; packet 09 uses it to
record per-dispatch `inconclusive` without silently shrinking the population.

`a_source` is the existing restricted `GateASourceInput` from the kernel model:
`object_id`, `layers`, `canonical_region_order`, `object_config`, `provenance`. It contains no
planning/final domains, no resolved policy, no graph, and no reference hashes. `object_id` is the
stable scene name (`object-a`), never the unstable numeric `PrintObject::get_id()`.

## Gate A API and Decision

- `project_gate_a(view: &serde_json::Value) -> Result<GateASourceInput, KernelError>` selects the
  `a_source` key, serializes it unchanged, and calls `extract_gate_a_source`.
- `extract_gate_a_source(source_bytes: &[u8]) -> Result<GateASourceInput, KernelError>` strictly
  deserializes the restricted subtree with `deny_unknown_fields`, closed enums, and
  required-presence `Option` keys. Unknown reference-like keys fail before construction.
- `build_planning_domain(&GateASourceInput) -> Result<GateAConstruction, KernelError>` is pure:
  no filesystem, no config lookup, no reference/B handle. It derives:
  - planning `internal`, `void`, and `temporary_anchor` domains from earlier surfaces and the
    documented Phase-2 anchor policy;
  - `PlanningLayer` source catalog, boundary provenance, and geometry operations;
  - `LightningParameters`, `scalar_source_region`, and per-region/layer `DerivedRegionPolicy`
    (`FillPolicyInput`), including the numeric angle, `fixed_angle`, templates, top/bottom
    overrides, and model alignment.
- The comparator (host-only) normalizes and compares planning layers, domain geometry (holes
  preserved), planning-boundary provenance (source/ring/segment identity, overlap intervals,
  generated operation lineage), parameters, scalar source, and policies. It returns per-case
  `pass | fail | inconclusive` plus named mismatches. It never mutates construction and never
  supplies a value to it.

`KernelError` already carries the required variants (`RequiredSourceInputUnavailable`,
`RequiredSourceInputMismatch`, `SourceProvenanceUnavailable`, `SourceProvenanceMismatch`,
`InvalidInput`, `UnrepresentableTransport`). A missing earlier source/width/template/transform is
`inconclusive`; a measured difference is `fail`.

## Measured Constraints

- The current capture has 36 variants. The five Classic families are present and replay-verified.
  `anchor_bridge` classic: faithful 27 nodes / 16 edges / 11 roots, wrong 67/43/24, aligned
  per-layer symmetric difference 142.55912927276 mm². Provenance: 16 source surfaces, 24 planning
  boundaries, 12 generated, 36 overlaps, 0 unavailable.
- `hole_void` classic now has a true hole ring: 4-point contour plus one 4-point hole in the
  z=0.4 `internal` and `final_internal` domains; 8 planning boundaries / 16 overlaps / 0
  unavailable. Arachne is 36/72/0 but is not gate coverage.
- Caller-level final-fill contributor attribution is incomplete: only `smoke_box` resolves every
  dispatch; the five families have unresolved groups created after the pre-anchor snapshot. This
  does not affect A's planning-domain comparison and is not repaired here.
- The recorder's `--replay` is reference-copy, not a rebuild-and-reslice. Gate A uses the recorded
  `replay.json` snapshot comparison (faithful vs wrong), which is the discriminator AC-3 needs.

## Files in Scope

| Area | Files |
| --- | --- |
| Recorder publisher | `tools/lightning_gate/record.py`, `tools/lightning_gate/README.md` |
| Portable fixtures | `crates/slicer-wasm-host/tests/fixtures/lightning_gate/{inputs,canonical,manifest}.json` |
| Kernel model capability extension | `modules/core-modules/lightning-infill/feasibility-kernel/src/model.rs` |
| Kernel reconstruction | `modules/core-modules/lightning-infill/feasibility-kernel/src/{domain,polygon,distance_field}.rs` (+ Cargo-generated lock) |
| Host dependency wiring | `crates/slicer-wasm-host/Cargo.toml` dev-dependencies, root `Cargo.toml` (`[workspace] exclude`), root `Cargo.lock` (generated) |
| Kernel manifest | `modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml` (workspace-sentinel removal, `clipper2-rust` exact pin) |
| Oracle tests | `crates/slicer-wasm-host/tests/integration/{main,lightning_oracle_tdd,lightning_oracle_support}.rs` |
| Outcome | `docs/specs/layer-module-preparation-plan.md` (append-only section), `docs/07_implementation_status.md` (`TASK-576` A line) |

The kernel `lib.rs` is read-only for this packet. `model.rs` was delivered read-only at
authoring time; the Step 1 fixture publisher measured that it cannot carry four pieces of
recorded A evidence (the `EarlierSurfaceKind` bottom-bridge surface, the recorder's `null`
segment identity on generated contributors, the `dont_filter_internal_bridges` /
`sparse_infill_pattern` config keys, and the `post_anchor` checkpoint plus five `LightningParameters`
scalars). Step 3a therefore owns the minimal capability extension, and any further DTO need is a
scope change, not an implicit fifth file. Gate A's answer must not be suppressed by a transport
gap: a case blocked by a missing DTO field is recorded as `inconclusive` naming that field, never
as a scientific `pass` or `fail`.

## Read-Only Context

Bounded windows only: the kernel model/API, the recorder's capture output format, host test
registration and common guest cache, and canonical functions through delegated reads. Do not load
the full capture tree, G-code, oversized JSON, lockfile contents, or generated bindings.

## Out-of-Bounds Files

Production lightning (`src/lib.rs`, host producer/driver, paint), WIT/SDK/macros/runtime,
`tools/lightning_gate/canonical-recorder.patch` (frozen), packet 08 and 09 implementation files,
other packets, ADRs/deviations, source-plan approved body, backlog entries other than `TASK-576`.

## Dispatches

- Canonical fact check for the Phase-2 anchor/domain and scalar/policy formulas: `SUMMARY ≤200
  words`, file+function only.
- Fixture conversion fact check (which recorded artifact supplies which field): `LOCATIONS ≤20`.
- Cargo check/test runs: `FACT` exit and test count, at most 20 failure lines.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Exact freshness outcomes:0 fresh;1 stale or lock divergence (rebuild or `--sync-locks` respectively);3 infrastructure unable to form an opinion, stop. Ordinary/accelerated artifacts are distinct; this gate uses ordinary artifacts. No guest failure attribution before this check. Guest execution uses `cargo xtask test`.

## Invariants

- No B/reference value reaches construction, even as an unused field; strict serde rejects it at
  any nesting.
- Source/config/provenance hashes derive only from typed A fields, never from the whole envelope.
- Normalization is deterministic: canonical 1 nm → PnP 100 nm, holes preserved, ring start
  rotated, exact rational overlap intervals.
- A scientific mismatch is recorded and may stay red; it is never repaired from reference data,
  defaulted, or skipped.
- No production path or public schema changes.

## Risks and Tradeoffs

- The source-only reconstruction may not match canonical on some family. That is a valid Gate A
  `fail` and the packet's product.
- Fixture conversion for five families is the main implementation cost; it is deterministic and
  driven from recorded artifacts.
- A `fail` blocks migration and returns the strategy for design review; it does not block packet
  closure.

## Context Cost Estimate

Aggregate M across seven bounded steps (Steps 1–6 plus the 3a capability extension); largest step
M. The fixture publisher and the reconstruction each need narrow, delegated fact checks rather than
broad source reads.

## Open Questions

- [FWD] Which family, if any, reproduces canonical domains/policy, and where does the first
  mismatch occur? Answer recorded at execution.
- Limitation: final-fill contributor provenance is incomplete (`combine_infill` and grouped
  union/`diff_ex` are unrecorded). A does not consume it; packet 09 records affected dispatches as
  inconclusive. This remains a migration requirement.
- Limitation: Arachne, translated, and density companions are captured but not gate coverage.
