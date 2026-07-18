# Design: 113_marshal-boundary-extraction

## Controlling Code Paths / Likely Surfaces

All in `crates/slicer-wasm-host/src/` (line ranges approximate; locate with `rg` before opening a ±40 window):

- `host.rs:542–654` — the `*Collected` accumulator structs (move to `marshal/accumulators.rs`).
- `host.rs:1667–2400` — IR→WIT leaf maps + marshal-in projections (`ir_to_wit_*`, `sliced_region_to_data`, `project_*_view`, `object_mesh_to_wit_mesh_object_view`).
- `host.rs:1859–1888` — `ir_to_wit_expolygon(s)_prepass` (dead dup; delete).
- `host.rs:3696–3719` — `finalization_role_ir_to_wit`, `finalization_path_ir_to_wit` (outbound dead dups; delete). `host.rs:3679` — `finalization_role_wit_to_ir` (**inbound; diverges** — keeps `Custom`, no `PrimeTower`/`Skirt` recovery; relocate unchanged, packet 115 fixes; do NOT delete).
- `host.rs:4458–4472` — `convert_postpass_role` (**inbound; diverges** — keeps `Custom`; relocate unchanged, packet 115 fixes; do NOT delete). The outbound `convert_postpass_role_to_wit` lives in `dispatch.rs:92–117` and IS a dead dup (delete).
- `host.rs:4505–5177` — WIT→IR converters incl. the three bucketing converters `convert_infill_output` (4578), `convert_support_output` (4728), `convert_perimeter_output` (4920) and `merge_slice_postprocess_into` (5115).
- `dispatch.rs:92–272` — postpass WIT converters (`convert_postpass_role_to_wit`, `collect_postpass_output`); `convert_postpass_role_to_wit` is also a dead dup of `ir_to_wit_extrusion_role`.
- `dispatch.rs:1331–1807` — marshal-in helpers (`push_slice_regions`, `push_perimeter_regions`, harvest `*_from`) and `deconstruct_layer_ctx:2216` — the per-stage router that calls the bucketing converters (stays in `dispatch.rs`; repointed to `marshal::convert_*`).

## Neighboring Tests / Fixtures

- `crates/slicer-wasm-host/tests/contract/` — the standing behaviour guard (AC-6).
- `crates/slicer-wasm-host/tests/common/` — mesh/geometry helpers (do not widen; per ADR-0007 amendment they are duplicated by design).
- New unit tests live inline in `marshal/origin.rs` under `#[cfg(test)] mod tests`.

## Architecture Constraints

- `marshal` is an **in-process module** (`src/marshal/`), never a crate: the WIT types exist only after `bindgen!` expands inside `slicer-wasm-host` (host-side analogue of ADR-0003). `marshal` must not reference `wasmtime` (AC-2).
- The four `bindgen!` invocations stay in `host.rs`; their count must remain 4 (ADR-0005). This packet does not touch them.
- Per-stage harvest routing stays in `dispatch.rs`, co-located with the stage→export match (ADR-0006). Do **not** move the `match stage_id` into `marshal`.
- Builder methods (`HostInfillOutputBuilder::push_*`, etc.) stay on `HostExecutionContext`; only the `*Collected` data structs move, so no `host ↔ marshal` cycle forms.
- (No `wasm-staleness` constraint: the change surface is host-only Rust and feeds no guest build path. No `coord-system` constraint: converters relocate geometry types but introduce no mm↔unit conversion.)

## Selected Approach

Flat functions, one per concept, both directions co-located, over a shared `OriginBucket<R>` that owns the all-or-none origin-attribution rule. **Rejected** (per ADR-0021): a `ToWit<World>`/`FromWit`/`Harvest`/`Project` trait family — its justification (`ExtrusionRole` across two worlds) is deleted by Step 1's de-duplication, and it cuts against ADR-0003/0005. **Rejected**: a single deep `harvest_layer_commit` swallowing the stage match (splits the stage taxonomy across two files with no compile-time guard).

## Explicit Code Change Surface

Primary (≤3 per step; see `implementation-plan.md`):
- **New**: `crates/slicer-wasm-host/src/marshal/{mod,origin,accumulators,out,leaf,in_}.rs`.
- `crates/slicer-wasm-host/src/host.rs` — delete dead dups; move converters/accumulators out; repoint Host-impl call sites to `marshal::*`.
- `crates/slicer-wasm-host/src/dispatch.rs` — repoint `deconstruct_layer_ctx` + harvest/marshal-in helpers to `marshal::*`; keep wasmtime mechanics and the stage router.
- `crates/slicer-wasm-host/src/lib.rs` — add `mod marshal;` and adjust re-exports.

## Read-Only Context the Implementer Needs

- ADR-0021 (~140 lines) — read in full; canonical `OriginBucket`/`MarshalError`/`OriginId` signatures and the `convert_infill_output` rewrite are reproduced in §Data and Contract Notes.
- `docs/02_ir_schemas.md` — **delegate** a FACT for exact `InfillIR`/`InfillRegion`/`PerimeterIR`/`SupportIR` field names; do not read in full (> 600 lines).

## Out-of-Bounds Files

