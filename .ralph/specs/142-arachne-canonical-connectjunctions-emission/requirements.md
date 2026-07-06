# Requirements: 142-arachne-canonical-connectjunctions-emission

## Packet Metadata

- Grouped task IDs: **none** (provenanced by the second-pass Arachne parity
  audit `target/arachne_parity_audit_20260706_020657.md` findings N2 and N4,
  encoded as committed red tests at `b2ea52b7`; the crosswalk is
  `docs/DEVIATION_LOG.md`'s `D-141-JUNCTION-BANDS` entry, which A2 supersedes
  for the junction-metadata + emission layer).
- Backlog source: `docs/07_implementation_status.md` (no `TASK-###` for N1–N13).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 141 (A1) fixed junction *geometry* — canonical `generateJunctions`
(upward half-edges, in-band beads, single `get_beding` at peak). But the layer
that assembles those junctions into toolpath *lines* remains divergent in two
blocking ways (N2 + N4), and one in-tree test
(`arachne_pipeline.rs:122`) actively asserts the divergent semantics A2 must
correct.

**N2 (line assembly):** PNP's `chain_junctions_for_bead` /
`emit_chain_lines` / `generate_toolpaths` (`generate_toolpaths.rs:401-758`)
collects every central `NORMAL` edge into one `full_chain`, then per bead index
emits one polyline spanning the chain, merging at shared vertices by *wider
width*. `ExtrusionJunction::perimeter_index` is zeroed at generation
(`:299-306`) and later overwritten by `assign_perimeter_indices`
(`pipeline.rs:384-390`) with the junction's *sequence position within its
line*. Canonical `connectJunctions` (`SkeletalTrapezoidation.cpp:2283-2327`)
instead pairs junctions **per quad** (`from_junctions` = junctions of
`edge_to_peak`, `to_junctions` = junctions of `edge_from_peak->twin`), merges
secondary fans by **`perimeter_index` pop-back dedup** (not width), and grows
lines via `addToolpathSegment` (`:2198-2234`) — extend the last `ExtrusionLine`
if the new `from` is within 10 µm of its last junction (same width, not a
3-way), else start a new line. `perimeter_index` on each junction **is the
bead/inset index** (`junction_idx`), which is what the pop-back rule keys on.
PNP's redefinition ("index within the wall sequence at that vertex",
`pipeline.rs:378-390`) breaks any downstream consumer expecting Orca semantics
and makes the pop-back rule unimplementable without re-plumbing.

**N4 (`is_odd`):** PNP sets `is_odd = bead_idx % 2 == 1`
(`generate_toolpaths.rs:632`) — "odd-indexed inset". Canonical
(`ExtrusionLine.hpp:62-70`) is "centerline bead of an odd bead count — a
gap-fill line with no companion on the other side, not a closed loop",
computed per segment in `connectJunctions` (`:2344-2354`): requires
`bead_count % 2 == 1`, `transition_ratio == 0`, the junction being the
innermost of the fan, and endpoint proximity (0.005 mm) to the quad's peak node.
With PNP's definition every 2nd, 4th, … wall is classified as gap-fill:
`remove_small_lines` (`arachne/remove_small.rs:57`, mirroring
`WallToolPaths.cpp:838-856`) only removes `is_odd && !is_closed` lines, so
short open fragments of REAL inner walls get silently deleted; the stitcher
groups by `is_odd` (`stitch.rs:83`), so mislabelled walls can't join their
peers; and the flag is forwarded verbatim across the host boundary
(`slicer-wasm-host/src/host.rs:1818`, `slicer-sdk/src/host.rs:721`).

This packet supersedes `D-141-JUNCTION-BANDS` for the junction-metadata +
emission layer only; A1's junction *geometry* (upward-half-edge, in-band,
no-clamp) remains canonical and untouched. A2 also corrects the in-tree test
`arachne_pipeline.rs:122` (`arachne_pipeline_perimeter_index_is_sequential_per_line`),
which actively asserts the divergent sequence-position semantics — a conflict
the audit didn't flag but grilling surfaced (user decision: update in place to
bead-index semantics, same test name, new assertion).

## In Scope

