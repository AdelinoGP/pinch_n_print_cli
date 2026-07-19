#![allow(missing_docs)]

use infill_linker::InfillLinker;
use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillRegion, Point2, Point3WithWidth,
    Polygon,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::{ConfigViewBuilder, PerimeterRegionViewBuilder};
use slicer_sdk::traits::LayerModule;

fn square(x_mm: f32, width_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x_mm, 0.0),
                Point2::from_mm(x_mm + width_mm, 0.0),
                Point2::from_mm(x_mm + width_mm, 10.0),
                Point2::from_mm(x_mm, 10.0),
            ],
        },
        holes: vec![],
    }
}

fn path(x_start_mm: f32, x_end_mm: f32, y_mm: f32, width_mm: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: x_start_mm,
                y: y_mm,
                z: 0.2,
                width: width_mm,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: x_end_mm,
                y: y_mm,
                z: 0.2,
                width: width_mm,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn config(line_width: f64, density: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("line_width", line_width)
        .float("infill_density", density)
        .build()
}

fn view(
    region_id: u64,
    area: ExPolygon,
    wall_source_region_id: Option<u64>,
    tool_index: u32,
    line_width: f64,
    density: f64,
) -> slicer_sdk::views::PerimeterRegionView {
    let mut view = PerimeterRegionViewBuilder::new()
        .object_id("object")
        .region_id(region_id)
        .add_infill_area(area)
        .wall_source_region_id(wall_source_region_id)
        .tool_index(tool_index)
        .build();
    view.set_config(config(line_width, density));
    view
}

fn run(
    prior: &[InfillRegion],
    views: &[slicer_sdk::views::PerimeterRegionView],
) -> InfillOutputBuilder {
    let module_config = config(0.4, 0.2);
    let module = InfillLinker::on_print_start(&module_config).unwrap();
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, views, prior, &mut output, &module_config)
        .unwrap();
    output
}

fn sparse_region(region_id: u64, paths: Vec<ExtrusionPath3D>) -> InfillRegion {
    InfillRegion {
        object_id: "object".to_string(),
        region_id,
        sparse_infill: paths,
        solid_infill: vec![],
        ironing: vec![],
    }
}

#[test]
fn wall_sharing_same_config_union_link() {
    let paths_a = (1..=5)
        .map(|index| path(0.0, 10.0, index as f32, 0.4))
        .collect::<Vec<_>>();
    let paths_b = (1..=5)
        .map(|index| path(6.0, 15.0, index as f32, 0.4))
        .collect::<Vec<_>>();
    let prior = vec![sparse_region(1, paths_a), sparse_region(2, paths_b)];
    let views = vec![
        view(1, square(0.0, 10.0), Some(100), 0, 0.4, 0.2),
        view(2, square(5.0, 10.0), Some(100), 0, 0.4, 0.2),
        view(100, square(0.0, 15.0), None, 0, 0.4, 0.2),
    ];
    let output = run(&prior, &views);

    assert!(output
        .sparse_paths()
        .iter()
        .zip(output.sparse_path_origins())
        .any(|(path, origin)| {
            origin == &Some(("object".to_string(), 1))
                && path.points.iter().any(|point| point.x <= 0.1)
                && path.points.iter().any(|point| point.x >= 14.9)
        }));
}

#[test]
fn wall_sharing_diff_config_no_inset_on_shared_arc() {
    let prior = vec![
        sparse_region(1, vec![path(8.0, 10.0, 5.0, 0.4)]),
        sparse_region(2, vec![path(10.0, 12.0, 5.0, 0.8)]),
    ];
    let views = vec![
        view(1, square(0.0, 10.0), Some(100), 0, 0.4, 1.0),
        view(2, square(10.0, 10.0), Some(100), 0, 0.8, 1.0),
        view(100, square(0.0, 20.0), None, 0, 0.4, 0.2),
    ];
    let output = run(&prior, &views);

    for (path, origin) in output
        .sparse_paths()
        .iter()
        .zip(output.sparse_path_origins())
    {
        let min_x = path
            .points
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = path
            .points
            .iter()
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(!(min_x < 9.9 && max_x > 10.1));
        match origin.as_ref().map(|(_, region_id)| *region_id) {
            Some(1) => assert!(max_x >= 10.0 - 0.5 * 0.4),
            Some(2) => assert!(min_x <= 10.0 + 0.5 * 0.8),
            other => panic!("unexpected origin: {other:?}"),
        }
    }
}

#[test]
fn walls_separated_regions_never_connected() {
    let prior = vec![
        sparse_region(1, vec![path(8.0, 10.0, 5.0, 0.4)]),
        sparse_region(2, vec![path(10.0, 12.0, 5.0, 0.4)]),
    ];
    let views = vec![
        view(1, square(0.0, 10.0), None, 0, 0.4, 1.0),
        view(2, square(10.0, 10.0), None, 0, 0.4, 1.0),
    ];
    let output = run(&prior, &views);

    assert_eq!(output.sparse_paths().len(), 2);
    assert!(output.sparse_paths().iter().all(|path| {
        let min_x = path
            .points
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = path
            .points
            .iter()
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max);
        !(min_x < 9.9 && max_x > 10.1)
    }));
}

#[test]
fn different_tool_never_connected() {
    let prior = vec![
        sparse_region(1, vec![path(8.0, 10.0, 5.0, 0.4)]),
        sparse_region(2, vec![path(10.0, 12.0, 5.0, 0.4)]),
    ];
    let views = vec![
        view(1, square(0.0, 10.0), Some(100), 0, 0.4, 1.0),
        view(2, square(10.0, 10.0), Some(100), 1, 0.4, 1.0),
        view(100, square(0.0, 20.0), None, 0, 0.4, 0.2),
    ];
    let output = run(&prior, &views);

    assert_eq!(output.sparse_paths().len(), 2);
    assert!(output
        .sparse_path_origins()
        .iter()
        .all(|origin| origin.is_some()));
}
