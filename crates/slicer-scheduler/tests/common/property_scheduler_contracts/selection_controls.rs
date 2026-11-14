//! Deterministic AC-5 control for packet `proptest-openrscad_07_pbt-scheduler`:
//! configured claim-role resolution, startup claim dedup/selection and support
//! family pairing, driven through the real scheduler seams.
//!
//! Scope boundary, stated once and deliberately: this is **startup / dedup /
//! dispatch-selection** coverage. It exercises
//! `slicer_scheduler::validation::resolve_held_claims` (with
//! `slicer_scheduler::validation::FillHolders`,
//! `slicer_scheduler::validation::module_id_matches_holder` and
//! `slicer_scheduler::validation::FILL_CLAIM_IDS`),
//! `slicer_scheduler::execution_plan::dedup_same_claim_modules_with_typed_wall_generator`
//! and `slicer_scheduler::execution_plan::validate_support_family_pairing`.
//! It is **not** live per-region support aggregation: synthetic
//! `slicer_scheduler::validation::ConflictScope::Region` observations or
//! hand-built `ActiveRegion`s would not prove the runtime support-plan
//! aggregation seam, so none are used here and no such claim is made.
//! `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs`
//! owns the typed claim scope/write/disabled rules and
//! `crates/slicer-scheduler/tests/common/property_scheduler_contracts/registry_controls.rs`
//! owns the region-split registry witness.
//!
//! Nothing here reads `PNP_PROPERTY_CONTEXT`, builds a `PropertyRun` or consults
//! the property registry. The tables below are hand-computed witnesses: every
//! expected kept-module id and every expected drop diagnostic is written out,
//! so a mutation that folds case, strips a community tail, drops a fill
//! candidate, collapses a support family, ignores the typed
//! `wall_generator`/`spiral_vase` selector or aborts on an incomplete family
//! pair fails with an exact name.
//!
//! The retained sets also prove the family-specific rule explicitly:
//! per-family support planners coexist in one stage, and `support-generator`
//! renderers are never deduplicated (their selection is per region at dispatch
//! time, i.e. outside this startup pass).

#![allow(missing_docs)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use slicer_ir::{ConfigValue, SemVer};
use slicer_scheduler::validation::module_id_matches_holder;
use slicer_scheduler::{
    dedup_same_claim_modules_with_typed_wall_generator, resolve_held_claims,
    validate_support_family_pairing, DiagnosticLevel, FillHolders, LoadDiagnostic, LoadedModule,
    LoadedModuleBuilder, SupportFamilyPairingError, FILL_CLAIM_IDS, STAGE_ORDER,
};

// ── Canonical claim / stage vocabulary used by the declarative tables ───────

const TOP_FILL: &str = "claim:top-fill";
const BOTTOM_FILL: &str = "claim:bottom-fill";
const BRIDGE_FILL: &str = "claim:bridge-fill";
const SPARSE_FILL: &str = "claim:sparse-fill";
const RAFT_FILL: &str = "claim:raft-fill";

const STAGE_PERIMETERS: &str = "Layer::Perimeters";
const STAGE_INFILL: &str = "Layer::Infill";
const STAGE_SUPPORT: &str = "Layer::Support";
const STAGE_POST_PROCESS: &str = "Layer::InfillPostProcess";

/// The normative catalog from the packet's H-CLAIMS exception table. Written
/// out so a production `FILL_CLAIM_IDS` that drifts away from the spec (for
/// example by absorbing `claim:raft-fill`) is caught here; the validator
/// exception behaviour itself is owned by
/// `crates/slicer-scheduler/tests/common/property_scheduler_contracts/claims_controls.rs`.
const CONFIGURED_FILL_IDS: &[&str] = &[TOP_FILL, BOTTOM_FILL, BRIDGE_FILL, SPARSE_FILL];

// ── Shared fixture constructor ────────────────────────────────────────────

fn version() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

fn module(id: &str, stage: &str, claims: &[&str]) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        version(),
        stage,
        slicer_schema::TIER_LAYER,
        PathBuf::from(format!("{id}.wasm")),
    )
    .claims(claims.iter().map(|claim| (*claim).to_owned()).collect())
    .min_host_version(version())
    .min_ir_schema(version())
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// The typed selector channel accepts `slicer_ir::ConfigValue`; an absent
/// `wall_generator` is `None`, never `String("")`.
fn wall_generator_value(value: Option<&str>) -> Option<ConfigValue> {
    value.map(|value| ConfigValue::String(value.to_owned()))
}

