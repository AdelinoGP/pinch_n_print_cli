//! Parser for the optional `Metadata/layer_config_ranges.xml` member of a 3MF
//! archive (packet `config-scope-resolution_09_layer-range-scope`).
//!
//! The member carries per-object world-Z intervals with raw string option
//! values, written by OrcaSlicer/Bambu Studio. This module is a pure data
//! producer: it returns raw ordinal/bound/string records and maps one-based
//! ordinals onto loaded object ids. It never types values, mutates IR, or
//! resolves configuration precedence.
//!
//! Canonical shape:
//!
//! ```xml
//! <objects>
//!  <object id="1">
//!   <range min_z="0.4" max_z="0.8">
//!    <option opt_key="layer_height">0.1</option>
//!   </range>
//!  </object>
//! </objects>
//! ```
//!
//! The `<object id>` is a **one-based ordinal** over the loaded object list,
//! not a 3MF model XML id. A missing member is an empty successful result;
//! malformed XML or invalid structure fails the whole read atomically.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Read;
use std::path::Path;

use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::Reader;
use slicer_ir::ObjectId;

/// ZIP member carrying the canonical layer-range declarations.
const RANGES_MEMBER: &str = "Metadata/layer_config_ranges.xml";

/// Failure loading `Metadata/layer_config_ranges.xml` from a 3MF archive.
///
/// Every variant aborts the whole read: no partial record vector is ever
/// exposed alongside an error.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerRangeParseError {
    /// The 3MF file or the range member could not be opened or read.
    ArchiveRead {
        /// Underlying I/O or ZIP error text.
        detail: String,
    },
    /// The range member is not well-formed XML, or its root is not `<objects>`.
    MalformedXml {
        /// Parser or shape diagnostic text.
        detail: String,
    },
    /// A required attribute (`id`, `min_z`, `max_z`, `opt_key`) is missing,
    /// empty, or cannot be decoded/parsed.
    InvalidAttribute {
        /// Name of the offending attribute.
        attribute: &'static str,
        /// Why the attribute value was rejected.
        detail: String,
    },
    /// Two `<object>` elements declare the same one-based ordinal.
    DuplicateObjectOrdinal {
        /// The repeated ordinal.
        object_ordinal: u32,
    },
    /// A range bound is non-finite, negative, or non-ascending.
    InvalidBounds {
        /// Parsed lower bound (millimetres).
        min_z: f64,
        /// Parsed upper bound (millimetres).
        max_z: f64,
    },
    /// An object ordinal is zero, or exceeds the loaded object list during
    /// mapping.
    InvalidObjectOrdinal {
        /// The offending ordinal.
        object_ordinal: u32,
    },
}

impl fmt::Display for LayerRangeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveRead { detail } => {
                write!(f, "layer range archive read failed: {detail}")
            }
            Self::MalformedXml { detail } => {
                write!(f, "layer range XML is malformed: {detail}")
            }
            Self::InvalidAttribute { attribute, detail } => {
                write!(
                    f,
                    "layer range attribute '{attribute}' is invalid: {detail}"
                )
            }
            Self::DuplicateObjectOrdinal { object_ordinal } => {
                write!(f, "duplicate layer range object ordinal {object_ordinal}")
            }
            Self::InvalidBounds { min_z, max_z } => {
                write!(
                    f,
                    "layer range bounds are invalid: min_z={min_z}, max_z={max_z}"
                )
            }
            Self::InvalidObjectOrdinal { object_ordinal } => {
                write!(
                    f,
                    "layer range object ordinal {object_ordinal} has no loaded object"
                )
            }
        }
    }
}

impl std::error::Error for LayerRangeParseError {}

/// One raw range record read from `Metadata/layer_config_ranges.xml`.
///
/// Bounds are authored millimetres; option values are the element text of
/// `<option>` children, typed only later by registry-aware ingestion.
#[derive(Debug, Clone, PartialEq)]
pub struct RawLayerConfigRange {
    /// One-based position of the owning `<object>` element among the loaded
    /// object list.
    pub object_ordinal: u32,
    /// Zero-based position of the owning `<object>` element in document order.
    /// Used only for deterministic ordering and tie-breaks.
    pub source_index: u32,
    /// Lower bound of the world-Z interval, in millimetres.
    pub min_z: f64,
    /// Upper bound of the world-Z interval, in millimetres.
    pub max_z: f64,
    /// Raw `opt_key -> option text` pairs declared by `<option>` children.
    pub values: BTreeMap<String, String>,
}

