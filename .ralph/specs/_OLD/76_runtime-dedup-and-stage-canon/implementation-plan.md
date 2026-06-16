# Packet 76 — Implementation plan

Order chosen to land lowest-risk first; macro (Candidate 2) last because it
would require a full guest rebuild (and is now deferred — see TASK-222).

Each step lists explicit pre/postcondition, exit condition, files-to-read,
files-to-edit, expected sub-agent dispatches, and context-cost estimate per the
spec-packet-generator convention.

## TASK-220 — Single sources of truth (3a, 3b, 3c, 3d)  ✅ implemented

### Step 220.1 — 3a: extract `source_key_matches_declared`

- **Precondition:** both `bind_module_config_view` and `config_key_declared`
  in `execution_plan.rs` carry independent inline wildcard-matching code paths.
- **Postcondition:** one `fn source_key_matches_declared(declared_key, candidate) -> bool`
  in `execution_plan.rs`; both call sites use it.
- **Exit condition:** `grep -c "fn source_key_matches_declared"` returns `1`;
  `cargo test -p slicer-runtime --test contract config_view_binding_tdd` passes.
- **Files to read:** `crates/slicer-runtime/src/execution_plan.rs` (matcher
  sites only — symbol-search, not full read).
- **Files to edit:** `crates/slicer-runtime/src/execution_plan.rs`.
- **Expected dispatches:** none (small, focused edit).
- **Context cost:** S.

### Step 220.2 — 3c: add `slicer_core::transform_point3`

- **Precondition:** 4 copies of an affine `(matrix, point) -> point` transform
  exist in `mesh_analysis`, `paint_segmentation`, `model_loader`, and
  `prepass_slice`; semantics differ (zero-matrix guard present in 2;
  perspective w-divide present in 1).
- **Postcondition:** `slicer_core::transform_point3(matrix: &[f64;16], p: Point3) -> Point3`
  with superset semantics (zero-matrix → identity, w-divide always applied);
  4 prior copies become one-line adapters (some take `&Transform3d`).
- **Exit condition:** `grep -c "fn transform_point3" crates/slicer-core/src/lib.rs`
  returns `1`; `cargo test -p slicer-core --lib transform_point3` passes (≥3
  unit tests: affine, zero-matrix, identity-w).
- **Files to read:** `crates/slicer-runtime/src/{mesh_analysis,paint_segmentation,model_loader,prepass_slice}.rs`
  (transform sites only).
- **Files to edit:** `crates/slicer-core/src/lib.rs` (add fn + unit tests), 4
  adapter sites.
- **Expected dispatches:** none.
- **Context cost:** S.

### Step 220.3 — 3b: add `ResolvedConfig::to_config_map`; gcode + dispatch delegate

- **Precondition:** `gcode_emit::resolved_config_to_map` and
  `dispatch::resolved_config_to_map` are divergent (dispatch missing 7 keys:
  `wall_generator`, `infill_type`, `{top,bottom,bridge,sparse}_fill_holder`,
  `support_type`).
- **Postcondition:** `ResolvedConfig::to_config_map(&self) -> HashMap<String, ConfigValue>`
  lives in `slicer-ir`, superset of the gcode CONFIG_BLOCK emitter (including
  `extensions` passthrough); both runtime call sites delegate.
- **Exit condition:** `grep -c "fn to_config_map" crates/slicer-ir/src/resolved_config.rs`
  returns `1`; `cargo test -p slicer-runtime --test integration gcode_header`
  passes (golden unchanged); `cargo test -p slicer-runtime --test contract` passes.
- **Files to read:** `crates/slicer-ir/src/resolved_config.rs` (field set),
  `crates/slicer-runtime/src/gcode_emit.rs` and
  `crates/slicer-runtime/src/dispatch.rs` (existing serializer bodies only).
