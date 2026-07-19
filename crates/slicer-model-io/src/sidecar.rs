//! Sidecar parser for OrcaSlicer/Bambu Studio `Metadata/model_settings.config`
//! inside a 3MF ZIP archive (Packet 56).
//!
//! Surfaces typed per-part metadata keyed by `<object id>` â†’ `<part id>`.
//! No IR mutation; no downstream consumer wiring. Pure data producer.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek};

/// Part subtype as encoded in the sidecar `<part subtype="â€¦">` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartSubtype {
    /// `normal_part` â€” regular geometry.
    NormalPart,
    /// `modifier_part` â€” mesh modifier volume.
    ModifierPart,
    /// `negative_part` â€” boolean subtraction volume.
    NegativePart,
    /// `support_enforcer` â€” support enforcer volume.
    SupportEnforcer,
    /// `support_blocker` â€” support blocker volume.
    SupportBlocker,
}

/// Per-part metadata from the sidecar.
pub struct PartSidecarInfo {
    /// Parsed subtype of this part.
    pub subtype: PartSubtype,
    /// Key-value pairs from `<metadata key="â€¦" value="â€¦"/>` inside this `<part>`.
    pub metadata: BTreeMap<String, String>,
}

/// Per-object sidecar data, containing one entry per `<part>`.
pub struct ObjectSidecarInfo {
    /// Map from part id to its sidecar metadata.
    pub parts: HashMap<u32, PartSidecarInfo>,
    /// Object-scoped `<metadata key="â€¦" value="â€¦"/>` entries (between
    /// `<object>` and its first `<part>`, or anywhere inside `<object>` not
    /// nested in a `<part>`).
    pub object_metadata: BTreeMap<String, String>,
}

/// Full sidecar parse result, carrying both per-object and per-plate metadata.
///
/// `plate_metadata` holds the key-value pairs from `<metadata key="â€¦" value="â€¦"/>`
/// inside the build-plate's `<plate>` element. OrcaSlicer authors build-wide
/// settings here (e.g. `filament_map_mode`, `filament_maps`, `thumbnail_file`).
/// These flow through the runtime as global config keys (no `object_config:`
/// prefix), because they apply to the whole build, not a single object.
pub struct ParsedSidecar {
    /// Per-object data, keyed by `<object id>`.
    pub objects: HashMap<u32, ObjectSidecarInfo>,
    /// Build-plate metadata (`<plate>` section in `model_settings.config`).
    pub plate_metadata: BTreeMap<String, String>,
}

/// Parse `Metadata/model_settings.config` from a 3MF ZIP archive.
///
/// Returns a [`ParsedSidecar`] with per-object and per-plate metadata.
/// - Missing sidecar file → empty `ParsedSidecar`, no warning (silent default).
/// - Read error or malformed XML → empty `ParsedSidecar` + `log::warn!`.
pub fn parse_3mf_sidecar<R: Read + Seek>(zip: &mut zip::ZipArchive<R>) -> ParsedSidecar {
    let sidecar_bytes = match zip.by_name("Metadata/model_settings.config") {
        Ok(mut file) => {
            let mut buf = Vec::new();
            match file.read_to_end(&mut buf) {
                Ok(_) => buf,
                Err(e) => {
                    log::warn!(
                        target: "slicer_model_io::sidecar",
                        "3MF sidecar read error: {e}; treating all parts as normal_part"
                    );
                    return ParsedSidecar {
                        objects: HashMap::new(),
                        plate_metadata: BTreeMap::new(),
                    };
                }
            }
        }
        Err(_) => {
            // Missing sidecar is the silent default per DEV-051.
            return ParsedSidecar {
                objects: HashMap::new(),
                plate_metadata: BTreeMap::new(),
            };
        }
    };

    parse_sidecar_bytes(&sidecar_bytes)
}