// ── 1. Holder identifier matching ─────────────────────────────────────────

/// One row of the `module_id_matches_holder` contract table:
/// `module_id_matches_holder(module_id, holder) == expected`.
#[derive(Debug, Clone, Copy)]
struct HolderMatchRow {
    label: &'static str,
    module_id: &'static str,
    holder: &'static str,
    expected: bool,
}

const HOLDER_MATCH_TABLE: &[HolderMatchRow] = &[
    HolderMatchRow {
        label: "configured full module id matches itself",
        module_id: "com.core.rectilinear-infill",
        holder: "com.core.rectilinear-infill",
        expected: true,
    },
    HolderMatchRow {
        label: "com.core. short name matches the built-in module",
        module_id: "com.core.rectilinear-infill",
        holder: "rectilinear-infill",
        expected: true,
    },
    HolderMatchRow {
        label: "matching is case-sensitive (short form)",
        module_id: "com.core.rectilinear-infill",
        holder: "Rectilinear-Infill",
        expected: false,
    },
    HolderMatchRow {
        label: "a prefix of the short name is not a match",
        module_id: "com.core.rectilinear-infill",
        holder: "rectilinear",
        expected: false,
    },
    HolderMatchRow {
        label: "community bare tail must not match",
        module_id: "com.acme.rectilinear-infill",
        holder: "rectilinear-infill",
        expected: false,
    },
    HolderMatchRow {
        label: "community full module id matches itself",
        module_id: "com.acme.rectilinear-infill",
        holder: "com.acme.rectilinear-infill",
        expected: true,
    },
    HolderMatchRow {
        label: "matching is case-sensitive (community full id)",
        module_id: "com.acme.rectilinear-infill",
        holder: "com.acme.Rectilinear-Infill",
        expected: false,
    },
    HolderMatchRow {
        label: "a com.core. full id does not match another namespace",
        module_id: "com.acme.rectilinear-infill",
        holder: "com.core.rectilinear-infill",
        expected: false,
    },
    HolderMatchRow {
        label: "an empty holder matches no non-empty module id",
        module_id: "com.core.rectilinear-infill",
        holder: "",
        expected: false,
    },
];

// ── 2. `resolve_held_claims` role resolution ──────────────────────────────

/// One declarative `resolve_held_claims` row. `holders` is
/// `[top, bottom, bridge, sparse]` and is spelled out per row so each row
/// states the whole configured selector state it depends on.
#[derive(Debug, Clone, Copy)]
struct HeldClaimsRow {
    label: &'static str,
    module_id: &'static str,
    declared: &'static [&'static str],
    holders: [&'static str; 4],
    expected: &'static [&'static str],
}

const HELD_CLAIMS_TABLE: &[HeldClaimsRow] = &[
    HeldClaimsRow {
        label: "all four configured roles match full and com.core.-short ids; raft survives; non-fill claims are ignored",
        module_id: "com.core.fill-all",
        declared: &[
            TOP_FILL,
            BOTTOM_FILL,
            BRIDGE_FILL,
            SPARSE_FILL,
            RAFT_FILL,
            "infill-generator",
        ],
        holders: [
            "com.core.fill-all",
            "fill-all",
            "com.core.fill-all",
            "fill-all",
        ],
        expected: &[TOP_FILL, BOTTOM_FILL, BRIDGE_FILL, SPARSE_FILL, RAFT_FILL],
    },
    HeldClaimsRow {
        label: "a declared fill claim is not held unless the module is the configured holder",
        module_id: "com.control.fill-gyroid",
        declared: &[SPARSE_FILL],
        holders: [
            "com.core.fill-all",
            "com.core.fill-all",
            "com.core.fill-all",
            "com.core.fill-all",
        ],
        expected: &[],
    },
    HeldClaimsRow {
        label: "a community module matches its full configured id",
        module_id: "com.acme.bridge-fill",
        declared: &[BRIDGE_FILL],
        holders: [
            "com.core.fill-all",
            "com.core.fill-all",
            "com.acme.bridge-fill",
            "com.core.fill-all",
        ],
        expected: &[BRIDGE_FILL],
    },
    HeldClaimsRow {
        label: "a community module is not matched by its bare tail",
        module_id: "com.acme.bridge-fill",
        declared: &[BRIDGE_FILL],
        holders: ["", "", "bridge-fill", ""],
        expected: &[],
    },
    HeldClaimsRow {
        label: "a com.core. short configured name is not matched by a community bare tail",
        module_id: "com.acme.wave-overhangs",
        declared: &[BOTTOM_FILL],
        holders: ["", "wave-overhangs", "", ""],
        expected: &[],
    },
    HeldClaimsRow {
        label: "a com.core. short configured name matches only the com.core. module",
        module_id: "com.core.wave-overhangs",
        declared: &[BOTTOM_FILL],
        holders: ["", "wave-overhangs", "", ""],
        expected: &[BOTTOM_FILL],
    },
    HeldClaimsRow {
        label: "role matching is case-sensitive",
        module_id: "com.core.fill-all",
        declared: &[TOP_FILL],
        holders: ["Fill-All", "", "", ""],
        expected: &[],
    },
    HeldClaimsRow {
        label: "raft is held by its declaring module independently of every configured role holder",
        module_id: "com.core.raft-only",
        declared: &["infill-generator", RAFT_FILL],
        holders: [
            "com.core.fill-all",
            "com.core.fill-all",
            "com.core.fill-all",
            "com.core.fill-all",
        ],
        expected: &[RAFT_FILL],
    },
];

