//! Immutable execution-plan contracts for the host scheduler.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, AnchoredEntity, CapabilityDerivedEventClosure, ConfigKey, ConfigValue,
    ConfigView, GlobalLayer, ModuleId, RegionKey, RegionPlan, StageId,
};

use crate::manifest::DiagnosticLevel;
use crate::manifest::{LoadDiagnostic, LoadedModule};
use crate::region_split::{aggregate_region_splits, AggregatedRegionSplitEntry};

/// Canonical scheduler stage ordering for the live host path
/// (docs/04 §Fixed Stage Order). Modules discovered by
/// [`load_live_modules_for_plan`] are grouped and sorted in this
/// order; stages not present among the loaded modules are skipped.
pub const STAGE_ORDER: &[&str] = &[
    "PrePass::MeshAnalysis",
    "PrePass::LayerPlanning",
    "PrePass::SeamPlanning",
    "PrePass::PaintSegmentation",
    "PrePass::RegionMapping",
    "PrePass::Slice",
    // OverhangAnnotation derives overhang from the committed slices
    // (OrcaSlicer's `detect_overhangs_for_lift` diffs consecutive `lslices`),
    // so it runs strictly AFTER Slice — never re-slicing the mesh.
    "PrePass::OverhangAnnotation",
    "PrePass::ShellClassification",
    "PrePass::SupportAnalysis",
    "PrePass::SupportGeometry",
    "PrePass::LightningTreeGen",
    "Layer::PaintRegionAnnotation",
    "Layer::SlicePostProcess",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::Infill",
    "Layer::InfillPostProcess",
    "Layer::Support",
    "Layer::SupportPostProcess",
    "Layer::AnchoredEvents",
    "Layer::PathOptimization",
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",
    "PostPass::GCodePostProcess",
    "PostPass::TextPostProcess",
];

/// Build the `Arc<ConfigView>` bound for one `LoadedModule` on the live
/// host/runtime path.
///
/// Pre-filters `source` to the module's declared `config_schema.entries`
/// keys (the canonical declared-read set per docs/03 §host-boundary
/// enforcement and docs/02 §pre-filtered config), then freezes the result
/// behind an `Arc` so downstream consumers cannot mutate the view they see.
///
/// This is the ONLY supported construction path for live-runtime config
/// views; test fixtures may still use `ConfigView::from_map`, but
/// production planning (main.rs / runtime binding) must route through this
/// helper to stay contract-compliant.
#[must_use]
pub fn bind_module_config_view(
    module: &LoadedModule,
    source: &HashMap<ConfigKey, ConfigValue>,
) -> Arc<ConfigView> {
    // Support `prefix:*` wildcard entries in the module's declared
    // config schema so per-object keys (e.g. `object_height:<uuid>`)
    // can be consumed by planners that only know a static schema.
    // A declared key of the form `<prefix>:*` expands to every source
    // key that begins with `<prefix>:`. Static declared keys continue
    // to require exact match (docs/03 §host-boundary enforcement;
    // docs/02 §pre-filtered config).
    let mut effective: Vec<String> = Vec::new();
    for declared_key in module.config_schema.entries.keys() {
        if declared_key.ends_with(":*") {
            for src_key in source.keys() {
                if source_key_matches_declared(declared_key, src_key) {
                    effective.push(src_key.clone());
                }
            }
        } else {
            effective.push(declared_key.clone());
        }
    }
    // Support-family dispatch is a host-level selection shared by the
    // planner and renderers. Its keys are not module-specific tuning knobs,
    // so expose them to paired support modules even when their manifests do
    // not repeat the common declaration.
    if module
        .claims()
        .iter()
        .any(|claim| claim.starts_with("support-family:"))
    {
        for key in [SUPPORT_GENERATOR_CONFIG_KEY, SUPPORT_FAMILY_CONFIG_KEY] {
            if source.contains_key(key) && !effective.iter().any(|entry| entry == key) {
                effective.push(key.to_string());
            }
        }
    }
    Arc::new(ConfigView::from_declared(
        source,
        effective.iter().map(String::as_str),
    ))
}

/// Returns true when `candidate` is satisfied by `declared_key`, treating a
/// trailing `:*` on `declared_key` as a `<prefix>:` wildcard; a static declared
/// key requires an exact match. Shared by [`bind_module_config_view`] (wildcard
/// expansion) and [`config_key_declared`] so the two stay in lockstep
/// (docs/03 §host-boundary enforcement).
fn source_key_matches_declared(declared_key: &str, candidate: &str) -> bool {
    if let Some(prefix) = declared_key.strip_suffix(":*") {
        candidate
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(':'))
    } else {
        declared_key == candidate
    }
}

/// Structured failure parsing a user-facing JSON config source.
#[derive(Debug, Clone)]
pub enum ConfigSourceParseError {
    /// The input was not valid JSON.
    InvalidJson {
        /// Human-readable serde error.
        message: String,
    },
    /// The top-level JSON value was not an object.
    NotAnObject,
    /// A value under `key` could not be mapped to any `ConfigValue` variant.
    UnsupportedValue {
        /// Key carrying the unsupported value.
        key: String,
    },
}

impl std::fmt::Display for ConfigSourceParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson { message } => write!(f, "invalid JSON config: {message}"),
            Self::NotAnObject => {
                write!(
                    f,
                    "top-level JSON config must be an object of key→value pairs"
                )
            }
            Self::UnsupportedValue { key } => {
                write!(f, "config key '{key}' has an unsupported JSON value (only bool, number, string, and homogeneous arrays are allowed)")
            }
        }
    }
}

impl std::error::Error for ConfigSourceParseError {}

/// Parse a user-facing JSON config source into a raw
/// `HashMap<ConfigKey, ConfigValue>` ready to be fed to
/// [`bind_module_config_view`].
///
/// JSON types map as: `bool → Bool`, integer number → `Int`, non-integer
/// number → `Float` (subnormals normalised to `0.0`, matching the WIT
/// boundary), string → `String`, and array → `List` (recursed element-wise).
/// `null` and nested object values are rejected with `UnsupportedValue`,
/// because `ConfigValue` has no `null`/record representation and silent
/// coercion would contradict docs/03 §host-boundary enforcement.
pub fn parse_cli_config_source(
    json: &str,
) -> Result<HashMap<ConfigKey, ConfigValue>, ConfigSourceParseError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| ConfigSourceParseError::InvalidJson {
            message: e.to_string(),
        })?;
    let object = match value {
        serde_json::Value::Object(m) => m,
        _ => return Err(ConfigSourceParseError::NotAnObject),
    };
    let mut out = HashMap::with_capacity(object.len());
    for (key, raw) in object {
        let value = json_to_config_value(&raw)
            .ok_or_else(|| ConfigSourceParseError::UnsupportedValue { key: key.clone() })?;
        out.insert(key, value);
    }
    Ok(out)
}

/// Returns true when `key` is satisfied by some entry in `declared`,
/// either as an exact match or via a `prefix:*` wildcard pattern that
/// [`bind_module_config_view`] also accepts. See that helper for the
/// full rationale (docs/03 §host-boundary enforcement).
fn config_key_declared(
    declared: &std::collections::BTreeMap<String, crate::manifest::ConfigFieldEntry>,
    key: &str,
) -> bool {
    declared
        .keys()
        .any(|declared_key| source_key_matches_declared(declared_key, key))
}

/// Config key selecting which perimeter-generator module wins the
/// `perimeter-generator` claim when more than one module declares it.
/// Mirrors OrcaSlicer's `wall_generator` setting (`"classic"` | `"arachne"`).
///
/// Read directly from the raw CLI/JSON config source at module-load time —
/// like `use_relative_e_distances` / `thumbnail_path`
/// (`docs/config/host-keys.toml` `[host_runtime]`) — rather than through
/// `ResolvedConfig`, because this selection has to happen before
/// `ResolvedConfig` is built (module loading / claim dedup runs first; see
/// `crates/slicer-runtime/src/run.rs`).
pub const WALL_GENERATOR_CONFIG_KEY: &str = "wall_generator";

