//! The `openrscad`-backed implementation. No upstream type appears in a public
//! signature; everything below is private to this crate.
//!
//! # Ordering of work
//!
//! 1. options and limit validation, before anything is parsed or allocated;
//! 2. source-byte accounting;
//! 3. a joined worker with the caller's validated stack, running
//!    parse + parameter evaluation + program evaluation + render + conversion;
//! 4. external-resource denial, checked **before** any other outcome, because a
//!    resolver request means the caller's source left the sandbox regardless of
//!    what geometry survived;
//! 5. the single fail-closed diagnostic classifier;
//! 6. dimensional analysis and the render dispatch;
//! 7. checked conversion.
//!
//! A fatal parse/parameter/evaluation failure ends before render. Nothing in
//! this module can turn a rejected outcome into accepted geometry.

use std::ops::Range;
use std::sync::Mutex;

use openrscad_ir::Node;
use openrscad_syntax::ast::{Arg, Expr, ListElem, Param, Spanned, Stmt};

use crate::conversion::{convert_contours, convert_mesh, ConversionFailure, RawMesh};
use crate::diagnostics::{
    echo_diagnostic, error_diagnostic, make_diagnostic, primary_error_kind, Channel,
};
use crate::{
    ScadDiagnostic, ScadDiagnosticClass, ScadDiagnosticStage, ScadEngineInfo, ScadError,
    ScadErrorKind, ScadEvalOptions, ScadEvalOutput, ScadGeometry, ScadLimits, ScadProvenance,
    ScadSource,
};

/// Lowercase SHA-256 hex of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The immutable upstream revision the vendored engine is based on.
pub(crate) const ENGINE_REVISION: &str = "1a45bd33f2886c32bda32fe6e8358a67ab03f5ef";

/// Lowercase SHA-256 of the ordered applied patch set in
/// `vendor/openrscad/patches/`. Recomputed by
/// `tests/scad_feature_contract.rs` from those files, so an edit that forgets to
/// update this constant fails.
pub(crate) const ENGINE_PATCH_DIGEST: &str =
    "a9e67c4a4ab05153672581881480d70bbee11bf02c9642db79c2d446feec6e02";

/// The native backend selected for this target.
pub(crate) const ENGINE_BACKEND: &str = "manifold_cpp";

/// Identity of the compiled engine.
pub(crate) fn engine_info() -> ScadEngineInfo {
    ScadEngineInfo {
        revision: ENGINE_REVISION.to_string(),
        patch_digest: Some(ENGINE_PATCH_DIGEST.to_string()),
        backend: ENGINE_BACKEND.to_string(),
    }
}

/// A resolver that records every request and returns nothing.
///
/// This is the whole resource boundary. There is no caller-supplied resolver, no
/// working-directory base and no system-font registration: `source.label` is
/// diagnostic data, never a path.
struct DenyAllResolver {
    requests: Mutex<Vec<String>>,
}

impl DenyAllResolver {
    fn new() -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
        }
    }

    fn record(&self, op: &str, path: &str, from_dir: &str) {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(format!("{op}({path:?}) from {from_dir:?}"));
        }
    }

    fn take_requests(&self) -> Vec<String> {
        let mut requests = match self.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };
        std::mem::take(&mut requests)
    }
}

impl openrscad_eval::FileResolver for DenyAllResolver {
    fn load(&self, path: &str, from_dir: &str) -> Option<openrscad_eval::LoadedFile> {
        self.record("load", path, from_dir);
        None
    }

    fn load_bytes(&self, path: &str, from_dir: &str) -> Option<Vec<u8>> {
        self.record("load_bytes", path, from_dir);
        None
    }
}

/// Evaluate `source` with the pinned engine.
pub(crate) fn evaluate(
    source: &ScadSource,
    options: &ScadEvalOptions,
) -> Result<ScadEvalOutput, ScadError> {
    // Options come first: an invalid limit must not be able to reserve a thread
    // stack or reach the parser.
    if let Some(why) = options.first_violation() {
        return Err(ScadError::new(
            ScadErrorKind::InvalidOptions,
            &source.label,
            why,
        ));
    }

    let stack_bytes = options.limits.worker_stack_bytes;
    let source_owned = source.clone();
    let options_owned = options.clone();
    let label = source.label.clone();

    let handle = std::thread::Builder::new()
        .name("scad-eval".to_string())
        .stack_size(stack_bytes)
        .spawn(move || evaluate_on_worker(&source_owned, &options_owned))
        .map_err(|e| {
            ScadError::new(
                ScadErrorKind::WorkerFailure,
                &label,
                format!("could not start the evaluation worker: {e}"),
            )
        })?;

    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(ScadError::new(
            ScadErrorKind::WorkerFailure,
            &label,
            "the evaluation worker panicked; no geometry was returned",
        )),
    }
}

