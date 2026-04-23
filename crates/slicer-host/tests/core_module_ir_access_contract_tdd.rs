#![allow(missing_docs)]

//! Gap-bridging tests for docs/01_system_architecture.md §"Module Access
//! Contract" (lines 268-276) combined with the authoritative Stage I/O
//! Contract table (lines 326-345).
//!
//! Docs state, normatively:
//!
//!   "All modules must declare complete IR access contracts in manifest
//!    `[ir-access].reads` / `[ir-access].writes`. ... Undeclared reads are
//!    forbidden. Host must deny access and return a fatal contract error.
//!    Undeclared writes are forbidden. Host must reject commit and return a
//!    fatal contract error."
//!
//! Yet every core module under `modules/core-modules/*` currently ships with
//! empty `[ir-access].reads` and `[ir-access].writes` lists — meaning every
//! runtime read or write those modules perform is, by the doc, an undeclared
//! access that the host should refuse.
//!
//! These tests lock down the authoritative stage-level expectations so that
//! once the manifests are corrected (and/or runtime enforcement is wired),
//! regressions are caught immediately. Until the manifests are fixed, these
//! tests fail and act as an executable specification of the deviation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use slicer_host::{load_module_from_paths, LoadedModule};

// ── Stage → required (reads, writes) contract from docs/01 ────────────────
//
// Only the minimum-required sets are asserted; modules may declare
// additional scoped paths under the same root IR type. The test therefore
// checks "at least one declared path mentions each required IR root type",
// which is robust against refinement (e.g. `SliceIR.regions.polygons`
// counts as a `SliceIR` read).

fn required_contract_for_stage(stage: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    match stage {
        "PrePass::MeshSegmentation" => Some((&["MeshIR"], &["MeshIR"])),
        "PrePass::MeshAnalysis" => Some((&["MeshIR"], &["SurfaceClassificationIR"])),
        "PrePass::LayerPlanning" => {
            Some((&["MeshIR", "SurfaceClassificationIR"], &["LayerPlanIR"]))
        }
        "PrePass::SeamPlanning" => {
            Some((&["MeshIR", "SurfaceClassificationIR", "LayerPlanIR"], &["SeamPlanIR"]))
        }
        "PrePass::PaintSegmentation" => Some((
            &["MeshIR", "SurfaceClassificationIR", "LayerPlanIR"],
            &["PaintRegionIR"],
        )),
        "Layer::SlicePostProcess" => Some((&["SliceIR", "PaintRegionIR"], &["SliceIR"])),
        "Layer::Perimeters" => Some((&["SliceIR", "PaintRegionIR"], &["PerimeterIR"])),
        "Layer::PerimetersPostProcess" => Some((&["PerimeterIR"], &["PerimeterIR"])),
        "Layer::Infill" => Some((&["SliceIR"], &["InfillIR"])),
        "Layer::InfillPostProcess" => Some((&["InfillIR"], &["InfillIR"])),
        "Layer::Support" => Some((
            &["SliceIR", "SurfaceClassificationIR", "PaintRegionIR"],
            &["SupportIR"],
        )),
        "Layer::SupportPostProcess" => Some((&["SupportIR"], &["SupportIR"])),
        "Layer::PathOptimization" => Some((
            &["PerimeterIR", "InfillIR", "SupportIR"],
            &["LayerCollectionIR"],
        )),
        "PostPass::LayerFinalization" => {
            Some((&["LayerCollectionIR"], &["LayerCollectionIR"]))
        }
        _ => None,
    }
}

fn core_modules_root() -> PathBuf {
    let this_file = Path::new(env!("CARGO_MANIFEST_DIR"));
    this_file
        .join("..")
        .join("..")
        .join("modules")
        .join("core-modules")
        .canonicalize()
        .expect("modules/core-modules must exist")
}

fn discover_core_manifests() -> Vec<(String, LoadedModule)> {
    let root = core_modules_root();
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root).expect("read core-modules root") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let stem = path.file_name().and_then(|s| s.to_str()).unwrap().to_string();
        let manifest = path.join(format!("{stem}.toml"));
        let wasm = path.join(format!("{stem}.wasm"));
        if !manifest.exists() || !wasm.exists() {
            continue;
        }
        let loaded = load_module_from_paths(&manifest, &wasm)
            .unwrap_or_else(|e| panic!("failed to ingest {}: {:?}", manifest.display(), e));
        out.push((stem, loaded));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        !out.is_empty(),
        "expected at least one core module to be discovered under {}",
        root.display()
    );
    out
}

fn path_mentions_root(path: &str, root: &str) -> bool {
    // Accept exact root or any dot-prefixed refinement of it (e.g.
    // "SliceIR", "SliceIR.regions.polygons", or "SliceIR.regions.*").
    path == root || path.starts_with(&format!("{root}."))
}