/// Config key selecting whether the slice is a "spiral vase" print.
/// Mirrors OrcaSlicer's `spiral_mode` / `spiral_vase` setting. When `true`,
/// the classic perimeter generator (`com.core.classic-perimeters`) is forced
/// for the `perimeter-generator` claim regardless of `wall_generator`, because
/// Arachne's variable-width walls are incompatible with spiral-vase mode
/// (OrcaSlicer gates Arachne dispatch on `!spiral_mode` at
/// `LayerRegion.cpp:138-141`).
///
/// Read directly from the raw CLI/JSON config source at module-load time, like
/// [`WALL_GENERATOR_CONFIG_KEY`], because this selection has to happen before
/// `ResolvedConfig` is built (module loading / claim dedup runs first; see
/// `crates/slicer-runtime/src/run.rs`).
pub const SPIRAL_VASE_CONFIG_KEY: &str = "spiral_vase";

/// Default `wall_generator` value used when the config key is absent.
/// Keeps every existing golden/regression test slicing with
/// `classic-perimeters` unchanged (packet 112 Step 10).
pub const DEFAULT_WALL_GENERATOR: &str = "classic";

const PERIMETER_GENERATOR_CLAIM: &str = "perimeter-generator";
const CLASSIC_PERIMETERS_MODULE_ID: &str = "com.core.classic-perimeters";
const ARACHNE_PERIMETERS_MODULE_ID: &str = "com.core.arachne-perimeters";

/// Config key selecting which support module wins the `support-generator`
/// claim when more than one module declares it. Mirrors OrcaSlicer's
/// `support_type` setting (`"normal(auto)"` | `"tree(auto)"` | manual
/// variants; legacy `"normal"`/`"tree"`/`"hybrid(auto)"` spellings can still
/// appear in old 3MF sidecars).
///
/// Read directly from the raw CLI/JSON config source at module-load time —
/// the same channel OrcaSlicer's raw `enable_support` reaches pnp through
/// (the GUI merges its translated config over the raw Orca config in the 3MF
/// sidecar; the raw `support_type` string rides along) — rather than through
/// `ResolvedConfig`, because this selection has to happen before
/// `ResolvedConfig` is built (module loading / claim dedup runs first; see
/// `crates/slicer-runtime/src/run.rs`).
pub const SUPPORT_GENERATOR_CONFIG_KEY: &str = "support_type";
/// Config key carrying the canonical per-region support family.
pub const SUPPORT_FAMILY_CONFIG_KEY: &str = "support_family";

const SUPPORT_GENERATOR_CLAIM: &str = "support-generator";
const SUPPORT_PLANNER_CLAIM: &str = "support-planner";

/// Resolve a raw `support_type` config value (`config_source.get("support_type")`,
/// e.g. `Some("tree(auto)")` — OrcaSlicer's spelling) to the module id it
/// selects for the `support-generator` claim. Values starting with `tree`
/// (`tree(auto)` / `tree(manual)`) or `hybrid` (the legacy `hybrid(auto)`
/// spelling — OrcaSlicer itself migrates it to `tree(auto)` at config load,
/// and a raw 3MF sidecar may still carry it) select
/// `com.core.tree-support`. Absent (`None`) and every other value fall back
/// to `com.core.traditional-support` — which is also the alphabetical first
/// winner, so an absent key keeps historical behaviour byte-for-byte.
/// Resolve canonical support-family selection, including legacy aliases.
/// `support_type`, when present, intentionally overrides the canonical value.
pub fn select_support_family(
    support_family: Option<&str>,
    support_type: Option<&str>,
) -> &'static str {
    slicer_ir::canonical_support_family(support_type.or(support_family))
}

/// Returns whether a module may receive an active region under support-family
/// dispatch. Modules without a support-family claim remain region-agnostic.
pub fn module_claims_match_active_region(claims: &[String], region: &ActiveRegion) -> bool {
    let mut has_support_family_claim = false;
    for claim in claims {
        let Some(family) = claim.strip_prefix("support-family:") else {
            continue;
        };
        has_support_family_claim = true;
        let support_family = region
            .resolved_config
            .extensions
            .get(SUPPORT_FAMILY_CONFIG_KEY)
            .and_then(|value| match value {
                ConfigValue::String(value) => Some(value.as_str()),
                _ => None,
            });
        // `support_type` remains a compatibility alias. `normal(auto)` is the
        // default enum value, so only an explicit legacy extension or a
        // non-default enum value overrides the canonical family (see
        // `SupportType::family_claim`).
        let support_type = region
            .resolved_config
            .extensions
            .get(SUPPORT_GENERATOR_CONFIG_KEY)
            .and_then(|value| match value {
                ConfigValue::String(value) => Some(value.as_str()),
                _ => None,
            })
            .or(region.resolved_config.support_type.family_claim());
        let selected = select_support_family(support_family, support_type);
        if family == selected {
            return true;
        }
    }
    !has_support_family_claim
}

/// Structured diagnostic for an incomplete support planner/renderer pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportFamilyPairingError {
    /// Family IDs that have a planner but no renderer.
    pub missing_renderers: Vec<String>,
    /// Family IDs that have a renderer but no planner.
    pub missing_planners: Vec<String>,
}

impl std::fmt::Display for SupportFamilyPairingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "support family pairing invalid: missing renderers {:?}, missing planners {:?}",
            self.missing_renderers, self.missing_planners
        )
    }
}

impl std::error::Error for SupportFamilyPairingError {}

/// Validate support family pairs, returning warnings for incomplete families.
///
/// Incomplete families are intentionally non-fatal: their regions simply have
/// no complete planner/renderer route and therefore produce no support plan.
pub fn validate_support_family_pairing(
    modules: &[LoadedModule],
) -> Result<Vec<SupportFamilyPairingError>, SupportFamilyPairingError> {
    use std::collections::BTreeSet;
    let mut planners = BTreeSet::new();
    let mut renderers = BTreeSet::new();
    for module in modules {
        for claim in module.claims() {
            if let Some(family) = claim.strip_prefix("support-family:") {
                if module.claims().iter().any(|c| c == "support-planner") {
                    planners.insert(family.to_string());
                }
                if module.claims().iter().any(|c| c == SUPPORT_GENERATOR_CLAIM) {
                    renderers.insert(family.to_string());
                }
            }
        }
    }
    let missing_renderers = planners.difference(&renderers).cloned().collect();
    let missing_planners = renderers.difference(&planners).cloned().collect();
    let error = SupportFamilyPairingError {
        missing_renderers,
        missing_planners,
    };
    if error.missing_renderers.is_empty() && error.missing_planners.is_empty() {
        Ok(Vec::new())
    } else {
        Ok(vec![error])
    }
}

/// Resolve a raw `wall_generator` config value (`config_source.get("wall_generator")`,
/// e.g. `Some("arachne")`) to the module id it selects for the
/// `perimeter-generator` claim. Absent (`None`) or unrecognized values fall
/// back to [`DEFAULT_WALL_GENERATOR`] (`"classic"`).
fn wall_generator_preferred_module_id(wall_generator: Option<&str>) -> &'static str {
    match wall_generator {
        Some("arachne") => ARACHNE_PERIMETERS_MODULE_ID,
        _ => CLASSIC_PERIMETERS_MODULE_ID,
    }
}

