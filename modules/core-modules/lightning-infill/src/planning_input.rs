//! Typed preparation records -> the lightning kernel's planner inputs.
//!
//! This adapter is additive: it supplies the inputs the relocated algorithm
//! already reads and changes no arithmetic. The planner consumes only
//! `SliceIR::{global_layer_index, regions}` and, per region, `object_id`,
//! `region_id`, `variant_chain` and `polygons`; every other `SlicedRegion`
//! field keeps its default. `SliceIR::z` is carried unread for layer identity.
//!
//! The five `ResolvedConfig` scalars the planner reads are print-wide on the
//! host but per-region on the typed surface, so they must agree across every
//! committed region config. A disagreement is a named `57412` preparation
//! failure, never a silent pick.

use std::collections::BTreeMap;

use slicer_ir::{ResolvedConfig, SliceIR, SlicedRegion};
use slicer_sdk::error::ModuleError;
use slicer_sdk::preparation::{PreparationRegion, PreparationRegionConfig, PreparationRegionKey};

/// Module-scoped config-disagreement / unusable-config code (design D3/D2).
pub(crate) const CONFIG_DISAGREEMENT_CODE: u32 = 57412;

/// The planner's config surface: the four per-region scalars plus the pinned
/// `sparse_fill_holder` key that makes the host branch unreachable.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PlannerScalars {
    line_width: f64,
    infill_density: f32,
    layer_height: f64,
    infill_angle: f32,
}

impl PlannerScalars {
    fn read(view: &slicer_ir::ConfigView) -> Result<Self, ModuleError> {
        Ok(Self {
            line_width: view.require_float("line_width")?,
            infill_density: view.require_float("infill_density")? as f32,
            layer_height: view.require_float("layer_height")?,
            infill_angle: view.require_float("infill_angle")? as f32,
        })
    }

    fn agree_with(&self, other: &Self) -> Result<(), ModuleError> {
        if self.line_width.to_bits() != other.line_width.to_bits() {
            return Err(disagreement("line_width"));
        }
        if self.infill_density.to_bits() != other.infill_density.to_bits() {
            return Err(disagreement("infill_density"));
        }
        if self.layer_height.to_bits() != other.layer_height.to_bits() {
            return Err(disagreement("layer_height"));
        }
        if self.infill_angle.to_bits() != other.infill_angle.to_bits() {
            return Err(disagreement("infill_angle"));
        }
        Ok(())
    }
}

fn disagreement(key: &str) -> ModuleError {
    ModuleError::fatal(
        CONFIG_DISAGREEMENT_CODE,
        format!("config disagrees across committed regions: {key}"),
    )
}

fn region_matches(key: &PreparationRegionKey, region: &PreparationRegion) -> bool {
    key.global_layer_index == region.global_layer_index
        && key.object_id == region.object_id
        && key.region_id == region.region_id
        && key.variant_chain == region.variant_chain
}

/// Whether a committed region carries a real effective layer height.
///
/// Derived regions (paint segmentation, region split) are constructed from
/// `SlicedRegion::default()`, so the field keeps the `f32` default `0.0` rather
/// than a carried value. A 0 mm layer is not representable, so a non-positive
/// reading means "not supplied" and the cross-check has nothing to compare.
fn carries_layer_height(region: &PreparationRegion) -> bool {
    region.effective_layer_height > 0.0
}

