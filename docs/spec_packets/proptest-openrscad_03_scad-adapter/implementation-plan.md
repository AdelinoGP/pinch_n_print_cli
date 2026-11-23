# Implementation Plan: scad-adapter

Implement the public contract in `packet.spec.md` without adding Proptest or SDK/runtime dependencies. Pin and audit before adapter-specific compatibility assertions. Each step leaves both default-off and enabled configurations compiling; a failed feasibility gate leaves SCAD adoption incomplete, not the direct property queue blocked.

## Step 1: Select the real pin and establish Windows build evidence

- Objective: verify the chosen engine sources and native prerequisites before binding the adapter to an assumed API.
- Surface:
  - `Cargo.toml`, `Cargo.lock`: add planned workspace member `crates/slicer-scad`, coherent immutable engine dependency entries and resulting actual resolution.
  - New `crates/slicer-scad/Cargo.toml`, `src/lib.rs`: begin default-off `openrscad` boundary with all native engine dependencies optional and no SDK/runtime/Proptest edge. Use the repository lints/package policy.
  - New `docs/scad/windows-feasibility.md`: record baseline source/dirty-content state, current Rust/MSRV policy, CMake/MSVC identities, dependency graphs, enabled/default build commands/statuses and separate measured compile context.
  - New `docs/scad/openrscad-compatibility.md`: audit the selected `parse`, expression/program budget/mode entrypoints, resolver, all diagnostic paths, exact rendering, dimensional dispatch, native worker stack, RNG/font/resource behavior and conversion representations.
- Approach: select an actual `matthova/openrscad` revision; if a fork is needed, preserve its upstream base, patch order/digest and immutable resolution. Do not switch the repository toolchain or use upstream Windows CI as this project's result. Bootstrap a real Windows library call using the pinned public APIs, not an external executable subprocess product importer.
- Verify: `cargo build -p slicer-scad --no-default-features`; `cargo build -p slicer-scad --features openrscad`; default workspace and enabled/default adapter metadata/tree evidence; record full logs and actual status. Use stub-free compiled boundaries as later types arrive, not placeholder acceptance tests.
- Done when: a real immutable coherent resolution builds on Windows; gaps in combined budget/options, contour diagnostics or resource isolation are named against actual selected-pin functions.

## Step 2: Add the stable data/error contract and feature-off behavior

- Objective: make downstream ingestion/scripted packets consume one opaque-to-upstream interface.
- Surface:
  - New `crates/slicer-scad/src/lib.rs`: all planned `ScadSource`, `ScadParameter`, `ScadLimits`, `ScadEvalOptions`, geometry/diagnostic/engine/provenance/output/error types and exact `engine_info`/`evaluate` signatures from the spec.
  - New `crates/slicer-scad/tests/scad_feature_contract.rs`: actual disabled/enabled API/engine identity assertions; no whole-file off-feature gate that silently runs zero tests in an acceptance configuration.
  - `crates/slicer-scad/Cargo.toml`: serde and any compatible source-digest support; all engine edges remain optional/native.
- Approach: derive the specified data traits; make options validation independent of serde construction. Disabled `evaluate` fails before parsing/access. Select bounded documented default limits from the Windows pilot; continue measuring/revise with evidence in step 6 rather than claiming unmeasured defaults accepted.
- New contract: the exact public Rust block in the spec, including `ScadGeometry::{Empty,Contours,Mesh}` and every `ScadErrorKind`, is authoritative. No public backend/preview/resolver escape hatch.
- Fallout: all types are new, so there are no old literal/version assertions to migrate. New `ScadLimits`, `ScadDiagnostic`, `ScadProvenance` and `ScadError` have watched named-field counts: test construction uses FRU over documented defaults where meaningful, otherwise explicit `// exhaustive: ...` waivers naming the pinned data contract. `ScadEvalOptions` composes bounded `ScadLimits::default()`; do not invent unsafe defaults for errors/provenance just to satisfy FRU.
- Verify: `F`, and its enabled-feature counterpart, using the log/status wrapper in the spec; `cargo check -p slicer-scad --all-targets` in both feature states.
- Done when: feature-off behavior is a tested public result, enabled identity matches the real dependency pin, and no upstream Rust type is exposed.

