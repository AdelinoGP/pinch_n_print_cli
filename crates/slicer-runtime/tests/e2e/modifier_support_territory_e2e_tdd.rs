//! SchemaBridgeMap ticket 19 (reopened): a modifier that flips `support_type`
//! from the global `tree(auto)` to `normal(auto)` over the +y half of
//! `SupportTest` must yield BOTH families, each on its own side.
//!
//! Before the territory fix the tree planner's branches drifted into the free
//! air under the modifier half, the traditional planner's column covered the
//! same air, and the host's cross-family guard rejected both sides on every
//! overlapping layer: the tree stopped at z = 18.0 and no traditional support
//! was published at all. The ticket-19 e2e only covered the inverse direction
//! (traditional base, thin tree modifier band), so nothing caught it.
//!
//! Fixture: `resources/support_test_modifier_normal_in_tree.3mf` — the
//! user's `SupportTest.3mf` with project settings and thumbnails stripped —
//! under the three-key config below. Every threshold here was measured on
//! the fixed backend: the overhang plane sits at z = 25.2 (layer 125), the
//! last interface layer at z = 24.8 (layer 123), and the modifier footprint
//! is the +y half (y >= 98.6 mm) of the object.
//!
//! Authoritative pipe commands:
//!   `cargo test -p slicer-runtime --test e2e -- modifier_support_territory`

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use pnp_cli_locator::pnp_cli_bin;
use slicer_ir::{ConfigValue, ExPolygon, SupportPlanEntry, SupportPlanRole};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn fixture() -> PathBuf {
    workspace_root().join("resources/support_test_modifier_normal_in_tree.3mf")
}

fn config() -> HashMap<String, ConfigValue> {
    HashMap::from([
        ("enable_support".to_string(), ConfigValue::Bool(true)),
        (
            "support_type".to_string(),
            ConfigValue::String("tree(auto)".to_string()),
        ),
        ("layer_height".to_string(), ConfigValue::Float(0.2)),
    ])
}

/// Last support layer of the fixture: the interface band ends at z = 24.8
/// (one `support_top_z_distance` under the z = 25.2 overhang plane).
const TOP_SUPPORT_LAYER: u32 = 123;
/// Lowest layer the tree must still be interfacing on (z = 24.6).
const INTERFACE_BAND_BOTTOM: u32 = 122;

fn area(poly: &ExPolygon) -> f64 {
    fn ring(points: &[slicer_ir::Point2]) -> f64 {
        points
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let b = &points[(i + 1) % points.len()];
                (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64)
            })
            .sum::<f64>()
            .abs()
            * 0.5
    }
    ring(&poly.contour.points) - poly.holes.iter().map(|h| ring(&h.points)).sum::<f64>()
}

fn roles_of(entry: &SupportPlanEntry) -> Vec<ExPolygon> {
    entry
        .roles
        .iter()
        .flat_map(|role| role.regions.iter().cloned())
        .collect()
}

fn overlap_area(a: &[ExPolygon], b: &[ExPolygon]) -> f64 {
    slicer_core::polygon_ops::intersection(a, b)
        .iter()
        .map(area)
        .sum()
}

fn prepass(mesh: slicer_ir::MeshIR) -> slicer_runtime::run::PrepassContext {
    let root = workspace_root();
    slicer_runtime::run::prepare_prepass_context(
        Arc::new(mesh),
        config(),
        &[root.join("modules/core-modules")],
        true,
        false,
    )
    .expect("prepass must succeed")
}

fn tree_top_interface_layer(entries: &[SupportPlanEntry]) -> Option<u32> {
    entries
        .iter()
        .filter(|entry| {
            entry.family_id == "tree"
                && entry.decline_reason.is_none()
                && entry.roles.iter().any(|role| {
                    role.role == SupportPlanRole::TopInterface && !role.regions.is_empty()
                })
        })
        .map(|entry| entry.global_layer_index as u32)
        .max()
}