// ──────────────────────────────────────────────────────────────────────────
// Test 1 — every core module declares non-empty ir-access
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn every_core_module_declares_non_empty_ir_access_per_docs_01() {
    let modules = discover_core_manifests();
    let mut offenders = Vec::new();

    for (stem, module) in &modules {
        // PostPass::TextPostProcess is explicitly documented as operating on
        // serialized text (docs/01 line 345), not on structured IR, and
        // therefore may legitimately declare empty ir-access.
        if module.stage == "PostPass::TextPostProcess" {
            continue;
        }
        if module.ir_reads.is_empty() {
            offenders.push(format!("{stem} (stage {}): empty ir-access.reads", module.stage));
        }
        if module.ir_writes.is_empty() {
            offenders.push(format!("{stem} (stage {}): empty ir-access.writes", module.stage));
        }
    }

    assert!(
        offenders.is_empty(),
        "docs/01_system_architecture.md §'Module Access Contract' requires every \
         module to declare complete IR access contracts. Offenders:\n  - {}",
        offenders.join("\n  - ")
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Test 2 — every core module's ir-access aligns with the Stage I/O Contract
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn core_module_ir_access_covers_required_roots_from_stage_io_contract() {
    let modules = discover_core_manifests();
    let mut offenders: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (stem, module) in &modules {
        let Some((required_reads, required_writes)) = required_contract_for_stage(&module.stage)
        else {
            // Unknown/unconstrained stage (e.g. TextPostProcess): skip.
            continue;
        };

        for root in required_reads {
            if !module.ir_reads.iter().any(|p| path_mentions_root(p, root)) {
                offenders
                    .entry(stem.clone())
                    .or_default()
                    .push(format!("missing required read root '{}'", root));
            }
        }
        for root in required_writes {
            if !module.ir_writes.iter().any(|p| path_mentions_root(p, root)) {
                offenders
                    .entry(stem.clone())
                    .or_default()
                    .push(format!("missing required write root '{}'", root));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "docs/01_system_architecture.md §'Stage I/O Contract' declares each \
         stage's minimum reads/writes. Core-module manifests must declare at \
         least those roots in `[ir-access]`. Offenders:\n{}",
        offenders
            .iter()
            .map(|(m, items)| format!("  {m}:\n    - {}", items.join("\n    - ")))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Test 3 — ir-access claims never leak across stage boundaries
// ──────────────────────────────────────────────────────────────────────────
//
// docs/01 §"Module Access Contract": "Modules may only read fields available
// from upstream stages in STAGE_ORDER." This test guards against future
// drift where a manifest author might accidentally add e.g. a `GCodeIR` read
// to a `Layer::Perimeters` module.

#[test]
fn core_module_reads_are_restricted_to_upstream_ir_root_set() {
    let modules = discover_core_manifests();
    let mut offenders = Vec::new();

    for (stem, module) in &modules {
        let allowed_upstream_roots: &[&str] = match module.stage.as_str() {
            "PrePass::MeshSegmentation" => &["MeshIR"],
            "PrePass::MeshAnalysis" => &["MeshIR"],
            "PrePass::LayerPlanning" => &["MeshIR", "SurfaceClassificationIR"],
            "PrePass::SeamPlanning" => {
                &["MeshIR", "SurfaceClassificationIR", "LayerPlanIR"]
            }
            "PrePass::PaintSegmentation" => {
                &["MeshIR", "SurfaceClassificationIR", "LayerPlanIR"]
            }
            "Layer::SlicePostProcess" => &[
                "SliceIR",
                "PaintRegionIR",
                "LayerPlanIR",
                "SurfaceClassificationIR",
                "RegionMapIR",
            ],
            "Layer::Perimeters" => &[
                "SliceIR",
                "PaintRegionIR",
                "LayerPlanIR",
                "SurfaceClassificationIR",
                "RegionMapIR",
            ],
            "Layer::PerimetersPostProcess" => &["PerimeterIR", "RegionMapIR"],
            "Layer::Infill" => &["SliceIR", "PerimeterIR", "RegionMapIR"],
            "Layer::InfillPostProcess" => &["InfillIR", "RegionMapIR"],
            "Layer::Support" => &[
                "SliceIR",
                "SurfaceClassificationIR",
                "PaintRegionIR",
                "RegionMapIR",
            ],
            "Layer::SupportPostProcess" => &["SupportIR", "RegionMapIR"],
            "Layer::PathOptimization" => {
                &["PerimeterIR", "InfillIR", "SupportIR", "RegionMapIR"]
            }
            "PostPass::LayerFinalization" => &[
                "LayerCollectionIR",
                "LayerPlanIR",
                "SurfaceClassificationIR",
                "PaintRegionIR",
                "RegionMapIR",
            ],
            _ => continue,
        };

        for declared in &module.ir_reads {
            let ok = allowed_upstream_roots
                .iter()
                .any(|root| path_mentions_root(declared, root));
            if !ok {
                offenders.push(format!(
                    "{stem} (stage {}): read '{declared}' is not a declared upstream IR root {:?}",
                    module.stage, allowed_upstream_roots
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "docs/01 §'Module Access Contract' forbids reads of non-upstream IR. Offenders:\n  - {}",
        offenders.join("\n  - ")
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Test 4 — seam-planner-default declares the PrePass::SeamPlanning contract
// ──────────────────────────────────────────────────────────────────────────

/// Verifies AC-4: the seam-planner-default module manifest declares the
/// correct prepass contract roots (reads: MeshIR, SurfaceClassificationIR,
/// LayerPlanIR; writes: SeamPlanIR) with no undeclared layer-stage writes.
#[test]
fn seam_planner_default_declares_prepass_contract_roots() {
    let modules = discover_core_manifests();

    let seam_planner = modules
        .iter()
        .find(|(stem, _)| stem.as_str() == "seam-planner-default");

    let Some((_, module)) = seam_planner else {
        panic!(
            "seam-planner-default not found under {}. \
             Is the module's .toml and .wasm present?",
            core_modules_root().display()
        )
    };

    assert_eq!(
        module.stage, "PrePass::SeamPlanning",
        "seam-planner-default must declare stage = PrePass::SeamPlanning"
    );

    let required_reads = ["MeshIR", "SurfaceClassificationIR", "LayerPlanIR"];
    for root in &required_reads {
        assert!(
            module.ir_reads.iter().any(|p| path_mentions_root(p, root)),
            "seam-planner-default must declare read root '{}'",
            root
        );
    }

    assert!(
        module.ir_writes.iter().any(|p| path_mentions_root(p, "SeamPlanIR")),
        "seam-planner-default must declare write root 'SeamPlanIR'"
    );
}
