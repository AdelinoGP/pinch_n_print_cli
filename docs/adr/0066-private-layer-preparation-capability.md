# ADR-0066 — Private preparation belongs to the consuming Layer module

Status: **Accepted.** Confirmed with the approved source plan (interview Q23);
not yet implemented.

Cross-layer strategies such as lightning need whole-print planning before
ordinary Layer execution. We choose an opt-in preparation capability in the same
artifact as the consuming Layer module, with a private immutable plan whose
format belongs to that module. The host owns transport, lifetime, access checks,
and selection, while module authors own planning and interpretation. Full design
and acceptance gates: [Generalized private Layer-module preparation](../specs/layer-module-preparation-plan.md).

## Why this shape

The module remains in one scheduled stage. A separately versioned preparation
capability extends ADR-0045's export model without recreating a monolithic tier
world. Selecting or replacing the module selects or replaces preparation and
consumption together. This gives private plans better locality than a companion
planner artifact, which would require a dependency and payload-compatibility
agreement between independently replaceable halves. A shared plan catalog adds
provider resolution and cross-module compatibility that this ownership model
does not need.

Preparation runs after the relevant completed PrePass products and before Layer
execution. It receives declaration-gated whole-print context plus distinct
selected targets with effective configuration. Layers continue run-to-completion;
the host retains plan bytes, not unfinished arenas or live module state. Native
and WASM adapters follow ADR-0056's single module model.

## Consequences

- Plans are module-private named byte pieces, published atomically on successful
  preparation. A selected module requires preparation success; successful empty
  output is a ready no-work plan, not a missing prerequisite.
- Owner-wide reads are immutable and bounded by explicit requested ranges. Plans
  are print-scoped in-memory data, including anchored consumers. Initial resource
  policy is accounting and measurement, with no new retained-plan quota.
- Shared full-identity targeting/configuration is a separate blocking
  prerequisite. Existing variant-key delivery defects must not be frozen into
  the preparation contract.
- `visual-debug` support is optional per module but included in the framework.
  Requested preparation views produce standard diagnostic geometry during
  preparation, committed atomically alongside the opaque plan and read by the
  ordinary post-commit capture path. Initial views are XY. There is no separate
  visualization export or host decoder for the private plan format.
- A real combined-export/native pilot must prove resource and import composition,
  fresh-instance reads, and ordinary-guest compatibility before integration.
  Framework completion is independent of lightning migration correctness.
- Lightning migration requires a demonstrated canonical-correct PrePass input
  strategy and portable module-owned preparation. When that gate and migration
  acceptance pass, remove the old host producer, `LightningTreeIR` path, and
  `lightning-tree-segments` accessor directly, with affected contract/binding
  changes. There is no deprecation shim or approximate-geometry fallback.

## Executable pilot evidence

**PILOT evidence only — not production availability.** Packet
`layer-module-preparation_02_preparation-capability-pilot` supplies the
controlled-fixture composition witness required above. The accepted decision
is unchanged; production preparation is not active through this pilot.

The imported `preparation-types` resources in
`crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit` remain
host-owned. `plan-access.current()` reuses their defining resource identity;
`prepare.prepare-print` reuses `slicer:common/module-errors.module-error`.
`PREPARATION_QUALIFIED_EXPORT` and the `PreparationExportSchema` sidecar
(`crates/slicer-schema/src/lib.rs`) identify the independent capability,
not a scheduled stage. All nine composed worlds preserve their ordinary stage.
`resolve_stage_from_world` and `compare_worlds` (`xtask/src/wit_verify.rs`)
verify the single scheduled-stage candidate and the capability surface.

Executed commands and results supplied by the implementation session:

| Command | Observed result |
|---|---|
| `wasm-tools component wit crates/slicer-schema/wit` | Exit 0; package group with all nine composed worlds resolved |
| `cargo test -p slicer-wasm-host --test contract preparation_pilot_` | 9 passed |
| `cargo xtask build-guests --check` | Exit 0; all guests fresh |
| `cargo build --workspace --tests` | Exit 0 |
| `cargo test -p slicer-wasm-host --test contract anchored_events` | 4 passed |

