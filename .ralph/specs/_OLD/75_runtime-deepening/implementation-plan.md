# Implementation Plan — Packet 75

## Execution Rules

- Narrow validation only. Full `cargo test --workspace` runs once, at the Acceptance Ceremony, via a sub-agent
  returning `FACT pass/fail`.
- One commit per phase. After each phase: `cargo build --workspace` → the phase's narrow tests → `cargo clippy
  --workspace --all-targets -- -D warnings` → `cargo xtask build-guests --check` (must report no `STALE:`; no
  phase touches a guest input).
- No behaviour change. A test needing its assertion weakened is a red flag — investigate, don't weaken.
- Execution order: Phase 1 → 2 → 3 → 4 (as listed).

---

### Phase 1 — PrePass stage runner (TASK-216)
- **Read:** `prepass.rs:400–746`; `instrumentation.rs:307–348`; `blackboard.rs:140–161`.
- **Edit:** add `BuiltinStageSpec` + `run_builtin_stage` (owns guard/size/instrument/finish). Convert the six
  inline blocks (`:407–621`) to `run_builtin_stage(spec, …)` calls in place; each spec's `execute` closure keeps
  the stage's current commit (`commit_*`/`replace_slice_ir`) and any extras (rtree, paint_semantic_configs).
  Preserve `early_stages`/fallback/`late_stages` skeleton + `stage_requires_region_map`.
- **Regression test:** instrumentation spy (`tests/executor/`) asserting one `on_stage_end` per built-in in
  declared order.
- **ADR-0001** under `docs/adr/` (create dir).
- **Verify / exit:** AC-1.1, AC-1.2, AC-1.3; clippy + build-guests --check clean. Commit
  `refactor(prepass): unify host-built-in stage bracket behind run_builtin_stage (TASK-216)`.

### Phase 2 — Pure IR harvest extraction (TASK-217)
- **Read:** `dispatch.rs:234–305, 1658–2113`; `wit_host.rs:2512`.
- **Edit:** add `harvest_*_from(proposals)` cores (move bodies); reduce wrappers to `harvest_*_from(ctx.<field>)`.
  Make `wit_host::parse_canonical_region_id` `pub(crate)`; delete `dispatch.rs:1658` copy; repoint 3 call sites.
  Relocate `collect_postpass_output` beside the cores. Add inline `_from` unit tests over synthetic vectors.
- **Verify / exit:** AC-2.1, AC-2.2, AC-2.3; clippy clean. Commit
  `refactor(dispatch): extract pure harvest cores; dedup region-id parser (TASK-217)`.

### Phase 3 — WIT marshalling `with:` unification (TASK-218)
- **Read:** `wit_host.rs:236–520` (bindgen blocks), `1461–1900` (converters/host-services), `2526/3477/3916/4699`
  (HostConfigView), the per-world converter clusters (`3314–4063, 4536–4985`).
- **Edit:** ensure `pub mod layer` first; add `with:` geometry (+config) remaps to prepass/finalization/postpass.
  Delete redundant converters (`p_/f_/pp_wit_to_ir`, `*_ir_to_wit`, `ir_point3_to_*`, `ir_bounds_to_*`,
  finalization/postpass role/path/retract converters) and 3 of 4 `HostConfigView` impls; point remaining
  host-services bodies at the shared converters + existing `ir_clip_polygons`/`ir_offset_polygons`/
  `ir_simplify_polygon`/`*_mesh_query` helpers. **Fallback:** geometry-only if config fights bindgen; macro ×4 if
  `with:` fails outright (don't ship both).
- **ADR-0002** under `docs/adr/`.
- **Verify / exit:** AC-3.1, AC-3.2, AC-3.3 (build + clippy = type identity; build-guests --check = ABI stable);
  contract bucket green. Commit `refactor(wit_host): unify cross-world marshalling via bindgen with: remap (TASK-218)`.

### Phase 4 — Model intake assembly seam (TASK-219)
- **Read:** `model_loader.rs:145–215, 345–449, 1972–2115`; `helpers_cmd.rs:360–461, 512–528`;
  `slice_ir.rs:401–428`.
- **Edit:** add `assemble_object`/`assemble_objects`; route all five wrap sites through them; delete
  `compute_z_extent_for_component`; collapse identity-transform helpers; make the four pure 3MF helpers
  `pub(crate)`. Add file-free helper tests + the single-component z-extent equivalence regression test. Sharpen
  CONTEXT.md **Split to objects** (concept-level, drop "in the GUI" exclusivity).
- **Verify / exit:** AC-4.1, AC-4.2, AC-4.3, AC-4.4. Commit
  `refactor(model_loader): single ObjectMesh assembly seam; dedup z-extent (TASK-219)`.

### Acceptance Ceremony (packet close)
- All four phases' narrow gates green. Dispatch full `cargo test --workspace` to a sub-agent → `FACT pass/fail`.
- E2E: `pnp_cli slice` on an STL and a 3MF fixture; byte-compare reference `.gcode`.
- `cargo xtask build-guests --check` clean. Flip `packet.spec.md` `status: implemented`.
