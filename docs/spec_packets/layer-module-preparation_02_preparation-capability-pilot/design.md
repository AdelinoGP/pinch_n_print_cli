# Design: preparation-capability-pilot

## Controlling Code Paths — grounded pre-existing seams

- `StageSpec`, `STAGES`, `TIER_LAYER` (`crates/slicer-schema/src/lib.rs`) enumerate module-backed Layer methods/packages/worlds. Derive coverage using nonempty package, not the narrative historical stage counts; host-only hooks are not invented exports.
- `slicer_module`, `detect_stage_methods`, `generate_slicer_module_impl`, `build_layer_infill_glue`, `emit_world_preamble` (`crates/slicer-macros/src/lib.rs`) select one stage and generate ordinary guest/native glue. The attribute currently ignores its token argument; the new opt-in must actually be parsed and validated.
- `LayerModule::from_config`, `run_infill`, `run_path_optimization` (`crates/slicer-sdk/src/traits.rs`); `NativeStageEntry`, `NativeLayerRequest`, `NativeLayerResponse` (`crates/slicer-sdk/src/native.rs`) carry the existing native seam. Do not add fields to these envelopes.
- `host::layer_perimeters` (`crates/slicer-wasm-host/src/host.rs`) defines canonical shared resource identities; `host::layer_infill` aliases shared interfaces via `bindgen! with`. Existing plain `infill-module` / `path-optimization-module` WIT worlds import common/config/IR resources and export only their stage.
- `production_classic_perimeters_instantiates_with_layer_linker` (`crates/slicer-wasm-host/tests/contract/production_guest_smoke_tdd.rs`) provides the actual ordinary typed-load driver; contract aggregator `mod production_guest_smoke_tdd` (`crates/slicer-wasm-host/tests/contract/main.rs`) verifies its real home.
- `discover_guests` (`xtask/src/build_guests.rs`) scans direct test-guest directories requiring `Cargo.toml`, cdylib and wit-bindgen; it stages `<directory>.component.wasm` at test-guests root and uses the shared guest target. `resolve_stage_from_world`, `compare_worlds`, `macro_embedded_wit_files`, `SHARED_PACKAGES` (`xtask/src/wit_verify.rs`) currently distinguish one stage from five shared packages, so a preparation package needs explicit non-stage classification and export checking.

## Architecture Constraints

- ADR-0066 explicitly extends ADR-0045 with a declared independently versioned capability, not an additional scheduled stage. ADR-0056 requires same-source native/WASM calls. Pilot fixture registration is not production ingestion/override/eligibility proof.
- Plain worlds and shared imported WIT remain byte-for-byte source unchanged; no existing schema/version bump, no `SlicerModuleSchema` or `StageSpec` field addition. New sidecars avoid full existing struct-literal and version-assertion fallout. New struct literals follow docs/21; production exhaustive/test FRU or reasoned waiver.
- Imported capability resources use host backing identities and fresh ResourceTables; only immutable owner-bound bytes and readiness survive, never guest instance/resource index. A native call-local serialized facade has the same bounds/errors as guest imports and is restored on failure/unwind.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface — authoritative implementation edit inventory

All names/paths marked NET-NEW are proposed, not claimed existing. Extra files are necessary to span WIT, SDK/macro, both guest artifacts, registered actual-driver tests and freshness without exceeding three edits per step.

