# Generalized private Layer-module preparation

Status: **Approved source plan — ready for packet generation.** The interview
decisions, consolidated interface, acceptance witnesses, and dependency map are
confirmed as a complete design. Production implementation has not begun.

Decision record: [ADR-0066 — Private preparation belongs to the consuming Layer module](../adr/0066-private-layer-preparation-capability.md).

## Goal

Replace lightning-specific host algorithm ownership with a generalized mechanism
through which a `Layer::*` module can prepare whole-print data before layer
execution and consume its immutable plan during ordinary layer processing.

Lightning is the motivating migration. The mechanism must be usable by Layer
stages beyond infill and by external as well as integrated modules.

## Settled requirements

- **Layer lifetime:** preserve run-to-completion `LayerArena` ownership. Do not
  introduce between-stage whole-print barriers or retain unfinished layer arenas.
- **Preparation timing:** run preparation in late PrePass, after the relevant
  completed analysis and final region-resolved prepass geometry are available,
  before Layer execution begins.
- **Algorithm ownership:** preparation and plan interpretation belong to modules.
  Investigate module-owned payload formats first; host-defined geometric plans
  are a fallback only if the module-owned approach proves infeasible.
- **Private ownership:** the preparation mechanism is reusable across Layer
  stages, but an individual plan is private to its planner–consumer relationship.
  Unrelated modules do not consume that plan. Shared plans were permitted as an
  alternative if demonstrably simpler; the design comparison found additional
  provider-resolution and format-compatibility machinery rather than a simplicity
  advantage.
- **Automatic activation:** selecting a Layer module automatically selects its
  preparation. Users do not configure the halves independently.
- **One artifact:** whole-print preparation and ordinary Layer processing belong
  to the same selected module artifact. An executable capability pilot is the
  first implementation gate before migrating lightning.
- **Immutable consumption:** Layer execution consumes prepared immutable data;
  preparation does not depend on preserving a live module instance across calls.
- **Whole-print read context (Q7):** preparation may inspect declared, completed
  PrePass inputs across the whole print, including regions where its module is
  not selected. Selection targets and their effective configuration are described
  separately. Read access does not grant output ownership.
- **Owner-wide plan reads (Q8):** an ordinary Layer call may read any portion of
  its owning module's immutable prepared plan through bounded reads. The host
  does not restrict private plan reads to the current layer.
- **Required success (Q9):** a preparation-capable module requires successful
  preparation before Layer execution. Failed or missing preparation stops the
  slice. Successful empty/no-work preparation is a valid, explicit outcome and
  must be distinguishable from preparation that did not complete.
- **Named opaque pieces (Q10):** a prepared plan is a private name-to-bytes
  collection. The module defines piece names and encoding. The host does not
  interpret names as layer/region addresses or inspect their geometric meaning.
  An empty collection published by successful preparation explicitly means no
  work. Consumers open pieces and read bounded ranges.
- **Print-scoped memory (Q11):** host memory retains the immutable plan for the
  current print, including any anchored Layer invocations that consume it. Release
  storage on completion or abort. Account for retained size and bounded transfers;
  the capability pilot must measure these costs. Disk spilling and cross-print
  caching are outside the chosen initial storage contract.
- **One-call publication per piece (Q12):** publish each unique piece name and its
  complete bytes in one call. Duplicate names are contract errors. Publish the
  collection atomically only after successful preparation; discard all staged
  pieces on failure. Module authors partition large data into additional pieces
  rather than relying on an append/open/close writer lifecycle.
- **Independent framework completion (Q13):** the generic framework may complete
  on its own contract acceptance. Lightning migration and host-path retirement
  remain blocked until a canonical-correct late-PrePass input strategy is
  demonstrated and migration acceptance passes. A failed geometry proof returns
  the migration for a design decision; approximate geometry is not a fallback.
- **Separate routing prerequisite (Q14):** the full-identity routing/configuration
  repair is separate blocking work. Preparation integration cannot complete until
  that prerequisite is implemented and verified; its defects must not be copied
  into the new preparation contract.
- **Selected work only (Q15):** prepare a module once if it has eligible selected
  work anywhere in the print; skip preparation when it has none. Ordinary Layer
  invocations of a preparation-capable module also use the shared eligibility
  rules and are skipped where it has no eligible work. Eligibility must cover
  module-backed non-fill stages, merged perimeter sources, support carriers,
  raft work, and anchored invocations rather than only fill-holder strings.
- **Accounting first (Q16):** record retained plan size and transfer sizes,
  validate byte-range and length arithmetic, and measure these costs in the pilot.
  The initial framework adds no retained-plan quota.
- **Metadata diagnostics (Q17):** the initial choice covers preparation lifecycle,
  owner, declared inputs, selected-target summary, piece names/sizes, and timing
  when instrumented. Q18 expands this baseline with plan visualization.
- **Plan visual debugging (Q18):** include a generic, optional module-supplied
  visual projection mechanism and integration with existing `visual-debug`
  bundles in framework scope. The gated lightning migration must expose useful
  plan geometry for comparison with emitted infill. Visualization support is
  optional for other preparation-capable modules, including non-geometric plans.