/// Enforce claim uniqueness across modules in the same stage.
///
/// For each `(stage, claim)` pair, resolves exactly one winning `module_id`
/// and drops the rest. Emits one `LoadDiagnostic` per dropped module so
/// operators can see which module "won" each claim. Modules with no
/// `claims.holds` entries are kept unchanged.
///
/// The winner is normally the alphabetically-first candidate. Two claims are
/// documented exceptions. The `perimeter-generator` claim (packet 112 Step
/// 10): when both `com.core.classic-perimeters` and
/// `com.core.arachne-perimeters` are candidates, the winner is resolved by
/// `wall_generator` (see [`wall_generator_preferred_module_id`]) instead —
/// this closes a production defect where the production loader called this
/// dedup with no config input and silently selected `arachne-perimeters`
/// (alphabetically first) with no way for a user's config to express intent,
/// and `incompatible-with` never fired because dedup runs before
/// `validate_startup_dag`. The `support-generator` claim (and the
/// family-scoped half of `support-planner`) is the second exception, but in
/// the opposite direction: it is **not deduplicated at all**. Packet 221
/// moved support-family selection from load time to dispatch time, where
/// [`module_claims_match_active_region`] picks a renderer/planner pair per
/// region from that region's resolved `support_family` / `support_type`.
/// Every family candidate must therefore survive this pass; collapsing them
/// to one winner would make the losing family undispatchable for any region
/// that selects it.
///
/// Consequence: the `support_type` argument threaded through
/// [`dedup_same_claim_modules_with_wall_generator`] no longer influences this
/// function's result. It is retained (as `_support_type`) rather than removed
/// because it is part of a `pub` signature that the production live-loader
/// `slicer_wasm_host::load_live_modules_for_plan_with_config` calls, and
/// because the same raw value is what the region-mapping pass later resolves
/// into `ResolvedConfig`. **There is no support_type-preferred-module branch
/// here any more — do not add one back.** An earlier revision selected
/// `com.core.tree-support` over `com.core.traditional-support` here (without
/// it, `tree-support` lost by alphabetical accident, `traditional` sorting
/// before `tree`); that selection now lives entirely in
/// [`select_support_family`] / [`module_claims_match_active_region`].
///
/// Matches docs/04 §2 "Global claim conflicts" (exactly one holder
/// globally per claim) and docs/10 §Glossary ("Exactly one holder per
/// (layer, object, region, claim) at execution"). Per-region scoping
/// is deferred to the region-mapping pass; at live-load time we only
/// enforce the global/stage constraint.
/// Test-only wrapper around [`dedup_same_claim_modules`] so integration
/// tests can exercise the claim dedup path without building a full
/// `LoadModulesReport`. Behaviour is identical to the private helper with
/// `wall_generator` and `support_type` absent (`None`), i.e.
/// [`DEFAULT_WALL_GENERATOR`] (`"classic"`) applies if a
/// `perimeter-generator` collision is present. `support_type` is inert here
/// (see above); support claims are never deduplicated. See
/// [`dedup_same_claim_modules_with_wall_generator`] for the config-aware
/// entry point the production live-loader uses.
#[doc(hidden)]
pub fn dedup_same_claim_modules_for_test(
    modules: &mut Vec<LoadedModule>,
    diagnostics: &mut Vec<LoadDiagnostic>,
) -> Vec<LoadedModule> {
    dedup_same_claim_modules(modules, diagnostics, None, false, None)
}

/// Config-aware claim dedup: identical to [`dedup_same_claim_modules_for_test`]
/// except `wall_generator` (the raw `config_source.get("wall_generator")`
/// string value, or `None` if the key is absent), `spiral_vase` (the raw
/// `config_source.get("spiral_vase")` bool value, or `false` if absent) and
/// `support_type` (the raw `config_source.get("support_type")` string value,
/// or `None` if the key is absent) are threaded through. Only
/// `wall_generator` / `spiral_vase` affect the outcome, resolving the
/// `perimeter-generator` claim; when `spiral_vase` is `true`, the classic
/// perimeter generator is forced for that claim regardless of
/// `wall_generator` (Arachne is incompatible with spiral-vase mode).
/// `support_type` is accepted but **unreachable** as a selector: support
/// claims are not deduplicated post-packet-221 (see
/// [`dedup_same_claim_modules_for_test`]'s doc comment). This is the entry
/// point
/// `slicer_wasm_host::load_live_modules_for_plan_with_config` (the
/// production live-loader) uses.
pub fn dedup_same_claim_modules_with_wall_generator(
    modules: &mut Vec<LoadedModule>,
    diagnostics: &mut Vec<LoadDiagnostic>,
    wall_generator: Option<&str>,
    spiral_vase: bool,
    support_type: Option<&str>,
) -> Vec<LoadedModule> {
    dedup_same_claim_modules(
        modules,
        diagnostics,
        wall_generator,
        spiral_vase,
        support_type,
    )
}

fn dedup_same_claim_modules(
    modules: &mut Vec<LoadedModule>,
    diagnostics: &mut Vec<LoadDiagnostic>,
    wall_generator: Option<&str>,
    spiral_vase: bool,
    // Intentionally unused, and intentionally still in the signature. Support
    // renderers/planners are selected per region at dispatch time
    // (`module_claims_match_active_region`), never here — see this module's
    // `dedup_same_claim_modules_for_test` doc comment. Reading `support_type`
    // in this function would re-introduce the pre-221 mutual exclusion and
    // make one support family undispatchable.
    _support_type: Option<&str>,
) -> Vec<LoadedModule> {
    use std::collections::BTreeMap;

    let mut sorted: Vec<LoadedModule> = std::mem::take(modules);
    sorted.sort_by(|a, b| a.id.cmp(&b.id));

    // ── Pass 1: collect (stage, claim) -> candidate module ids ──────────
    // `sorted` is already alphabetical by id, so each `candidate_ids` Vec
    // built below is too. Fill-role claims (packet 37) are excluded here —
    // see the rationale on the skip below in pass 3.
    let mut candidates_for: BTreeMap<(StageId, String), Vec<ModuleId>> = BTreeMap::new();
    for module in &sorted {
        for claim in &module.claims {
            if crate::validation::FILL_CLAIM_IDS.contains(&claim.as_str())
                || claim.starts_with("support-family:")
            {
                continue;
            }
            candidates_for
                .entry((module.stage.clone(), claim.clone()))
                .or_default()
                .push(module.id.clone());
        }
    }

    // ── Pass 2: resolve exactly one winner per contested (stage, claim) ──
    // Computed BEFORE the per-module pass below so `perimeter-generator`
    // can be resolved by `wall_generator` config rather than by iteration
    // order (packet 112 Step 10 — see this function's doc comment).
    let mut winner_for: BTreeMap<(StageId, String), ModuleId> = BTreeMap::new();
    // Family-scoped support planners coexist per family (mirroring how
    // `support-generator` is retained per region); only planners without a
    // `support-family:` claim fall back to global alphabetical resolution.
    let family_scoped: std::collections::HashSet<ModuleId> = sorted
        .iter()
        .filter(|module| {
            module
                .claims
                .iter()
                .any(|claim| claim.starts_with("support-family:"))
        })
        .map(|module| module.id.clone())
        .collect();
    for ((stage, claim), candidate_ids) in &candidates_for {
        if candidate_ids.len() < 2 {
            continue; // sole holder; nothing to resolve
        }
        if claim == PERIMETER_GENERATOR_CLAIM {
            // Spiral-vase mode is incompatible with Arachne's variable-width
            // walls, so force the classic generator regardless of
            // `wall_generator` (OrcaSlicer gates Arachne dispatch on
            // `!spiral_mode`).
            let preferred = if spiral_vase {
                CLASSIC_PERIMETERS_MODULE_ID
            } else {
                wall_generator_preferred_module_id(wall_generator)
            };
            if candidate_ids.iter().any(|id| id == preferred) {
                winner_for.insert((stage.clone(), claim.clone()), preferred.to_string());
                continue;
            }
            // Preferred module isn't actually among the candidates (e.g. a
            // community module reusing this claim name) — fall through to
            // the alphabetical default below.
        }
        if claim == SUPPORT_GENERATOR_CLAIM {
            // Support renderers are selected per region. Retaining every family
            // candidate here is required for atomic planner/renderer dispatch.
            continue;
        }
        if claim == SUPPORT_PLANNER_CLAIM {
            // Support planners are selected per family, exactly like renderers
            // are selected per region. Family-scoped planners coexist; only
            // non-family planners are deduplicated alphabetically (fallback).
            let global_candidates: Vec<&ModuleId> = candidate_ids
                .iter()
                .filter(|id| !family_scoped.contains(*id))
                .collect();
            if global_candidates.len() < 2 {
                continue;
            }
            winner_for.insert((stage.clone(), claim.clone()), global_candidates[0].clone());
            continue;
        }
        // Default: alphabetically-first candidate wins (docs/04 §2 "Global
        // claim conflicts").
        winner_for.insert((stage.clone(), claim.clone()), candidate_ids[0].clone());
    }

    // ── Pass 3: keep every module whose claims all match their winner ───
    let mut kept: Vec<LoadedModule> = Vec::with_capacity(sorted.len());
    for module in sorted {
        let mut losing_claim: Option<(String, ModuleId)> = None;
        for claim in &module.claims {
            // Fill-role claims (packet 37) are per-region-configurable via
            // `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder` and resolved
            // at dispatch time in `slicer-wasm-host/src/dispatch.rs`. They must
            // NOT be deduplicated at startup: multiple modules legitimately
            // declare the same fill claim and the per-region resolver picks the
            // active holder. Without this skip, gyroid wins `claim:sparse-fill`
            // alphabetically and rectilinear (which holds all four) is dropped
            // whole, defeating any user config that names rectilinear for
            // top/bottom/bridge — see DEV-065 and docs/04 §"Validation Passes".
            if crate::validation::FILL_CLAIM_IDS.contains(&claim.as_str())
                || claim == SUPPORT_GENERATOR_CLAIM
                || (claim == SUPPORT_PLANNER_CLAIM && family_scoped.contains(&module.id))
            {
                continue;
            }
            let key = (module.stage.clone(), claim.clone());
            if let Some(winner) = winner_for.get(&key) {
                if winner != &module.id {
                    losing_claim = Some((claim.clone(), winner.clone()));
                    break;
                }
            }
        }
        if let Some((claim, winner)) = losing_claim {
            diagnostics.push(LoadDiagnostic {
                level: DiagnosticLevel::Info,
                path: module.wasm_path.clone(),
                field: Some(String::from("claims.holds")),
                message: format!(
                    "module '{id}' in stage '{stage}' dropped: claim '{claim}' \
                     already held by '{winner}' (first-winner dedup; docs/04 §2)",
                    id = module.id,
                    stage = module.stage,
                    claim = claim,
                    winner = winner,
                ),
            });
            continue;
        }
        kept.push(module);
    }

    kept
}

