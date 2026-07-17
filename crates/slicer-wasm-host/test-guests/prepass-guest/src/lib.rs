wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:world-prepass/prepass-module",
    generate_all,
});

struct Component;

impl Guest for Component {
    fn run_mesh_analysis(_objects: Vec<ObjectId>, _output: MeshAnalysisOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_layer_planning(_objects: Vec<ObjectId>, _output: LayerPlanOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_seam_planning(_objects: Vec<MeshObjectView>, _output: SeamPlanningOutput, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_support_geometry(
        _objects: Vec<MeshObjectView>,
        _layer_plan: LayerPlanView,
        _region_segmentation: RegionSegmentationView,
        _support_geometry: SupportGeometryView,
        _output: SupportGeometryOutput,
        _config: ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

export!(Component);
