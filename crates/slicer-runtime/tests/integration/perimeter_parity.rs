//! Structural perimeter integration checks backed by the real pipeline.

use std::path::{Path, PathBuf};

pub use crate::common::perimeter_harness::{run_pipeline_capturing_perimeters, WallGenerator};
use slicer_core::flow::{flow_to_width, line_width_to_spacing};
use slicer_ir::{PerimeterIR, WallBoundaryType};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/perimeter_parity")
        .join(name)
}

type Tri = [[f32; 3]; 3];

fn triangle_normal(tri: &Tri) -> [f32; 3] {
    let [a, b, c] = *tri;
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 1e-9 {
        [n[0] / len, n[1] / len, n[2] / len]
    } else {
        [0.0, 0.0, 0.0]
    }
}

fn write_binary_stl(path: &Path, triangles: &[Tri]) {
    let mut buf = Vec::with_capacity(84 + triangles.len() * 50);
    buf.extend_from_slice(&[0u8; 80]);
    buf.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
    for tri in triangles {
        for coordinate in triangle_normal(tri) {
            buf.extend_from_slice(&coordinate.to_le_bytes());
        }
        for vertex in tri {
            for coordinate in *vertex {
                buf.extend_from_slice(&coordinate.to_le_bytes());
            }
        }
        buf.extend_from_slice(&0u16.to_le_bytes());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create fixture dir");
    }
    std::fs::write(path, buf).expect("failed to write STL fixture file");
}

fn write_config_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create fixture dir");
    }
    let text = serde_json::to_string_pretty(value).expect("config JSON must serialize");
    std::fs::write(path, text).expect("failed to write config fixture file");
}

fn prism(bottom: [[f32; 3]; 4], top: [[f32; 3]; 4]) -> Vec<Tri> {
    let [b0, b1, b2, b3] = bottom;
    let [t0, t1, t2, t3] = top;
    let mut triangles = vec![[b0, b2, b1], [b0, b3, b2], [t0, t1, t2], [t0, t2, t3]];
    let mut side = |bi, bj, ti, tj| {
        triangles.push([bi, bj, tj]);
        triangles.push([bi, tj, ti]);
    };
    side(b0, b1, t0, t1);
    side(b1, b2, t1, t2);
    side(b2, b3, t2, t3);
    side(b3, b0, t3, t0);
    triangles
}

