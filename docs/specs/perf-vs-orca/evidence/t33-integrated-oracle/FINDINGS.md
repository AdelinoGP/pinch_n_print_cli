# Ticket 33 — Integrated/external matched-output oracle gate: FINDINGS

Route ticket: [Integrated/external matched-output oracle gate](../../issues/33-integrated-external-matched-output-oracle-gate.md).
Precursor: [Integrated-parity oracle experiment](../../issues/23-integrated-parity-oracle.md).
Historical divergent record: [perf-split Finding 3](../perf-split/FINDINGS.md).

**Verdict: the oracle gate FAILS — the integrated leg is not output-equivalent to
the external leg, and the divergence is root-located in code.** The integrated
(native-dispatch) `Layer::InfillPostProcess` request is not enriched from the
arena's partitioned `SliceIR` the way the WASM leg's request is, so the infill
linker silently falls back to the un-partitioned union boundary and emits a
differing job (fewer `;TYPE:` sections, more printed sparse/solid/bridge path
length — paths the external leg clips away print natively). This also silently
bypasses ticket 37's empty-commit semantics whenever the only boundary is a
role partition. No timing claim can rest on integrated dispatch until this is
repaired; nothing was fixed, committed or re-timed in this take.

## Enabling repair (needed to run the gate at all)

`cargo xtask dist --edition integrated` failed preflight before this take:
`crates/pnp-cli/Cargo.toml` lacked an `integrated-raft-default` passthrough
feature and `slicer-integrated-modules` had no `raft-default` row. Packet 240b
 added the module after ADR-0056/0057's "every core module integrates" contract
(the registry contract is one feature per core module — editions ADR-0057);
`raft-default` is an ordinary `#[slicer_module]` `LayerModule`
(module path `modules/core-modules/raft-default`), so the repair is registry
drift, not a designed exclusion.

- `crates/slicer-integrated-modules/Cargo.toml`: added the path dependency and
  the `raft-default = ["dep:raft-default"]` feature.
- `crates/slicer-integrated-modules/src/lib.rs`: added the import, a
  `RAFT_DEFAULT_MANIFEST` const, and the 24th `integrated_registry!` row
  (`Layer` family, standard labels).
- `crates/pnp-cli/Cargo.toml`: added the `integrated-raft-default`
  passthrough feature.

