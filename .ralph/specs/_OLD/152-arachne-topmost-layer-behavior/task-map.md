# Task Map: 152-arachne-topmost-layer-behavior

No `docs/07_implementation_status.md` task IDs are grouped by this packet
(`task_ids: none` — audit-driven). The backlog rows live in
`docs/18_arachne_parity_audit.md`; this map ties each packet step to its
backlog row and closure artifact.

| Packet step | Gap | Backlog row (docs/18) | Closure artifact |
| --- | --- | --- | --- |
| Step 1 | shared plumbing (WIT `arachne-params` + mirrors) | prerequisite for both gaps; no own row | AC-4 `rg` hit in `common.wit`; guests rebuilt clean |
| Step 2 | G10 `removeSmallLines` top-layer exception | headline `:141`, table row `:185` | G10 row marked closed (P152) |
| Steps 3–4 | G3 `only_one_wall_top` (part 1 topmost single wall; part 2 second pass) | headline `:68`, table row `:178` | G3 row marked closed (P152); `D-104d-MIN-WIDTH-TOP-SURFACE-NONE` narrowed — arachne half landed, classic remainder split into `D-152-CLASSIC-MIN-WIDTH-TOP-SURFACE-REMAINDER` |
| Step 5 | bookkeeping | — | Doc Impact greps all hit |

Related but explicitly out of scope: the classic-perimeters
`min_width_top_surface` threshold behavior (successor deviation above;
read-and-discarded at `classic-perimeters/src/lib.rs:224-239`), the
`interface_shells` upper-slice branch (follow-up if a locking test needs it),
and packets 150/151 (percent config type; wall_count/winding).

With this packet, gaps G1–G10 from the audit are all closed (G11 excluded by
decision).
