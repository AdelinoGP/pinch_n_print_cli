# HANDOFF — region_id↔tool conflation: pipeline-wide tool/identity split

> **SUPERSEDED (2026-06-25).** The split this document hands off was implemented directly under packet
> `125_voronoi-oom-hardening` (not a successor packet). See the rewritten `packet.spec.md` /
> `requirements.md` / `design.md` / `implementation-plan.md` / `task-map.md` in this directory for the
> shipped, verifiable contract. Some details below were corrected during implementation (notably:
> `print-entity-view` **did** receive `tool-index` for the finalization input deep-copy — §1.2 fact #2
> here is stale; `RegionMapIR.tool_configs` was not added — per-tool config composes at emit and at
> the RegionMapping overlay instead). This file is retained for historical diagnosis context only.

**Origin:** packet `125_voronoi-oom-hardening` (status: `active`, held). Packet 125 stopped the
painted-model OOM (crash-stop floor + emit guard + restored tripwire) and restored painted fuzzy
skin, but **3 executor-bucket tests are intentionally left red** because the true fix is a
pipeline-wide schema change, not a bug-fix detail. This document hands that fix to a dedicated
follow-up packet (suggested slug: `126_region-id-tool-identity-split`).

**Commit with the red state:** `fecf2435` (`fix(parity): bound painted-model region_id↔tool OOM; defer pipeline-wide tool/identity split`).

**Golden rule for the successor:** do NOT revert Step 2's `DEFAULT_TOOL(0)` fallback to green the
two identity tests — that re-exposes the 9.9 GiB OOM. Do NOT edit the red tests to accept `0` —
that masks a real invariant. The only correct fix is to give the entity its own tool field.

---

## 1. DIAGNOSE

### 1.1 Root cause (code-confirmed, not inferred)
`region_id` is **dual-purpose** and the pipeline conflates the two uses:

- **As a region IDENTITY** (back-reference): `paint_segmentation/mod.rs:169-178`
  (`paint_variant_region_id`) derives `region_id` as a 64-bit hash of the `PaintValue`
  (scalar → `f64::to_bits`, `mod.rs:139`). Captured crash value: `region_id =
  0x3E8281949ECA9508` (= f64 ≈ 1.38e-7). Host postpasses key on `(object_id, region_id)`:
  `layer_executor.rs:1017` (`backfill_resolved_seam`), `:1169/:1187` (`SlicePostProcess` pairing),
  `:1082` (`PerimetersPostProcess` merge).
- **As a TOOL INDEX:** `layer_executor.rs` stored `resolved_tool` INTO `RegionKey.region_id`
  (`:747/:777`), and BOTH consumers read it back as the tool:
  - host: `emit.rs:268` `required_tool = first_entity.region_key.region_id as u32` (+ ~10 sibling
    `region_key.region_id as u32` sites); `emit.rs:616-617` `max_tool = filament_per_tool.keys().max()`
    then `vec![0.0f32; (max_tool + 1)]`.
  - **guest:** `modules/core-modules/path-optimization-default/src/lib.rs:114-115`
    `fn tool_index_of(entity) -> u32 { entity.region_key.region_id as u32 }` — used at `:192/:224`
    to emit `from_tool`/`to_tool` tool changes (`:315 push_tool_change`). Comment at `:112`:
    *"Tool index is propagated through region_key.region_id at assembly time."*

The OOM: a painted (fuzzy) region's `region_id` IDENTITY (~2.66e9 after `as u32`) leaked into the
tool slot via `.unwrap_or(region.region_id)` → `emit.rs` sized a ~9.9 GiB dense vector.

### 1.2 What packet 125 already landed (do not redo)
- **Crash-stop floor (Step 2):** `layer_executor.rs` both fallbacks → `.unwrap_or(DEFAULT_TOOL)`
  (`DEFAULT_TOOL: u64 = 0`). A `region_id` identity can no longer reach the tool slot. AC-1 green.
