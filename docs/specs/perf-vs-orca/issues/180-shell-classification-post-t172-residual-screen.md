# `ShellClassification` post-t172 residual attribution and speedup screen

Type: task / attribution + candidate screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 177
Graduated from: [Destination ruling: CPU-corroboration semantics and sample count for cell closure](176-destination-cpu-corroboration-ruling.md),
because `PrePass::ShellClassification` was the #1 stage across all four
supports-off cells (**32.2–44.0 %** of cell wall in pre-t172
[t171](171-post-t169-prepass-reprice.md), plus **11.6–15.0 %** on every
supports-on cell) — making its surviving post-t172 residual a direct multi-cell
lever that can simultaneously close `benchy` supports-off (`1.14–1.15x → < 1.0x`),
chip away at all four supports-on cells (including binding `benchy-classic-on`),
and push `base` supports-off (`0.77–0.79x`) toward the follow-on `<= 0.50x`
target.

## Question

**After [t166](166-shell-opening-canonical-parity.md),
[t169](169-canonical-offset-edge-decimation-and-flat-opening.md), and
[t172](172-port-offset-concave-crossing.md) brought Clipper2 per-call offset
cost to ~1.04–1.30x of canonical C++, what remains inside
`crates/slicer-runtime/src/slice_postprocess_prepass.rs::commit_shell_classification_builtin`
— and what work-set, locality, or parallelism candidates cut its surviving
post-t172 wall across all eight cells?**

Concretely:

1. **Reconcile against [t177](177-post-t175-full-matrix-attribution.md) first:**
   confirm `PrePass::ShellClassification`'s exact post-t175 wall share,
   effective parallelism (`thread / wall`), and host clipper op breakdown
   (`offset`, `clip_polygons`, `offset2_ex`, `closing_ex`) across all 8 cells.
2. **Decompose `commit_shell_classification_builtin`
   (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) into named
   sub-phases and call sites** on `benchy-classic-off`, `benchy-classic-on`,
   and `base-classic-off`:
   - Pass 1 top/bottom surface detection (`apply_opening` + `difference`/`intersection`),
   - Pass 2 shell-thickness shadow/propagation cascade,
   - Internal-bridge site computation (`compute_internal_bridge_layer` /
     `gate_internal_bridge_sites` — noting `t133` S5 already windowed the
     negative erosion and `t171` priced S5's residual at `0.83 %` pre-t172),
   - Serial vs parallel brackets (checking whether any serial commit/merge
     loop like [t75](75-shellclassification-phase-b-commit.md) or
     low-fan-out pass still caps effective parallelism below 12 threads),
   - Work-set comparison against canonical `v2.5.0` (`40ecc66f78`)
     `PrintObject::detect_surfaces_type` / `process_external_surfaces` /
     `discover_vertical_shells` (citing file + function only, never line
     numbers): are there still redundant full-layer booleans/offsets or
     whole-layer operands where canonical operates on a narrower active set or
     fewer passes?
3. **Price concrete candidate shapes:** state each candidate's ceiling in ms
   and % of cell wall across the supports-off and supports-on cells, classify
   each as **exact output-preserving** vs **canonical-parity under the existing
   t111/t128 prepass exception**, and verify whether it clears the standing
   all-8-cell `0.80 %` gate.
4. **Decide graduation:** graduate implementation ticket(s) for any candidate
   that materially closes `benchy` supports-off (`1.14–1.15x → < 1.0x`) and/or
   cuts `benchy-classic-on` + `base` supports-off toward `<= 0.50x`, or record
   explicitly if `ShellClassification` is already at its canonical floor.

## Scope

- Blocked by [177](177-post-t175-full-matrix-attribution.md) so the screen
  starts from verified post-t175 stage shares.
- Attribution and candidate pricing only; no production commit in this take,
  any diagnostic probe cleanly reverted and saved as `probe.patch`.

## Pre-declared gate

**Attribution + screen only (no landing gate).** Correctness bar:

- `cargo xtask build-guests --check` exits 0 before and after probe revert.
- Baseline attribution passes reproduce `t175`'s active `reference.json`
  byte-for-byte (`exit 0`, `degraded = false`, `0` code-1200).
- Every candidate's ceiling is re-derived from measured per-subphase thread
  time and measured stage parallelism on the post-t175 tree.

## Acceptance

- Evidence directory `docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/`
  with `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, `probe.patch` (if used),
  and checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` update, and graduated
  implementation ticket(s) for any surviving route candidate.

## Answer

_(open)_