- **XY plan views first (Q20):** initial plan visualization is top-down XY for
  selected layers. Front/side plan projections are a later extension.
- **Committed diagnostic snapshots (Q21, clarifying Q19):** match the existing
  post-commit capture model. When visual capture is requested, preparation emits
  a standard host-readable diagnostic projection alongside its private plan.
  Publish the projection atomically with successful plan publication, then let
  `visual-debug` capture and render it. There is no separate visualization export
  or post-preparation call back into the module. Normal slices do not request
  diagnostic geometry.
- **Direct lightning retirement (Q22):** when the gated migration lands, remove
  the old producer, typed `LightningTreeIR` path, and shared
  `lightning-tree-segments` accessor outright. There is no deprecation period,
  compatibility shim, or silent empty-result fallback. Update affected contracts,
  bindings, and guests as part of the same migration; framework introduction
  remains additive for ordinary modules.
- **Lightning placement:** retain cross-layer preparation before the Layer tier;
  moving lightning generation into `LayerFinalization` is not the chosen approach.

## Packaging decision — agreed

### Chosen: one artifact with a preparation capability

The selected Layer module owns both whole-print preparation and per-layer
consumption. A manifest-declared preparation capability accompanies its ordinary
scheduled-stage export. The host retains module-defined plan data after
preparation completes. The consolidated contract uses independently versioned
capability transport; the executable pilot must prove its WIT composition.

This requires extending the existing module contract, macro-generated exports,
typed WASM dispatch, and native registration. It avoids independently selected
planner artifacts and producer/consumer payload-version negotiation.

### Rejected alternative: automatically paired companion planner

A separate module in a generic late-PrePass stage produces a private plan for
its declared Layer consumer. This follows the existing single-stage-per-module
model but requires a dependency and compatibility contract between the pair.

**Decision:** the user selected one artifact in interview Q6, including an
executable WASM/native pilot before lightning migration. "Layer preparation"
and "prepared layer plan" replace the packaging-oriented working term
"Layer preparation pair" in `CONTEXT.md`.

## Design tree

The interview resolves prerequisites before their dependent choices. Source-code
facts are investigated rather than delegated to the user as questions.

| Decision branch | Current state | Dependent decisions |
|---|---|---|
| Preparation timing and arena lifetime | Settled above | Input availability; permitted algorithms |
| Private ownership and automatic activation | Settled above | Owner identity; access enforcement |
| One artifact or companion planner | Settled: one artifact (Q6) | Export contract; registration; payload compatibility |
| Preparation targets | Separate routing prerequisite (Q14); selected work only (Q15) | Precise shared projection acceptance; full identity; non-region invocations |
| Input visibility and configuration | Whole-print declared context (Q7); shared projection is a separate blocking prerequisite (Q14) | Concrete projection exported by prerequisite; integration witnesses |
| Plan addressing and access | Owner-wide reads (Q8); module-named opaque pieces (Q10) | Contract below, subject to final confirmation |
| Failure semantics | Required success (Q9); atomic publication (Q12) | Contract below, subject to final confirmation |
| Resource ownership | Print-scoped host memory (Q11); accounting first (Q16) | Contract below; pilot measurements |
| Lightning input fidelity | Explicit migration gate (Q13); framework may complete independently | Wall-inset geometry; shell/paint preservation; canonical witnesses |
| Compatibility and migration | Additive framework; direct gated lightning retirement (Q22) | Combined-export pilot; affected-contract inventory at migration |
| Diagnostics and acceptance | Metadata (Q17); plan visuals (Q18); XY (Q20); committed snapshots (Q21) | Contract and acceptance below, subject to final confirmation |
| Final approval | Approved in interview Q23 | Packet-generation-ready source plan |

## Consolidated framework contract — for final confirmation

The following names describe **new interfaces to implement**, not existing SDK
or WIT functionality. The capability pilot fixes generated binding composition;
it may refine mechanical type placement without changing these semantics.

### Declaration, exports, and authoring

- `[stage]` remains singular. Preparation is a declared lifecycle capability of
  that stage's module, not an additional scheduled-stage membership.
- Add a separately versioned `slicer:layer-preparation` WIT package, initially on
  the nonzero-major `1.0.0` track. It defines the preparation export, imported
  host-owned input/output resources, and owner-private plan reads. Canonical WIT
  lives under `crates/slicer-schema/wit/`.
- The module manifest gains an optional `[preparation]` declaration identifying
  the qualified preparation interface, preparation IR read permissions, and
  supported named visual views. These are parsed, stored, and validated rather
  than tolerated as unknown metadata. Ordinary stage IR permissions stay distinct
  from preparation permissions. Existing declared config-key filtering applies
  to both calls; each region gets its effective config through the shared
  full-identity projection.
- Manifest declaration and compiled export metadata must agree. A declared
  preparation export that is missing or incompatible fails before execution.
  A module author cannot opt in through an undetected extra method or export.