- **Emit guard (Step 4):** `emit.rs` rejects `max_tool >= MAX_PLAUSIBLE_TOOLS (1024)` with typed
  `GCodeEmitError::ToolIndexOutOfRange { tool, max }` (`error.rs`) BEFORE the dense vec — defense in
  depth even if a future leak reappears. `postpass.rs` has the matching translation arm. AC-N1 green.
- **Tripwire (Step 1/6):** guarded `>1 GiB` SINGLE-allocation `#[global_allocator]` in
  `crates/slicer-runtime/tests/executor/main.rs:60` (re-entrancy guard; no cumulative backstop to
  avoid false trips). Permanent. `mmu_no_oversized_alloc_repeat` (10×) green (AC-5).
- **Painted fuzzy skin (Step 3 — but see §1.3 D14 caveat):** `paint_segmentation/mod.rs:737/748`
  now synthesizes `segment_annotations` for painted variant chains so the guest applies jitter.
  cube_fuzzyPainted painted-face points: 0 → 221.

### 1.3 The 3 red tests (exact state)
1. **`layer_executor_tdd::ordered_entities_assembled_with_preserved_region_identity`**
   (asserts `region_key.region_id` == source identity 1 and 2; `layer_executor_tdd.rs:886-897`).
   FAIL: region_ids all 0. Cause: Step 2's `.unwrap_or(0)` zeroes the identity for tool-less
   geometry. INVARIANT (real back-ref consumers depend on it; see §1.1).
2. **`layer_world_deep_copy_tdd::layer_world_builder_commit_preserves_entities_tool_changes_and_z_hops`**
   (asserts entity `region_id` == 11/22; `layer_world_deep_copy_tdd.rs:287`). FAIL: left=0.
   Cause: cascade of #1 (fixture has no feature_flags/SliceIR → fallback → 0). Same invariant.
3. **`paint_channel_consumer_paths_tdd::paint_channel_fuzzy_skin_strokes_reach_fuzzy_variant_chain`**
   (asserts `!has_fuzzy_in_segment_annotations` AND `has_fuzzy_variant_chain`;
   `paint_channel_consumer_paths_tdd.rs:438-453`). FAIL on (a): Step 3 put FuzzySkin into
   `segment_annotations`, but **D14** (`closure-log.md:121`) reserves `segment_annotations` for
   modifier-volumes only. The `variant_chain` already carries `("fuzzy_skin", Flag(true))`
   (`mod.rs:746`). This is a CONTAINED guest fix (see §3.2), separable from the identity split.

### 1.4 Bisect / provenance (so the successor doesn't re-pivot)
- Bisect verdict: the OOM is **pre-existing committed** behavior, not the uncommitted WIP (WIP
  stashed → OOM persisted with identical value). The original "discretize OOM" hypothesis and the
  paint-seg/sort-pass/discretize-cap work items were **dropped** (discretize has a real *latent*
  unbounded loop per `OOM_FINDINGS.md`, but it is NOT this crash — log as a separate optional
  hardening follow-up, do not pursue here).
- The crash site was pinned through 4 hypotheses (build → discretize → emit → region_id source);
  every hop is now code-evidenced. **Do not re-pin by inference — the chain in §1.1 is closed.**

---

## 2. VERIFY (reproduce + confirm behavior)

### 2.1 Reproduce the 3 red tests
```
cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log
# expect: 164 passed / 3 failed (the three in §1.3). Allocator must NOT false-trip (no exit 173).
# narrow:
cargo test -p slicer-runtime --test executor ordered_entities_assembled_with_preserved_region_identity
cargo test -p slicer-runtime --test executor layer_world_builder_commit_preserves_entities_tool_changes_and_z_hops
cargo test -p slicer-runtime --test executor paint_channel_fuzzy_skin_strokes_reach_fuzzy_variant_chain
```

