//! Strict preparation declarations, separate from ordinary IR permissions.

use std::collections::BTreeSet;

use slicer_schema::preparation::{PreparationDeclaration, PreparationDeclarationError};
use toml::Value;

use crate::LoadedModule;

const PREPARATION_INTERFACE: &str = "slicer:layer-preparation/prepare@1.0.0";

/// Parse only preparation metadata from the same text used for manifest ingestion.
///
/// Names are preserved exactly; this does not grant ordinary IR read permissions.
pub fn parse_preparation_declaration(
    text: &str,
) -> Result<Option<PreparationDeclaration>, PreparationDeclarationError> {
    use PreparationDeclarationError as Error;

    let root = text
        .parse::<Value>()
        .map_err(|error| Error::InvalidTable(error.to_string()))?;
    let Some(value) = root.get("preparation") else {
        return Ok(None);
    };
    let table = value
        .as_table()
        .ok_or_else(|| Error::InvalidTable("preparation must be a table".to_owned()))?;
    for key in table.keys() {
        if !matches!(key.as_str(), "interface" | "input_reads" | "views") {
            return Err(Error::UnknownKey(key.clone()));
        }
    }
    let interface = table
        .get("interface")
        .ok_or_else(|| Error::MissingInterface("preparation.interface".to_owned()))?;
    if interface.as_str() != Some(PREPARATION_INTERFACE) {
        return Err(Error::UnsupportedInterface(format!(
            "preparation.interface must equal {PREPARATION_INTERFACE}; got {interface:?}"
        )));
    }
    let input_reads = table
        .get("input_reads")
        .ok_or_else(|| Error::MissingInputReads("preparation.input_reads".to_owned()))?;
    let input_reads = parse_names(
        input_reads,
        "preparation.input_reads",
        Error::InvalidInputRead,
        Error::DuplicateInputRead,
    )?;
    let views = table
        .get("views")
        .map(|value| {
            parse_names(
                value,
                "preparation.views",
                Error::InvalidViewName,
                Error::DuplicateViewName,
            )
        })
        .transpose()?
        .unwrap_or_default();
    Ok(Some(PreparationDeclaration::new(
        PREPARATION_INTERFACE,
        input_reads,
        views,
    )))
}

fn parse_names(
    value: &Value,
    field: &str,
    invalid: fn(String) -> PreparationDeclarationError,
    duplicate: fn(String) -> PreparationDeclarationError,
) -> Result<Vec<String>, PreparationDeclarationError> {
    let entries = value
        .as_array()
        .ok_or_else(|| invalid(format!("{field} must be an array of strings")))?;
    let mut seen = BTreeSet::new();
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = entry
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| invalid(format!("{field} entries must be nonempty strings")))?;
        if !seen.insert(name) {
            return Err(duplicate(name.to_owned()));
        }
        names.push(name.to_owned());
    }
    Ok(names)
}

/// Immutable companion metadata without changing ordinary module ingestion.
#[derive(Debug)]
pub struct PreparationManifest {
    module: LoadedModule,
    preparation: Option<PreparationDeclaration>,
}

impl PreparationManifest {
    /// Store a loaded module and its separately validated declaration.
    pub fn new(module: LoadedModule, preparation: Option<PreparationDeclaration>) -> Self {
        Self {
            module,
            preparation,
        }
    }

    /// Return the original loaded module.
    #[must_use]
    pub fn module(&self) -> &LoadedModule {
        &self.module
    }

