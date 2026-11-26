//! Opt-in property tests for this module's private single-pass placeholder engine.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_16_pbt-io-gcode/packet.spec.md`
//! § H-GCODE-TEMPLATE; registry record `H-GCODE-TEMPLATE` in
//! `test-properties.toml`):
//!
//! * `H-GCODE-TEMPLATE` — the real private `substitute_placeholders` and
//!   `format_placeholder_value` against an independent left-to-right,
//!   line-scoped byte scanner, plus the real `run_gcode_postprocess` over a
//!   bounded sequence of every `GCodeCommand` variant with empty templates.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `substitute_placeholders`, `format_placeholder_value` and the `MachineGcodeEmit`
//! `PostpassModule` entrypoint are reachable from this child module without
//! widening production visibility; the substitution kernels stay private.
//!
//! # Independent expected model
//!
//! The checker never calls the engine to obtain an expected value.
//!
//! * `scan_template` re-implements the documented contract a different way:
//!   it splits the template into lines, walks each line with `str::find`, and
//!   appends substituted values to the output *without ever scanning them*.
//!   The production scanner walks a flat byte index instead. The line split
//!   is the "`]` before the next newline" rule made structural.
//! * The lookup handed to the SUT is built from an explicit value-kind table
//!   ([`ValueKind::expected`]) that states each rendering as a literal string,
//!   including the empty-list (`None`), first-element-recursion (`Some("3")`)
//!   and unsupported `Percent` / `FloatOrPercent` (`None`) cases. The engine's
//!   own `format_placeholder_value` is then compared to that table.
//! * `expected_passthrough` maps an input command stream to the documented
//!   re-emission shape (`ExtrusionMode` becomes `Raw("M82")`/`Raw("M83")`;
//!   every other variant is preserved verbatim) instead of copying
//!   `reemit_command`.
//!
//! No recursion is claimed: substituted values are inserted verbatim. The
//! `TemplateModel::rescan_substituted` mutation exists only so the controls can
//! show that a recursively-rescanning oracle is rejected; it is bounded at
//! [`RESCAN_DEPTH_LIMIT`] so a self-referential value table still terminates.
//!
//! Canonical `PlaceholderParser::process` throws on an unknown key. PnP's
//! warn-and-pass (verbatim `[key]`, one aggregated warning) is deliberate and
//! is what this property asserts; it is **not** a canonical-parity claim.
//!
//! # Literal review (manual; the scanners do not cover this file)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`.
//!
//! Watched-type literal sites here: **none**. The only IR value constructed in
//! this file is `GCodeCommand`, an enum whose struct-variants are explicitly
//! excluded from the watchlist (`collect_watchlist` records struct definitions
//! only; `docs/21_data_defaults_and_fixtures.md` §3). `ConfigView` is built
//! through `ConfigView::from_map`. `TemplateCase`, `ValueEntry`,
//! `TemplateObservation`, `TemplateLimits`, `TemplateModel`, `CommandModel`,
//! `Token` and the mutation enums are module-private test types, not watched
//! crate-public structs. No FRU waiver is therefore required — and none is
//! claimed: this is a manual review, not a scanner pass.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap.
//! `template_controls` is a set of mutation controls over the **same**
//! `check_template_case` / `check_command_passthrough` the property uses: each
//! first asserts the unmodified claim is accepted, then asserts the
//! single-field mutation is rejected, and the rejection message names the
//! surface that disagreed. The property requires non-vacuous positive
//! populations (counters asserted after the run), so a generator regression
//! that stopped producing `[`-bearing templates would fail rather than pass
//! silently. The `GCodeCommand` coverage roster is not a free-hand list: the
//! `command_variant_index` match has no wildcard arm, so a new variant stops
//! this file from compiling instead of silently dropping out of the domain.
//! The shrink probe asserts a nonminimal fixed-shape witness per malformed
//! mode; it does not claim global minimality or a shrink-step count, and the
//! `no_shrink` probe is a determinism pin on the mode selector.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole, GCodeCommand, RetractMode};
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::postpass_types::GcodeOutputCommand;
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::traits::PostpassModule;

use super::{format_placeholder_value, substitute_placeholders, MachineGcodeEmit};

const PROPERTY_ID: &str = "H-GCODE-TEMPLATE";
const PROPERTY_TEST: &str = "property_tests::proptest_placeholder_substitution";
/// Exact `PropertyRun::bound` keys of the registry record.
const BOUND_KEYS: [&str; 2] = ["max_template_bytes", "max_placeholders"];
/// The registry binds no command-count ceiling; this is a fixed small bound.
const MAX_COMMANDS: usize = 6;
/// Bounded rescan depth for the recursion mutation (see the module docs).
const RESCAN_DEPTH_LIMIT: usize = 3;

