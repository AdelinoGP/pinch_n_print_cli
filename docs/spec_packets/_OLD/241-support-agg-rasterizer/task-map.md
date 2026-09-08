# Task Map: 241-support-agg-rasterizer

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-419` | `Step 1` | `docs/specs/support-families-anchored-entities-plan.md` §7 | `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` (new submodule) + `main.rs` `mod` line, `crates/slicer-runtime/tests/fixtures/golden/` (dir does not exist — create it) + `p241_baseline.json` | — | S | Pre-port measurement baseline on `SupportAdversarial.stl`; nothing else may precede it |
| `TASK-420` | `Step 2` | plan §3 Ruling 7 | none (design.md note only) | `SupportMaterial.cpp` class `SupportGridPattern` (delegated) | S | Read-only canonical fidelity probe before coding — NOT the port; the port is Steps 3-4 |
| `TASK-421` | `Step 3` | `docs/08_coordinate_system.md` (constraint) | `modules/core-modules/traditional-support-planner/src/agg_raster.rs`, crate Cargo.toml, `tests/agg_rasterizer_tdd.rs` | constructor + statics (Step-2 return) | M | Grid construction; AC-2 |
| `TASK-422` | `Step 4` | — | `agg_raster.rs`, `tests/agg_rasterizer_tdd.rs` | `extract_support` / `contours_simplified` / `seed_fill_block` | M | Seed fill + extraction + island filter; AC-3/AC-4 |
| `TASK-423` | `Step 5` | `docs/03_wit_and_manifest.md` §Config Field Types Reference (enum row) | manifest knob, `lib.rs` `from_config` parse block, `docs/15_config_keys_reference.md` | — | S | Knob declaration + module-side rejection (defense-in-depth; the host `ConfigBoundsIndex` already rejects bad enum values first); AC-1/AC-N1 |
| `TASK-424` | `Step 6` | plan §3 Ruling 8 | `lib.rs` propagation loop branch, `agg_raster.rs` glue, `tests/agg_rasterizer_tdd.rs` routing test | instantiation site (canonical `generate_support_layers` region) | M | agg selectable, legacy_semantic DEFAULT since the 2026-09-03 human decision; AC-5 |
| `TASK-424` | `Step 6b` | plan §3 Ruling 8 | `tests/traditional_family_tdd.rs` assertion re-baselining only | — | S | Legacy-guard reconciliation, done while agg was still the default; AC-N2. Split from Step 6 to hold the 3-file edit cap |
| `TASK-425` | `Step 7` | plan §7 E1/E2 | integration measurement tests | `fb7b995050` (already reproduced by legacy; AC-6 guard) / `a95607d7bf` (improvement; AC-7) symptoms (plan-cited); fixture `SupportAdversarial.stl` | M | Measurement gate; AC-6/AC-7/AC-8 |
| `TASK-426` | `Step 8` | plan §13 T7 | wedge proof + measured hint update | — | S | Real-mesh validation |
| `TASK-427` | `Step 9` | plan §8 | `docs/07_implementation_status.md` rows, doc greps | — | S | Closure gates |
| `TASK-428` | `Step 9` | plan §8 | recorded-metrics appendix in requirements.md | — | S | Registration + human-gate readiness. The metrics Step 9 recorded were measured under the since-removed clamp; they are retained only as labelled history and were SUPERSEDED by the Step-14 re-measurement |
| `TASK-424` | `Step 10` | — | `modules/core-modules/traditional-support-planner/src/lib.rs` (clamp removed from the `RasterizerMode::Agg` arm of `SupportPlanner::plan_candidate`) | `seed_fill_block`, `contours_simplified`, `dilate_trimming_region` (delegated) | S | Root-cause probe (recorded): halo proven canonical; H1/H2/H3 refuted; four legacy tests measured red under the agg default |
| `TASK-420` | `Step 11` | — | none (design.md + DEV-166 record only) | `SupportGridPattern` statics (delegated) | S | Canonical probe (recorded): halo deliberate; canonical prints undemanded material; canonical has no decline concept; port faithful where doubted |
| `TASK-423` | `Step 12` | — | manifest default + guest parse default, `docs/15_config_keys_reference.md` | — | S | Default flipped to `legacy_semantic`; owned by a CONCURRENT worker, not by the doc pass |
| `TASK-424` | `Step 13` | — | agg divergence tests | — | S | Divergence pinned as tests (halo present; `NoRoute` does not fire under agg for a LOCAL obstacle, but still fires when occupancy covers the whole grid neighbourhood); owned by the same concurrent worker |
| `TASK-425` | `Step 14` | plan §7 E1/E2 | `docs/spec_packets/241-support-agg-rasterizer/requirements.md` appendix, `docs/spec_packets/241-support-agg-rasterizer/design.md` risk bullet, DEV-166 metrics sentence in `docs/DEVIATION_LOG.md` | — | M | **DONE** — AC-6/AC-7/AC-8 + F-I1 re-measured 2026-09-03 against the unclamped opt-in agg mode; figures in the requirements.md appendix |
| `TASK-428` | `Step 15` | — | packet docs + DEV-166 row (documentation only) | — | S | Documentation honesty pass (recorded): default, AC-5, clamp-rejection, staleness banners |
| `TASK-427` | `Step 16` | plan §8 | closure gates + doc-impact greps | — | S | **DONE** — gates re-run on the post-clamp-removal tree (`cargo check`/`clippy --workspace --all-targets`, `cargo xtask check-literals`, `cargo xtask build-guests --check`: all exit 0) |
| `TASK-426` | `Step 17` | — | `modules/core-modules/traditional-support-planner/src/lib.rs` (`merge_region_identity_entries`) | — | S | Wedge duplicate-region abort UNBLOCKED by a temporary producer-side merge; root cause unfixed (DEV-167) |
| `TASK-428` | `Step 18` | — | packet docs (documentation only) | — | S | Documentation reconciliation after the clamp rejection (recorded) |
| — | `Step 19` | — | none — no work performed | — | — | **NOT DONE, DECLINED** by binding human decision 2026-09-03: producer fix + AC-N2 restoration transferred to packet `241b-support-plan-ownership-seam` |
| `TASK-428` | `Step 20` | `docs/02_ir_schemas.md` § IR 9b | packet docs + `docs/DEVIATION_LOG.md` (documentation only) | — | S | Close-out record: AC-N2 RED (26 passed / 2 failed), Packet Completion Gate NOT MET, `status:` stays `draft`, DEV-167 filed |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate
exceeds M.

**Close-out status (2026-09-03): packet 241 closes NARROW and NOT GREEN.**
`agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`
(`crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`) no longer aborts,
but only because `merge_region_identity_entries`
(`modules/core-modules/traditional-support-planner/src/lib.rs`) is in place as a documented
temporary unblock (DEV-167). **AC-N2 is RED and stays red** —
`cargo test -p traditional-support-planner --test traditional_family_tdd` measures
26 passed / 2 failed (`coarse_same_region_sources_keep_distinct_body_membership`,
`coarse_source_preference_keeps_mixed_source_memberships`). The Packet Completion Gate is NOT
MET and `status:` remains `draft`. Ownership of the real fix is packet
`241b-support-plan-ownership-seam`.
