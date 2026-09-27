//! Registry-typed ingestion of flat and explicitly scoped configuration.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use slicer_ir::{ConfigKey, ConfigValue, ModifierId, ObjectId};

use crate::{ConfigSchemaRegistry, RegistryEntry, ResolutionError};

/// The configuration scope receiving an authored value.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum ConfigScope {
    /// The print-wide configuration scope.
    Global,
    /// Configuration for one object.
    Object(ObjectId),
    /// Configuration for one world-Z interval of one object.
    ///
    /// `range_index` is the deterministic zero-based index assigned after
    /// sorting the object's ranges by `(min_z, max_z, source_index)`.
    LayerRange {
        /// The containing object identifier.
        object_id: ObjectId,
        /// Zero-based index of the range within its object.
        range_index: u32,
    },
    /// Configuration for one modifier belonging to one object.
    Modifier {
        /// The containing object identifier.
        object_id: ObjectId,
        /// The modifier identifier.
        modifier_id: ModifierId,
    },
    /// Configuration associated with one paint semantic.
    PaintSemantic(String),
    /// Configuration associated with one tool index.
    Tool(u32),
}

/// Authored configuration entries for one scope.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScopeDelta {
    /// Values explicitly supplied by the author.
    pub values: BTreeMap<ConfigKey, ConfigValue>,
}

impl ScopeDelta {
    /// Iterate over authored values in canonical key order.
    pub fn iter(&self) -> impl Iterator<Item = (&ConfigKey, &ConfigValue)> {
        self.values.iter()
    }
}

/// All authored scope deltas in deterministic scope order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScopedConfig {
    /// Authored deltas keyed by their typed scope.
    pub deltas: BTreeMap<ConfigScope, ScopeDelta>,
    /// Authored world-Z ranges per object, normalized and indexed.
    pub layer_ranges: BTreeMap<ObjectId, Vec<LayerConfigRange>>,
}

impl ScopedConfig {
    /// Return the authored delta for `scope`, if one was supplied.
    #[must_use]
    pub fn delta(&self, scope: &ConfigScope) -> Option<&ScopeDelta> {
        self.deltas.get(scope)
    }

    /// Return the authored global delta, if one was supplied.
    #[must_use]
    pub fn global(&self) -> Option<&ScopeDelta> {
        self.delta(&ConfigScope::Global)
    }

    /// Iterate over scope deltas in the deterministic [`ConfigScope`] order.
    pub fn iter(&self) -> impl Iterator<Item = (&ConfigScope, &ScopeDelta)> {
        self.deltas.iter()
    }

    /// Iterate over scopes in deterministic order.
    pub fn scopes(&self) -> impl Iterator<Item = &ConfigScope> {
        self.deltas.keys()
    }

    /// Return the normalized layer ranges authored for `object_id`.
    #[must_use]
    pub fn ranges_for(&self, object_id: &ObjectId) -> &[LayerConfigRange] {
        self.layer_ranges.get(object_id).map_or(&[], Vec::as_slice)
    }

    /// Return whether any object carries authored layer ranges.
    #[must_use]
    pub fn has_layer_ranges(&self) -> bool {
        !self.layer_ranges.is_empty()
    }
}

/// One raw world-Z range as it reaches the typed ingestor.
///
/// The cross-crate transport from the model loader: bounds are world-Z
/// millimetres and `values` are raw authored strings. Registry typing, scope
/// admission, and interval validation all happen inside
/// [`ConfigIngestor::ingest_layer_ranges`].
#[derive(Clone, Debug, PartialEq)]
pub struct LayerRangeInput {
    /// Stable object identifier.
    pub object_id: ObjectId,
    /// Source order within the parsed part, used as the final tie-break.
    pub source_index: u32,
    /// Inclusive world-Z lower bound in millimetres.
    pub min_z: f64,
    /// Exclusive world-Z upper bound in millimetres.
    pub max_z: f64,
    /// Raw authored values keyed by canonical (snake_case) key.
    pub values: BTreeMap<String, String>,
}

/// One validated, registry-typed world-Z range committed to a [`ScopedConfig`].
#[derive(Clone, Debug, PartialEq)]
pub struct LayerConfigRange {
    /// Typed scope identifying the containing object and range index.
    pub scope: ConfigScope,
    /// Inclusive world-Z lower bound in millimetres.
    pub min_z: f64,
    /// Exclusive world-Z upper bound in millimetres.
    pub max_z: f64,
    /// Registry-typed values stated by the range.
    pub delta: ScopeDelta,
}

