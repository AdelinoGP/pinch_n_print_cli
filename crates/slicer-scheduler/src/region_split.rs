//! Cross-manifest aggregation of `[[region_split]]` declarations.
//!
//! The public surface is:
//! - [`aggregate_region_splits`] — fold `LoadedModule` region-split lists into
//!   a `BTreeMap<semantic_name, AggregatedRegionSplitEntry>`.
//! - [`canonical_variant_chain_order`] — return semantic names in
//!   `(priority, name)` canonical order.
//!
//! The aggregate is exactly the union of the loaded modules' declarations —
//! no semantics are seeded implicitly. A core semantic such as `material` or
//! `fuzzy_skin` is present iff a loaded manifest declares it (the core
//! modules do; see ADR-0071).
//!
//! See packet 92, AC-7, AC-8, AC-N2.

use std::collections::BTreeMap;

use crate::manifest::{DiagnosticLevel, LoadDiagnostic, LoadedModule};

pub use slicer_ir::slice_ir::AggregatedRegionSplitEntry;

/// Aggregate `[[region_split]]` declarations across all loaded modules into a
/// `BTreeMap<semantic_name, AggregatedRegionSplitEntry>`.
///
/// Cross-manifest invariants:
/// - Same semantic, same `priority` + `value_type`, different modules: merge
///   `declaring_modules` (no diagnostic).
/// - Same semantic, different `priority` or `value_type`: pick the first seen
///   and push a `DiagnosticLevel::Warning` diagnostic.
/// - Different semantics with the same priority: push a `DiagnosticLevel::Warning`
///   diagnostic naming both semantics, the shared priority, and the lex-tiebreaker
///   order.  (AC-7)
///
/// For `(priority, name)` canonical order, callers must use
/// [`canonical_variant_chain_order`] rather than raw `BTreeMap::iter()`.
///
/// AC-N2: empty input → empty `BTreeMap`, no diagnostics.
pub fn aggregate_region_splits(
    modules: &[LoadedModule],
    diagnostics: &mut Vec<LoadDiagnostic>,
) -> BTreeMap<String, AggregatedRegionSplitEntry> {
    let mut agg: BTreeMap<String, AggregatedRegionSplitEntry> = BTreeMap::new();

    for module in modules {
        let module_id = module.id().to_owned();

        for decl in module.region_splits() {
            let semantic = &decl.semantic;

            match agg.get_mut(semantic) {
                Some(existing) => {
                    if existing.priority == decl.priority && existing.value_type == decl.value_type
                    {
                        // Same contract — just add the module to the declaring list.
                        if !existing.declaring_modules.contains(&module_id) {
                            existing.declaring_modules.push(module_id.clone());
                            existing.declaring_modules.sort();
                        }
                    } else {
                        // Conflicting contract — warn and keep the first-seen entry.
                        diagnostics.push(LoadDiagnostic {
                            level: DiagnosticLevel::Warning,
                            path: module.wasm_path().to_path_buf(),
                            field: Some("region_split".to_owned()),
                            message: format!(
                                "Conflicting region-split declaration for semantic \"{semantic}\": \
                                 module \"{module_id}\" declares priority={p2}/value_type={vt2:?} \
                                 but earlier declaration has priority={p1}/value_type={vt1:?}. \
                                 Keeping first-seen entry.",
                                p1 = existing.priority,
                                vt1 = existing.value_type,
                                p2 = decl.priority,
                                vt2 = decl.value_type,
                            ),
                        });
                    }
                }
                None => {
                    agg.insert(
                        semantic.clone(),
                        AggregatedRegionSplitEntry {
                            priority: decl.priority,
                            value_type: decl.value_type,
                            declaring_modules: vec![module_id.clone()],
                        },
                    );
                }
            }
        }
    }

    // AC-7: detect tied priorities (≥ 2 distinct semantics sharing a priority).
    // Group semantic names by priority.
    let mut by_priority: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for (name, entry) in &agg {
        by_priority
            .entry(entry.priority)
            .or_default()
            .push(name.clone());
    }

    for (priority, mut semantics) in by_priority {
        if semantics.len() < 2 {
            continue;
        }
        semantics.sort(); // lex tiebreaker order for the message
        let names_str = semantics.join(", ");
        // Collect module info for the message: "name (from module_a, module_b)"
        let detail: Vec<String> = semantics
            .iter()
            .map(|sname| {
                let mods = agg[sname].declaring_modules.join(", ");
                format!("{sname} (from {mods})")
            })
            .collect();
        let detail_str = detail.join(", ");
        diagnostics.push(LoadDiagnostic {
            level: DiagnosticLevel::Warning,
            // No single path is authoritative; use an empty PathBuf as sentinel.
            path: std::path::PathBuf::new(),
            field: Some("region_split".to_owned()),
            message: format!(
                "Tied region-split priority {priority}: {detail_str}. \
                 Lex tiebreaker order: {names_str}."
            ),
        });
    }

    agg
}

/// Return semantic names in `(priority, name)` canonical order — the order
/// callers should use when constructing or interpreting `variant_chain`.
///
/// AC-8: the returned slice is sorted by `(priority, semantic_name)`.
pub fn canonical_variant_chain_order(
    agg: &BTreeMap<String, AggregatedRegionSplitEntry>,
) -> Vec<String> {
    let mut pairs: Vec<_> = agg
        .iter()
        .map(|(name, entry)| (entry.priority, name.clone()))
        .collect();
    pairs.sort(); // (u32, String) sorts by (priority, name) — deterministic
    pairs.into_iter().map(|(_p, name)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The aggregate contains exactly the semantics declared by the loaded
    /// modules: no core semantics are seeded implicitly (ADR-0071). With an
    /// empty module list the aggregate must be empty; a module declaring only
    /// `material` must produce a map holding `material` and NOT `fuzzy_skin`.
    #[test]
    fn aggregate_contains_only_declared_semantics() {
        // Empty module list → empty aggregate (AC-N2 behaviour, re-pinned
        // here at the module level).
        let empty = aggregate_region_splits(&[], &mut Vec::new());
        assert!(
            empty.is_empty(),
            "no declarations means no semantics; core names must not be seeded"
        );
        assert!(!empty.contains_key("material"));
        assert!(!empty.contains_key("fuzzy_skin"));

        // A module that declares `material` only → `material` present,
        // `fuzzy_skin` absent.
        let declared_material = crate::manifest::LoadedModuleBuilder::new(
            "com.example.material-only",
            slicer_ir::SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            "Layer::Perimeters",
            String::new(),
            std::path::PathBuf::from("fixtures/material-only.wasm"),
        )
        .region_splits(vec![crate::manifest::RegionSplitDeclaration {
            semantic: "material".to_owned(),
            priority: 100,
            value_type: slicer_ir::slice_ir::RegionSplitValueType::ToolIndex,
        }])
        .build();

        let aggregated = aggregate_region_splits(&[declared_material], &mut Vec::new());
        let material = aggregated
            .get("material")
            .expect("declared material must be aggregated");
        assert_eq!(material.priority, 100);
        assert!(
            !aggregated.contains_key("fuzzy_skin"),
            "undeclared core semantics must not be seeded into the aggregate"
        );
    }
}
