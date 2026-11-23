---
status: implemented
packet: layer-module-preparation_02_preparation-capability-pilot
task_ids:
  - TASK-573
---

# layer-module-preparation_02_preparation-capability-pilot

## Goal

Build an executable same-artifact, native/WASM preparation pilot using real imported host resources, immutable named pieces, fresh ordinary Layer calls, and measured transport accounting, without activating preparation in the slicing pipeline.

## Problem Statement

An encode/decode probe cannot prove same-artifact typed resource identity, macro/native authoring, or fresh-store consumption. This packet must produce executable evidence before row 03 finalizes production bindings. Controlled fixtures keep the proof independent of routing repair and lightning fidelity.

## Architecture Constraints

- ADR-0066 explicitly extends ADR-0045 with a declared independently versioned capability, not an additional scheduled stage. ADR-0056 requires same-source native/WASM calls. Pilot fixture registration is not production ingestion/override/eligibility proof.
- Plain worlds and shared imported WIT remain byte-for-byte source unchanged; no existing schema/version bump, no `SlicerModuleSchema` or `StageSpec` field addition. New sidecars avoid full existing struct-literal and version-assertion fallout. New struct literals follow docs/21; production exhaustive/test FRU or reasoned waiver.
- Imported capability resources use host backing identities and fresh ResourceTables; only immutable owner-bound bytes and readiness survive, never guest instance/resource index. A native call-local serialized facade has the same bounds/errors as guest imports and is restored on failure/unwind.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

### WIT composition and identity

New `preparation-types` defines error variants `not-ready`, `invalid-name`, `duplicate-name`, `poisoned-output`, `range-error`, `arithmetic-overflow`, `access-denied` and imported resource methods specified in the forward summary. `prepare` reuses the existing `slicer:common/module-errors.module-error` identity. `plan-access` imports/uses `prepared-plan-view` from the same defining interface; no duplicate exported resource declarations.

For each `STAGES` module-backed Layer row create `prepared-<wit_world>` with a qualified include such as `include slicer:layer-infill/infill-module@2.0.0;` plus capability imports/export (place the version after the world name, not between package and world, and take the version from the live canonical WIT; no composed-world `include` exists in-tree yet, so Step 1's parser check is its first validation). Macro generation selects that world only on explicit opt-in, with the full original IR/config/common imports. Host pilot bindings map those shared interfaces to `host::layer_perimeters` in `crates/slicer-wasm-host/src/host.rs`; capability resources map to one canonical pilot definer's backing types and all composed bindings alias it. Register ordinary host traits using the ordinary context projection and new capability Host traits against the pilot context. Do not double-register conflicting interfaces; actual typed instantiation is the decisive gate.

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

## Risks and Tradeoffs

Resource identity/linker composition is precisely the feasibility question. Scope does not authorize fallback to retained guest state or altered ordinary signatures. Native call-local TLS is transport binding, not persisted module state; restoration/concurrent owner isolation needs tests. Compile-matrix subprocesses must fail loudly when toolchain/componentization is unavailable, never silently skip. Pilot-local storage must not be mistaken for production framework completion.
