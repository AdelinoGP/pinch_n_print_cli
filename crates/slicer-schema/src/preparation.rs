//! Shared declaration and transport contracts for Layer preparation.

use std::fmt;

/// Immutable preparation metadata, preserving exact declared UTF-8 names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationDeclaration {
    interface: String,
    input_reads: Vec<String>,
    views: Vec<String>,
}

impl PreparationDeclaration {
    /// Construct metadata; declaration validation belongs to the manifest parser.
    pub fn new(interface: impl Into<String>, input_reads: Vec<String>, views: Vec<String>) -> Self {
        Self {
            interface: interface.into(),
            input_reads,
            views,
        }
    }

    /// Return the declared qualified interface, not a world or package name.
    #[must_use]
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Return the exact declared input read names.
    #[must_use]
    pub fn input_reads(&self) -> &[String] {
        &self.input_reads
    }

    /// Return the exact declared view names.
    #[must_use]
    pub fn views(&self) -> &[String] {
        &self.views
    }
}

/// Declaration failures with module, field, or expected/actual detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparationDeclarationError {
    /// A non-table preparation value.
    InvalidTable(String),
    /// An absent interface field.
    MissingInterface(String),
    /// Wrong interface type, value, qualification, or version.
    UnsupportedInterface(String),
    /// An absent input_reads field (even an empty array must be declared).
    MissingInputReads(String),
    /// Wrong input_reads array/entry type or an empty entry.
    InvalidInputRead(String),
    /// A repeated exact input_reads entry.
    DuplicateInputRead(String),
    /// Wrong views array/entry type or an empty name.
    InvalidViewName(String),
    /// A repeated exact view name.
    DuplicateViewName(String),
    /// Any other preparation key, including the input-reads alias.
    UnknownKey(String),
    /// A declared but absent capability export.
    MissingExport(String),
    /// Wrong compiled capability metadata, version, signature, or resource identity.
    IncompatibleExport(String),
    /// A capability export without a preparation declaration.
    UndeclaredExport(String),
    /// A capability paired with a non-Layer family or stage.
    UnsupportedStage(String),
}

impl fmt::Display for PreparationDeclarationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (reason, detail) = match self {
            Self::InvalidTable(detail) => ("invalid preparation table", detail),
            Self::MissingInterface(detail) => ("missing preparation interface", detail),
            Self::UnsupportedInterface(detail) => ("unsupported preparation interface", detail),
            Self::MissingInputReads(detail) => ("missing preparation input_reads", detail),
            Self::InvalidInputRead(detail) => ("invalid preparation input read", detail),
            Self::DuplicateInputRead(detail) => ("duplicate preparation input read", detail),
            Self::InvalidViewName(detail) => ("invalid preparation view name", detail),
            Self::DuplicateViewName(detail) => ("duplicate preparation view name", detail),
            Self::UnknownKey(detail) => ("unknown preparation key", detail),
            Self::MissingExport(detail) => ("missing preparation export", detail),
            Self::IncompatibleExport(detail) => ("incompatible preparation export", detail),
            Self::UndeclaredExport(detail) => ("undeclared preparation export", detail),
            Self::UnsupportedStage(detail) => ("unsupported preparation stage", detail),
        };
        write!(f, "{reason}: {detail}")
    }
}

impl std::error::Error for PreparationDeclarationError {}

/// Transport failures with a one-to-one canonical WIT variant mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationTransportError {
    /// Preparation has not published a readable plan.
    NotReady,
    /// A piece name is invalid.
    InvalidName,
    /// A piece name has already been written.
    DuplicateName,
    /// The output transaction is poisoned.
    PoisonedOutput,
    /// A requested byte range is invalid.
    RangeError,
    /// Range arithmetic overflowed.
    ArithmeticOverflow,
    /// Resource access is denied.
    AccessDenied,
}

impl PreparationTransportError {
    /// Return the canonical preparation-error WIT variant name.
    #[must_use]
    pub fn wit_variant(&self) -> &'static str {
        match self {
            Self::NotReady => "not-ready",
            Self::InvalidName => "invalid-name",
            Self::DuplicateName => "duplicate-name",
            Self::PoisonedOutput => "poisoned-output",
            Self::RangeError => "range-error",
            Self::ArithmeticOverflow => "arithmetic-overflow",
            Self::AccessDenied => "access-denied",
        }
    }
}

impl fmt::Display for PreparationTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.wit_variant())
    }
}

impl std::error::Error for PreparationTransportError {}

#[cfg(test)]
mod tests {
    use super::{PreparationDeclaration, PreparationTransportError};
    use std::collections::BTreeSet;

    #[test]
    fn transport_variants_match_canonical_wit() {
        let wit = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/wit/deps/layer-preparation/layer-preparation.wit"
        ))
        .expect("canonical layer-preparation WIT must be readable");
        let body = wit
            .split_once("enum preparation-error")
            .expect("canonical WIT must declare enum preparation-error")
            .1
            .trim_start()
            .strip_prefix('{')
            .expect("preparation-error enum must start with a brace")
            .split_once('}')
            .expect("preparation-error enum must have a closing brace")
            .0;
        let variants: Vec<_> = body
            .lines()
            .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
            .collect::<Vec<_>>()
            .join("\n")
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect();
        assert_eq!(variants.len(), 7, "canonical WIT variant count drifted");
        let canonical: BTreeSet<_> = variants.iter().map(String::as_str).collect();
        let errors = [
            PreparationTransportError::NotReady,
            PreparationTransportError::InvalidName,
            PreparationTransportError::DuplicateName,
            PreparationTransportError::PoisonedOutput,
            PreparationTransportError::RangeError,
            PreparationTransportError::ArithmeticOverflow,
            PreparationTransportError::AccessDenied,
        ];
        let mapped: BTreeSet<_> = errors.iter().map(|error| error.wit_variant()).collect();
        assert_eq!(mapped.len(), 7, "transport mapping must be one-to-one");
        assert_eq!(
            mapped, canonical,
            "transport mapping must match canonical WIT"
        );
    }

    #[test]
    fn declaration_accessors_return_constructor_input() {
        let interface = "slicer:layer-preparation/prepare@1.0.0";
        let input_reads = vec!["fixture".to_owned(), "exact.input".to_owned()];
        let views = vec!["輪郭-plan".to_owned(), "Case Sensitive".to_owned()];
        let declaration =
            PreparationDeclaration::new(interface, input_reads.clone(), views.clone());
        assert_eq!(declaration.interface(), interface);
        assert_eq!(declaration.input_reads(), input_reads.as_slice());
        assert_eq!(declaration.views(), views.as_slice());
        assert_ne!(declaration.interface(), "prepared-perimeters-module");
        assert_ne!(declaration.interface(), "slicer:layer-preparation@1.0.0");
    }
}
