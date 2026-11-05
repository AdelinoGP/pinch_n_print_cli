# t116 — canonical flat-bridge classification: candidate and 16-cell proof

Ticket: [PrePass::Slice candidate: canonical flat-bridge classification](../../issues/116-prep-slice-canonical-bridge-classification.md)
(wayfinder map [Slice performance vs OrcaSlicer](../../map.md)).
Take: 2026-11-04/05. Baseline = HEAD `456bd349` (the t115 post-merge tree);
candidate = this take's working tree. Matched job, `RAYON_NUM_THREADS=12`,
`--no-default-module-paths`, in-tree `modules/core-modules` (ordinary) and the
mode-distinct accelerated staging (accelerated).

## Verdict

**Candidate built and output-verified; the timed gate is PAUSED, not yet decided.**
The campaign was paused on 2026-11-05 with the paired A/B at 30 of 48 rows
(pair 1 complete, pair 2 complete on seven of eight cells, pair 3 unrun). Nothing
below claims a wall result that has not been measured.

What is established:

- **16-cell output contract: met.** Both arms are byte-identical within each
  build in all eight cells (`ordinary ≡ accelerated`), the baseline reproduces
  the t115 post-merge reference set exactly, and the candidate moves the output
  in every cell.
- **Delta stop rule: not triggered.** The eight-cell audit finds **0 geometry
  lines** changed in any structural section. The wall and support differences are
  extrusion-token-only (same `G1` coordinates and feedrate, different `E`) on
  segments the new bridge flag covers, plus the retired
  `; flat_bridge_closing_join = miter` appendix row and time-estimate `M73`
  counters.
- **Paired gate: PENDING.** See §4 for the exact state and the resume command.
  The 0.80 % floor is not yet asserted in this document.

The candidate also removes a config-surface lie: the retired key was the only
knob the prepass read for the closing.

## 1. What changed

`assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)
now applies canonical OrcaSlicer's classification instead of the PnP-local
enclosure test:

```text
bridged = opening_ex(flat_bridge, 0.1 × external_perimeter_width, Miter, 3.0)
```

- Deleted: `closing_ex(support = region.polygons − unsupported, 12 mm, join)`,
  the `closed − support` refill mask, the per-component `≥ 10 %` fraction gate,
  `MAX_FLAT_BRIDGE_SPAN_MM`, `FLAT_BRIDGE_ENCLOSURE_RADIUS_MM`,
  `FLAT_BRIDGE_ENCLOSURE_MIN_FRACTION`, `parse_flat_bridge_join` and
  `expoly_set_area`.
- Added: canonical's `ApplySafetyOffset::Yes` — the caller inflates the lower
  layer by `SCALED_EPSILON` (1 PnP unit = 1e-4 mm, square join) before
  `difference(current, lower)`.
- Added: canonical's lower-layer guard. `PrintObject::detect_surfaces_type`
  demotes a bottom with `lower_layer == nullptr` to `stBottom`, so the caller
  skips the flat-bridge assembly entirely when the object has no previous
  cross-section. This keeps the build-plate bottom a bottom and removes the
  layer-0 closing whose result `gate_bridge_areas_by_unsupported_span` threw
  away anyway.
- Added: `resolve_external_perimeter_width_mm`, mirroring
  `support_analysis_producer::resolve_contact_params` (`outer_wall_line_width`
  extension when absolute, else typed `line_width`, else 0.4 mm). Covered by four
  unit tests in the same module.
- Retired the `flat_bridge_closing_join` config key end to end:
  `ResolvedConfig` declaration plus its `PartialEq`/`Hash` clauses, the
  `docs/config/host-keys.toml` row, `docs/15_config_keys_reference.md`, the
  `host_keys_doc_lock_tdd` accessor, and the scheduler manifest's enum-metadata
  assertions (with the key retired, no declared key carries `wire_type: "enum"`
  any more — the macro branch stays, its only live instance is gone).

Tests re-derived under the canonical rule:

- `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`:
  `flat_bridge_gap_and_free_edge_keep_only_the_gap_component` became
  `flat_bridge_canonical_rule_keeps_gap_and_free_edge` (both components are
  bridges now), and the closing-join vertex-count guard
  `flat_bridge_enclosure_closing_avoids_round_arc_explosion` retired with the
  closing it guarded; its replacement `flat_bridge_opening_drops_sub_width_sliver`
  asserts the 0.06 mm sliver is opened away and the 4 mm gap survives in full.
- `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`: both flat-bridge
  fixtures kept their assertions; their doc-comments now name the canonical rule
  and the lower-layer guard.
- `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`
  (`wedge_multi_layer_top_bottom_evidence`): the y-extension underside is a
  free-edge bottom that canonical classifies as a **bridge**. The whole-file
  `Bottom surface` count drops to exactly one (the build-plate base) and the
  per-layer assertions now require `Bridge` — not `Bottom surface` — at print_z
  29.2.

## 2. Output: the 16-cell set

Eight cells × two arms per build, hashes from `target/t116/output/rows.csv`
(reduced into `reduced/summary.json`).

| cell | baseline ordinary | candidate ordinary | bytes (base → cand) | Δ bytes |
| --- | --- | --- | ---: | ---: |
| benchy classic-off | `39298f40…` | `35600ee4…` | 3,925,588 → 3,926,024 | +436 |
| benchy classic-on | `3b8fa86a…` | `596f6da5…` | 8,252,446 → 8,252,846 | +400 |
| benchy arachne-off | `9ef7617b…` | `f21df124…` | 4,064,304 → 4,064,091 | −213 |
| benchy arachne-on | `d4644c14…` | `e90f491d…` | 8,390,202 → 8,390,069 | −133 |
| base classic-off | `910afa32…` | `6a6a3d9d…` | 21,023,773 → 21,026,410 | +2,637 |
| base classic-on | `4b6d2843…` | `8f46fe92…` | 42,507,946 → 42,510,534 | +2,588 |
| base arachne-off | `7f36b089…` | `655c99d4…` | 20,846,194 → 20,847,171 | +977 |
| base arachne-on | `d8ef39a5…` | `8b5d1de0…` | 42,330,299 → 42,331,252 | +953 |

- **Baseline reproduction.** Every baseline ordinary hash equals the
  corresponding `output_audit.<cell>.post_sha256` in
  [`../t115-post-merge/summary.json`](../t115-post-merge/summary.json), and every
  baseline accelerated hash equals its ordinary partner.
- **Arm equality.** Candidate ordinary equals candidate accelerated in all eight
  cells (`reduced/summary.json` `arm_equality`).
- Byte deltas include the retired 37-byte appendix row
  (`; flat_bridge_closing_join = miter`); the sign follows the net
  bridge/fill re-role, which grows on base classic and shrinks on the two benchy
  arachne cells.

## 3. Delta audit (the stop rule)

`audit_delta.py` splits both files into per-layer, per-`;TYPE:` sections and
classifies every changed line as `extrusion_only` (identical motion, different
`E`), `appendix` (`;` comments: the config appendix and the time/filament
estimates), `progress_counter` (`M73`), or `geometry` (a moved toolpath).

| cell | changed section types | structural geometry lines |
| --- | --- | ---: |
| benchy classic-off | Bottom surface, Bridge, Gap infill, Inner wall, Internal Bridge, Outer wall, Sparse infill | **0** |
| benchy classic-on | … + Internal solid infill, Support | **0** |
| benchy arachne-off | Bottom surface, Bridge, Inner wall, Internal Bridge, Sparse infill | **0** |
| benchy arachne-on | … + Internal solid infill, Outer wall, Support | **0** |
| base classic-off | Bottom surface, Bridge, Gap infill, Inner wall, Internal Bridge, Outer wall, Sparse infill | **0** |
| base classic-on | … + Support, Support interface | **0** |
| base arachne-off | Bottom surface, Bridge, Inner wall, Internal Bridge, Outer wall, Sparse infill | **0** |
| base arachne-on | Bottom surface, Bridge, Inner wall, Internal Bridge, Sparse infill, Support | **0** |

**Reading the structural entries.** Every `Outer wall`, `Inner wall`,
`Support` and `Support interface` line that differs is an **`E`-token-only**
change on an unchanged `G1 … X Y Z … F` move: the per-vertex bridge flag feeds
perimeter/support flow, so a newly-classified bridge span prints its wall or
support segment at bridge flow. No coordinate, feedrate, travel, skirt, brim or
raft line moved. The remaining structural entries in `changed_types` are the
config appendix (`; filament used`, `; estimated printing time`) and `M73`
progress counters, both downstream of the time estimate.

The two visible non-geometry effects are therefore:

1. `; flat_bridge_closing_join = miter` disappears from every appendix (the key
   is retired), and
2. `M73 P/Q` values shift with the time estimate.

Both are recorded in `reduced/summary.json` `delta_audit.<cell>.appendix_changed_lines`
and `changed_types`.

## 4. Paired gate — PAUSED

**State at pause (2026-11-05).** `target/t116/ab/rows.csv` holds **30 of 48**
rows: pair 1 complete on all eight cells, pair 2 complete on seven of eight
(`base-arachne-on` unrun in pair 2), pair 3 unrun. The three-pair median gate has
therefore **not** been evaluated and no wall claim is made.

The two interruptions were infrastructure, not measurement failures: the first
run was killed mid-pair-2 (background process recycled), and the resumed run was
stopped on request when the human went to sleep. Rows are append-only, so every
completed run is kept.

Resume (idempotent — `-OnlyMissing` skips recorded `(pair, cell, arm)` rows):

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/run_t116.ps1 \
  -Mode ab -Pairs 3 -PairFrom 2 -PairTo 3 -Append -OnlyMissing -BatchId ab
python docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/reduce_t116.py
python docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/verify.py
```