Verification after the repair: `cargo check --workspace --all-targets` exit 0;
`cargo clippy --workspace --all-targets -- -D warnings` exit 0;
`cargo xtask check-literals` 0 violations; `check-test-quality --report` on the
touched lib 0 findings; `cargo xtask build-guests --check` exit 0;
`cargo test -p slicer-integrated-modules` green (1 full-coverage test target
incl. the registry's roster tests); staged dist re-plans and stages
1 binary + 0 modules (all 24 integrated, verified below).

## Provenance proofs (both legs, per snapshot)

- `target/dist/developer/pnp_cli.exe module diagnose --module-dir
  target/dist/developer/modules`: pass, 24 modules, all `external`.
- `target/dist/integrated/pnp_cli.exe module diagnose --module-dir
  target/dist/integrated/modules`: pass, 24 modules, all `integrated`.
- The external arm's slice invocation carries `--no-default-module-paths
  --no-integrated-modules` so no integrated row can serve it; the integrated
  arm carries `--no-default-module-paths` and no `--no-integrated-modules`
  (its own staged modules dir is empty — 0 directories — so nothing shadows).
- Guest freshness: `cargo xtask build-guests --check` exit 0 immediately
  before the proof slices.

## Measured gate run (benchy classic supports-off, matched-pair configs)

First differing-cell discriminator, one uninstrumented run per leg,
12 threads, `pnp-classic-supports-off.json`, `tmp/3dbenchy.stl`:

| arm | wall s | cpu s | cpu/wall | bytes | sha256 (first 12) |
| --- | ---: | ---: | ---: | ---: | --- |
| external | 23.535 | 129.141 | 5.487 | 4,330,081 | `9949c618c1fa` |
| integrated | 14.823 | 100.484 | 6.779 | 4,550,163 | `b5d129b6214a` |

Both legs `slice_complete status=ok, fatal=0, non_fatal=0, degraded=false`;
zero unclosed-loop warnings; zero shadow warnings on the integrated leg.

The `;TYPE:` census reproduces perf-split Finding 3's exact shape:

| TYPE | external | integrated |
| --- | ---: | ---: |
| Sparse infill | 204 | 143 |
| Bridge | 22 | 7 |
| Bottom surface | 24 | 7 |
| Top surface | 70 | 31 |
| Outer wall / Inner wall / Brim / Skirt / Gap infill / Internal Bridge / Internal solid infill | equal | equal |

## Root cause (localized in code, confirmed by two independent jobs)

`Layer::InfillPostProcess` gets its regions from two different constructions:

- **WASM leg:** `push_infill_postprocess_regions`
  (`crates/slicer-wasm-host/src/dispatch.rs`) iterates the arena's **`SliceIR`
  regions** (whose `sparse_infill_area` / `top_solid_fill` /
  `bottom_solid_fill` / `bridge_areas` are the partitioned, pairwise-disjoint
  fill polygons written by `sync_perimeter_infill_areas_into_slice`,
  `crates/slicer-runtime/src/region_partition.rs`) and projects them into
  `PerimeterRegionData` with wall geometry from the `PerimeterIR` donor.
- **Native leg:** `build_native_layer_request_impl`
  (`crates/slicer-wasm-host/src/marshal/native.rs`) builds
  `perimeter_regions` from `input.perimeter` directly via
  `PerimeterRegionView::from_ir` (`crates/slicer-sdk/src/views.rs`). The
  `PerimeterRegion` IR struct carries no role-partition fields at all, so the
  four partition accessors default to empty — exactly the "ADR-0028 fields
  default empty" shape whose wasm twin is enriched *after construction* by
  the slice-region arm.

The consumer's fallback then degrades silently:
`RoleBoundaries::from_view` (`modules/core-modules/infill-linker/src/
orchestrate.rs`) reads the four empty partitions; `is_partitioned()` is
`false`; `for_role` takes the union fallback and hands the linker
`infill_areas` for **every** role instead of the role's own partition. The
linker's re-clip (its 97.65% cost center, ticket 21) then runs against the
wrong boundary and its output differs from the tested external chain:

- fewer `;TYPE:` transitions (paths that the partitioned clip cuts into separate
  role-bounded pieces remain merged over the union), and
- **more printed infill/bridge/surface path length**: on the t44 frozen job,
  integrated prints +95.2 mm Bridge, +441.7 mm Internal Bridge, +470.1 mm
  Bottom surface, +572.5 mm Sparse infill, +698.7 mm Top surface, +760.6 mm
  Internal solid infill vs external — paths the external leg clips away are
  printed on the native leg (double-extrusion over other roles' areas).
- On exactly-equal-geodes layers (Z=1 of the recon) the bodies are the **same
  move multiset**, differing only by an inserted `M106 S255` (fan gating is
  order/bridge-occupancy sensitive) and `M73` R/S differences
  (`estimate_print_with_elapsed`/`inject_m73` are order-sensitive; the
  remaining-time estimate follows the reordering). The same mechanism — a
  different upstream entity grouping entering the same emitter — is the
  parsimonious explanation for the section-count deltas on the divergent
  layers too (`;TYPE:` sections are emitted only on `role_changed`;
  grouped/merged entity streams collapse sections).
- The `M73`/`M106` deltas and the per-role length deltas are consistent with
  one defect (postprocess-view projection), but only the role-partition loss
  is *code-proven* on the native leg; the emitter-order mechanism is the
  recorded mechanism for the observed section collapse, not a separately
  probed fix.

Ticket-23's repairs (region eligibility + per-region config at the native
view seam, `native_projection_filters_and_resolves_configured_regions` /
`view_seam_identity_tdd.rs`) do **not** cover this: they gate/resolve the
`SliceRegionView` list and per-region config; the postprocess enrichment
(fields the `PerimeterRegionView` never gets from `PerimeterIR`) is a
separate projection that the wasm leg implements in its dispatch arm and the
native leg omits.

Ticket-37's empty-commit protocol is also bypassed natively in the
all-empty-partition case: its wasm/nature-mirrored predicate depends on
`Some(empty)` reaching the linker as a *partitioned* boundary; with the
native leg's all-empty `RoleBoundaries` (never `is_partitioned()`),
`for_role` returns the union instead, so a natively-dispatched linker never
sees the `Some(empty)` verdict for a role the host partitioned away. The
empty-commit contract tests pass both legs because they feed constructed
inputs directly; through the real native request builder the
`is_partitioned()` distinction is unreachable.

## Corroborating reproduction (t44 frozen job, fresh snapshots)

Same divergence on the second job (frozen `benchy-arachne.json`,
0.5 nozzle / 3 walls / 25% gyroid; external arm is byte-identical to the
ticket-44 approved fresh baseline `1b71f83db378…` — the t44 job itself
reproduces, so the arms differ only by dispatch provenance):

| TYPE | external | integrated |
| --- | ---: | ---: |
| Sparse infill | 139 | 118 |
| Bridge | 18 | 3 |
| Bottom surface | 23 | 4 |
| Top surface | 32 | 18 |
| Outer wall / Inner wall / Brim / Skirt / Internal Bridge / Internal solid infill | equal | equal |

## Consequence for the map

1. **The oracle gate stays closed.** Integrated timings may not inform module
   perf work (and the all-integrated comparison used in perf-split Finding 3's
   process-CPU table is now attributable to this defect).
2. **A named fix is now scoped**: enrich the native `Layer::InfillPostProcess`
   perimeter views from the arena's partitioned `SliceIR` — the same
   donor/enrichment `push_infill_postprocess_regions` performs (including
   `tool_index` from `RegionMapIR` and `wall_source_region_id`), not just
   `set_config`. That repair needs its own authorization (it is a
   correctness repair in native dispatch, not part of this measurement
   take); after it, the gate reruns from proof.
3. The gate's PASS would have been bounded to examined cells; with FAIL on
   the first examined cell the bounded-PASS question is moot this take.

## Method caveats

- n=1 per cell per leg (one proof slice each); the gate is a byte/TYPE
  equality gate — the divergence is reproduced on both frozen jobs, so
  nondeterminism (DEV-093-class) is not the explanation; outer-leg stability
  is cross-checked by the archived external-vs-external scoreboard
  (`evidence/matched-pair/results/`) and by the t44 external proof rows.
- No wall/CPU ratio significance is claimed from the two proof runs: with
  output NON-equivalence, the integrated leg prints *more* geometry (more
  sparse path length), so its lower wall/CPU is not even a legitimate
  oracle-cost reading. Timing was captured for load context only and is
  disclosed per row.
- OrcaSlicer-side numbers are absent here by design: the ticket is an
  internal-dispatch qualification, not a PNP-vs-Orca scoreboard.

## Reproduction (raw tree)

Reduced proof evidence is tracked in-repo: per-arm CSV, both stderr event
streams (slice_complete, dispatch diagnostics), and the three scripts. The
~8.9 MB raw G-code pair was **not** retained (regenerable via the exact run
commands below; its sha256/bytes/TYPE counts are pinned in FINDINGS and CSV).

Rerun commands (one-shot guard requires clearing the cell's directory first):

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t33-integrated-oracle/run-t33.ps1 \
  -Phase proof -Arm external -Fixture benchy -Generator classic -Supports off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t33-integrated-oracle/run-t33.ps1 \
  -Phase proof -Arm integrated -Fixture benchy -Generator classic -Supports off
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t33-integrated-oracle/summarize-t33.ps1 \
  -Cells benchy-classic-off
```

(one-shot guard: existing rows for a cell block reruns; delete the raw dir to
re-derive. t44 recon used
`target/dist/{developer,integrated}/pnp_cli.exe slice --model tmp/3dbenchy.stl
--config docs/specs/perf-vs-orca/evidence/t44-clip-universe-hoist/benchy-arachne.json`.)