fn evaluate_on_worker(
    source: &ScadSource,
    options: &ScadEvalOptions,
) -> Result<ScadEvalOutput, ScadError> {
    let limits = &options.limits;
    let label = source.label.as_str();
    let mut gathered: Vec<ScadDiagnostic> = Vec::new();

    // --- 1. aggregate source byte budget -------------------------------------
    let mut bytes = source.text.len();
    for p in &options.parameters {
        bytes = bytes
            .saturating_add(p.name.len())
            .saturating_add(p.expression.len());
    }
    if bytes > limits.max_source_bytes {
        return Err(ScadError::new(
            ScadErrorKind::ComplexityLimit,
            label,
            format!(
                "source plus overrides is {bytes} bytes, above max_source_bytes ({})",
                limits.max_source_bytes
            ),
        ));
    }

    // --- 2. parse the program ------------------------------------------------
    let program = match openrscad_syntax::parse(&source.text) {
        Ok(program) => program,
        Err(error) => {
            let diagnostic = error_diagnostic(
                ScadDiagnosticStage::Parse,
                "parse_error",
                error.message.clone(),
                Some(error.span),
            );
            return Err(ScadError::new(ScadErrorKind::Parse, label, error.message)
                .with_diagnostics(vec![diagnostic]));
        }
    };

    // --- 3. AST bounds, across the program and every override expression -----
    let mut ast_meter = AstMeter::new(limits);
    if let Err(limit) = ast_meter.statements(&program, 1) {
        return Err(ScadError::new(ScadErrorKind::ComplexityLimit, label, limit));
    }

    // --- 4. parameters, under one shared allowance ---------------------------
    if let Some(seed) = options.random_seed {
        openrscad_eval::seed_random_state(seed);
    }

    let mut fuel = limits.evaluation_steps;
    let mut overrides: Vec<(String, openrscad_eval::Value)> = Vec::new();
    let resolver = DenyAllResolver::new();
    for parameter in &options.parameters {
        if !is_valid_scad_identifier(&parameter.name) {
            let diagnostic = error_diagnostic(
                ScadDiagnosticStage::Parameter,
                "invalid_parameter_name",
                format!(
                    "{:?} is not a SCAD identifier (case-sensitive, optional `$` prefix)",
                    parameter.name
                ),
                None,
            );
            gathered.push(diagnostic);
            return Err(ScadError::new(
                ScadErrorKind::InvalidParameter,
                label,
                format!("invalid override name {:?}", parameter.name),
            )
            .with_parameter(parameter.name.clone())
            .with_diagnostics(gathered));
        }

        // Parse the expression the adapter's way, so AST bounds, spans and
        // statement-injection rejection are all available before evaluation.
        match parse_parameter_expression(&parameter.expression, &mut ast_meter) {
            Ok(()) => {}
            Err(ParameterProblem::Limit(limit)) => {
                let diagnostic = error_diagnostic(
                    ScadDiagnosticStage::Parameter,
                    "parameter_complexity_limit",
                    limit.clone(),
                    None,
                );
                gathered.push(diagnostic);
                return Err(ScadError::new(ScadErrorKind::ComplexityLimit, label, limit)
                    .with_parameter(parameter.name.clone())
                    .with_diagnostics(gathered));
            }
            Err(ParameterProblem::Syntax { message, span }) => {
                let diagnostic = error_diagnostic(
                    ScadDiagnosticStage::Parameter,
                    "invalid_parameter_expression",
                    message.clone(),
                    span,
                );
                gathered.push(diagnostic);
                return Err(
                    ScadError::new(ScadErrorKind::InvalidParameter, label, message)
                        .with_parameter(parameter.name.clone())
                        .with_diagnostics(gathered),
                );
            }
        }

        let evaluation = openrscad_eval::eval_const_expr_with_budget_and_resolver(
            &parameter.expression,
            fuel,
            &resolver,
        );
        match &evaluation {
            Ok(outcome) => {
                for warning in &outcome.warnings {
                    gathered.push(make_diagnostic(
                        Channel::Evaluation,
                        ScadDiagnosticStage::Parameter,
                        warning.code,
                        warning.message.clone(),
                        warning.span.clone().and_then(|span| {
                            expression_span(
                                span,
                                "__orscad_const = (".len(),
                                parameter.expression.len(),
                            )
                        }),
                    ));
                }
                for echo in &outcome.echoes {
                    let mut diagnostic = echo_diagnostic(echo.clone());
                    diagnostic.stage = ScadDiagnosticStage::Parameter;
                    gathered.push(diagnostic);
                }
            }
            Err(error) => gathered.push(error_diagnostic(
                ScadDiagnosticStage::Parameter,
                if error.budget_exhausted {
                    "parameter_budget_exhausted"
                } else {
                    "invalid_parameter_expression"
                },
                error.message.clone(),
                error.span.clone().and_then(|span| {
                    expression_span(span, "__orscad_const = (".len(), parameter.expression.len())
                }),
            )),
        }
        let requests = resolver.take_requests();
        if !requests.is_empty() {
            return Err(external_resource_error(
                label,
                requests,
                ScadDiagnosticStage::Parameter,
                gathered,
            )
            .with_parameter(parameter.name.clone()));
        }
        match evaluation {
            Ok(outcome) => {
                fuel = outcome.remaining;
                if let Some(kind) = primary_error_kind(&gathered) {
                    return Err(ScadError::new(kind, label, primary_message(&gathered))
                        .with_parameter(parameter.name.clone())
                        .with_diagnostics(gathered));
                }
                if matches!(outcome.value, openrscad_eval::Value::Undef) {
                    let message = "an override expression evaluated to `undef`";
                    gathered.push(error_diagnostic(
                        ScadDiagnosticStage::Parameter,
                        "undefined_parameter",
                        message,
                        None,
                    ));
                    return Err(
                        ScadError::new(ScadErrorKind::InvalidParameter, label, message)
                            .with_parameter(parameter.name.clone())
                            .with_diagnostics(gathered),
                    );
                }
                overrides.retain(|(name, _)| name != &parameter.name);
                overrides.push((parameter.name.clone(), outcome.value));
            }
            Err(error) => {
                let kind = if error.budget_exhausted {
                    ScadErrorKind::EvaluationBudgetExceeded
                } else {
                    ScadErrorKind::InvalidParameter
                };
                return Err(ScadError::new(kind, label, error.message)
                    .with_parameter(parameter.name.clone())
                    .with_diagnostics(gathered));
            }
        }
    }

    // --- 5. evaluate the program under the remaining allowance --------------
    let evaluation = openrscad_eval::eval_program_with_budget_and_params(
        &program,
        &resolver,
        ".",
        &overrides,
        fuel,
        openrscad_eval::RenderMode::Exact,
    );
    let requests = resolver.take_requests();

    let output = match evaluation {
        Ok(output) => output,
        Err(error) => {
            let kind = if error.budget_exhausted {
                ScadErrorKind::EvaluationBudgetExceeded
            } else {
                ScadErrorKind::Evaluation
            };
            let diagnostic = error_diagnostic(
                ScadDiagnosticStage::Evaluation,
                if error.budget_exhausted {
                    "evaluation_budget_exhausted"
                } else {
                    "evaluation_error"
                },
                error.message.clone(),
                error.span.clone(),
            );
            gathered.push(diagnostic);
            // A request made before the failure is still the primary error.
            if !requests.is_empty() {
                return Err(external_resource_error(
                    label,
                    requests,
                    ScadDiagnosticStage::Evaluation,
                    gathered,
                ));
            }
            return Err(ScadError::new(kind, label, error.message).with_diagnostics(gathered));
        }
    };

    // External requests outrank every geometry outcome, including a non-empty
    // sibling primitive that would otherwise look successful.
    if !requests.is_empty() {
        return Err(external_resource_error(
            label,
            requests,
            ScadDiagnosticStage::Evaluation,
            gathered,
        ));
    }

    // --- 6. classify every evaluation diagnostic -----------------------------
    for warning in &output.warnings {
        gathered.push(make_diagnostic(
            Channel::Evaluation,
            ScadDiagnosticStage::Evaluation,
            warning.code,
            warning.message.clone(),
            warning.span.clone(),
        ));
    }
    for echo in &output.echoes {
        gathered.push(echo_diagnostic(echo.clone()));
    }
    if output.recursion_aborted && !gathered.iter().any(|d| d.code == "recursion_aborted") {
        gathered.push(make_diagnostic(
            Channel::Evaluation,
            ScadDiagnosticStage::Evaluation,
            "recursion_aborted",
            "a root-level statement was dropped after a recursion-depth abort; \
             the retained geometry does not describe the whole source"
                .to_string(),
            None,
        ));
    }
    if let Some(kind) = primary_error_kind(&gathered) {
        let message = primary_message(&gathered);
        return Err(ScadError::new(kind, label, message).with_diagnostics(gathered));
    }

    // --- 7. geometry-tree bound ----------------------------------------------
    let geometry_nodes = count_geometry_nodes(&output.node)
        .map_err(|failure| conversion_error(failure, label, &mut gathered))?;
    if geometry_nodes > limits.max_geometry_nodes {
        return Err(ScadError::new(
            ScadErrorKind::ComplexityLimit,
            label,
            format!(
                "evaluated CSG tree has {geometry_nodes} nodes, above max_geometry_nodes ({})",
                limits.max_geometry_nodes
            ),
        )
        .with_diagnostics(gathered));
    }

    // --- 7b. demanded facet bound, before tessellation -----------------------
    let demanded_facets = max_demanded_facets(&output.node);
    if demanded_facets > f64::from(limits.max_facets) {
        return Err(ScadError::new(
            ScadErrorKind::ComplexityLimit,
            label,
            format!(
                "the source requests {demanded_facets} facets, above max_facets ({})",
                limits.max_facets
            ),
        )
        .with_diagnostics(gathered));
    }

    // --- 8. dimensional analysis --------------------------------------------
    let dimension = dimension_of(&output.node);
    match dimension {
        Dimension::Mixed => {
            return Err(ScadError::new(
                ScadErrorKind::MixedDimensions,
                label,
                "the evaluated result mixes 2D and 3D geometry",
            )
            .with_diagnostics(gathered))
        }
        Dimension::Three if options.extrude_height_mm.is_some() => {
            return Err(ScadError::new(
                ScadErrorKind::ExtrusionHeightOn3d,
                label,
                "an explicit extrusion height was supplied for a 3D result",
            )
            .with_diagnostics(gathered))
        }
        _ => {}
    }

    // --- 9. render + convert -------------------------------------------------
    let geometry = match dimension {
        Dimension::Empty => ScadGeometry::Empty,
        Dimension::Two => match options.extrude_height_mm {
            None => render_contours(&output.node, limits, label, &mut gathered)?,
            Some(height) => {
                // Extrude exactly the evaluated 2D profile through the same engine
                // boundary, then treat the result as ordinary 3D geometry so the
                // extrusion's own diagnostics feed the same classifier.
                let extruded = Node::LinearExtrude {
                    height,
                    center: false,
                    twist: 0.0,
                    scale: [1.0, 1.0],
                    slices: None,
                    segments: 0,
                    frags: openrscad_ir::FragmentSpec::default(),
                    child: Box::new(output.node.clone()),
                };
                render_mesh(&extruded, limits, label, &mut gathered)?
            }
        },
        Dimension::Three => render_mesh(&output.node, limits, label, &mut gathered)?,
        Dimension::Mixed => unreachable!("mixed dimensions returned above"),
    };

    // --- 10. provenance ------------------------------------------------------
    let provenance = ScadProvenance {
        engine: engine_info(),
        source_label: source.label.clone(),
        source_digest: sha256_hex(source.text.as_bytes()),
        parameters: options.parameters.clone(),
        random_seed: options.random_seed,
    };

    Ok(ScadEvalOutput {
        geometry,
        diagnostics: gathered,
        provenance,
    })
}

