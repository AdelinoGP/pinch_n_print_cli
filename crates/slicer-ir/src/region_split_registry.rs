//! Region split registry helpers for P93 region-mapping cross-product expansion.
//!
//! Provides [`enumerate_canonical_chains`], the deterministic enumerator that
//! produces every subset of `(semantic, value)` pairs (including the empty
//! subset) used to expand `RegionMapIR.entries` per `(layer, ActiveRegion,
//! variant_chain)`. The kernel call site lives in `scan_paint_data`
//! (Step 3 of P93); this module is the standalone helper plus tests.

use crate::PaintValue;
use std::collections::HashMap;

/// Enumerate every subset of `(semantic, value)` pairs in canonical order.
///
/// For each semantic listed in `canonical_order` that appears in `variants`
/// with a non-empty `Vec<PaintValue>`, the enumeration produces chains that
/// either omit the semantic or include exactly one of its values. Semantics
/// listed in `canonical_order` but missing from `variants` (or with an empty
/// Vec) contribute a factor of 1 (i.e. they are silently skipped).
///
/// # Cardinality
///
/// The returned `Vec` has length `∏ (1 + K_i)` where `K_i = variants[canonical_order[i]].len()`
/// for semantics present with non-empty values (others contribute 1).
///
/// # Determinism
///
/// The enumeration order is fixed: depth-first, omit-branch first per axis,
/// then value-branches in the input `Vec<PaintValue>` order. The empty subset
/// is always the FIRST element of the returned `Vec`. Within each chain, the
/// `(String, PaintValue)` pairs appear in canonical-order sequence (i.e. the
/// order of `canonical_order`).
///
/// # Per-value ordering
///
/// This helper preserves the input `Vec<PaintValue>` order verbatim — it does
/// NOT re-sort `variants[sem]`. The caller is responsible for delivering a
/// deterministic value order. The canonical `PaintValue` comparator the
/// caller is expected to apply BEFORE invoking this helper (per P93
/// requirements.md) is:
///
/// `Flag < ToolIndex(0) < ToolIndex(1) < ... < Custom(s_lex)`
///
/// This convention is informational; the helper does not enforce it.
pub fn enumerate_canonical_chains(
    variants: &HashMap<String, Vec<PaintValue>>,
    canonical_order: &[String],
) -> Vec<Vec<(String, PaintValue)>> {
    let mut out = Vec::new();
    build(&mut out, Vec::new(), 0, canonical_order, variants);
    out
}

fn build(
    out: &mut Vec<Vec<(String, PaintValue)>>,
    acc: Vec<(String, PaintValue)>,
    idx: usize,
    canonical_order: &[String],
    variants: &HashMap<String, Vec<PaintValue>>,
) {
    if idx == canonical_order.len() {
        out.push(acc);
        return;
    }
    let sem = &canonical_order[idx];
    // Branch 1: omit this semantic.
    build(out, acc.clone(), idx + 1, canonical_order, variants);
    // Branches 2..K+1: include each value in input Vec order.
    if let Some(values) = variants.get(sem) {
        for v in values {
            let mut next = acc.clone();
            next.push((sem.clone(), v.clone()));
            build(out, next, idx + 1, canonical_order, variants);
        }
    }
}
