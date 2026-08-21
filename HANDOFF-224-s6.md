# HANDOFF-224-s6 — Packet 224 remediation session 6 (2026-08-21)

Continues `HANDOFF-224-s5.md`. **24 commits landed, `e85e546a..c5fe29c4`**, on
`parity/support-planners`. Working tree clean apart from the pre-existing
`HANDOFF-224*.md` dirt (untouched, per s5 rule 7).

---

## 0. THE FINDING THAT REFRAMES EVERYTHING

**OrcaSlicer source IS on this machine**, at `OrcaSlicerDocumented/src/libslic3r/`.
It is **gitignored**, so `git ls-files` returns nothing and a shallow `find` misses it.
Early in this session the coordinator searched too shallowly, concluded no checkout
existed, and **told five workers so**.

Three defects trace directly to that false premise. Each was produced by careful
reasoning from prose instead of source:

1. **`smooth_nodes` (F-33)** — shipped with a `(1,2,1)/4` kernel and 3 relaxation
   iterations. Canonical is `/3` unweighted and **100** iterations. Its author built a
   convexity argument to justify the wrong weights — answering a question canonical
   never poses. **9 of 13 audited claims diverged**, including an ellipse matrix applied
   unconditionally where canonical guards it, wrongly elongating axis-aligned branches.
2. **`smooth_outward` (F-37)** — ported as an arc-fillet resampler with an invented
   `segments` parameter. Canonical **clips** narrow corners on a mutable linked list. It
   cut sqrt(2)x too deep (chord misread as setback) and fired at ~2.9 deg instead of
   >45 deg. Its "strict superset" contract was false — canonical can delete a ring.
3. **Dual contact seeding** — `a40f971a` deliberately withheld canonical's
   `layer_nr - 1` shift from analysis contacts, on a theory formed without the source.
   Result: two roof bands offset by one layer, breaking AC-4's interface counts.

**Rule for the next session: read `OrcaSlicerDocumented/` directly. Never accept
"no checkout exists" without listing that directory.**

---

## 1. SECOND STRUCTURAL FINDING — G-23 invalidates the tree suite's green

`benchy_tree_support_regression_tripwire` runs with an empty `SupportGeometryView`, so
`TreeVolumes` builds `layer_outlines` from nothing: **collision and avoidance are empty
at every layer.** Its `tree_analysis()` also uses `..Default::default()`, leaving
`model_occupancy` empty, so the exact-Z fallback gate is inert too.

**Consequence: a 76/0 `tree-support-planner` suite coexisted with a planner that produced
no usable support on real meshes.** Every collision-consuming path — the move pass's
outward projection, STUDIO-4252 retry, `valid` clearing, `to_buildplate` recompute, the
avoidance ladder, the `draw_circles` carve — was untested. G-23 hid **three** separate
defects found this session.

**Do not treat the tree crate's green as evidence about collision behaviour.** Fixing
G-23 is the highest-value follow-up in the support area.

---

## 2. THIRD STRUCTURAL FINDING — the native/wasm request seam

The wasm leg builds its layer view via `dispatch_layer_call` plus the guest shim; the
native leg builds its own in `build_native_layer_request`
(`crates/slicer-wasm-host/src/marshal/native.rs`). **Every time an input is added to one
path and not the other, native silently renders nothing** — no error, no diagnostic, just
absent geometry. Hit THREE times:

- `85f1f889` threaded the support plan into the native OUTPUT but not the INPUT (fixed
  here, `c237b046`) — any natively-dispatched support module rendered no support.
- `ddf9dffe` — a module forgetting `begin_region` is not merely untagged; wasm recovers an
  origin via `HostExecutionContext::touch_slice_region`, native has no fallback, so the two
  legs diverge in identity.
- The same asymmetry governs `with_slice_ir`, attached on the support stage only.

**Recommended follow-up: make the two request builders share one construction path, or add
a test asserting they produce equivalent views for every stage.**

---

## 3. Defects fixed (24 commits)