// ── 3. Startup dedup declarations and selector modes ──────────────────────

/// One loaded-module declaration for the controlled dedup domain. Each module
/// carries at most one contested ordinary claim, so a drop is attributable to
/// exactly one claim.
#[derive(Debug, Clone, Copy)]
struct DedupDeclaration {
    id: &'static str,
    stage: &'static str,
    claims: &'static [&'static str],
}

const DEDUP_DECLARATIONS: &[DedupDeclaration] = &[
    // Two typed `perimeter-generator` candidates in one stage.
    DedupDeclaration {
        id: "com.core.classic-perimeters",
        stage: STAGE_PERIMETERS,
        claims: &["perimeter-generator"],
    },
    DedupDeclaration {
        id: "com.core.arachne-perimeters",
        stage: STAGE_PERIMETERS,
        claims: &["perimeter-generator"],
    },
    // Fill candidates: every configured fill claim has two declarers in one
    // stage, so a dedup that forgot the fill exception would drop all four
    // losers and be visible in both the kept set and the diagnostics.
    DedupDeclaration {
        id: "com.control.alpha-fill-all",
        stage: STAGE_INFILL,
        claims: &[TOP_FILL, BOTTOM_FILL, BRIDGE_FILL, SPARSE_FILL],
    },
    DedupDeclaration {
        id: "com.control.fill-wave",
        stage: STAGE_INFILL,
        claims: &[TOP_FILL],
    },
    DedupDeclaration {
        id: "com.control.fill-bottom",
        stage: STAGE_INFILL,
        claims: &[BOTTOM_FILL],
    },
    DedupDeclaration {
        id: "com.control.fill-bridge",
        stage: STAGE_INFILL,
        claims: &[BRIDGE_FILL],
    },
    DedupDeclaration {
        id: "com.control.fill-gyroid",
        stage: STAGE_INFILL,
        claims: &[SPARSE_FILL],
    },
    // `claim:raft-fill` is ordinary for dedup: one holder is retained, and the
    // separate two-holder witness below drops the lexical loser.
    DedupDeclaration {
        id: "com.control.fill-raft",
        stage: STAGE_INFILL,
        claims: &[RAFT_FILL],
    },
    // Two contested ordinary claims, one per stage pair.
    DedupDeclaration {
        id: "com.control.alpha-writer",
        stage: STAGE_INFILL,
        claims: &["claim:ordered-write"],
    },
    DedupDeclaration {
        id: "com.control.beta-writer",
        stage: STAGE_INFILL,
        claims: &["claim:ordered-write"],
    },
    DedupDeclaration {
        id: "com.control.post-alpha",
        stage: STAGE_POST_PROCESS,
        claims: &["claim:post-order"],
    },
    DedupDeclaration {
        id: "com.control.post-beta",
        stage: STAGE_POST_PROCESS,
        claims: &["claim:post-order"],
    },
    // The same claim in two different stages is not contested: the dedup key
    // is `(stage, claim)`, so both holders survive.
    DedupDeclaration {
        id: "com.control.stage-a-writer",
        stage: STAGE_INFILL,
        claims: &["claim:cross-stage"],
    },
    DedupDeclaration {
        id: "com.control.stage-b-writer",
        stage: STAGE_POST_PROCESS,
        claims: &["claim:cross-stage"],
    },
    // Support renderers and planners: per-family planners coexist; the single
    // non-family planner stays the sole global candidate.
    DedupDeclaration {
        id: "com.core.traditional-support",
        stage: STAGE_SUPPORT,
        claims: &["support-generator", "support-family:traditional"],
    },
    DedupDeclaration {
        id: "com.core.tree-support",
        stage: STAGE_SUPPORT,
        claims: &["support-generator", "support-family:tree"],
    },
    DedupDeclaration {
        id: "com.core.traditional-support-planner",
        stage: STAGE_SUPPORT,
        claims: &["support-planner", "support-family:traditional"],
    },
    DedupDeclaration {
        id: "com.core.tree-support-planner",
        stage: STAGE_SUPPORT,
        claims: &["support-planner", "support-family:tree"],
    },
    DedupDeclaration {
        id: "com.control.generic-planner",
        stage: STAGE_SUPPORT,
        claims: &["support-planner"],
    },
];

