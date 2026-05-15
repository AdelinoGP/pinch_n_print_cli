//! Sidecar parser for OrcaSlicer/Bambu Studio `Metadata/model_settings.config`
//! inside a 3MF ZIP archive (Packet 56).
//!
//! Surfaces typed per-part metadata keyed by `<object id>` → `<part id>`.
//! No IR mutation; no downstream consumer wiring. Pure data producer.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek};

/// Part subtype as encoded in the sidecar `<part subtype="…">` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartSubtype {
    /// `normal_part` — regular geometry.
    NormalPart,
    /// `modifier_part` — mesh modifier volume.
    ModifierPart,
    /// `negative_part` — boolean subtraction volume.
    NegativePart,
    /// `support_enforcer` — support enforcer volume.
    SupportEnforcer,
    /// `support_blocker` — support blocker volume.
    SupportBlocker,
}

/// Per-part metadata from the sidecar.
pub struct PartSidecarInfo {
    /// Parsed subtype of this part.
    pub subtype: PartSubtype,
    /// Key-value pairs from `<metadata key="…" value="…"/>` inside this `<part>`.
    pub metadata: BTreeMap<String, String>,
}

/// Per-object sidecar data, containing one entry per `<part>`.
pub struct ObjectSidecarInfo {
    /// Map from part id to its sidecar metadata.
    pub parts: HashMap<u32, PartSidecarInfo>,
}

/// Parse `Metadata/model_settings.config` from a 3MF ZIP archive.
///
/// Returns a map from object id → [`ObjectSidecarInfo`].
/// - Missing sidecar file → empty map, no warning (silent default).
/// - Read error or malformed XML → empty map + `log::warn!`.
pub fn parse_3mf_sidecar<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
) -> HashMap<u32, ObjectSidecarInfo> {
    let sidecar_bytes = match zip.by_name("Metadata/model_settings.config") {
        Ok(mut file) => {
            let mut buf = Vec::new();
            match file.read_to_end(&mut buf) {
                Ok(_) => buf,
                Err(e) => {
                    log::warn!(
                        target: "slicer_host::model_loader::sidecar",
                        "3MF sidecar read error: {e}; treating all parts as normal_part"
                    );
                    return HashMap::new();
                }
            }
        }
        Err(_) => {
            // Missing sidecar is the silent default per DEV-051.
            return HashMap::new();
        }
    };

    parse_sidecar_bytes(&sidecar_bytes)
}

fn parse_sidecar_bytes(sidecar_bytes: &[u8]) -> HashMap<u32, ObjectSidecarInfo> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(sidecar_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut result: HashMap<u32, ObjectSidecarInfo> = HashMap::new();
    let mut current_object_id: Option<u32> = None;
    let mut current_part_id: Option<u32> = None;
    let mut current_subtype = PartSubtype::NormalPart;
    let mut current_metadata: BTreeMap<String, String> = BTreeMap::new();
    let mut inside_part = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = sidecar_local_name(&name_bytes);
                match local {
                    b"object" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"id" {
                                if let Ok(s) = std::str::from_utf8(&attr.value) {
                                    if let Ok(id) = s.trim().parse::<u32>() {
                                        current_object_id = Some(id);
                                        result.entry(id).or_insert_with(|| ObjectSidecarInfo {
                                            parts: HashMap::new(),
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
                            let obj = result.entry(oid).or_insert_with(|| ObjectSidecarInfo {
                                parts: HashMap::new(),
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
                        current_object_id = None;
                        inside_part = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!(
                    target: "slicer_host::model_loader::sidecar",
                    "3MF sidecar XML parse error: {e}; treating all parts as normal_part"
                );
                return HashMap::new();
            }
            _ => {}
        }
        buf.clear();
    }

    let total_parts: usize = result.values().map(|o| o.parts.len()).sum();
    log::trace!(
        target: "slicer_host::model_loader::sidecar",
        "parse_3mf_sidecar: {} object(s), {} part(s)",
        result.len(),
        total_parts
    );

    result
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
                target: "slicer_host::model_loader::sidecar",
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
