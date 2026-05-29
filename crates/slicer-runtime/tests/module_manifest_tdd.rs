#![allow(missing_docs)]

#[test]
fn perimeter_modules_declare_arc_tolerance() {
    for path in [
        "../../modules/core-modules/classic-perimeters/classic-perimeters.toml",
        "../../modules/core-modules/arachne-perimeters/arachne-perimeters.toml",
    ] {
        let abs_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
        let manifest_text = std::fs::read_to_string(&abs_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", abs_path.display(), e));
        let parsed: toml::Value = toml::from_str(&manifest_text)
            .unwrap_or_else(|e| panic!("toml parse error in {}: {}", abs_path.display(), e));
        let schema = &parsed["config"]["schema"]["perimeter_arc_tolerance"];
        assert_eq!(schema["type"].as_str(), Some("float"), "type in {}", path);
        assert_eq!(
            schema["default"].as_float(),
            Some(0.0125),
            "default in {}",
            path
        );
        assert_eq!(schema["min"].as_float(), Some(0.0), "min in {}", path);
        assert_eq!(schema["max"].as_float(), Some(1.0), "max in {}", path);
    }
}