fn json_to_config_value(raw: &serde_json::Value) -> Option<ConfigValue> {
    match raw {
        serde_json::Value::Bool(b) => Some(ConfigValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(ConfigValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                let f = if f.is_subnormal() { 0.0 } else { f };
                Some(ConfigValue::Float(f))
            } else {
                None
            }
        }
        serde_json::Value::String(s) => Some(ConfigValue::String(s.clone())),
        serde_json::Value::Array(items) => {
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                converted.push(json_to_config_value(item)?);
            }
            Some(ConfigValue::List(converted))
        }
        serde_json::Value::Null | serde_json::Value::Object(_) => None,
    }
}

/// Frozen runtime scheduling state shared read-only across worker threads.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Topologically sorted prepass stages excluding host-built-ins.
    pub prepass_stages: Vec<CompiledStage>,
    /// Topologically sorted per-layer stages excluding host-built-ins.
    pub per_layer_stages: Vec<CompiledStage>,
    /// Dedicated sequential finalization bucket.
    pub layer_finalization_stage: Option<CompiledStage>,
    /// Topologically sorted postpass stages excluding host-built-ins and finalization.
    pub postpass_stages: Vec<CompiledStage>,
    /// Frozen global layer schedule.
    pub global_layers: Arc<Vec<GlobalLayer>>,
    /// Frozen per-region execution plans.
    pub region_plans: Arc<HashMap<RegionKey, RegionPlan>>,
    /// Precomputed index for O(1) lookup of active regions per (layer, module).
    /// Key: (global_layer_index, module_id) → Value: slice of ActiveRegion.
    pub module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>>,
    /// Cross-manifest aggregate of `[[region_split]]` declarations
    /// (semantic → priority/value-type/declaring modules).
    ///
    /// Empty `BTreeMap` when no loaded module declares region-split semantics
    /// — this is the production default today, which preserves AC-10
    /// byte-identical g-code. See packet 93, AC-1.
    pub aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry>,
}

/// Scheduler metadata for one anchored invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchoredInvocation {
    /// Global layer that owns this invocation's execution ordering.
    pub anchor_global_layer_index: u32,
    /// Capability-derived stage closure for the invocation.
    pub closure: CapabilityDerivedEventClosure,
    /// Provenance retained from the anchored entity.
    pub provenance: slicer_ir::AnchoredEntityProvenance,
    /// Whether this invocation may run concurrently with other layer work.
    pub layer_parallel_safe: bool,
}

impl AnchoredInvocation {
    /// Derive scheduling metadata from the entity's declared capabilities.
    pub fn from_entity(entity: &AnchoredEntity, layer_parallel_safe: bool) -> Self {
        Self {
            anchor_global_layer_index: entity.anchor_global_layer_index,
            closure: CapabilityDerivedEventClosure::derive(
                &entity.input_capabilities,
                &entity.output_capabilities,
            ),
            provenance: entity.provenance.clone(),
            layer_parallel_safe,
        }
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self {
            prepass_stages: Vec::new(),
            per_layer_stages: Vec::new(),
            layer_finalization_stage: None,
            postpass_stages: Vec::new(),
            global_layers: Arc::new(Vec::new()),
            region_plans: Arc::new(HashMap::new()),
            module_region_index: HashMap::new(),
            aggregated_region_split: BTreeMap::new(),
        }
    }
}

impl ExecutionPlan {
    /// Derive an anchored invocation without an event-kind or feature stage table.
    pub fn anchored_invocation(
        &self,
        entity: &AnchoredEntity,
        layer_parallel_safe: bool,
    ) -> AnchoredInvocation {
        AnchoredInvocation::from_entity(entity, layer_parallel_safe)
    }

    /// Build an ExecutionPlan with a precomputed module_region_index.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn build_with_index(
        prepass_stages: Vec<CompiledStage>,
        per_layer_stages: Vec<CompiledStage>,
        layer_finalization_stage: Option<CompiledStage>,
        postpass_stages: Vec<CompiledStage>,
        global_layers: Arc<Vec<GlobalLayer>>,
        region_plans: Arc<HashMap<RegionKey, RegionPlan>>,
    ) -> Self {
        // Build index for all Layer:: stages
        let mut module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>> = HashMap::new();
        for layer in global_layers.iter() {
            for stage in &per_layer_stages {
                for module in &stage.modules {
                    let key = (layer.index, module.module_id.clone());
                    let entry = module_region_index.entry(key).or_default();
                    entry.extend(
                        layer
                            .active_regions
                            .iter()
                            .filter(|region| {
                                module_claims_match_active_region(module.claims(), region)
                            })
                            .cloned(),
                    );
                }
            }
        }

        ExecutionPlan {
            prepass_stages,
            per_layer_stages,
            layer_finalization_stage,
            postpass_stages,
            global_layers,
            region_plans,
            module_region_index,
            aggregated_region_split: BTreeMap::new(),
        }
    }
}

/// One compiled scheduler stage ready for direct runtime iteration.
#[derive(Debug, Clone)]
pub struct CompiledStage {
    /// Canonical scheduler stage identifier.
    pub stage_id: StageId,
    /// Topologically sorted module invocations for this stage.
    pub modules: Vec<CompiledModuleStatic>,
}