- The SDK exposes a required `prepare_print(input, output)` method for opted-in
  modules. It has no default successful implementation. An explicit macro opt-in
  binds that method and the ordinary Layer method into the same artifact. The
  exact macro spelling is established by the pilot's compiled example.
- Preparation receives a whole-print input view and a private plan-output
  builder. It does not use one print-wide constructor config as a substitute for
  effective per-region settings. Ordinary Layer calls retain their per-call
  construction through `from_config`.
- Generate equivalent native preparation and Layer adapters from the same module
  source. The frozen selected binding supplies both calls. An external override
  replaces preparation and consumption together through existing module-id
  search precedence; the host never mixes the halves from different artifacts.
- Plain module worlds and stage signatures acquire no mandatory preparation
  export or plan-read parameter. Prepared guests compose the ordinary stage
  contract with the new capability imports/export. The host registers the needed
  typed imports for each invocation. The pilot must prove this composition with
  real resources and a plain pre-existing guest, not only an import-free WIT stub.
- Host-owned resources are declared in imported interfaces. Native/WASM adapters
  follow the existing canonical type-remapping seam; opaque plan bytes, not a
  resource-table index, survive between calls.

### Scheduling and input visibility

1. Complete the PrePass products used by selection, including paint/modifier
   materialization and relevant support/anchored-entity products. Compute the
   shared selection/configuration projection from the committed layer plan,
   region mapping, slices, and applicable late products.
2. For each preparation-capable module with eligible work, invoke preparation
   once over the whole print. Use a deterministic serial order based on the
   frozen module identities. Each preparation reads only completed PrePass
   products and writes its own private output; there are no dependencies on
   another module's prepared plan.
3. Validate and publish each successful plan. All required preparations must be
   ready before any ordinary or anchored Layer invocation begins.
4. Execute ordinary layers run-to-completion with fresh call-local views of the
   owner's immutable plan. Execute applicable anchored invocations using the
   same prepared owner data and selection contract.

`PrintPreparationView` must separate **read context** from **selected targets**:

- Read context spans the committed global-layer schedule and declared whole-print
  region/geometry/analysis views. Surface classification and enriched slice data
  need explicit guest projections; simply adding a host-side borrow is not
  completion. Mesh queries, shell/bridge information, seam/support products, and
  region configuration use typed, declaration-gated views appropriate to the
  existing IR contracts.
- No view represents future perimeter, infill, or other unfinished Layer output
  as if it were already available. Typed optional PrePass products carry an
  explicit absence state. An undeclared read is an access error, not an empty
  optional product. Missing required prerequisites fail before invocation.
- Selected targets identify the owner module's eligible invocation contexts,
  held roles where applicable, complete source identities, and effective
  configuration. Model-region identity includes the ordered `variant_chain`;
  perimeter source membership and carrier/anchored provenance remain explicit.
  Non-region invocations must not acquire fabricated model `RegionKey`s.
- Configuration is derived from authoritative resolved inputs and filtered by the
  module's declared keys. Preparation and consumption use the same projection;
  neither performs an independent first-match lookup or last-variant-wins merge.
- The separate routing prerequisite must provide the concrete shared projection
  and verify model, painted, modifier, merged-source, carrier, raft, and anchored
  cases. Its deliverable is the production seam plus behavioral evidence, not a
  hand-maintained target roster or a newly populated but unused index.
- Input access should permit layer/region batching rather than requiring every
  invocation to copy the whole print into guest memory. Input ordering is
  deterministic, using canonical schedule and full-identity order; polygon
  collection position is not an entity identity.

### Private plan interface and lifetime

The semantic interface is small:

```text
During preparation:
  output.put(name, bytes) -> result

During an eligible Layer invocation:
  plan.open(name) -> result<option<piece>>
  piece.len() -> byte length
  piece.read(offset, max_bytes) -> result<bytes>
```

- The host supplies owner identity implicitly. No operation accepts a provider
  module id, searches other owners, or falls back to a shared catalog.
- Names are nonempty UTF-8 strings with exact, case-sensitive matching. They are
  opaque keys, not filesystem paths; no path normalization or geometric scope
  parsing occurs. Every name can be published once per preparation.
- The module owns encoding, schema meaning, compression, grouping, and internal
  indexes. The host has no payload-schema version negotiation or codec registry.
  A new plan format requires module changes, not format-specific host/WIT changes.
- `read` returns at most `max_bytes`, clipped to remaining bytes. An offset equal
  to length is a valid empty read; an offset beyond length is an error. Lengths,
  offsets, conversions, and accounting use checked arithmetic. Reads do not
  advance a shared cursor, so concurrent calls cannot interfere.
- `open` returning no piece is distinct from an unavailable plan. A module may
  intentionally test for an optional private piece; its decoder must report a
  missing required piece or malformed required payload as a fatal module error.
  The host cannot infer semantic coverage from names and must not claim to do so.
- Preparation stages pieces in an invocation-local builder. Returning success
  publishes the entire validated collection and a successful-completion marker.
  Success with an empty collection is a ready no-work plan. An uncalled,
  unfinished, failed, or cancelled preparation never has that marker.
