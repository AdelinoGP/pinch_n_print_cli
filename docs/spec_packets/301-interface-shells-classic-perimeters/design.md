# Design: 301-interface-shells-classic-perimeters

## Controlling Code Paths

- Primary code path: `compute_region_updates` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`, lines 351–520) gains the neighbour-source gate — the Pass 1 `upper_polys` / `lower_polys` reads that today clone same-timeline polys via `clone_region_polys` switch to a precomputed per-slice collective union when the resolving config says `false`, while `true` keeps the pre-packet same-timeline shape; a `resolve_interface_shells` sibling of `resolve_shell_counts` (lines 1081–1106) reads the flag through the same first-timeline-entry `region_map.config_for` pattern and threads it into the per-timeline call.
- Config path: `resolve_interface_shells`' `config_for(&key)` read is the per-region config entry for the flag — the same seam packet 299's P73 stages use, so no new plumbing is invented. The macro emits `Default`, `apply_cli_key`, `to_config_map`, `host_config_keys`, and the ticket-126 overlay arm for the new row.
- Neighbouring tests/fixtures: `#[cfg(test)] mod tests` in `slice_postprocess_prepass.rs` (the `rect`/`slice_with` fixture helpers, lines 1137–1164); the `bottom_shadow_does_not_propagate_past_k_bot_via_a_later_seed` test pins the `compute_region_updates(&snapshot, &object_id, 0, &[...], k_top, k_bot, opening_r)` call shape the gate extends. The AC tests need the same direct-call shape (no per-object slice fixtures exist — every call site passes one `(object_id, region_id)`), so Step 2 builds the two-object cover through two direct calls over one shared snapshot, not through `commit_shell_classification_builtin`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The gate is a *source* switch, not a second classifier: no new stage, no new call site in `commit_shell_classification_builtin`'s loop, no ordering change. The collective union is precomputed once per slice index (one `union` over all timelines' same-slice `r_polys`), then each timeline's Pass 1 reads its slice's union only when its own flag resolves `false`.
- Per-region resolution, not global: each `(object_id, region_id)` timeline resolves the flag from its own first-timeline-entry config. Two bodies on one layer may disagree (paint/patch configs do); the union is shared geometry, the gate is per-timeline.
- `ConfigView::from_declared` whitelist is not engaged: the flag is a host-prepass input read through `region_map.config_for` (the `resolve_shell_counts` pattern), so no module manifest declares it; no manifest changes anywhere.
- Bounds: no numeric range rejection anywhere (ticket-113 rule — a bool has no range). The wrong-variant spelling is the typed `extract_bool` `TypeMismatch` (accepts `Bool` + `Int` 0/1, rejects everything else) — never a saturate. This packet has no rejection criterion of its own — AC-N1 is a no-leak invariant, not a validation gate.
- CONFIG_BLOCK: the `to_config_map` arm shadows the frozen `("interface_shells", "0")` padding twin with the live value (284–286 precedent); the table itself is never edited (rule 2 — load-bearing for the ≥80 floor, and the true-value spelling rides 132).
- Rule 4 does not fire: the gate branches inside one prepass seam's existing reads — no cross-module algorithm selection, no claim holders (`seam_position` precedent, map Notes Q8).
- ADR-0062/0063 locked-path conformance: the gate reads locked footprints as cover geometry (they are physical lower layers) but neither clips nor merges them — the self-clipping obligation stays with the lock emitter (AC-N1's disjoint twin plus the partition's pairwise-disjoint invariant, `SlicedRegion.sparse_infill_area` doc, `crates/slicer-ir/src/slice_ir.rs`).
- Draft-299 merge: packet 299 changes the same file's stage list and the same `resolve_shell_counts` call neighbourhood. This packet's resolver addition merges with — never duplicates — 299's signature at activation time; whichever lands second rebases onto the first. No shared helper, no cross-packet dep.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Doc-15 honest mechanics: the generated host-speeds table sources from `docs/config/host-keys.toml`, which the `host_keys_doc_lock_tdd` test ties to scalar `ResolvedConfig` defaults — bool rows need the lock's `resolved_bool` arm extended alongside the TOML row (Step 1; packet 299's regen-only precedent is corrected here, or AC-5's `--check` passes while the key stays absent from the table).