/// One expected dropped module: the module, its stage, the single claim that
/// lost, and the lexical winner named in the diagnostic.
#[derive(Debug, Clone, Copy)]
struct DropExpectation {
    module: &'static str,
    stage: &'static str,
    claim: &'static str,
    winner: &'static str,
}

const CLASSIC_KEPT: &[&str] = &[
    "com.control.alpha-fill-all",
    "com.control.alpha-writer",
    "com.control.fill-bottom",
    "com.control.fill-bridge",
    "com.control.fill-gyroid",
    "com.control.fill-raft",
    "com.control.fill-wave",
    "com.control.generic-planner",
    "com.control.post-alpha",
    "com.control.stage-a-writer",
    "com.control.stage-b-writer",
    "com.core.classic-perimeters",
    "com.core.traditional-support",
    "com.core.traditional-support-planner",
    "com.core.tree-support",
    "com.core.tree-support-planner",
];

const ARACHNE_KEPT: &[&str] = &[
    "com.control.alpha-fill-all",
    "com.control.alpha-writer",
    "com.control.fill-bottom",
    "com.control.fill-bridge",
    "com.control.fill-gyroid",
    "com.control.fill-raft",
    "com.control.fill-wave",
    "com.control.generic-planner",
    "com.control.post-alpha",
    "com.control.stage-a-writer",
    "com.control.stage-b-writer",
    "com.core.arachne-perimeters",
    "com.core.traditional-support",
    "com.core.traditional-support-planner",
    "com.core.tree-support",
    "com.core.tree-support-planner",
];

const CLASSIC_DROPS: &[DropExpectation] = &[
    DropExpectation {
        module: "com.control.beta-writer",
        stage: STAGE_INFILL,
        claim: "claim:ordered-write",
        winner: "com.control.alpha-writer",
    },
    DropExpectation {
        module: "com.control.post-beta",
        stage: STAGE_POST_PROCESS,
        claim: "claim:post-order",
        winner: "com.control.post-alpha",
    },
    DropExpectation {
        module: "com.core.arachne-perimeters",
        stage: STAGE_PERIMETERS,
        claim: "perimeter-generator",
        winner: "com.core.classic-perimeters",
    },
];

const ARACHNE_DROPS: &[DropExpectation] = &[
    DropExpectation {
        module: "com.control.beta-writer",
        stage: STAGE_INFILL,
        claim: "claim:ordered-write",
        winner: "com.control.alpha-writer",
    },
    DropExpectation {
        module: "com.control.post-beta",
        stage: STAGE_POST_PROCESS,
        claim: "claim:post-order",
        winner: "com.control.post-alpha",
    },
    DropExpectation {
        module: "com.core.classic-perimeters",
        stage: STAGE_PERIMETERS,
        claim: "perimeter-generator",
        winner: "com.core.arachne-perimeters",
    },
];

/// One row of the typed-selector table. `support_type` is accepted by the real
/// seam and must stay inert for support claims (packet 221 moved family
/// selection to dispatch time).
#[derive(Debug, Clone, Copy)]
struct DedupMode {
    label: &'static str,
    wall_generator: Option<&'static str>,
    spiral_vase: bool,
    support_type: Option<&'static str>,
    expected_kept: &'static [&'static str],
    expected_drops: &'static [DropExpectation],
}

