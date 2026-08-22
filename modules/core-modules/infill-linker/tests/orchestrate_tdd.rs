#![allow(missing_docs)]

use infill_linker::InfillLinker;
use slicer_core::flow::line_width_to_spacing;
use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillRegion, Point2, Point3WithWidth,
    Polygon,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::{ConfigViewBuilder, PerimeterRegionViewBuilder};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
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
                ..Default::default()
            },
            Point3WithWidth {
                x: x_end_mm,
                y: y_mm,
                z: 0.2,
                width: width_mm,
                ..Default::default()
            },
        ],
        ..extrusion_path3d_base(ExtrusionRole::SparseInfill)
    }
}

fn vertical_path(x_mm: f32, width_mm: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: x_mm,
                y: 0.0,
                z: 0.2,
                width: width_mm,
                ..Default::default()
            },
            Point3WithWidth {
                x: x_mm,
                y: 10.0,
                z: 0.2,
                width: width_mm,
                ..Default::default()
            },
        ],
        ..extrusion_path3d_base(ExtrusionRole::SparseInfill)
    }
}

fn same_xy(point: &Point3WithWidth, expected: (f32, f32)) -> bool {
    (point.x - expected.0).abs() < 1e-3 && (point.y - expected.1).abs() < 1e-3
}

fn contour_stub_lengths(path: &ExtrusionPath3D, input_endpoints: &[(f32, f32)]) -> Vec<f32> {
    let is_input = |point: &Point3WithWidth| {
        input_endpoints
            .iter()
            .any(|expected| same_xy(point, *expected))
    };
    let mut lengths = Vec::new();

    for (start_index, point) in path.points.iter().enumerate() {
        if !is_input(point) {
            continue;
        }
        for direction in [-1_i32, 1_i32] {
            let next_index = start_index as i32 + direction;
            if next_index < 0
                || next_index >= path.points.len() as i32
                || is_input(&path.points[next_index as usize])
            {
                continue;
            }

            let mut index = next_index;
            let mut previous = point;
            let mut length = 0.0;
            while index >= 0
                && index < path.points.len() as i32
                && !is_input(&path.points[index as usize])
            {
                let current = &path.points[index as usize];
                length +=
                    ((previous.x - current.x).powi(2) + (previous.y - current.y).powi(2)).sqrt();
                previous = current;
                index += direction;
            }
            if length > 1e-3 {
                lengths.push(length);
            }
        }
    }

    lengths
}

fn config(line_width: f64, density: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("line_width", line_width)
        .float("infill_density", density)
        .build()
}

fn config_with_anchor(
    line_width: f64,
    density: f64,
    anchor_length: f64,
    anchor_max: f64,
) -> ConfigView {
    ConfigViewBuilder::new()
        .float("line_width", line_width)
        .float("infill_density", density)
        .float("infill_anchor", anchor_length)
        .float("infill_anchor_max", anchor_max)
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
        // Both the union (`infill_areas`) and the host's per-role partition are
        // populated, so these cross-region tests exercise the per-role boundary
        // lookup (`RoleBoundaries::for_role`) rather than the unpartitioned
        // fallback. All fixture paths are `SparseInfill`.
        .add_infill_area(area.clone())
        .sparse_infill_area(vec![area])
        .wall_source_region_id(wall_source_region_id)
        .tool_index(tool_index)
        .build();
    view.set_config(config(line_width, density));
    view
}

fn solid_view_with_anchor(
    region_id: u64,
    area: ExPolygon,
    wall_source_region_id: Option<u64>,
    tool_index: u32,
    line_width: f64,
    density: f64,
    anchor_length: f64,
    anchor_max: f64,
) -> slicer_sdk::views::PerimeterRegionView {
    let mut view = PerimeterRegionViewBuilder::new()
        .object_id("object")
        .region_id(region_id)
        .add_infill_area(area.clone())
        .sparse_infill_area(vec![area.clone()])
        .top_solid_fill(vec![area])
        .wall_source_region_id(wall_source_region_id)
        .tool_index(tool_index)
        .build();
    view.set_config(config_with_anchor(
        line_width,
        density,
        anchor_length,
        anchor_max,
    ));
    view
}