fn external_resource_error(
    label: &str,
    requests: Vec<String>,
    stage: ScadDiagnosticStage,
    mut gathered: Vec<ScadDiagnostic>,
) -> ScadError {
    let message = format!(
        "the source requested external resources, which this adapter denies: {}",
        requests.join("; ")
    );
    let diagnostic = error_diagnostic(stage, "external_resource_denied", message.clone(), None);
    gathered.push(diagnostic);
    ScadError::new(ScadErrorKind::ExternalResourceDenied, label, message).with_diagnostics(gathered)
}

fn primary_message(diagnostics: &[ScadDiagnostic]) -> String {
    diagnostics
        .iter()
        .find(|d| d.class != ScadDiagnosticClass::Information)
        .map(|d| format!("{}: {}", d.code, d.message))
        .unwrap_or_else(|| "evaluation produced unacceptable diagnostics".to_string())
}

/// Render a purely 2D tree to checked contours.
fn render_contours(
    node: &Node,
    limits: &ScadLimits,
    label: &str,
    gathered: &mut Vec<ScadDiagnostic>,
) -> Result<ScadGeometry, ScadError> {
    let kernel = openrscad_geom::ManifoldKernel::new();
    let mut cache = openrscad_geom::GeomCache::new();
    let (contours, diagnostics) =
        match openrscad_geom::render_contours_cached_diag(node, &kernel, &mut cache) {
            Ok(result) => result,
            Err(error) => {
                return Err(ScadError::new(
                    ScadErrorKind::InvalidContours,
                    label,
                    format!("the contour render failed: {error}"),
                )
                .with_diagnostics(std::mem::take(gathered)))
            }
        };
    push_render_diagnostics(diagnostics, gathered, label)?;

    let Some(raw) = contours else {
        return Err(ScadError::new(
            ScadErrorKind::MixedDimensions,
            label,
            "the adapter classified this tree as 2D but the engine's contour dispatch did not",
        )
        .with_diagnostics(std::mem::take(gathered)));
    };

    let polygons = convert_contours(&raw, limits)
        .map_err(|failure| conversion_error(failure, label, gathered))?;
    if polygons.is_empty() {
        return Ok(ScadGeometry::Empty);
    }
    Ok(ScadGeometry::Contours(polygons))
}

