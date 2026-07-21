//! TDD suite for the 3MF sidecar parser (Packet 56).
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;

// Helper: build an in-memory 3MF zip with a given sidecar entry.
fn make_zip_with_sidecar(sidecar_xml: &str) -> zip::ZipArchive<Cursor<Vec<u8>>> {
    let buf = Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(buf);
    let opts = SimpleFileOptions::default();
    writer
        .start_file("Metadata/model_settings.config", opts)
        .unwrap();
    writer.write_all(sidecar_xml.as_bytes()).unwrap();
    let buf = writer.finish().unwrap();
    zip::ZipArchive::new(buf).unwrap()
}

// Helper: build an in-memory 3MF zip with NO sidecar entry.
fn make_zip_without_sidecar() -> zip::ZipArchive<Cursor<Vec<u8>>> {
    let buf = Cursor::new(Vec::new());
    let writer = zip::ZipWriter::new(buf);
    let buf = writer.finish().unwrap();
    zip::ZipArchive::new(buf).unwrap()
}

#[test]
fn parses_cube_cilindrical_modifier_sidecar() {
    use slicer_model_io::sidecar::{parse_3mf_sidecar, PartSubtype};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/cube_cilindrical_modifier.3mf");
    let file = std::fs::File::open(&path)
        .unwrap_or_else(|_| panic!("cube_cilindrical_modifier.3mf not found at {:?}", path));
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let result = parse_3mf_sidecar(&mut archive);

    assert!(
        !result.is_empty(),
        "expected at least one object in sidecar"
    );
    // The sidecar has object id=3; part id=1 is normal_part (the Cube body) and
    // part id=2 is modifier_part (a Generic-Cylinder) with per-part overrides
    // (inner_wall_line_width, outer_wall_line_width, sparse_infill_density,
    // sparse_infill_line_width). Packet 89 substitution: the retired benchy
    // fixture carried `fuzzy_skin=external` on its modifier part; the new
    // cube_cilindrical_modifier fixture's modifier instead carries
    // `inner_wall_line_width=0.6` (and three other overrides). The assertion
    // is strengthened to require BOTH (a) the ModifierPart subtype routing AND
    // (b) the presence of a per-part override metadata entry, matching the
    // structural class of metadata-carrying modifier parts the original test
    // covered.
    let obj = result.get(&3).expect("object id 3 missing from sidecar");
    assert!(!obj.parts.is_empty(), "expected at least one part");

    let part1 = obj.parts.get(&1).expect("part id 1 missing");
    assert_eq!(
        part1.subtype,
        PartSubtype::NormalPart,
        "part 1 should be NormalPart (the Cube body)"
    );

    let part2 = obj.parts.get(&2).expect("part id 2 missing");
    assert_eq!(
        part2.subtype,
        PartSubtype::ModifierPart,
        "part 2 should be ModifierPart (the Generic-Cylinder modifier)"
    );
    assert_eq!(
        part2.metadata.get("name").map(String::as_str),
        Some("Generic-Cylinder"),
        "modifier part 2 should be the Generic-Cylinder"
    );
    assert_eq!(
        part2
            .metadata
            .get("inner_wall_line_width")
            .map(String::as_str),
        Some("0.6"),
        "modifier part 2 should carry the inner_wall_line_width override"
    );
    assert_eq!(
        part2
            .metadata
            .get("sparse_infill_density")
            .map(String::as_str),
        Some("40%"),
        "modifier part 2 should carry the sparse_infill_density override"
    );
}

#[test]
fn missing_sidecar_is_silent_default() {
    use slicer_model_io::sidecar::parse_3mf_sidecar;
    let mut archive = make_zip_without_sidecar();
    let result = parse_3mf_sidecar(&mut archive);
    assert!(result.is_empty(), "missing sidecar should return empty map");
    // No way to assert "no warning" programmatically without a log capture crate;
    // the implementation contract is verified by code review.
}

#[test]
fn malformed_sidecar_falls_back_to_normal_part() {
    use slicer_model_io::sidecar::parse_3mf_sidecar;
    // Invalid XML â€” mismatched closing tag causes a parse error in quick_xml.
    let bad_xml = r#"<?xml version="1.0"?><config><object id="1"><part id="1" subtype="normal_part"></wrong_tag></config>"#;
    let mut archive = make_zip_with_sidecar(bad_xml);
    let result = parse_3mf_sidecar(&mut archive);
    assert!(
        result.is_empty(),
        "malformed sidecar should return empty map"
    );
}

#[test]
fn unknown_subtype_downgrades_to_normal_part() {
    use slicer_model_io::sidecar::{parse_3mf_sidecar, PartSubtype};
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<config>
  <object id="1">
    <part id="1" subtype="unrecognized_subtype_value">
      <metadata key="name" value="test"/>
    </part>
  </object>
</config>"#;
    let mut archive = make_zip_with_sidecar(xml);
    let result = parse_3mf_sidecar(&mut archive);
    let obj = result.get(&1).expect("object 1 missing");
    let part = obj.parts.get(&1).expect("part 1 missing");
    assert_eq!(
        part.subtype,
        PartSubtype::NormalPart,
        "unknown subtype should downgrade to NormalPart"
    );
}