fn run(
    prior: &[InfillRegion],
    views: &[slicer_sdk::views::PerimeterRegionView],
) -> InfillOutputBuilder {
    let module_config = config(0.4, 0.2);
    run_with_config(prior, views, &module_config)
}

fn run_with_config(
    prior: &[InfillRegion],
    views: &[slicer_sdk::views::PerimeterRegionView],
    module_config: &ConfigView,
) -> InfillOutputBuilder {
    let module = InfillLinker::from_config(&module_config).unwrap();
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, views, prior, &mut output, &module_config)
        .unwrap();
    output
}

fn sparse_region(region_id: u64, paths: Vec<ExtrusionPath3D>) -> InfillRegion {
    // exhaustive: this fixture explicitly sets every InfillRegion field used by orchestration.
    InfillRegion {
        object_id: "object".to_string(),
        region_id,
        sparse_infill: paths,
        solid_infill: vec![],
        ironing: vec![],
        internal_bridge_infill: Vec::new(),
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
            origin
                == &Some(slicer_sdk::builders::RegionOrigin {
                    object_id: "object".to_string(),
                    region_id: 1,
                })
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
        match origin.as_ref().map(|origin| origin.region_id) {
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

#[test]
fn solid_bucket_forces_unlimited_anchor_while_sparse_obeys_the_key() {
    let sparse = vec![path(0.0, 10.0, 2.0, 0.4), path(0.0, 10.0, 8.0, 0.4)];
    let mut solid_a = path(0.0, 10.0, 2.0, 0.4);
    solid_a.role = ExtrusionRole::TopSolidInfill;
    let mut solid_b = path(0.0, 10.0, 8.0, 0.4);
    solid_b.role = ExtrusionRole::TopSolidInfill;

    let prior = vec![
        // exhaustive: this fixture explicitly sets every InfillRegion field used by orchestration.
        InfillRegion {
            object_id: "object".to_string(),
            region_id: 1,
            sparse_infill: sparse,
            solid_infill: vec![solid_a, solid_b],
            ironing: vec![],
            internal_bridge_infill: Vec::new(),
        },
    ];
    let views = vec![solid_view_with_anchor(
        1,
        square(0.0, 10.0),
        None,
        0,
        0.4,
        0.2,
        2.0,
        1.0,
    )];
    let module_config = config_with_anchor(0.4, 0.2, 2.0, 1.0);
    let output = run_with_config(&prior, &views, &module_config);

    assert_eq!(
        output.sparse_paths().len(),
        2,
        "the sparse bucket must obey the one-millimetre anchor maximum"
    );
    assert_eq!(
        output.solid_paths().len(),
        1,
        "solid paths must retain unlimited whole-arc linking"
    );
}

#[test]
fn absent_anchor_keys_fall_back_to_four_hundred_percent_of_flow_spacing() {
    let base = line_width_to_spacing(0.4, 0.2).unwrap();
    let prior = vec![sparse_region(
        1,
        vec![vertical_path(1.0, 0.4), vertical_path(29.0, 0.4)],
    )];
    let views = vec![view(1, square(0.0, 30.0), None, 0, 0.4, 0.2)];

    let sparse = run(&prior, &views);
    let dense_config = config(0.4, 0.8);
    let dense = run_with_config(&prior, &views, &dense_config);

    let input_endpoints = [(1.0, 0.0999), (1.0, 9.9001), (29.0, 0.0999), (29.0, 9.9001)];
    let mut sparse_anchor_lengths = sparse
        .sparse_paths()
        .iter()
        .flat_map(|path| contour_stub_lengths(path, &input_endpoints))
        .collect::<Vec<_>>();
    let mut dense_anchor_lengths = dense
        .sparse_paths()
        .iter()
        .flat_map(|path| contour_stub_lengths(path, &input_endpoints))
        .collect::<Vec<_>>();
    sparse_anchor_lengths.sort_by(f32::total_cmp);
    dense_anchor_lengths.sort_by(f32::total_cmp);

    let expected_anchor_length = 4.0 * base;
    assert!((expected_anchor_length - 1.4283185).abs() < 1e-4);
    let anchor_length = *sparse_anchor_lengths
        .first()
        .expect("fallback anchor must emit a contour stub");
    assert!((anchor_length - 1.4283185).abs() < 1e-4);
    assert_eq!(
        sparse_anchor_lengths, dense_anchor_lengths,
        "infill_density alone must not change the fallback anchor length"
    );
}

#[test]
fn cross_tool_paths_not_compatible_in_orchestrate() {
    // ADR-0058: the per-path authored `tool_index` is a region-compatibility
    // axis in `paths_compatible`. This is a DIFFERENT axis from the
    // region-level `RegionRecord.tool_index` covered by
    // `different_tool_never_connected`: here both regions share region tool 0
    // and differ only in the authored tool carried by their paths.
    //
    // Observable: two wall-sharing regions that are compatible are linked as a
    // union group against the union boundary, so their paths run all the way to
    // the shared wall at x = 10 (and can chain into one spanning path). When
    // `paths_compatible` rejects them, each region is linked against its own
    // inset boundary instead, so region 1's paths stop short of x = 10.
    struct Linked {
        spans_both_regions: bool,
        max_x_of_first_region_paths: f32,
    }

    fn link_with_path_tools(first_tool: Option<u32>, second_tool: Option<u32>) -> Linked {
        let paths_a = (1..=5)
            .map(|index| {
                let mut path = path(0.0, 10.0, index as f32, 0.4);
                path.tool_index = first_tool;
                path
            })
            .collect::<Vec<_>>();
        let paths_b = (1..=5)
            .map(|index| {
                let mut path = path(6.0, 15.0, index as f32, 0.4);
                path.tool_index = second_tool;
                path
            })
            .collect::<Vec<_>>();
        let prior = vec![sparse_region(1, paths_a), sparse_region(2, paths_b)];
        let views = vec![
            view(1, square(0.0, 10.0), Some(100), 0, 0.4, 0.2),
            view(2, square(5.0, 10.0), Some(100), 0, 0.4, 0.2),
            view(100, square(0.0, 15.0), None, 0, 0.4, 0.2),
        ];
        let output = run(&prior, &views);

        let spans_both_regions = output.sparse_paths().iter().any(|path| {
            path.points.iter().any(|point| point.x <= 0.1)
                && path.points.iter().any(|point| point.x >= 14.9)
        });
        let max_x_of_first_region_paths = output
            .sparse_paths()
            .iter()
            .filter(|path| path.tool_index == first_tool)
            .flat_map(|path| path.points.iter())
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max);
        Linked {
            spans_both_regions,
            max_x_of_first_region_paths,
        }
    }

    // Control: identical geometry with a shared authored tool IS compatible, so
    // the regions union-link across the shared wall.
    let same_tool = link_with_path_tools(Some(0), Some(0));
    assert!(
        same_tool.spans_both_regions,
        "control: same-tool wall-sharing regions must union-link into one spanning path"
    );
    assert!(
        same_tool.max_x_of_first_region_paths >= 9.99,
        "control: union linking must run region 1's paths up to the shared wall, got {}",
        same_tool.max_x_of_first_region_paths
    );

    // Same geometry, differing authored tools: incompatible, so each region is
    // linked on its own inset boundary and nothing spans the shared wall.
    let cross_tool = link_with_path_tools(Some(0), Some(1));
    assert!(
        !cross_tool.spans_both_regions,
        "differing per-path tool_index must prevent cross-region union linking"
    );
    assert!(
        cross_tool.max_x_of_first_region_paths <= 9.95,
        "incompatible regions must be linked on their own inset boundary, got {}",
        cross_tool.max_x_of_first_region_paths
    );
}