- **`perimeter_index = bead_idx` at junction generation** in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs`: A1's rewritten
  `generate_junctions` already emits in-band beads via `get_beding`; A2 sets
  `perimeter_index = junction_idx` (the bead/inset index) at the point of
  junction creation (`:315,326` in the current code — the `perimeter_index: 0`
  placeholders). Canonical: `junction.perimeter_index = junction_idx`
  (`SkeletalTrapezoidation.cpp:2064-2077`).
- **Canonical `connectJunctions` per-quad emission** in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs:401-758`: replace
  `chain_junctions_for_bead` / `emit_chain_lines` / `generate_toolpaths`'s
  whole-chain-polyline-per-bead scheme with per-quad pairing:
  - `from_junctions` = junctions of `edge_to_peak`, `to_junctions` = junctions
    of `edge_from_peak->twin`;
  - secondary fans from `edge_to_peak->prev` / `edge_from_peak->next->twin`
    concatenated after a **`perimeter_index` pop-back dedup**
    (`from_junctions.back().perimeter_index <= from_prev_junctions.front().perimeter_index`
    → pop_back, `SkeletalTrapezoidation.cpp:2302-2314`);
  - segments appended via `addToolpathSegment` (`:2198-2234`): extend the last
    `ExtrusionLine` of the inset when the new `from` is within 10 µm of its
    last junction (same width, not a 3-way), else start a new line;
    `new_domain_start` forces a fresh line at each polygon-domain start.
- **Canonical `is_odd` per segment** in `generate_toolpaths.rs`: replace
  `is_odd: bead_idx % 2 == 1` (`:632`) with the per-segment rule
  (`SkeletalTrapezoidation.cpp:2344-2354`): `bead_count % 2 == 1`,
  `transition_ratio == 0`, innermost junction of the fan, endpoint proximity
  (0.005 mm) to the quad's peak node.
- **`passed_odd_edges` keyed on the physical edge** (not `(bead, edge, twin)`
  triple) — `SkeletalTrapezoidation.cpp:2355-2361`.
- **Delete `assign_perimeter_indices`** from
  `crates/slicer-core/src/arachne/pipeline.rs:384-390` — it becomes dead once
  `perimeter_index` is set at generation. Remove the call site at `:373` too.
- **Update `arachne_pipeline.rs:122` in place**:
  `arachne_pipeline_perimeter_index_is_sequential_per_line` → assert
  `junction.perimeter_index == line.inset_idx` (the N2 contract). Same test
  name, new assertion, explicit in the commit message. (User decision during
  grilling: update in place, not delete or `#[ignore]`.)
- **Scope decision (NOT a silent absorb)**: `ExtrusionJunction::perimeter_index`
  is `u32` at `slicer-ir::slice_ir.rs:1744,1798`, forwarded verbatim through
  `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814`. The
  semantic change (bead index vs sequence position) is **wire-type-transparent**
  — NO schema change, NO WIT change. The only in-tree consumer of the old
  sequence semantics is `arachne_pipeline.rs:122` (updated in this packet).
  Surface this as a scope decision in the packet's commit message; do not
  silently absorb.