/// One loaded module bound to immutable runtime execution metadata.
///
/// Construction goes through [`CompiledModuleBuilder`]: pass the module id to
/// [`CompiledModuleBuilder::new`], then chain setters for the optional
/// fields and call [`CompiledModuleBuilder::build`]. Field reads from
/// outside the crate go through the `pub fn` accessor methods declared
/// below.
///
/// Wasmtime handles (`WasmInstancePool`, `WasmComponent`) are NOT stored here;
/// they live in `slicer-wasm-host::LiveModuleBinding` on the live path.
#[derive(Debug, Clone)]
pub struct CompiledModuleStatic {
    /// Reverse-domain module identifier.
    pub(crate) module_id: ModuleId,
    /// Frozen IR read access mask derived from the manifest.
    pub(crate) ir_read_mask: IrAccessMask,
    /// Frozen IR write access mask derived from the manifest.
    pub(crate) ir_write_mask: IrAccessMask,
    /// Frozen module-specific config view.
    pub(crate) config_view: Arc<ConfigView>,
    /// Frozen `[claims].holds` from the manifest. Used by the host's
    /// fill-role resolver (`validation::resolve_held_claims`) to compute the
    /// per-call effective held set for `Layer::Infill`.
    pub(crate) claims: Vec<String>,
    /// Module IDs this module explicitly depends on (manifest
    /// `requires_modules`). Carried through to runtime so
    /// `compute_serial_edges_from_compiled` can emit
    /// `EdgeReason::ExplicitRequires` rows alongside `IrWriteRead`.
    pub(crate) requires_modules: Vec<ModuleId>,
    /// Pre-computed set of region-split semantic names declared by this module.
    /// Empty for paint-transparent modules (the common case). Used by the
    /// per-layer host dispatch filter in `layer_executor.rs` (packet 92).
    pub(crate) region_split_semantics: std::collections::HashSet<String>,
}

impl CompiledModuleStatic {
    /// Reverse-domain module identifier.
    pub fn module_id(&self) -> &ModuleId {
        &self.module_id
    }

    /// Frozen IR read access mask derived from the manifest.
    pub fn ir_read_mask(&self) -> &IrAccessMask {
        &self.ir_read_mask
    }

    /// Frozen IR write access mask derived from the manifest.
    pub fn ir_write_mask(&self) -> &IrAccessMask {
        &self.ir_write_mask
    }

    /// Frozen module-specific config view.
    pub fn config_view(&self) -> &Arc<ConfigView> {
        &self.config_view
    }

    /// Frozen `[claims].holds` from the manifest.
    pub fn claims(&self) -> &[String] {
        &self.claims
    }

    /// Module IDs this module explicitly depends on.
    pub fn requires_modules(&self) -> &[ModuleId] {
        &self.requires_modules
    }

    /// Pre-computed set of declared region-split semantic names.
    /// Empty for paint-transparent modules (the common case).
    pub fn region_split_semantics(&self) -> &std::collections::HashSet<String> {
        &self.region_split_semantics
    }
}

/// Builder for [`CompiledModuleStatic`]. The module id is the only positional
/// argument to [`CompiledModuleBuilder::new`]; the remaining fields default to
/// empty/`None` and are set via chained `Self`-consuming setters.
///
/// Wasmtime handles (`WasmInstancePool`, `WasmComponent`) are NOT part of this
/// builder; they are carried separately in `slicer-wasm-host::LiveModuleBinding`.
#[must_use = "CompiledModuleBuilder must be finalized with .build()"]
#[derive(Debug, Clone)]
pub struct CompiledModuleBuilder {
    module_id: ModuleId,
    ir_read_mask: IrAccessMask,
    ir_write_mask: IrAccessMask,
    config_view: Arc<ConfigView>,
    claims: Vec<String>,
    requires_modules: Vec<ModuleId>,
    region_split_semantics: std::collections::HashSet<String>,
}

impl CompiledModuleBuilder {
    /// Start a new builder for the given module identifier.
    pub fn new(module_id: impl Into<ModuleId>) -> Self {
        Self {
            module_id: module_id.into(),
            ir_read_mask: IrAccessMask::default(),
            ir_write_mask: IrAccessMask::default(),
            config_view: Arc::new(ConfigView::default()),
            claims: Vec::new(),
            requires_modules: Vec::new(),
            region_split_semantics: std::collections::HashSet::new(),
        }
    }

    /// Set the frozen IR read access mask.
    pub fn ir_read_mask(mut self, mask: IrAccessMask) -> Self {
        self.ir_read_mask = mask;
        self
    }

    /// Set the frozen IR write access mask.
    pub fn ir_write_mask(mut self, mask: IrAccessMask) -> Self {
        self.ir_write_mask = mask;
        self
    }

    /// Set the frozen module-specific config view.
    pub fn config_view(mut self, view: Arc<ConfigView>) -> Self {
        self.config_view = view;
        self
    }

    /// Set the manifest-declared held claim ids.
    pub fn claims(mut self, claims: Vec<String>) -> Self {
        self.claims = claims;
        self
    }

    /// Set the manifest-declared required peer modules.
    pub fn requires_modules(mut self, requires_modules: Vec<ModuleId>) -> Self {
        self.requires_modules = requires_modules;
        self
    }

    /// Set the pre-computed region-split semantic name set.
    pub fn region_split_semantics(mut self, semantics: std::collections::HashSet<String>) -> Self {
        self.region_split_semantics = semantics;
        self
    }

    /// Finalize into a [`CompiledModuleStatic`].
    pub fn build(self) -> CompiledModuleStatic {
        CompiledModuleStatic {
            module_id: self.module_id,
            ir_read_mask: self.ir_read_mask,
            ir_write_mask: self.ir_write_mask,
            config_view: self.config_view,
            claims: self.claims,
            requires_modules: self.requires_modules,
            region_split_semantics: self.region_split_semantics,
        }
    }
}

/// Minimal immutable IR access-mask representation for runtime planning.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct IrAccessMask {
    /// Declared manifest access paths.
    pub paths: Vec<String>,
}

/// One already-sorted stage bucket supplied by validation/topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortedStageModules {
    /// Canonical scheduler stage identifier.
    pub stage_id: StageId,
    /// Topologically sorted module identifiers for the stage.
    pub module_ids: Vec<ModuleId>,
}

/// One loaded module plus its config binding.
///
/// Wasmtime handles (`WasmInstancePool`, `WasmComponent`) are NOT stored here;
/// they live in `slicer-wasm-host::LiveModuleBinding` on the live path.
#[derive(Debug, Clone)]
pub struct ExecutionModuleBinding {
    /// Loaded manifest/module metadata.
    pub module: LoadedModule,
    /// Frozen config view bound for runtime execution.
    pub config_view: Arc<ConfigView>,
}

/// Immutable planning input assembled after validation and module loading.
#[derive(Debug, Clone)]
pub struct ExecutionPlanRequest {
    /// Already topologically sorted scheduler stages.
    pub sorted_stages: Vec<SortedStageModules>,
    /// Loaded modules and their runtime bindings.
    pub module_bindings: Vec<ExecutionModuleBinding>,
    /// Frozen global layer schedule.
    pub global_layers: Arc<Vec<GlobalLayer>>,
    /// Frozen per-region execution plans.
    pub region_plans: Arc<HashMap<RegionKey, RegionPlan>>,
}

/// Maximum allowed `GlobalLayer.index` value. Plans with layers at or above
/// this index are rejected per docs/02_ir_schemas.md and docs/12_architecture_gate_metrics.md.
pub const MAX_LAYER_INDEX: u32 = 100_000;

/// Default cap on `RegionMapIR` entry count per docs/04_host_scheduler.md.
pub use slicer_ir::DEFAULT_REGION_MAP_CAP;