/// Build the kernel's `Vec<SliceIR>` and `ResolvedConfig` from committed typed
/// preparation records.
///
/// An empty region population is a valid successful empty preparation: it
/// yields no layers and a default config whose `sparse_fill_holder` is pinned.
/// A region population with no matching config is an input-contract failure,
/// not an empty plan.
pub(crate) fn build_planner_inputs(
    regions: &[PreparationRegion],
    configs: &[PreparationRegionConfig],
) -> Result<(Vec<SliceIR>, ResolvedConfig), ModuleError> {
    let mut scalars: Option<PlannerScalars> = None;
    let mut layer_heights: BTreeMap<u32, f32> = BTreeMap::new();

    for config in configs {
        let current = PlannerScalars::read(&config.config)?;
        match &scalars {
            Some(existing) => existing.agree_with(&current)?,
            None => scalars = Some(current),
        }
        // The committed config population and the committed region population are
        // not guaranteed to be key-identical. Measured on the painted
        // `hole_void_classic` case: layer 0's region geometry carries chain `[]`
        // while its region-map key carries `[("material", ToolIndex(1))]`, so a
        // config can name a key with no geometry on that layer (and, symmetrically,
        // a region need not have its own config). The planner reads one print-wide
        // config, so a config is only cross-checked when it does name a region;
        // requiring a 1:1 population would reject a valid painted print.
        let Some(region) = regions
            .iter()
            .find(|region| region_matches(&config.key, region))
        else {
            continue;
        };
        // `effective_layer_height` is carried evidence, not a planner input: the
        // planner multiplies `config.layer_height` in every case, so this
        // comparison exists only to catch a config that disagrees with the layer it
        // was committed for. Derived regions (paint segmentation, region split) are
        // built from `SlicedRegion::default()`, which leaves the field at the `f32`
        // default `0.0`; no layer can be 0 mm (`lightning-infill.toml` floors
        // `layer_height` at 0.01), so a non-positive value means "not supplied".
        if carries_layer_height(region)
            && (current.layer_height - f64::from(region.effective_layer_height)).abs() > 1.0e-6
        {
            return Err(disagreement("layer_height"));
        }
        if carries_layer_height(region) {
            match layer_heights.get(&config.key.global_layer_index).copied() {
                Some(existing) if existing.to_bits() != region.effective_layer_height.to_bits() => {
                    return Err(disagreement("layer_height"));
                }
                Some(_) => {}
                None => {
                    layer_heights.insert(
                        config.key.global_layer_index,
                        region.effective_layer_height,
                    );
                }
            }
        }
    }

    if !regions.is_empty() && configs.is_empty() {
        return Err(ModuleError::fatal(
            CONFIG_DISAGREEMENT_CODE,
            "config disagrees across committed regions: line_width (no committed region configs)"
                .to_string(),
        ));
    }
    let Some(scalars) = scalars else {
        // Empty preparation: no regions, no configs, no work.
        return Ok((
            Vec::new(),
            ResolvedConfig {
                sparse_fill_holder: "lightning-infill".to_string(),
                ..Default::default()
            },
        ));
    };

    if scalars.line_width == 0.0 {
        return Err(ModuleError::fatal(
            CONFIG_DISAGREEMENT_CODE,
            "line_width is still the automatic sentinel (0.0) after expansion".to_string(),
        ));
    }

    let mut layers: BTreeMap<u32, SliceIR> = BTreeMap::new();
    for region in regions {
        let layer = layers.entry(region.global_layer_index).or_insert_with(|| SliceIR {
            global_layer_index: region.global_layer_index,
            z: region.z,
            ..SliceIR::default()
        });
        let sliced = SlicedRegion {
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            polygons: region.polygons.clone(),
            variant_chain: region
                .variant_chain
                .iter()
                .map(|(key, value)| (key.clone(), slicer_ir::PaintValue::from(value)))
                .collect(),
            ..SlicedRegion::default()
        };
        layer.regions.push(sliced);
    }

    Ok((
        layers.into_values().collect(),
        ResolvedConfig {
            sparse_fill_holder: "lightning-infill".to_string(),
            line_width: scalars.line_width,
            infill_density: scalars.infill_density,
            layer_height: scalars.layer_height,
            infill_angle: scalars.infill_angle,
            ..Default::default()
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use slicer_ir::ConfigValue;
    use slicer_sdk::preparation::{PreparationInputPhase, PreparationRegionKey};

    use super::*;

    fn planner_config(line_width: f64, layer_height: f64) -> slicer_ir::ConfigView {
        let mut values = HashMap::new();
        values.insert("line_width".to_string(), ConfigValue::Float(line_width));
        values.insert("infill_density".to_string(), ConfigValue::Float(0.2));
        values.insert("layer_height".to_string(), ConfigValue::Float(layer_height));
        values.insert("infill_angle".to_string(), ConfigValue::Float(45.0));
        slicer_ir::ConfigView::from_map(values)
    }

    fn region(
        object_id: &str,
        global_layer_index: u32,
        region_id: u64,
        effective_layer_height: f32,
    ) -> PreparationRegion {
        // exhaustive: `PreparationRegion` has no `Default`; every committed field is
        // spelled out so a new one is a visible compile error here rather than a
        // silently defaulted input.
        PreparationRegion {
            global_layer_index,
            z: 0.2,
            object_id: object_id.to_string(),
            region_id,
            variant_chain: Vec::new(),
            polygons: Vec::new(),
            infill_areas: Vec::new(),
            nonplanar_surface: None,
            effective_layer_height,
            segment_annotations: HashMap::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
            internal_solid_fill: Vec::new(),
            internal_bridge_areas: Vec::new(),
            internal_bridge_angles_deg: Vec::new(),
            raft_fill: Vec::new(),
            phase: PreparationInputPhase::CommittedPrepass,
        }
    }

    fn config_for(region: &PreparationRegion, line_width: f64, layer_height: f64) -> PreparationRegionConfig {
        PreparationRegionConfig {
            key: PreparationRegionKey {
                global_layer_index: region.global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
                variant_chain: region.variant_chain.clone(),
            },
            config: planner_config(line_width, layer_height),
        }
    }

    /// Regression: paint segmentation and region split rebuild their regions from
    /// `SlicedRegion::default()`, so `effective_layer_height` is `0.0` there. The
    /// painted `hole_void_classic` case failed preparation with `57412
    /// layer_height` before this was treated as "not supplied".
    #[test]
    fn derived_region_with_defaulted_height_is_not_a_disagreement() {
        let regions = vec![region("object-a-region-0", 0, 0, 0.0)];
        let configs = vec![config_for(&regions[0], 0.45, 0.2)];

        let (slices, resolved) = build_planner_inputs(&regions, &configs)
            .expect("a derived region's defaulted height is not a disagreement");

        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].regions.len(), 1);
        assert_eq!(slices[0].z, 0.2);
        assert_eq!(resolved.layer_height, 0.2);
        assert_eq!(resolved.sparse_fill_holder, "lightning-infill");
    }

    /// Regression: the committed config population and the committed region
    /// population are not key-identical. Measured on the painted
    /// `hole_void_classic` case, layer 0's geometry carries chain `[]` while its
    /// region-map key carries `[("material", ToolIndex(1))]`; requiring a 1:1
    /// population rejected the print with `57412` before planning.
    #[test]
    fn config_key_without_matching_region_is_accepted() {
        let regions = vec![region("object-a-region-0", 0, 0, 0.2)];
        let mut configs = vec![config_for(&regions[0], 0.45, 0.2)];
        configs[0].key.variant_chain = vec![(
            "material".to_string(),
            slicer_sdk::preparation::PreparationPaintValue::ToolIndex(1),
        )];

        let (slices, resolved) = build_planner_inputs(&regions, &configs)
            .expect("a painted config key with no geometry on that layer is not a defect");

        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].regions.len(), 1);
        assert_eq!(resolved.line_width, 0.45);
    }

    #[test]
    fn supplied_layer_height_disagreement_is_57412() {
        let regions = vec![region("obj", 0, 0, 0.3)];
        let configs = vec![config_for(&regions[0], 0.45, 0.2)];

        let error = build_planner_inputs(&regions, &configs)
            .expect_err("a carried height that disagrees with the config must fail");

        assert_eq!(error.code, 57412);
        assert!(error.fatal);
        assert_eq!(
            error.message,
            "config disagrees across committed regions: layer_height"
        );
    }

    #[test]
    fn per_layer_height_disagreement_is_57412() {
        let regions = vec![region("obj", 0, 0, 0.2), region("obj", 0, 1, 0.21)];
        let configs = vec![
            config_for(&regions[0], 0.45, 0.2),
            config_for(&regions[1], 0.45, 0.2),
        ];

        let error = build_planner_inputs(&regions, &configs)
            .expect_err("two carried heights on one layer cannot both be right");

        assert_eq!(error.code, 57412);
        assert_eq!(
            error.message,
            "config disagrees across committed regions: layer_height"
        );
    }

    #[test]
    fn cross_region_scalar_disagreement_names_the_key() {
        let regions = vec![region("obj", 0, 0, 0.2), region("obj", 1, 0, 0.2)];
        let configs = vec![
            config_for(&regions[0], 0.45, 0.2),
            config_for(&regions[1], 0.6, 0.2),
        ];

        let error = build_planner_inputs(&regions, &configs)
            .expect_err("per-region scalars must agree print-wide");

        assert_eq!(error.code, 57412);
        assert_eq!(
            error.message,
            "config disagrees across committed regions: line_width"
        );
    }

    #[test]
    fn surviving_zero_line_width_sentinel_is_57412() {
        let regions = vec![region("obj", 0, 0, 0.2)];
        let configs = vec![config_for(&regions[0], 0.0, 0.2)];

        let error = build_planner_inputs(&regions, &configs)
            .expect_err("the auto-width sentinel must not reach the kernel");

        assert_eq!(error.code, 57412);
        assert_eq!(
            error.message,
            "line_width is still the automatic sentinel (0.0) after expansion"
        );
    }
}