**Provisional: the rows recorded so far (2 pairs, one cell with 1 pair), not the
gate.** Median wall of the recorded pairs, same pairing per cell:

| cell | pairs | baseline median | candidate median | Δ | × floor |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2 | 7.31 s | 5.82 s | 1.49 s (20.4 %) | 25.5x |
| benchy classic-on | 2 | 11.89 s | 10.24 s | 1.65 s (13.9 %) | 17.4x |
| benchy arachne-off | 2 | 7.42 s | 5.97 s | 1.45 s (19.5 %) | 24.4x |
| benchy arachne-on | 2 | 11.81 s | 10.28 s | 1.53 s (13.0 %) | 16.2x |
| base classic-off | 2 | 111.29 s | 81.33 s | 29.96 s (26.9 %) | 33.7x |
| base classic-on | 2 | 175.38 s | 155.38 s | 20.00 s (11.4 %) | 14.3x |
| base arachne-off | 2 | 104.68 s | 79.57 s | 25.10 s (24.0 %) | 30.0x |
| base arachne-on | 1 | 192.87 s | 176.14 s | 16.73 s (8.7 %) | 10.8x |

These are two-sample medians from a campaign that was interrupted twice; they
are **not** the three-pair gate and must not be quoted as the result. The
`-Mode output` single runs point the same way (base classic-off 111.63 → 85.70 s
wall, 1,118.6 → 803.8 s CPU; benchy classic-off 7.07 → 5.77 s wall, 59.1 →
42.8 s CPU), and those ran while clippy was still compiling on this host.


## 5. Disclosures

- **The t121 per-layer regression is present in both arms.** HEAD carries the
  post-merge per-layer dispatch regression (ticket 121), so the cell walls here
  are inflated relative to the pre-merge set (base classic-off 111.6 s against
  t103's 75.4 s). Both arms pay it equally; the paired delta isolates the
  prepass change. The absolute walls are the merged tree's, not performance
  claims about the pre-merge tree.
- **No Orca arm.** The post-merge Orca scoreboard is ticket 121's closing
  measurement; this take measures the candidate against the PnP baseline only.
  No ratio of record is quoted here.
- **Output-batch wall times are not timing claims.** The `-Mode output` batch ran
  while compilation (clippy) was still finishing on this host; only its hashes
  are used. Timing comes from the `-Mode ab` pairs.
- **Ordinary and accelerated arms use their own staged module trees.** The
  accelerated arm is the controlled-perimeter build
  (`cargo xtask dist --accelerated --edition developer`), the ordinary arm is
  `cargo build --release -p pnp-cli` + in-tree `modules/core-modules`. Guest
  WASM is unchanged by this ticket (host-only change); both freshness checks
  (`cargo xtask build-guests --check` and `--accelerated --check`) exit 0.
- **The safety offset is numerically subsumed by the opening.** Canonical's
  `SCALED_EPSILON` inflate (1 unit) only moves the unsupported boundary, and the
  opening at 0.04 mm removes boundary slivers anyway. It is implemented for
  canonical fidelity, not as a measured effect.

## Evidence index

- `run_t116.ps1` — campaign driver (`-Mode output`, `-Mode ab`).
- `audit_delta.py` — section-level delta classifier.
- `reduce_t116.py` — raw CSV + audit rows → `reduced/summary.json`.
- `verify.py` — exits 0 from a plain checkout; `--self-test` mutates the summary
  in memory and rejects every negative control.
- `reduced/summary.json` — the committed numbers.