### 2.2 Confirm the crash-stop floor + slice behavior (clamped tool)
```
# OOM is gone, witnessed by the tripwire (must NOT exit 173); painted fuzzy face has jitter:
cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter -- --nocapture
#   painted_face_pts ~= 221 (RIGHT/+X fuzzy face) > 2 * unpainted (~12, LEFT/-X bare).
# emit guard rejects a synthetic out-of-range tool id (no >1 GiB alloc):
cargo test -p slicer-gcode -- emit_rejects_out_of_range_tool_id
# 10x repeat under the tripwire:
cargo test -p slicer-runtime --test executor -- mmu_no_oversized_alloc_repeat
```
Current (clamped) behavior to PRESERVE after the split: cube_fuzzyPainted is **single-tool**;
painted geometry correctly emits tool 0; the fuzzy paint is a FuzzySkin semantic (no material tool).
The split must keep this (tool 0 for fuzzy) AND restore region_id identity for the back-refs.

### 2.3 Gates that must stay green throughout
```
cargo clippy --workspace --all-targets -- -D warnings      # currently CLEAN
cargo xtask build-guests --check                           # currently CLEAN
```
Windows note: a crashed executor test locks `target/debug/deps/executor-*.exe` →
`taskkill //F //IM executor-*.exe`. Always `tee` to `target/test-output.log` and read the file.

---

## 3. PLAN (contract/schema changes for the follow-up packet)

### 3.1 Primary: separate the resolved tool from the region identity (fixes red tests #1, #2)
Add a first-class tool field so `region_id` is a PURE identity and the tool is explicit.