/// Prepass level: territory is published for the minted sub-region, the
/// traditional column fills it on every support layer, the tree keeps out of
/// it and still reaches the overhang.
#[test]
fn modifier_support_territory_clips_families_to_their_own_side() {
    let mesh = slicer_model_io::load_model(&fixture()).expect("load fixture");
    let object_id = mesh.objects[0].id.clone();
    assert_eq!(
        mesh.objects[0].modifier_volumes.len(),
        1,
        "fixture carries exactly one parameter modifier"
    );
    let ctx = prepass(mesh);
    let analysis = ctx
        .blackboard
        .support_analysis()
        .expect("support analysis committed");

    let sub_region_ids: BTreeSet<u64> = analysis
        .family_assignments
        .iter()
        .filter_map(|((object, region_id), family)| {
            (object == &object_id
                && family == "traditional"
                && slicer_ir::is_modifier_namespace_id(*region_id))
            .then_some(*region_id)
        })
        .collect();
    assert!(
        !sub_region_ids.is_empty(),
        "modifier must mint a traditional-family sub-region; assignments={:?}",
        analysis.family_assignments
    );
    let territory_layers: BTreeSet<u32> = analysis
        .support_territory
        .keys()
        .filter(|key| key.object_id == object_id && sub_region_ids.contains(&key.region_id))
        .map(|key| key.global_support_layer_index)
        .collect();
    for layer in 0..=TOP_SUPPORT_LAYER {
        assert!(
            territory_layers.contains(&layer),
            "territory must be published on every support layer; missing {layer}, have {:?}",
            territory_layers
        );
    }
    let territory_at = |layer: u32| -> Vec<ExPolygon> {
        analysis
            .support_territory
            .iter()
            .filter(|(key, _)| {
                key.object_id == object_id
                    && key.global_support_layer_index == layer
                    && sub_region_ids.contains(&key.region_id)
            })
            .flat_map(|(_, polys)| polys.iter().cloned())
            .collect()
    };

    let plan = ctx
        .blackboard
        .support_plan()
        .expect("support plan committed");

    // Traditional: present on every support layer, inside its footprint.
    for layer in 0..=TOP_SUPPORT_LAYER {
        let traditional: Vec<&SupportPlanEntry> = plan
            .entries
            .iter()
            .filter(|entry| {
                entry.object_id == object_id
                    && entry.family_id == "traditional"
                    && entry.decline_reason.is_none()
                    && entry.global_layer_index == layer as i32
                    && sub_region_ids.contains(&entry.region_id)
                    && entry.roles.iter().any(|role| !role.regions.is_empty())
            })
            .collect();
        assert!(
            !traditional.is_empty(),
            "traditional support must be planned for the modifier sub-region at layer {layer}"
        );
        let footprint = territory_at(layer);
        for entry in traditional {
            let outside = slicer_core::polygon_ops::difference(&roles_of(entry), &footprint);
            let outside_area: f64 = outside.iter().map(area).sum();
            assert!(
                outside_area < 1.0,
                "traditional roles must stay inside the modifier footprint at layer {layer}; \
                 outside area {outside_area}"
            );
        }
    }

    // Tree: never inside the footprint, still reaching the overhang.
    let tree: Vec<&SupportPlanEntry> = plan
        .entries
        .iter()
        .filter(|entry| {
            entry.object_id == object_id
                && entry.family_id == "tree"
                && entry.decline_reason.is_none()
        })
        .collect();
    for entry in &tree {
        let layer = entry.global_layer_index as u32;
        let inside = overlap_area(&roles_of(entry), &territory_at(layer));
        assert!(
            inside < 1.0,
            "tree roles must have no area inside the modifier footprint at layer {layer}; \
             inside area {inside}"
        );
    }
    let top = tree_top_interface_layer(&plan.entries)
        .expect("tree must publish a top interface somewhere");
    assert!(
        top >= INTERFACE_BAND_BOTTOM,
        "tree must reach the overhang with a top interface (layer >= {INTERFACE_BAND_BOTTOM}); \
         highest interface layer {top}"
    );
}