/// Deterministic control values: the registry routine and extended profiles
/// both bind `max_template_bytes = 64`, `max_placeholders = 6`.
const CONTROL_MAX_TEMPLATE_BYTES: usize = 64;
const CONTROL_MAX_PLACEHOLDERS: usize = 6;

/// Known lookup keys. `café` is non-ASCII and is also used by the forced
/// nested-bracket witness; `other` is pinned to the no-rescan probe value so a
/// re-scanning implementation is always observable when the template names it.
const KEY_TABLE: [&str; 6] = ["speed", "bed_temp", "tool", "z", "café", "other"];
/// Index of the key whose lookup value always contains `[other]`.
const RESCAN_KEY_INDEX: usize = 5;
/// Keys that are never present in the generated lookup (disjoint from
/// [`KEY_TABLE`] by construction).
const UNKNOWN_TABLE: [&str; 3] = ["missing", "nope", "unknown"];
/// Literal-run alphabet: ASCII plus two multi-byte characters.
const LITERAL_ALPHABET: [char; 5] = ['a', 'b', ' ', 'é', '☕'];

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct TemplateLimits {
    max_template_bytes: usize,
    max_placeholders: usize,
    max_tokens: usize,
    max_commands: usize,
}

fn live_limits(run: &PropertyRun) -> TemplateLimits {
    let mut bounds = BTreeMap::new();
    for key in BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("H-GCODE-TEMPLATE bound `{key}`: {error}"));
        bounds.insert(key, value);
    }
    let max_template_bytes =
        usize::try_from(bounds["max_template_bytes"]).expect("template byte bound fits in usize");
    let max_placeholders =
        usize::try_from(bounds["max_placeholders"]).expect("placeholder-count bound fits in usize");
    TemplateLimits {
        max_template_bytes,
        max_placeholders,
        max_tokens: 2 * max_placeholders + 2,
        max_commands: MAX_COMMANDS,
    }
}

/// Deterministic controls have no `PropertyRun` context; these are the exact
/// registry profile values.
fn control_limits() -> TemplateLimits {
    TemplateLimits {
        max_template_bytes: CONTROL_MAX_TEMPLATE_BYTES,
        max_placeholders: CONTROL_MAX_PLACEHOLDERS,
        max_tokens: 2 * CONTROL_MAX_PLACEHOLDERS + 2,
        max_commands: MAX_COMMANDS,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Construction-first token sequence
// ─────────────────────────────────────────────────────────────────────────────

/// One construction-first template token. Literals never contain `[`, `]` or
/// `\n`; the bracket-bearing shapes are their own tokens, so every token's
/// contribution to the rendered template is unambiguous.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// A literal run that contains no bracket and no newline (may be empty).
    Literal(String),
    /// `[key]` with a key from [`KEY_TABLE`].
    Known(u32),
    /// `[key]` with a key that is never in the lookup.
    Unknown(u32),
    /// `[]` — an empty bracketed key.
    EmptyBrackets,
    /// A bare `[` with no `]` before the next newline.
    UnclosedOpen,
    /// A `]` before any `[`.
    StrayClose,
    /// `[[key]]` — nested brackets around a key.
    Nested(u32),
    /// A line separator.
    Newline,
}

const TOKEN_KINDS: u16 = 8;

impl Token {
    fn decode(raw: u16) -> Self {
        let kind = raw % TOKEN_KINDS;
        let payload = raw / TOKEN_KINDS;
        match kind {
            0 => Self::Literal(literal_text(payload)),
            1 => Self::Known(u32::from(payload % KEY_TABLE.len() as u16)),
            2 => Self::Unknown(u32::from(payload % UNKNOWN_TABLE.len() as u16)),
            3 => Self::EmptyBrackets,
            4 => Self::UnclosedOpen,
            5 => Self::StrayClose,
            6 => Self::Nested(u32::from(payload % KEY_TABLE.len() as u16)),
            _ => Self::Newline,
        }
    }

    fn render(&self) -> String {
        match self {
            Self::Literal(text) => text.clone(),
            Self::Known(index) => format!("[{}]", KEY_TABLE[*index as usize]),
            Self::Unknown(index) => format!("[{}]", UNKNOWN_TABLE[*index as usize]),
            Self::EmptyBrackets => "[]".to_string(),
            Self::UnclosedOpen => "[".to_string(),
            Self::StrayClose => "]".to_string(),
            Self::Nested(index) => format!("[[{}]]", KEY_TABLE[*index as usize]),
            Self::Newline => "\n".to_string(),
        }
    }

    fn is_placeholder(&self) -> bool {
        matches!(
            self,
            Self::Known(_) | Self::Unknown(_) | Self::EmptyBrackets | Self::Nested(_)
        )
    }
}

fn literal_text(mut payload: u16) -> String {
    let len = usize::from(payload % 5);
    payload /= 5;
    let mut text = String::new();
    for _ in 0..len {
        text.push(LITERAL_ALPHABET[(payload % 5) as usize]);
        payload /= 5;
    }
    text
}

/// The fixed bracket-shape choice kept stable through shrinking. Chosen with
/// `prop::sample::select(...).no_shrink()` in the property so a shrink cannot
/// switch the malformed class mid-search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MalformedMode {
    UnclosedOpen,
    StrayClose,
    EmptyBrackets,
    Nested,
}

