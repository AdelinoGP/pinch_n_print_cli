//! Cross-manifest aggregation of `[[region_split]]` declarations.
//!
//! The public surface is:
//! - [`aggregate_region_splits`] — fold `LoadedModule` region-split lists into
//!   a `BTreeMap<semantic_name, AggregatedRegionSplitEntry>`.
//! - [`canonical_variant_chain_order`] — return semantic names in
//!   `(priority, name)` canonical order.
//!
//! See packet 92, AC-7, AC-8, AC-N2.

use std::collections::BTreeMap;

use crate::manifest::{DiagnosticLevel, LoadDiagnostic, LoadedModule};

pub use slicer_ir::slice_ir::AggregatedRegionSplitEntry;

/// Seed the built-in paint semantics into an aggregated region-split registry.
///
/// `material` and `fuzzy_skin` are always-on region-split discriminators —
/// CONTEXT.md's "Variant chain" section fixes their canonical precedence
/// (Material before FuzzySkin), and canonical OrcaSlicer splits painted
/// material/fuzzy-skin geometry unconditionally. The manifest
/// `[[region_split]]` opt-in (packet 92) is how community semantics and
/// additional module-owned semantics join the registry; the built-ins come
/// from `slicer_schema::CORE_REGION_SPLIT_PRIORITIES`, the same table the
/// manifest validator treats as the registered contract. `material`'s value
/// domain is tool indices and `fuzzy_skin`'s is a flag (`value_type =
/// "scalar"` is forbidden — D13). Existing entries win: a loaded module's
/// declaration of a core semantic has already passed the
/// `CorePriorityMismatch` validation, and aggregation's first-seen rule is
/// preserved.
pub fn seed_core_region_splits(aggregated: &mut BTreeMap<String, AggregatedRegionSplitEntry>) {
    for (semantic, priority) in slicer_schema::CORE_REGION_SPLIT_PRIORITIES {
        aggregated
            .entry((*semantic).to_owned())
            .or_insert_with(|| AggregatedRegionSplitEntry {
                priority: *priority,
                value_type: if *semantic == "material" {
                    slicer_ir::slice_ir::RegionSplitValueType::ToolIndex
                } else {
                    slicer_ir::slice_ir::RegionSplitValueType::Flag
                },
                declaring_modules: Vec::new(),
            });
    }
}

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
mod seed_tests {
    use super::*;

    #[test]
    fn seed_core_region_splits_seeds_builtins_and_preserves_declared_entries() {
        let mut seeded: BTreeMap<String, AggregatedRegionSplitEntry> = BTreeMap::new();
        seed_core_region_splits(&mut seeded);
        let material = seeded.get("material").expect("material must be seeded");
        assert_eq!(material.priority, 100);
        assert_eq!(
            material.value_type,
            slicer_ir::slice_ir::RegionSplitValueType::ToolIndex
        );
        let fuzzy = seeded.get("fuzzy_skin").expect("fuzzy_skin must be seeded");
        assert_eq!(fuzzy.priority, 200);
        assert_eq!(
            fuzzy.value_type,
            slicer_ir::slice_ir::RegionSplitValueType::Flag
        );

        let mut declared: BTreeMap<String, AggregatedRegionSplitEntry> = BTreeMap::new();
        declared.insert(
            "material".to_owned(),
            AggregatedRegionSplitEntry {
                priority: 100,
                value_type: slicer_ir::slice_ir::RegionSplitValueType::ToolIndex,
                declaring_modules: vec!["com.core.example".to_owned()],
            },
        );
        seed_core_region_splits(&mut declared);
        assert_eq!(declared.get("material").unwrap().declaring_modules.len(), 1);
        assert!(declared.contains_key("fuzzy_skin"));
    }
}