**Tree re-port completed** (steps 6 and 7 of 7): `43b1e8bb` smooth_nodes plus ellipse
matrix; `936786db` F-3 carve keeps the remainder; `3319ad35` nine canonical corrections
to step 6.

**Real-geometry defects** (none in the audit; all found by meeting real meshes):

- `c3c1ed5a` — emit gates rejected a node on ANY overlap with occupancy. Canonical
  *expects* nodes inside collision (`move_out_expolys` reverts, the carve handles it).
  Per-stage counts proved contacts/propagation/move/prune were healthy and EMIT destroyed
  the column (68 of 72 nodes rejected at layer 99). Also fixed dual contact seeding.
- `c4f67120` — the carve differenced against RAW `model_occupancy` (zero inflation);
  940 of 19856 vertices sat within 0.05 mm of the wall, some inside. Now inflated by
  `support_object_xy_distance`; min clearance exactly 0.3500 mm.
- `ac43eab9` — **not a support bug**: `execute_paint_segmentation` built BASE regions from
  whole-layer, all-objects contours, so **every object received every other object's
  cross-section and every toolpath was emitted once per object** (one XY appeared 240x in
  an 80-layer slice). Single-object layers take the original path bit-for-bit; zero
  goldens moved.
- `de6d53b6` — cross-family rejection was scoped by neither object nor layer, so two
  distinct objects choosing different families annihilated each other's support.
- `c237b046` — native layer request never attached the support plan (see section 2).

**Also**: `e18eb96a` F-7/F-36/F-49/F-38; `d3d289cf` smooth_outward re-port; `050d5c3a`
F-37 regularization wiring; `63a968a9` F-44; `ddf9dffe` ironing begin_region;
`71900b22` 1201 attribution; `12a47388` / `770b048d` / `977fefb0` docs, deviations,
F-32/46/47/48.

---

## 4. Test-discipline lessons, measured

1. **`cargo test --workspace` is FAIL-FAST, and it fooled this session TWICE.** A run
   reported "258 binaries / 2112 passed" and looked complete; it had skipped the entire
   `e2e` binary (128 tests) because `contract` failed first. A later run showed
   "259 binaries / 2242 tests" and the coordinator treated THAT as complete too — it was
   also truncated. The true complete figure, measured with `--no-fail-fast`, is
   **386 result lines (345 test binaries + 41 doc-test suites), 3806 passed**.
   **Never report a workspace total that was not produced with `--no-fail-fast`.**
   Two of the four remaining failures had never been executed by ANY packet-224 run.
2. **The `contract` binary was covered by none of this session's narrow commands**
   (`--lib`, `--test integration -- support`). It held a real production regression.
   Narrow greens do not compose into a workspace green.
3. **Golden reblessing must be classified, never assumed.** The legacy golden's drift was
   diffed first: exactly one line (`support_type = Traditional` to `normal(auto)`,
   attributable to `0d0c8d4a`), **zero toolpath bytes**. That also independently
   corroborated that `ac43eab9` is bit-for-bit inert on single-object models.
4. **The tree golden's "40 duplicate endpoints" gate needed the right metric.** Raw repeat
   rows went 40 to 70, which looks like failure; zero-length segments went **40 to 0**,
   and the 70 repeats are exactly the shared vertices of 7 layers x 11 spanning-tree
   edges. The old golden repeated OFF-GRID points at multiplicity 4; the new one is
   on-grid at multiplicity 2.

---

## 5. State at handoff

- Full suite, COMPLETE measurement (`--no-fail-fast`): **3806 passed / 4 failed / 7
  ignored across 386 result lines** (345 test binaries + 41 doc-test suites).
  **THE WORKSPACE IS NOT GREEN.** The 4 failures are listed in section 8.
- Gates green: `clippy --workspace --all-targets -D warnings`, `check-literals`,
  `check-deviations --check` (was RED before this session), `gen-config-docs --check`,
  `build-guests --check`.
- Deviations recorded: **DEV-135..DEV-140** (six; a seventh deliberately excluded as not a
  divergence in intended behaviour).

### Review artifacts, in `target/review-224/`

