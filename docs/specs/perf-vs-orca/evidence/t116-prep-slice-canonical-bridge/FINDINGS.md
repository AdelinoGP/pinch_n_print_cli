# t116 — canonical flat-bridge classification: candidate and 16-cell proof

Ticket: [PrePass::Slice candidate: canonical flat-bridge classification](../../issues/116-prep-slice-canonical-bridge-classification.md)
(wayfinder map [Slice performance vs OrcaSlicer](../../map.md)).
Take: 2026-11-04/05. Baseline = HEAD `456bd349` (the t115 post-merge tree);
candidate = this take's working tree. Matched job, `RAYON_NUM_THREADS=12`,
`--no-default-module-paths`, in-tree `modules/core-modules` (ordinary) and the
mode-distinct accelerated staging (accelerated).

## Verdict

**BUILD.** The canonical rule replaces a PnP-local discriminator that had no
canonical counterpart, and it is cheaper in every cell:

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
- **Paired gate: 8/8 cells clear the 0.80 % floor** at **11.1x–30.5x**, with
  process CPU corroborating in every cell (table in §4).

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

| cell | changed section types | fill geometry lines | structural geometry lines |
| --- | --- | ---: | ---: |
| benchy classic off | Bottom surface, Bridge, Gap infill, Inner wall, Internal Bridge, Outer wall, Sparse infill | 1215 | **0** |
| benchy classic on | Bottom surface, Bridge, Gap infill, Inner wall, Internal Bridge, Internal solid infill, Outer wall, Sparse infill, Support | 1208 | **0** |
| benchy arachne off | Bottom surface, Bridge, Inner wall, Internal Bridge, Sparse infill | 1663 | **0** |
| benchy arachne on | Bottom surface, Bridge, Inner wall, Internal Bridge, Internal solid infill, Outer wall, Sparse infill, Support | 1658 | **0** |
| base classic off | Bottom surface, Bridge, Gap infill, Inner wall, Internal Bridge, Outer wall, Sparse infill | 1325 | **0** |
| base classic on | Bottom surface, Bridge, Inner wall, Internal Bridge, Sparse infill, Support, Support interface | 1323 | **0** |
| base arachne off | Bottom surface, Bridge, Inner wall, Internal Bridge, Outer wall, Sparse infill | 1274 | **0** |
| base arachne on | Bottom surface, Bridge, Inner wall, Internal Bridge, Sparse infill, Support | 1274 | **0** |

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

## 4. Paired gate

Three interleaved pairs per cell, baseline ordinary vs candidate ordinary,
`RAYON_NUM_THREADS=12`. Wall is process creation-to-exit; CPU is kernel+user via
the process handle. The pair order alternates so neither arm always leads.

| cell | baseline median | candidate median | delta | floor (0.80 %) | x floor | CPU delta |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic off | 7.23 s | 5.79 s | 1.44 s (20.0 %) | 58 ms | **25.0x** | −16.6 s |
| benchy classic on | 11.58 s | 10.21 s | 1.37 s (11.9 %) | 93 ms | **14.8x** | −16.6 s |
| benchy arachne off | 7.27 s | 5.94 s | 1.34 s (18.4 %) | 58 ms | **22.9x** | −16.9 s |
| benchy arachne on | 11.75 s | 10.12 s | 1.64 s (13.9 %) | 94 ms | **17.4x** | −17.0 s |
| base classic off | 106.83 s | 83.98 s | 22.85 s (21.4 %) | 855 ms | **26.7x** | −278.3 s |
| base classic on | 176.41 s | 160.73 s | 15.68 s (8.9 %) | 1,411 ms | **11.1x** | −258.0 s |
| base arachne off | 106.39 s | 80.39 s | 26.00 s (24.4 %) | 851 ms | **30.5x** | −288.9 s |
| base arachne on | 185.47 s | 164.41 s | 21.06 s (11.4 %) | 1,484 ms | **14.2x** | −269.2 s |

**All eight cells clear the 0.80 % floor, process CPU corroborating in every
cell** (the candidate's median CPU is 17 s lower on benchy and 258–289 s lower
on base). The narrowest multiple is base classic-on at **11.1x**; the widest is
base arachne-off at **30.5x**.

Raw sample ranges (3 per arm) are in `reduced/summary.json` `ab.<cell>`.
Seven of eight cells are *strictly separated* on wall and all eight on CPU; the
exception is base classic-on, where the candidate's slowest pair (174.90 s)
sits 0.55 s above the baseline's fastest (174.35 s) — the medians are 15.68 s
apart and the floor multiple is 11.1x, so the gate is not marginal, but the
overlap is disclosed rather than smoothed over.

**Campaign interruptions.** The first run was killed mid-pair-2 by a session
recycle and the campaign was paused overnight on request; pairs 1–2 were
measured in the first window and pair 3 roughly four hours later on a host that
was otherwise idle. Each pair measures both arms back-to-back, so the pairing
absorbs the window shift, and the per-pair deltas are recorded in
`reduced/summary.json` (`ab.<cell>.pairs`) rather than being collapsed into a
single median.

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
- **Host load was not independently measured.** The slicer saturates the 12
  configured Rayon threads, so a CPU-load percentage cannot separate this
  workload from anything else on the box; the only load evidence recorded is
  free memory (≈16.7 GB free during the first window) and the absence of other
  user workloads. Every claim here is a paired A/B; no absolute wall is offered
  as a machine capability.
- **One cell is not strictly separated on wall.** Base classic-on's candidate
  slowest pair (174.90 s) sits 0.55 s above the baseline fastest (174.35 s); the
  gate is the median floor (11.1x) and the CPU medians separate by 258 s, but the
  overlap is recorded rather than hidden. See §4.

## Evidence index

- `run_t116.ps1` — campaign driver (`-Mode output`, `-Mode ab`).
- `audit_delta.py` — section-level delta classifier.
- `reduce_t116.py` — raw CSV + audit rows → `reduced/summary.json`.
- `verify.py` — exits 0 from a plain checkout; `--self-test` mutates the summary
  in memory and rejects every negative control.
- `reduced/summary.json` — the committed numbers.