const DEDUP_MODES: &[DedupMode] = &[
    DedupMode {
        label: "typed wall_generator = classic",
        wall_generator: Some("classic"),
        spiral_vase: false,
        support_type: None,
        expected_kept: CLASSIC_KEPT,
        expected_drops: CLASSIC_DROPS,
    },
    DedupMode {
        label: "typed wall_generator = arachne",
        wall_generator: Some("arachne"),
        spiral_vase: false,
        support_type: None,
        expected_kept: ARACHNE_KEPT,
        expected_drops: ARACHNE_DROPS,
    },
    DedupMode {
        label: "absent wall_generator falls back to classic",
        wall_generator: None,
        spiral_vase: false,
        support_type: None,
        expected_kept: CLASSIC_KEPT,
        expected_drops: CLASSIC_DROPS,
    },
    DedupMode {
        label: "unrecognized wall_generator falls back to classic",
        wall_generator: Some("banana"),
        spiral_vase: false,
        support_type: None,
        expected_kept: CLASSIC_KEPT,
        expected_drops: CLASSIC_DROPS,
    },
    DedupMode {
        label: "spiral_vase forces classic over arachne",
        wall_generator: Some("arachne"),
        spiral_vase: true,
        support_type: None,
        expected_kept: CLASSIC_KEPT,
        expected_drops: CLASSIC_DROPS,
    },
    DedupMode {
        label: "support_type stays inert for startup claim dedup",
        wall_generator: Some("classic"),
        spiral_vase: false,
        support_type: Some("tree(auto)"),
        expected_kept: CLASSIC_KEPT,
        expected_drops: CLASSIC_DROPS,
    },
];

/// Structured view of one drop diagnostic, compared exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DropDiagnosticView {
    level: DiagnosticLevel,
    field: Option<String>,
    path: PathBuf,
    message: String,
}

fn diagnostic_views(diagnostics: &[LoadDiagnostic]) -> Vec<DropDiagnosticView> {
    diagnostics
        .iter()
        .map(|diagnostic| DropDiagnosticView {
            level: diagnostic.level,
            field: diagnostic.field.clone(),
            path: diagnostic.path.clone(),
            message: diagnostic.message.clone(),
        })
        .collect()
}

fn expected_diagnostic_views(drops: &[DropExpectation]) -> Vec<DropDiagnosticView> {
    drops
        .iter()
        .map(|drop| DropDiagnosticView {
            level: DiagnosticLevel::Info,
            field: Some(String::from("claims.holds")),
            path: PathBuf::from(format!("{}.wasm", drop.module)),
            message: format!(
                "module '{}' in stage '{}' dropped: claim '{}' already held by '{}' \
                 (first-winner dedup; docs/04 §2)",
                drop.module, drop.stage, drop.claim, drop.winner
            ),
        })
        .collect()
}

// ── 4. Support family pairing table ───────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct PairingExpectation {
    missing_renderers: &'static [&'static str],
    missing_planners: &'static [&'static str],
}

/// One declarative pairing row. Modules are `(id, claims)` pairs; the stage is
/// the same canonical `Layer::Support` stage for every row because
/// `validate_support_family_pairing` is stage-independent by contract.
#[derive(Debug, Clone, Copy)]
struct PairingCase {
    label: &'static str,
    modules: &'static [(&'static str, &'static [&'static str])],
    expected: &'static [PairingExpectation],
}

const PAIRING_CASES: &[PairingCase] = &[
    PairingCase {
        label: "complete traditional + tree planner/renderer pairs",
        modules: &[
            (
                "com.core.traditional-support-planner",
                &["support-planner", "support-family:traditional"],
            ),
            (
                "com.core.traditional-support",
                &["support-generator", "support-family:traditional"],
            ),
            (
                "com.core.tree-support-planner",
                &["support-planner", "support-family:tree"],
            ),
            (
                "com.core.tree-support",
                &["support-generator", "support-family:tree"],
            ),
        ],
        expected: &[],
    },
    PairingCase {
        label: "planner-only families are reported in sorted order",
        modules: &[
            (
                "com.control.zeta-planner",
                &["support-planner", "support-family:zeta"],
            ),
            (
                "com.control.alpha-planner",
                &["support-planner", "support-family:alpha"],
            ),
        ],
        expected: &[PairingExpectation {
            missing_renderers: &["alpha", "zeta"],
            missing_planners: &[],
        }],
    },
    PairingCase {
        label: "renderer-only families are reported in sorted order",
        modules: &[
            (
                "com.control.mango-renderer",
                &["support-generator", "support-family:mango"],
            ),
            (
                "com.control.beta-renderer",
                &["support-generator", "support-family:beta"],
            ),
        ],
        expected: &[PairingExpectation {
            missing_renderers: &[],
            missing_planners: &["beta", "mango"],
        }],
    },
    PairingCase {
        label: "mismatched planner and renderer families report both sides",
        modules: &[
            (
                "com.control.planner-family-planner",
                &["support-planner", "support-family:planner-family"],
            ),
            (
                "com.control.renderer-family-renderer",
                &["support-generator", "support-family:renderer-family"],
            ),
        ],
        expected: &[PairingExpectation {
            missing_renderers: &["planner-family"],
            missing_planners: &["renderer-family"],
        }],
    },
    PairingCase {
        label: "a support-family claim on neither role is inert",
        modules: &[
            (
                "com.core.traditional-support-planner",
                &["support-planner", "support-family:traditional"],
            ),
            (
                "com.core.traditional-support",
                &["support-generator", "support-family:traditional"],
            ),
            ("com.control.orphan-family", &["support-family:orphan"]),
        ],
        expected: &[],
    },
];