## Step 3: Close evaluation, diagnostic and resource gaps at the pin

- Objective: implement strict semantics before allowing any geometry conversion success.
- Surface:
  - New `crates/slicer-scad/src/engine.rs`: joined worker, source/AST/geometry/facet limits, expression/name handling, one shared finite fuel allowance, exact mode, dimensional analysis, deny-all resolver, local RNG/cache state and explicit extrusion.
  - New `crates/slicer-scad/src/diagnostics.rs`: one fail-closed classifier reached by program evaluation, mesh render, contour/projection/lowering and extrusion.
  - Selected upstream/fork source: focused combined-mode/override/fuel and budgeted-expression APIs, diagnostic-returning contour entry, plus resource/seed seams where inspection proves necessary. Record actual symbols and contribution/patch provenance in compatibility docs; no fictional public combined API is assumed.
  - New `crates/slicer-scad/tests/scad_adapter_contracts.rs`: parameter/exact-mode/resource/dimension/diagnostic/isolation fixtures.
- Approach: evaluate all override expressions in order under the same budget, retain originals/last-wins semantics, forbid injected trailing statements, and audit native bypasses. Exact render remains exact even for explicit `$preview=true`. Capture denied requests so a sibling primitive cannot hide them. Classify root-recursion partial output and render approximation/fallback before conversion. Pure 2D without height remains contours; true mixed dimensions fail. Route public `evaluate` through these actual helpers; tests of detached classifiers/resolvers alone are insufficient.
- Verify: `A` for `scad_parameters_budget_and_exact_mode`, `scad_rejects_resources_even_with_remaining_geometry`, `scad_rejects_partial_approximate_degraded_and_unknown`, `scad_informational_diagnostics_and_state_isolation` and `scad_rejects_invalid_options_and_dimensions`; `U` for `scad_diagnostic_channels_fail_closed`, each using log capture. Compile every new test target with `required-features = ["openrscad"]` and verify named discovery/execution.
- Done when: real pinned engine calls prove all accepted/rejected outcomes, every diagnostic-producing contour path is observable, and upstream focused patch tests plus adapter tests pass. No universal warning suppression or hidden resource access remains.

## Step 4: Implement checked geometry conversion and independent counterexamples

- Objective: return valid PnP geometry without saturating, dropping or repairing damaged data.
- Surface:
  - New `crates/slicer-scad/src/conversion.rs`: checked f64-to-grid mapping using `UNITS_PER_MM`, even-odd forest/winding reconstruction, pre/post-quantization validation; checked f64→f32 mesh conversion, flat indices, exact-position incidence/orientation validation and cardinality limits.
  - Registered private unit-test modules in `conversion.rs` / `lib.rs`: raw upstream contour/mesh counterexamples can drive otherwise unconstructible conversion faults without making upstream types public.
  - `crates/slicer-scad/tests/scad_adapter_contracts.rs`: public holed/nested/disconnected/extruded/3D cases and public quantization failure witnesses.
- Approach: independently enumerate expected forest ownership and point membership for shuffled/reversed nested fixtures; derive primitive/extrusion bounds, area and volume from dimensions. Prove all arithmetic intermediate ranges or use checked/exact arithmetic; do not use the production grouping algorithm as expected output. Normalize only redundant exact representation, not epsilon welding or dropped degenerates. Document coordinate and mm³ tolerance derivations before assertions.
- Fallout: `Point2`, `Point3`, `Polygon`, `ExPolygon` and `IndexedTriangleSet` shapes are consumed unchanged, with no schema/WIT bump; new fixture sites obey `docs/21_data_defaults_and_fixtures.md`. Update any earlier adapter expected values impacted by the canonical-correct checks in the same step, never loosen validity.
- Verify: `U` for `scad_conversion_contour_forest` and `scad_rejects_conversion_damage`; `A` for `scad_even_odd_contours_and_explicit_extrusion` and real conversion negatives; `cargo xtask check-literals`; `cargo xtask check-test-quality --report`.
- Done when: exact checked outputs have independently justified topology/numerics; each named conversion corruption is rejected instead of repaired, including after f32/subnormal normalization and integer quantization.