    /// Return the optional preparation declaration.
    #[must_use]
    pub fn preparation(&self) -> Option<&PreparationDeclaration> {
        self.preparation.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use slicer_ir::SemVer;

    use super::*;
    use crate::manifest::{ingest_manifest_text, ModuleProvenance};
    use crate::LoadedModuleBuilder;

    fn parse(fields: &str) -> Result<Option<PreparationDeclaration>, PreparationDeclarationError> {
        parse_preparation_declaration(&format!("[preparation]\n{fields}"))
    }

    fn fields(tail: &str) -> String {
        format!("interface = '{PREPARATION_INTERFACE}'\n{tail}")
    }

    #[test]
    fn naming_exception_is_scoped() {
        let declaration = parse(&fields("input_reads = ['mesh', '輪郭']\nviews = ['Plan']"))
            .expect("valid preparation table")
            .expect("declaration present");
        assert_eq!(declaration.interface(), PREPARATION_INTERFACE);
        assert_eq!(declaration.input_reads(), ["mesh", "輪郭"]);
        assert_eq!(declaration.views(), ["Plan"]);
        assert_eq!(
            parse(&fields("input-reads = ['mesh']")),
            Err(PreparationDeclarationError::UnknownKey(
                "input-reads".to_owned()
            ))
        );

        let legacy = r#"
[module]
id = "preparation-fixture"
version = "0.1.0"
[stage]
id = "PostPass::LayerFinalization"
[hints]
layer-parallel-safe = false
[ir-access]
reads = ["legacy.read"]
writes = []
[claims]
holds = []
requires = []
[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"
"#;
        let ingest = |text: &str| {
            ingest_manifest_text(
                text,
                Path::new("fixtures/preparation.toml"),
                Path::new("fixtures/preparation.wasm"),
                false,
                ModuleProvenance::External,
            )
            .map_err(Box::new)
        };
        let ingested = ingest(legacy).expect("legacy ir-access is accepted");
        assert_eq!(ingested.module.ir_reads(), ["legacy.read"]);
        let renamed = ingest(&legacy.replace("[ir-access]", "[ir_access]"))
            .expect_err("ir_access must not alias the required legacy table");
        assert_eq!(renamed.field.as_deref(), Some("ir-access.reads"));
    }

    #[test]
    fn absent_table_and_empty_inputs_are_legal() {
        assert_eq!(
            parse_preparation_declaration("[module]\nid = 'fixture'"),
            Ok(None)
        );
        let declaration = parse(&fields("input_reads = []"))
            .expect("empty input array is valid")
            .expect("table present");
        assert!(declaration.input_reads().is_empty());
        assert!(declaration.views().is_empty());
    }

    #[test]
    fn rejects_non_table_and_missing_fields() {
        assert!(matches!(
            parse_preparation_declaration("preparation = 'x'"),
            Err(PreparationDeclarationError::InvalidTable(_))
        ));
        assert!(matches!(
            parse("input_reads = []"),
            Err(PreparationDeclarationError::MissingInterface(_))
        ));
        assert!(matches!(
            parse(&fields("")),
            Err(PreparationDeclarationError::MissingInputReads(_))
        ));
    }

    #[test]
    fn rejects_unsupported_interfaces() {
        for interface in [
            "42",
            "'other'",
            "'slicer:layer-preparation'",
            "'slicer:layer-preparation/prepare'",
            "'slicer:layer-preparation/prepare@2.0.0'",
        ] {
            assert!(
                matches!(
                    parse(&format!("interface = {interface}\ninput_reads = []")),
                    Err(PreparationDeclarationError::UnsupportedInterface(_))
                ),
                "{interface}"
            );
        }
    }

    #[test]
    fn rejects_invalid_and_duplicate_inputs() {
        for inputs in ["'mesh'", "[42]", "['']"] {
            assert!(
                matches!(
                    parse(&fields(&format!("input_reads = {inputs}"))),
                    Err(PreparationDeclarationError::InvalidInputRead(_))
                ),
                "{inputs}"
            );
        }
        assert_eq!(
            parse(&fields("input_reads = ['mesh', 'mesh']")),
            Err(PreparationDeclarationError::DuplicateInputRead(
                "mesh".to_owned()
            ))
        );
    }

    #[test]
    fn rejects_invalid_and_duplicate_views() {
        for views in ["'plan'", "[42]", "['']"] {
            assert!(
                matches!(
                    parse(&fields(&format!("input_reads = []\nviews = {views}"))),
                    Err(PreparationDeclarationError::InvalidViewName(_))
                ),
                "{views}"
            );
        }
        assert_eq!(
            parse(&fields("input_reads = []\nviews = ['plan', 'plan']")),
            Err(PreparationDeclarationError::DuplicateViewName(
                "plan".to_owned()
            ))
        );
    }

    #[test]
    fn rejects_unknown_keys() {
        assert_eq!(
            parse(&fields("input_reads = []\nextra = true")),
            Err(PreparationDeclarationError::UnknownKey("extra".to_owned()))
        );
    }

    #[test]
    fn preserves_exact_names_without_projection_validation() {
        let declaration = parse(&fields("input_reads = [' ', 'Mesh', 'mesh', ' mesh ', '未知']\nviews = ['Plan', 'plan', ' plan ']"))
            .expect("only shape is validated")
            .expect("table present");
        assert_eq!(
            declaration.input_reads(),
            [" ", "Mesh", "mesh", " mesh ", "未知"]
        );
        assert_eq!(declaration.views(), ["Plan", "plan", " plan "]);
    }

    #[test]
    fn companion_round_trip() {
        let module = LoadedModuleBuilder::new(
            "fixture",
            SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            "PostPass::LayerFinalization",
            slicer_schema::TIER_LAYER,
            PathBuf::from("fixtures/fixture.wasm"),
        )
        .claims(Vec::new())
        .min_host_version(SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        })
        .min_ir_schema(SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        })
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .ir_reads(vec!["ordinary.read".to_owned()])
        .layer_parallel_safe(true)
        .build();
        let expected_module = format!("{module:?}");
        let declaration = PreparationDeclaration::new(
            PREPARATION_INTERFACE,
            vec!["prepared.input".to_owned()],
            vec!["plan".to_owned()],
        );
        let companion = PreparationManifest::new(module, Some(declaration.clone()));
        assert_eq!(format!("{:?}", companion.module()), expected_module);
        assert_eq!(companion.module().ir_reads(), ["ordinary.read"]);
        assert_eq!(companion.preparation(), Some(&declaration));
        let absent = PreparationManifest::new(companion.module.clone(), None);
        assert_eq!(absent.preparation(), None);
    }
}
