# Ticket 24 same-run capture — `host:slice` span-vs-wall contradiction

Supports [host:slice closing_ex span contradiction](../../issues/24-host-slice-closing-span-contradiction.md)'s
`## Answer`. Attribution only (§3.2: no wall claims from instrumented/profiled
runs).

## Provenance

- 2026-09-23, this worktree (`feature/perf-vs-orca` at `3f51b7b9` + dirty docs),
  release `pnp_cli`.
- `cargo xtask build-guests --check` exit `0` before the run (§3.5).
- `tmp/3dbenchy.stl`, **default config** (not the matched job), 240 layers.
- One run with `--profile --instrument-stderr` together, so both sides of the
  contradiction come from a single capture — no cross-run mixing.

Repro (cmd):

```
target\release\pnp_cli.exe slice --model tmp\3dbenchy.stl --module-dir modules\core-modules --output <scratch>\out.gcode --profile --instrument-stderr 2> <scratch>\events.jsonl
target\release\pnp_cli.exe profile --from <scratch>\events.jsonl
```

## The two numbers, same run

`module_complete` (wall — its own event timestamps bracket it):

```
{"event":"module_start","timestamp_ms":1790197252410,...,"phase":"prepass","stage":"PrePass::Slice","module_id":"host:slice",...}
{"event":"module_complete","timestamp_ms":1790197256300,...,"module_id":"host:slice","status":"ok","elapsed_ms":3889,"wasm_peak_kb":0}
```

Run context (`phase_complete` / `slice_complete`): prepass 10,368 ms /
per_layer 14,057 ms / postpass 534 ms / whole slice 26,928 ms
(`degraded:false`, `non_fatal_error_count:0`).

`profile_summary` native rows (accumulated per-thread spans):

```
host:slice                    total_wall_ns 39,616,937,800   calls 5,921
  polygon_ops::closing_ex     total_wall_ns 34,277,317,800   calls 240
  polygon_ops::clip_polygons  total_wall_ns  2,926,661,500   calls 5,005
  polygon_ops::offset         total_wall_ns  2,412,809,300   calls 676
  <module self>               self_wall_ns         149,200
```

(Other native rows: `host:overhang_annotation` 14.594 s / 3,107,
`host:shell_classification` 14.400 s / 4,056 (incl. `closing_ex` 325.9 ms /
183), `host:native` 11.382 s / 30,854, `host:mesh_analysis` 169.6 ms / 343;
native total 80.162 s across 5 built-ins.)

## Arithmetic

- `closing_ex` spans 34.277 s inside a 3,889 ms wall bracket = **8.8x** — the
  mean concurrency of the per-layer fan-out.
- Σ all native rows (80.162 s) exceeds the whole run's wall (26.928 s).
- Row identity: 34,277,317,800 + 2,926,661,500 + 2,412,809,300 + 149,200 =
  39,616,937,800 ns — every nanosecond of the row is scope spans.
- Activation identity: 240 + 5,005 + 676 = 5,921 — every marked call flushed
  as its own per-thread "activation" (no nesting, `self == total` throughout).
- None of the above is possible as wall; all of it is expected as Σ per-thread
  spans.

Verdict and code mechanism: the ticket's `## Answer`. The original figures
(`host:slice` 25.9 s / `closing_ex` 22.3 s vs `module_complete` 3,152 ms,
stable at 22.7 / 23.5 / 22.3 across three runs —
`../perf-emit-walls/FINDINGS.md`) show the same shape at their vintage; the
named repro captures there were scratch and were not retained.