/// Structured planning failure for immutable execution-plan assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionPlanError {
    /// A sorted stage referenced a module with no runtime binding.
    MissingModuleBinding {
        /// Stage that referenced the missing binding.
        stage_id: StageId,
        /// Missing module identifier.
        module_id: ModuleId,
    },
    /// A runtime binding declared a stage inconsistent with the sorted stage input.
    StageMismatch {
        /// Bound module identifier.
        module_id: ModuleId,
        /// Stage implied by the sorted input.
        expected_stage: StageId,
        /// Stage declared by the loaded module.
        actual_stage: StageId,
    },
    /// Multiple runtime bindings targeted the same module id.
    DuplicateModuleBinding {
        /// Duplicate module identifier.
        module_id: ModuleId,
    },
    /// A `GlobalLayer.index` exceeds the documented budget (>= 100_000).
    LayerIndexBudgetExceeded {
        /// The offending layer index.
        layer_index: u32,
        /// The configured budget cap.
        budget: u32,
    },
    /// The `RegionMapIR` entry count exceeds the configured cap.
    RegionMapCapExceeded {
        /// Computed entry count.
        entry_count: usize,
        /// Configured cap.
        cap: usize,
    },
    /// A module binding's `ConfigView` exposes a key that is not in the
    /// module's declared `[config.schema]` — a contract violation per
    /// docs/03 §host-boundary enforcement and docs/02 §pre-filtered config.
    /// Callers MUST route every per-module `ConfigView` through
    /// [`bind_module_config_view`] to avoid this error.
    UndeclaredConfigKey {
        /// Module whose `ConfigView` leaked an undeclared key.
        module_id: ModuleId,
        /// The offending undeclared key.
        key: String,
    },
}

impl std::fmt::Display for ExecutionPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingModuleBinding {
                stage_id,
                module_id,
            } => {
                write!(
                    f,
                    "stage '{stage_id}' references unknown module '{module_id}'"
                )
            }
            Self::StageMismatch {
                module_id,
                expected_stage,
                actual_stage,
            } => {
                write!(f, "module '{module_id}' declared stage '{actual_stage}' but was placed in '{expected_stage}'")
            }
            Self::DuplicateModuleBinding { module_id } => {
                write!(f, "duplicate runtime binding for module '{module_id}'")
            }
            Self::LayerIndexBudgetExceeded {
                layer_index,
                budget,
            } => {
                write!(
                    f,
                    "layer index {layer_index} exceeds budget (must be < {budget}); \
                     reduce layer count or increase layer height"
                )
            }
            Self::RegionMapCapExceeded { entry_count, cap } => {
                write!(
                    f,
                    "region map has {entry_count} entries, exceeding cap of {cap}; \
                     reduce region granularity, raise cap, or split job"
                )
            }
            Self::UndeclaredConfigKey { module_id, key } => {
                write!(
                    f,
                    "module '{module_id}' config view exposes undeclared key '{key}'; \
                     bind per-module ConfigView via bind_module_config_view() \
                     (see docs/03 §host-boundary enforcement)"
                )
            }
        }
    }
}

impl std::error::Error for ExecutionPlanError {}

/// Builds the immutable runtime execution plan.
///
/// Validates documented resource-bound contracts before assembling the plan:
/// - Every `GlobalLayer.index` must be `< 100_000` (docs/02_ir_schemas.md).
/// - `RegionMapIR` entry count must not exceed `DEFAULT_REGION_MAP_CAP` (docs/04_host_scheduler.md).
pub fn build_execution_plan(
    request: &ExecutionPlanRequest,
    diagnostics: &mut Vec<LoadDiagnostic>,
) -> Result<ExecutionPlan, ExecutionPlanError> {
    // ── Layer budget check ──────────────────────────────────────────
    for layer in request.global_layers.iter() {
        if layer.index >= MAX_LAYER_INDEX {
            return Err(ExecutionPlanError::LayerIndexBudgetExceeded {
                layer_index: layer.index,
                budget: MAX_LAYER_INDEX,
            });
        }
    }

    // ── Region map cap check ────────────────────────────────────────
    let region_count = request.region_plans.len();
    if region_count > DEFAULT_REGION_MAP_CAP {
        return Err(ExecutionPlanError::RegionMapCapExceeded {
            entry_count: region_count,
            cap: DEFAULT_REGION_MAP_CAP,
        });
    }

    let mut bindings_by_module_id = HashMap::with_capacity(request.module_bindings.len());
    for binding in &request.module_bindings {
        let module_id = binding.module.id.clone();
        // ── Declared-read guard (docs/03 §host-boundary enforcement) ──
        // Every key visible through the bound ConfigView must appear in
        // the module's declared `[config.schema]`. This is the invariant
        // upheld by `bind_module_config_view`; enforce it at plan-build
        // time so any caller bypassing the helper still fails closed.
        for key in binding.config_view.keys() {
            let support_family_module = binding
                .module
                .claims()
                .iter()
                .any(|claim| claim.starts_with("support-family:"));
            if !config_key_declared(&binding.module.config_schema.entries, &key)
                && !(support_family_module
                    && (key == SUPPORT_GENERATOR_CONFIG_KEY || key == SUPPORT_FAMILY_CONFIG_KEY))
            {
                return Err(ExecutionPlanError::UndeclaredConfigKey { module_id, key });
            }
        }
        if bindings_by_module_id
            .insert(module_id.clone(), binding)
            .is_some()
        {
            return Err(ExecutionPlanError::DuplicateModuleBinding { module_id });
        }
    }

    let mut prepass_stages = Vec::new();
    let mut per_layer_stages = Vec::new();
    let mut layer_finalization_stage = None;
    let mut postpass_stages = Vec::new();

    for sorted_stage in &request.sorted_stages {
        let mut modules = Vec::with_capacity(sorted_stage.module_ids.len());

        for module_id in &sorted_stage.module_ids {
            let binding = bindings_by_module_id.get(module_id).ok_or_else(|| {
                ExecutionPlanError::MissingModuleBinding {
                    stage_id: sorted_stage.stage_id.clone(),
                    module_id: module_id.clone(),
                }
            })?;

            if binding.module.stage != sorted_stage.stage_id {
                return Err(ExecutionPlanError::StageMismatch {
                    module_id: binding.module.id.clone(),
                    expected_stage: sorted_stage.stage_id.clone(),
                    actual_stage: binding.module.stage.clone(),
                });
            }

            modules.push(CompiledModuleStatic {
                module_id: binding.module.id.clone(),
                ir_read_mask: IrAccessMask {
                    paths: binding.module.ir_reads.clone(),
                },
                ir_write_mask: IrAccessMask {
                    paths: binding.module.ir_writes.clone(),
                },
                config_view: Arc::clone(&binding.config_view),
                claims: binding.module.claims.clone(),
                requires_modules: binding.module.requires_modules.clone(),
                region_split_semantics: binding.module.region_split_semantics.clone(),
            });
        }

        if modules.is_empty() {
            continue;
        }

        let compiled_stage = CompiledStage {
            stage_id: sorted_stage.stage_id.clone(),
            modules,
        };

        if sorted_stage.stage_id.starts_with("PrePass::") {
            prepass_stages.push(compiled_stage);
        } else if sorted_stage.stage_id.starts_with("Layer::") {
            per_layer_stages.push(compiled_stage);
        } else if sorted_stage.stage_id == "PostPass::LayerFinalization" {
            layer_finalization_stage = Some(compiled_stage);
        } else if sorted_stage.stage_id.starts_with("PostPass::") {
            postpass_stages.push(compiled_stage);
        }
    }

    // Always-on host built-in: Layer::PaintRegionAnnotation must appear in the
    // per-layer plan even when no WASM module claims it, so the host annotator
    // runs before downstream stages (Perimeters, Infill, etc.) need segment_annotations.
    let paint_stage_id = "Layer::PaintRegionAnnotation".to_string();
    if !per_layer_stages
        .iter()
        .any(|s| s.stage_id == paint_stage_id)
    {
        // Insert before the first stage in STAGE_ORDER that comes after
        // PaintRegionAnnotation (SlicePostProcess, then Perimeters, then
        // any later Layer stage).
        let insert_at = per_layer_stages
            .iter()
            .position(|s| {
                s.stage_id == "Layer::SlicePostProcess"
                    || s.stage_id == "Layer::Perimeters"
                    || s.stage_id == "Layer::PerimetersPostProcess"
                    || s.stage_id == "Layer::Infill"
                    || s.stage_id == "Layer::InfillPostProcess"
                    || s.stage_id == "Layer::Support"
                    || s.stage_id == "Layer::SupportPostProcess"
                    || s.stage_id == "Layer::PathOptimization"
            })
            .unwrap_or(per_layer_stages.len());
        per_layer_stages.insert(
            insert_at,
            CompiledStage {
                stage_id: paint_stage_id,
                modules: Vec::new(),
            },
        );
    }

    // ── Precompute module_region_index for O(1) resolve_active_regions ──
    let mut module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>> = HashMap::new();
    for layer in request.global_layers.iter() {
        for stage in &request.sorted_stages {
            if !stage.stage_id.starts_with("Layer::") {
                continue;
            }
            for module_id in &stage.module_ids {
                // Only index for modules that are actually bound
                if bindings_by_module_id.contains_key(module_id) {
                    let entry = module_region_index
                        .entry((layer.index, module_id.clone()))
                        .or_default();
                    let module = bindings_by_module_id
                        .get(module_id)
                        .expect("module binding checked above");
                    entry.extend(
                        layer
                            .active_regions
                            .iter()
                            .filter(|region| {
                                module_claims_match_active_region(module.module.claims(), region)
                            })
                            .cloned(),
                    );
                }
            }
        }
    }

    // ── Cross-manifest aggregate of [[region_split]] declarations ─────
    // Computed once at plan-build time so the host's `PrePass::RegionMapping`
    // builtin can deterministically reference module declarations without
    // re-walking the manifest set. AC-1 / packet 93.
    let modules_for_agg: Vec<LoadedModule> = request
        .module_bindings
        .iter()
        .map(|b| b.module.clone())
        .collect();
    let aggregated_region_split = aggregate_region_splits(&modules_for_agg, diagnostics);

    Ok(ExecutionPlan {
        prepass_stages,
        per_layer_stages,
        layer_finalization_stage,
        postpass_stages,
        global_layers: Arc::clone(&request.global_layers),
        region_plans: Arc::clone(&request.region_plans),
        module_region_index,
        aggregated_region_split,
    })
}

