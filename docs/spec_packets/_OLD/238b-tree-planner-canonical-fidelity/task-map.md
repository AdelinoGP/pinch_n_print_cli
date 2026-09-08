# Task Map: 238b-tree-planner-canonical-fidelity

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-369` | `Step 1` | `docs/specs/support-families-anchored-entities-plan.md` §12 div 1.1 | `crates/slicer-runtime/tests/executor/` (new pinning test), comment debt in tree-support-planner | `TreeSupport.cpp::generate_contact_points` (Q1) | S | forward-dep reconciliation vs 238a landed enum |
| `TASK-370` | `Step 2` | plan §12 div 5.1 | `modules/core-modules/tree-support-planner/src/lib.rs` (`move_out_expolys` + 3 call sites) | `move_out_expolys` (Q7) | M | from0 comment correction included |
| `TASK-371` | `Step 3` | plan §12 div 2.1/7.2 | decision record + retry/branch-A arg sites, `smooth_nodes_tdd.rs` | `smooth_nodes`, `drop_nodes` (Q2/Q8) | M | smoothing decision resolved in writing; DEV-141/143 dispositioned here |
| `TASK-372` | `Step 4` | plan §12 div 2.2/2.3/8.1 | `build_roles`, `structural_body_regions`, emit simplify gate; tree_family/mst tests | `draw_circles` (Q3) | M | golden-drift classification per E3 |
| `TASK-373` | `Step 5` | plan §12 div 4.1/4.2/7.1 | emit gates radius-baked; largest-part carve; wall_clearance tests | `calculate_collision`, `avoid_object_remove_extra_small_parts` (Q4/Q5) | M | retires F-13 interim disc test from production |
| `TASK-374` | `Step 6` | plan §12 div 4.6/5.5/5.6 | contact-seeding + branch-A inflation; parent-inherited roof counter; to_buildplate tests | `move_nodes`, `drop_nodes` (Q8/Q10) | S | F-14 raw-outline exception pinned, not "fixed" |
| `TASK-375` | `Step 7` | plan §12 div 3.3/4.3 | `common.wit` offset miter param; SDK host/batch; planner call sites at 3.0 | `ClipperUtils.hpp` defaults (Q6) | M | additive optional field; other callers unchanged |
| `TASK-376` | `Step 8` | plan §12 div 4.4/4.5 | `TreeVolumes::new` ctor simplify; union-composing variant | `TreeSupportData` ctor, `ExPolygon::simplify` (Q9) | S | topology unit test required |
| `TASK-377` | `Step 9` | plan §12 div 3.2 | shim reachability gate; diagnostics_tdd; boundary record | `generate_contact_points` overhang sampling | S | recorded boundary is a valid AC-10 outcome |
| `TASK-378` | `Step 10` | plan §12 div 5.7 (Ruling 3) | style enum in `from_config`; strong movement; hybrid minting; NEW tree_style_styles_tdd.rs; scheduler negative | `drop_nodes` is_strong; hybrid ePolygon minting | M | consumes 238a `support_style` declaration |
| `TASK-379` | `Step 11` | DEV-144 | `wall-counts: list<u32>` in `record support-plan-skeleton` (`prepass-support-geometry.wit`) + `wall_counts` on `SupportPlanSkeleton` (`slice_ir.rs`) + both marshal legs + emit site | `need_extra_wall` producer/consumer | M | schema minor bump derived-at-activation; T9 leg-pair discipline |
| `TASK-380` | `Step 12` | plan §8 human gate | gates; `tmp/p238b-*` artifacts; docs/07 registration via worker | reference G-code comparison | M | goldens rebless only with classified E3 justification |

Registration of these rows in `docs/07_implementation_status.md` is deferred to the
packet-owned closure step (Step 12 / TASK-380), executed through a worker dispatch.

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