/// Control: the same fixture with the modifier removed plans the tree to the
/// same height and publishes no territory.
#[test]
fn modifier_support_territory_control_without_modifier_is_unchanged() {
    let mut mesh = slicer_model_io::load_model(&fixture()).expect("load fixture");
    mesh.objects[0].modifier_volumes.clear();
    let ctx = prepass(mesh);
    let analysis = ctx
        .blackboard
        .support_analysis()
        .expect("support analysis committed");
    assert!(analysis.support_territory.is_empty());
    let plan = ctx
        .blackboard
        .support_plan()
        .expect("support plan committed");
    assert!(plan.entries.iter().all(|entry| entry.family_id == "tree"));
    let top = tree_top_interface_layer(&plan.entries).expect("tree interface");
    assert!(top >= INTERFACE_BAND_BOTTOM, "control tree top {top}");
}

/// Per-layer support extrusion summary (`;TYPE:Support` and
/// `;TYPE:Support interface`) from G-code: `(z, count, any point with
/// x > 105.3 && y > 98.6)`.
fn support_layers(gcode: &str) -> Vec<(f32, usize, bool)> {
    let mut layers: Vec<(f32, usize, bool)> = Vec::new();
    let mut role: Option<&str> = None;
    for raw in gcode.lines() {
        let line = raw.trim();
        if let Some(z) = line.strip_prefix(";Z:") {
            layers.push((z.parse::<f32>().unwrap_or(f32::NAN), 0, false));
            role = None;
        } else if let Some(kind) = line.strip_prefix(";TYPE:") {
            role = Some(kind.trim());
        } else if matches!(role, Some("Support" | "Support interface"))
            && line.starts_with("G1")
            && line.contains(" E")
            && line.contains('X')
        {
            let Some(current) = layers.last_mut() else {
                continue;
            };
            current.1 += 1;
            let mut x = None;
            let mut y = None;
            for word in line.split_whitespace() {
                if let Some(v) = word.strip_prefix('X') {
                    x = v.parse::<f32>().ok();
                } else if let Some(v) = word.strip_prefix('Y') {
                    y = v.parse::<f32>().ok();
                }
            }
            if let (Some(x), Some(y)) = (x, y) {
                if x > 105.3 && y > 98.6 {
                    current.2 = true;
                }
            }
        }
    }
    layers
}