impl ExecutionPlan {
    /// O(1) lookup of active regions for a (layer, module) pair via precomputed index.
    pub fn resolve_active_regions(
        &self,
        layer: &GlobalLayer,
        module: &CompiledModuleStatic,
    ) -> &[ActiveRegion] {
        self.module_region_index
            .get(&(layer.index, module.module_id.clone()))
            .map(|v: &Vec<ActiveRegion>| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod dedup_tests {
    use std::path::PathBuf;

    use slicer_ir::SemVer;

    use super::dedup_same_claim_modules;
    use crate::manifest::{ConfigFieldEntry, LoadDiagnostic, LoadedModule, LoadedModuleBuilder};

    fn loaded(id: &str, stage: &str, holds: &[&str]) -> LoadedModule {
        LoadedModuleBuilder::new(
            id,
            SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            stage,
            slicer_schema::TIER_LAYER,
            PathBuf::from(format!("fixtures/{id}.wasm")),
        )
        .claims(holds.iter().map(|s| (*s).to_string()).collect())
        .min_host_version(SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        })
        .min_ir_schema(SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        })
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .layer_parallel_safe(true)
        .build()
    }

    #[test]
    fn same_claim_same_stage_defaults_to_classic_wall_generator_and_emits_diagnostic() {
        // Regression guard for the pre-2026-04 Benchy MVP failure mode:
        // classic-perimeters and arachne-perimeters both held
        // `perimeter-generator` in `Layer::Perimeters` and both committed
        // to the arena, producing a `LayerArenaError::SlotAlreadyOccupied`
        // masked as the generic string "arena commit failed".
        //
        // Packet 112 Step 10: with no `wall_generator` config supplied
        // (`None`), the winner must be `classic-perimeters` — the documented
        // [`DEFAULT_WALL_GENERATOR`] — NOT whichever module happens to sort
        // first alphabetically. Before this fix, `arachne-perimeters` won by
        // alphabetical accident with no way for a user's config to override
        // it, and this silently changed production behaviour the moment
        // `arachne-perimeters` became functional.
        let mut modules = vec![
            loaded(
                "com.core.classic-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.arachne-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics, None, false, None);

        assert_eq!(kept.len(), 1, "exactly one holder survives per claim");
        assert_eq!(kept[0].id, "com.core.classic-perimeters");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("perimeter-generator"));
        assert!(diagnostics[0]
            .message
            .contains("com.core.classic-perimeters"));
        assert!(diagnostics[0]
            .message
            .contains("com.core.arachne-perimeters"));
    }

    #[test]
    fn wall_generator_config_selects_arachne_despite_default_classic() {
        // Packet 112 Step 10: an explicit `wall_generator = "arachne"` must
        // flip the winner even though classic is the documented default.
        let mut modules = vec![
            loaded(
                "com.core.classic-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.arachne-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept =
            dedup_same_claim_modules(&mut modules, &mut diagnostics, Some("arachne"), false, None);

        assert_eq!(kept.len(), 1, "exactly one holder survives per claim");
        assert_eq!(kept[0].id, "com.core.arachne-perimeters");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .message
            .contains("com.core.arachne-perimeters"));
    }

    #[test]
    fn wall_generator_unrecognized_value_falls_back_to_classic_default() {
        // An unrecognized `wall_generator` string (typo, unsupported value)
        // must not panic or drop both candidates — it falls back to the
        // documented default rather than silently keeping neither.
        let mut modules = vec![
            loaded(
                "com.core.classic-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.arachne-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept =
            dedup_same_claim_modules(&mut modules, &mut diagnostics, Some("bogus"), false, None);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "com.core.classic-perimeters");
    }

    #[test]
    fn support_type_tree_selects_tree_support_holder() {
        // OrcaSlicer's `support_type` spelling `tree(auto)` — the raw value
        // the GUI's 3MF sidecar carries — must resolve the `tree` support
        // family. Support renderers are selected per region by family claim,
        // so dedup retains BOTH family holders rather than collapsing to one.
        let mut modules = vec![
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator", "support-family:tree"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(
            &mut modules,
            &mut diagnostics,
            None,
            false,
            Some("tree(auto)"),
        );

        assert_eq!(kept.len(), 2, "both family holders survive per claim");
        assert!(diagnostics.is_empty());
        assert_eq!(
            super::select_support_family(None, Some("tree(auto)")),
            "tree",
            "tree(auto) resolves the tree family"
        );
    }

    #[test]
    fn support_type_absent_defaults_to_traditional_support_holder() {
        // No `support_type` config: the traditional family is the documented
        // default. Both family holders survive dedup; the absent config
        // resolves the traditional family for a region.
        let mut modules = vec![
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator", "support-family:tree"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics, None, false, None);

        assert_eq!(kept.len(), 2, "both family holders survive per claim");
        assert!(diagnostics.is_empty());
        assert_eq!(
            super::select_support_family(None, None),
            "traditional",
            "absent support_type resolves the traditional family"
        );
    }

    #[test]
    fn support_type_normal_falls_back_to_traditional_support_holder() {
        // `normal(auto)` — and by extension `normal(manual)` — resolves the
        // traditional family explicitly. Both family holders survive dedup.
        let mut modules = vec![
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator", "support-family:tree"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(
            &mut modules,
            &mut diagnostics,
            None,
            false,
            Some("normal(auto)"),
        );

        assert_eq!(kept.len(), 2, "both family holders survive per claim");
        assert!(diagnostics.is_empty());
        assert_eq!(
            super::select_support_family(None, Some("normal(auto)")),
            "traditional",
            "normal(auto) resolves the traditional family"
        );
    }

    #[test]
    fn support_type_tree_manual_selects_tree_support_holder() {
        // Orca's manual variant (enforcer-only mode) carries the same
        // tree/normal geometry prefix; pnp has no enforcer-only concept, so
        // it resolves the same tree family as `tree(auto)`. Both family
        // holders survive dedup.
        let mut modules = vec![
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator", "support-family:tree"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(
            &mut modules,
            &mut diagnostics,
            None,
            false,
            Some("tree(manual)"),
        );

        assert_eq!(kept.len(), 2, "both family holders survive per claim");
        assert!(diagnostics.is_empty());
        assert_eq!(
            super::select_support_family(None, Some("tree(manual)")),
            "tree",
            "tree(manual) resolves the tree family"
        );
    }

    #[test]
    fn support_type_hybrid_legacy_selects_tree_support_holder() {
        // Legacy `hybrid(auto)` spellings (old OrcaSlicer files) are
        // migrated by Orca itself to `tree(auto)` at config load; a raw 3MF
        // sidecar may still carry the old spelling, so the resolver honours
        // Orca's own migration and resolves the tree family. Both family
        // holders survive dedup.
        let mut modules = vec![
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator", "support-family:tree"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(
            &mut modules,
            &mut diagnostics,
            None,
            false,
            Some("hybrid(auto)"),
        );

        assert_eq!(kept.len(), 2, "both family holders survive per claim");
        assert!(diagnostics.is_empty());
        assert_eq!(
            super::select_support_family(None, Some("hybrid(auto)")),
            "tree",
            "hybrid(auto) resolves the tree family"
        );
    }

    #[test]
    fn support_type_unrecognized_value_falls_back_to_traditional_support_holder() {
        // An unrecognized `support_type` string (typo, unsupported value)
        // must not panic or drop both candidates — it falls back to the
        // traditional family. Both family holders survive dedup.
        let mut modules = vec![
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator", "support-family:tree"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept =
            dedup_same_claim_modules(&mut modules, &mut diagnostics, None, false, Some("bogus"));

        assert_eq!(kept.len(), 2, "both family holders survive per claim");
        assert!(diagnostics.is_empty());
        assert_eq!(
            super::select_support_family(None, Some("bogus")),
            "traditional",
            "unrecognized support_type resolves the traditional family"
        );
    }

    #[test]
    fn support_type_preferred_module_not_among_candidates_keeps_alphabetical_default() {
        // A community module reusing the claim name is not in the preferred
        // set. Support renderers are selected per region by family claim, so
        // dedup retains BOTH candidates; the community module carries no
        // family claim and is therefore region-agnostic, while the core
        // traditional holder is selected for traditional regions.
        let mut modules = vec![
            loaded(
                "com.community.fancy-support",
                "Layer::Support",
                &["support-generator"],
            ),
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(
            &mut modules,
            &mut diagnostics,
            None,
            false,
            Some("tree(auto)"),
        );

        assert_eq!(
            kept.len(),
            2,
            "both support-generator candidates survive per claim"
        );
        assert!(diagnostics.is_empty());
        assert_eq!(
            super::select_support_family(None, Some("tree(auto)")),
            "tree",
            "tree(auto) resolves the tree family"
        );
    }

    #[test]
    fn different_stages_same_claim_name_do_not_collide() {
        // Claims are scoped by stage: two modules can legitimately both
        // declare the same claim name across different stages.
        let mut modules = vec![
            loaded("mod.a", "Layer::Perimeters", &["x"]),
            loaded("mod.b", "Layer::Infill", &["x"]),
        ];
        let mut diagnostics = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics, None, false, None);
        assert_eq!(kept.len(), 2);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn modules_with_no_claims_are_always_kept() {
        let mut modules = vec![
            loaded("mod.a", "Layer::Perimeters", &[]),
            loaded("mod.b", "Layer::Perimeters", &[]),
        ];
        let mut diagnostics = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics, None, false, None);
        assert_eq!(kept.len(), 2);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn bind_config_view_expands_prefix_wildcard_entries() {
        // Regression guard for planner-specific per-object config keys.
        // `layer-planner-default.toml` declares `"object_height:*"`, and
        // the bound ConfigView must preserve every matching source key
        // that was explicitly provided to the host/runtime plan builder.
        use slicer_ir::ConfigValue;
        use std::collections::HashMap;

        let mut module = loaded("planner", "PrePass::LayerPlanning", &[]);
        module.config_schema.entries.insert(
            "object_height:*".to_string(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );
        module.config_schema.entries.insert(
            "layer_height".to_string(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );

        let mut source: HashMap<String, ConfigValue> = HashMap::new();
        source.insert("object_height:abc".into(), ConfigValue::Float(48.0));
        source.insert("object_height:xyz".into(), ConfigValue::Float(12.5));
        source.insert("layer_height".into(), ConfigValue::Float(0.2));
        source.insert("unrelated_key".into(), ConfigValue::Float(1.0));

        let view = super::bind_module_config_view(&module, &source);
        let mut keys: Vec<String> = view.keys().to_vec();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "layer_height".to_string(),
                "object_height:abc".to_string(),
                "object_height:xyz".to_string(),
            ],
            "declared `object_height:*` must expand to every matching source key",
        );
    }

    #[test]
    fn config_key_declared_accepts_exact_and_wildcard() {
        use std::collections::BTreeMap;
        let mut declared: BTreeMap<String, ConfigFieldEntry> = BTreeMap::new();
        declared.insert(
            "layer_height".into(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );
        declared.insert(
            "object_height:*".into(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );

        assert!(super::config_key_declared(&declared, "layer_height"));
        assert!(super::config_key_declared(&declared, "object_height:a"));
        assert!(super::config_key_declared(
            &declared,
            "object_height:long-uuid"
        ));
        assert!(!super::config_key_declared(&declared, "object_height"));
        assert!(!super::config_key_declared(&declared, "random_key"));
    }

    #[test]
    fn canonical_benchy_core_modules_keep_all_infill_holders_under_fill_claim_dedup() {
        // Post-DEV-065: the legacy `infill-generator` claim was retired from
        // every infill manifest in favour of packet-37's four granular
        // fill-role claims (`claim:{top,bottom,bridge,sparse}-fill`). Those
        // are per-region-configurable and intentionally exempt from startup
        // dedup. The remaining non-fill claims (support-generator) keep the
        // first-winner-wins behaviour; `perimeter-generator` resolves by
        // `wall_generator` (packet 112 Step 10) — `None` here defaults to
        // classic.
        let mut modules = vec![
            loaded(
                "com.core.arachne-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.classic-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.gyroid-infill",
                "Layer::Infill",
                &["claim:sparse-fill"],
            ),
            loaded(
                "com.core.lightning-infill",
                "Layer::Infill",
                &["claim:sparse-fill"],
            ),
            loaded(
                "com.core.rectilinear-infill",
                "Layer::Infill",
                &[
                    "claim:top-fill",
                    "claim:bottom-fill",
                    "claim:bridge-fill",
                    "claim:sparse-fill",
                ],
            ),
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator", "support-family:traditional"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator", "support-family:tree"],
            ),
        ];
        let mut diagnostics = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics, None, false, None);

        let ids: Vec<&str> = kept.iter().map(|m| m.id.as_str()).collect();
        // All three infill modules survive — per-region resolution picks the
        // active holder per (object, region). Perimeters collapse to one
        // holder per stage; support renderers are selected per region by
        // family claim, so BOTH family holders survive.
        assert_eq!(
            ids,
            [
                "com.core.classic-perimeters",
                "com.core.gyroid-infill",
                "com.core.lightning-infill",
                "com.core.rectilinear-infill",
                "com.core.traditional-support",
                "com.core.tree-support",
            ]
        );
        // One drop: arachne-perimeters (loses to classic, the default
        // wall_generator). tree-support is retained alongside
        // traditional-support. Fill-role drops are NOT emitted.
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics.iter().all(|d| !d.message.contains("fill")));
    }
}
