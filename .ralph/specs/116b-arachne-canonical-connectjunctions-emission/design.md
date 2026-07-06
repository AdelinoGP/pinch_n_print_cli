# Design: 116b-arachne-canonical-connectjunctions-emission

## Controlling Code Paths

- Primary code path: `crates/slicer-core/src/arachne/generate_toolpaths.rs:401-758` (`chain_junctions_for_bead`, `emit_chain_lines`, `generate_toolpaths`) — the whole-chain-polyline-per-bead + width-merge + `perimeter_index: 0` scheme A2 rewrites to canonical per-quad `connectJunctions`.
- Neighboring code paths: `generate_toolpaths.rs:632` (`is_odd: bead_idx % 2 == 1`), `pipeline.rs:384-390` (`assign_perimeter_indices` — deleted), `pipeline.rs:373` (call site — deleted), `arachne_pipeline.rs:122` (in-place update).
- Neighboring tests/fixtures: `arachne_parity_red_perimeter_index.rs` (N2 red test), `arachne_parity_red_is_odd_semantics.rs` (N4 red tests ×2), `arachne_pipeline.rs:122` (in-place update target), `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` + `stitch_*.json` (re-baseline candidates).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **`perimeter_index` semantic change is wire-type-transparent.** `ExtrusionJunction::perimeter_index` is `u32` at `slicer-ir::slice_ir.rs:1744,1798`, forwarded verbatim through `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814`. The semantic change (bead index vs sequence position) does NOT change the wire type — NO schema change, NO WIT change. A2 must NOT edit `slicer-sdk/src/host.rs` or `slicer-wasm-host/src/host.rs`; the change is transparent at the boundary. The only in-tree consumer of the old semantics is `arachne_pipeline.rs:122` (updated in place).
- Packet-specific constraint: **A2 must keep N1 red tests GREEN.** A2 builds on A1's junction geometry; regressing A1's `generate_junctions` rewrite means backing out.
- Packet-specific constraint: **A2 must NOT remove the π hack (`pipeline.rs:334`) or the 0.1× filter-dist fudge (`pipeline.rs:272-277`).** Those are Packet C's (`118`) scope, strictly after A2.
- Packet-specific constraint: **WASM staleness does NOT apply** — A2's change surface is `slicer-core`-internal; no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.

## Code Change Surface

- Selected approach: faithful port of canonical `connectJunctions` per-quad emission + `perimeter_index = bead_idx` + canonical `is_odd`, replacing PNP's whole-chain-polyline-per-bead + width-merge + sequence-position + inset-parity scheme. The `connectJunctions` walk reuses A1's upward-half-edge junction fans; A2 does NOT re-derive junction geometry.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — `chain_junctions_for_bead`/`emit_chain_lines`/`generate_toolpaths` (`:401-758`) rewritten to per-quad `connectJunctions` (from/to pairing, `perimeter_index` pop-back merge, `addToolpathSegment` line growth); `generate_junctions`'s `perimeter_index: 0` placeholders (`:315,326`) set to `junction_idx`; `is_odd: bead_idx % 2 == 1` (`:632`) replaced with per-segment canonical rule; `passed_odd_edges` keyed on physical edge.
  - `crates/slicer-core/src/arachne/pipeline.rs` — delete `assign_perimeter_indices` (`:384-390`) and its call site (`:373`).
  - `crates/slicer-core/tests/arachne_pipeline.rs` — `arachne_pipeline_perimeter_index_is_sequential_per_line` (`:122`) updated in place: assertion changes from `junction.perimeter_index == expected_idx` (sequence position) to `junction.perimeter_index == line.inset_idx` (bead index). Same test name.
  - `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` + `stitch_*.json` — re-baselined via self-capture if A2's emission changes drift them past A1's re-baseline.