- **Fixture re-baseline (this packet's own stage only)**:
  `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` —
  re-record via self-capture if A1 didn't already cover it (A1 and A2 both
  touch `generate_toolpaths`; coordinate via commit order — A2 re-baselines
  only if its emission changes drift the fixture past A1's re-baseline).
  `crates/slicer-core/tests/fixtures/arachne/stitch_*.json` if the `is_odd`
  grouping change affects the stitch fixtures (likely — `stitch.rs:83` groups
  by `is_odd`).
- **Deviation-log entry**: `D-142-CONNECTJUNCTIONS-EMISSION` (new ID, addendum
  on `D-141-JUNCTION-BANDS`, supersession pattern — no in-place edits to
  A1's narrative).

## Out of Scope

- **N1 (junction geometry) and N7 (`BeadingPropagation`)** — owned by Packet A1
  (`141`). A2 builds on A1's upward-half-edge junction fans.
- **N3 (transition ends) and N8 (`generateExtraRibs`)** — Packet B (`143`).
- **N5 (π hack) and N6 (`filterNoncentralRegions`)** — Packet C (`144`), strictly
  after A2. A2 does NOT remove the π workaround.
- **N9–N13** — Packets D, E, F.
- **`cube_4color.3mf` e2e closure gate** — record-only across A2 (per
  `docs/specs/arachne-parity-N1-N13-plan.md`); Packet F blocks on green.
- **`cargo test --workspace`** — only at Packet F's closure ceremony.
- **New WIT/IR schema changes** — `perimeter_index` stays `u32`; the semantic
  change is wire-type-transparent.
- **`slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814`** —
  these forward `perimeter_index` verbatim; NO code change needed (the field's
  wire type is unchanged). The semantic change is transparent at the boundary.
  Do NOT edit these files.
- **`OrcaSlicerDocumented/` C++ oracle build** — declined; self-captured
  fixtures + red tests only.

## Authoritative Docs

- `docs/02_ir_schemas.md` — §"Arachne extrusion-line geometry (Packet 112)"
  (lines ~1091-1150) — read directly; purpose: confirm
  `ExtrusionJunction::perimeter_index` (`u32`) and `ExtrusionLine::is_odd`
  (`bool`) field shapes and confirm NO schema change.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` and
  `D-141-JUNCTION-BANDS` entries — read full; purpose: substrate + A1's
  addendum that A2 supersedes.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; purpose: cross-packet
  policies (the `arachne_pipeline.rs:122` in-place update decision, the e2e
  record-only policy, the fixture re-baseline distributed-per-packet policy).
- `.ralph/specs/113c-arachne-faithful-graph-construction/requirements.md`
  §"OrcaSlicer Reference Obligations" (the `orca-delegation` snippet) — A2
  carries this contract forward verbatim.

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2283-2327` — `connectJunctions` per-quad from/to pairing + `perimeter_index` pop-back merge.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2198-2234` — `addToolpathSegment` (extend vs new-line decision, 10 µm tolerance, `new_domain_start`).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2344-2354` — canonical `is_odd` per-segment rule.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2355-2361` — `passed_odd_edges` physical-edge key.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.hpp:62-70` — `is_odd` semantics.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:838-856` — `removeSmallLines` eligibility gate.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (`perimeter_index == line.inset_idx`), `AC-2` (even
  bead count → no `is_odd`), `AC-3` (inset-1 survives `remove_small_lines`)
  from `packet.spec.md`. All three are red tests committed at `b2ea52b7` and
  currently FAIL; A2 is done when they pass **without weakened assertions**.
- Negative cases: `AC-N1` (`arachne_pipeline.rs:122` updated in place to
  bead-index semantics and passes).
- Cross-packet impact: unblocks `143` (B — beading interpolation reads canonical
  junction fans), `144` (C — π hack removal strictly after A2).
- Refinements not captured in Given/When/Then:
  - `perimeter_index = bead_idx` is set at junction *generation* (in A1's
    rewritten `generate_junctions`), NOT in a post-pass. `assign_perimeter_indices`
    is deleted.
  - The `connectJunctions` per-quad walk reuses A1's upward-half-edge junction
    fans; A2 does NOT re-derive junction geometry.
  - `is_odd` is computed per segment during `connectJunctions`, not as a
    post-pass on `ExtrusionLine`.
  - `passed_odd_edges` is a `BTreeSet`/`HashSet` of physical edge indices (not
    `(bead, edge, twin)` triples).
  - `arachne_pipeline.rs:122` is updated in place (same test name, new
    assertion) — explicit in the commit message, per grilling decision.
  - `slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814` are
    NOT edited — `perimeter_index` is `u32` at both boundaries, wire-type-
    transparent. The semantic change is transparent at the boundary.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate
subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index -- n2_junction_perimeter_index_is_bead_index --nocapture 2>&1 \| tee target/test-output-a2-ac1.log` | AC-1: perimeter_index == inset_idx | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics -- n4_even_bead_count_lines_are_never_marked_odd --nocapture 2>&1 \| tee target/test-output-a2-ac2.log` | AC-2: even bead count → no is_odd | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics -- n4_even_inner_wall_survives_remove_small_lines --nocapture 2>&1 \| tee target/test-output-a2-ac3.log` | AC-3: inset-1 survives remove_small_lines | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_perimeter_index_is_sequential_per_line --nocapture 2>&1 \| tee target/test-output-a2-neg1.log` | AC-N1: in-place update passes | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 \| tee target/test-output-a2-n1-still-green.log` | N1 stays green (A1's fix preserved) | FACT pass (expected — confirms A2 didn't regress A1) |
| `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast 2>&1 \| tee target/test-output-a2-n3-still-red.log` | N3 stays red (A2 doesn't own it) | FACT fail (expected) |
| `cargo test -p slicer-core --features host-algos --test generate_toolpaths --test stitch --test remove_small 2>&1 \| tee target/test-output-a2-regression.log` | generate_toolpaths/stitch/remove_small regression (fixtures re-baselined) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --config resources/test_config/cube_4color-arachne.json --output /tmp/a2-cube4color.gcode 2>&1 \| tail -5` then `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end --nocapture 2>&1 \| tee target/test-output-a2-e2e.log` | e2e closure delta (record-only per cross-cutting policy; A2 records the failure count in its commit msg, does NOT block on green) | FACT pass/fail + summary line (record-only) |
| `rg -q 'D-142-CONNECTJUNCTIONS-EMISSION' docs/DEVIATION_LOG.md` | Deviation log entry present | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (A2's surface is `slicer-core`-internal; no guest feed expected) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot
express:

- **A2 must keep N1 red tests GREEN.** A2 builds on A1's junction geometry; if
  A1's N1 red tests regress during A2, the implementer has broken A1's
  `generate_junctions` rewrite and must back out. The "stays green" verification
  command gates this.
- **A2 must keep N3 red tests RED.** A2 owns only N2+N4; if N3 accidentally
  turns green, the implementer has crossed scope into Packet B.
- **A2 must NOT remove the π hack (`pipeline.rs:334`) or the 0.1× filter-dist
  fudge (`pipeline.rs:272-277`).** Those are Packet C's (`144`) scope, strictly
  after A2.
- **`arachne_pipeline.rs:122` is updated in place** — same test name, new
  assertion (`perimeter_index == line.inset_idx`), explicit in the commit
  message. Do NOT delete the test or mark it `#[ignore]`.
- **`slicer-sdk/src/host.rs:717` and `slicer-wasm-host/src/host.rs:1814` are NOT
  edited.** `perimeter_index` is `u32` at both boundaries; the semantic change
  is wire-type-transparent. Editing these files would imply a schema change
  that doesn't exist.
- **Fixture re-baseline is atomic per fixture and records rationale.**
  `toolpaths_tapered_wedge.json` may already be re-baselined by A1; A2
  re-baselines only if its emission changes drift the fixture past A1's
  re-baseline. `stitch_*.json` (if it exists) likely drifts because `is_odd`
  grouping changes. Coordinate via commit order.
- **Deviation-log correction uses the supersession pattern** — new
  `D-142-CONNECTJUNCTIONS-EMISSION` + addendum on `D-141-JUNCTION-BANDS`. No
  in-place edits to A1's narrative.

## Context Discipline Notes

Packet-specific context-budget hazards:

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (~953 LOC) is the
  primary edit target for both steps — can be full-read for this packet (A1
  already full-read it; A2's context budget assumes the implementer has A1's
  context or re-reads).
- `crates/slicer-core/src/arachne/pipeline.rs` — range-read `:260-390` only
  (the `assign_perimeter_indices` deletion + call-site removal); do NOT read
  `:334` or `:272-277` (Packet C's scope).
- `crates/slicer-core/src/arachne/{stitch,remove_small}.rs` — read-only for
  A2 (the `is_odd` consumers); A2 changes the `is_odd` *producer*, not the
  consumers. `stitch.rs:83` and `remove_small.rs:57` are read-only confirmations.
- `crates/slicer-core/tests/arachne_pipeline.rs:122` — read the test + its
  fixture; the in-place update is surgical (one assertion block).
- Likely temptation reads to skip: `slicer-sdk/src/host.rs` (no edit —
  wire-type-transparent), `slicer-wasm-host/src/host.rs` (no edit),
  `OrcaSlicerDocumented/` (delegate), `modules/core-modules/arachne-perimeters/`
  (A2's surface is `slicer-core`-internal).
- Sub-agent return-format hints for the heaviest dispatches: the
  `connectJunctions` SUMMARY dispatch (`SkeletalTrapezoidation.cpp:2283-2327`)
  should request the per-quad from/to pairing structure + the pop-back merge
  rule explicitly, NOT just a callee summary. The `is_odd` SUMMARY
  (`:2344-2354`) should request the four conditions (`bead_count % 2 == 1`,
  `transition_ratio == 0`, innermost, endpoint proximity) explicitly.