| File | Selected change |
| --- | --- |
| NET-NEW `crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit` | Capability types/errors/interfaces and nine schema-derived composed Layer worlds; keep ordinary deps unchanged. |
| `crates/slicer-schema/src/lib.rs` | NET-NEW `PreparationExportSchema` sidecar and capability qualified export constant; leave existing records/constants unchanged. |
| NET-NEW `crates/slicer-sdk/src/preparation.rs` | Required `LayerPreparation`, facade types, native scoped read binding; WASM callback adaptation seam; no module-owned codec in SDK. |
| `crates/slicer-sdk/src/lib.rs` | Register public preparation module; no broad prelude changes. |
| `crates/slicer-sdk/src/native.rs` | NET-NEW `NativePreparedStageEntry`, independent of `NativeStageEntry` and existing request/response shapes. |
| `crates/slicer-sdk/tests/layer_module_tdd.rs` | Steps 2–3 required-trait, scoped serialized-read and paired native-entry tests; existing SDK test target requires feature `test`. |
| `crates/slicer-macros/src/lib.rs` | Parse `preparation`; compose selected ordinary world and capability; emit same-source export/native pointers and sidecar schema; required trait invocation causes missing-method compilation failure. Keep existing no-opt-in generated exports. |
| `crates/slicer-macros/build.rs` | Watch new canonical embedded WIT as well as existing files. |
| `crates/slicer-macros/tests/slicer_module_tdd.rs` | Opt-in parser/one-stage regression tests; not behavioral transport evidence. |
| `xtask/src/wit_verify.rs` | Classify preparation as capability, retain exactly one scheduled-stage candidate, compare embedded capability types and exported prepare interface strictly; update include-set/watch-set assertions to authority-derived exact sets, not weakened counts. Add `preparation_pilot_` unit tests. |
| NET-NEW `crates/slicer-wasm-host/test-guests/preparation-pilot-infill/Cargo.toml`, `src/lib.rs` | Dual `rlib`/`cdylib`, empty workspace sentinel, explicit sdk/ir/schema/wit-bindgen path/version dependencies consistent with existing guest patterns; real macro module and module-owned numeric codec. |
| NET-NEW `crates/slicer-wasm-host/test-guests/preparation-pilot-path/Cargo.toml`, `src/lib.rs` | Same dual-target pattern; PathOptimization textual codec and actual comments. |
| `crates/slicer-wasm-host/Cargo.toml` | Native fixture dev-dependencies; ordinary production deps/features unchanged. |
| NET-NEW `crates/slicer-wasm-host/tests/contract/preparation_pilot_host.rs` | Actual combined typed bindings, imported Host implementations, staging/ready storage/accounting and fresh-store driver. Test-only implementation avoids prematurely normalizing row 03 production storage. |
| NET-NEW `crates/slicer-wasm-host/tests/contract/preparation_capability_pilot_tdd.rs` | Actual adapter witnesses named in AC commands; subprocess schema-derived compile driver and report evidence. |
| `crates/slicer-wasm-host/tests/contract/main.rs` | Register both NET-NEW modules explicitly. |
| `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `docs/adr/0066-private-layer-preparation-capability.md` | Exact doc-impact sections, actual results and unmeasured gaps. |

## Data and Contract Notes

### WIT composition and identity

New `preparation-types` defines error variants `not-ready`, `invalid-name`, `duplicate-name`, `poisoned-output`, `range-error`, `arithmetic-overflow`, `access-denied` and imported resource methods specified in the forward summary. `prepare` reuses the existing `slicer:common/module-errors.module-error` identity. `plan-access` imports/uses `prepared-plan-view` from the same defining interface; no duplicate exported resource declarations.

For each `STAGES` module-backed Layer row create `prepared-<wit_world>` with a qualified include such as `include slicer:layer-infill/infill-module@1.0.0;` plus capability imports/export (place the version after the world name, not between package and world). Macro generation selects that world only on explicit opt-in, with the full original IR/config/common imports. Host pilot bindings map those shared interfaces to `host::layer_perimeters` in `crates/slicer-wasm-host/src/host.rs`; capability resources map to one canonical pilot definer's backing types and all composed bindings alias it. Register ordinary host traits using the ordinary context projection and new capability Host traits against the pilot context. Do not double-register conflicting interfaces; actual typed instantiation is the decisive gate.

The existing `emit_world_preamble` (`crates/slicer-macros/src/lib.rs`) assembles inline WIT by nesting common/config/IR/type packages around the selected ordinary file. Its prepared branch must also nest every canonical Layer package referenced by the new capability file's composed worlds so all includes resolve during parsing, while selecting only the requested composed world for export. Plain branch assembly stays unchanged. The artifact checker must observe just one reachable scheduled-stage package plus capability, not treat the parsed helper-world roster as extra exported stages.

The NET-NEW test `PilotHostContext` wraps `HostExecutionContext` plus a capability-specific resource table, owner-bound fixture permission, staging and read storage. Existing ordinary host methods receive `&mut ordinary` through linker projections. Imported capability handles are inserted/deleted in their call's capability table with concrete mapped types. No field addition to `HostExecutionContext` or its builder is needed. This is a real resource host, not an import-free WIT encode probe.

`resolve_stage_from_world` must exclude only the recognized capability from stage candidates, not exclude arbitrary unknown packages. `compare_worlds` allows it only against canonical capability identity, compares its resources, and pins its exported preparation surface in both directions like the ordinary selected stage. A plain artifact must not be required to have the capability. Preserve the existing negative tests for unknown/multiple stage packages. The new package's nonzero initial 1.0.0 version is agreed new transport, not a guessed future existing IR version.

### SDK and native facade

WASM wrapper constructors accept call-local generated import adapters for bounded input/put operations; `PreparedPlanView::current()` is connected to the macro world's `plan-access.current` import. Native `with_prepared_plan` uses an unwind-safe thread-local scoped binding of an immutable serialized facade, restored at return and not leaking to another thread/print/owner. It must not cache a current resource table index or invoke a host-only algorithm. A facade is created per ordinary call; no eagerly copied whole plan. A module static required preparation method avoids substituting one constructor config for all target settings. Ordinary native/guest `from_config` remains unchanged per call.

### Controlled fixtures and independent oracle

- Infill preparation reads controlled `[3,5,11,17]`, puts `numbers` complete and a second empty `π` piece, with module-owned byte-to-number codec. Ordinary configuration uses NET-NEW fixture-only snake_case keys `pilot_range_offset`, `pilot_range_max_bytes`, `pilot_require_numbers`; none are production config declarations. Fresh calls read distinct bounded ranges and emit two-point `ExtrusionPath3D` via existing `InfillOutputBuilder::push_sparse_path` (`crates/slicer-sdk/src/builders.rs`). Expected X values are explicit fixture constants, not decoder-produced expectations. Y=0, Z=0.2 mm, width=0.4 mm, flow_factor=1 and speed_factor=1 are fixed and asserted; the role is existing `ExtrusionRole::SparseInfill` (`crates/slicer-ir/src/slice_ir.rs`). `Point3WithWidth`/`ExtrusionPath3D` retain existing complete fields from `crates/slicer-ir/src/slice_ir.rs`, test literals use FRU/waivers, production fixture literals exhaustive.
- PathOptimization preparation reads `alpha|beta`, publishes a module-owned text piece, and fresh calls emit `alpha`/`beta` via existing `GcodeOutputBuilder::push_comment`/`commands` (`crates/slicer-sdk/src/postpass_builders.rs`). Its input uses perimeter-region/collection builder family, not an Infill signature disguised as another stage. Assert actual typed G-code comment records, not a diagnostic log string.
- Compile driver derives every stage from `STAGES`, emits scratch crates from a schema-based template containing actual stage signatures, invokes macro compilation for prepared/plain/native/wasm forms and componentizes guest witnesses using the same build toolchain. It reports coverage by qualified export and rejects an empty set. This scratch driver is not production guest registration and cannot claim it executes every stage; two representative real adapters provide behavior evidence.
- Preparation guest store is genuinely dropped before read stores exist. Native prep call/value and scoped input are dropped before new consumer values. Owner/print tests retain bytes only. Instrument actual boundaries and distinguish fixture payload lengths (test data) from measured runtime results.
- Before any SDK/macro/WIT implementation edit, verify ordinary guest freshness, archive the existing `modules/core-modules/classic-perimeters/classic-perimeters.wasm` to `target/preparation-pilot/plain-before.component.wasm`, and save a SHA-256 provenance record alongside it. The plain compatibility test requires this exact old artifact and pre-edit hash; fail loudly if missing, never replace it with a newly compiled plain fixture. Final freshness checks validate current staged artifacts separately from this intentional compatibility snapshot.
- Negative preparation modes belong to the module fixture codec: numeric input `[]` means explicit successful empty publication; first-byte sentinels 255/254/253/252 request duplicate-put/guest-trap/returned-module-failure/caught-duplicate-then-success. Successful `[3,5,11,17]` contains no sentinel. The returned failure uses existing ModuleError fields `code`, `message`, `fatal` (`crates/slicer-sdk/src/error.rs`), including a `fatal=false` control that must still stop this required dependency while preserving the original details. Malformed payload, missing required/optional and bounds modes are controlled consumer config/fixture inputs, not fabricated successful output. Concurrent native/guest read tests use fresh wrappers on the same immutable bytes and assert both independently expected ranges without shared cursor interference.

### Publication and accounting

Staged `BTreeMap<String, Arc<[u8]>>` plus poison state becomes owner-bound ready immutable storage only on successful unpoisoned return. Ready-empty is represented separately from absent storage. Duplicate/name errors poison even if caught; trap/module failure discards all staging and original details remain. Module decoder decides optional/required pieces; host cannot infer coverage from opaque keys. Names use exact nonempty UTF-8 matching, no path/layer interpretation.

Checked conversions apply before allocation; offset equal to length returns empty, greater fails, reads clip by checked remaining length without summing unchecked offset+max. Accounting separates name UTF-8 bytes, payload bytes and transfer records, including input/put/read boundaries. A synthetic arithmetic seam tests overflow without giant allocation. Metadata serialization is a separate whitelist (owner, phase, ready, piece names/lengths, accounting and measured durations), never serialized storage or raw bytes. Store release is proved with weak references after consumer unwinding, not process memory claims.

## Read-Only Context

- `crates/slicer-schema/src/lib.rs` StageSpec/STAGES windows; ordinary WIT deps complete small files.
- `crates/slicer-macros/src/lib.rs` `slicer_module`/`generate_slicer_module_impl` (28–187), `build_layer_infill_glue` (3598–3641), symbol-located `emit_world_preamble` and native adapter windows.
- `crates/slicer-wasm-host/src/host.rs` canonical definer/alias window (363–458), symbol-located context/resource push/output getter windows; no full-file read.
- `xtask/src/build_guests.rs` `discover_guests` test-guest block (217–299), symbol-located verified build/freshness routines; no edit needed for discovery.
- `xtask/src/wit_verify.rs` stage/classification/comparison window (191–349), include/watch assertion windows (1280–1459), negative resolver tests by symbol.
- `crates/slicer-sdk/src/traits.rs` LayerModule (377–405), PathOptimization (558–578); builders and IR structs by symbol windows only.
- Governing documents listed in requirements, bounded section reads; no production routing prerequisite source inspection required.

## Out-of-Bounds Files

Other packets, source plan/queue, production runtime/scheduler dispatch/manifest/blackboard and lightning paths, visual-debug modules, ordinary WIT deps, existing request/response structs and IR versions. No generated code/target artifacts/lockfiles/vendor or `OrcaSlicerDocumented/` body reads. Artifact paths may be loaded/hashed by the test tool, never browsed as source. Implementation completion backlog change is worker-only and TASK-573-only after acceptance; no authoring backlog edit.

## Expected Sub-Agent Dispatches

Every dispatch requires its answer outside thinking, escaped and redundant draft/implementation state where relevant.
- Question: can the composed resources typecheck/instantiate without changing ordinary identities? Scope: new WIT, macro and contract pilot host only; return FACT at most five lines, failing symbol/signature only.
- Question: does each actual pilot/negative test execute and falsify its stated input? Scope: named contract tests and existing log; return FACT pass/fail and nonzero executed count, failure SNIPPETS at most 20 lines.
- Question: does capability-aware artifact checking retain plain compatibility and reject mutated exports? Scope: xtask named resolver/comparator unit tests and freshness exit; return FACT at most five lines.
- Question: do doc-impact sections describe actual pilot evidence without production claims? Scope: three exact sections; return FACT at most five lines.
- Independent preflight: S0–S8 plus AC/doc checks; author does not self-award PREFLIGHT PASS.

## Risks and Tradeoffs

Resource identity/linker composition is precisely the feasibility question. Scope does not authorize fallback to retained guest state or altered ordinary signatures. Native call-local TLS is transport binding, not persisted module state; restoration/concurrent owner isolation needs tests. Compile-matrix subprocesses must fail loudly when toolchain/componentization is unavailable, never silently skip. Pilot-local storage must not be mistaken for production framework completion.

## Context Cost Estimate

Aggregate M with bounded independent steps; largest M (macro composition or actual resource driver). Dispatch heavy compilation/test output as FACT, no full expansions/logs. No L step.

## Open Questions

- [FWD] Row 03 promotes the pilot host backings and normalizes print storage/declarations while preserving these tested names/shapes. Pilot-only `read-fixture` is not its production input API; row 04 replaces controlled input projection.
- [FWD] Actual measured timing/transfer results and compiled binding mechanical details are populated only after execution; later packets cannot claim them already obtained.
- [BLOCK] Independent preflight must pass before activation. True agreed-contract feasibility failure blocks dependent implementation and requires a design decision, not a workaround.