/// G-code level, through the real `pnp_cli` binary and the shipped module
/// set: support reaches z >= 24.6 and the modifier half carries support.
#[test]
fn modifier_support_territory_gcode_has_support_on_both_sides_up_to_the_overhang() {
    let root = workspace_root();
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    std::fs::create_dir_all(&target).expect("target dir");
    let config_path = target.join("modifier_support_territory_config.json");
    std::fs::write(
        &config_path,
        r#"{"enable_support": true, "support_type": "tree(auto)", "layer_height": 0.2}"#,
    )
    .expect("write config");
    let output_path = target.join("modifier_support_territory.gcode");
    let bin = pnp_cli_bin();
    let output = Command::new(&bin)
        .args(["slice", "--model"])
        .arg(fixture())
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--module-dir")
        .arg(root.join("modules/core-modules"))
        .arg("--instrument-stderr")
        .output()
        .expect("pnp_cli must execute");
    assert!(
        output.status.success(),
        "pnp_cli slice failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The plan's red signal: the reject-both guard used to mint 2121 of these
    // on this fixture, each carrying a 1200 "support demand unmet" warning.
    // Territory arbitrates by clipping (Info 1205), so both must be gone.
    let events = String::from_utf8_lossy(&output.stderr);
    let rejections = events.matches("cross-family positive-area overlap").count();
    assert_eq!(
        rejections, 0,
        "cross-family arbitration must clip, not reject"
    );
    let unmet = events.matches("\"code\":1200").count();
    assert_eq!(unmet, 0, "no support demand may be reported unmet");
    let gcode = std::fs::read_to_string(&output_path).expect("read gcode");
    let layers = support_layers(&gcode);
    let with_support: Vec<&(f32, usize, bool)> =
        layers.iter().filter(|layer| layer.1 > 0).collect();
    let top_z = with_support
        .iter()
        .map(|layer| layer.0)
        .fold(f32::MIN, f32::max);
    assert!(
        top_z >= 24.6,
        "support must reach the overhang (z >= 24.6); last support layer z = {top_z}"
    );
    let modifier_half_layers = with_support.iter().filter(|layer| layer.2).count();
    assert!(
        modifier_half_layers > 0,
        "support must be printed in the modifier half (x > 105.3, y > 98.6)"
    );

    // Control: the same object with the modifier volume removed still reaches
    // the overhang, and carries no support in the half the modifier used to
    // own beyond what the tree already put there. The point is that the
    // territory machinery is inert without a modifier.
    let mut control_mesh = slicer_model_io::load_model(&fixture()).expect("load fixture");
    control_mesh.objects[0].modifier_volumes.clear();
    let control_model = target.join("modifier_support_territory_control.3mf");
    slicer_model_io::write_3mf(
        &control_mesh,
        std::fs::File::create(&control_model).expect("create control 3mf"),
    )
    .expect("write control 3mf");
    let control_out = target.join("modifier_support_territory_control.gcode");
    let control_run = Command::new(&bin)
        .args(["slice", "--model"])
        .arg(&control_model)
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(&control_out)
        .arg("--module-dir")
        .arg(root.join("modules/core-modules"))
        .output()
        .expect("pnp_cli must execute");
    assert!(
        control_run.status.success(),
        "control slice failed: {}",
        String::from_utf8_lossy(&control_run.stderr)
    );
    let control_layers = support_layers(&std::fs::read_to_string(&control_out).expect("read"));
    let control_top = control_layers
        .iter()
        .filter(|layer| layer.1 > 0)
        .map(|layer| layer.0)
        .fold(f32::MIN, f32::max);
    assert!(
        control_top >= 24.6,
        "control must still reach the overhang; last support layer z = {control_top}"
    );
    // Every layer from the plate to the interface band carries support in
    // the modifier half: the traditional column is a straight prism.
    let missing: Vec<f32> = layers
        .iter()
        .filter(|layer| layer.0 <= 24.6 && !layer.2)
        .map(|layer| layer.0)
        .collect();
    assert!(
        missing.is_empty(),
        "modifier-half support missing on layers {missing:?}"
    );
}

/// Per-layer `(z, outer wall loops, inner wall loops, role names)` where a
/// loop is a maximal run of extruding `G1`s inside one `;TYPE:` block.
fn wall_layers(gcode: &str) -> Vec<(f32, u32, u32, BTreeSet<String>)> {
    let mut layers: Vec<(f32, u32, u32, BTreeSet<String>)> = Vec::new();
    let mut role: Option<String> = None;
    let mut prev_extruding = false;
    for raw in gcode.lines() {
        let line = raw.trim();
        if let Some(z) = line.strip_prefix(";Z:") {
            layers.push((z.parse::<f32>().unwrap_or(f32::NAN), 0, 0, BTreeSet::new()));
            role = None;
            prev_extruding = false;
        } else if let Some(kind) = line.strip_prefix(";TYPE:") {
            role = Some(kind.trim().to_string());
            prev_extruding = false;
            if let Some(current) = layers.last_mut() {
                current.3.insert(kind.trim().to_string());
            }
        } else if line.starts_with("G1") || line.starts_with("G0") {
            let extruding = line.starts_with("G1") && line.contains(" E") && line.contains('X');
            if extruding && !prev_extruding {
                if let Some(current) = layers.last_mut() {
                    match role.as_deref() {
                        Some("Outer wall") => current.1 += 1,
                        Some("Inner wall") => current.2 += 1,
                        _ => {}
                    }
                }
            }
            prev_extruding = extruding;
        }
    }
    layers
}

fn slice_with_gui_config(model: &PathBuf, name: &str) -> String {
    let root = workspace_root();
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    std::fs::create_dir_all(&target).expect("target dir");
    let output_path = target.join(format!("modifier_support_territory_{name}.gcode"));
    let output = Command::new(pnp_cli_bin())
        .args(["slice", "--model"])
        .arg(model)
        .arg("--config")
        .arg(root.join("resources/support_test_modifier_normal_in_tree_gui.config.json"))
        .arg("--output")
        .arg(&output_path)
        .arg("--module-dir")
        .arg(root.join("modules/core-modules"))
        .output()
        .expect("pnp_cli must execute");
    assert!(
        output.status.success(),
        "pnp_cli slice failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read gcode")
}

/// Ticket 19 residuals R1 and R2, under the GUI-translated project config
/// (`only_one_wall_top`, three top shells, thick internal bridges):
///
/// - R1: the two shell layers under the top (z 29.6 / 29.8) used to print
///   with no inner walls (29.8 with no walls at all — the perimeter guest's
///   `only_one_wall_top` second pass failed and discarded every wall) and
///   29.6 as one whole-layer external `Bridge`. Both were caused by the guest
///   never receiving `internal_solid_fill`, so the whole shell shadow read as
///   exposed top; the bridge role was the infill module emitting the qualified
///   internal-bridge sites as external bridges.
/// - R2: with the modifier the wall seam ran along the modifier edge on those
///   layers because only `polygons` was restored into the perimeter source
///   region; the wall structure must now match the modifier-free control.
#[test]
fn modifier_support_territory_top_shell_layers_keep_full_walls_and_internal_bridge_role() {
    let root = workspace_root();
    let fixture = root.join("resources/support_test_modifier_normal_in_tree_gui.3mf");
    let gcode = slice_with_gui_config(&fixture, "gui");
    let layers = wall_layers(&gcode);
    let at = |z: f32| {
        layers
            .iter()
            .find(|layer| (layer.0 - z).abs() < 0.01)
            .unwrap_or_else(|| panic!("layer z={z} missing"))
    };
    let reference = at(25.4);
    assert!(
        reference.1 >= 1 && reference.2 >= 2,
        "reference layer 25.4 must carry outer and inner walls: {reference:?}"
    );
    for z in [29.6_f32, 29.8] {
        let layer = at(z);
        assert_eq!(
            (layer.1, layer.2),
            (reference.1, reference.2),
            "z={z} must carry the same wall loops as the reference layer; roles {:?}",
            layer.3
        );
        assert!(
            !layer.3.contains("Bridge"),
            "z={z} must not print as an external bridge; roles {:?}",
            layer.3
        );
        assert!(
            layer.3.contains("Internal Bridge") || layer.3.contains("Internal solid infill"),
            "z={z} must be an internal bridge or internal solid shell; roles {:?}",
            layer.3
        );
    }
    let top = at(30.0);
    assert!(
        top.1 >= 1 && top.2 == 0,
        "only_one_wall_top keeps a single wall on the exposed top: {top:?}"
    );

    // R2 control: the same project (same embedded project settings) with the
    // modifier part removed must show the same wall structure.
    let control_path = root.join("resources/support_test_modifier_normal_in_tree_gui_control.3mf");
    let control = wall_layers(&slice_with_gui_config(&control_path, "gui_control"));
    for z in [29.6_f32, 29.8, 30.0] {
        let with_modifier = at(z);
        let without = control
            .iter()
            .find(|layer| (layer.0 - z).abs() < 0.01)
            .expect("control layer");
        assert_eq!(
            (with_modifier.1, with_modifier.2),
            (without.1, without.2),
            "z={z}: wall loops must match the modifier-free control"
        );
    }
}