- Rejected alternatives:
  - **Delete `arachne_pipeline.rs:122` and rely on the N2 red test as the sole oracle** — rejected during grilling (user decision: update in place). Keeps a regression guard at the pipeline level.
  - **Mark `arachne_pipeline.rs:122` `#[ignore]` with a pointer** — rejected (accumulates ignored tests).
  - **Edit `slicer-sdk/src/host.rs:717` / `slicer-wasm-host/src/host.rs:1814` to "reflect the new semantics"** — rejected (the field is `u32` at both boundaries; the semantic change is wire-type-transparent; editing them would imply a schema change that doesn't exist).
  - **Make `apply_transitions` absorb end-generation** — rejected (that's Packet B's N3 scope; A2 owns N2+N4 only).

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — role: N2 emission rewrite + N4 `is_odd` + `perimeter_index = bead_idx`; expected change: rewrite `:401-758` to per-quad `connectJunctions`, set `:315,326` to `junction_idx`, replace `:632` `is_odd` rule, rework `passed_odd_edges`.
- `crates/slicer-core/src/arachne/pipeline.rs` — role: delete dead `assign_perimeter_indices`; expected change: delete `:384-390` and call site `:373`.
- `crates/slicer-core/tests/arachne_pipeline.rs` — role: in-place update of the divergent test; expected change: `:122` assertion block updated to `perimeter_index == line.inset_idx`.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-core/tests/arachne_parity_red_perimeter_index.rs` — full (157 lines); purpose: AC-1 oracle.
- `crates/slicer-core/tests/arachne_parity_red_is_odd_semantics.rs` — full (194 lines); purpose: AC-2 + AC-3 oracle + the `FixedBeadingStrategy` + `two_bead_single_edge_graph` fixture shape.
- `crates/slicer-core/src/arachne/stitch.rs` — range-read `:83` (the `is_odd` grouping key); purpose: confirm `is_odd` consumer shape (A2 changes the producer, not the consumer).
- `crates/slicer-core/src/arachne/remove_small.rs` — range-read `:57` (the `is_odd && !is_closed` gate); purpose: same.
- `docs/02_ir_schemas.md` lines ~1091-1150 — purpose: `ExtrusionJunction::perimeter_index` / `ExtrusionLine::is_odd` field shapes + confirm NO schema change.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` + `D-116A-JUNCTION-BANDS` entries — purpose: substrate + A1's addendum.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — delegate parity checks via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-sdk/src/host.rs:717` — NOT edited (wire-type-transparent). Read-only confirmation only if needed.
- `crates/slicer-wasm-host/src/host.rs:1814` — NOT edited (same). Read-only confirmation only if needed.
- `crates/slicer-core/src/arachne/pipeline.rs:334` (π hack) and `:272-277` (0.1× fudge) — Packet C's scope; A2 leaves them.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:640-740` (`apply_transitions`) — Packet B's scope.
- `crates/slicer-core/src/beading/{distributed,widening,redistribute,outer_wall_inset,limited,factory}.rs` — Packet B's trait extension.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` — large JSONs; Packet F owns the cross-crate batch.

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "SUMMARY of `SkeletalTrapezoidation.cpp:2283-2327` `connectJunctions` — explicitly ask for the per-quad from/to pairing structure + the `perimeter_index` pop-back merge rule (`from_junctions.back().perimeter_index <= from_prev_junctions.front().perimeter_index` → pop_back); return ≤ 200 words, no code unless asked" — purpose: confirm Step 1's emission rewrite.
- "SUMMARY of `SkeletalTrapezoidation.cpp:2198-2234` `addToolpathSegment` — ask for the extend-vs-new-line decision (10 µm tolerance, same width, not 3-way) and the `new_domain_start` flag; return ≤ 200 words" — purpose: confirm line-growth shape.
- "SUMMARY of `SkeletalTrapezoidation.cpp:2344-2354` canonical `is_odd` — ask for the four conditions (`bead_count % 2 == 1`, `transition_ratio == 0`, innermost, endpoint proximity 0.005 mm) and the `passed_odd_edges` physical-edge key; return ≤ 200 words" — purpose: confirm Step 2's `is_odd` rewrite.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index -- n2_junction_perimeter_index_is_bead_index --nocapture`; return FACT (pass) or SNIPPETS (fail with assertion + ≤ 20 lines)" — purpose: validate AC-1.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT pass/fail" — purpose: validate AC-2 + AC-3.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast`; return FACT pass (expected — N1 stays green)" — purpose: gate A2 didn't regress A1.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — N3 stays red)" — purpose: gate scope.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_perimeter_index_is_sequential_per_line --nocapture`; return FACT pass/fail" — purpose: validate AC-N1 (in-place update).
- "Find all callers of `assign_perimeter_indices`; return LOCATIONS" — purpose: confirm no orphan call sites after deletion.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. `ExtrusionJunction::perimeter_index` stays `u32`; `ExtrusionLine::is_odd` stays `bool`. The semantic change is wire-type-transparent at `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814` — both files are NOT edited.
- WIT boundary considerations: **none**. No WIT/IR schema change. The `perimeter_index` semantic change is a `slicer-core`-internal contract change that is transparent at the host boundary (the field's wire type is unchanged).
- Determinism: A2's rewrite preserves determinism (per-quad pairing is index-ordered; the pop-back merge is deterministic given the `perimeter_index` values; `is_odd` is a deterministic per-segment predicate). `passed_odd_edges` is a `BTreeSet`/`HashSet` of physical edge indices (deterministic under ties via index-ascending).

## Locked Assumptions and Invariants

- `perimeter_index = bead_idx` is set at junction *generation* (in A1's rewritten `generate_junctions`), NOT in a post-pass. `assign_perimeter_indices` is deleted.
- `is_odd` is computed per segment during `connectJunctions`, not as a post-pass on `ExtrusionLine`.
- `passed_odd_edges` is keyed on the physical edge index, not `(bead, edge, twin)` triple.
- `arachne_pipeline.rs:122` is updated in place (same test name, new assertion) — explicit in the commit message.
- `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814` are NOT edited — wire-type-transparent.
- A2 keeps N1 red tests GREEN (gated) and N3 red tests RED (gated).
- A2 does NOT remove the π hack or the 0.1× filter-dist fudge (Packet C's scope).
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.

## Risks and Tradeoffs

- **The `connectJunctions` per-quad walk is the most complex rewrite in the A1→A2 chain.** It replaces a whole-chain-polyline-per-bead scheme with per-quad pairing + pop-back merge + `addToolpathSegment` line growth. Risk is contained by the N2 red test (the pop-back merge's observable is `perimeter_index == inset_idx`) and the existing `generate_toolpaths`/`stitch`/`remove_small` regression suite.
- **`is_odd` change affects `stitch.rs:83` grouping and `remove_small.rs:57` eligibility.** The consumers are unchanged (A2 changes the producer); the regression suite gates this. The N4 red tests are the parity oracle.
- **`arachne_pipeline.rs:122` in-place update could mask a regression if the new assertion is too weak.** The N2 red test (`arachne_parity_red_perimeter_index.rs`) is the strict oracle; the pipeline-level test is a regression guard, not the primary oracle.
- **Bisect confusion across A1→A2 boundary.** Between A1 and A2, N2/N4 red tests stay red. A2's commit message must record the boundary.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 1 — the `connectJunctions` emission rewrite, the bulk of the work).
- Highest-risk dispatch: the `connectJunctions` SUMMARY dispatch — its return could blow budget if it returns code instead of prose. Required return format: `SUMMARY ≤ 200 words, no code unless asked`.

## Open Questions

- [FWD] Does `addToolpathSegment`'s 10 µm tolerance need to be in slicer units (100 µm = 1000 units) or is 10 µm = 100 units? The audit says 10 µm; `docs/08_coordinate_system.md` says 1 unit = 100 nm = 0.1 µm, so 10 µm = 100 units. The implementer confirms via the delegated SUMMARY and `docs/08_coordinate_system.md`.
- [FWD] Does the `is_odd` endpoint proximity (0.005 mm) need to be in slicer units? 0.005 mm = 50 units (1 unit = 100 nm). The implementer confirms via the delegated SUMMARY.

None activation-blocking.