- Failed publication, including a duplicate name, publishes nothing. Contract
  violations poison the invocation: catching a builder error in guest code and
  returning success must not publish a partially accepted plan.
- A required preparation error, trap, missing export, invalid output, or missing
  ready plan aborts the slice with owner and phase diagnostics. Original module
  error details are preserved; failure of this required dependency is explicit
  and does not rely on the existing ordinary-stage fatal-only bug. Cancellation
  uses the pipeline's cancellation outcome and drops staged output.
- Read views and WIT resource handles are call-local. The host retains immutable
  byte storage independently and creates fresh wrappers for each invocation.
  Native dispatch offers equivalent serialized-plan semantics, not an
  algorithm-specific native object shortcut.
- Retain plans for the print, including anchored consumers. Release them after
  completion or abort/cancellation unwinding and in-flight calls finish. There is
  no guest-instance, mutable module object, or unfinished `LayerArena` retained
  for this purpose.
- Track payload bytes and piece-name bytes separately, plus transfer counts/byte
  sizes and diagnostic-projection storage when present. Those accounting totals
  are not a claim about allocator overhead or process peak memory. Initial
  retention has no new quota; transfer/range validation and measured pilot
  evidence remain required.

### Prepared plan visual projections

This extends the existing `visual-debug` request/capture/render path; it does not
add a second visual-debug command or a module drawing export.

**Declaration and selection**

- A module optionally declares named preparation views, such as a view of its
  committed planning geometry. Names belong to that module; the host does not
  hardcode a lightning view or inspect private piece names to discover one.
- Add a structured preparation-tap selector identifying the full module id and
  declared view name. Preserve existing stage-tap requests. Gate the new request
  shape behind an appropriate request-schema revision, derived from the current
  schema when the visual work is authored.
- Validate request syntax and declared owner/view before module execution, then
  resolve real layers and owner eligibility against the committed schedule and
  shared selection. An unknown view, unsupported visualization, inactive owner,
  or unresolved layer fails with a named reason and no successful partial bundle.
- Initial plan taps support model-source, top-down XY views. A standalone final
  G-code file contains no private preparation plan to reconstruct. Front/side
  plan views remain outside the initial projection contract.

**Projection data and publication**

- When a declared view is requested, supply an opt-in projection sink to
  preparation. With no request there is no diagnostic geometry allocation or
  retention by the framework. The sink's optional presence is ordinary data,
  not an optional WIT export.
- The standard typed projection supports points, open polylines, and filled or
  outline polygons with holes, grouped by module-defined diagnostic class names.
  Optional labels identify features. XY geometry uses canonical `Point2` units;
  layer attribution and Z come from the committed schedule. Rendering follows
  `docs/08_coordinate_system.md` and uses existing `Projector` transforms.
- Classes describe diagnostic geometry, not extrusion roles. Deterministic
  display colors, glyphs, and screen strokes distinguish classes and primitive
  kinds; stroke styling does not claim an extrusion width or deposited volume.
- A requested view supplies a whole-print projection snapshot once, with explicit
  layer attribution, so framing and class assignment can be selection-independent.
  The renderer selects layers from that snapshot. Store it once, not one
  whole-print clone per rendered layer.
- Capture the final plan projection. Module authors should derive it from the
  same final plan representation they serialize. Intermediate checkpoints are
  not implicitly part of this view contract.
- Validate projection geometry, finite values where floats are used, layer
  identity, declared class/view references, and unique scene identity. A requested
  view must be explicitly completed even if it contains no geometry. Missing
  output is an error; an explicit empty projection is valid and labeled empty.
- Plan and requested projection publication are atomic. Failed preparation or
  invalid requested projection leaves neither available. Debug capture must not
  change the semantic prepared plan or ordinary Layer output; verify this with
  capture-enabled versus capture-disabled execution.

**Capture and bundle behavior**

- A preparation-only request runs the required whole-print PrePass/preparation
  closure, then reads the committed projection using the existing post-commit
  capture model. It constructs no `LayerArena` merely to view the plan. Requests
  comparing plan and later-stage output additionally execute the required
  selected-layer closure.
- Extend `CapturedIr`/`StageCapture` in
  `crates/slicer-runtime/src/layer_executor.rs` and rendering in
  `crates/slicer-runtime/src/visual_debug_render.rs` with a generic diagnostic
  projection. Do not disguise planned branches as emitted `InfillIR`.
- Use the existing PNG plus `manifest.json` bundle, shared model-wide framing,
  deterministic ordering, and overwrite/fail-closed behavior. Give each entry
  owner module/version, owning Layer stage, `preparation` capture phase, view
  name, projection schema, real layer/Z, class legend, empty status, and the
  typed geometry mirror needed to inspect the pixels numerically.
- Record actual executed preparation owners and lifecycle phase separately from
  ordinary stage/layer execution. Rendered-layer selection must not suggest that
  whole-print preparation only examined those layers. Schema changes preserve
  existing request/manifest compatibility according to the visual-debug contract.
- Images localize geometry differences; independent canonical evidence proves
  lightning correctness. Both belong in migration acceptance.