Orca-matched config, release `pnp_cli`, `--module-dir modules/core-modules`:

- `SupportTest_normal.gcode` — 122 `;TYPE:Support` blocks, 2 interface, 1391 support
  extrusion moves, 150 layers, degraded=false, 0 errors
- `SupportTest_tree.gcode` — 120 support blocks, 2 interface, 6526 support extrusion
  moves, 150 layers, degraded=false, 0 errors
- `report-normal.html`, `report-tree.html`, `config-normal.json`, `config-tree.json`

Orca reference comparison (context only; no test reads Orca, per the AC-6 gate):
Normal 121 support / **3** interface; Tree 122 support / 2 interface. The Normal 2-vs-3
difference is the already-registered **G-18**. Layer counts differ (150 vs 452) because
the references were cut at a finer layer height.

---

## 6. Packet status — still `draft`, deliberately

The workspace is not green (section 8) AND the definition of done includes **"generated G-code from SupportTest.stl for both Tree and
Traditional passes human review"**. That review has not happened. Every other element is
met. **Do not mark the packet `implemented` until a human inspects the artifacts above.**
`TASK-335` stays unchecked in `docs/07_implementation_status.md`.

## 7. Open work, explicitly not done

- **F-37 piece 2** — base-interface (`num_top_base_interface_layers`) role. Blast radius is
  NOT contained: `SupportPlanRole` is mirrored in WIT twice, needs
  `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` bumped, a paired `SupportRole` variant, match
  arms in slicer-macros / both marshal sites / dispatch / both planners / both renderers,
  and a new `ExtrusionRole` plus `;TYPE:` decision. ~10 files, 2 WIT edits, 1 schema bump.
  Canonical derivation recorded in `050d5c3a`.
- **G-23** — tripwire exercises neither collision nor avoidance (section 1).
- **The native/wasm request seam** (section 2).
- `interface_regularize.rs` is byte-identical in both renderers — consolidate when next
  touched.
- STUDIO-4252 retry passes `radius_sample_resolution + EPSILON` where canonical passes
  `max_move_between_samples` for both args (escape direction only).
- `execute_paint_segmentation`'s `matching_base.is_empty()` fallback still clones
  whole-layer contours; unreachable with a well-formed region_map.
- Routed out by earlier human decision, unchanged: F-8/9/10/17/30/39/40/42-part/43, F-29.

---

## 8. THE WORKSPACE IS NOT GREEN — 4 failures outstanding

Measured with `cargo xtask test --summary --workspace --no-fail-fast`. Two of these had
never been executed by any prior packet-224 run, because fail-fast aborted first.

1. `prepass_support_geometry_layer_plan_tdd::planner_emits_one_entry_per_region_in_region_map`
   (`crates/slicer-runtime/tests/executor/`) — expected 2 entries for
   (layer=5, object=obj-multi), **got 0**.
2. `prepass_support_geometry_layer_plan_tdd::planner_walks_real_layer_plan_with_variable_layer_heights`
   — highest entry first point z=0.8, expected ~2.0.
3. `fixture_invariants` (`crates/slicer-runtime/tests/integration/main.rs`) — **packet
   AC-1**. "tree: no plate-terminated entry".
4. `interface_is_topmost_and_carved_out` — layer 118 carries interface geometry but no
   SupportBody. **May be red-by-design**: `df6b75cd` says six gates "are RED on purpose
   and stay red until the defects they now bind on are fixed" and names this one (F-4).
   Needs a verdict, not an assumption.

**Prime suspect for (1) and (3):** commit `c3c1ed5a` gated the legacy mesh-facet contact
path on "this object has no admissible tree analysis candidates". Consolidating to one
contact source is canonically right, but that gate may starve cases the analysis path does
not cover — (1) is a multi-region fixture getting zero entries, (3) reports no
plate-terminated tree entry at all. Fix without reintroducing dual seeding; `final_gcode_roles`
(AC-4, interface counts 1/2/3) must stay green.

Gates remain green: clippy, check-literals, check-deviations, gen-config-docs,
build-guests.
