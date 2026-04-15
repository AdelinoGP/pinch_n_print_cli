//! Mesh segmentation prepass module for ModularSlicer.
//!
//! Emits whole-triangle paint marks through the WIT
//! `mesh-segmentation-output::mark-triangle-paint` drain. The macro
//! path (`#[slicer_module]`) owns the SDK→WIT bridge after STEP H; no
//! hand-authored wit-guest duplicate is needed.
//!
//! # Mark source
//!
//! Marks are driven by host-supplied config keys of the form:
//!
//!   `mesh_seg_mark:<object_id>:<facet_index>:<semantic>` = `<value>`
//!
//! That key shape mirrors the canonical segmentation contract from
//! docs/03_wit_and_manifest.md: the WIT `run-mesh-segmentation` export
//! provides only `list<object-id>` + the config view, so the only
//! deterministic mark source a macro-authored module can rely on is
//! the declared config. Unpainted meshes (including Benchy) carry no
//! `mesh_seg_mark:*` keys and the module is a deterministic zero-mark
//! no-op, which is the correct semantic.
//!
//! # Ordering
//!
//! Marks are emitted in the deterministic order:
//!   1. objects in the order supplied by the host,
//!   2. within each object, marks keyed by `(facet_index asc, semantic asc)`,
//!   3. ties broken by key-string order.
//!
//! `MeshSegmentationIR.marks` preserves the host's mark-push order so
//! tests can rely on a stable byte image for determinism checks.
//!
//! # Why the SDK path replaced the hand-written wit-guest
//!
//! Before STEP H the canonical `mesh-segmentation` component was a
//! hand-written `wit_bindgen::generate!` duplicate because the SDK
//! `MeshSegmentationOutput` builder exposed `push_modification` /
//! `ObjectMeshModification` (full mesh rebuild) — a shape with no
//! representation on the WIT surface. STEP H added
//! `MeshSegmentationOutput::mark_triangle_paint(...)` that matches
//! the WIT `mark-triangle-paint` method one-to-one and let the macro
//! arm drain it, so the canonical module can now follow the same
//! `#[slicer_module]` + 2-line `pub use` wit-guest pattern as the
//! other core modules (docs/05 §Authoring, STEP F precedent).

use slicer_sdk::prelude::*;

/// Mesh segmentation prepass module (config-driven marks).
pub struct MeshSegmentation;

#[slicer_module]
impl PrepassModule for MeshSegmentation {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(MeshSegmentation)
    }

    fn run_mesh_segmentation(
        &self,
        objects: &[MeshObjectView],
        output: &mut MeshSegmentationOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let mut parsed: Vec<ParsedMark> = Vec::new();
        for key in config.keys() {
            if let Some(value) = config.get(&key) {
                if let Some(mark) = parse_mark(&key, value) {
                    parsed.push(mark);
                }
            }
        }

        let known_object_ids: Vec<&str> =
            objects.iter().map(|o| o.object_id.as_str()).collect();
        let object_rank = |id: &str| -> usize {
            known_object_ids
                .iter()
                .position(|o| *o == id)
                .unwrap_or(known_object_ids.len())
        };
        parsed.sort_by(|a, b| {
            object_rank(&a.object_id)
                .cmp(&object_rank(&b.object_id))
                .then_with(|| a.object_id.cmp(&b.object_id))
                .then_with(|| a.facet_index.cmp(&b.facet_index))
                .then_with(|| a.semantic.cmp(&b.semantic))
        });

        for mark in parsed {
            output
                .mark_triangle_paint(
                    mark.object_id,
                    mark.facet_index,
                    mark.semantic,
                    mark.value,
                )
                .map_err(|e| ModuleError::fatal(1, e))?;
        }

        Ok(())
    }
}

/// Parsed `mesh_seg_mark:<object_id>:<facet_index>:<semantic> = <value>` entry.
#[derive(Debug, Clone)]
pub struct ParsedMark {
    /// Target object id.
    pub object_id: String,
    /// Target triangle index within the object's mesh.
    pub facet_index: u32,
    /// Paint semantic (e.g. "support_enforcer").
    pub semantic: String,
    /// Paint value, serialized to a string for WIT transport.
    pub value: String,
}

/// Parse one `mesh_seg_mark:*` config entry. Returns `None` if the key
/// does not match the canonical shape or if any field is malformed.
///
/// Accepts the same `ConfigValue` variants the hand-written wit-guest
/// accepted before STEP H (`String`, `Int`, `Float`, `Bool`) so
/// existing `mesh_seg_mark:*` configs continue to work verbatim
/// through the macro path.
pub fn parse_mark(key: &str, value: &ConfigValue) -> Option<ParsedMark> {
    const PREFIX: &str = "mesh_seg_mark:";
    let rest = key.strip_prefix(PREFIX)?;
    let mut parts = rest.splitn(3, ':');
    let object_id = parts.next()?.to_string();
    let facet_str = parts.next()?;
    let semantic = parts.next()?.to_string();
    if object_id.is_empty() || semantic.is_empty() {
        return None;
    }
    let facet_index: u32 = facet_str.parse().ok()?;
    let value_str = match value {
        ConfigValue::String(s) => s.clone(),
        ConfigValue::Int(i) => i.to_string(),
        ConfigValue::Float(f) => f.to_string(),
        ConfigValue::Bool(b) => b.to_string(),
        _ => return None,
    };
    Some(ParsedMark {
        object_id,
        facet_index,
        semantic,
        value: value_str,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mark_accepts_canonical_shape() {
        let pm = parse_mark(
            "mesh_seg_mark:obj-1:42:support_enforcer",
            &ConfigValue::String("enabled".into()),
        )
        .expect("well-formed key must parse");
        assert_eq!(pm.object_id, "obj-1");
        assert_eq!(pm.facet_index, 42);
        assert_eq!(pm.semantic, "support_enforcer");
        assert_eq!(pm.value, "enabled");
    }

    #[test]
    fn parse_mark_coerces_non_string_values() {
        let pm = parse_mark(
            "mesh_seg_mark:obj:0:tool",
            &ConfigValue::Int(3),
        )
        .unwrap();
        assert_eq!(pm.value, "3");
        let pm = parse_mark(
            "mesh_seg_mark:obj:0:flag",
            &ConfigValue::Bool(true),
        )
        .unwrap();
        assert_eq!(pm.value, "true");
    }

    #[test]
    fn parse_mark_rejects_non_prefix_keys() {
        assert!(parse_mark("layer_height", &ConfigValue::Float(0.2)).is_none());
    }

    #[test]
    fn parse_mark_rejects_malformed_shape() {
        // Missing semantic segment.
        assert!(parse_mark(
            "mesh_seg_mark:obj:5",
            &ConfigValue::String("x".into())
        )
        .is_none());
        // Empty object id.
        assert!(parse_mark(
            "mesh_seg_mark::5:sem",
            &ConfigValue::String("x".into())
        )
        .is_none());
        // Non-numeric facet index.
        assert!(parse_mark(
            "mesh_seg_mark:obj:not-a-number:sem",
            &ConfigValue::String("x".into())
        )
        .is_none());
        // Unsupported value kind (a list).
        assert!(parse_mark(
            "mesh_seg_mark:obj:0:sem",
            &ConfigValue::List(vec![])
        )
        .is_none());
    }
}