## Code Change Surface

- Selected approach: one `declare_resolved_config!` bool row (canonical default `false`) + `to_config_map` arm → `resolve_interface_shells` sibling resolver → per-slice collective unions + Pass 1 source switch in `compute_region_updates` → four unit tests (collective-cover, extra-bottom, defaults-identity, disjoint-no-leak) → lock-test bool arm + host-keys row → DEV-192 + docs regen. No IR field, no WIT line, no emitter arm — the gate changes the existing `top_solid_fill` / `bottom_solid_fill` buckets both perimeter modules already consume.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — 1 macro row (`interface_shells` bool `false` via `extract_bool`) + 1 `to_config_map` `ConfigValue::Bool` arm.
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` — `resolve_interface_shells` + per-slice union precompute + Pass 1 source switch in `compute_region_updates` + tests (`interface_shells_false_uses_collective_upper_cover`, `interface_shells_true_marks_bottom_on_other_material`, `interface_shells_defaults_are_identity_single_region`, `interface_shells_disjoint_objects_unaffected`).
  - `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` — `resolved_bool` arm for the new key.
  - `docs/config/host-keys.toml` — 1 `[resolved_config]` row (`interface_shells = { default = false }`).
  - `docs/DEVIATION_LOG.md` — DEV-192; `docs/15_config_keys_reference.md` — regen via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons: (a) declare the key in perimeter-module manifests and read it module-side — rejected because the decision is cross-layer (a layer's shell state depends on neighbours), which the per-layer module dispatch cannot see; the prepass is the architecture's cross-layer seam (rule 4: new decision points go where the architecture puts them). (b) Fold into draft packet 299 — rejected: 299's preflight is closed and its four stages own different decisions (vertical shells, extra-solid insertion, sparse combination); the multi-material gate is the P76 packet boundary the queue already fixed, and 299's scope statement excludes it. (c) Build `detect_surfaces_type`'s per-layer surfaces cache as a second site — rejected: this tree has no per-layer surface cache; the Pass 1 differences are the tree's surface-typing site, and a second cache would double-cover the same layers for no new behaviour.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/resolved_config.rs` - role: single source of truth for the default + CLI binding + map arm; expected change: one macro row + one `to_config_map` arm (the macro emits `Default`, `apply_cli_key`, `host_config_keys`, and the ticket-126 overlay arm).
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - role: prepass host; the resolver + gate + tests land here; expected change: `resolve_interface_shells` + per-slice unions + Pass 1 source switch + four unit tests.
- `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`, `docs/config/host-keys.toml`, `docs/DEVIATION_LOG.md`, `docs/15_config_keys_reference.md` - role: lock arm + TOML row + DEV-192 + regen.
- Justification for the fourth group: the lock pair is one mechanical row each (the doc-15 regen reads the TOML row); the deviation row + regen are the packet's ledger close. No IR/WIT/emitter change exists to split further.

