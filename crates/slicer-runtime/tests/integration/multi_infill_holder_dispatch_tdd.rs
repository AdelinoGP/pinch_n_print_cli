//! Regression: multi-module infill dispatch.
//!
//! Pre-fix: `dedup_same_claim_modules` (`execution_plan.rs:203-246`) dropped
//! rectilinear-infill and lightning-infill at startup because all three infill
//! modules declared the legacy `infill-generator` claim and only the
//! alphabetical first-winner survived. This made the user-expected
//! configurations impossible:
//! - "rectilinear for top/bottom + gyroid for sparse" — needed two modules loaded.
//! - "lightning sparse on object A + rectilinear sparse on object B" — needed
//!   per-region holder selection.
//!
//! Post-fix:
//! - A1: `infill-generator` retired from rectilinear, gyroid, lightning manifests.
//! - A2: `module_id_matches_holder` helper accepts both full module IDs and
//!   short names in `*_fill_holder` config keys.
//! - A3: top-surface-ironing declares `reads = ["InfillIR"]` to order itself
//!   after infill modules (eliminates the previously-advisory WriteConflict).
//!
//! See `docs/specs/infill-fill-partition-plan.md` Phase A and DEV-065.

use slicer_runtime::validation::{
    module_id_matches_holder, resolve_held_claims, FillHolders, FILL_CLAIM_IDS,
};

// ── AC-2: full module ID matches in resolve_held_claims ──────────────────────

#[test]
fn ac2_full_module_id_resolves_held_claims_per_role() {
    let holders = FillHolders {
        top: "com.core.rectilinear-infill",
        bottom: "com.core.rectilinear-infill",
        bridge: "com.core.rectilinear-infill",
        sparse: "com.core.gyroid-infill",
    };

    let rectilinear_declared: Vec<String> = vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ];
    let gyroid_declared: Vec<String> = vec!["claim:sparse-fill".into()];
    let lightning_declared: Vec<String> = vec!["claim:sparse-fill".into()];

    let rectilinear = resolve_held_claims(
        "com.core.rectilinear-infill",
        &rectilinear_declared,
        &holders,
    );
    let gyroid = resolve_held_claims("com.core.gyroid-infill", &gyroid_declared, &holders);
    let lightning = resolve_held_claims("com.core.lightning-infill", &lightning_declared, &holders);

    assert_eq!(
        rectilinear,
        vec![
            String::from("claim:top-fill"),
            String::from("claim:bottom-fill"),
            String::from("claim:bridge-fill"),
        ],
        "rectilinear configured for top/bottom/bridge but NOT sparse — must hold three"
    );
    assert_eq!(
        gyroid,
        vec![String::from("claim:sparse-fill")],
        "gyroid configured for sparse — must hold only sparse"
    );
    assert!(
        lightning.is_empty(),
        "lightning not configured as any holder → empty held set; got {lightning:?}"
    );
}

// ── AC-3: short name matches in resolve_held_claims ──────────────────────────

#[test]
fn ac3_short_name_resolves_identically_to_full_id() {
    // Same scenario as AC-2 but with short-name config values. The
    // `module_id_matches_holder` helper strips the `com.core.` namespace
    // before comparison; outcome must be byte-identical to AC-2.
    let holders = FillHolders {
        top: "rectilinear-infill",
        bottom: "rectilinear-infill",
        bridge: "rectilinear-infill",
        sparse: "gyroid-infill",
    };

    let rectilinear_declared: Vec<String> = vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ];
    let gyroid_declared: Vec<String> = vec!["claim:sparse-fill".into()];

    let rectilinear = resolve_held_claims(
        "com.core.rectilinear-infill",
        &rectilinear_declared,
        &holders,
    );
    let gyroid = resolve_held_claims("com.core.gyroid-infill", &gyroid_declared, &holders);

    assert_eq!(
        rectilinear,
        vec![
            String::from("claim:top-fill"),
            String::from("claim:bottom-fill"),
            String::from("claim:bridge-fill"),
        ],
        "short-name config must resolve identically to full-ID config (AC-2)"
    );
    assert_eq!(
        gyroid,
        vec![String::from("claim:sparse-fill")],
        "short-name config must resolve identically to full-ID config (AC-2)"
    );
}

