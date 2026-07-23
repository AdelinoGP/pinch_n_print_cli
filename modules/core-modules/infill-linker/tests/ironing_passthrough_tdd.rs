#![allow(missing_docs)]

use infill_linker::InfillLinker;
use slicer_ir::{ConfigView, ExtrusionPath3D, ExtrusionRole, InfillRegion, Point3WithWidth};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::traits::LayerModule;

fn path(role: ExtrusionRole, speed_factor: f32, x_offset: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: x_offset,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
            Point3WithWidth {
                x: x_offset + 2.0,
                y: 1.0,
                z: 0.2,
                width: 0.35,
                flow_factor: 0.8,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
            Point3WithWidth {
                x: x_offset + 4.0,
                y: 0.5,
                z: 0.2,
                width: 0.45,
                flow_factor: 1.2,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
        ],
        role,
        speed_factor,
    }
}

fn prior_infill() -> Vec<InfillRegion> {
    vec![
        InfillRegion {
            object_id: "object-a".to_string(),
            region_id: 7,
            sparse_infill: vec![path(ExtrusionRole::SparseInfill, 0.91, 0.0)],
            solid_infill: vec![path(ExtrusionRole::TopSolidInfill, 1.13, 1.0)],
            ironing: vec![
                path(ExtrusionRole::Ironing, 0.73, 2.0),
                path(ExtrusionRole::Ironing, 1.25, 3.0),
            ],
        },
        InfillRegion {
            object_id: "object-b".to_string(),
            region_id: 11,
            sparse_infill: vec![],
            solid_infill: vec![],
            ironing: vec![path(ExtrusionRole::Ironing, 0.66, -2.0)],
        },
    ]
}

#[test]
fn ironing_passthrough_identical() {
    let config = ConfigView::new();
    let module = InfillLinker::on_print_start(&config).unwrap();
    let prior = prior_infill();
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill_postprocess(0, &[], &prior, &mut output, &config)
        .unwrap();

    let expected_sparse: Vec<_> = prior
        .iter()
        .flat_map(|region| region.sparse_infill.iter().cloned())
        .collect();
    let expected_solid: Vec<_> = prior
        .iter()
        .flat_map(|region| region.solid_infill.iter().cloned())
        .collect();
    let expected_ironing: Vec<_> = prior
        .iter()
        .flat_map(|region| region.ironing.iter().cloned())
        .collect();

    assert_eq!(output.sparse_paths(), expected_sparse.as_slice());
    assert_eq!(output.solid_paths(), expected_solid.as_slice());
    assert_eq!(output.ironing_paths(), expected_ironing.as_slice());
    assert_eq!(
        output.sparse_path_origins(),
        &[Some(("object-a".to_string(), 7))]
    );
    assert_eq!(
        output.solid_path_origins(),
        &[Some(("object-a".to_string(), 7))]
    );
    assert_eq!(
        output.ironing_path_origins(),
        &[
            Some(("object-a".to_string(), 7)),
            Some(("object-a".to_string(), 7)),
            Some(("object-b".to_string(), 11)),
        ]
    );
}
