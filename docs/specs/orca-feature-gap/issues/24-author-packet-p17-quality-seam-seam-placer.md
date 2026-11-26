# 24 — Author packet P17 — Quality / Seam — seam-placer

Type: task
Status: resolved
Assignee: wayfinder session (ses_f9bc6a0bcffeYU3rgi6IhdeNUQ) — claimed 2026-09-02
Blocked by: 06, 102
Map: ../map.md

## Question

Author the spec packet for **P17 — Quality / Seam — seam-placer** — 1 keys, Tier A plumbing, owner seam-placer. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P17 — Quality / Seam — seam-placer):

`staggered_inner_seams`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the authoring decision is recorded and the direct implementation
is complete; no retained packet is required.

## Answer

Resolved 2026-09-02 by direct implementation instead of creating a spec packet,
per the user's ruling that this single-key change was small enough to complete
in-session.

`staggered_inner_seams` is now live in the seam-placer manifest and is parsed by
`SeamPlacer::from_config` (`modules/core-modules/seam-placer/src/lib.rs`) with a
canonical default of `false`. `run_wall_postprocess`
(`modules/core-modules/seam-placer/src/lib.rs`) shifts only eligible inner loops
associated with the selected outer contour, traverses each closed loop forward,
clamps the offset to the local width profile, and preserves closure, feature
flags, widths, non-target walls, and the resolved outer seam identity.

The implementation keeps XY-only closure semantics for variable Z/width closing
metadata and normalizes corner-turn signs before applying the geometry-based
convex/concave correction. Disjoint and nested contour regressions prevent
staggering unrelated inner loops. The generated config reference was refreshed.

The remaining parity limitation is deliberate: `SeamCandidate`
(`crates/slicer-ir/src/slice_ir.rs`) does not carry canonical candidate-local
angle metadata, so the placer uses a deterministic angle derived from the
stored outer wall geometry. Exact canonical corner parity would require
threading that metadata through the IR.

## Direct implementation

Focused coverage in
`modules/core-modules/seam-placer/tests/staggered_inner_seams_tdd.rs` covers
default-off identity, forward interpolation, width clamping, final-to-first
wraparound, XY-only closure, reversed winding, outer/inner target filtering,
and disjoint/nested wall preservation. The seam-placer suite, workspace
all-target checks, workspace clippy, literal/config-doc checks, fresh guest
artifacts, and the seam-placer runtime contract test pass.
