# Integrated-parity oracle experiment

Type: task
Status: open

## Question

Can the integrated (natively compiled) path serve as a perf oracle for module
work — or do the confirmed native/WASM asymmetries keep it disqualified?

The asymmetries (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §11.5): native builds views for every
slice region where WASM filters first via `module_receives_slice_region`
(`crates/slicer-wasm-host/src/dispatch.rs`); native does **not** apply
per-region `RegionMapIR` config overrides (`build_native_layer_request` in
`crates/slicer-wasm-host/src/marshal/native.rs` assigns `module.config_view`
only, where WASM resolves `config_fields_per_region` through
`HostExecutionContext`) — a confirmed semantic parity gap, not just perf. The
surface-derivation duplication suspect is eliminated by construction (shared
via the [Prepared-region arena cache](05-prepared-region-arena-cache.md)).

Discriminating experiment: extend
`native_and_wasm_layer_views_are_field_identical`
(`crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs`) with an
excluded region + a `RegionMapIR` override + non-empty classification + claims,
and assert count/ID/config/claim equality.

Answer the oracle question directly: usable as-is, usable after a named fix
(state the fix's size), or permanently disqualified. The oracle matters to this
map as a cheaper measurement mode (integrated runs skip WASM instantiation
cost), not as a correctness program of its own.
