# Integrated-parity oracle experiment

Type: task
Status: resolved
Assignee: wayfinder session (ses_f2453f655ffepUKHG9FHhC64kx), 2026-09-26

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

## Answer

**Not usable as-is; repairable, but not yet qualified as a perf oracle.**
`WasmRuntimeDispatcher::run_stage` (`crates/slicer-wasm-host/src/dispatch.rs`)
previously sent every native slice region to the SDK and
`build_native_layer_request_with_raft` (`crates/slicer-wasm-host/src/marshal/native.rs`)
assigned the module-level config to every region. WASM's
`push_slice_regions` filters support-family-ineligible regions and modifier
footprints, while `HostExecutionContext::config_fields_for`
(`crates/slicer-wasm-host/src/host.rs`) reads the per-region effective config
derived from `RegionMapIR`. These are semantic differences, not a timing
interpretation.

The named fix is localized to native layer dispatch and projection:
`run_stage` now passes the actual `GlobalLayer` to
`build_native_layer_request_for_layer`; that builder shares
`module_receives_slice_region`'s eligibility predicate, leaves synthetic
support carriers ungated, and uses a `SliceIR`-eligible, `RegionKey`-matched,
declared-key-filtered config table for slice and perimeter views. Perimeter-only
identities and regions without a map entry retain the object-level fallback,
as on the WASM leg. No WIT, guest, or scheduler contract changed.

`native_projection_filters_and_resolves_configured_regions`
(`crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs`) covers an
excluded region, nonempty classification, a held claim, a per-region config
override, and a perimeter-only fallback. It asserts the selected identity,
claim, classification population, and config value. It does **not** call the
WASM `push_slice_regions`/`HostExecutionContext` config accessor; it is a
native projection regression, not an end-to-end parity proof.
`integrated_parity_rectilinear_infill`
(`crates/slicer-runtime/tests/contract/integrated_parity_rectilinear_infill_tdd.rs`)
passes after its fixture's effective `RegionMapIR` config was corrected to
include the same `line_width` as the module-level config: with the fixed native
config precedence, the former fixture's zero default width suppressed its
native commit. Both narrow tests passed; `cargo xtask build-guests --check`
returned clean before the guest-dependent test, and slicer-wasm-host all-target
clippy, `check-literals`, and touched-file test-quality checks passed.

This still does **not** authorize integrated timings as an external-module
oracle. The historical all-integrated versus external full-slice G-code TYPE
divergence is documented in
`docs/specs/perf-vs-orca/evidence/perf-split/FINDINGS.md` §Finding 3; a
single-region projection test and one narrow infill contract cannot rule out
other dispatch or output differences. The remaining qualification is the new
[Integrated/external matched-output oracle gate](33-integrated-external-matched-output-oracle-gate.md):
compare real paired outputs and investigate any differences before using
integrated run times to rank module work. No speedup is claimed here and this
candidate was not auto-committed.