/// A [`RawLayerConfigRange`] whose ordinal has been mapped to a loaded
/// [`ObjectId`].
#[derive(Debug, Clone, PartialEq)]
pub struct MappedLayerConfigRange {
    /// The loaded object this range applies to.
    pub object_id: ObjectId,
    /// Zero-based position of the owning `<object>` element in document order.
    pub source_index: u32,
    /// Lower bound of the world-Z interval, in millimetres.
    pub min_z: f64,
    /// Upper bound of the world-Z interval, in millimetres.
    pub max_z: f64,
    /// Raw `opt_key -> option text` pairs declared by `<option>` children.
    pub values: BTreeMap<String, String>,
}

/// Read `Metadata/layer_config_ranges.xml` from a 3MF archive.
///
/// Returns the raw range records in document order. An absent member is an
/// empty successful result; malformed XML, duplicate object ordinals, zero or
/// invalid ordinals, and invalid bounds return a [`LayerRangeParseError`] with
/// no partial output.
pub fn read_3mf_layer_config_ranges(
    path: &Path,
) -> Result<Vec<RawLayerConfigRange>, LayerRangeParseError> {
    let file = std::fs::File::open(path).map_err(|err| LayerRangeParseError::ArchiveRead {
        detail: format!("cannot open {}: {err}", path.display()),
    })?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|err| {
        LayerRangeParseError::ArchiveRead {
            detail: format!("cannot read 3MF archive {}: {err}", path.display()),
        }
    })?;
    let mut member = match archive.by_name(RANGES_MEMBER) {
        Ok(member) => member,
        Err(zip::result::ZipError::FileNotFound) => return Ok(Vec::new()),
        Err(err) => {
            return Err(LayerRangeParseError::ArchiveRead {
                detail: format!("cannot read {RANGES_MEMBER}: {err}"),
            })
        }
    };
    let mut bytes = Vec::new();
    member
        .read_to_end(&mut bytes)
        .map_err(|err| LayerRangeParseError::ArchiveRead {
            detail: format!("cannot read {RANGES_MEMBER}: {err}"),
        })?;
    parse_ranges_xml(&bytes)
}

/// Map one-based raw ordinals onto the loaded object list, preserving `raw`
/// order.
///
/// A zero ordinal or an ordinal past the end of `object_ids` returns
/// [`LayerRangeParseError::InvalidObjectOrdinal`] with no partial output.
pub fn map_layer_config_ranges(
    raw: &[RawLayerConfigRange],
    object_ids: &[ObjectId],
) -> Result<Vec<MappedLayerConfigRange>, LayerRangeParseError> {
    let mut mapped = Vec::with_capacity(raw.len());
    for record in raw {
        let ordinal = record.object_ordinal;
        if ordinal == 0 || ordinal as usize > object_ids.len() {
            return Err(LayerRangeParseError::InvalidObjectOrdinal {
                object_ordinal: ordinal,
            });
        }
        mapped.push(MappedLayerConfigRange {
            object_id: object_ids[ordinal as usize - 1].clone(),
            source_index: record.source_index,
            min_z: record.min_z,
            max_z: record.max_z,
            values: record.values.clone(),
        });
    }
    Ok(mapped)
}

/// Structural frame tracked while streaming the range document.
enum Frame {
    /// The `<objects>` root element.
    Objects,
    /// An `<object>` element directly below the root.
    Object { ordinal: u32, source_index: u32 },
    /// A `<range>` element directly below an `<object>`.
    Range {
        object_ordinal: u32,
        source_index: u32,
        min_z: f64,
        max_z: f64,
        values: BTreeMap<String, String>,
    },
    /// An `<option>` element directly below a `<range>`.
    Option { opt_key: String, text: String },
    /// Any element in an unrecognized position; its subtree is skipped.
    Unknown,
}

