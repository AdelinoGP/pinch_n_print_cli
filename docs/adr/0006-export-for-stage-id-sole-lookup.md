# ADR-0006: `slicer_schema::export_for_stage_id` is the sole stage→export lookup

## Status

Accepted (packet 83, 2026-06-02)

## Context

Before packet 83 the project had **two parallel lookups** for the WIT export
name belonging to a stage id:

1. **`slicer_schema::STAGES[*].wit_export`** — the canonical table in
   `slicer-schema`, with each stage's `wit_export` field documented as the
   single source of truth for the WIT host-binding ↔ guest-export name
   mapping.

2. **`slicer_runtime::dispatch::export_name_for_stage(stage_id: &StageId) ->
   &'static str`** — a hardcoded match-arm in `dispatch.rs` (lines 47–67 in
   the pre-P83 file) that duplicated the table. The function was called from
   the dispatcher to route a stage to its wasmtime call_* entry point.

The two lookups carried the same data but were maintained independently. Any
new stage required edits in both places; any rename had two failure modes
(forget to update the dispatcher table, or forget to update the schema). The
dispatcher table also called itself authoritative in passing comments while
the schema docstring claimed authority — a contradiction nobody had resolved.

## Decision

`dispatch::export_name_for_stage` is **deleted**. `slicer_schema` gains a
public lookup function:

```rust
/// Look up the WIT export name for a stage id from the single source of truth in [`STAGES`].
///
/// Returns `None` for unknown stage ids. Dispatcher impls MUST use this lookup; they MUST NOT
/// hardcode their own stage-id → wit-export table.
pub fn export_for_stage_id(stage_id: &str) -> Option<&'static str> {
    STAGES.iter().find(|s| s.stage_id == stage_id).map(|s| s.wit_export)
}
```

All callers — `slicer-wasm-host::dispatch`, `slicer-runtime::dag_cli`,
`slicer-runtime/benches/wasm_modules.rs`,
`slicer-runtime/tests/contract/dispatch_tdd.rs` — switch to
`slicer_schema::export_for_stage_id`. A TDD test in `slicer-schema` iterates
`STAGES` to confirm the lookup is total over the canonical table and that
unknown ids return `None`.

## Consequences

- Single source of truth restored. Adding a stage requires editing exactly
  one place (`STAGES` in `slicer-schema/src/lib.rs`).
- Renames cannot silently drift between dispatcher and schema.
- The `Option<&'static str>` return type (vs the old `&'static str`) forces
  callers to handle unknown stage ids explicitly. Pre-P83 callers were
  hand-coded to never call with an unknown id; the new API documents that
  assumption at the type level.
- Dispatchers depend on `slicer-schema` for the lookup. This is a new dep
  edge (`slicer-wasm-host → slicer-schema`) but is small and aligned with
  schema's role as the canonical contract crate.

## Alternatives considered

- **Keep `slicer-schema` as pure `&'static` data; put the lookup in
  `slicer-wasm-host`.** Rejected: the lookup is a planning-time concern (the
  DAG validator and `dag_cli` call it without instantiating any WASM).
  Putting it in wasm-host pulls a wasmtime dep into the planning crate via
  the lookup, which contradicts P85's extraction plan for `slicer-scheduler`.
  Schema is the natural home.
- **Move the lookup into the `#[slicer_module]` macro expansion.** Rejected:
  the dispatcher is the consumer, not the module author. The macro generates
  the guest side; the dispatcher lookup happens on the host side. Different
  layer.

## Verification

- `! grep -rE 'pub fn export_name_for_stage' crates/` returns no matches.
- `grep -qE 'pub fn export_for_stage_id' crates/slicer-schema/src/lib.rs`
  succeeds.
- `cargo test -p slicer-schema --test export_for_stage_id_tdd` passes (the
  TDD test iterates `STAGES` and asserts totality + unknown-rejection).
- All callers compile against the new `Option<&'static str>` return type;
  dispatcher impls handle `None` via the same error path that previously
  guarded against unknown stage ids.
