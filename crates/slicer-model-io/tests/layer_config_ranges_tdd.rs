//! TDD suite for the optional `Metadata/layer_config_ranges.xml` member of a
//! 3MF archive: raw parse records plus one-based ordinal mapping
//! (packet `config-scope-resolution_09_layer-range-scope`, Step 2b).
//!
//! Expected values are literals pinned from the canonical XML shape; no
//! expectation is derived from production helpers.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;

use slicer_model_io::{
    map_layer_config_ranges, read_3mf_layer_config_ranges, LayerRangeParseError,
    RawLayerConfigRange,
};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/layer_range_one_range.3mf")
}

#[test]
fn parses_canonical_single_range_fixture_and_maps_one_based_ordinal() {
    let path = fixture_path();
    let raw = read_3mf_layer_config_ranges(&path).expect("canonical fixture must parse");

    assert_eq!(raw.len(), 1, "canonical fixture carries exactly one range");
    let record = &raw[0];
    assert_eq!(record.object_ordinal, 1);
    assert_eq!(record.source_index, 0);
    assert_eq!(record.min_z, 0.4);
    assert_eq!(record.max_z, 0.8);

    let mut expected_values = BTreeMap::new();
    expected_values.insert("layer_height".to_string(), "0.1".to_string());
    assert_eq!(record.values, expected_values);

    let mapped = map_layer_config_ranges(&raw, &["obj-a".to_string()])
        .expect("ordinal 1 maps onto the first loaded object");
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].object_id.as_str(), "obj-a");
    assert_eq!(mapped[0].source_index, 0);
    assert_eq!(mapped[0].min_z, 0.4);
    assert_eq!(mapped[0].max_z, 0.8);
    assert_eq!(mapped[0].values, expected_values);

    // The one-based range ordinal is NOT the 3MF model XML object id. The
    // fixture deliberately uses model id 7 so a regression back to XML-id
    // mapping is caught here.
    let file = std::fs::File::open(&path).expect("fixture opens");
    let mut archive = zip::ZipArchive::new(file).expect("fixture is a ZIP");
    let mut model = String::new();
    archive
        .by_name("3D/3dmodel.model")
        .expect("fixture carries a model part")
        .read_to_string(&mut model)
        .expect("model part reads as UTF-8");
    assert!(model.contains(r#"id="7""#), "fixture model object id is 7");
    assert!(
        !model.contains(r#"<object id="1""#),
        "fixture model must not use the range ordinal as its XML id"
    );
}

/// Temp directory holding one generated 3MF; removed on drop.
struct TempZipDir {
    dir: PathBuf,
}

impl TempZipDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let suffix = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "slicer-layer-config-ranges-tdd-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir is creatable");
        Self { dir }
    }

    fn zip_path(&self) -> PathBuf {
        self.dir.join("ranges.3mf")
    }
}