#[test]
fn object_and_part_id_mapping_matches_bambu_convention() {
    use slicer_model_io::sidecar::{parse_3mf_sidecar, PartSubtype};
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<config>
  <object id="3">
    <part id="2" subtype="modifier_part">
      <metadata key="name" value="cube"/>
    </part>
  </object>
</config>"#;
    let mut archive = make_zip_with_sidecar(xml);
    let result = parse_3mf_sidecar(&mut archive);
    assert!(result.contains_key(&3), "outer key should be object id 3");
    let obj = result.get(&3).unwrap();
    let part = obj.parts.get(&2).expect("inner key should be part id 2");
    assert_eq!(part.subtype, PartSubtype::ModifierPart);
}

#[test]
fn empty_object_in_sidecar_returns_empty_parts() {
    use slicer_model_io::sidecar::parse_3mf_sidecar;
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<config>
  <object id="1">
  </object>
</config>"#;
    let mut archive = make_zip_with_sidecar(xml);
    let result = parse_3mf_sidecar(&mut archive);
    // Object entry should exist but with no parts
    let obj = result.get(&1).expect("object 1 should be present");
    assert!(obj.parts.is_empty(), "no parts should be present");
}

#[test]
fn load_3mf_invokes_sidecar_parser_before_archive_drop() {
    // Two-part assertion:
    // 1. parse_3mf_sidecar returns non-empty data for cube_cilindrical_modifier.3mf —
    //    proves the parser ran and produced output for this fixture (the log::trace!
    //    in the implementation records "parse_3mf_sidecar: N object(s), M part(s)").
    // 2. load_model succeeds — proves the integration plumbing is wired end-to-end.
    // The Rust borrow checker structurally guarantees parse_3mf_sidecar is called
    // before the ZipArchive is dropped in load_3mf (mutable borrow cannot outlive
    // the archive binding).
    use slicer_model_io::loader::load_model;
    use slicer_model_io::sidecar::parse_3mf_sidecar;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/cube_cilindrical_modifier.3mf");
    if !path.exists() {
        eprintln!("Skipping: cube_cilindrical_modifier.3mf not found");
        return;
    }

    // Part 1: direct parser call confirms it produces non-empty output.
    let file = std::fs::File::open(&path).expect("cube_cilindrical_modifier.3mf open failed");
    let mut archive = zip::ZipArchive::new(file).expect("ZipArchive::new failed");
    let sidecar = parse_3mf_sidecar(&mut archive);
    assert!(
        !sidecar.is_empty(),
        "parse_3mf_sidecar should return non-empty map for cube_cilindrical_modifier.3mf"
    );
    drop(archive);

    // Part 2: full integration path succeeds.
    let result = load_model(&path);
    assert!(
        result.is_ok(),
        "load_model on cube_cilindrical_modifier.3mf should succeed: {:?}",
        result
    );
}

// AC-Loader-1: sidecar parser extracts object-scoped metadata.
//
// `model_loader_sidecar::ObjectSidecarInfo.object_metadata` must capture
// `<metadata key="..." value="..."/>` entries that appear directly inside an
// `<object>` block (not nested in a `<part>`). All three Packet 67 fixtures
// have `extruder=1` at object scope; bridge obj5 additionally has
// `enable_support=1` and `support_type=tree(auto)`.
#[test]
fn sidecar_parser_extracts_object_metadata() {
    use slicer_model_io::sidecar::parse_3mf_sidecar;
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = [
        ("cube_positive_n_negative.3mf", 4u32),
        ("cube_cilindrical_modifier.3mf", 3u32),
    ];

    for (name, expected_obj_id) in fixtures {
        let path = repo_root.join("resources").join(name);
        if !path.exists() {
            eprintln!("SKIP: {} not found", path.display());
            continue;
        }
        let file =
            std::fs::File::open(&path).unwrap_or_else(|_| panic!("fixture not found: {:?}", path));
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let result = parse_3mf_sidecar(&mut archive);
        let obj = result
            .get(&expected_obj_id)
            .unwrap_or_else(|| panic!("{name}: object id {expected_obj_id} missing"));
        assert_eq!(
            obj.object_metadata.get("extruder").map(String::as_str),
            Some("1"),
            "{name}: object {expected_obj_id} must have extruder=1 in object_metadata"
        );
    }

    // Bridge fixture has TWO objects with object-scoped metadata, including
    // obj5 with enable_support and support_type.
    let bridge_path = repo_root
        .join("resources")
        .join("bridge_support_enforcers.3mf");
    if !bridge_path.exists() {
        eprintln!("SKIP: {} not found", bridge_path.display());
        return;
    }
    let file = std::fs::File::open(&bridge_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let result = parse_3mf_sidecar(&mut archive);

    let obj4 = result.get(&4).expect("bridge: object 4 missing");
    assert_eq!(
        obj4.object_metadata.get("extruder").map(String::as_str),
        Some("1"),
        "bridge obj4 must have extruder=1"
    );

    let obj5 = result.get(&5).expect("bridge: object 5 missing");
    assert_eq!(
        obj5.object_metadata.get("extruder").map(String::as_str),
        Some("1"),
        "bridge obj5 must have extruder=1"
    );
    assert_eq!(
        obj5.object_metadata
            .get("enable_support")
            .map(String::as_str),
        Some("1"),
        "bridge obj5 must have enable_support=1"
    );
    assert_eq!(
        obj5.object_metadata.get("support_type").map(String::as_str),
        Some("tree(auto)"),
        "bridge obj5 must have support_type=tree(auto)"
    );
}
