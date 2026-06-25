# Requirements: 125_voronoi-oom-hardening (rescoped)

## Packet Metadata

- Grouped task IDs: **none** (bug-fix from the 2026-06-24 diagnose session).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M`
- **Rescope note:** WI-1 falsified the original Voronoi-OOM hypothesis; this packet now targets the
  confirmed `region_id`↔tool-index conflation. Slug retained (committed dir); see `packet.spec.md`.

## Problem Statement

Slicing painted models (`cube_fuzzyPainted.3mf`, and the same path on `cube_4color`) aborts with an
uncatchable ~9.9 GiB OOM in the G-code emitter. WI-1 traced and code-confirmed the full chain (the
crash is **pre-existing in committed code** — bisect: it persists with the uncommitted WIP stashed and
guests rebuilt, so the Stage-0 gate / `slice_has_paint` plumbing is innocent):

1. **Root (by design):** `region_id` is a 64-bit *paint-variant identity*, derived by
   `paint_variant_region_id` (`paint_segmentation/mod.rs:169-178`,
   `base.saturating_mul(STRIDE).saturating_add(paint_variant_hash(...))`). It is intentionally large and
   non-sequential — a region identity, never a tool index. The captured value was
   `0x3E8281949ECA9508`, constant across layers.
2. **Conflation:** `layer_executor.rs` resolves the tool as
   `paint_tool.or(spatial_tool).or(variant_tool).or(modifier_tool).unwrap_or(region.region_id)` (walls,
   ~:739-743) and `spatial_tool.or(variant_tool).unwrap_or(region.region_id)` (paths, ~:773). When all
   resolvers return `None` — which they wrongly do for a *painted* entity, because
   `paint_tool = dominant_tool_index(&wl.feature_flags)` (~:727) yields `None` — the fallback stores the
   `region_id` **identity** into `RegionKey.region_id` (the **tool slot**).
3. **OOM:** `slicer-gcode/src/emit.rs` reads `region_key.region_id as u32` as the tool index (~:268 and
   ~10 other sites) → `max_tool = filament_per_tool.keys().max()` = `0x9ECA9508` = **2,664,076,552**
   (the low 32 bits of the identity) → `vec![0.0f32; max_tool + 1]` (~:637-638) = 2,664,076,553 × 4 B =
   **9.924 GiB** → tripwire/OOM.

Fixing it at the **source** (a valid tool in the slot) repairs every downstream `region_id as u32`
emit site at once. This packet does the correct parity fix (A: paint→tool resolution; B: safe fallback)
plus an emit-side bound-check guard, and keeps the tripwire + the non-vacuous fuzzy test.

## In Scope

- **(B) Safe fallback:** replace both `.unwrap_or(region.region_id)` fallbacks in `layer_executor.rs`
  (walls ~:743, paths ~:773) with a bounded valid default (tool `0`) so a `region_id` identity can never
  enter the tool slot. (Crash-stop + correctness floor.)
- **(A) Correct paint→tool resolution:** make `dominant_tool_index(&wl.feature_flags)` resolve for
  painted entities — trace why `feature_flags` lack the painted tool and populate it so painted regions
  carry their real tool index (the fallback then never fires for painted geometry). (Parity.)
- **(Guard) Emit bound-check:** in `slicer-gcode/src/emit.rs`, validate/clamp the tool id before sizing
  the dense per-tool `Vec` (reject an out-of-range id with a typed error). Defense-in-depth.
- **Keep:** the WI-1 guarded `>1 GiB` global allocator in the executor bucket (permanent tripwire); the
  non-vacuous `cube_fuzzy_painted_face_jitter` hardening.
- **Cleanup:** remove the temporary WI-1 diagnostic dumps in `emit.rs` (kept for fix-validation).

## Out of Scope

- Renaming / un-overloading the `region_id`↔tool field (latent design smell; would touch all `as u32`
  emit sites + `RegionKey` — separate refactor).
- The separate, catchable `fpv.is_finite()` boostvoronoi panics surfaced on the painted path.
- The `boostvoronoi::discretize` unbounded-loop cap (real latent bug, not this crash) — separate
  optional hardening follow-up.
- Anything Voronoi / paint-seg robustification / sort-pass (the dropped original WIs).
- Re-enabling painted gap-fill (independent of this OOM; revisit separately).
- `cpp_map` determinism / byte-exact gcode (deferred, gap #5).

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PaintValue`, region/`RegionKey` shape (delegate a field-name FACT).
- `CLAUDE.md` §"Test Discipline", §"Config Key Naming" — small, load directly.
- `OOM_FINDINGS.md` + WI-1 capture — confirmed-chain evidence; read first.

(No OrcaSlicer Reference Obligations: a host-side tool-resolution bug fix; the "region_id == tool index"
convention is internal. Parity is verified against the model's own painted tool set, not OrcaSlicer source.)

## Acceptance Summary

- Positive: `AC-1`..`AC-6` in `packet.spec.md`. Refinements: AC-1 asserts the *exact* fallback value is
  `0` and that no entity's tool slot equals a known `paint_variant_region_id` output; AC-3 compares the
  emitted `T<n>` set to the model's painted tool set (equality, each `< extruder count`).
- Negative: `AC-N1` (emit bound-check rejects an out-of-range tool id; no >1 GiB alloc).
- Cross-packet impact: none (the Stage-0 gate WIP is innocent and untouched by this fix).

## Verification Commands

| Command | Purpose | Return hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test integration -- tool_fallback_never_leaks_region_identity` | AC-1 safe fallback | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- painted_entity_resolves_real_tool` | AC-2 paint→tool (parity) | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_paint` | AC-3 + AC-6 tool set + no regression | FACT pass/fail (12/12) |
| `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_face_jitter` | AC-4 non-vacuous, no OOM | FACT pass/fail + which assertion |
| `cargo test -p slicer-runtime --test executor -- mmu_no_oversized_alloc_repeat` | AC-5 tripwire green ×10 | FACT pass/fail |
| `cargo test -p slicer-gcode -- emit_rejects_out_of_range_tool_id` | AC-N1 emit guard | FACT pass/fail |
| `rg -q "global_allocator" crates/slicer-runtime/tests/executor/` | tripwire retained | FACT hit |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

No AC uses `cargo test --workspace` — reserve it for the closure ceremony as a single FACT dispatch.

## Step Completion Expectations

- WI-1 is already complete (diagnosis + tripwire in tree); WI-6 must NOT remove the allocator, only the
  temporary `emit.rs` dumps.
- WI-2 (safe fallback) alone stops the OOM; WI-3 (paint→tool) is required for AC-2/AC-3 parity — a green
  AC-4 with a still-broken `paint_tool` would mean painted entities silently use tool 0 (regression), so
  AC-2/AC-3 gate that WI-3 actually landed.
- No step may regress `cube_4color_paint` (AC-6).

## Context Discipline Notes

- `layer_executor.rs` and `slicer-gcode/src/emit.rs` are large — locate by symbol (`dominant_tool_index`,
  `unwrap_or(region.region_id)`, `region_key.region_id as u32`, `filament_per_tool.keys().max()`); read
  ±40 lines. Line numbers drift (the tree has the WI-1 dumps + WIP).
- `paint_segmentation/mod.rs:169-178` is read-only context (region_id derivation) — do not change it;
  `region_id`-as-paint-identity is by design.
- Heaviest dispatch: the emit/tool trace — return `LOCATIONS` of the `region_key.region_id as u32` sites,
  not file bodies.