/// Render a 3D tree (or the adapter's explicit extrusion wrapper) to a checked mesh.
fn render_mesh(
    node: &Node,
    limits: &ScadLimits,
    label: &str,
    gathered: &mut Vec<ScadDiagnostic>,
) -> Result<ScadGeometry, ScadError> {
    let kernel = openrscad_geom::ManifoldKernel::new();
    let mut cache = openrscad_geom::GeomCache::new();
    let (mesh, diagnostics) = match openrscad_geom::render_cached_diag(node, &kernel, &mut cache) {
        Ok(result) => result,
        Err(error) => {
            return Err(ScadError::new(
                ScadErrorKind::InvalidMesh,
                label,
                format!("the exact render failed: {error}"),
            )
            .with_diagnostics(std::mem::take(gathered)))
        }
    };
    push_render_diagnostics(diagnostics, gathered, label)?;

    if mesh.tris.is_empty() {
        return Ok(ScadGeometry::Empty);
    }

    let raw = RawMesh {
        vertices: mesh.verts.clone(),
        indices: mesh.tris.iter().flatten().copied().collect(),
    };
    let checked =
        convert_mesh(&raw, limits).map_err(|failure| conversion_error(failure, label, gathered))?;
    Ok(ScadGeometry::Mesh(checked))
}

fn push_render_diagnostics(
    diagnostics: openrscad_geom::RenderDiagnostics,
    gathered: &mut Vec<ScadDiagnostic>,
    label: &str,
) -> Result<(), ScadError> {
    for message in diagnostics.warnings {
        gathered.push(make_diagnostic(
            Channel::Render,
            ScadDiagnosticStage::Render,
            message.code,
            message.message,
            None,
        ));
    }
    for message in diagnostics.errors {
        gathered.push(make_diagnostic(
            Channel::Render,
            ScadDiagnosticStage::Render,
            message.code,
            message.message,
            None,
        ));
    }
    if let Some(kind) = primary_error_kind(gathered) {
        let message = primary_message(gathered);
        return Err(ScadError::new(kind, label, message).with_diagnostics(std::mem::take(gathered)));
    }
    Ok(())
}

