# Design: 02-rev1_runtime-access-audit-and-declaration-enforcement

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-host/src/wit_host.rs` — Add `runtime_reads` tracking to `HostExecutionContext`; instrument WIT view resource methods
  - `crates/slicer-host/src/prepass.rs` — Wire read audits from `execute_prepass`
  - `crates/slicer-host/src/layer_executor.rs` — Wire read audits from per-layer execution
  - `crates/slicer-host/src/postpass.rs` — Wire read audits from postpass execution
  - `crates/slicer-host/src/validation.rs` — Fix `WriteConflict.orderable` semantics
  - `crates/slicer-host/tests/dag_validation_tdd.rs` — Add positive orderable test
- Neighboring tests:
  - `crates/slicer-host/tests/pipeline_tdd.rs` — `access_audits_live_path`
  - `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` — manifest-level contract
  - `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` — (green, not reopened)

## Architecture Constraints

- **`HostExecutionContext`** must carry read audit state per dispatch call so it can be returned alongside write audits.
- **WIT view resource methods** must record the exact IR path accessed when called. This requires modifying the generated bindings or wrapping them.
- **Read audits must use exact paths** (e.g., `SliceIR.regions.polygons`) for enforcement; top-level roots may be used for coarse reporting.
- **Undeclared access enforcement** must be fatal (no graceful degradation for contract violations).
- **`WriteConflict.orderable`** must check DAG reachability (`can_reach`) not just `ir_reads` containment.

## Proposed Changes

### Step 1: Add Read Audit Field to HostExecutionContext

In `crates/slicer-host/src/wit_host.rs`, add a `runtime_reads: Vec<String>` field to `HostExecutionContext`. Populate it when WIT view resource methods are called.

The WIT view resource methods that read IR data (by IR root type):

| View Resource | Method(s) | IR Path(s) Recorded |
|---|---|---|
| `slice-region-view` | `polygons()`, `infill-areas()`, `boundary-paint()` | `SliceIR`, `SliceIR.regions`, `SliceIR.regions.polygons`, etc. |
| `perimeter-region-view` | `wall-loops()`, `infill-areas()` | `PerimeterIR`, `PerimeterIR.wall-loops`, etc. |
| `infill-region-view` | (via output builder push, tracked as write) | — |
| Prepass mesh view | `object-bounds()`, etc. | `MeshIR` |
| Prepass layer planning | reads from layer plan proposals | `LayerPlanIR` |

The precise paths recorded should match the field names from `docs/02_ir_schemas.md`.

### Step 2: Extract Read Audits from HostExecutionContext

After each module call returns, the dispatcher must extract `ctx.runtime_reads` and merge them into the outer audit vector alongside `runtime_writes`.

For prepass: modify `execute_prepass` to pass a mutable audit vector into the call chain so reads are recorded per module.

For per-layer: modify `execute_single_layer` similarly.

For postpass: reads are typically not a concern (postpass modules emit GCode), but any IR reads in postpass stages should be tracked.

### Step 3: Fix WriteConflict.orderable Semantics

In `crates/slicer-host/src/validation.rs`, `validate_write_conflicts`:

Current (incorrect):
```rust
let orderable = right.ir_reads.contains(&field)
    || left.ir_reads.contains(&field);
```

Correct: `orderable` should be `true` only when there is a reachability path from one module to the other via the conflicting field. Since `validate_write_conflicts` already computes `reachability` (a `BTreeMap<ModuleId, BTreeMap<ModuleId, bool>>`), use it:

```rust
let left_reads_right = right.ir_reads.contains(&field)
    && reachability[&left.module_id][&right.module_id];
let right_reads_left = left.ir_reads.contains(&field)
    && reachability[&right.module_id][&left.module_id];
let orderable = left_reads_right || right_reads_left;
```

This means: the conflict is orderable if module A writes field F and module B reads F AND there is already a reachability edge A→B (meaning B can be ordered after A to resolve the conflict). If neither module reads F, or if neither creates a reachability edge, the conflict is non-orderable.

### Step 4: Add Positive orderable Test Case

In `crates/slicer-host/tests/dag_validation_tdd.rs`, add a test:
- Module A writes `PerimeterIR`; Module B reads `PerimeterIR` AND writes `PerimeterIR` (same field conflict)
- The reachability from A→B exists (B requires A's output)
- Assert `orderable == true`

### Step 5: Add Per-Criterion Verification Commands

Update all 8 acceptance criteria in `packet.spec.md` to end with `|` followed by the specific verification command.

## Data and Contract Notes

- `ModuleAccessAudit.runtime_reads: Vec<String>` — exact IR paths read during a module call.
- `ModuleAccessAudit.runtime_writes: Vec<String>` — exact IR paths written during a module call.
- `WriteConflict.orderable: bool` — true only when a DAG edge (reachability path) exists via the conflicting field.
- WIT view resource methods are called by guest modules — they are the read boundary.
- The host does not currently track reads per-call; this is the primary gap.

## Risks and Tradeoffs

- **Performance**: Recording every WIT view method call adds overhead. Mitigation: only record the top-level IR root (e.g., `SliceIR`) for coarse reporting; exact paths only for modules under enforcement.
- **Generated WIT bindings**: The `wasmtime::component::bindgen!` macro generates types but host implementations must be provided via callbacks. We must ensure the read-audit callbacks survive the generated-code boundaries.
- **Path canonicalization**: Exact paths must match manifest declaration format exactly (e.g., `SliceIR` vs `SliceIR.regions.polygons`). Use path prefix matching.

## Open Questions

- **Q1**: Should read auditing be all-or-nothing, or only for modules that will be subject to enforcement? (All modules, per spec.)
- **Q2**: For postpass stages, do any modules read IR data, or only write GCodeIR? (If only writes, postpass auditing may be write-only.)
- **Q3**: Is there existing test infrastructure for the WIT view resource methods that we can extend with read-audit assertions? (Check `dispatch_tdd.rs`.)
- **Q4**: Does the dispatch_tdd linker error affect our ability to run tests for this packet? (Need to verify with `cargo test --package slicer-host --test dag_validation_tdd`.)