impl LayerConfigRange {
    /// Whether this range covers a layer whose top print Z is `layer_top_z`.
    ///
    /// Membership is half-open `[min_z, max_z)` evaluated against the layer
    /// **top** Z, and this method is the single authority for it — the resolver
    /// and the runtime kernel both call it, so the two can never disagree.
    ///
    /// Each bound is compared as the smaller of its authored `f64` millimetre
    /// value and its `f32` image. Both representations reach this predicate: a
    /// layer top travels as an `f32` (`GlobalLayer.z`, widened back to `f64`),
    /// while callers may also state an authored bound directly. Narrowing is
    /// monotonic, so the adjusted `max_z` is never greater than its authored
    /// value and no top ordered at or above the authored bound is admitted;
    /// taking the minimum also excludes the transported image of a top placed
    /// exactly on it, which is what keeps the authored rule half-open.
    /// `f32(0.7)` is `0.69999998807…`, below its own authored bound, and
    /// `f32(0.8)` is `0.80000001192…`, above its own; the minimum handles both
    /// without a fixed epsilon, so a power-of-two bound (where the two
    /// neighbouring `f32` gaps differ by a factor of two) cannot swallow a
    /// genuinely interior layer, and subnormal or very large bounds need no
    /// special case. The lower bound works symmetrically.
    ///
    /// A bound that lies *between* two `f32` values cannot be honoured exactly:
    /// narrowing moves it by up to half a local `f32` ULP, so a top within that
    /// distance of the bound — in either direction — resolves to the bound
    /// rather than to its authored side. Where the authored bound is itself
    /// representable (every ordinary print height) the rule is exact in both
    /// representations; the band only matters for a bound that is not.
    #[must_use]
    pub fn covers(&self, layer_top_z: f64) -> bool {
        // `as f32` saturates to an infinity only at magnitudes no print can
        // reach; fall back to the authored bound there so the comparison stays
        // finite instead of becoming vacuously open or closed.
        let narrower = |bound: f64| match bound as f32 {
            narrowed if narrowed.is_finite() => f64::from(narrowed),
            _ => bound,
        };
        let lower = self.min_z.min(narrower(self.min_z));
        let upper = self.max_z.min(narrower(self.max_z));
        lower <= layer_top_z && layer_top_z < upper
    }

    /// Validate one range's interval and scope variant before committing it.
    ///
    /// # Errors
    ///
    /// Returns [`LayerRangeLoadError::InvalidInterval`] unless every bound is
    /// finite, `min_z >= 0.0`, and `min_z < max_z`.
    pub fn new(
        object_id: ObjectId,
        range_index: u32,
        min_z: f64,
        max_z: f64,
        delta: ScopeDelta,
    ) -> Result<Self, LayerRangeLoadError> {
        let valid = min_z.is_finite() && max_z.is_finite() && min_z >= 0.0 && min_z < max_z;
        if !valid {
            return Err(LayerRangeLoadError::InvalidInterval {
                object_id,
                range_index,
                min_z,
                max_z,
            });
        }
        Ok(Self {
            scope: ConfigScope::LayerRange {
                object_id,
                range_index,
            },
            min_z,
            max_z,
            delta,
        })
    }
}

/// Fatal failure while loading registry-typed world-Z ranges.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerRangeLoadError {
    /// A stated key is not admitted at its range's scope.
    Denied(ResolutionError),
    /// Two ranges on one object state unequal values over a non-empty overlap.
    ConflictingOverlap {
        /// Object whose ranges conflict.
        object_id: ObjectId,
        /// Configuration key with divergent values.
        key: ConfigKey,
        /// Earlier range index of the overlapping pair.
        first_range: u32,
        /// Later range index of the overlapping pair.
        second_range: u32,
    },
    /// A range's bounds are non-finite, negative, or not ascending.
    InvalidInterval {
        /// Object carrying the invalid interval.
        object_id: ObjectId,
        /// Zero-based index of the invalid range.
        range_index: u32,
        /// Authored inclusive lower bound.
        min_z: f64,
        /// Authored exclusive upper bound.
        max_z: f64,
    },
    /// A range names an object with no declared ranges.
    UnknownObject {
        /// Object identifier with no ranges.
        object_id: ObjectId,
    },
    /// A range states a `layer_height` that cannot compose a physical profile.
    ///
    /// Raised for a finite, non-positive height — `0.0` or a negative value.
    /// Non-finite text never reaches this variant: `coerce_value` rejects it as
    /// `ConfigIngestionError::TypeMismatch` while typing the authored string.
    /// `layer_height` is the one key the profile consumes directly, so a
    /// non-positive value is a load error rather than a value the profile can
    /// silently skip: dropping it would silently ignore an authored range.
    InvalidLayerHeight {
        /// Object whose range states the invalid height.
        object_id: ObjectId,
        /// Zero-based index of the offending range.
        range_index: u32,
        /// The authored height in millimetres.
        layer_height: f64,
    },
    /// A raw value could not be typed by the registry declaration.
    Ingestion(ConfigIngestionError),
}

impl fmt::Display for LayerRangeLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied(error) => error.fmt(formatter),
            Self::ConflictingOverlap {
                object_id,
                key,
                first_range,
                second_range,
            } => write!(
                formatter,
                "layer ranges {first_range} and {second_range} of object {object_id:?} state conflicting values for {key:?}"
            ),
            Self::InvalidInterval {
                object_id,
                range_index,
                min_z,
                max_z,
            } => write!(
                formatter,
                "layer range {range_index} of object {object_id:?} has invalid bounds [{min_z}, {max_z})"
            ),
            Self::UnknownObject { object_id } => {
                write!(formatter, "layer range names unknown object {object_id:?}")
            }
            Self::InvalidLayerHeight {
                object_id,
                range_index,
                layer_height,
            } => write!(
                formatter,
                "layer range {range_index} of object {object_id:?} states an unusable layer_height {layer_height}; \
                 it must be finite and positive"
            ),
            Self::Ingestion(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LayerRangeLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Denied(error) => Some(error),
            Self::Ingestion(error) => Some(error),
            Self::ConflictingOverlap { .. }
            | Self::InvalidInterval { .. }
            | Self::UnknownObject { .. }
            | Self::InvalidLayerHeight { .. } => None,
        }
    }
}