### Metadata, compatibility, and ownership diagnostics

- Module/DAG inspection describes declared preparation, its permitted inputs and
  private owner relation without instantiating WASM. Runtime instrumentation adds
  actual eligibility, preparation start/completion/failure, ready/no-work status,
  and measured duration when instrumented. Static declarations are not reported
  as executed work.
- Piece metadata includes owner, names, lengths, and accounting. Normal
  diagnostics do not export raw opaque payloads. Visual-debug exports the
  standard diagnostic projection, not the private plan codec.
- Framework-only changes must preserve loading of ordinary pre-existing guests
  that satisfy their original stage contract. A new preparation package must not
  by itself make unrelated guests WIT-stale. Changes to the routing prerequisite
  or later shared-contract retirement have their own explicit compatibility
  inventory and acceptance.
- Lightning retirement is direct after its gate: remove the producer, slot,
  schema, SDK accessor, host marshalling, and shared WIT accessor in the same
  migration. Inventory affected import/type identities and apply necessary
  contract-version/binding changes. Rebuild affected guests and verify named
  incompatibility errors; no legacy shim or grace period is part of this plan.

## Workstreams and packet-generation gates

Use topic-scoped packet naming from the current repository workflow. These are
dependency labels, not reserved packet numbers.

| Workstream | Deliverable | Blocking edges / completion |
|---|---|---|
| Routing/configuration prerequisite | Shared full-identity selection/configuration projection and targeted delivery fixes, including native/WASM tests | Separate work by Q14; blocks production preparation integration and its acceptance |
| Capability pilot | Real same-artifact preparation + Layer dispatch, fresh resource wrappers, immutable named-piece transport, SDK/native adapter example, ordinary-guest compatibility and measured transfer/retention evidence | May proceed independently of routing repair using a controlled fixture; blocks finalization of framework bindings and SDK authoring |
| Framework transport and integration | Manifest validation, private storage, runner adapters, activation, errors, declaration-gated whole-print input views, metadata and lifecycle diagnostics | Requires successful pilot and the routing prerequisite for integration closure |
| Plan visual-debug integration | Optional declared typed projections, atomic diagnostic snapshots, generic XY rendering, request/manifest evolution, truthful closure reporting | Requires framework capture seam; required before framework closure by Q18 |
| Framework acceptance | All generic behavioral witnesses below, docs/ADR alignment, relevant build/check gates | Independent completion allowed by Q13; does not close lightning migration |
| Lightning geometry/portability gate | Reproducible canonical planning-domain/grounding witness and portable module-owned kernel proof; concrete migration design | May research alongside framework work; failed or inconclusive proof blocks migration and returns for design review |
| Lightning migration and direct retirement | Module-owned preparation/decoder, declared visual projection, per-region effective config, canonical-correct output, deletion of old host contract | Requires framework acceptance plus successful geometry/portability gate; closes only with end-to-end migration evidence |

The source plan is sufficient to generate prerequisite, pilot, framework, visual,
and explicitly scoped feasibility packets after approval. **Do not generate an
implementation-ready lightning replacement packet from the current geometry
hypothesis.** First record the gate's verified input strategy, canonical
reference provenance, parameter/grouping policy, and supported configuration
coverage in this source plan. If either pilot reveals a need to change the
agreed interface or arena model, stop the dependent work and obtain a design
decision rather than introducing a workaround.

### Required acceptance witnesses

Tests follow `docs/22_test_quality.md`; each witness must exercise production
selection/dispatch or the actual public transport seam and assert observable
values, not only success. Proposed new test names are assigned by the packets;
the scenarios below define what they must falsify.

**Routing prerequisite**

- Distinct painted variants of the same object/base-region with different
  effective settings reach preparation and consumption distinctly. Permuting
  input region order cannot select a different variant's configuration. This
  names the root cause: loss of `variant_chain` in a delivery-map key.
- Modifier children, merged perimeter sources, and multiple held roles are
  attributed correctly. A module selected only on a region or layer is not
  replaced by the global default or a short-name-only predicate.
- Support carriers with no corresponding slice identity, raft invocations, and
  anchored invocations are represented and selected through production rules.
  Missing optional support products have an explicit tested outcome. Do not
  declare an unverified target category covered by a vacuous empty collection.

**Pilot and framework**

- A real guest prepares known named data, its preparation instance/store is
  discarded, and fresh Layer calls read different ranges and produce independently
  expected output. Repeat through the native adapter with equivalent inputs.
- An unrelated owner using the same piece name cannot observe the first owner's
  bytes. Missing owner-ready state fails; an explicitly ready empty plan succeeds
  as no-work. Missing optional piece and missing required piece are distinguishable.
- Duplicate name, invalid range, undeclared input, failed/trapped preparation,
  invalid declaration/export, and poisoned-output cases fail at the intended
  seam without publishing partial state. Module-returned failure details survive.
- Selecting an external override uses its preparation and its consumer together.
  An unused prepared module is neither prepared nor dispatched; a module selected
  only through a non-default region is prepared. No test injects a ready plan to
  claim activation coverage.