fn conversion_error(
    failure: ConversionFailure,
    label: &str,
    gathered: &mut Vec<ScadDiagnostic>,
) -> ScadError {
    let diagnostic = error_diagnostic(
        ScadDiagnosticStage::Conversion,
        "conversion_rejected",
        failure.message.clone(),
        None,
    );
    let mut diagnostics = std::mem::take(gathered);
    diagnostics.push(diagnostic);
    ScadError::new(failure.kind, label, failure.message).with_diagnostics(diagnostics)
}

// ---------------------------------------------------------------------------
// Identifier validation
// ---------------------------------------------------------------------------

/// True when `name` is a case-sensitive SCAD identifier, optionally `$`-prefixed.
///
/// `$` specials are ordinary dynamic variables in SCAD (`$fn`, `$t`, `$preview`,
/// and user-defined `$` names are all legal), so they are validated by the same
/// grammar rather than normalised, stripped or rejected.
pub(crate) fn is_valid_scad_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let start_ok = |c: char| c == '_' || c.is_ascii_alphabetic();
    let rest_ok = |c: char| c == '_' || c.is_ascii_alphanumeric();

    if first == '$' {
        matches!(chars.next(), Some(c) if start_ok(c)) && chars.all(rest_ok)
    } else {
        start_ok(first) && chars.all(rest_ok)
    }
}

// ---------------------------------------------------------------------------
// AST measurement
// ---------------------------------------------------------------------------

enum ParameterProblem {
    Limit(String),
    Syntax {
        message: String,
        span: Option<Range<usize>>,
    },
}

fn expression_span(
    span: Range<usize>,
    prefix_len: usize,
    expression_len: usize,
) -> Option<Range<usize>> {
    let start = span.start.saturating_sub(prefix_len).min(expression_len);
    let end = span.end.saturating_sub(prefix_len).min(expression_len);
    (start < end).then_some(start..end)
}

fn parse_parameter_expression(
    expression: &str,
    meter: &mut AstMeter,
) -> Result<(), ParameterProblem> {
    // Same shape as the engine's own `__orscad_const = (<src>);` wrapper: the
    // check below is what makes "exactly one expression, no injected statements"
    // a parse-time property rather than a first-match-wins accident.
    const PREFIX: &str = "__scad_param = (";
    let wrapped = format!("{PREFIX}{expression});");
    let program = openrscad_syntax::parse(&wrapped).map_err(|error| ParameterProblem::Syntax {
        message: error.message,
        span: expression_span(error.span, PREFIX.len(), expression.len()),
    })?;

    let mut statements = program.iter();
    let single = match (statements.next(), statements.next()) {
        (Some(statement), None) => statement,
        _ => {
            return Err(ParameterProblem::Syntax {
                message: "a parameter expression must be exactly one expression and cannot \
                          inject statements"
                    .to_string(),
                span: expression_span(0..wrapped.len(), PREFIX.len(), expression.len()),
            })
        }
    };

    match &single.node {
        // Only the user's expression consumes AST capacity, not our synthetic
        // assignment. The same meter already contains the whole program.
        Stmt::Assign { name, value } if name == "__scad_param" => {
            meter.expr(value, 1).map_err(ParameterProblem::Limit)
        }
        _ => Err(ParameterProblem::Syntax {
            message: "a parameter expression must be exactly one expression and cannot inject \
                          statements"
                .to_string(),
            span: expression_span(single.span.clone(), PREFIX.len(), expression.len()),
        }),
    }
}

struct AstMeter {
    nodes: usize,
    max_nodes: usize,
    max_depth: usize,
}

impl AstMeter {
    fn new(limits: &ScadLimits) -> Self {
        Self {
            nodes: 0,
            max_nodes: limits.max_ast_nodes,
            max_depth: limits.max_ast_depth,
        }
    }

    fn enter(&mut self, depth: usize) -> Result<(), String> {
        if depth > self.max_depth {
            return Err(format!(
                "AST depth exceeds max_ast_depth ({})",
                self.max_depth
            ));
        }
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > self.max_nodes {
            return Err(format!(
                "AST nodes exceed max_ast_nodes ({})",
                self.max_nodes
            ));
        }
        Ok(())
    }

    fn statements(&mut self, statements: &[Spanned<Stmt>], depth: usize) -> Result<(), String> {
        for statement in statements {
            self.statement(statement, depth)?;
        }
        Ok(())
    }

    fn statement(&mut self, statement: &Spanned<Stmt>, depth: usize) -> Result<(), String> {
        self.enter(depth)?;
        match &statement.node {
            Stmt::Assign { value, .. } => self.expr(value, depth + 1),
            Stmt::ModuleCall { args, children, .. } => {
                self.args(args, depth + 1)?;
                self.statements(children, depth + 1)
            }
            Stmt::ModuleDef { params, body, .. } => {
                self.params(params, depth + 1)?;
                self.statements(body, depth + 1)
            }
            Stmt::FunctionDef { params, body, .. } => {
                self.params(params, depth + 1)?;
                self.expr(body, depth + 1)
            }
            Stmt::For { bindings, body } => {
                self.bindings(bindings, depth + 1)?;
                self.statements(body, depth + 1)
            }
            Stmt::If { cond, then, els } => {
                self.expr(cond, depth + 1)?;
                self.statements(then, depth + 1)?;
                self.statements(els, depth + 1)
            }
            Stmt::Let { bindings, body } => {
                self.bindings(bindings, depth + 1)?;
                self.statements(body, depth + 1)
            }
            Stmt::Block(body) => self.statements(body, depth + 1),
            Stmt::Include { .. } | Stmt::Use { .. } => Ok(()),
        }
    }