- **Files to edit:** `crates/slicer-ir/src/resolved_config.rs`,
  `crates/slicer-runtime/src/{gcode_emit,dispatch}.rs`.
- **Expected dispatches:** the gcode golden may need a sub-agent run to verify
  byte-identity on the Benchy CONFIG_BLOCK output — FACT pass/fail only.
- **Context cost:** S.

### Step 220.4 — 3d: add `stage_order.rs`; manifest/validation/dag_cli forward

- **Precondition:** `manifest::known_stage_ids` and
  `validation::stage_order_index` carry short copies of the canonical
  `execution_plan::STAGE_ORDER`; both have silently dropped
  `PrePass::SeamPlanning`, `PrePass::SupportGeometry`, and
  `Layer::PaintRegionAnnotation`. A module declaring any of those stages is
  rejected at startup with `UnknownStage`.
- **Postcondition:** new `crates/slicer-runtime/src/stage_order.rs` exposes
  `known_stage_ids`, `is_known_stage`, `stage_order_index`, `tier_of`, all
  derived from `STAGE_ORDER`. `manifest.rs`, `validation.rs`, `dag_cli.rs`
  delete their inline literals and forward.
- **Exit condition:** new file derives (no 19/22-entry inline literal);
  `cargo test -p slicer-runtime --test unit stage_canon_seam_support_tdd`
  passes (3 positive: SeamPlanning, SupportGeometry, PaintRegionAnnotation +
  1 negative: misspelled stage still rejected with `UnknownStage`);
  `cargo test -p slicer-runtime --test unit dag_validation_tdd` passes.
- **Files to read:** `crates/slicer-runtime/src/execution_plan.rs`
  (`STAGE_ORDER` only), `crates/slicer-runtime/src/{manifest,validation,dag_cli}.rs`
  (existing copies only), `crates/slicer-schema/src/lib.rs` (`STAGES`
  consistency).
- **Files to edit:** add `crates/slicer-runtime/src/stage_order.rs`; register
  `mod stage_order;` in `crates/slicer-runtime/src/lib.rs`;
  `crates/slicer-runtime/src/{manifest,validation,dag_cli}.rs` forward; add
  `crates/slicer-runtime/tests/unit/stage_canon_seam_support_tdd.rs` and
  register it in `crates/slicer-runtime/tests/unit/main.rs`.
- **Expected dispatches:** none.
- **Context cost:** S.

## TASK-221 — Pipeline/region collapse (1a, 1b)  ✅ implemented

### Step 221.1 — 1a: single-pass region stamping

- **Precondition:** `commit_region_mapping_builtin` runs `execute_region_mapping_inner`,
  then re-runs modifier + paint stamping with a different base, discarding the
  inner result. Two passes; second pass wins.
- **Postcondition:** `execute_region_mapping_inner` gains
  `host_config: Option<(&BTreeMap<String, ResolvedConfig>, &ResolvedConfig)>`.
  `None` (test/e2e path) keeps `region.resolved_config` as base; `Some`
  (commit path) uses the per-object map / default as base then stamps modifier
  deltas + paint overlays once. The duplicate post-hoc loop in
  `commit_region_mapping_builtin` is deleted.
- **Exit condition:** `grep -c "host_config: Option" crates/slicer-runtime/src/region_mapping.rs`
  returns `1` (positive structural evidence of the host-config Option threading
  that replaces the post-hoc re-stamp loop); `cargo test -p slicer-runtime --test e2e`
  passes.
- **Files to read:** `crates/slicer-runtime/src/region_mapping.rs`
  (`commit_region_mapping_builtin` + `execute_region_mapping_inner` only).
- **Files to edit:** `crates/slicer-runtime/src/region_mapping.rs`.
- **Expected dispatches:** the e2e suite (~500s) should be dispatched as a
  background FACT — never absorb the full output.
- **Context cost:** M (e2e dispatch cost is the bulk).

### Step 221.2 — 1b: extract `run_pipeline_core`

