# Implementation Plan: traditional-support-family

## Execution Rules
- Work one atomic step at a time; map every step to `TASK-333`.
- Use TDD, implementation, then the narrowest falsifying validation.
- Do not activate while TASK-331 blockers remain unresolved.

## Steps
### Step 1: Establish traditional family package and claims
- Task IDs: `TASK-333`
- Objective: add `traditional-support-planner` identity and pair it with `traditional-support` using `support-family:traditional`.
- Precondition: current renderer claim is verified at `traditional-support.toml:9-18`.
- Postcondition: scheduler selects a matched traditional planner/renderer pair for `normal*`/`classic*` aliases.
- Files allowed to read: traditional/tree manifests; delegated scheduler claim validation.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support-planner/Cargo.toml`; `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`; `modules/core-modules/traditional-support/traditional-support.toml`.
- Files explicitly out of bounds: algorithms, generated WIT, packet 213, status ledger.
- Expected dispatch: locate claim-pair validation and manifest integration target; scope `crates/slicer-scheduler`; return `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: plan §6; delegated `docs/03_wit_and_manifest.md` summary.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration support_family_selection -- --exact`
- Exit condition: both role claims and `support-family:traditional` are present without global fallback selection.

### Step 2: Implement contact, propagation, interface, obstacle, and termination planning
- Task IDs: `TASK-333`
- Objective: plan complete structural traditional bodies from assigned demands and exact-Z host geometry.
- Precondition: TASK-331 exports `SupportAnalysisIR` and the host exact-Z support query service required by this planner.
- Postcondition: contacts propagate downward, interfaces use configured layers/pattern, obstacles are avoided, and termination is valid.
- Files allowed to read: `modules/core-modules/traditional-support-planner/src/lib.rs` bounded ranges; delegated region/query locations.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support-planner/src/lib.rs`; `modules/core-modules/traditional-support-planner/Cargo.toml`; `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`.
- Files explicitly out of bounds: host exact-Z implementation, tree planner, renderer implementation.
- Expected dispatch: locate config keys and region geometry consumers (`polygons`, `overhang_areas`); scope support modules and SDK; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan §§4, 5, 10; delegated SupportMaterial locations.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2095`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2592`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:1451`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2760`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2953`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3068`/`:3070`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3074`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3106`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:3208`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:480`/`OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp:47`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:2735`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:523`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:555`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:1980`, `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp:487` (delegated).
- Verification: `cargo test -p traditional-support-planner --test traditional_family_tdd contact_area_planning -- --exact`; `cargo test -p traditional-support-planner --test traditional_family_tdd base_interface_obstacle -- --exact`; `cargo test -p traditional-support-planner --test traditional_family_tdd anchored_termination -- --exact`
- Test-target wiring: add `[[test]] name = "traditional_family_tdd" path = "tests/traditional_family_tdd.rs"` in the new planner manifest; this binary drives AC-1, AC-2, AC-3, AC-6, and AC-N1.
- Exit condition: one connected planned structure can reach a valid model/plate termination without obstacle overlap.

### Step 3: Migrate structural output and polygon-only renderer
- Task IDs: `TASK-333`
- Objective: emit traditional semantic roles and render only planned polygons through anchored support events.
- Precondition: TASK-331 structural fields and TASK-330 anchored hooks are available.
- Postcondition: `SupportIR` retains family/body/demand/role identity and renderer never calls region eligibility accessors.
- Files allowed to read: traditional renderer/tests; Layer::Support WIT and `LayerModule` locations delegated.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support/src/lib.rs`; `modules/core-modules/traditional-support/Cargo.toml`; `modules/core-modules/traditional-support/tests/traditional_family_tdd.rs`.
- Files explicitly out of bounds: tree renderer, host schema migration, generated bindings.
- Blast-radius discipline: inventory every traditional `SupportIR`/plan literal and flat-path assertion before editing; update all affected literals/assertions in this step.
- Expected dispatch: locate traditional output consumers and support postprocess/ironing stage wiring; scope module and runtime tests; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan §§2, 5, 9; delegated `docs/02_ir_schemas.md` summary.
- OrcaSlicer refs: none required for identity plumbing.
- Verification: `cargo test -p traditional-support --test traditional_family_tdd planned_polygon_renderer -- --exact`; `cargo xtask build-guests --check`
- Test-target wiring: add `[[test]] name = "traditional_family_tdd" path = "tests/traditional_family_tdd.rs"` in the renderer manifest; the binary drives AC-4 and AC-N2.
- Exit condition: renderer emits paths only from planned polygons and retains attribution through support postprocess.

### Step 4: Enforce invalid, missing, declined, and disabled paths
- Task IDs: `TASK-333`
- Objective: prove complete-body rejection, structured decline, missing-plan failure, and no fallback filler.
- Precondition: Steps 1-3 compile against TASK-331 draft shapes.
- Postcondition: invalid/missing traditional plans are observable failures or degraded unmet demand, never silently filled.
- Files allowed to read: traditional tests and real-slice runtime fixture; delegated test target wiring.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`; `modules/core-modules/traditional-support/tests/traditional_family_tdd.rs`; `crates/slicer-runtime/tests/integration/traditional_support_family.rs`.
- Files explicitly out of bounds: mixed-family routing, packet 213, status ledger.
- Expected dispatch: locate real end-to-end slice driver and test aggregator; scope runtime integration tests; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan invariants 1, 3, 5, 13, 14.
- OrcaSlicer refs: none.
- Verification: `cargo test -p traditional-support-planner --test traditional_family_tdd invalid_body_rejected -- --exact`; `cargo test -p traditional-support --test traditional_family_tdd mismatched_or_missing_plan -- --exact`
- Exit condition: no invalid, missing, declined, or disabled input creates fallback support paths.

### Step 5: Register the runtime integration target
- Task IDs: `TASK-333`
- Objective: mount the traditional runtime fixture in the existing integration aggregator and declare the Cargo target that owns that aggregator.
- Precondition: Step 4's runtime fixture exists and the real aggregator remains `crates/slicer-runtime/tests/integration/main.rs`.
- Postcondition: `cargo test -p slicer-runtime --test integration` builds the single aggregator binary and includes `traditional_support_family`.
- Files allowed to read: `crates/slicer-runtime/tests/integration/main.rs`; `crates/slicer-runtime/Cargo.toml`.
- Files allowed to edit (at most 2): `crates/slicer-runtime/tests/integration/main.rs`.
- Files explicitly out of bounds: mixed-family routing, packet 213, status ledger.
- Expected dispatch: add `mod traditional_support_family;` beside the existing integration submodules; the `integration` Cargo target is already registered by TASK-330 (packet 219 Step 0) — confirm it exists, do not re-add; return `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: `crates/slicer-runtime/tests/integration/main.rs` aggregator and `crates/slicer-runtime/Cargo.toml` target declarations.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test integration --no-run`.
- Exit condition: the fixture is registered and the named `integration` test binary is planned and buildable.

## Per-Step Budget Roll-Up
| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | claims |
| Step 2 | M | planning geometry |
| Step 3 | M | structural renderer migration |
| Step 4 | M | rejection/e2e evidence |
| Step 5 | S | integration target wiring |

## Packet Completion Gate
- All steps and exits complete; every AC command passes.
- TASK-331 exact-Z and WIT blockers are resolved before implementation status.
- Guest freshness and model-backed traditional support taps are inspected downstream.

## Acceptance Ceremony
- Re-dispatch all AC commands and inspect traditional support at matched heights for `tmp/SupportTest.stl`.