fn parse_ranges_xml(bytes: &[u8]) -> Result<Vec<RawLayerConfigRange>, LayerRangeParseError> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(false);

    let mut stack: Vec<Frame> = Vec::new();
    let mut records: Vec<RawLayerConfigRange> = Vec::new();
    let mut seen_ordinals: BTreeSet<u32> = BTreeSet::new();
    let mut next_source_index: u32 = 0;
    let mut root_seen = false;

    let mut buf = Vec::new();
    loop {
        let event =
            reader
                .read_event_into(&mut buf)
                .map_err(|err| LayerRangeParseError::MalformedXml {
                    detail: err.to_string(),
                })?;
        match event {
            Event::Start(ref element) => {
                open_element(
                    &mut stack,
                    &mut seen_ordinals,
                    &mut next_source_index,
                    &mut root_seen,
                    element,
                )?;
            }
            Event::Empty(ref element) => {
                open_element(
                    &mut stack,
                    &mut seen_ordinals,
                    &mut next_source_index,
                    &mut root_seen,
                    element,
                )?;
                close_top(&mut stack, &mut records);
            }
            Event::End(_) => close_top(&mut stack, &mut records),
            Event::Text(ref text) => {
                if let Ok(decoded) = text.decode() {
                    append_option_text(&mut stack, &decoded);
                }
            }
            Event::CData(ref text) => {
                if let Ok(decoded) = text.decode() {
                    append_option_text(&mut stack, &decoded);
                }
            }
            Event::GeneralRef(ref reference) => {
                if matches!(stack.last(), Some(Frame::Option { .. })) {
                    let resolved = resolve_reference(reference)?;
                    append_option_text(&mut stack, &resolved);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if !root_seen {
        return Err(LayerRangeParseError::MalformedXml {
            detail: "missing <objects> root element".to_string(),
        });
    }
    if !stack.is_empty() {
        return Err(LayerRangeParseError::MalformedXml {
            detail: format!(
                "{} element(s) left unclosed at end of document",
                stack.len()
            ),
        });
    }
    Ok(records)
}

/// Open an element, pushing the structural frame it enters. Errors immediately
/// on invalid attributes or shape violations.
fn open_element(
    stack: &mut Vec<Frame>,
    seen_ordinals: &mut BTreeSet<u32>,
    next_source_index: &mut u32,
    root_seen: &mut bool,
    element: &BytesStart<'_>,
) -> Result<(), LayerRangeParseError> {
    let name = element.name();
    let local = local_name(name.as_ref());
    let frame = match stack.last() {
        None => {
            if *root_seen {
                return Err(LayerRangeParseError::MalformedXml {
                    detail: "multiple root elements".to_string(),
                });
            }
            if local != b"objects" {
                return Err(LayerRangeParseError::MalformedXml {
                    detail: format!(
                        "root element is <{}>, expected <objects>",
                        String::from_utf8_lossy(local)
                    ),
                });
            }
            *root_seen = true;
            Frame::Objects
        }
        Some(Frame::Objects) => {
            if local != b"object" {
                Frame::Unknown
            } else {
                let id_text = require_attribute(element, "id")?;
                let ordinal = id_text.trim().parse::<u32>().map_err(|err| {
                    LayerRangeParseError::InvalidAttribute {
                        attribute: "id",
                        detail: format!("'{id_text}' is not a positive integer: {err}"),
                    }
                })?;
                if ordinal == 0 {
                    return Err(LayerRangeParseError::InvalidObjectOrdinal { object_ordinal: 0 });
                }
                if !seen_ordinals.insert(ordinal) {
                    return Err(LayerRangeParseError::DuplicateObjectOrdinal {
                        object_ordinal: ordinal,
                    });
                }
                let source_index = *next_source_index;
                *next_source_index += 1;
                Frame::Object {
                    ordinal,
                    source_index,
                }
            }
        }
        Some(Frame::Object {
            ordinal,
            source_index,
        }) => {
            if local != b"range" {
                Frame::Unknown
            } else {
                let min_z_text = require_attribute(element, "min_z")?;
                let max_z_text = require_attribute(element, "max_z")?;
                let min_z = parse_bound(&min_z_text, "min_z")?;
                let max_z = parse_bound(&max_z_text, "max_z")?;
                if !min_z.is_finite() || !max_z.is_finite() || min_z < 0.0 || min_z >= max_z {
                    return Err(LayerRangeParseError::InvalidBounds { min_z, max_z });
                }
                Frame::Range {
                    object_ordinal: *ordinal,
                    source_index: *source_index,
                    min_z,
                    max_z,
                    values: BTreeMap::new(),
                }
            }
        }
        Some(Frame::Range { .. }) => {
            if local != b"option" {
                Frame::Unknown
            } else {
                let opt_key = require_attribute(element, "opt_key")?;
                let opt_key = opt_key.trim().to_string();
                if opt_key.is_empty() {
                    return Err(LayerRangeParseError::InvalidAttribute {
                        attribute: "opt_key",
                        detail: "must not be empty".to_string(),
                    });
                }
                Frame::Option {
                    opt_key,
                    text: String::new(),
                }
            }
        }
        Some(Frame::Option { .. }) | Some(Frame::Unknown) => Frame::Unknown,
    };
    stack.push(frame);
    Ok(())
}

/// Pop the top frame and apply its effects: a closed `<range>` emits a record,
/// a closed `<option>` commits its text into the enclosing range.
fn close_top(stack: &mut Vec<Frame>, records: &mut Vec<RawLayerConfigRange>) {
    match stack.pop() {
        Some(Frame::Range {
            object_ordinal,
            source_index,
            min_z,
            max_z,
            values,
        }) => records.push(RawLayerConfigRange {
            object_ordinal,
            source_index,
            min_z,
            max_z,
            values,
        }),
        Some(Frame::Option { opt_key, text }) => {
            if let Some(Frame::Range { values, .. }) = stack.last_mut() {
                values.insert(opt_key, text.trim().to_string());
            }
        }
        _ => {}
    }
}

/// Append decoded character data to the option currently being read, if any.
fn append_option_text(stack: &mut [Frame], text: &str) {
    if let Some(Frame::Option { text: value, .. }) = stack.last_mut() {
        value.push_str(text);
    }
}

/// Resolve a `&entity;` or `&#nn;` reference to its replacement text.
fn resolve_reference(reference: &BytesRef<'_>) -> Result<String, LayerRangeParseError> {
    if let Some(ch) =
        reference
            .resolve_char_ref()
            .map_err(|err| LayerRangeParseError::MalformedXml {
                detail: err.to_string(),
            })?
    {
        return Ok(ch.to_string());
    }
    let name = reference
        .decode()
        .map_err(|err| LayerRangeParseError::MalformedXml {
            detail: err.to_string(),
        })?;
    match quick_xml::escape::resolve_predefined_entity(&name) {
        Some(replacement) => Ok(replacement.to_string()),
        None => Err(LayerRangeParseError::MalformedXml {
            detail: format!("unknown entity reference &{name};"),
        }),
    }
}

/// Fetch and XML-unescape a required attribute by local name.
fn require_attribute(
    element: &BytesStart<'_>,
    name: &'static str,
) -> Result<String, LayerRangeParseError> {
    let mut found: Option<String> = None;
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|err| LayerRangeParseError::InvalidAttribute {
            attribute: name,
            detail: format!("malformed attribute list: {err}"),
        })?;
        if local_name(attribute.key.as_ref()) == name.as_bytes() {
            let value = attribute.unescape_value().map_err(|err| {
                LayerRangeParseError::InvalidAttribute {
                    attribute: name,
                    detail: format!("value cannot be decoded: {err}"),
                }
            })?;
            found = Some(value.into_owned());
        }
    }
    found.ok_or_else(|| LayerRangeParseError::InvalidAttribute {
        attribute: name,
        detail: "missing required attribute".to_string(),
    })
}

/// Parse one bound attribute; finiteness and ordering are validated by the
/// caller so the error can name both endpoints.
fn parse_bound(value: &str, attribute: &'static str) -> Result<f64, LayerRangeParseError> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|err| LayerRangeParseError::InvalidAttribute {
            attribute,
            detail: format!("'{value}' is not a number: {err}"),
        })
}

/// Strip any namespace prefix from an element or attribute name.
fn local_name(name: &[u8]) -> &[u8] {
    name.iter()
        .rposition(|&b| b == b':')
        .map(|i| &name[i + 1..])
        .unwrap_or(name)
}