const MALFORMED_MODES: [MalformedMode; 4] = [
    MalformedMode::UnclosedOpen,
    MalformedMode::StrayClose,
    MalformedMode::EmptyBrackets,
    MalformedMode::Nested,
];

impl MalformedMode {
    fn token(self) -> Token {
        match self {
            Self::UnclosedOpen => Token::UnclosedOpen,
            Self::StrayClose => Token::StrayClose,
            Self::EmptyBrackets => Token::EmptyBrackets,
            // `café` is multi-byte, so the nested witness is also non-ASCII.
            Self::Nested => Token::Nested(4),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::UnclosedOpen => "unclosed-open",
            Self::StrayClose => "stray-close",
            Self::EmptyBrackets => "empty-brackets",
            Self::Nested => "nested",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Lookup values: fixed table, exact expected rendering
// ─────────────────────────────────────────────────────────────────────────────

/// One generated lookup value shape. The expected rendering is a literal
/// string in [`ValueKind::expected`], never a call to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    EmptyString,
    PlainText,
    /// Contains `[other]` so a re-scanning implementation is observable.
    RescanProbe,
    UnicodeText,
    ZeroInt,
    NegativeInt,
    PositiveInt,
    HalfFloat,
    TrueBool,
    EmptyList,
    ScalarList,
    /// `List([List([Int(3)])])` — first-element recursion reaches a scalar.
    NestedList,
    /// `List([Percent(25)])` — first element is unsupported, so `None`.
    PercentList,
    PercentValue,
    FloatOrPercentValue,
    /// `List([List([])])` — recursion reaches an empty list, so `None`.
    NestedEmptyList,
}

const VALUE_KIND_COUNT: u8 = 16;

impl ValueKind {
    fn from_raw(raw: u8) -> Self {
        match raw % VALUE_KIND_COUNT {
            0 => Self::EmptyString,
            1 => Self::PlainText,
            2 => Self::RescanProbe,
            3 => Self::UnicodeText,
            4 => Self::ZeroInt,
            5 => Self::NegativeInt,
            6 => Self::PositiveInt,
            7 => Self::HalfFloat,
            8 => Self::TrueBool,
            9 => Self::EmptyList,
            10 => Self::ScalarList,
            11 => Self::NestedList,
            12 => Self::PercentList,
            13 => Self::PercentValue,
            14 => Self::FloatOrPercentValue,
            _ => Self::NestedEmptyList,
        }
    }

    fn to_config(self) -> ConfigValue {
        match self {
            Self::EmptyString => ConfigValue::String(String::new()),
            Self::PlainText => ConfigValue::String("plain".to_string()),
            Self::RescanProbe => ConfigValue::String("has [other] inside".to_string()),
            Self::UnicodeText => ConfigValue::String("café ☕".to_string()),
            Self::ZeroInt => ConfigValue::Int(0),
            Self::NegativeInt => ConfigValue::Int(-7),
            Self::PositiveInt => ConfigValue::Int(42),
            Self::HalfFloat => ConfigValue::Float(0.5),
            Self::TrueBool => ConfigValue::Bool(true),
            Self::EmptyList => ConfigValue::List(Vec::new()),
            Self::ScalarList => ConfigValue::List(vec![ConfigValue::Int(7)]),
            Self::NestedList => {
                ConfigValue::List(vec![ConfigValue::List(vec![ConfigValue::Int(3)])])
            }
            Self::PercentList => ConfigValue::List(vec![ConfigValue::Percent(25.0)]),
            Self::PercentValue => ConfigValue::Percent(25.0),
            Self::FloatOrPercentValue => ConfigValue::FloatOrPercent {
                value: 50.0,
                is_percent: true,
            },
            Self::NestedEmptyList => ConfigValue::List(vec![ConfigValue::List(Vec::new())]),
        }
    }

    /// The documented rendering, stated as an exact string.
    fn expected(self) -> Option<&'static str> {
        match self {
            Self::EmptyString => Some(""),
            Self::PlainText => Some("plain"),
            Self::RescanProbe => Some("has [other] inside"),
            Self::UnicodeText => Some("café ☕"),
            Self::ZeroInt => Some("0"),
            Self::NegativeInt => Some("-7"),
            Self::PositiveInt => Some("42"),
            Self::HalfFloat => Some("0.5"),
            Self::TrueBool => Some("true"),
            Self::ScalarList => Some("7"),
            Self::NestedList => Some("3"),
            Self::EmptyList
            | Self::PercentList
            | Self::PercentValue
            | Self::FloatOrPercentValue
            | Self::NestedEmptyList => None,
        }
    }
}

/// One key/value pair handed to the substitution engine. `rendered` is the
/// exact expected rendering, or `None` when the key must stay out of the
/// lookup (so its placeholder stays verbatim and is reported unresolved).
#[derive(Debug, Clone)]
struct ValueEntry {
    key: &'static str,
    value: ConfigValue,
    rendered: Option<&'static str>,
}

impl ValueEntry {
    fn new(key: &'static str, value: ConfigValue, rendered: Option<&'static str>) -> Self {
        Self {
            key,
            value,
            rendered,
        }
    }