The preparation tests in `preparation_capability_pilot_tdd`
(`crates/slicer-wasm-host/tests/contract/preparation_capability_pilot_tdd.rs`)
were `preparation_pilot_fresh_wasm`,
`preparation_pilot_native`, `preparation_pilot_stage_surface`,
`preparation_pilot_plain_guest`, `preparation_pilot_accounting`,
`preparation_pilot_isolation`, `preparation_pilot_publication_failures`,
`preparation_pilot_ranges`, and `preparation_pilot_export_mismatch`.
The stage-surface matrix derived 9 Layer rows from `STAGES`
(`crates/slicer-schema/src/lib.rs`), compiled 18 plain/prepared scratch crates
for native and componentized WASM (36 configurations), and observed exactly
the derived set. Opt-in without `LayerPreparation::prepare_print`
(`crates/slicer-sdk/src/preparation.rs`) failed compilation as required.

The archived pre-edit `classic-perimeters.wasm` at
`target/preparation-pilot/plain-before.component.wasm`, SHA-256
`6499a7d809a1812bc581adab2bc394ca660f209b0f38b9ce459e9c47c43e5c5c`,
loaded through the typed ordinary path with preparation imports registered
alongside the ordinary linker, without a preparation export or parameter.
The archive is a pre-edit snapshot, never replaced by a newly compiled fixture.

Measured AC-5 accounting witness (observations, not acceptance bounds):

Durations are single-run observations on the implementation environment;
values vary per run, are informational only, and have no latency threshold.

| Measurement | WASM | Native |
|---|---|---|
| Preparation duration | 150 800 ns | 16 500 ns |
| Ready-empty preparation duration | 137 000 ns | 6 300 ns |
| Host-read durations | 2 100 ns; 1 500 ns | 1 000 ns; 800 ns |

Retained payload was 4 bytes; retained UTF-8 names were 9 bytes (`numbers`:
7, `π`: 2). Ready-empty retained 0 payload/name bytes. Per run, input was
1 transfer / 4 bytes, puts were 2 / 4 bytes, and reads were 2 / 4 bytes.
Ready-empty input was 1 / 0 bytes, with no puts or reads. Durations are absent
when instrumentation is off. Allocator/process overhead and process peak
memory are **UNMEASURED**. No latency acceptance threshold is claimed.

Metadata is whitelisted to owner, phase, ready state, piece names/lengths,
payload/name accounting, transfer sizes/counts, and measured durations; it
excludes raw opaque payloads. Payload-exclusion checks were challenged with
injected payload arrays, disguised accounting arrays, and escaped
payload-bearing strings. Weak references proved store release after consumer
unwinding.

**Scope boundary:** controlled-fixture evidence only. Row 03 still owns
production declaration parsing/validation and normalized host storage;
test-local staging and `read-fixture` are not its production API. Row 04 owns
typed whole-print input projections; row 05 owns scheduling, selection,
override, and cancellation lifecycle activation; row 06 owns diagnostic/visual
projections. None is marked complete here. No lightning algorithm, migration,
or parity claim is made.

## Relationship to existing decisions

- **ADR-0045:** extend one scheduled-stage export with a declared, independently
  versioned preparation capability. Preserve explicit typed compatibility and
  host-owned imported resource identity.
- **ADR-0056:** preserve one ingestion, selection, and override model with
  equivalent native/WASM preparation and Layer adapters.
- **ADR-0029:** replace its host-owned lightning production decision when the
  gated migration lands; retain the requirement for whole-print computation
  before per-layer sampling. Until then its implementation remains present.
- **ADR-0037 / ADR-0040:** extend post-commit, dependency-closure-aware visual
  captures with generic module-provided diagnostic projections.
