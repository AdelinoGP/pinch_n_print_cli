# Document profile wall-column accumulated-thread-time semantics

Type: task
Status: open

## Question

[host:slice closing_ex span contradiction](24-host-slice-closing-span-contradiction.md)
resolved the span-vs-wall contradiction as a units mismatch: the `--profile`
table's wall columns are accumulated per-thread spans, never wall. The verdict
is recorded in that ticket's `## Answer` and in the map's recipe traps, but the
columns themselves still describe their units as "Wall-clock nanoseconds" with
only an observer-effect caveat:

- `ScopeTotals::total_wall_ns` / `self_wall_ns`
  (`crates/slicer-wasm-host/src/profiling.rs`).
- `ProfileModuleRow::total_wall_ns` / `self_wall_ns`
  (`crates/slicer-runtime/src/profiling_report.rs`) — its doc comment reads
  "Total wall-clock nanoseconds covered by marks across every call."
- The `profiling_report.rs` module doc's "observer-effect contract" covers
  mark-inflation but not cross-thread summation: `flush_native` merges
  concurrent worker activations with `entry.wall_ns += wall_ns` and
  `scope.total_wall_ns += row.total_wall_ns`, and its own doc already calls
  the per-activation units "accumulated" only implicitly.

A reader who consults the type docs — the natural place — can still repeat the
error ticket 24 had to resolve: comparing `closing_ex`'s 22–34 s accumulated
span to `module_complete`'s ~3.2 s wall.

## Work

- Document the semantics where the columns are defined and where they are
  rendered: each value sums completed activations, and concurrent per-thread
  activations are added, so a run-wide total can exceed wall time by the mean
  concurrency; the value is a work-share signal, never comparable to
  `module_complete` / phase walls.
- Cover `ScopeTotals`, `ProfileModuleRow`, and the module-level doc; check the
  rendered summary headings and the closing note
  (`format_profile_summary`) for the same gap.
- Docs/comments only; no behavior change, no timing runs. If a comment change
  would require a code edit for clarity, keep it to docs.

Deliverable: the wall-column docs state accumulated-thread-time vs wall, so
the map's recipe-trap entry can point at them instead of carrying the
semantics alone.