- `crates/slicer-schema/wit/**` — no WIT change in this packet.
- `modules/core-modules/**`, `crates/slicer-runtime/test-guests/**`, any `target/`, any `Cargo.lock`.
- `OrcaSlicerDocumented/**` — no parity concern.
- Other crates (`slicer-runtime`, `slicer-scheduler`) — only their compile status matters, via delegated `cargo check`.

## Expected Sub-Agent Dispatches

- Step 1: "Is each of these six fns byte-identical (modulo type namespace) to its layer-world counterpart? `finalization_role_ir_to_wit`↔`ir_to_wit_extrusion_role`, `finalization_role_wit_to_ir`↔`convert_extrusion_role`, `finalization_path_ir_to_wit`↔`ir_to_wit_extrusion_path`, `convert_postpass_role`↔`convert_extrusion_role`, `convert_postpass_role_to_wit`↔`ir_to_wit_extrusion_role`, `ir_to_wit_expolygon_prepass`↔`ir_to_wit_expolygon`." Scope: `crates/slicer-wasm-host/src/{host,dispatch}.rs`. Return: `FACT` (identical / differs-how per pair).
- Steps 3–7: "Run `cargo check --workspace --all-targets`; return FACT pass/fail + first error file:line." (after each move).
- Step 3/8: "Run `cargo test -p slicer-wasm-host --lib marshal::origin`; return FACT `^test result` line + any failing assertion."
- Field-name dispatch: "From `docs/02_ir_schemas.md`, list the exact field names of `InfillRegion`, `PerimeterRegion` (IR), `SupportRegion`. Return FACT ≤5 lines."

## Data and Contract Notes

Canonical signatures (from ADR-0021; implement verbatim):

```rust
// marshal/origin.rs
pub struct OriginId { pub object_id: String, pub region_id: u64 } // Clone+Eq+Hash
pub enum MarshalError {
    UntaggedPayload { kind: &'static str, index: usize },
    OriginLengthMismatch { kind: &'static str, origins: usize, payloads: usize },
    NonFiniteFloat { field: &'static str, index: usize },
}
impl From<MarshalError> for String { /* preserves today's Result<_, String> */ }

pub struct OriginBucket<R> { tagged: bool, regions: Vec<(OriginId, R)>, mint: fn(&OriginId) -> R }
impl<R> OriginBucket<R> {
    pub fn new(any_tagged: bool, mint: fn(&OriginId) -> R) -> Self;
    pub fn drain<T>(&mut self, kind: &'static str, payloads: Vec<T>,
                    origins: &[Option<OriginId>], place: impl FnMut(&mut R, T))
        -> Result<(), MarshalError>;
    pub fn into_regions(self) -> Vec<R>;
}
```

`new(false, …)` mints exactly one anonymous region (`OriginId{ "", 0 }`); `drain` then ignores origins. `new(true, …)` mints regions on first sight of each origin, in first-appearance order; an untagged element or length mismatch errors. `convert_infill_output` becomes ~25 LoC: build three leaf-mapped path vecs, OR the three origin slices into `any_tagged`, `OriginBucket::new`, three `drain` calls (`sparse_infill`/`solid_infill`/`ironing` push closures), `into_regions`. `convert_perimeter_output` keeps its rotated-vs-original wall selection *inside* the function (it chooses which `(payloads, origins)` pair to feed `drain`).

## Locked Assumptions and Invariants

- Converter output for valid input is unchanged (AC-6). First-seen bucket ordering must match the existing `Vec::position`-based loop exactly.
- `MarshalError` Display reproduces today's error strings closely enough that any test asserting on message substrings still passes; tests should prefer asserting on the variant.
- `OriginId` equality/hash semantics equal the old `(String, u64)` tuple semantics.
- The inbound finalization/postpass role converters are relocated with **identical (Custom-preserving) behaviour**; this packet does NOT recover `PrimeTower`/`Skirt` (packet 115 does). Folding that fix in here would violate the behaviour-preserving guarantee above.

## Risks and Tradeoffs

- **Ordering drift**: if `OriginBucket` changes bucket order, identity-keyed IR diffs change. Mitigated by AC-6 + the first-seen-order unit test.
- **Large mechanical diff** across two 2000–5000-line files; mitigated by incremental steps each gated on `cargo check` (cross-step invariant).
- **Hidden non-identical "dup"**: a named converter may differ subtly (custom-tag handling). Step 1's delegated diff is the guard; non-identical ones become an [FWD] question, not a blind delete.

## Context Cost Estimate

- Aggregate: **M** (sum of step costs below; no L step).
- Largest single step: Step 5 (move + rewrite the three bucketing converters and repoint `deconstruct_layer_ctx`) — M.
- Highest-risk dispatch: the Step 1 byte-identity confirmation (drives the deletion set).

## Open Questions

- `None (resolved).` The Step-1 diff was run during implementation. The **outbound** role converters (`finalization_role_ir_to_wit`, `convert_postpass_role_to_wit`), the path converter (`finalization_path_ir_to_wit`), and the expolygon converters are byte-identical to layer → deleted (AC-1). The **inbound** role converters (`finalization_role_wit_to_ir`, `convert_postpass_role`) are NOT identical — they keep `Custom(s) => Custom(s)` instead of recovering `PrimeTower`/`Skirt`, a latent bug (ADR-0021 §Amendment). They are excluded from AC-1, relocated into `marshal` unchanged (AC-1b), and fixed in packet 115. No blocker remains.