fn solid_box(min: [f32; 3], max: [f32; 3]) -> Vec<Tri> {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    prism(
        [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
    )
}

fn annulus_frame_mesh() -> Vec<Tri> {
    let (z0, z1) = (0.0f32, 3.0f32);
    let outer = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let hole = [[6.0, 6.0], [14.0, 6.0], [14.0, 14.0], [6.0, 14.0]];
    let point = |xy: [f32; 2], z: f32| -> [f32; 3] { [xy[0], xy[1], z] };
    let mut triangles = Vec::new();
    for i in 0..4 {
        let j = (i + 1) % 4;
        triangles.push([
            point(outer[i], z0),
            point(outer[j], z0),
            point(outer[j], z1),
        ]);
        triangles.push([
            point(outer[i], z0),
            point(outer[j], z1),
            point(outer[i], z1),
        ]);
        triangles.push([point(hole[j], z0), point(hole[i], z0), point(hole[i], z1)]);
        triangles.push([point(hole[j], z0), point(hole[i], z1), point(hole[j], z1)]);
        triangles.push([point(outer[i], z0), point(hole[j], z0), point(outer[j], z0)]);
        triangles.push([point(outer[i], z0), point(hole[i], z0), point(hole[j], z0)]);
        triangles.push([point(outer[i], z1), point(outer[j], z1), point(hole[j], z1)]);
        triangles.push([point(outer[i], z1), point(hole[j], z1), point(hole[i], z1)]);
    }
    triangles
}

#[test]
fn annulus_true_hole_produces_inner_perimeters() {
    let dir = std::env::temp_dir().join("pnp_annulus_true_hole");
    std::fs::create_dir_all(&dir).expect("mk temp dir");
    let mesh_path = dir.join("annulus_frame.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &annulus_frame_mesh());
    write_config_json(
        &config_path,
        &serde_json::json!({ "layer_height": 0.2, "first_layer_height": 0.2 }),
    );
    let perimeters = run_pipeline_capturing_perimeters(
        &mesh_path,
        &config_path,
        &[core_modules_dir()],
        WallGenerator::Classic,
    )
    .expect("annulus_frame real pipeline run must succeed");
    let max_walls = perimeters
        .iter()
        .flat_map(|perimeter| perimeter.regions.iter())
        .map(|region| region.walls.len())
        .max()
        .unwrap_or(0);
    assert_eq!(
        max_walls, 6,
        "true-hole frame must yield 6 wall loops (3 outer + 3 hole); got {max_walls}"
    );
    let has_hole_wall = perimeters.iter().any(|perimeter| {
        perimeter.regions.iter().any(|region| {
            region.walls.iter().any(|wall| {
                !wall.path.points.is_empty()
                    && wall.path.points.iter().all(|point| {
                        point.x >= 4.0 && point.x <= 16.0 && point.y >= 4.0 && point.y <= 16.0
                    })
            })
        })
    });
    assert!(
        has_hole_wall,
        "expected a wall loop confined to the frame interior; none found"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn arachne_outer_wall_boundary_type_survives_wasm_boundary() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mesh_path = tmp.path().join("mesh.stl");
    let config_path = tmp.path().join("config.json");
    write_binary_stl(&mesh_path, &solid_box([0.0, 0.0, 0.0], [10.0, 10.0, 1.0]));
    write_config_json(
        &config_path,
        &serde_json::json!({
            "layer_height": 0.2,
            "first_layer_height": 0.2,
            "wall_generator": "arachne"
        }),
    );
    let perimeters = run_pipeline_capturing_perimeters(
        &mesh_path,
        &config_path,
        &[core_modules_dir()],
        WallGenerator::Arachne,
    )
    .expect("arachne WASM pipeline run must succeed");
    let outer_wall = perimeters
        .iter()
        .flat_map(|perimeter| perimeter.regions.iter())
        .flat_map(|region| region.walls.iter())
        .find(|wall| wall.perimeter_index == 0)
        .expect("a perimeter_index == 0 wall loop must be emitted");
    assert_eq!(
        outer_wall.boundary_type,
        WallBoundaryType::ExteriorSurface,
        "outer wall boundary_type must survive the WASM boundary"
    );
}

fn run_arachne_fixture(dir: &Path, mesh_filename: &str) -> Vec<PerimeterIR> {
    let actual = run_pipeline_capturing_perimeters(
        &dir.join(mesh_filename),
        &dir.join("config.json"),
        &[core_modules_dir()],
        WallGenerator::Arachne,
    )
    .unwrap_or_else(|error| panic!("{}: real pipeline run must succeed: {error}", dir.display()));
    assert!(
        !actual.is_empty(),
        "{}: capture must contain layers",
        dir.display()
    );
    for (layer_index, perimeter) in actual.iter().enumerate() {
        for (region_index, region) in perimeter.regions.iter().enumerate() {
            for (wall_index, wall) in region.walls.iter().enumerate() {
                assert!(
                    wall.path.points.len() >= 2,
                    "{}: layer {layer_index}, region {region_index}, wall {wall_index} needs at least two points",
                    dir.display()
                );
                for point in &wall.path.points {
                    assert!(
                        point.x.is_finite()
                            && point.y.is_finite()
                            && point.z.is_finite()
                            && point.width.is_finite(),
                        "{}: layer {layer_index}, region {region_index}, wall {wall_index} has non-finite coordinates",
                        dir.display()
                    );
                }
            }
        }
    }
    actual
}

#[test]
fn arachne_perimeter_parity() {
    use std::collections::BTreeSet;

    {
        let dir = fixture_dir("tapered_wedge");
        let perimeters = run_arachne_fixture(&dir, "tapered_wedge.stl");
        let region = perimeters
            .iter()
            .flat_map(|perimeter| perimeter.regions.iter())
            .find(|region| !region.walls.is_empty())
            .expect("tapered_wedge: at least one region with walls must be captured");
        assert!(
            region.walls.len() > 1,
            "tapered_wedge: expected more than one WallLoop from the SKT graph, got {}",
            region.walls.len()
        );
        const CONFIGURED_LINE_WIDTH_MM: f32 = 0.4;
        let expected_width_mm = flow_to_width(
            line_width_to_spacing(CONFIGURED_LINE_WIDTH_MM, 0.2).unwrap(),
            0.2,
        );
        const FLOW_SPACING_TOLERANCE_MM: f32 = 0.01;
        let all_widths: Vec<f32> = region
            .walls
            .iter()
            .flat_map(|wall| wall.width_profile.widths.iter().copied())
            .collect();
        assert!(
            !all_widths.is_empty(),
            "tapered_wedge: at least one width sample must be present"
        );
        for width in all_widths {
            assert!(
                (width - expected_width_mm).abs() < FLOW_SPACING_TOLERANCE_MM,
                "tapered_wedge: every captured width must equal the configured wall line width \
                 ({expected_width_mm}mm +/- {FLOW_SPACING_TOLERANCE_MM}mm), got {width}mm"
            );
        }
    }

    {
        let dir = fixture_dir("narrow_strip_widening");
        let perimeters = run_arachne_fixture(&dir, "narrow_strip_widening.stl");
        let first_walled_layer = perimeters
            .iter()
            .find(|perimeter| {
                perimeter
                    .regions
                    .iter()
                    .any(|region| !region.walls.is_empty())
            })
            .expect(
                "narrow_strip_widening: expected >= 1 rescued wall (Widening strategy), got 0 \
                 walls across all layers — the thin feature was dropped instead of widened",
            );
        const INITIAL_LAYER_MIN_MM: f32 = 0.34;
        const MIN_BEAD_WIDTH_MM: f32 = 0.4;
        const CLAMP_TOLERANCE_MM: f32 = 0.05;
        let initial_widths: Vec<f32> = first_walled_layer
            .regions
            .iter()
            .filter(|region| !region.walls.is_empty())
            .flat_map(|region| region.walls[0].width_profile.widths.iter().copied())
            .collect();
        assert!(
            !initial_widths.is_empty(),
            "narrow_strip_widening: initial-layer rescued wall must carry >= 1 width sample"
        );
        assert!(
            initial_widths
                .iter()
                .all(|width| (*width - INITIAL_LAYER_MIN_MM).abs() < CLAMP_TOLERANCE_MM),
            "narrow_strip_widening: initial-layer widths must be clamped toward \
             initial_layer_min_bead_width ({INITIAL_LAYER_MIN_MM}mm +/- {CLAMP_TOLERANCE_MM}mm): \
             {initial_widths:?}"
        );
        let later_widths: Vec<f32> = perimeters
            .iter()
            .filter(|perimeter| {
                perimeter.global_layer_index > first_walled_layer.global_layer_index
            })
            .flat_map(|perimeter| perimeter.regions.iter())
            .filter(|region| !region.walls.is_empty())
            .flat_map(|region| region.walls[0].width_profile.widths.iter().copied())
            .collect();
        assert!(
            !later_widths.is_empty(),
            "narrow_strip_widening: expected at least one non-initial layer with walls"
        );
        assert!(
            later_widths
                .iter()
                .all(|width| (*width - MIN_BEAD_WIDTH_MM).abs() < CLAMP_TOLERANCE_MM),
            "narrow_strip_widening: non-initial widths must be clamped toward min_bead_width \
             ({MIN_BEAD_WIDTH_MM}mm +/- {CLAMP_TOLERANCE_MM}mm): {later_widths:?}"
        );
    }

    {
        let dir = fixture_dir("max_bead_count_cap");
        let perimeters = run_arachne_fixture(&dir, "max_bead_count_cap.stl");
        let total_walls: usize = perimeters
            .iter()
            .flat_map(|perimeter| perimeter.regions.iter())
            .map(|region| region.walls.len())
            .sum();
        assert!(
            total_walls > 0,
            "max_bead_count_cap: expected an emitted wall"
        );
        let mut max_seen = 0;
        for wall in perimeters
            .iter()
            .flat_map(|perimeter| perimeter.regions.iter())
            .flat_map(|region| region.walls.iter())
        {
            max_seen = max_seen.max(wall.perimeter_index);
            assert!(
                wall.perimeter_index <= 9,
                "max_bead_count_cap: wall index {} exceeds cap 9",
                wall.perimeter_index
            );
        }
        assert!(
            max_seen <= 9,
            "max_bead_count_cap: observed cap must be <= 9"
        );
    }

    {
        let dir = fixture_dir("complex_multi_feature");
        let perimeters = run_arachne_fixture(&dir, "complex_multi_feature.stl");
        let region = perimeters
            .iter()
            .flat_map(|perimeter| perimeter.regions.iter())
            .find(|region| !region.walls.is_empty())
            .expect("complex_multi_feature: a walled region is required");
        assert!(
            region.walls.len() > 1,
            "complex_multi_feature: expected multiple wall loops, got {}",
            region.walls.len()
        );
    }

    {
        let dir = fixture_dir("cube_4color_arachne");
        let source_mesh = repo_root().join("resources").join("cube_4color.3mf");
        let mesh_path = dir.join("cube_4color.3mf");
        std::fs::copy(&source_mesh, &mesh_path)
            .unwrap_or_else(|error| panic!("failed to copy cube_4color.3mf: {error}"));
        let perimeters = run_arachne_fixture(&dir, "cube_4color.3mf");
        let tool_indices: BTreeSet<u32> = perimeters
            .iter()
            .flat_map(|perimeter| perimeter.regions.iter())
            .map(|region| region.region_id as u32)
            .collect();
        assert!(
            tool_indices.len() >= 4,
            "cube_4color_arachne: expected at least 4 tool indices, got {tool_indices:?}"
        );
    }
}