// ── Fixture premises ──────────────────────────────────────────────────────

/// Guards the dedup table's populations: every retention claim is meaningful
/// only if a competing candidate exists, and the cross-stage row is meaningful
/// only if the two modules really are in different stages.
fn assert_dedup_premises() {
    assert!(
        !DEDUP_DECLARATIONS.is_empty(),
        "the dedup domain must not be empty"
    );
    for stage in [
        STAGE_PERIMETERS,
        STAGE_INFILL,
        STAGE_SUPPORT,
        STAGE_POST_PROCESS,
    ] {
        assert!(
            STAGE_ORDER.contains(&stage),
            "dedup stage {stage} must be a canonical STAGE_ORDER stage"
        );
    }

    for fill in CONFIGURED_FILL_IDS {
        let declarers: Vec<&DedupDeclaration> = DEDUP_DECLARATIONS
            .iter()
            .filter(|declaration| declaration.claims.contains(fill))
            .collect();
        assert!(
            declarers.len() >= 2,
            "fill retention for {fill} needs at least two declarers, found {declarers:?}"
        );
        let stages: BTreeSet<&str> = declarers
            .iter()
            .map(|declaration| declaration.stage)
            .collect();
        assert_eq!(
            stages.len(),
            1,
            "the {fill} candidates must share one stage so the claim is contested"
        );
    }

    let renderers: Vec<&DedupDeclaration> = DEDUP_DECLARATIONS
        .iter()
        .filter(|declaration| declaration.claims.contains(&"support-generator"))
        .collect();
    assert!(
        renderers.len() >= 2,
        "the support-generator retention witness needs at least two renderers"
    );
    assert_eq!(
        renderers
            .iter()
            .map(|declaration| declaration.stage)
            .collect::<BTreeSet<_>>()
            .len(),
        1,
        "the renderers must share one stage so a dedup would have a contest"
    );

    let family_planners: Vec<&DedupDeclaration> = DEDUP_DECLARATIONS
        .iter()
        .filter(|declaration| {
            declaration.claims.contains(&"support-planner")
                && declaration
                    .claims
                    .iter()
                    .any(|claim| claim.starts_with("support-family:"))
        })
        .collect();
    assert!(
        family_planners.len() >= 2,
        "the per-family planner retention witness needs at least two planners"
    );
    assert_eq!(
        family_planners
            .iter()
            .map(|declaration| declaration.stage)
            .collect::<BTreeSet<_>>()
            .len(),
        1,
        "both family planners must share one stage to prove per-family coexistence"
    );

    let cross_stage: Vec<&DedupDeclaration> = DEDUP_DECLARATIONS
        .iter()
        .filter(|declaration| declaration.claims.contains(&"claim:cross-stage"))
        .collect();
    assert_eq!(
        cross_stage.len(),
        2,
        "the cross-stage witness needs exactly two holders"
    );
    assert_ne!(
        cross_stage[0].stage, cross_stage[1].stage,
        "the cross-stage witness holders must be in different stages"
    );
}