impl From<ConfigIngestionError> for LayerRangeLoadError {
    fn from(error: ConfigIngestionError) -> Self {
        Self::Ingestion(error)
    }
}

impl From<ResolutionError> for LayerRangeLoadError {
    fn from(error: ResolutionError) -> Self {
        Self::Denied(error)
    }
}

/// A non-fatal issue encountered while retaining an authored value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IngestionWarning {
    /// The key was not declared by the registry and was dropped.
    UnrecognizedKey {
        /// The original flat wire key.
        wire_key: String,
        /// The canonical key decoded from the wire key.
        key: String,
        /// The nearest canonical registry key, when sufficiently close.
        suggestion: Option<String>,
    },
    /// A tolerant-mode diagnostic: the value was retained unchanged because the declaration cannot represent the authored shape.
    UntypedValue {
        /// The canonical key receiving the value.
        key: String,
        /// The registry field type that could not represent the value.
        declared_type: String,
        /// The authored scalar spelling or debug representation.
        authored: String,
    },
}

/// A fatal error while decoding or typing an authored value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigIngestionError {
    /// A recognized scope prefix did not have the required shape.
    MalformedScopeKey {
        /// The original flat wire key.
        wire_key: String,
        /// The required wire-key shape.
        expected: String,
    },
    /// An authored value could not be represented by the declared field type.
    TypeMismatch {
        /// The canonical key receiving the value.
        key: String,
        /// The registry field type required by the declaration.
        expected: String,
        /// The authored scalar spelling or debug representation.
        authored: String,
    },
}

impl fmt::Display for ConfigIngestionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedScopeKey { wire_key, expected } => write!(
                formatter,
                "malformed scope key {wire_key:?}; expected {expected}"
            ),
            Self::TypeMismatch {
                key,
                expected,
                authored,
            } => write!(
                formatter,
                "config key {key:?}: expected {expected}, authored {authored:?}"
            ),
        }
    }
}

impl std::error::Error for ConfigIngestionError {}

/// The completed result of typed configuration ingestion.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IngestionOutcome {
    /// Authored values grouped by typed scope.
    pub scoped: ScopedConfig,
    /// Authored values for registry-declared selectors in the global scope.
    pub selector_values: BTreeMap<ConfigKey, ConfigValue>,
    /// Non-fatal warnings raised while ingesting authored values.
    pub warnings: Vec<IngestionWarning>,
}

/// Transactional ingestor for registry-typed configuration values.
pub struct ConfigIngestor<'registry> {
    registry: &'registry ConfigSchemaRegistry,
    deltas: BTreeMap<ConfigScope, ScopeDelta>,
    layer_ranges: BTreeMap<ObjectId, Vec<LayerConfigRange>>,
    warnings: Vec<IngestionWarning>,
    untyped_global_keys: BTreeSet<ConfigKey>,
    tolerant: bool,
}

impl<'registry> ConfigIngestor<'registry> {
    /// Start an ingestor backed by a reconciled schema registry.
    #[must_use]
    pub fn new(registry: &'registry ConfigSchemaRegistry) -> Self {
        Self::with_mode(registry, false)
    }

    /// Start an ingestor that retains declared values the schema cannot type.
    #[must_use]
    pub fn tolerant(registry: &'registry ConfigSchemaRegistry) -> Self {
        Self::with_mode(registry, true)
    }

    fn with_mode(registry: &'registry ConfigSchemaRegistry, tolerant: bool) -> Self {
        Self {
            registry,
            deltas: BTreeMap::new(),
            layer_ranges: BTreeMap::new(),
            warnings: Vec::new(),
            untyped_global_keys: BTreeSet::new(),
            tolerant,
        }
    }

    /// Decode flat wire keys, type authored values, and retain them by scope.
    pub fn ingest_flat(
        &mut self,
        values: &HashMap<ConfigKey, ConfigValue>,
    ) -> Result<(), ConfigIngestionError> {
        let mut ordered = values.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(key, _)| *key);

