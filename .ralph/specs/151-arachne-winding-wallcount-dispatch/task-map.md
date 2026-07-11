# Task Map: 151-arachne-winding-wallcount-dispatch

No `docs/07_implementation_status.md` task IDs are grouped by this packet
(`task_ids: none` — audit-driven). The backlog rows live in
`docs/18_arachne_parity_audit.md`; this map ties each packet step to its
backlog row and closure artifact.

| Packet step | Gap / bug | Backlog row (docs/18) | Closure artifact |
| --- | --- | --- | --- |
| Step 1 | wall_count → max_bead_count wiring bug (discovered in planning; not a numbered gap — docs/18 does not mention it) | none (planning discovery) | new `D-151-WALLCOUNT-MAXBEAD-UNWIRED` entry in `docs/DEVIATION_LOG.md` (closed same-packet) |
| Step 1 downstream | bridge fixture regression: `arachne_parity_pipeline_bridge_flow_factor_on_overhang` (15-lock file) and `arachne_parity_arachne_path_is_bridge_flag_set_per_vertex` (Caller B, 15-lock file) — both fixed same-packet via the `native_bridge_region` helper (bridge area enlarged to 12×12, Caller B local updated to match; user-approved exception to "test bodies immutable"). `arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one` (gap test, packet-150 G5) is a SEPARATE genuine D-104g thick_bridges stub parity gap and remains open. | none (Step 1 downstream) | `D-151b-BRIDGE-FIXTURE-REGRESSION` entry in `docs/DEVIATION_LOG.md` **closed** (2026-07-11): helper + Caller B local fixed; both 15-lock tests now pass with real coverage |
| Step 2 | G1 `wall_direction` | headline `:54`, table row `:176` | G1 row marked closed (P151) |
| Step 3 | G2 `only_one_wall_first_layer` | headline `:62`, table row `:177` | G2 row marked closed (P151) |
| Step 4 | G7 `overhang_reverse` | headline `:111`, table row `:182` | G7 row marked closed (P151); `D-104c-OVERHANG-REVERSE-NONE` (`DEVIATION_LOG.md:80`) closed |
| Step 5 | G9 `wall_maximum_resolution`/`wall_maximum_deviation` | headline `:131`, table row `:184` | G9 row marked closed (P151) |
| Steps 6a/6b | G8 spiral vase forces classic | headline `:122`, table row `:183` | G8 row marked closed (P151) |
| Step 7 | bookkeeping | — | Doc Impact greps all hit |

Related but explicitly out of scope: `DEV-070` (wall_sequence ownership —
`docs/07_implementation_status.md:262`), packets 150 (flow/percent type) and
152 (top-surface / inset renumbering).