/// Explicitly re-states the family-specific retention rule against the actual
/// kept set, derived from the declaration table rather than a second roster.
fn assert_retention_rule(kept: &BTreeSet<String>) {
    let mut fill_candidates = 0usize;
    let mut renderers = 0usize;
    let mut family_planners = 0usize;

    for declaration in DEDUP_DECLARATIONS {
        if declaration
            .claims
            .iter()
            .any(|claim| FILL_CLAIM_IDS.contains(claim))
        {
            fill_candidates += 1;
            assert!(
                kept.contains(declaration.id),
                "configured fill candidate {} must survive startup dedup; multiple modules \
                 may declare the same fill claim and the per-region holder resolver picks \
                 the active one",
                declaration.id
            );
        }
        if declaration.claims.contains(&"support-generator") {
            renderers += 1;
            assert!(
                kept.contains(declaration.id),
                "support-generator renderer {} must never be deduplicated; renderer selection \
                 is per region at dispatch time",
                declaration.id
            );
        }
        if declaration.claims.contains(&"support-planner")
            && declaration
                .claims
                .iter()
                .any(|claim| claim.starts_with("support-family:"))
        {
            family_planners += 1;
            assert!(
                kept.contains(declaration.id),
                "family-scoped support planner {} must coexist with planners of other \
                 families",
                declaration.id
            );
        }
    }

    assert!(
        fill_candidates >= 2,
        "the fill retention rule must be checked against a nonempty population"
    );
    assert!(
        renderers >= 2,
        "the renderer retention rule must be checked against at least two renderers"
    );
    assert!(
        family_planners >= 2,
        "the planner retention rule must be checked against at least two families"
    );
}

// ── The deterministic AC-5 control ────────────────────────────────────────