- **Precondition:** `run_pipeline_with_raw_config` and
  `run_pipeline_with_instrumentation` have byte-identical bodies (prepass →
  per-layer → finalization → postpass + CONFIG_BLOCK serialization).
  `run_pipeline_with_events` has a distinct body (bare gcode, no CONFIG_BLOCK).
- **Postcondition:** `run_pipeline_core(config, raw_config, sink, instrumentation)`
  holds the shared body. `run_pipeline_with_raw_config` and
  `run_pipeline_with_instrumentation` forward (`&NoopInstrumentation` for the
  former). `run_pipeline_with_events` is intentionally NOT merged (the
  CONFIG_BLOCK difference is locked by `pipeline_tdd`; merging breaks 3
  assertions). A comment in `pipeline.rs` records the rationale.
- **Exit condition:** `grep -c "fn run_pipeline_core" crates/slicer-runtime/src/pipeline.rs`
  returns `1`; `cargo test -p slicer-runtime --test integration pipeline_tdd`
  and `--test contract dispatch_tdd` pass.
- **Files to read:** `crates/slicer-runtime/src/pipeline.rs` (3 entry-point
  bodies only).
- **Files to edit:** `crates/slicer-runtime/src/pipeline.rs`;
  `crates/slicer-runtime/src/{prepass,wit_host}.rs` (incidental signature
  updates only); `crates/slicer-runtime/tests/integration/run_pipeline_with_instrumentation_tdd.rs`
  (fixture signature update).
- **Expected dispatches:** none.
- **Context cost:** S.

## TASK-222 — Macro conversion DRY (2) — DEFERRED, NOT RECOMMENDED

Deferred from packet 76 and reassessed as **not worth implementing**. Kept on
the backlog only so the reasoning is not re-litigated.

Reassessment (2026-05-31):

1. **Headline benefit disproven.** The depth review pitched this as making the
   "largest untestable surface" `#[test]`-able. Per-world `wit_bindgen::generate!`
   makes that impossible (ADR-0003) — the conversions stay
   guest-round-trip-only before and after. Only pure dedup remains.
2. **Compiler-guarded, low-churn duplication.** A WIT type change makes every
   world fail to compile (loud, not silent drift), so the type system already
   provides most of the anti-drift value dedup would add. The converters move
   only when WIT geometry types change — rare and gated.
3. **Role types ARE shared** (`slicer_sdk::ir` is `pub use slicer_ir as ir`),
   so a unified emitter is feasible — but it still needs several parameterized
   arms (per-world WIT type paths; 12- vs 14-variant role directions; per-world
   WIT builder resource names), i.e. a non-trivial abstraction for partial
   dedup.
4. **Likely worsens navigability.** Replaces N short literal `match` blocks
   with a token-emitting meta-function on the most fragile file — more
   indirection for a future agent/human to trace.
5. **Highest risk, lowest payoff in the candidate set.** 2757-line macro
   driving all 32 guests; verification needs a full guest rebuild.

**Recommendation:** do not implement as a standalone task. Only collapse the
two genuinely byte-identical pairs **opportunistically**, if a future WIT
change already forces that area open: the gcode-drain shared by postpass/layer,
and the `ExtrusionPath3D` converters shared by finalization/layer. The durable
value from this candidate — ADR-0003 — is already captured.

## Close

- `cargo xtask build-guests --check` → clean.
- Full `cargo test --workspace` via sub-agent (pass/fail FACT).
- Record outcome in this packet's AC-CLOSE.

## Status

- TASK-220, TASK-221: implemented + verified (narrow suites green; clippy
  clean; AC-3d test wired into `tests/unit/main.rs` 2026-05-31 — original
  packet 76 submission left it unwired, caught in spec-review).
- TASK-222: deferred and reassessed as **not recommended** (see above). The
  durable insight (ADR-0003) is retained; the macro rewrite is dropped.