- Concurrent Layer reads are stable and print isolation prevents cross-print
  reuse. Normal, failure, and cancellation paths release retained storage after
  consumers finish. Layer-arena execution traces still prove run-to-completion.
- Derive module-backed Layer-stage coverage from the schema authority. Compile
  prepared/plain shapes for that surface; execute representative real adapters
  with distinct input families, including a non-infill prepared module. A
  compile witness proves linking only, not dispatch behavior.
- A pre-existing ordinary guest loads through the new framework host. Artifact
  freshness distinguishes a preparation-contract change from an unrelated stage
  package; use checker exit status rather than absence of a `STALE` message.
- Measure payload/name retention, transfers, and representative whole-print
  preparation versus layer-read costs. Clearly distinguish measured values from
  accounting totals and any unmeasured allocator/process overhead.

**Plan visual-debug**

- A prepared module emits known points, polylines, and polygons with a hole into
  a requested view; assert typed coordinates, scene/class identity, and rendered
  placement. Use shared framing with an ordinary stage capture.
- The module emits the same semantic plan and ordinary stage output with capture
  enabled and disabled. No diagnostic projection is retained when unrequested.
- Preparation-only capture runs the whole-print preparation and no Layer arenas;
  a selected-layer render records truthful execution versus render selection.
- Unknown owner/view, inactive owner, invalid layer, undeclared projection,
  missing requested scene, and invalid geometry fail without a successful
  partial bundle. Explicit empty scenes are successful and clearly labeled.
- Rendered-layer subsets preserve full-plan framing and class assignment.
  Snapshot storage is shared once across rendered layers, not cloned wholesale
  per image. Existing request/schema behaviors remain covered.
- Lightning's migration view shows relevant planned geometry and its relation to
  later emitted infill. The framework witness uses generic diagnostic primitives
  and requires no lightning-specific renderer arm.

**Lightning gate and migration**

- Establish an independent, reproducible reference for canonical planning-domain
  construction and its relation to final fill surfaces, including temporary
  anchor adjustments. Validate the comparator against a deliberately wrong
  planning domain. A visual resemblance or a self-recorded current-host baseline
  is not this oracle.
- Verify tree topology/grounding and emitted geometry across the supported wall
  strategies, holes, changing island topology, region/paint/modifier selection,
  and effective config changes. State the fixture's expected physical/canonical
  relationship before choosing numerical tolerances.
- Verify preservation of shell/internal-solid/internal-bridge inputs through
  paint reconstruction, and guest exposure of every field the algorithm consumes.
- Build and execute the portable preparation kernel through WASM and native
  adapters. A source grep showing no host-only imports is not portability proof.
- Slice through real module activation, preparation, Layer consumption, linking,
  and final output. The existing sampler test that hand-injects tree segments is
  insufficient. Assert correct module/region ownership and meaningful nonempty
  output where expected.
- After migration, verify direct retirement of the host producer and typed
  contract plus the intended compatibility failure for affected old guests.

### Implementation verification discipline

Each generated packet must name its narrow runnable test targets and required
features after inspecting the applicable Cargo manifests. Use the repository's
guest-freshness gates and tee all Cargo test output to `target/test-output.log`.
WIT changes require building test targets; check/clippy gates use `--all-targets`.
Run the literal and test-quality gates required by `AGENTS.md`. The workspace
test suite is reserved for authorized packet-close acceptance after narrower
verification passes, via the enforced `cargo xtask test` entry point. This design
interview has not run these implementation gates.

## Verified architectural anchors

- `docs/01_system_architecture.md`, **PrePass Stage Order**, distinguishes the
  scheduler's validation order from runtime execution order. Shell classification
  and paint segmentation already enrich the committed whole-print slice before
  late planner dispatch. Preparation should build on those products.
- `docs/05_module_sdk.md`, **Module State Lifecycle**, specifies reconstruction
  through `from_config` for each ordinary stage call. Prepared data must have an
  explicit lifetime independent of those module values.
- ADR-0045 (`docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md`)
  requires independently versioned per-stage packages and declared typed
  instantiation. A preparation capability needs an explicit extension to that
  decision, including host-owned resource identity and compatibility behavior.
- ADR-0056 (`docs/adr/0056-integrated-modules-native-dispatch.md`) requires one
  module ingestion/selection model with equivalent native and WASM dispatch.
- `slicer_module` and `generate_slicer_module_impl`
  (`crates/slicer-macros/src/lib.rs`) enforce and generate the current single
  scheduled-stage surface. `SlicerModuleSchema`
  (`crates/slicer-schema/src/lib.rs`) describes that export surface.
- `NativeStageEntry` (`crates/slicer-sdk/src/native.rs`) carries the current
  ordinary dispatch entry; `LoadedModule`
  (`crates/slicer-scheduler/src/manifest.rs`) has no preparation declaration.
- `PrepassStageInput` (`crates/slicer-wasm-host/src/binding.rs`) exposes committed
  whole-print slices, region mapping, and selected support products to the runner.
  It does not currently carry `SurfaceClassificationIR`; host availability is
  distinct from guest exposure.
