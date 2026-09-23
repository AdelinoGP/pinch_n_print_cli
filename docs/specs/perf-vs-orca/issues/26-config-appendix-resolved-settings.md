# Config appendix must reflect resolved settings

Type: task
Status: open

## Question

The PNP G-code config appendix (`crates/slicer-gcode/src/serialize.rs`'s static
defaults + same-name overlay) prints **static defaults** for keys whose
resolved-config name differs from the appendix key name: `wall_loops` always
prints `2` regardless of `wall_count` (measured 2026-09-22 during the
[Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md)
work: `wall_count` 1 and 3 both printed `wall_loops = 2` while the wall output
changed accordingly — same-name keys like `wall_generator`,
`sparse_infill_pattern`, `support_type` overlay correctly).

The ticket-11 matched job used 2 walls, so its disclosure is coincidentally
correct. Any other wall count silently mis-discloses the settings evidence the
fairness contract ("validate actual generator dispatch per run", labels prove
nothing) relies on.

Work:

- Derive appendix values from the resolved config (map `wall_count` to
  `wall_loops`; audit every static table key for the same name-mismatch lie).
- Add a check that a changed setting shows up in the appendix, so the
  disclosure surface cannot silently rot again.

Disclosure-only: no slicing behavior changes. Parallel-takeable (not a
timing/acceptance ticket).
