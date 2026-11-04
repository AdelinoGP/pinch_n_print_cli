//! Kernel files are included byte-identically from the module; never edit them here.

#[path = "../../../src/kernel/distance_field.rs"]
pub mod distance_field;
#[path = "../../../src/kernel/generator.rs"]
#[allow(clippy::too_many_arguments)]
pub mod generator;
#[path = "../../../src/kernel/layer.rs"]
pub mod layer;
#[path = "../../../src/kernel/tree_node.rs"]
pub mod tree_node;

use crate::model::{EvidenceLocation, KernelError, LightningParameters};
use generator::Generator;
use slicer_ir::{mm_to_units, Point2};

/// Flat ring set for one layer; must match the included kernel's parent alias.
pub type Polygons = Vec<Vec<Point2>>;

/// Construct the module kernel from recorded parameters without substituting scalars.
#[allow(clippy::result_large_err)]
pub fn generator_from_parameters(
    outlines: Vec<Polygons>,
    parameters: &LightningParameters,
) -> Result<Generator, KernelError> {
    if parameters.supporting_radius_mm.is_none() != (parameters.density_fraction == 0.0) {
        return Err(KernelError::InvalidInput {
            context: EvidenceLocation {
                object_id: String::new(),
                global_layer_index: 0,
                region: None,
                source: None,
                checkpoint: None,
            },
            reason: "supporting_radius_mm must be None if and only if density_fraction is zero"
                .to_owned(),
        });
    }
    let spacing_mm = slicer_core::flow::line_width_to_spacing(
        parameters.extrusion_width_mm as f32,
        parameters.layer_height_mm as f32,
    )
    .map_or(0.0, |spacing| spacing);
    Ok(Generator::new(
        outlines,
        parameters.density_fraction,
        mm_to_units(parameters.extrusion_width_mm as f32),
        parameters.multiline as i64,
        mm_to_units(parameters.layer_height_mm as f32),
        parameters.overhang_angle_deg,
        parameters.prune_angle_deg,
        parameters.straightening_angle_deg,
        mm_to_units(spacing_mm * 0.5),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_scalars_match_canonical_tdd() {
        // exhaustive: recorded canonical parameter set exercises every scalar input.
        let mut parameters = LightningParameters {
            nozzle_diameter_mm: 0.4,
            extrusion_width_mm: 0.44999999999999996,
            density_fraction: 0.15,
            multiline: 1,
            layer_height_mm: 0.2,
            overhang_angle_deg: 45.0,
            prune_angle_deg: 5.0,
            straightening_angle_deg: 5.0,
            supporting_radius_mm: Some(3.0),
            wall_supporting_radius_mm: 0.19999999999999998,
            prune_length_mm: 0.017497,
            straightening_max_distance_mm: 0.017497,
        };
        let generator = generator_from_parameters(Vec::new(), &parameters).unwrap();
        assert_eq!(generator.supporting_radius, 30_000);
        assert_eq!(generator.wall_supporting_radius, 2_000);
        assert_eq!(generator.prune_length, 175);
        assert_eq!(generator.straightening_max_distance, 175);
        assert_eq!(
            generator.supporting_radius,
            mm_to_units(parameters.supporting_radius_mm.unwrap() as f32)
        );

        parameters.density_fraction = 0.0;
        assert!(matches!(
            generator_from_parameters(Vec::new(), &parameters),
            Err(KernelError::InvalidInput { reason, .. })
                if reason.contains("supporting_radius_mm")
        ));
        parameters.supporting_radius_mm = None;
        assert_eq!(
            generator_from_parameters(Vec::new(), &parameters)
                .unwrap()
                .supporting_radius,
            0
        );
        parameters.density_fraction = 0.15;
        assert!(matches!(
            generator_from_parameters(Vec::new(), &parameters),
            Err(KernelError::InvalidInput { reason, .. })
                if reason.contains("supporting_radius_mm")
        ));
    }
}