// ── AC-2/3: holder matcher accepts both forms ────────────────────────────────

#[test]
fn module_id_matcher_accepts_full_and_short_names_for_com_core() {
    // Full match
    assert!(module_id_matches_holder(
        "com.core.rectilinear-infill",
        "com.core.rectilinear-infill"
    ));
    // Short match — strip com.core. prefix
    assert!(module_id_matches_holder(
        "com.core.rectilinear-infill",
        "rectilinear-infill"
    ));
    // No spurious cross-matches
    assert!(!module_id_matches_holder(
        "com.core.rectilinear-infill",
        "gyroid-infill"
    ));
    // Community namespace: only full match works (no canonical short form)
    assert!(module_id_matches_holder(
        "com.acme.fancy-infill",
        "com.acme.fancy-infill"
    ));
    assert!(!module_id_matches_holder(
        "com.acme.fancy-infill",
        "fancy-infill"
    ));
    // Empty holder doesn't accidentally match anything
    assert!(!module_id_matches_holder("com.core.rectilinear-infill", ""));
}

// ── NEG-1: unknown holder produces empty held set ────────────────────────────

#[test]
fn neg1_unknown_holder_yields_empty_held_claims_for_every_module() {
    let holders = FillHolders {
        top: "com.core.does-not-exist",
        bottom: "com.core.does-not-exist",
        bridge: "com.core.does-not-exist",
        sparse: "com.core.does-not-exist",
    };

    let rectilinear_declared: Vec<String> = vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ];

    let rectilinear = resolve_held_claims(
        "com.core.rectilinear-infill",
        &rectilinear_declared,
        &holders,
    );

    assert!(
        rectilinear.is_empty(),
        "unknown holder must produce empty held-claims set (graceful degradation: \
         module loads, runs, but emits no paths via should_emit gate); got {rectilinear:?}"
    );
}

// ── Conventions: FILL_CLAIM_IDS is the canonical four ────────────────────────

#[test]
fn fill_claim_ids_array_contains_exactly_the_four_packet_37_claims() {
    let expected: Vec<&str> = vec![
        "claim:top-fill",
        "claim:bottom-fill",
        "claim:bridge-fill",
        "claim:sparse-fill",
    ];
    assert_eq!(
        FILL_CLAIM_IDS,
        &expected[..],
        "FILL_CLAIM_IDS must contain exactly the four packet-37 fill-role claims"
    );
}

// ── AC-1: in-tree infill manifests no longer declare infill-generator ────────

#[test]
fn ac1_in_tree_infill_manifests_no_longer_declare_legacy_infill_generator() {
    // Static check on the manifest TOML files. If a future packet re-adds
    // `infill-generator` to any of these manifests, `dedup_same_claim_modules`
    // will silently drop two of the three modules again — exactly the bug A1
    // closed.
    use std::path::PathBuf;
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    for module in &["rectilinear-infill", "gyroid-infill", "lightning-infill"] {
        let path = workspace_root
            .join("modules/core-modules")
            .join(module)
            .join(format!("{module}.toml"));
        let toml = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        // Look for an UNcommented declaration of the legacy claim. Comments
        // referencing the deprecation are fine — and expected.
        for (idx, line) in toml.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            assert!(
                !trimmed.contains("\"infill-generator\""),
                "{module}.toml:{} declares the retired `infill-generator` claim — \
                 see DEV-065 (2026-06-09) and `docs/specs/infill-fill-partition-plan.md` Phase A1. \
                 Line: {line:?}",
                idx + 1,
            );
        }
    }
}
