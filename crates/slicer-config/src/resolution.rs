//! Deterministic resolution of typed configuration scope deltas.

use std::collections::BTreeMap;
use std::fmt;

use slicer_ir::{ConfigResolutionError, ConfigValue, ResolvedConfig};

use crate::{
    expand_automatic_values, ConfigSchemaRegistry, ConfigScope, ExpansionContext, ExpansionError,
    ScopeDelta, ScopedConfig,
};

/// The object-level planning values needed to construct the shared Z grid.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedObjectLayerConfig {
    /// Stable object identifier.
    pub object_id: String,
    /// Object height in millimetres.
    pub object_height: f64,
    /// Effective ordinary layer height in millimetres.
    pub layer_height: f64,
    /// Effective first-layer height in millimetres.
    pub first_layer_height: f64,
    /// Number of support raft layers preceding model layers.
    pub support_raft_layers: u32,
}

/// Applicable typed scopes for one resolution query.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolutionTarget {
    /// Object whose object and modifier deltas apply.
    pub object_id: String,
    /// Applicable modifier identifiers in ascending priority order.
    pub modifier_ids: Vec<String>,
    /// Selected paint semantics. Resolution applies them in lexical order.
    pub paint_semantics: Vec<String>,
    /// Selected tool, when tool-specific configuration applies.
    pub tool_index: Option<u32>,
}

/// Failure while resolving a typed configuration scope stack.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolutionError {
    /// A delta could not be applied to its typed `ResolvedConfig` field.
    Application(ConfigResolutionError),
    /// Phase-B automatic-value expansion failed.
    Expansion(ExpansionError),
    /// An object height was absent, non-finite, zero, or negative.
    InvalidObjectHeight {
        /// Object carrying the invalid height.
        object_id: String,
        /// Invalid finite value, or `None` when the value was non-finite.
        value: Option<f64>,
    },
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Application(error) => error.fmt(formatter),
            Self::Expansion(error) => error.fmt(formatter),
            Self::InvalidObjectHeight { object_id, value } => match value {
                Some(value) => write!(
                    formatter,
                    "object {object_id:?} height must be positive and finite, got {value}"
                ),
                None => write!(
                    formatter,
                    "object {object_id:?} height must be positive and finite"
                ),
            },
        }
    }
}

impl std::error::Error for ResolutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Application(error) => Some(error),
            Self::Expansion(error) => Some(error),
            Self::InvalidObjectHeight { .. } => None,
        }
    }
}

impl From<ConfigResolutionError> for ResolutionError {
    fn from(error: ConfigResolutionError) -> Self {
        Self::Application(error)
    }
}

impl From<ExpansionError> for ResolutionError {
    fn from(error: ExpansionError) -> Self {
        Self::Expansion(error)
    }
}

fn apply_delta(
    config: &mut ResolvedConfig,
    delta: Option<&ScopeDelta>,
) -> Result<(), ResolutionError> {
    let Some(delta) = delta else {
        return Ok(());
    };

    for (key, value) in delta.iter() {
        if !config.apply_cli_key(key, value)? {
            config.extensions.insert(key.clone(), value.clone());
        }
    }
    Ok(())
}

/// Resolve all applicable deltas in canonical precedence order, then run Phase B.
///
/// Precedence is global, object, the reserved layer-range seam, modifiers in
/// target order, paint semantics in lexical order, and finally the selected
/// tool. A delta's presence, including a value equal to the default, is always
/// an override.
pub fn resolve_scope_stack(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    target: &ResolutionTarget,
    expansion: &ExpansionContext,
) -> Result<ResolvedConfig, ResolutionError> {
    let mut config = ResolvedConfig::default();

    apply_delta(&mut config, scoped.global())?;
    apply_delta(
        &mut config,
        scoped.delta(&ConfigScope::Object(target.object_id.clone())),
    )?;

    // Reserved precedence seam: layer-range deltas belong here once their
    // typed source and applicability model land.

    for modifier_id in &target.modifier_ids {
        apply_delta(
            &mut config,
            scoped.delta(&ConfigScope::Modifier {
                object_id: target.object_id.clone(),
                modifier_id: modifier_id.clone(),
            }),
        )?;
    }

    let mut paint_semantics = target.paint_semantics.clone();
    paint_semantics.sort();
    paint_semantics.dedup();
    for semantic in paint_semantics {
        apply_delta(
            &mut config,
            scoped.delta(&ConfigScope::PaintSemantic(semantic)),
        )?;
    }

    if let Some(tool_index) = target.tool_index {
        apply_delta(&mut config, scoped.delta(&ConfigScope::Tool(tool_index)))?;
    }

    expand_automatic_values(registry, &mut config, expansion, target.tool_index)?;
    Ok(config)
}

/// Resolve object planning inputs in object-id order for shared Z-grid construction.
///
/// All object heights are validated before any scope stack is resolved, so an
/// invalid height rejects the query atomically.
pub fn query_z_grid(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    object_heights: &BTreeMap<String, f64>,
    expansion: &ExpansionContext,
) -> Result<Vec<ResolvedObjectLayerConfig>, ResolutionError> {
    for (object_id, object_height) in object_heights {
        if !object_height.is_finite() || *object_height <= 0.0 {
            return Err(ResolutionError::InvalidObjectHeight {
                object_id: object_id.clone(),
                value: object_height.is_finite().then_some(*object_height),
            });
        }
    }

    object_heights
        .iter()
        .map(|(object_id, object_height)| {
            let config = resolve_scope_stack(
                registry,
                scoped,
                &ResolutionTarget {
                    object_id: object_id.clone(),
                    ..ResolutionTarget::default()
                },
                expansion,
            )?;
            let support_raft_layers = support_raft_layers(&config)?;

            Ok(ResolvedObjectLayerConfig {
                object_id: object_id.clone(),
                object_height: *object_height,
                layer_height: config.layer_height,
                first_layer_height: config.first_layer_height,
                support_raft_layers,
            })
        })
        .collect()
}

fn support_raft_layers(config: &ResolvedConfig) -> Result<u32, ResolutionError> {
    let Some(value) = config.extensions.get("support_raft_layers") else {
        return Ok(0);
    };

    match value {
        ConfigValue::Int(value) => u32::try_from(*value).map_err(|_| {
            ConfigResolutionError::OutOfRange {
                key: "support_raft_layers".to_owned(),
                value: *value as f64,
                min: Some(0.0),
                max: Some(f64::from(u32::MAX)),
                index: None,
            }
            .into()
        }),
        other => Err(ConfigResolutionError::TypeMismatch {
            key: "support_raft_layers".to_owned(),
            expected: "Int",
            actual: config_value_name(other).to_owned(),
        }
        .into()),
    }
}

fn config_value_name(value: &ConfigValue) -> &'static str {
    match value {
        ConfigValue::Bool(_) => "Bool",
        ConfigValue::Int(_) => "Int",
        ConfigValue::Float(_) => "Float",
        ConfigValue::String(_) => "String",
        ConfigValue::Percent(_) => "Percent",
        ConfigValue::FloatOrPercent { .. } => "FloatOrPercent",
        ConfigValue::List(_) => "List",
    }
}
