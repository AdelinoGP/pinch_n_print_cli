# Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen

Type: task / attribution + candidate screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 177
Graduated from: [Destination ruling: CPU-corroboration semantics and sample count for cell closure](176-destination-cpu-corroboration-ruling.md),
using the pre-t172 fuel census in [t171](171-post-t169-prepass-reprice.md)
(where `com.core.tree-support` held **57.7–68.0 %** of all supports-on guest
fuel and `com.core.infill-linker` held **52.5–57.6 %** of benchy supports-off
guest fuel) and the post-t175 realization that `benchy-classic-on` (binding,
~1.57x est.) is **not** boolean-bound (only `2.93 s` `host:native` clip thread
remains) and both `< 1.0x` on `benchy` and the follow-on `<= 0.50x` goal need
real per-layer/guest cuts.

## Question

**Using [t177](177-post-t175-full-matrix-attribution.md)'s verified post-t175
per-layer wall and guest-fuel shares, what specific kernels inside
`modules/core-modules/tree-support/src/lib.rs` (`com.core.tree-support`) and
`modules/core-modules/infill-linker/src/` (`com.core.infill-linker`) dominate
remaining per-layer wall on the four `benchy` cells and `base` supports-on —
and what exact, output-preserving (or scoped-exception) candidates clear the
standing 0.80 % gate with meaningful headroom toward `< 1.0x` and `<= 0.50x`?**

Concretely:

1. **Reconcile against [t177](177-post-t175-full-matrix-attribution.md) first:**
   confirm the post-t175 per-layer phase wall, per-module wall/fuel share, and
   remaining headroom for both `com.core.tree-support` (supports-on) and
   `com.core.infill-linker` (all 8 cells, especially `benchy` supports-off).
   Do not rely on pre-t172 fuel figures once t177 is resolved.
2. **Attribute `com.core.tree-support` (`modules/core-modules/tree-support/src/lib.rs`)
   post-t66/t160/t175:**
   - Remember prior screens so they are not re-run blind:
     [t64](64-tree-support-renderer-attribution.md) &
     [t66](66-paint-policy-dead-walk-reduction.md) already landed the paint-walk
     fix and left `skeleton_wall_count` / `point_in_expolygon` bbox scans in fog
     (~15.9 % of renderer fuel then);
     [t160](160-tree-support-prepass-batch-probe.md) exonerated per-layer
     placement (batch-folding was slower; the renderer's cost is its own work,
     null-renderer ceiling was 9.2–12.6 % of cell wall then).
   - Break down current `tree-support` fuel and wall on `benchy-classic-on` and
     `base-classic-on` into named inner kernels (e.g., skeleton/wall generation,
     `point_in_expolygon` / containment scans, infill/interface hatching,
     in-guest `polygon_ops`, marshal/decode) and price the top 2–3 exact
     candidates.
3. **Attribute `com.core.infill-linker` (`modules/core-modules/infill-linker/src/`)
   post-t44/t62/t65/t172:**
   - Remember prior screens: [t44](44-clip-universe-hoist.md) hoisted the clip
     universe, [t62](62-infill-linker-post-hoist-residual.md) measured the
     post-hoist residual, and [t65](65-infill-linker-batch-execute-experiment.md)
     tested batching.
   - Break down current `infill-linker` fuel and wall on `benchy-classic-off`,
     `benchy-arachne-off`, and `benchy-classic-on` (`connect.rs`, `graph.rs`,
     `offset.rs`, `orchestrate.rs`, `lib.rs`) and price the top exact
     candidates (e.g., clip-universe / spatial culling, graph/nearest-neighbor
     lookup, or redundant per-path work).
4. **Price route impact & decide graduations:** state each candidate's ceiling
   in ms, % of cell wall (ordinary & accelerated), whether it clears the
   applicable gate (8-cell `0.80 %` for `infill-linker`; 4-cell `t137`-class or
   8-cell for `tree-support`), and graduate only candidates with a verified,
   non-trivial wall ceiling.

## Scope

- Blocked by [177](177-post-t175-full-matrix-attribution.md) so the screen
  starts from verified post-t175 per-layer and module shares.
- Attribution and candidate pricing only; no production commit in this take,
  any diagnostic probe cleanly reverted and saved as `probe.patch`.

## Pre-declared gate

**Attribution + screen only (no landing gate).** Correctness bar:

- `cargo xtask build-guests --check` exits 0 before and after probe revert.
- Baseline attribution passes reproduce `t175`'s active `reference.json`
  byte-for-byte (`exit 0`, `degraded = false`, `0` code-1200).
- Every candidate's ceiling is priced in both deterministic guest fuel and
  implied wall share against `t177`'s phase/module split (never from `--profile`
  thread-time as wall).

## Acceptance

- Evidence directory `docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/`
  with `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, `probe.patch` (if used),
  and checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` update, and graduated
  implementation ticket(s) for any candidate clearing the route bar.

## Answer

_(open)_