impl Drop for TempZipDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Writes a temp 3MF carrying `xml` as the range member, or no range member
/// at all when `xml` is `None`.
fn temp_zip_with_ranges_member(xml: Option<&str>) -> TempZipDir {
    let temp = TempZipDir::new();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("3D/3dmodel.model", options)
        .expect("model member starts");
    writer
        .write_all(br#"<model unit="millimeter"/>"#)
        .expect("model member writes");
    if let Some(xml) = xml {
        writer
            .start_file("Metadata/layer_config_ranges.xml", options)
            .expect("range member starts");
        writer
            .write_all(xml.as_bytes())
            .expect("range member writes");
    }
    let bytes = writer.finish().expect("zip finalizes").into_inner();
    std::fs::write(temp.zip_path(), bytes).expect("temp 3MF writes");
    temp
}

/// Writes `xml` into a temp 3MF and reads it back through the parser under
/// test.
fn parse_xml(xml: &str) -> Result<Vec<RawLayerConfigRange>, LayerRangeParseError> {
    let temp = temp_zip_with_ranges_member(Some(xml));
    read_3mf_layer_config_ranges(&temp.zip_path())
}

#[test]
fn malformed_or_invalid_ranges_fail_atomically_but_missing_part_is_empty() {
    // Malformed XML: `<object>` and `<range>` never close.
    let err = parse_xml(r#"<objects><object id="1"><range min_z="0.4" max_z="0.8">"#)
        .expect_err("malformed XML must fail");
    assert!(
        matches!(err, LayerRangeParseError::MalformedXml { .. }),
        "expected MalformedXml, got {err:?}"
    );

    // Missing min_z.
    let err = parse_xml(r#"<objects><object id="1"><range max_z="0.8"><option opt_key="layer_height">0.1</option></range></object></objects>"#)
        .expect_err("missing min_z must fail");
    assert!(
        matches!(
            err,
            LayerRangeParseError::InvalidAttribute {
                attribute: "min_z",
                ..
            }
        ),
        "expected InvalidAttribute(min_z), got {err:?}"
    );

    // Missing opt_key.
    let err = parse_xml(r#"<objects><object id="1"><range min_z="0.4" max_z="0.8"><option>0.1</option></range></object></objects>"#)
        .expect_err("missing opt_key must fail");
    assert!(
        matches!(
            err,
            LayerRangeParseError::InvalidAttribute {
                attribute: "opt_key",
                ..
            }
        ),
        "expected InvalidAttribute(opt_key), got {err:?}"
    );

    // Invalid (non-numeric) id attribute.
    let err = parse_xml(r#"<objects><object id="seven"><range min_z="0.4" max_z="0.8"><option opt_key="layer_height">0.1</option></range></object></objects>"#)
        .expect_err("non-numeric id must fail");
    assert!(
        matches!(
            err,
            LayerRangeParseError::InvalidAttribute {
                attribute: "id",
                ..
            }
        ),
        "expected InvalidAttribute(id), got {err:?}"
    );

    // Non-finite bound.
    let err = parse_xml(r#"<objects><object id="1"><range min_z="inf" max_z="0.8"><option opt_key="layer_height">0.1</option></range></object></objects>"#)
        .expect_err("non-finite min_z must fail");
    assert!(
        matches!(err, LayerRangeParseError::InvalidBounds { min_z, .. } if min_z.is_infinite()),
        "expected InvalidBounds with infinite min_z, got {err:?}"
    );

    // NaN bound.
    let err = parse_xml(r#"<objects><object id="1"><range min_z="0.4" max_z="NaN"><option opt_key="layer_height">0.1</option></range></object></objects>"#)
        .expect_err("NaN max_z must fail");
    assert!(
        matches!(err, LayerRangeParseError::InvalidBounds { max_z, .. } if max_z.is_nan()),
        "expected InvalidBounds with NaN max_z, got {err:?}"
    );

    // Negative min_z.
    let err = parse_xml(r#"<objects><object id="1"><range min_z="-0.5" max_z="0.8"><option opt_key="layer_height">0.1</option></range></object></objects>"#)
        .expect_err("negative min_z must fail");
    assert!(
        matches!(
            err,
            LayerRangeParseError::InvalidBounds { min_z, .. } if min_z < 0.0
        ),
        "expected InvalidBounds with negative min_z, got {err:?}"
    );

    // min_z >= max_z (strictly greater and equal).
    for (min_z, max_z) in [("0.8", "0.4"), ("0.4", "0.4")] {
        let xml = format!(
            r#"<objects><object id="1"><range min_z="{min_z}" max_z="{max_z}"><option opt_key="layer_height">0.1</option></range></object></objects>"#
        );
        let err = parse_xml(&xml).expect_err("non-ascending bounds must fail");
        assert!(
            matches!(err, LayerRangeParseError::InvalidBounds { .. }),
            "expected InvalidBounds for min_z={min_z} max_z={max_z}, got {err:?}"
        );
    }

    // Zero object ordinal.
    let err = parse_xml(r#"<objects><object id="0"><range min_z="0.4" max_z="0.8"><option opt_key="layer_height">0.1</option></range></object></objects>"#)
        .expect_err("object id 0 must fail");
    assert_eq!(
        err,
        LayerRangeParseError::InvalidObjectOrdinal { object_ordinal: 0 }
    );

    // Duplicate object ordinal: the first `<object>` + `<range>` is fully
    // valid; the read must still fail atomically (no partial vector).
    let err = parse_xml(r#"<objects><object id="1"><range min_z="0.4" max_z="0.8"><option opt_key="layer_height">0.1</option></range></object><object id="1"><range min_z="0.8" max_z="1.2"><option opt_key="layer_height">0.2</option></range></object></objects>"#)
        .expect_err("duplicate object ordinal must fail");
    assert_eq!(
        err,
        LayerRangeParseError::DuplicateObjectOrdinal { object_ordinal: 1 }
    );

    // Absent member is the empty successful default.
    let no_member = temp_zip_with_ranges_member(None);
    let parsed = read_3mf_layer_config_ranges(&no_member.zip_path())
        .expect("a missing layer_config_ranges.xml is an empty success");
    assert!(parsed.is_empty(), "absent member yields no records");
}

#[test]
fn unmapped_object_ordinal_is_atomic_load_error() {
    // exhaustive: wire-shape contract pins the exact raw record under test
    let raw = vec![RawLayerConfigRange {
        object_ordinal: 2,
        source_index: 0,
        min_z: 0.4,
        max_z: 0.8,
        values: BTreeMap::new(),
    }];

    let err = map_layer_config_ranges(&raw, &["obj-a".to_string()])
        .expect_err("ordinal 2 with one loaded object must fail atomically");
    assert_eq!(
        err,
        LayerRangeParseError::InvalidObjectOrdinal { object_ordinal: 2 }
    );
}