- `execute_prepass_after_region_map` (`crates/slicer-runtime/src/prepass.rs`)
  currently gates lightning generation with the global default
  `sparse_fill_holder == "lightning-infill"`. The replacement must derive
  activation from effective selection rather than a lightning-name special case.

## Correctness questions requiring evidence

### Visual debugging of private plans — scope agreed

The user asked whether `visual-debug` for plans belongs in this effort after Q17
and selected its inclusion in Q18. The extension is an opt-in, module-owned visual
projection of a prepared plan into standard diagnostic geometry. The host renders the projection
without decoding the private plan format. Module authors can expose branches,
outlines, or grounding markers while retaining ownership of the actual payload.

`docs/19_visual_debug.md`, **Support-family visual inspection**, already describes
comparing a support plan with later `Layer::Support` output. Its **Tap Classes And
Execution Closure** section distinguishes whole-print PrePass captures from
selected-layer arena execution. These are precedents, not evidence that generic
private-plan visualization currently exists.

The extension must preserve versioned requests, deterministic capture/manifest
output, shared framing, named rejection of unsupported views, and truthful
execution-closure reporting. A preparation-only render needs whole-print
preparation and then selected-layer visualization; it must not create unfinished
Layer arenas. A visual capture is diagnostic evidence, not a canonical-parity
oracle. Q21 settles capture timing: preparation supplies an opt-in diagnostic
snapshot which becomes visible only with the successful plan commit. It is
captured post-commit, matching other PrePass taps. Q20 limits initial views to XY.
The consolidated contract above defines the diagnostic primitives and structured
owner/view selector; the visual packet must encode those in the canonical schema
and prove compatibility. The generic mechanism is required for framework closure;
lightning's projection is required when its migration closes.

### Selection and configuration must agree with consumption

Preparation targets describe selected work and its identity, not a prediction of
the final geometry emitted by intervening Layer stages. Target projection must
cover the ordinary stage's actual selection semantics, including merged perimeter
sources and any support-carrier or anchored invocation that can reach that module.
The separate routing prerequisite must deliver the concrete common target view
against the semantic requirements above; framework integration is blocked on its
acceptance rather than guessing at that view.

Source inspection found a delivery gap that must not become the new contract:
`WasmRuntimeDispatcher::run_stage`
(`crates/slicer-wasm-host/src/dispatch.rs`) resolves a region's configuration using
the complete `RegionKey`, then stores the fields in `config_fields_per_region`
under only `(object_id, region_id)`. Distinct painted variants can overwrite the
same bucket. Its held-claim lookup also ignores `variant_chain`. A shared
preparation/consumption projection must preserve the intended region identity and
configuration contract; copying a last-variant-wins artifact is not a correctness
criterion. Q14 places the required routing/configuration repair in a separate
prerequisite workstream. Its acceptance must establish full-identity delivery and
expose a reusable selection/configuration projection before framework integration
closes.

`support_carrier_regions` (`crates/slicer-wasm-host/src/dispatch.rs`) tests whether
an identity is present in the slice, not whether its polygons are empty. Carriers
can therefore be derived after the support plan commits without predicting later
polygon changes. `perimeter_source_regions`
(`crates/slicer-wasm-host/src/marshal/mod.rs`) merges modifier children back into
base sources using the matching variant chain. These existing semantics should
be reused rather than reimplemented independently for preparation.

### Private piece semantics

Q8 and Q10 place plan addressing inside the module. Host validation can establish
owner access, unique piece names, valid byte ranges, resource lifetime, and a
successful publication. It cannot infer required geometric coverage from opaque
piece names or payloads. The owner module must validate its required pieces and
decode its own format; a missing required piece cannot silently become a valid
empty geometry result.

### Preparation failure versus successful no-work

Q9 chooses preparation as a required dependency. A Layer module must not silently
run with a missing plan after preparation failure. The contract must preserve a
successful no-work outcome separately from missing/failed preparation. The
consolidated contract records ready status separately from collection contents.

`docs/04_host_scheduler.md`, **Error Handling Policy**, specifies degraded
continuation for ordinary non-fatal module errors, while documenting the current
fatal-only dispatch limitation. `WasmRuntimeDispatcher`'s runner implementations
(`crates/slicer-wasm-host/src/dispatch.rs`) also map native entry failures to
phase-specific fatal errors. The new preparation prerequisite must have explicit
error semantics rather than relying on that existing limitation or claiming it
has already been fixed. The broader ordinary-stage error-policy repair is not an
implicit consequence of this design decision.

### Lightning planning geometry

Late PrePass contains useful shell, bridge, paint, and region information. Actual
wall-inset `sparse_infill_area` is established later by
`sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`). The current
`generate_lightning_trees` (`crates/slicer-core/src/algos/lightning/mod.rs`) uses
region polygons and print-wide configuration; that is an implementation
limitation, not a complete definition of available PrePass information.

Planning from PrePass geometry and subsequently applying/clipping against the
current fill area is a candidate design. Its correctness has not been established
by a runtime or canonical comparison. Packet acceptance must distinguish generic
transport correctness from lightning algorithm/input fidelity.

