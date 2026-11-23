# Document profile wall-column accumulated-thread-time semantics

Type: task
Status: resolved (2026-10-07; docs/comments only — semantics documented at the
definition sites, the rendered heading/note, and the two owning docs; see
## Answer)
Assignee: agpen (current OpenCode wayfinder session), 2026-10-07 — human's small
non-timing ticket family, first in number order after [mesh_ops fixture
limits](29-mesh-ops-fixture-limits.md) resolved

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

## Answer — resolved 2026-10-07

**The wall columns now state what they are, at every surface a reader touches:
each value sums one span per completed activation *and* concurrent worker
activations, so a run-wide total is accumulated thread-time — a work-share
signal that can exceed the run's own wall-clock by the mean concurrency, and is
never comparable to `module_complete`/phase walls or process CPU.** Docs and doc
comments only; no behavior change beyond the rendered table's heading/note
strings, and no timing runs.

Where the semantics now live (paths from the workspace root, per the in-tree
citation style):

- **`crates/slicer-runtime/src/profiling_report.rs`** — a new module-doc section
  `Wall columns are accumulated thread-time, never wall` names `flush_native`'s
  per-thread flush and its additive merges (`entry.wall_ns += wall_ns`,
  `scope.total_wall_ns += row.total_wall_ns`) and carries the ticket-24
  exemplar (`closing_ex` ≈34 s accumulated inside a `host:slice` block whose
  `module_complete` was 3,889 ms). Type docs updated on
  `ProfileScopeRow::{self,total}_wall_ns`, `ProfileModuleRow::{total,self}_wall_ns`,
  `ProfileSummary::wall_total_ns`, `ProfileUnit::WallNs`, and the `flush_native`
  doc. The rendered wall heading is now
  `units: wall-clock accumulated thread-time, not run wall` with a second line
  naming the parallelism, and the closing note gains the accumulation rule and
  the explicit "never compare it to a module_complete/phase wall".
- **`crates/slicer-wasm-host/src/profiling.rs`** — `ScopeTotals::{total,self}_wall_ns`
  docs now say the fold sums completed activations and that run-wide consumers
  add them across calls and threads, so a summed value is accumulated
  thread-time, never wall.
- **`docs/17_agent_debugging.md`** — the "Reading it" list and the
  observer-effect contract each gain an accumulated-thread-time bullet, the
  JSONL `wall_ns` fields are annotated, and the rendered example shows the new
  heading.
- **`docs/09_progress_events.md`** — the `1.5.0` `profile_summary` row
  annotates `wall_total_ns` / `total_wall_ns` / `self_wall_ns` as accumulated
  thread-time.
- **`docs/adr/0055-fuel-based-module-profiling.md`** — a Consequences bullet
  records the unit alongside the existing mark-inflation limitation.

**Falsifiability:** `summary_always_states_the_observer_effect_contract`
(`profiling_report.rs`) now asserts the rendered note states
`accumulate one span per completed activation` and
`never compare it to a module_complete/phase wall`, so a future edit that drops
the unit from the rendered table fails a test rather than silently restoring
the ticket-24 trap.

**Same-session corroboration (not a timing claim):** the fresh live-slice
output printed by `profile_flag_tdd::profile_on_reports_host_builtins_in_wall_clock_units`
shows `host:slice` at `total_wall_ns` 1,349,251,000 (3,664 calls) while
`slice_complete.elapsed_ms` is 28,068 — the columns are a fold over 3,664
activations, not a run interval. Ticket 24's evidence remains the exemplar of
the failure mode (Σ native rows 80.162 s against a 26.928 s run).

**Validation:** `cargo test -p slicer-runtime --lib profiling_report` (22
passed), `cargo test -p slicer-wasm-host --lib profiling` (16 passed),
`cargo test -p slicer-runtime --test unit profile` (2 passed, including the
fresh live-slice fixture), `cargo test -p pnp-cli --lib profile_cmd` (7
passed), `rustfmt --check` on both touched Rust files, `cargo build
--workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo xtask check-literals`, and touched-file `cargo xtask
check-test-quality --report` — all clean. No guest rebuild was needed (no
`modules/` change) and no wall claim is made.

**Boundary:** this is documentation of the measurement unit, not a timing run
or an optimization. The map's recipe-trap entry can now link here instead of
carrying the semantics alone.
