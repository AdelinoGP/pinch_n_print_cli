# Ticket 22 — raw probe lines (verbatim extraction)

Probe lines as emitted, extracted from the full stderr captures. The captures
themselves are heavy (tens to hundreds of MB) and stay in gitignored
`target/t22/` per the map's evidence policy; these lines are the evidence.

## benchy — `target/t22/benchy-final.jsonl` (2026-09-24, probe build)

```
t22-planner-split p0_volumes_new=42616us/1 p3c_collision_ladder=136430us/27 p2_contacts=319211us/1 p3d_avoidance_ladder=277876us/16 p3_drop_nodes_loop=414214us/1 p4_f14_and_erase=292us/1 p5_smooth_nodes=18058us/1 p6d_inflate_occupancy=2268224us/365 p6e_node_roles=91468us/186 p6f_node_ellipses=120342us/186 p6a_total=4248474us/183 p6a_carve_per_region=4249304us/716 p6c_union_simplify=2430656us/716 p6b_final_difference=266150us/716 p6_emit_pass=9484857us/1 p7_stamp_and_interpolate=4973us/1 p8_push_entries=8us/1 p1_plan_for_object=10244424us/1 carve_disjoint=11018 carve_overlap=10694
```

```json
{"probe":"PERF-T22-PROBE","label":"PrePass::SupportGeometry","offset":{"calls":4977,"wall_ms":2410.9344,"batch_calls":28,"batch_wall_ms":31.7574,"batch_inner_ms":244.0561,"batch_items":6480,"batch_parallel":27},"clip":{"calls":29000,"wall_ms":5649.6097,"batch_calls":0,"batch_wall_ms":0,"batch_inner_ms":0,"batch_items":0,"batch_parallel":0,"bbox_disjoint_calls":12237,"bbox_disjoint_ms":1470.7255,"bbox_overlap_ms":4178.8842},"simplify":{"calls":0,"wall_ms":0,"batch_calls":0,"batch_wall_ms":0},"exactz":{"hits":0,"misses":179,"cross_section_ms":565.5188},"aggregate":{"calls":1,"wall_ms":841.6865,"validate_ms":835.0619,"union_ms":0.1068,"territory_ms":0,"overlap_ms":0.0941},"plan_clone_ms":0,"commit_ms":0}
```

Run context: `slice_complete` 39,449 ms, `degraded:false`,
`non_fatal_error_count:0`; `module_complete` `com.core.tree-support-planner`
9,065 ms; `stage_complete` `PrePass::SupportGeometry` ~9.1 s.

## base — `target/t22/base-final.jsonl` (2026-09-24, complete bracket set)

```
t22-planner-split p0_volumes_new=520925us/1 p3c_collision_ladder=756924us/49 p2_contacts=2178219us/1 p3d_avoidance_ladder=3510262us/38 p3_drop_nodes_loop=5247652us/1 p4_f14_and_erase=1855us/1 p5_smooth_nodes=121685us/1 p6d_inflate_occupancy=61369697us/864 p6e_node_roles=1497055us/433 p6f_node_ellipses=554200us/433 p6a_total=122750328us/504 p6a_carve_per_region=122753911us/1724 p6c_union_simplify=72764193us/1724 p6b_final_difference=2352289us/1724 p6_emit_pass=262039495us/1 p7_stamp_and_interpolate=33266us/1 p8_push_entries=20us/1 p1_plan_for_object=269639934us/1 carve_disjoint=78832 carve_overlap=69161
```

