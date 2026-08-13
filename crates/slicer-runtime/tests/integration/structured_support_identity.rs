use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerStageCommit, Point3WithWidth, SupportEntry, SupportIR,
    SupportRole,
};
use slicer_runtime::layer_executor::{apply_for_test, StageApplyContext};
use slicer_runtime::LayerArena;

/// The support commit boundary must retain family/body/demand attribution while
/// carrying the actual path into the layer arena used by downstream stages.
pub fn structured_support_identity() {
    let path = ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 1.0,
                y: 2.0,
                z: 0.2,
                ..Default::default()
            },
            Point3WithWidth {
                x: 3.0,
                y: 4.0,
                z: 0.2,
                ..Default::default()
            },
        ],
        role: ExtrusionRole::SupportMaterial,
        speed_factor: 1.0,
    };
    let support = SupportIR {
        entries: vec![SupportEntry {
            family_id: "family-anchored".into(),
            body_id: "body-anchored".into(),
            demand_ids: vec!["demand-anchored".into()],
            object_id: "object-1".into(),
            region_id: 7,
            role: SupportRole::SupportBody,
            paths: vec![path.clone()],
        }],
        ..Default::default()
    };

    let mut arena = LayerArena::new();
    apply_for_test(
        &mut arena,
        LayerStageCommit::Support(support),
        &StageApplyContext {
            stage_id: "Layer::Support",
            module_id: "anchored-support-family",
            layer_index: 3,
            seam_plan: None,
        },
    )
    .expect("structured support commit must succeed");

    let committed = arena.support().expect("support must reach Layer::Support");
    let entry = &committed.entries[0];
    assert_eq!(entry.family_id, "family-anchored");
    assert_eq!(entry.body_id, "body-anchored");
    assert_eq!(entry.demand_ids, ["demand-anchored"]);
    assert_eq!(entry.object_id, "object-1");
    assert_eq!(entry.region_id, 7);
    assert_eq!(entry.role, SupportRole::SupportBody);
    assert_eq!(entry.paths, [path]);
}