## Step 5: Prove Windows native containment and fresh evaluation state

- Objective: establish the supported worker stack and honest test timeout/abort behavior without changing the library-based product API.
- Surface:
  - New `crates/slicer-scad/tests/scad_process_limits.rs`: supervising parent and explicitly selected child test modes; register standalone target with required engine feature. Persist source/options/engine identity before a dangerous child call and retain full native exit status.
  - `crates/slicer-scad/src/engine.rs`: validated worker stack, join/error handling and clean per-call state.
  - `docs/scad/windows-feasibility.md`: actual Windows seeded/concurrent/recursion/process observations, resolved stack and process limits.
- Approach: cover parse, override evaluation, evaluator and render inside killable child supervision; prove deadline expiry and abort detection using controlled children and run a real bounded adapter fixture there. Join/kill children and prove no worker survives. No sleep-driven correctness assertion, detached timed-out worker, per-case panic hook or catch-unwind-only native containment. Record retained cases as unminimized when no minimization occurred.
- Verify: `L` for `scad_process_supervision_retains_repro_and_joins`; rerun the bounded isolation fixture under `A` with the canonical log wrapper; inspect archived artifacts rather than rerunning to recover truncated logs.
- Done when: observed Windows failures distinguish evaluator fuel from process timeout/abnormal termination and leave usable pre-execution reproducers; the product API promises no render deadline.

## Step 6: Accept the adapter prerequisite, wire the required lane and document handoff

- Objective: close this packet's adapter-prerequisite portion of G-SCAD only with real Windows evidence and dependency isolation; leave recipe generation/shrink measurements, final eligible property profiles and full G-SCAD closure to #4.
- Surface:
  - `.github/workflows/ci.yml`: new `jobs.scad-adapter-windows`, required feature-off/enabled narrow gates, CMake/MSVC setup, per-run log archival and failure artifact uploads; preserve existing jobs.
  - New `docs/scad/adapter.md`, compatibility/evidence docs; `docs/08_coordinate_system.md` checked external-f64 section; `docs/01_system_architecture.md` ownership link.
- Approach: measure representative adapter evaluation/exact-render costs for AC-3's holed-2D/indexed-3D fixtures separately from compilation; select and record finite adapter defaults/contract-test settings, not final eligible property profiles. Record the adapter-prerequisite disposition and pending #4 recipe generation/shrink/profile obligations in `docs/scad/windows-feasibility.md`. Preserve immutable engine/patch provenance and compatibility classification at that pin. Verify default workspace/adapter/CLI/discovered guest normal/build graphs and enabled adapter absence of SDK/runtime/Proptest. A cold or default build claim must name the measured build context, not infer cost from graph size.
- Verify: all `F`/`A`/`U`/`L` checks on Windows and all default/enabled build/check/clippy/literal/test-quality commands in the spec. No workspace suite is required; broad/guest tests, if needed for an actual discovered dependency change, use the gated xtask entrypoint and freshness rules. Independently preflight/closure-review with `/spec-review docs/spec_packets/proptest-openrscad_03_scad-adapter` under the normal workflow.
- Done when: this packet's adapter-prerequisite acceptance criteria and representative evaluation/exact-render measurements pass, the required Windows job runs the delivered checks, docs checks pass and downstream can consume the public block unchanged. This packet can be implemented without waiting for #4; full G-SCAD remains incomplete until #4 supplies recipe generation/shrink measurements and final eligible property profiles. If pin/patch/build/diagnostic feasibility fails, document the exact blocked adapter obligation and do not mark this packet implemented; direct-domain work continues independently.