fn parse_sidecar_bytes(sidecar_bytes: &[u8]) -> ParsedSidecar {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(sidecar_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut objects: HashMap<u32, ObjectSidecarInfo> = HashMap::new();
    let mut plate_metadata: BTreeMap<String, String> = BTreeMap::new();

    let mut current_object_id: Option<u32> = None;
    let mut current_part_id: Option<u32> = None;
    let mut current_subtype = PartSubtype::NormalPart;
    let mut current_metadata: BTreeMap<String, String> = BTreeMap::new();
    let mut current_object_metadata: BTreeMap<String, String> = BTreeMap::new();
    let mut inside_part = false;
    let mut inside_plate = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = sidecar_local_name(&name_bytes);
                match local {
                    b"object" => {
                        current_object_metadata = BTreeMap::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"id" {
                                if let Ok(s) = std::str::from_utf8(&attr.value) {
                                    if let Ok(id) = s.trim().parse::<u32>() {
                                        current_object_id = Some(id);
                                        objects.entry(id).or_insert_with(|| ObjectSidecarInfo {
                                            parts: HashMap::new(),
                                            object_metadata: BTreeMap::new(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    b"part" => {
                        inside_part = true;
                        current_part_id = None;
                        current_subtype = PartSubtype::NormalPart;
                        current_metadata = BTreeMap::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"id" => {
                                    if let Ok(s) = std::str::from_utf8(&attr.value) {
                                        current_part_id = s.trim().parse().ok();
                                    }
                                }
                                b"subtype" => {
                                    current_subtype = parse_part_subtype(&attr.value);
                                }
                                _ => {}
                            }
                        }
                    }
                    b"plate" => {
                        // Begin accumulating the build-plate's metadata.
                        // The single plate in a typical 3MF is what we want; a
                        // hypothetical second plate would still flow through,
                        // the last writer wins (the sidecar is a tiny XML
                        // config, not a real list).
                        inside_plate = true;
                    }
                    b"metadata" if inside_part => {
                        let mut key = String::new();
                        let mut val = String::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"key" => {
                                    key = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                                b"value" => {
                                    val = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                                _ => {}
                            }
                        }
                        if !key.is_empty() {
                            current_metadata.insert(key, val);
                        }
                    }
                    b"metadata" if inside_plate && !inside_part => {
                        let mut key = String::new();
                        let mut val = String::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"key" => {
                                    key = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                                b"value" => {
                                    val = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                                _ => {}
                            }
                        }
                        if !key.is_empty() {
                            plate_metadata.insert(key, val);
                        }
                    }
                    b"metadata" if current_object_id.is_some() && !inside_part && !inside_plate => {
                        let mut key = String::new();
                        let mut val = String::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"key" => {
                                    key = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                                b"value" => {
                                    val = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                                _ => {}
                            }
                        }
                        if !key.is_empty() {
                            current_object_metadata.insert(key, val);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = sidecar_local_name(&name_bytes);
                match local {
                    b"part" => {
                        inside_part = false;
                        if let (Some(oid), Some(pid)) = (current_object_id, current_part_id) {
                            let obj = objects.entry(oid).or_insert_with(|| ObjectSidecarInfo {
                                parts: HashMap::new(),
                                object_metadata: BTreeMap::new(),
                            });
                            obj.parts.insert(
                                pid,
                                PartSidecarInfo {
                                    subtype: current_subtype,
                                    metadata: std::mem::take(&mut current_metadata),
                                },
                            );
                        }
                        current_part_id = None;
                    }
                    b"object" => {
                        if let Some(oid) = current_object_id {
                            let obj = objects.entry(oid).or_insert_with(|| ObjectSidecarInfo {
                                parts: HashMap::new(),
                                object_metadata: BTreeMap::new(),
                            });
                            obj.object_metadata = std::mem::take(&mut current_object_metadata);
                        }
                        current_object_id = None;
                        inside_part = false;
                    }
                    b"plate" => {
                        inside_plate = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!(
                    target: "slicer_model_io::sidecar",
                    "3MF sidecar XML parse error: {e}; treating all parts as normal_part"
                );
                return ParsedSidecar {
                    objects: HashMap::new(),
                    plate_metadata: BTreeMap::new(),
                };
            }
            _ => {}
        }
        buf.clear();
    }

    let total_parts: usize = objects.values().map(|o| o.parts.len()).sum();
    log::trace!(
        target: "slicer_model_io::sidecar",
        "parse_3mf_sidecar: {} object(s), {} part(s), {} plate metadata key(s)",
        objects.len(),
        total_parts,
        plate_metadata.len()
    );

    ParsedSidecar {
        objects,
        plate_metadata,
    }
}

fn parse_part_subtype(raw: &[u8]) -> PartSubtype {
    match raw {
        b"normal_part" => PartSubtype::NormalPart,
        b"modifier_part" => PartSubtype::ModifierPart,
        b"negative_part" => PartSubtype::NegativePart,
        b"support_enforcer" => PartSubtype::SupportEnforcer,
        b"support_blocker" => PartSubtype::SupportBlocker,
        other => {
            let s = String::from_utf8_lossy(other);
            log::warn!(
                target: "slicer_model_io::sidecar",
                "3MF sidecar unrecognized subtype '{}': downgrading to normal_part",
                s
            );
            PartSubtype::NormalPart
        }
    }
}

fn sidecar_local_name(name: &[u8]) -> &[u8] {
    name.iter()
        .rposition(|&b| b == b':')
        .map(|i| &name[i + 1..])
        .unwrap_or(name)
}