    fn bindings(&mut self, bindings: &[(String, Expr)], depth: usize) -> Result<(), String> {
        for (_, value) in bindings {
            self.expr(value, depth)?;
        }
        Ok(())
    }

    fn args(&mut self, args: &[Arg], depth: usize) -> Result<(), String> {
        for arg in args {
            self.expr(&arg.value, depth)?;
        }
        Ok(())
    }

    fn params(&mut self, params: &[Param], depth: usize) -> Result<(), String> {
        for param in params {
            if let Some(default) = &param.default {
                self.expr(default, depth)?;
            }
        }
        Ok(())
    }

    fn expr(&mut self, expr: &Expr, depth: usize) -> Result<(), String> {
        self.enter(depth)?;
        match expr {
            Expr::Number(_) | Expr::Bool(_) | Expr::Str(_) | Expr::Undef | Expr::Ident(_) => Ok(()),
            Expr::Vector(elements) => {
                for element in elements {
                    self.list_element(element, depth + 1)?;
                }
                Ok(())
            }
            Expr::Range { start, step, end } => {
                self.expr(start, depth + 1)?;
                if let Some(step) = step {
                    self.expr(step, depth + 1)?;
                }
                self.expr(end, depth + 1)
            }
            Expr::Unary { expr, .. } => self.expr(expr, depth + 1),
            Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs, depth + 1)?;
                self.expr(rhs, depth + 1)
            }
            Expr::Ternary { cond, then, els } => {
                self.expr(cond, depth + 1)?;
                self.expr(then, depth + 1)?;
                self.expr(els, depth + 1)
            }
            Expr::Index { base, index } => {
                self.expr(base, depth + 1)?;
                self.expr(index, depth + 1)
            }
            Expr::Member { base, .. } => self.expr(base, depth + 1),
            Expr::Call { args, .. } | Expr::CallValue { args, .. } => {
                if let Expr::CallValue { callee, .. } = expr {
                    self.expr(callee, depth + 1)?;
                }
                self.args(args, depth + 1)
            }
            Expr::Let { bindings, body } => {
                self.bindings(bindings, depth + 1)?;
                self.expr(body, depth + 1)
            }
            Expr::FunctionLiteral { params, body } => {
                self.params(params, depth + 1)?;
                self.expr(body, depth + 1)
            }
            Expr::Echo { args, body } | Expr::Assert { args, body } => {
                self.args(args, depth + 1)?;
                self.expr(body, depth + 1)
            }
        }
    }

    fn list_element(&mut self, element: &ListElem, depth: usize) -> Result<(), String> {
        self.enter(depth)?;
        match element {
            ListElem::Item(expr) => self.expr(expr, depth + 1),
            ListElem::Each(inner) => self.list_element(inner, depth + 1),
            ListElem::For { bindings, body } => {
                self.bindings(bindings, depth + 1)?;
                self.list_element(body, depth + 1)
            }
            ListElem::CFor {
                init,
                cond,
                update,
                body,
            } => {
                self.bindings(init, depth + 1)?;
                self.expr(cond, depth + 1)?;
                self.bindings(update, depth + 1)?;
                self.list_element(body, depth + 1)
            }
            ListElem::If { cond, then, els } => {
                self.expr(cond, depth + 1)?;
                self.list_element(then, depth + 1)?;
                if let Some(els) = els {
                    self.list_element(els, depth + 1)?;
                }
                Ok(())
            }
            ListElem::Let { bindings, body } => {
                self.bindings(bindings, depth + 1)?;
                self.list_element(body, depth + 1)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dimensional analysis
// ---------------------------------------------------------------------------

/// Evaluated dimensionality of a CSG subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dimension {
    /// No geometry-bearing node.
    Empty,
    /// Purely 2D.
    Two,
    /// Purely 3D.
    Three,
    /// Both 2D and 3D geometry in one result.
    Mixed,
}

/// Classify the evaluated tree's dimensionality.
///
/// This is deliberately *not* `openrscad_geom::is_2d`, which answers "does this
/// contain any 2D node" and so cannot see a mixed result. Dimension-changing
/// wrappers must have operands of the expected dimension; other wrappers are
/// transparent. `%` background geometry is excluded because the renderer excludes it.
pub(crate) fn dimension_of(node: &Node) -> Dimension {
    match node {
        Node::Empty => Dimension::Empty,
        Node::Group(children)
        | Node::Union(children)
        | Node::Difference(children)
        | Node::Intersection(children)
        | Node::Hull(children)
        | Node::Minkowski(children) => combine_dimensions(children),
        Node::Square { .. } | Node::Circle { .. } | Node::Polygon { .. } => Dimension::Two,
        // A dimension-changing wrapper whose operand is EMPTY produces an empty
        // result, not a mixed one: the evaluator preserves the wrapper around
        // `Node::Empty`, so `linear_extrude(1){}` and `projection(){}` must still
        // report `Empty` per the empty-result contract.
        Node::Projection { child, .. } => match dimension_of(child) {
            Dimension::Empty => Dimension::Empty,
            Dimension::Three => Dimension::Two,
            _ => Dimension::Mixed,
        },
        Node::Offset { child, .. } => match dimension_of(child) {
            Dimension::Empty => Dimension::Empty,
            Dimension::Three => Dimension::Mixed,
            other => other,
        },
        Node::LinearExtrude { child, .. }
        | Node::RotateExtrude { child, .. }
        | Node::Roof { child, .. } => match dimension_of(child) {
            Dimension::Empty => Dimension::Empty,
            Dimension::Two => Dimension::Three,
            _ => Dimension::Mixed,
        },
        Node::Cube { .. }
        | Node::Sphere { .. }
        | Node::Cylinder { .. }
        | Node::Polyhedron { .. } => Dimension::Three,
        Node::Import { format, .. } => {
            if format.eq_ignore_ascii_case("dxf") || format.eq_ignore_ascii_case("svg") {
                Dimension::Two
            } else {
                Dimension::Three
            }
        }
        Node::Translate { child, .. }
        | Node::Rotate { child, .. }
        | Node::Scale { child, .. }
        | Node::Mirror { child, .. }
        | Node::MultMatrix { child, .. }
        | Node::Resize { child, .. }
        | Node::Color { child, .. }
        | Node::Highlight(child)
        | Node::Provenance { child, .. } => dimension_of(child),
        // `%` background geometry is documented as excluded from the rendered and
        // exported mesh, so it contributes no dimensionality.
        Node::Background(_) => Dimension::Empty,
    }
}

fn combine_dimensions(children: &[Node]) -> Dimension {
    let mut accumulator = Dimension::Empty;
    for child in children {
        let dimension = dimension_of(child);
        match (accumulator, dimension) {
            (Dimension::Empty, next) => accumulator = next,
            (_, Dimension::Empty) => {}
            (seen, next) if seen == next => {}
            _ => return Dimension::Mixed,
        }
    }
    accumulator
}

/// Number of nodes in the evaluated CSG tree, for the geometry-node bound.
/// Reject invalid polyhedron faces here, before tessellation can skip them.
pub(crate) fn count_geometry_nodes(node: &Node) -> Result<usize, ConversionFailure> {
    match node {
        Node::Empty => Ok(1),
        Node::Group(children)
        | Node::Union(children)
        | Node::Difference(children)
        | Node::Intersection(children)
        | Node::Hull(children)
        | Node::Minkowski(children) => children.iter().try_fold(1_usize, |count, child| {
            Ok(count.saturating_add(count_geometry_nodes(child)?))
        }),
        Node::Polyhedron { points, faces } => {
            for (index, face) in faces.iter().enumerate() {
                if face.len() < 3 || face.iter().any(|&vertex| vertex as usize >= points.len()) {
                    return Err(ConversionFailure::new(
                        ScadErrorKind::InvalidMesh,
                        format!("polyhedron face {index} must have at least three vertices and only in-range point indices"),
                    ));
                }
            }
            Ok(1)
        }
        Node::Square { .. }
        | Node::Circle { .. }
        | Node::Polygon { .. }
        | Node::Cube { .. }
        | Node::Sphere { .. }
        | Node::Cylinder { .. }
        | Node::Import { .. } => Ok(1),
        Node::Offset { child, .. }
        | Node::Projection { child, .. }
        | Node::LinearExtrude { child, .. }
        | Node::RotateExtrude { child, .. }
        | Node::Roof { child, .. }
        | Node::Translate { child, .. }
        | Node::Rotate { child, .. }
        | Node::Scale { child, .. }
        | Node::Mirror { child, .. }
        | Node::MultMatrix { child, .. }
        | Node::Resize { child, .. }
        | Node::Color { child, .. }
        | Node::Highlight(child)
        | Node::Background(child)
        | Node::Provenance { child, .. } => Ok(count_geometry_nodes(child)?.saturating_add(1)),
    }
}

/// Largest demanded facet count found anywhere in the evaluated tree, or `0.0`.
///
/// `FragmentSpec::fn_` is the `$fn` value in effect at the primitive's call site
/// (`0` means "derive from `$fa`/`$fs`"). Circle, sphere and cylinder demands
/// include `openrscad_geom::fragments` with the same radius as tessellation.
/// Offset, extrusion and import nodes are bounded only by explicit `$fn` here:
/// their derived counts need the rendered profile. Output vertex/triangle
/// limits still apply to every node.
pub(crate) fn max_demanded_facets(node: &Node) -> f64 {
    let own = match node {
        Node::Sphere { r, frags } | Node::Circle { r, frags } => frags
            .fn_
            .max(f64::from(openrscad_geom::fragments(*r, *frags))),
        Node::Cylinder { r1, r2, frags, .. } => frags
            .fn_
            .max(f64::from(openrscad_geom::fragments(r1.max(*r2), *frags))),
        Node::LinearExtrude { frags, .. }
        | Node::RotateExtrude { frags, .. }
        | Node::Offset { frags, .. }
        | Node::Import { frags, .. } => frags.fn_,
        _ => 0.0,
    };
    let children: f64 = match node {
        Node::Group(children)
        | Node::Union(children)
        | Node::Difference(children)
        | Node::Intersection(children)
        | Node::Hull(children)
        | Node::Minkowski(children) => children.iter().map(max_demanded_facets).fold(0.0, f64::max),
        Node::Offset { child, .. }
        | Node::Projection { child, .. }
        | Node::LinearExtrude { child, .. }
        | Node::RotateExtrude { child, .. }
        | Node::Roof { child, .. }
        | Node::Translate { child, .. }
        | Node::Rotate { child, .. }
        | Node::Scale { child, .. }
        | Node::Mirror { child, .. }
        | Node::MultMatrix { child, .. }
        | Node::Resize { child, .. }
        | Node::Color { child, .. }
        | Node::Highlight(child)
        | Node::Background(child)
        | Node::Provenance { child, .. } => max_demanded_facets(child),
        _ => 0.0,
    };
    own.max(children)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scad_identifier_validation_is_case_sensitive_and_accepts_specials() {
        for name in ["x", "_x", "X1", "$fn", "$t", "$preview", "$_user"] {
            assert!(is_valid_scad_identifier(name), "{name} should be valid");
        }
        for name in ["", "1x", "a-b", "a.b", "$", "$$x", "x y", "ünicode", "a$b"] {
            assert!(!is_valid_scad_identifier(name), "{name} should be invalid");
        }
        // Case sensitivity is the point: these are different names.
        assert!(is_valid_scad_identifier("X"));
        assert!(is_valid_scad_identifier("x"));
        assert_ne!("X", "x");
    }

    #[test]
    fn scad_parameter_expression_rejects_injected_statements() {
        let mut meter = AstMeter::new(&ScadLimits::default());
        assert!(parse_parameter_expression("1 + 2 * [1, 2]", &mut meter).is_ok());
        assert!(parse_parameter_expression("1); cube(1); y = (2", &mut meter).is_err());
        assert!(parse_parameter_expression("", &mut meter).is_err());
    }

    #[test]
    fn scad_parameter_expression_respects_ast_depth() {
        // Parentheses are not AST nodes; real operators are.
        let mut deep = "1".to_string();
        for _ in 0..64 {
            deep = format!("({deep} + 1)");
        }
        let shallow = ScadLimits {
            max_ast_depth: 8,
            ..ScadLimits::default()
        };
        assert!(parse_parameter_expression(&deep, &mut AstMeter::new(&shallow)).is_err());
        assert!(
            parse_parameter_expression(&deep, &mut AstMeter::new(&ScadLimits::default())).is_ok()
        );
    }

    #[test]
    fn scad_dimensions_classify_wrappers_background_and_mixtures() {
        let square = Node::Square {
            size: [1.0, 1.0],
            center: false,
        };
        let cube = Node::Cube {
            size: [1.0, 1.0, 1.0],
            center: false,
        };
        assert_eq!(dimension_of(&square), Dimension::Two);
        assert_eq!(dimension_of(&cube), Dimension::Three);
        assert_eq!(dimension_of(&Node::Empty), Dimension::Empty);

        let translated_square = Node::Translate {
            v: [1.0, 2.0, 0.0],
            child: Box::new(square.clone()),
        };
        assert_eq!(dimension_of(&translated_square), Dimension::Two);

        // `%` background geometry is excluded from the render, so a result with
        // only background geometry is empty, not mixed.
        let background_cube = Node::Background(Box::new(cube.clone()));
        assert_eq!(dimension_of(&background_cube), Dimension::Empty);
        assert_eq!(
            dimension_of(&Node::Union(vec![square.clone(), background_cube.clone()])),
            Dimension::Two
        );

        assert_eq!(
            dimension_of(&Node::Union(vec![square.clone(), cube.clone()])),
            Dimension::Mixed
        );
        assert_eq!(
            dimension_of(&Node::Difference(vec![cube.clone(), cube.clone()])),
            Dimension::Three
        );

        // A dimension-changing wrapper around empty geometry is still empty: the
        // evaluator keeps the wrapper, and `linear_extrude(1){}` must not become a
        // mixed-dimension error.
        let empty_extrude = Node::LinearExtrude {
            height: 1.0,
            center: false,
            twist: 0.0,
            scale: [1.0, 1.0],
            slices: None,
            segments: 0,
            frags: openrscad_ir::FragmentSpec::default(),
            child: Box::new(Node::Empty),
        };
        assert_eq!(dimension_of(&empty_extrude), Dimension::Empty);
        assert_eq!(
            dimension_of(&Node::Projection {
                cut: false,
                child: Box::new(Node::Empty),
            }),
            Dimension::Empty
        );
        assert_eq!(
            dimension_of(&Node::Offset {
                r: 1.0,
                delta: 0.0,
                chamfer: false,
                frags: openrscad_ir::FragmentSpec::default(),
                child: Box::new(Node::Empty),
            }),
            Dimension::Empty
        );
    }

    #[test]
    fn scad_geometry_node_count_walks_the_whole_tree() {
        let leaf = Node::Cube {
            size: [1.0, 1.0, 1.0],
            center: false,
        };
        // The tree walk now also validates polyhedra; the counts are unchanged
        // but are returned only after successful validation.
        assert_eq!(
            count_geometry_nodes(&Node::Empty).expect("valid empty node"),
            1
        );
        assert_eq!(count_geometry_nodes(&leaf).expect("valid cube"), 1);
        assert_eq!(
            count_geometry_nodes(&Node::Union(vec![leaf.clone(), leaf.clone()]))
                .expect("valid union"),
            3
        );
    }
}
