use slicer_ir::CURRENT_MESH_IR_SCHEMA_VERSION;
use slicer_runtime::SliceRunOptions;

#[test]
fn default_is_a_quiet_empty_baseline() {
    let options = SliceRunOptions::default();

    assert_eq!(options.mesh.schema_version, CURRENT_MESH_IR_SCHEMA_VERSION);
    assert!(options.mesh.objects.is_empty());
    assert!(options.model_label.is_empty());
    assert!(options.config_path.is_none());
    assert!(options.output_path.is_none());
    assert!(options.thumbnail.is_none());
    assert!(options.report.is_none());
    assert!(options.cancel_flag.is_none());
    assert!(options.module_dirs.is_empty());
    assert!(options.config_overrides.is_empty());
    assert!(!options.no_default_module_paths);
    assert!(!options.report_verbose);
    assert!(!options.instrument_stderr);
    assert!(!options.profile);
    assert!(!options.profile_verbose);
    assert!(!options.progress_events);
}

#[test]
fn default_supports_field_remainder_initialization() {
    let options = SliceRunOptions {
        profile: true,
        ..Default::default()
    };

    assert!(options.profile);
}