#[test]
fn scheduler_claim_selection_and_pairing_controls() {
    // ── Configured fill catalog: exactly the four documented ids. ─────────
    let mut production_fill_ids: Vec<&str> = FILL_CLAIM_IDS.to_vec();
    production_fill_ids.sort_unstable();
    let mut documented_fill_ids: Vec<&str> = CONFIGURED_FILL_IDS.to_vec();
    documented_fill_ids.sort_unstable();
    assert_eq!(
        production_fill_ids, documented_fill_ids,
        "FILL_CLAIM_IDS must remain exactly the four configured fill ids; \
         claim:raft-fill is deliberately absent"
    );
    assert!(
        !production_fill_ids.contains(&RAFT_FILL),
        "claim:raft-fill is an ordinary manifest-declared claim, never a configured fill role"
    );

    // ── 1a. Holder identifier matching, including case sensitivity. ──────
    assert!(
        !HOLDER_MATCH_TABLE.is_empty(),
        "the holder-match table must not be empty"
    );
    for row in HOLDER_MATCH_TABLE {
        assert_eq!(
            module_id_matches_holder(row.module_id, row.holder),
            row.expected,
            "holder row `{}`: module_id_matches_holder({:?}, {:?})",
            row.label,
            row.module_id,
            row.holder
        );
    }

    // ── 1b. `resolve_held_claims` against a `FillHolders` record. ────────
    assert!(
        !HELD_CLAIMS_TABLE.is_empty(),
        "the held-claims table must not be empty"
    );
    for row in HELD_CLAIMS_TABLE {
        let holders = FillHolders {
            top: row.holders[0],
            bottom: row.holders[1],
            bridge: row.holders[2],
            sparse: row.holders[3],
        };
        let declared: Vec<String> = strings(row.declared);
        assert_eq!(
            resolve_held_claims(row.module_id, &declared, &holders),
            strings(row.expected),
            "held-claims row `{}` for module {}",
            row.label,
            row.module_id
        );
    }

    // ── 2-3. Startup dedup through the typed selector seam. ──────────────
    assert_dedup_premises();

    for mode in DEDUP_MODES {
        let mut modules: Vec<LoadedModule> = DEDUP_DECLARATIONS
            .iter()
            .map(|declaration| module(declaration.id, declaration.stage, declaration.claims))
            .collect();
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let wall_generator = wall_generator_value(mode.wall_generator);
        let kept = dedup_same_claim_modules_with_typed_wall_generator(
            &mut modules,
            &mut diagnostics,
            wall_generator.as_ref(),
            mode.spiral_vase,
            mode.support_type,
        );

        let actual_kept: Vec<String> = kept.iter().map(|module| module.id().to_owned()).collect();
        assert_eq!(
            actual_kept,
            strings(mode.expected_kept),
            "dedup mode `{}` must keep exactly the expected module sequence",
            mode.label
        );
        let kept_set: BTreeSet<String> = actual_kept.iter().cloned().collect();
        assert_retention_rule(&kept_set);

        let actual_diagnostics = diagnostic_views(&diagnostics);
        assert_eq!(
            actual_diagnostics,
            expected_diagnostic_views(mode.expected_drops),
            "dedup mode `{}` must emit exactly one Info/claims.holds drop diagnostic per \
             dropped module, naming the claim and winner",
            mode.label
        );
        assert_eq!(
            DEDUP_DECLARATIONS.len(),
            actual_kept.len() + actual_diagnostics.len(),
            "dedup mode `{}` must account for every declaration as either kept or dropped \
             with exactly one diagnostic",
            mode.label
        );
    }

    // ── 3. Preferred perimeter module absent: alphabetical fallback. ─────
    let mut boundary_modules = vec![
        module(
            "com.acme.perimeter-alpha",
            STAGE_PERIMETERS,
            &["perimeter-generator"],
        ),
        module(
            "com.acme.perimeter-beta",
            STAGE_PERIMETERS,
            &["perimeter-generator"],
        ),
    ];
    assert!(
        !boundary_modules
            .iter()
            .any(|module| module.id() == "com.core.arachne-perimeters"),
        "the requested preferred module must genuinely be absent from the fallback fixture"
    );
    let mut boundary_diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let arachne = ConfigValue::String(String::from("arachne"));
    let boundary_kept = dedup_same_claim_modules_with_typed_wall_generator(
        &mut boundary_modules,
        &mut boundary_diagnostics,
        Some(&arachne),
        false,
        None,
    );
    assert_eq!(
        boundary_kept
            .iter()
            .map(|module| module.id().to_owned())
            .collect::<Vec<_>>(),
        vec![String::from("com.acme.perimeter-alpha")],
        "when the preferred perimeter module is not a candidate the lexical first candidate wins"
    );
    assert_eq!(
        diagnostic_views(&boundary_diagnostics),
        expected_diagnostic_views(&[DropExpectation {
            module: "com.acme.perimeter-beta",
            stage: STAGE_PERIMETERS,
            claim: "perimeter-generator",
            winner: "com.acme.perimeter-alpha",
        }]),
        "the alphabetical fallback must report the same named diagnostic"
    );

    // ── 2b. `claim:raft-fill` is ordinary for dedup too. ─────────────────
    let mut raft_modules = vec![
        module("com.control.raft-alpha", STAGE_INFILL, &[RAFT_FILL]),
        module("com.control.raft-beta", STAGE_INFILL, &[RAFT_FILL]),
    ];
    let mut raft_diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let raft_kept = dedup_same_claim_modules_with_typed_wall_generator(
        &mut raft_modules,
        &mut raft_diagnostics,
        None,
        false,
        None,
    );
    assert_eq!(
        raft_kept
            .iter()
            .map(|module| module.id().to_owned())
            .collect::<Vec<_>>(),
        vec![String::from("com.control.raft-alpha")],
        "claim:raft-fill must be deduplicated as an ordinary claim, not retained as fill"
    );
    assert_eq!(
        diagnostic_views(&raft_diagnostics),
        expected_diagnostic_views(&[DropExpectation {
            module: "com.control.raft-beta",
            stage: STAGE_INFILL,
            claim: RAFT_FILL,
            winner: "com.control.raft-alpha",
        }]),
        "the ordinary raft drop must name the claim and lexical winner"
    );

    // ── 4. Support family pairing warns but never aborts. ────────────────
    assert!(
        !PAIRING_CASES.is_empty(),
        "the pairing table must not be empty"
    );
    for case in PAIRING_CASES {
        assert!(
            !case.modules.is_empty(),
            "pairing case `{}` must declare modules",
            case.label
        );
        let modules: Vec<LoadedModule> = case
            .modules
            .iter()
            .map(|(id, claims)| module(id, STAGE_SUPPORT, claims))
            .collect();
        let warnings = validate_support_family_pairing(&modules).unwrap_or_else(|error| {
            panic!(
                "pairing case `{}` must warn rather than abort, got {error:?}",
                case.label
            )
        });
        let expected: Vec<SupportFamilyPairingError> = case
            .expected
            .iter()
            .map(|expectation| SupportFamilyPairingError {
                missing_renderers: strings(expectation.missing_renderers),
                missing_planners: strings(expectation.missing_planners),
            })
            .collect();
        assert_eq!(
            warnings, expected,
            "pairing case `{}` must report the exact sorted missing-renderer/planner lists",
            case.label
        );
    }
}