        let mut pending = Vec::with_capacity(ordered.len());
        for (wire_key, authored) in ordered {
            let (scope, key) = decode_wire_key(wire_key)?;
            pending.push(self.prepare_entry(scope, key, wire_key, authored)?);
        }
        self.commit(pending);
        Ok(())
    }

    /// Ingest values already associated with an explicit typed scope.
    pub fn ingest_delta(
        &mut self,
        scope: ConfigScope,
        values: &HashMap<ConfigKey, ConfigValue>,
    ) -> Result<(), ConfigIngestionError> {
        let mut ordered = values.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(key, _)| *key);

        let mut pending = Vec::with_capacity(ordered.len());
        for (key, authored) in ordered {
            pending.push(self.prepare_entry(scope.clone(), key.clone(), key, authored)?);
        }
        self.commit(pending);
        Ok(())
    }

    /// Ingest raw world-Z ranges, typing every value through the registry.
    ///
    /// Ranges are grouped by object and normalized by `(min_z, max_z,
    /// source_index)`, then receive their zero-based `range_index` in that
    /// order. Every value is typed exactly as [`Self::ingest_delta`] types an
    /// explicit delta; an undeclared key warns and is dropped, a mistyped value
    /// is a fatal [`LayerRangeLoadError::Ingestion`].
    ///
    /// The whole input is validated before any state changes:
    ///
    /// - every typed key must be admitted by
    ///   [`ConfigSchemaRegistry::admission_set`](crate::ConfigSchemaRegistry::admission_set)
    ///   at its range's scope, else [`LayerRangeLoadError::Denied`];
    /// - two ranges of one object that overlap with non-empty intersection
    ///   may not state unequal typed values for any key other than
    ///   `layer_height`, else [`LayerRangeLoadError::ConflictingOverlap`].
    ///
    /// A successful call replaces the ingested ranges of every named object;
    /// a failed call changes nothing.
    ///
    /// # Errors
    ///
    /// Returns [`LayerRangeLoadError`] as described above.
    pub fn ingest_layer_ranges(
        &mut self,
        inputs: &[LayerRangeInput],
    ) -> Result<(), LayerRangeLoadError> {
        let mut grouped: BTreeMap<ObjectId, Vec<&LayerRangeInput>> = BTreeMap::new();
        for input in inputs {
            grouped
                .entry(input.object_id.clone())
                .or_default()
                .push(input);
        }

        let mut warnings = Vec::new();
        let mut pending: BTreeMap<ObjectId, Vec<PendingLayerRange>> = BTreeMap::new();

        for (object_id, mut authored_ranges) in grouped {
            authored_ranges.sort_by(|first, second| {
                first
                    .min_z
                    .total_cmp(&second.min_z)
                    .then_with(|| first.max_z.total_cmp(&second.max_z))
                    .then_with(|| first.source_index.cmp(&second.source_index))
            });

            let mut ranges = Vec::with_capacity(authored_ranges.len());
            for (index, input) in authored_ranges.into_iter().enumerate() {
                // The count is bounded by the already-materialized input slice,
                // so it is far below `u32::MAX` on any real host.
                let range_index = index as u32;
                let scope = ConfigScope::LayerRange {
                    object_id: object_id.clone(),
                    range_index,
                };
                if !valid_layer_range_interval(input.min_z, input.max_z) {
                    return Err(LayerRangeLoadError::InvalidInterval {
                        object_id: object_id.clone(),
                        range_index,
                        min_z: input.min_z,
                        max_z: input.max_z,
                    });
                }

                let mut delta = ScopeDelta::default();
                for (key, authored) in &input.values {
                    let authored = ConfigValue::String(authored.clone());
                    let entry = self.prepare_entry(scope.clone(), key.clone(), key, &authored)?;
                    if let Some(value) = entry.value {
                        delta.values.insert(entry.key, value);
                    }
                    if let Some(warning) = entry.warning {
                        warnings.push(warning);
                    }
                }

                // `layer_height` is the one key the profile consumes directly.
                // Reject an unusable value here rather than letting the profile
                // skip it: a silently skipped range would apply every other
                // value it states while quietly ignoring the authored height.
                if let Some(value) = delta.values.get("layer_height") {
                    match value {
                        ConfigValue::Float(height) if height.is_finite() && *height > 0.0 => {}
                        other => {
                            let layer_height = match other {
                                ConfigValue::Float(height) => *height,
                                ConfigValue::Int(height) => *height as f64,
                                _ => f64::NAN,
                            };
                            return Err(LayerRangeLoadError::InvalidLayerHeight {
                                object_id: object_id.clone(),
                                range_index,
                                layer_height,
                            });
                        }
                    }
                }

                let admission = self.registry.admission_set(&scope);
                for key in delta.values.keys() {
                    if !admission.contains(key) {
                        return Err(LayerRangeLoadError::Denied(ResolutionError::ScopeDenied {
                            key: key.clone(),
                            scope: scope.clone(),
                        }));
                    }
                }

                ranges.push(PendingLayerRange {
                    range_index,
                    min_z: input.min_z,
                    max_z: input.max_z,
                    delta,
                });
            }
            pending.insert(object_id, ranges);
        }

        validate_layer_range_overlaps(&pending)?;

        // Commit only after the entire input validated. The new ranges are
        // built first so a construction failure cannot leave a partial object.
        let mut committed = Vec::with_capacity(pending.len());
        for (object_id, ranges) in pending {
            let mut object_ranges = Vec::with_capacity(ranges.len());
            for range in ranges {
                object_ranges.push(LayerConfigRange::new(
                    object_id.clone(),
                    range.range_index,
                    range.min_z,
                    range.max_z,
                    range.delta,
                )?);
            }
            committed.push((object_id, object_ranges));
        }
        for (object_id, ranges) in committed {
            self.layer_ranges.insert(object_id, ranges);
        }
        self.warnings.extend(warnings);
        self.warnings.sort_by(warning_order);
        Ok(())
    }

    /// Finish ingestion and return the accepted authored values and derived selectors.
    ///
    /// An undeclared key is absent: it warns and is dropped at preparation,
    /// never reaching a delta.
    #[must_use]
    pub fn finish(self) -> IngestionOutcome {
        let selector_values = self
            .deltas
            .get(&ConfigScope::Global)
            .map(|delta| {
                delta
                    .values
                    .iter()
                    .filter_map(|(key, value)| {
                        self.registry_entry(key)
                            .filter(|entry| {
                                entry.selector && !self.untyped_global_keys.contains(key)
                            })
                            .map(|_| (key.clone(), value.clone()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();

        IngestionOutcome {
            scoped: ScopedConfig {
                deltas: self.deltas,
                layer_ranges: self.layer_ranges,
            },
            selector_values,
            warnings: self.warnings,
        }
    }

    fn prepare_entry<'value>(
        &self,
        scope: ConfigScope,
        key: ConfigKey,
        wire_key: &str,
        authored: &'value ConfigValue,
    ) -> Result<PendingEntry, ConfigIngestionError> {
        // A legacy alias of a registry-declared key is not "undeclared": resolve
        // the authored spelling through the alias table before the
        // declared-ness check, so it survives ingestion to scheduler-side
        // canonicalization and both-spellings conflict detection. The
        // authored spelling is retained in the delta unchanged — rewriting it
        // here would hide one of the two spellings from
        // `reject_alias_conflicts`. A truly unknown key still falls through
        // to the warn-then-drop path below.
        let canonical_key = canonical_config_key(&key);
        let entry = self.registry_entry(&key).or_else(|| {
            (canonical_key != key)
                .then(|| self.registry_entry(canonical_key))
                .flatten()
        });
        let Some(entry) = entry else {
            // Warn-then-drop: an undeclared key never reaches a delta, the
            // resolved config, a bound view, or `CONFIG_BLOCK`.
            return Ok(PendingEntry {
                scope,
                key: key.clone(),
                value: None,
                warning: Some(IngestionWarning::UnrecognizedKey {
                    wire_key: wire_key.to_owned(),
                    key: key.clone(),
                    suggestion: self.suggestion(&key),
                }),
            });
        };

        let (value, warning) = match coerce_value(&key, authored, &entry.field_type) {
            Ok(value) => (value, None),
            Err(error) if self.tolerant => {
                if is_non_finite_type_mismatch(authored, &entry.field_type) {
                    return Err(error);
                }

                let value = match coerce_scalar_list(&key, authored, &entry.field_type) {
                    Ok(Some(value)) => value,
                    Ok(None) => authored.clone(),
                    Err(error) => return Err(error),
                };
                let warning = IngestionWarning::UntypedValue {
                    key: key.clone(),
                    declared_type: entry.field_type.clone(),
                    authored: authored_text(authored),
                };
                (value, Some(warning))
            }
            Err(error) => return Err(error),
        };
        Ok(PendingEntry {
            scope,
            key,
            value: Some(value),
            warning,
        })
    }

    fn commit(&mut self, pending: Vec<PendingEntry>) {
        for entry in pending {
            let untyped_global = matches!(
                (&entry.scope, &entry.warning),
                (
                    ConfigScope::Global,
                    Some(IngestionWarning::UntypedValue { .. })
                )
            );
            if untyped_global {
                self.untyped_global_keys.insert(entry.key.clone());
            }
            // An undeclared key carries no value and is dropped here; only its
            // warning survives.
            if let Some(value) = entry.value {
                self.deltas
                    .entry(entry.scope)
                    .or_default()
                    .values
                    .insert(entry.key, value);
            }
            if let Some(warning) = entry.warning {
                self.warnings.push(warning);
            }
        }
        self.warnings.sort_by(warning_order);
    }

    fn registry_entry(&self, key: &str) -> Option<&RegistryEntry> {
        self.registry.entry(key).or_else(|| {
            let (base, _) = key.rsplit_once(':')?;
            let wildcard = format!("{base}:*");
            self.registry.entry(&wildcard)
        })
    }

    fn suggestion(&self, key: &str) -> Option<String> {
        let mut candidates = self.registry.keys().collect::<Vec<_>>();
        candidates.sort_unstable();

        candidates
            .into_iter()
            .filter_map(|candidate| {
                let distance = levenshtein(key, candidate);
                (distance <= 2).then_some((distance, candidate))
            })
            .min_by(
                |(first_distance, first_key), (second_distance, second_key)| {
                    first_distance
                        .cmp(second_distance)
                        .then_with(|| first_key.cmp(second_key))
                },
            )
            .map(|(_, candidate)| candidate.to_owned())
    }
}

struct PendingEntry {
    scope: ConfigScope,
    key: ConfigKey,
    /// `None` for an undeclared key: warned once, never retained.
    value: Option<ConfigValue>,
    warning: Option<IngestionWarning>,
}

/// One validated layer range awaiting atomic commit.
struct PendingLayerRange {
    range_index: u32,
    min_z: f64,
    max_z: f64,
    delta: ScopeDelta,
}

/// Validate one world-Z interval for a layer range.
fn valid_layer_range_interval(min_z: f64, max_z: f64) -> bool {
    min_z.is_finite() && max_z.is_finite() && min_z >= 0.0 && min_z < max_z
}

/// Reject unequal non-`layer_height` values over non-empty range overlaps.
///
/// Grouped per object; `layer_height` is governed by profile trimming instead
/// (canonical `layer_height_profile_from_ranges`), so it is never a conflict.
fn validate_layer_range_overlaps(
    pending: &BTreeMap<ObjectId, Vec<PendingLayerRange>>,
) -> Result<(), LayerRangeLoadError> {
    for (object_id, ranges) in pending {
        for (first_index, first) in ranges.iter().enumerate() {
            for second in ranges.iter().skip(first_index + 1) {
                // Non-empty intersection: `[min_z, max_z)` half-open overlap.
                if !(first.min_z < second.max_z && second.min_z < first.max_z) {
                    continue;
                }
                for (key, first_value) in &first.delta.values {
                    if key == "layer_height" {
                        continue;
                    }
                    let Some(second_value) = second.delta.values.get(key) else {
                        continue;
                    };
                    if first_value != second_value {
                        return Err(LayerRangeLoadError::ConflictingOverlap {
                            object_id: object_id.clone(),
                            key: key.clone(),
                            first_range: first.range_index,
                            second_range: second.range_index,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn warning_order(first: &IngestionWarning, second: &IngestionWarning) -> std::cmp::Ordering {
    match (first, second) {
        (
            IngestionWarning::UnrecognizedKey {
                wire_key: first_wire,
                key: first_key,
                suggestion: first_suggestion,
            },
            IngestionWarning::UnrecognizedKey {
                wire_key: second_wire,
                key: second_key,
                suggestion: second_suggestion,
            },
        ) => first_wire
            .cmp(second_wire)
            .then_with(|| first_key.cmp(second_key))
            .then_with(|| first_suggestion.cmp(second_suggestion)),
        (
            IngestionWarning::UntypedValue {
                key: first_key,
                declared_type: first_type,
                authored: first_authored,
            },
            IngestionWarning::UntypedValue {
                key: second_key,
                declared_type: second_type,
                authored: second_authored,
            },
        ) => first_key
            .cmp(second_key)
            .then_with(|| first_type.cmp(second_type))
            .then_with(|| first_authored.cmp(second_authored)),
        (IngestionWarning::UnrecognizedKey { .. }, IngestionWarning::UntypedValue { .. }) => {
            std::cmp::Ordering::Less
        }
        (IngestionWarning::UntypedValue { .. }, IngestionWarning::UnrecognizedKey { .. }) => {
            std::cmp::Ordering::Greater
        }
    }
}

/// Legacy config-key spellings and the canonical key each resolves to.
///
/// This table mirrors `CONFIG_KEY_ALIASES` in
/// `slicer_scheduler::config_resolution` (the scheduler crate depends on this
/// one, so the table cannot be shared in that direction; keep both tables in
/// sync when adding an alias). It exists here only to answer the
/// declared-ness question during ingestion: a legacy spelling of a
/// registry-declared key is not "undeclared" and must survive to
/// scheduler-side canonicalization and both-spellings conflict detection
/// (`reject_alias_conflicts`).
const CONFIG_KEY_ALIASES: [(&str, &str); 2] = [
    ("first_layer_line_width", "initial_layer_line_width"),
    ("support_overhang_angle", "support_threshold_angle"),
];

/// Resolve a legacy key spelling to its canonical key, mirroring
/// `slicer_scheduler::config_resolution::canonical_config_key`.
fn canonical_config_key(key: &str) -> &str {
    for (legacy, canonical) in CONFIG_KEY_ALIASES {
        if key == legacy {
            return canonical;
        }
    }
    key
}

fn decode_wire_key(wire_key: &str) -> Result<(ConfigScope, ConfigKey), ConfigIngestionError> {
    if wire_key == "object_config" || wire_key.starts_with("object_config:") {
        let parts = wire_key.split(':').collect::<Vec<_>>();
        if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
            return Err(ConfigIngestionError::MalformedScopeKey {
                wire_key: wire_key.to_owned(),
                expected: "object_config:<object_id>:<key>".to_owned(),
            });
        }
        return Ok((
            ConfigScope::Object(parts[1].to_owned()),
            parts[2].to_owned(),
        ));
    }

    if wire_key == "paint_config" || wire_key.starts_with("paint_config:") {
        let parts = wire_key.split(':').collect::<Vec<_>>();
        if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
            return Err(ConfigIngestionError::MalformedScopeKey {
                wire_key: wire_key.to_owned(),
                expected: "paint_config:<semantic>:<key>".to_owned(),
            });
        }
        return Ok((
            ConfigScope::PaintSemantic(parts[1].to_owned()),
            parts[2].to_owned(),
        ));
    }

    if wire_key == "tool_config" || wire_key.starts_with("tool_config:") {
        let parts = wire_key.split(':').collect::<Vec<_>>();
        let tool = parts.get(1).and_then(|value| value.parse::<u32>().ok());
        if parts.len() != 3 || tool.is_none() || parts[2].is_empty() {
            return Err(ConfigIngestionError::MalformedScopeKey {
                wire_key: wire_key.to_owned(),
                expected: "tool_config:<u32>:<key>".to_owned(),
            });
        }
        return Ok((
            ConfigScope::Tool(tool.expect("validated tool index")),
            parts[2].to_owned(),
        ));
    }

    Ok((ConfigScope::Global, wire_key.to_owned()))
}

fn coerce_value(
    key: &str,
    authored: &ConfigValue,
    field_type: &str,
) -> Result<ConfigValue, ConfigIngestionError> {
    let mismatch = || ConfigIngestionError::TypeMismatch {
        key: key.to_owned(),
        expected: field_type.to_owned(),
        authored: authored_text(authored),
    };

    match field_type {
        "bool" => match authored {
            ConfigValue::Bool(value) => Ok(ConfigValue::Bool(*value)),
            ConfigValue::String(value) => parse_bool(value)
                .map(ConfigValue::Bool)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "int" => match authored {
            ConfigValue::Int(value) => Ok(ConfigValue::Int(*value)),
            ConfigValue::String(value) => value
                .trim()
                .parse::<i64>()
                .map(ConfigValue::Int)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "float" => match authored {
            ConfigValue::Float(value) if value.is_finite() => Ok(ConfigValue::Float(*value)),
            ConfigValue::Int(value) => Ok(ConfigValue::Float(*value as f64)),
            ConfigValue::String(value) => parse_finite_float(value)
                .map(ConfigValue::Float)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "string" | "enum" => match authored {
            ConfigValue::String(value) => Ok(ConfigValue::String(value.clone())),
            _ => Err(mismatch()),
        },
        "percent" => match authored {
            ConfigValue::Percent(value) => Ok(ConfigValue::Percent(*value)),
            ConfigValue::FloatOrPercent { value, .. } if value.is_finite() => {
                Ok(ConfigValue::Percent(*value))
            }
            ConfigValue::Int(value) => Ok(ConfigValue::Percent(*value as f64)),
            ConfigValue::Float(value) if value.is_finite() => Ok(ConfigValue::Percent(*value)),
            ConfigValue::String(value) => parse_percent(value)
                .map(ConfigValue::Percent)
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "float_or_percent" => match authored {
            ConfigValue::FloatOrPercent { value, is_percent } => Ok(ConfigValue::FloatOrPercent {
                value: *value,
                is_percent: *is_percent,
            }),
            ConfigValue::Percent(value) if value.is_finite() => Ok(ConfigValue::FloatOrPercent {
                value: *value,
                is_percent: true,
            }),
            ConfigValue::Int(value) => Ok(ConfigValue::FloatOrPercent {
                value: *value as f64,
                is_percent: false,
            }),
            ConfigValue::Float(value) if value.is_finite() => Ok(ConfigValue::FloatOrPercent {
                value: *value,
                is_percent: false,
            }),
            ConfigValue::String(value) => parse_float_or_percent(value)
                .map(|(value, is_percent)| ConfigValue::FloatOrPercent { value, is_percent })
                .map_err(|_| mismatch()),
            _ => Err(mismatch()),
        },
        "float-list" => {
            // Canonical bed-point wire (canonical
            // `ConfigOptionPoints::deserialize`): entries split on ',' and
            // each coordinate pair on 'x', so a point string like "0x0"
            // deserializes into two flat coordinates (the `bed_shape`
            // flat-pair model). Numeric entries stay single floats.
            let entries: Vec<&ConfigValue> = match authored {
                ConfigValue::List(items) => items.iter().collect(),
                other => vec![other],
            };
            let mut flat = Vec::with_capacity(entries.len() * 2);
            for item in entries {
                match item {
                    ConfigValue::Float(value) if value.is_finite() => {
                        flat.push(ConfigValue::Float(*value));
                    }
                    ConfigValue::Int(value) => flat.push(ConfigValue::Float(*value as f64)),
                    ConfigValue::String(text) => {
                        for entry in text
                            .trim()
                            .split(',')
                            .filter(|part| !part.trim().is_empty())
                        {
                            if let Some((x, y)) = parse_point_pair(entry) {
                                flat.push(ConfigValue::Float(x));
                                flat.push(ConfigValue::Float(y));
                            } else if let Ok(value) = parse_finite_float(entry) {
                                flat.push(ConfigValue::Float(value));
                            } else {
                                return Err(mismatch());
                            }
                        }
                    }
                    _ => return Err(mismatch()),
                }
            }
            Ok(ConfigValue::List(flat))
        }
        "int-list" => coerce_list(key, authored, field_type, |item| match item {
            ConfigValue::Int(value) => Some(ConfigValue::Int(*value)),
            ConfigValue::String(value) => value.parse::<i64>().ok().map(ConfigValue::Int),
            _ => None,
        }),
        "string-list" => coerce_list(key, authored, field_type, |item| match item {
            ConfigValue::String(value) => Some(ConfigValue::String(value.clone())),
            _ => None,
        }),
        _ => Err(mismatch()),
    }
}

fn coerce_scalar_list(
    key: &str,
    authored: &ConfigValue,
    field_type: &str,
) -> Result<Option<ConfigValue>, ConfigIngestionError> {
    let ConfigValue::List(items) = authored else {
        return Ok(None);
    };
    if field_type.ends_with("-list") {
        return Ok(None);
    }

    let mut coerced = Vec::with_capacity(items.len());
    for item in items {
        match coerce_value(key, item, field_type) {
            Ok(value) => coerced.push(value),
            Err(error) if is_non_finite_scalar_value(item, field_type) => return Err(error),
            Err(_) => return Ok(None),
        }
    }
    Ok(Some(ConfigValue::List(coerced)))
}

fn is_non_finite_type_mismatch(authored: &ConfigValue, field_type: &str) -> bool {
    match authored {
        ConfigValue::List(items) if field_type.ends_with("-list") => {
            items.iter().any(|item| match field_type {
                "float-list" => is_non_finite_scalar_value(item, "float"),
                _ => false,
            })
        }
        ConfigValue::List(items) => items
            .iter()
            .any(|item| is_non_finite_scalar_value(item, field_type)),
        _ => is_non_finite_scalar_value(authored, field_type),
    }
}

fn is_non_finite_scalar_value(authored: &ConfigValue, field_type: &str) -> bool {
    match (field_type, authored) {
        ("float", ConfigValue::Float(value)) => !value.is_finite(),
        ("float", ConfigValue::String(value)) => is_non_finite_float_text(value),
        ("percent", ConfigValue::FloatOrPercent { value, .. }) => !value.is_finite(),
        ("percent", ConfigValue::Float(value)) => !value.is_finite(),
        ("percent", ConfigValue::String(value)) => is_non_finite_percent_text(value),
        ("float_or_percent", ConfigValue::Percent(value)) => !value.is_finite(),
        ("float_or_percent", ConfigValue::Float(value)) => !value.is_finite(),
        ("float_or_percent", ConfigValue::String(value)) => {
            is_non_finite_float_or_percent_text(value)
        }
        _ => false,
    }
}

fn is_non_finite_float_text(value: &str) -> bool {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .is_some_and(|value| !value.is_finite())
}

fn is_non_finite_percent_text(value: &str) -> bool {
    value
        .trim()
        .strip_suffix('%')
        .is_some_and(is_non_finite_float_text)
}

fn is_non_finite_float_or_percent_text(value: &str) -> bool {
    value
        .trim()
        .strip_suffix('%')
        .map_or_else(|| is_non_finite_float_text(value), is_non_finite_float_text)
}

fn coerce_list<F>(
    key: &str,
    authored: &ConfigValue,
    field_type: &str,
    mut convert: F,
) -> Result<ConfigValue, ConfigIngestionError>
where
    F: FnMut(&ConfigValue) -> Option<ConfigValue>,
{
    let ConfigValue::List(items) = authored else {
        return Err(ConfigIngestionError::TypeMismatch {
            key: key.to_owned(),
            expected: field_type.to_owned(),
            authored: authored_text(authored),
        });
    };

    items
        .iter()
        .map(&mut convert)
        .collect::<Option<Vec<_>>>()
        .map(ConfigValue::List)
        .ok_or_else(|| ConfigIngestionError::TypeMismatch {
            key: key.to_owned(),
            expected: field_type.to_owned(),
            authored: authored_text(authored),
        })
}

/// Parse one canonical bed-point entry ("XxY", canonical
/// `ConfigOptionPoints::deserialize`) into its two flat coordinates.
fn parse_point_pair(value: &str) -> Option<(f64, f64)> {
    let (x, y) = value.trim().split_once('x')?;
    Some((parse_finite_float(x).ok()?, parse_finite_float(y).ok()?))
}

fn parse_finite_float(value: &str) -> Result<f64, ()> {
    // Canonical scalar wire (canonical `ConfigOptionFloat::deserialize`): a
    // numeric string parses with its optional trailing `%` ignored, yielding
    // the percent magnitude — mirrors `ConfigView::get_float`'s string branch
    // so validation and readers share one wire vocabulary.
    let trimmed = value.trim();
    let trimmed = trimmed.strip_suffix('%').unwrap_or(trimmed).trim();
    let value = trimmed.parse::<f64>().map_err(|_| ())?;
    value.is_finite().then_some(value).ok_or(())
}

fn parse_bool(value: &str) -> Result<bool, ()> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("true") || trimmed == "1" {
        return Ok(true);
    }
    if trimmed.eq_ignore_ascii_case("false") || trimmed == "0" {
        return Ok(false);
    }
    Err(())
}

fn parse_percent(value: &str) -> Result<f64, ()> {
    let trimmed = value.trim();
    let number = trimmed.strip_suffix('%').ok_or(())?;
    parse_finite_float(number)
}

fn parse_float_or_percent(value: &str) -> Result<(f64, bool), ()> {
    let trimmed = value.trim();
    if let Some(number) = trimmed.strip_suffix('%') {
        return Ok((parse_finite_float(number)?, true));
    }
    Ok((parse_finite_float(trimmed)?, false))
}

fn authored_text(value: &ConfigValue) -> String {
    match value {
        ConfigValue::Bool(value) => value.to_string(),
        ConfigValue::Int(value) => value.to_string(),
        ConfigValue::Float(value) => value.to_string(),
        ConfigValue::String(value) => value.clone(),
        ConfigValue::List(value) => format!("{value:?}"),
        ConfigValue::Percent(value) => format!("{value}%"),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                format!("{value}%")
            } else {
                value.to_string()
            }
        }
    }
}

fn levenshtein(first: &str, second: &str) -> usize {
    let second_chars = second.chars().collect::<Vec<_>>();
    let mut previous = (0..=second_chars.len()).collect::<Vec<_>>();
    for (first_index, first_char) in first.chars().enumerate() {
        let mut current = vec![first_index + 1; second_chars.len() + 1];
        for (second_index, second_char) in second_chars.iter().enumerate() {
            current[second_index + 1] = if first_char == *second_char {
                previous[second_index]
            } else {
                1 + previous[second_index]
                    .min(previous[second_index + 1])
                    .min(current[second_index])
            };
        }
        previous = current;
    }
    previous[second_chars.len()]
}