Direct inspection of canonical `Generator::generateInitialInternalOverhangs` and
`Generator::generateTrees` (`src/libslic3r/Fill/Lightning/Generator.cpp`) shows that
they collect `stInternal` and `stInternalVoid` fill surfaces across the object's
regions. `PrintObject::bridge_over_infill` (`src/libslic3r/PrintObject.cpp`)
temporarily expands solid anchor areas into sparse areas, builds the lightning
generator from the modified surfaces, then restores the original surfaces.
`Filler::_fill_surface_single` (`src/libslic3r/Fill/FillLightning.cpp`) later
samples and clips using its supplied fill expolygon. Therefore acceptance must
verify the canonical relationship between planning domains and final fill domains;
requiring them to be literally identical would itself misstate canonical behavior.
Conversely, final clipping alone is not evidence that different planning domains
preserve equivalent tree topology or grounding.

The local canonical checkout used for this source inspection is
`D:/slicerProject/pinch_n_print_cli_2/OrcaSlicerDocumented/`. This is a local source
location, not a portable fixture dependency; implementation must establish a
reproducible oracle with canonical file/function provenance rather than requiring
that machine-specific path.

The current driver also groups by `(object_id, region_id)` and pairs islands
across layers by polygon-vector position in `generate_lightning_trees`
(`crates/slicer-core/src/algos/lightning/mod.rs`). Canonical `generateTrees`
collects all eligible outlines for a layer instead. Migration must audit these
differences; relocation alone is not proof of algorithm correctness.

The lightning module is currently hidden behind `host-algos` in
`crates/slicer-core/src/algos/mod.rs`; that feature enables `rayon` and
`boostvoronoi` in `crates/slicer-core/Cargo.toml`. The lightning files directly use
polygon operations, geometry helpers, and IR types. A module-owned portable
kernel is plausible, but has not been compiled for WASM in this investigation;
the extraction must not drag unrelated host-algorithm features into the guest.

Earlier source investigation also identified incomplete preservation of
`internal_solid_fill` and `internal_bridge_areas` during paint-region
reconstruction (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`). The
eventual migration plan must verify and resolve the relevant preservation gap.

### Executable capability proof

An earlier temporary WIT encode/decode probe demonstrated representation of opaque
bytes and a read-only host resource. It did not demonstrate a working combined
preparation-plus-Layer artifact, shared resource identities during real host
instantiation, or runtime/memory performance.

For the chosen one-artifact design, the first implementation gate is an
executable witness that prepares data, discards the preparation instance, and
consumes the saved bytes from fresh ordinary Layer invocations. Its required
contract and native/WASM coverage are specified above; execution remains a gate,
not evidence already obtained by this interview.

## Completion status

This document is a packet-generation input. The criteria below were satisfied as
of the Q23 approval:

- [x] Every design-tree branch is resolved with a decision or an explicit
  executable feasibility gate and a specified consequence of failure.
- [x] The public interface, inputs, ownership, ordering, errors, configuration,
  compatibility, and access rules are defined precisely enough to implement
  without another design interview.
- [x] Migration and retirement work for the lightning-specific host path is
  defined.
- [x] Acceptance witnesses cover effective region selection, private access,
  fresh-instance consumption, native/WASM behavior, and lightning correctness.
  Transport tests are explicitly excluded as algorithm-parity evidence.
- [x] Consequential architectural decisions are recorded in ADR-0066; glossary
  terms were added to `CONTEXT.md`.
- [x] The user confirmed the complete design (interview Q23).

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | routing-identity-prerequisite | Deliver the shared full-identity selection/configuration projection and fix variant-chain delivery so preparation and ordinary consumption target and configure identically. | - | - | pending | - |
| 2 | preparation-capability-pilot | Prove the same-artifact preparation capability end to end: combined exports, resource/import composition, fresh-instance named-piece reads, native parity, ordinary-guest compatibility, and measured retention/transfer costs. | - | - | pending | - |
| 3 | preparation-framework | Implement manifest declaration/validation, private plan storage, activation, runner adapters, declaration-gated whole-print input views, failure semantics, and lifecycle diagnostics for preparation-capable Layer modules. | - | #1, #2 | pending | - |
| 4 | preparation-plan-visuals | Add optional declared XY diagnostic projections published atomically with the plan and captured/rendered through the existing visual-debug request/capture/manifest path. | - | #3 | pending | - |
| 5 | lightning-geometry-gate | Establish the reproducible canonical planning-domain/grounding oracle and portable module-owned kernel proof, and record the verified input strategy in the source plan. | - | - | pending | - |
| 6 | lightning-migration | Migrate lightning to module-owned preparation and directly retire the host producer, `LightningTreeIR`, and `lightning-tree-segments` accessor. | - | #3, #4, #5 (gate outcome recorded in this plan) | pending | - |

Row 6 is not generatable until row 5 records the verified input strategy in this
plan. The plan's "Framework acceptance" workstream is realized by the acceptance
criteria of rows 1–4, not by a separate closure packet.