| # | Change | File | Notes |
|---|--------|------|-------|
| P1 | Add `tool_index: u32` to `PrintEntity` | `crates/slicer-ir/src/slice_ir.rs:1913` | IR struct; **no `Default` derive** → every construction site must set it (compiler-guided). docs/02 + schema (minor) bump. |
| P2 | Add `tool-index: u32` to the WIT entity view | `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit:23-29` (`print-entity-view`) | **Confirm this is the same record the guest's `OrderedEntityView` resolves to** (path-optimization world). bindgen regen + guest rebuild. docs/03 (WIT) impact. |
| P3 | Project `tool_index` into the WIT view at the host bridge | wherever host builds `print-entity-view` from `PrintEntity` (grep the bindgen `impl`/conversion) | keep region-key.region_id = identity. |
| P4 | Guest reads the tool from the new field | `modules/core-modules/path-optimization-default/src/lib.rs:114-115` | `tool_index_of` → `entity.tool_index` (not `region_key.region_id`). Guest rebuild + bindgen. |
| P5 | Host emit reads the new field | `crates/slicer-gcode/src/emit.rs` (~10 `region_key.region_id as u32` sites incl. `:268`, the `max_tool` path `:616-617`) | switch all tool reads to `entity.tool_index`. Keep the `MAX_PLAUSIBLE_TOOLS` guard as belt-and-suspenders. |
| P6 | Assembly sets the tool, restores identity | `crates/slicer-runtime/src/layer_executor.rs:739-743`, `:773`, `:747/:777` | `tool_index = resolved_tool` (keep fix A so it's the REAL painted tool, falling back to `DEFAULT_TOOL=0`); `region_key.region_id = region.region_id` (pure identity again — un-overwrite). |
| P7 | Fix all `PrintEntity` construction sites | compiler-guided (tests/benches too) | `slicer-runtime/benches/shell_classification.rs`, the executor/unit test fixtures that build `PrintEntity`, etc. |

After P1–P7 the two identity tests pass (region_id is the identity) AND tool changes / emit stay
correct (tool comes from `tool_index`). Do NOT change the test assertions.

### 3.2 Secondary (separable, contained): D14 FuzzySkin routing (fixes red test #3)
- Revert packet 125's `paint_segmentation/mod.rs:737/748` `segment_annotations` synthesis back to
  `HashMap::new()` (keep `segment_annotations` modifier-volume-only per D14).
- Teach `build_wall_flags` (`crates/slicer-core/src/perimeter_utils.rs:61`, fuzzy read at `:97-103`
  / `:148-165`) to set `flag.fuzzy_skin = true` when the region's `variant_chain` contains
  `("fuzzy_skin", Flag(true))` (already present at `mod.rs:746`). Guest dep → rebuild.
- Re-verify cube_fuzzyPainted still gets jitter (painted_face_pts ≫ unpainted) AND
  `paint_channel_fuzzy_skin_strokes_reach_fuzzy_variant_chain` passes (no fuzzy in
  segment_annotations). This can be its own small packet OR a phase-0 of the split packet.

### 3.3 Doc / contract impact (this is NOT `none`)
- `docs/02_ir_schemas.md`: document `PrintEntity.tool_index`; bump the IR schema version per its
  versioning rules.
- `docs/03_wit_and_manifest.md`: document the `print-entity-view.tool-index` field.
- Record the deviation: packet 125 scoped the field-separation OUT as "a separate refactor"; the
  full-bucket acceptance ceremony **falsified that scoping** (the conflation cannot be fixed in one
  field — every single-field option breaks a consumer). The split is in-scope-now because the
  bucket proved it necessary. Carry this rationale into the new packet's `design.md` Code Change
  Surface + deviation log.

---

## 4. IMPLEMENT (directives for clean execution)

1. **Stand it up as its own packet** (`spec-packet-generator`). It is a WIT/IR contract + guest +
   bindgen schema change with a real blast radius — not a continuation of the thrice-pivoted
   125 bug-fix. Status `draft` → `active` only after the spec is implementation-grade.
2. **First sub-step = blast-radius confirmation** (the §3.1-P2 gate): confirm the exact WIT record
   the guest's `OrderedEntityView` binds to and that adding `tool-index` there is the minimal
   surface. If multiple worlds expose the entity view, enumerate every guest that reads
   `region_key.region_id` as a tool (grep `region_id as u32` / `tool_index_of` across `modules/`)
   so none is missed.
3. **Sequence to keep the build green between steps** (suggested): (a) add `tool_index` to IR +
   WIT + bridge, defaulting it to the current `region_id`-derived value so behavior is unchanged
   and the build stays green; (b) flip emit + guest reads to `tool_index`; (c) flip assembly to set
   `tool_index = resolved_tool` and restore `region_id = identity`; (d) do §3.2 (D14). Run the
   FULL executor bucket after EACH sub-step, not just at the end — that is exactly where packet 125
   looked green on its 7 ACs but was red bucket-wide.
4. **Risk isolation:** the IR/WIT change is mechanical but wide. Use the missing `Default` on
   `PrintEntity` as the compiler's checklist for construction sites. After each guest-affecting
   edit: `cargo xtask build-guests --check` (rebuild if `STALE:`) before running guest/executor
   tests. Keep the emit `MAX_PLAUSIBLE_TOOLS` guard and the `>1 GiB` tripwire — they are the safety
   net that proves no regression re-opens the OOM.
5. **Acceptance gate (hard):** the FULL `cargo test -p slicer-runtime --test executor` bucket green
   (all of: the 3 currently-red tests + the 7 packet-125 ACs stay green), `cargo clippy --workspace
   --all-targets -- -D warnings` clean, `cargo xtask build-guests --check` clean. Optionally
   `cargo test --workspace` once at closure as a single FACT dispatch. Do NOT declare done on a
   subset of tests — the subset-green/bucket-red gap is the specific failure mode that produced
   this handoff.
6. **Do not** revert `DEFAULT_TOOL`, do not edit the red tests to pass, do not fold in the latent
   `boostvoronoi::discretize` cap (separate optional item) or the `fpv.is_finite()` painted-path
   panics (caught by existing `catch_unwind`; separate follow-up).

---

## Appendix — key references
- Captured value: `region_id = 0x3E8281949ECA9508` (4504305052643136776); `as u32 = 2,664,076,552`
  = `max_tool`; `vec![0.0f32; 2,664,076,553]` = 9.924 GiB.
- `OOM_FINDINGS.md` (boostvoronoi standalone investigation) — authoritative on the *latent*
  discretize loop; NOT this crash. The misnamed `discretize_degenerate_args.txt` fixture was
  removed (its analysis applies to the emit path, captured here).
- Packet 125 docs (`packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`) hold
  the rescoped acceptance criteria; this handoff supersedes them for the deferred split.