    fn from_kind(key: &'static str, kind: ValueKind) -> Self {
        Self::new(key, kind.to_config(), kind.expected())
    }
}

fn is_top_level_empty_list(value: &ConfigValue) -> bool {
    matches!(value, ConfigValue::List(items) if items.is_empty())
}

// ─────────────────────────────────────────────────────────────────────────────
// Cases
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct TemplateCase {
    template: String,
    tokens: Vec<Token>,
    entries: Vec<ValueEntry>,
}

/// Bytes are capped by stopping the token walk; placeholder count is capped by
/// skipping placeholder tokens. Neither is a rejection, so the profile's
/// `max_local_rejects = 0` holds for every generated case.
fn build_case(
    mode: MalformedMode,
    raws: &[u16],
    kinds: &[u8; KEY_TABLE.len() - 1],
    limits: &TemplateLimits,
) -> TemplateCase {
    let entries = KEY_TABLE
        .iter()
        .enumerate()
        .map(|(index, key)| {
            let kind = if index == RESCAN_KEY_INDEX {
                ValueKind::RescanProbe
            } else {
                ValueKind::from_raw(kinds.get(index).copied().unwrap_or(0))
            };
            ValueEntry::from_kind(key, kind)
        })
        .collect();

    let mut template = String::new();
    let mut tokens = Vec::new();
    let mut placeholders = 0usize;

    let forced = mode.token();
    let forced_text = forced.render();
    if forced_text.len() <= limits.max_template_bytes {
        if forced.is_placeholder() {
            placeholders += 1;
        }
        template.push_str(&forced_text);
        tokens.push(forced);
    }

    for raw in raws {
        let token = Token::decode(*raw);
        let text = token.render();
        if template.len() + text.len() > limits.max_template_bytes {
            break;
        }
        if token.is_placeholder() {
            if placeholders >= limits.max_placeholders {
                continue;
            }
            placeholders += 1;
        }
        template.push_str(&text);
        tokens.push(token);
    }

    TemplateCase {
        template,
        tokens,
        entries,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent scanner (the oracle)
// ─────────────────────────────────────────────────────────────────────────────

/// Mutation knobs for the controls. The property always passes
/// `TemplateModel::default()`.
#[derive(Debug, Clone, Copy, Default)]
struct TemplateModel {
    /// Re-scan substituted values (and their substitutes) as templates.
    rescan_substituted: bool,
    /// Report unresolved keys in reverse order instead of sorted.
    descending_unresolved: bool,
    /// Report the first unresolved key twice.
    duplicate_unresolved: bool,
    /// Treat a top-level empty list as an empty-string rendering.
    empty_list_is_empty_string: bool,
}

fn lookup_from(case: &TemplateCase, empty_list_is_empty_string: bool) -> HashMap<String, String> {
    case.entries
        .iter()
        .filter_map(|entry| {
            let rendered = entry.rendered.map(|text| text.to_string()).or_else(|| {
                if empty_list_is_empty_string && is_top_level_empty_list(&entry.value) {
                    Some(String::new())
                } else {
                    None
                }
            });
            rendered.map(|text| (entry.key.to_string(), text))
        })
        .collect()
}

/// Left-to-right, line-scoped scanner: at `[`, look for `]` before the end of
/// that line; found → insert the lookup value verbatim (never rescanned);
/// absent → the rest of the line is literal. Unresolved keys are reported
/// sorted and deduplicated.
fn scan_template(
    template: &str,
    lookup: &HashMap<String, String>,
    model: &TemplateModel,
    depth: usize,
) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut unresolved: Vec<String> = Vec::new();

    for (line_index, line) in template.split('\n').enumerate() {
        if line_index > 0 {
            out.push('\n');
        }
        let mut rest = line;
        loop {
            let Some(open) = rest.find('[') else {
                out.push_str(rest);
                break;
            };
            out.push_str(&rest[..open]);
            let after_open = &rest[open + 1..];
            let Some(close) = after_open.find(']') else {
                // No `]` on this line: the remainder is literal.
                out.push_str(&rest[open..]);
                break;
            };
            let key = &after_open[..close];
            rest = &after_open[close + 1..];
            match lookup.get(key) {
                Some(value) => {
                    if model.rescan_substituted && depth + 1 < RESCAN_DEPTH_LIMIT {
                        let (nested, nested_unresolved) =
                            scan_template(value, lookup, model, depth + 1);
                        out.push_str(&nested);
                        unresolved.extend(nested_unresolved);
                    } else {
                        out.push_str(value);
                    }
                }
                None => {
                    unresolved.push(key.to_string());
                    out.push('[');
                    out.push_str(key);
                    out.push(']');
                }
            }
        }
    }

    unresolved.sort();
    unresolved.dedup();
    if model.descending_unresolved {
        unresolved.reverse();
    }
    if model.duplicate_unresolved {
        if let Some(first) = unresolved.first().cloned() {
            unresolved.push(first);
        }
    }
    (out, unresolved)
}

#[derive(Debug, Clone)]
struct TemplateObservation {
    text: String,
    unresolved: Vec<String>,
}

fn check_template_case(
    case: &TemplateCase,
    model: &TemplateModel,
) -> Result<TemplateObservation, String> {
    for entry in &case.entries {
        let expected = if model.empty_list_is_empty_string && is_top_level_empty_list(&entry.value)
        {
            Some(String::new())
        } else {
            entry.rendered.map(|text| text.to_string())
        };
        let actual = format_placeholder_value(&entry.value);
        if actual != expected {
            return Err(format!(
                "format_placeholder_value({:?}) for `{}` returned {:?}, expected {:?}",
                entry.value, entry.key, actual, expected
            ));
        }
    }

    // The SUT input never depends on the mutation model: the honest table
    // decides which keys reach the lookup.
    let sut_lookup = lookup_from(case, false);
    let expected_lookup = lookup_from(case, model.empty_list_is_empty_string);
    let (expected_text, expected_unresolved) =
        scan_template(&case.template, &expected_lookup, model, 0);
    let (actual_text, actual_unresolved) = substitute_placeholders(&case.template, &sut_lookup);

    if actual_text != expected_text {
        return Err(format!(
            "rendered text for template {:?} was {:?}, expected {:?}",
            case.template, actual_text, expected_text
        ));
    }
    if actual_unresolved != expected_unresolved {
        return Err(format!(
            "unresolved keys for template {:?} were {:?}, expected {:?}",
            case.template, actual_unresolved, expected_unresolved
        ));
    }

    let mut sorted = actual_unresolved.clone();
    sorted.sort();
    sorted.dedup();
    if sorted != actual_unresolved {
        return Err(format!(
            "unresolved keys {:?} are not sorted and deduplicated",
            actual_unresolved
        ));
    }

    Ok(TemplateObservation {
        text: actual_text,
        unresolved: actual_unresolved,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Command pass-through
// ─────────────────────────────────────────────────────────────────────────────

const COORD_VALUES: [f32; 6] = [-2.0, -0.5, 0.0, 0.25, 1.0, 7.5];
const SPEED_VALUES: [f32; 3] = [0.0, 1200.0, 3600.5];
const TEMPERATURE_VALUES: [f32; 3] = [0.0, 200.0, 210.5];
const RETRACT_MODE_VALUES: [RetractMode; 2] = [RetractMode::Gcode, RetractMode::Firmware];
const MOVE_ROLE_VALUES: [ExtrusionRole; 3] = [
    ExtrusionRole::OuterWall,
    ExtrusionRole::SparseInfill,
    ExtrusionRole::BridgeInfill,
];
/// `Raw` texts that never open an injection walk (`;LAYER_CHANGE`) and never
/// look like an extrusion-role marker (`;TYPE:`). The pass-through contract
/// covers those two markers elsewhere; here every variant is driven with empty
/// templates only.
const RAW_TEXT_VALUES: [&str; 4] = ["G28", "M104 S200", "; note", ""];
const COMMENT_TEXT_VALUES: [&str; 3] = ["hello world", "; layer", "café ☕"];

#[derive(Debug, Clone, Copy, Default)]
struct CommandModel {
    /// Expect the two leading commands in swapped order.
    swap_leading_pair: bool,
}

/// Empty templates: only the module's registry/transport keys are present.
fn passthrough_config() -> ConfigView {
    ConfigView::from_map(HashMap::from([
        (
            "bed_temperature_initial_layer_single".to_string(),
            ConfigValue::Int(60),
        ),
        (
            "nozzle_temperature_initial_layer".to_string(),
            ConfigValue::Int(215),
        ),
    ]))
}

fn expected_passthrough(commands: &[GCodeCommand]) -> Vec<GCodeCommand> {
    commands
        .iter()
        .map(|command| match command {
            GCodeCommand::ExtrusionMode { absolute } => GCodeCommand::Raw {
                text: if *absolute { "M82" } else { "M83" }.to_string(),
            },
            other => other.clone(),
        })
        .collect()
}

fn check_command_passthrough(
    commands: &[GCodeCommand],
    model: &CommandModel,
) -> Result<Vec<GCodeCommand>, String> {
    let config = passthrough_config();
    let module = MachineGcodeEmit::from_config(&config)
        .map_err(|error| format!("from_config: {}", error.message))?;
    let mut output = GcodeOutputBuilder::new();
    module
        .run_gcode_postprocess(commands, &mut output, &config)
        .map_err(|error| format!("run_gcode_postprocess: {}", error.message))?;

    let mut expected = expected_passthrough(commands);
    if model.swap_leading_pair && expected.len() >= 2 {
        expected.swap(0, 1);
    }

    let emitted = output.commands();
    if emitted.len() != expected.len() {
        return Err(format!(
            "emitted {} commands for {} inputs",
            emitted.len(),
            expected.len()
        ));
    }
    for (index, command) in emitted.iter().enumerate() {
        match command {
            GcodeOutputCommand::Command(actual) => {
                if actual != &expected[index] {
                    return Err(format!(
                        "command {index} was {actual:?}, expected {:?}",
                        expected[index]
                    ));
                }
            }
            GcodeOutputCommand::ZHop { .. } => {
                return Err(format!("unexpected ZHop at output index {index}"));
            }
        }
    }
    Ok(expected)
}

fn command_strategy() -> impl Strategy<Value = GCodeCommand> {
    prop_oneof![
        any::<bool>().prop_map(|absolute| GCodeCommand::ExtrusionMode { absolute }),
        (
            prop::option::of(prop::sample::select(COORD_VALUES.to_vec())),
            prop::option::of(prop::sample::select(COORD_VALUES.to_vec())),
            prop::option::of(prop::sample::select(COORD_VALUES.to_vec())),
            prop::option::of(prop::sample::select(COORD_VALUES.to_vec())),
            prop::option::of(prop::sample::select(SPEED_VALUES.to_vec())),
            prop::sample::select(MOVE_ROLE_VALUES.to_vec()),
        )
            .prop_map(|(x, y, z, e, f, role)| GCodeCommand::Move {
                x,
                y,
                z,
                e,
                f,
                role
            }),
        (
            prop::sample::select(SPEED_VALUES.to_vec()),
            prop::sample::select(SPEED_VALUES.to_vec()),
            prop::sample::select(RETRACT_MODE_VALUES.to_vec()),
        )
            .prop_map(|(length, speed, mode)| GCodeCommand::Retract {
                length,
                speed,
                mode
            }),
        (
            prop::sample::select(SPEED_VALUES.to_vec()),
            prop::sample::select(SPEED_VALUES.to_vec()),
            prop::sample::select(RETRACT_MODE_VALUES.to_vec()),
        )
            .prop_map(|(length, speed, mode)| GCodeCommand::Unretract {
                length,
                speed,
                mode
            }),
        (0u8..=255).prop_map(|value| GCodeCommand::FanSpeed { value }),
        (
            0u32..=3,
            prop::sample::select(TEMPERATURE_VALUES.to_vec()),
            any::<bool>(),
        )
            .prop_map(|(tool, celsius, wait)| GCodeCommand::Temperature {
                tool,
                celsius,
                wait
            }),
        (0u32..=4, 0u32..=3, 0u32..=3).prop_map(|(after_entity_index, from, to)| {
            GCodeCommand::ToolChange {
                after_entity_index,
                from,
                to,
            }
        }),
        prop::sample::select(COMMENT_TEXT_VALUES.to_vec()).prop_map(|text| GCodeCommand::Comment {
            text: text.to_string()
        }),
        prop::sample::select(RAW_TEXT_VALUES.to_vec()).prop_map(|text| GCodeCommand::Raw {
            text: text.to_string()
        }),
    ]
}

/// Stable index per `GCodeCommand` variant for the non-vacuity coverage check.
fn command_variant_index(command: &GCodeCommand) -> usize {
    match command {
        GCodeCommand::Move { .. } => 0,
        GCodeCommand::Retract { .. } => 1,
        GCodeCommand::Unretract { .. } => 2,
        GCodeCommand::FanSpeed { .. } => 3,
        GCodeCommand::Temperature { .. } => 4,
        GCodeCommand::ToolChange { .. } => 5,
        GCodeCommand::Comment { .. } => 6,
        GCodeCommand::Raw { .. } => 7,
        GCodeCommand::ExtrusionMode { .. } => 8,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_placeholder_substitution() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST).expect("H-GCODE-TEMPLATE context");
    let limits = live_limits(&run);

    let strategy = (
        prop::sample::select(MALFORMED_MODES.to_vec()).no_shrink(),
        prop::collection::vec(any::<u16>(), 0..=limits.max_tokens),
        any::<[u8; 5]>(),
        prop::collection::vec(command_strategy(), 0..=limits.max_commands),
    )
        .prop_map(move |(mode, raws, kinds, commands)| {
            (build_case(mode, &raws, &kinds, &limits), commands)
        });

    let cases = Cell::new(0u64);
    let substitutions = Cell::new(0u64);
    let unresolved_cases = Cell::new(0u64);
    let nested_cases = Cell::new(0u64);
    let multiline_cases = Cell::new(0u64);
    let rescan_witness_cases = Cell::new(0u64);
    let passthrough_sequences = Cell::new(0u64);
    let passthrough_commands = Cell::new(0u64);
    let variant_coverage = Cell::new([0u64; 9]);

    run.check(&strategy, |(case, commands), _guard| {
        cases.set(cases.get() + 1);

        let observation = match check_template_case(&case, &TemplateModel::default()) {
            Ok(observation) => observation,
            Err(message) => return fail(format!("template case {case:?}: {message}")),
        };
        if observation.text != case.template {
            substitutions.set(substitutions.get() + 1);
        }
        if !observation.unresolved.is_empty() {
            unresolved_cases.set(unresolved_cases.get() + 1);
        }
        if case
            .tokens
            .iter()
            .any(|token| matches!(token, Token::Nested(_)))
        {
            nested_cases.set(nested_cases.get() + 1);
        }
        if case.template.contains('\n') {
            multiline_cases.set(multiline_cases.get() + 1);
        }
        let rescan_witness = case.tokens.iter().any(|token| match token {
            Token::Known(index) | Token::Nested(index) => *index as usize == RESCAN_KEY_INDEX,
            _ => false,
        });
        if rescan_witness {
            rescan_witness_cases.set(rescan_witness_cases.get() + 1);
        }

        if let Err(message) = check_command_passthrough(&commands, &CommandModel::default()) {
            return fail(format!("command sequence {commands:?}: {message}"));
        }
        if !commands.is_empty() {
            passthrough_sequences.set(passthrough_sequences.get() + 1);
            passthrough_commands.set(passthrough_commands.get() + commands.len() as u64);
        }
        let mut coverage = variant_coverage.get();
        for command in &commands {
            coverage[command_variant_index(command)] += 1;
        }
        variant_coverage.set(coverage);
        Ok(())
    })
    .expect("H-GCODE-TEMPLATE single-pass substitution and command pass-through");

    assert!(cases.get() > 0, "no template case was generated");
    assert!(
        substitutions.get() > 0,
        "no generated template substituted a value"
    );
    assert!(
        unresolved_cases.get() > 0,
        "no generated template reported an unresolved key"
    );
    assert!(
        nested_cases.get() > 0,
        "no generated template carried a nested-bracket witness"
    );
    assert!(
        multiline_cases.get() > 0,
        "no generated template carried a line separator"
    );
    assert!(
        rescan_witness_cases.get() > 0,
        "no generated template substituted a value that itself contains a placeholder"
    );
    assert!(
        passthrough_sequences.get() > 0,
        "no generated command sequence was driven through run_gcode_postprocess"
    );
    assert!(
        passthrough_commands.get() > 0,
        "run_gcode_postprocess saw no commands"
    );
    let coverage = variant_coverage.get();
    assert!(
        coverage.iter().all(|count| *count > 0),
        "every GCodeCommand variant must be driven at least once; coverage: {coverage:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls and shrink probe
// ─────────────────────────────────────────────────────────────────────────────

fn crafted(template: &str, entries: Vec<ValueEntry>) -> TemplateCase {
    TemplateCase {
        template: template.to_string(),
        tokens: Vec::new(),
        entries,
    }
}

#[test]
fn template_controls() {
    recursion_into_substituted_values_is_rejected();
    unordered_or_duplicated_unresolved_keys_are_rejected();
    empty_list_empty_string_substitute_is_rejected();
    command_order_swap_is_rejected();
    template_witness_probe();
}

fn recursion_into_substituted_values_is_rejected() {
    let case = crafted(
        "[speed]",
        vec![
            ValueEntry::new(
                "speed",
                ConfigValue::String("has [other] inside".to_string()),
                Some("has [other] inside"),
            ),
            ValueEntry::new("other", ConfigValue::String("X".to_string()), Some("X")),
        ],
    );
    let honest = check_template_case(&case, &TemplateModel::default())
        .expect("the engine does not re-scan a substituted value");
    assert_eq!(honest.text, "has [other] inside");

    let model = TemplateModel {
        rescan_substituted: true,
        ..Default::default()
    };
    let error =
        check_template_case(&case, &model).expect_err("a re-scanning oracle must be rejected");
    assert!(error.contains("rendered text"), "{error}");
    assert!(error.contains("[other]"), "{error}");
}

fn unordered_or_duplicated_unresolved_keys_are_rejected() {
    let case = crafted("[zed] [alpha]", Vec::new());
    let honest = check_template_case(&case, &TemplateModel::default())
        .expect("unresolved keys are sorted and deduplicated");
    assert_eq!(
        honest.unresolved,
        vec!["alpha".to_string(), "zed".to_string()]
    );

    let descending = TemplateModel {
        descending_unresolved: true,
        ..Default::default()
    };
    let error = check_template_case(&case, &descending)
        .expect_err("an unsorted unresolved list must be rejected");
    assert!(error.contains("unresolved keys"), "{error}");

    let duplicated = TemplateModel {
        duplicate_unresolved: true,
        ..Default::default()
    };
    let error = check_template_case(&case, &duplicated)
        .expect_err("a duplicated unresolved key must be rejected");
    assert!(error.contains("unresolved keys"), "{error}");
}

fn empty_list_empty_string_substitute_is_rejected() {
    let case = crafted(
        "M104 S[speed]",
        vec![ValueEntry::new(
            "speed",
            ConfigValue::List(Vec::new()),
            None,
        )],
    );
    let honest = check_template_case(&case, &TemplateModel::default())
        .expect("an empty list keeps its placeholder verbatim");
    assert_eq!(honest.text, "M104 S[speed]");
    assert_eq!(honest.unresolved, vec!["speed".to_string()]);

    let model = TemplateModel {
        empty_list_is_empty_string: true,
        ..Default::default()
    };
    let error = check_template_case(&case, &model)
        .expect_err("an empty-string substitute for an empty list must be rejected");
    assert!(error.contains("format_placeholder_value"), "{error}");
}

fn command_order_swap_is_rejected() {
    let commands = vec![
        GCodeCommand::FanSpeed { value: 1 },
        GCodeCommand::FanSpeed { value: 2 },
        GCodeCommand::Raw {
            text: "G28".to_string(),
        },
    ];
    let honest = check_command_passthrough(&commands, &CommandModel::default())
        .expect("command order and payload are preserved");
    assert_eq!(honest, commands);

    let model = CommandModel {
        swap_leading_pair: true,
    };
    let error = check_command_passthrough(&commands, &model)
        .expect_err("a swapped command order must be rejected");
    assert!(error.contains("command 0"), "{error}");
}

fn template_witness_probe() {
    let limits = control_limits();
    let kinds: [u8; 5] = [0, 3, 5, 9, 13];

    for mode in MALFORMED_MODES {
        let strategy =
            any::<[u16; 8]>().prop_map(move |raws| build_case(mode, &raws, &kinds, &limits));
        let mut runner = TestRunner::deterministic();
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("template witness tree");

        assert_eq!(
            tree.current().tokens.first(),
            Some(&mode.token()),
            "the forced {} token is missing from the witness",
            mode.label()
        );
        check_template_case(&tree.current(), &TemplateModel::default())
            .expect("the initial template witness checks");

        let initial = format!("{:?}", tree.current());
        let mut simplified = 0usize;
        let mut changed = false;
        while tree.simplify() {
            simplified += 1;
            let current = tree.current();
            assert_eq!(
                current.tokens.first(),
                Some(&mode.token()),
                "shrinking dropped the {} witness token",
                mode.label()
            );
            check_template_case(&current, &TemplateModel::default())
                .expect("a simplified template witness still checks");
            changed |= format!("{current:?}") != initial;
            assert!(
                tree.complicate(),
                "complicate must restore the last simplification"
            );
            check_template_case(&tree.current(), &TemplateModel::default())
                .expect("a restored template witness still checks");
        }
        assert!(
            simplified > 0,
            "false-only shrink probe for the {} witness",
            mode.label()
        );
        assert!(
            changed,
            "no allowed reduction changed the {} witness from its nonminimal start",
            mode.label()
        );
        println!(
            "mode={} simplify={simplified} changed={changed}",
            mode.label()
        );
    }

    // The malformed-mode selector itself must never shrink: a shrinking mode
    // tree could switch a witness's class mid-search.
    let mut runner = TestRunner::deterministic();
    let modes = prop::sample::select(MALFORMED_MODES.to_vec()).no_shrink();
    let mut tree = modes.new_tree(&mut runner).expect("malformed-mode tree");
    let before = format!("{:?}", tree.current());
    assert!(!tree.simplify(), "the malformed-mode selector simplified");
    assert!(
        !tree.complicate(),
        "the malformed-mode selector complicated"
    );
    assert_eq!(format!("{:?}", tree.current()), before);
}