```json
{"probe":"PERF-T22-PROBE","label":"PrePass::SupportGeometry","offset":{"calls":23535,"wall_ms":65077.5671,"batch_calls":50,"batch_wall_ms":509.1237,"batch_inner_ms":3825.4839,"batch_items":24255,"batch_parallel":49},"clip":{"calls":171636,"wall_ms":194039.6183,"batch_calls":0,"batch_wall_ms":0,"batch_inner_ms":0,"batch_items":0,"batch_parallel":0,"bbox_disjoint_calls":79369,"bbox_disjoint_ms":52588.6737,"bbox_overlap_ms":141450.9446},"simplify":{"calls":0,"wall_ms":0,"batch_calls":0,"batch_wall_ms":0},"exactz":{"hits":0,"misses":431,"cross_section_ms":21135.1055},"aggregate":{"calls":1,"wall_ms":23809.5613,"validate_ms":23767.4564,"union_ms":0.3719,"territory_ms":0,"overlap_ms":0.4865},"plan_clone_ms":0,"commit_ms":0}
```

Run context: `slice_complete` 734,988 ms, `degraded:false`,
`non_fatal_error_count:0`; `module_complete` `com.core.tree-support-planner`
270,691 ms; `stage_complete` `PrePass::SupportGeometry` 295,032 ms.

## base — `target/t22/base-def.jsonl` (2026-09-24, probe build, first complete split)

```
t22-planner-split p0_volumes_new=280424us/1 p3c_collision_ladder=470151us/49 p2_contacts=1879893us/1 p3d_avoidance_ladder=2478938us/38 p3_drop_nodes_loop=3890399us/1 p4_f14_and_erase=1983us/1 p5_smooth_nodes=113034us/1 p6a_total=89540555us/504 p6a_carve_per_region=89544063us/1724 p6c_union_simplify=58215243us/1724 p6b_final_difference=1797400us/1724 p6_emit_pass=198177928us/1 p7_stamp_and_interpolate=29001us/1 p8_push_entries=15us/1 p1_plan_for_object=204107730us/1 carve_disjoint=78832 carve_overlap=69161
```

```json
{"probe":"PERF-T22-PROBE","label":"PrePass::SupportGeometry","offset":{"calls":23535,"wall_ms":49504.8731,"batch_calls":50,"batch_wall_ms":270.2787,"batch_inner_ms":2851.8733,"batch_items":24255,"batch_parallel":49},"clip":{"calls":171636,"wall_ms":146841.4184,"batch_calls":0,"batch_wall_ms":0,"batch_inner_ms":0,"batch_items":0,"batch_parallel":0,"bbox_disjoint_calls":79369,"bbox_disjoint_ms":38673.648,"bbox_overlap_ms":108167.7704},"simplify":{"calls":0,"wall_ms":0,"batch_calls":0,"batch_wall_ms":0},"exactz":{"hits":0,"misses":431,"cross_section_ms":27411.7175},"aggregate":{"calls":1,"wall_ms":30116.1635,"validate_ms":30061.0474,"union_ms":0.2959,"territory_ms":0,"overlap_ms":0.288},"plan_clone_ms":0,"commit_ms":0}
```

Run context: `slice_complete` 609,445 ms, `degraded:false`,
`non_fatal_error_count:0`; `module_complete` `com.core.tree-support-planner`
204,757 ms; `stage_complete` `PrePass::SupportGeometry` 235,233 ms.

## base — earlier capture (`target/t22/base-classic-on.jsonl`)

The first base capture (before the `p6d`/`p6e`/`p6f` brackets existed) is
consistent with the one above on the shared terms (`p6_emit_pass` 247.1 s /
97.7% of `p1_plan_for_object` 252.8 s; `clip` 187,005.79 ms over 171,636
calls). It also shows the probe's early JSON-less flush format:

```
[PERF-T22-PROBE] PrePass::SupportGeometry offset calls=23535 wall=57327.4459ms batch_calls=50 batch_wall=268.8828ms batch_inner=2872.5066ms items=24255 | clip calls=171636 wall=187005.7904ms batch_calls=0 batch_wall=0ms batch_inner=0ms items=0 | simplify calls=0 wall=0ms batch_calls=0 batch_wall=0ms | exactz hits=0 misses=431 cross_section=66179.8865ms | aggregate calls=1 wall=69955.6967ms validate=69845.9901ms union=2.4136ms territory=0ms overlap=0.4423ms | plan_clone=0ms commit=0ms
```