## Read-Only Context

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `351-520` only - purpose: `compute_region_updates` Pass-1 + Pass-2 shapes the gate extends.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1081-1106` only - purpose: `resolve_shell_counts`' `config_for` pattern the sibling resolver mirrors.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines `1137-1164` only - purpose: `rect` / `slice_with` fixture helpers the tests reuse.
- `crates/slicer-ir/src/resolved_config.rs` - lines `2035-2040` only - purpose: neighbouring `bridge_no_support` bool row the new row sits beside.
- `crates/slicer-ir/src/resolved_config.rs` - lines `682-693` only - purpose: `extract_bool` accepted spellings (`Bool` + `Int` 0/1).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Unrelated crates - delegate symbol lookups; do not browse
- `crates/slicer-wasm-host/test-guests/**` - no test-guest edit needed; no WIT change, no guest surface touched
- Draft packet `299-*/` - read Goal line only via dispatch if needed; never open its design file
- `modules/**` - no module manifest or emitter changes; the gate changes prepass buckets, never module dispatch
- `crates/slicer-gcode/src/serialize.rs` - the padding twin is read-only context (spelling witness); never edited

## Expected Sub-Agent Dispatches

- Question: exact `detect_surfaces_type` same-region-vs-collective arms (upper `upper_slices` source, lower overhang + extra-bottom construction, `!spiral_mode` conjunct); scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp`; return: `SUMMARY`; purpose: Step 2 gate borrow.
- Question: did `cargo test -p slicer-ir` pass after the row lands; scope: `crates/slicer-ir`; return: `FACT`; purpose: Step 1 gate.
- Question: did `cargo xtask gen-config-docs --check` pass; scope: repo root; return: `FACT`; purpose: Step 4 gate.

## Data and Contract Notes

- IR/manifest contracts: none — no new IR field, no manifest row, no WIT line. The gate changes the existing `top_solid_fill` / `bottom_solid_fill` buckets and the existing `top_shell_index` / `bottom_shell_index` stamps.
- WIT boundary: untouched (no new accessor — the views already expose the buckets and stamps the gate changes).
- Determinism/scheduler constraints: the unions precompute sequentially before the per-timeline loop (the timeline loop's own comment keeps it sequential on ordinary prints). The per-slice union is order-independent (`union` of a set of polys for one slice index — no fold over timeline order), so repeated `false` runs are identical by construction (AC-2 pins determinism on the multi-body fixture).

## Locked Assumptions and Invariants

- Canonical default is locked: `false`. Defaults are identity on single-body prints (AC-4): one timeline means the collective union equals the region's own polys, so the default run matches the pre-packet baseline byte-for-byte — the inverse of 299's emitting default.
- DEV-192 clauses (minted at authoring, re-derive `max(DEV-*)` before writing): (a) the `!spiral_mode` conjunct is not borrowed — `spiral_mode` is unimplemented queue scope; (b) the `discover_vertical_shells` collective merge is not borrowed as a second site — the horizontal gate is the P76 analog and 299 owns that file's stage list; (c) the perimeter-side same-region masks are inherited, not re-armed — both modules consume the buckets the gate changes (the arachne second pass documents its in-module divergence from Orca's inline derivation); (d) canonical's `Print.cpp` reslice-invalidation entries ride ticket 124.
- The port's per-region timelines are BASE timelines (ShellClassification runs before PaintSegmentation): paint-split bodies share the pre-split classification; no per-variant re-gate exists.

## Risks and Tradeoffs

- The `false` default is live on multi-body prints: intersecting bodies newly classify collectively versus the pre-packet always-self-standing shape. AC-2 pins the intended change; AC-N1 pins the no-leak boundary (disjoint bodies identical); any sloping multi-body fixture churn lands as test-fixture updates with measured justification (map test discipline).
- The likeliest silent defect is an upper/lower source swap (top reads the lower union or vice versa) — pinned by AC-2/AC-3 asserting the two directions separately on mirrored fixtures.
- Draft-299 merge hazard: both packets edit `resolve_shell_counts`' neighbourhood and the same prepass file. The implementer rebases onto whichever lands first; the AC commands name only this packet's tests, so a 299-first landing cannot silently satisfy them.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — gate + four tests in one file)
- Highest-risk dispatch and required return format: surface-typing arm borrow — `SUMMARY` (≤200 words) from `PrintObject.cpp`; reject any reply pasting the whole function.

## Open Questions

None. `[FWD]` none pending; `[BLOCK]` none.
