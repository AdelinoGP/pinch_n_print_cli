//! Validated scripted strategies and truthfully reported generation faults.
//!
//! A value tree only ever exposes a **certified** case: a recipe that was
//! emitted, evaluated through the public adapter, and independently validated
//! against the requested domain and solid witness. Candidate rejections are
//! ordinary domain outcomes and are counted within finite limits; a fatal
//! adapter, artifact or accounting error is recorded as a generator fault so
//! the harness cannot report a passing run.

use std::sync::{Mutex, OnceLock};

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{Reason, TestRunner};
use slicer_scad::ScadEvalOptions;

use super::super::domains::{
    geometry_cases, GeometryCase, GeometryStrategy, GeometryValueTree, TopologyClass,
};
use super::reference::{certify_contours, certify_solid};
use super::{
    check_bounds, evaluate_recipe, invalid_bounds, recipe_complexity, Recipe2d, Recipe3d,
    RecipeComplexity, ScadRecipe, ScriptedCase, ScriptedDomain, ScriptedError, SolidWitness,
};

/// Bound on initial proposals before a strategy reports exhaustion.
const MAX_INITIAL_PROPOSALS: usize = 8;
/// Bound on recorded rejection reasons kept for one process.
const MAX_RECORDED_REASONS: usize = 64;

/// A generator fault that happened outside any property callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratorFault {
    /// Human-readable cause, including the concrete failing stage.
    pub(crate) message: String,
}

/// Per-process generator telemetry shared by a strategy, its trees and the
/// harness audit in [`super::run`].
#[derive(Debug, Default)]
pub(crate) struct GeneratorTelemetry {
    fault: Mutex<Option<GeneratorFault>>,
    reasons: Mutex<Vec<String>>,
    evaluations: Mutex<u64>,
    shrink_completed: Mutex<Option<bool>>,
}

thread_local! {
    /// The artifact directory of the `PropertyRun` on this thread, if any.
    ///
    /// The harness sets this when it loads a registry context, so attempt
    /// journaling never depends on a process-global environment variable that a
    /// sibling test could change.
    static ATTEMPT_ROOT: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

thread_local! {
    /// The generator version of the `PropertyRun` on this thread, if any.
    static GENERATOR_VERSION: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// Record the artifact directory and generator version attempt journals belong
/// under, or clear them. Both are thread-local so a sibling test's context can
/// never be observed here.
pub(crate) fn set_attempt_root(root: Option<std::path::PathBuf>, generator_version: Option<u64>) {
    ATTEMPT_ROOT.with(|slot| *slot.borrow_mut() = root);
    GENERATOR_VERSION.with(|slot| slot.set(generator_version));
}

/// The generator version recorded by the active `PropertyRun`, if any.
pub(crate) fn attempt_generator_version() -> Option<u64> {
    GENERATOR_VERSION.with(std::cell::Cell::get)
}

/// The attempt root for this thread, if a `PropertyRun` is active.
pub(crate) fn attempt_root() -> Option<std::path::PathBuf> {
    ATTEMPT_ROOT.with(|slot| slot.borrow().clone())
}

fn telemetry() -> &'static GeneratorTelemetry {
    static TELEMETRY: OnceLock<GeneratorTelemetry> = OnceLock::new();
    TELEMETRY.get_or_init(GeneratorTelemetry::default)
}

/// Record a fatal generator fault. The first fault wins, so the root cause is
/// not overwritten by a later consequence.
pub(crate) fn record_generator_fault(message: impl Into<String>) {
    let telemetry = telemetry();
    let mut fault = telemetry
        .fault
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if fault.is_none() {
        *fault = Some(GeneratorFault {
            message: message.into(),
        });
    }
}

/// Take and clear the recorded generator fault.
pub(crate) fn take_generator_fault() -> Option<GeneratorFault> {
    telemetry()
        .fault
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
}

/// Clear all per-process generator telemetry before a new run.
pub(crate) fn reset_generator_telemetry() {
    let telemetry = telemetry();
    *telemetry
        .fault
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    telemetry
        .reasons
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
    *telemetry
        .evaluations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = 0;
    *telemetry
        .shrink_completed
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

fn record_reason(reason: impl Into<String>) {
    let telemetry = telemetry();
    let mut reasons = telemetry
        .reasons
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if reasons.len() < MAX_RECORDED_REASONS {
        reasons.push(reason.into());
    }
}

fn record_evaluation() {
    let telemetry = telemetry();
    let mut evaluations = telemetry
        .evaluations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *evaluations += 1;
}

/// The declared shrink-completion state, once any tree established one.
pub(crate) fn last_shrink_completed() -> Option<bool> {
    *telemetry()
        .shrink_completed
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn set_shrink_completed(value: bool) {
    let telemetry = telemetry();
    let mut state = telemetry
        .shrink_completed
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // One limit hit is enough to keep the whole run incomplete.
    *state = Some(match *state {
        Some(false) => false,
        _ => value,
    });
}

/// The recorded rejection reasons, for attempt artifacts.
pub(crate) fn rejection_reasons() -> Vec<String> {
    telemetry()
        .reasons
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// A strategy whose values are independently certified scripted cases.
#[derive(Debug, Clone)]
pub struct ScriptedStrategy {
    domain: ScriptedDomain,
    options: ScadEvalOptions,
    source: Source,
}

#[derive(Debug, Clone)]
enum Source {
    Contours {
        geometry: GeometryStrategy,
        mode: BoxedStrategy<[u32; 6]>,
    },
    Solid {
        witness: SolidWitness,
        mode: BoxedStrategy<[u32; 5]>,
    },
}

/// A reversible, bounded exploration of simpler certified cases.
///
/// Exhausting the candidate or evaluation limit does not prove global
/// minimality, and never turns a known failure into a pass.
#[derive(Debug)]
pub struct ScriptedValueTree {
    domain: ScriptedDomain,
    options: ScadEvalOptions,
    recipe: ScadRecipe,
    case: ScriptedCase,
    complexity: RecipeComplexity,
    history: Vec<Certified>,
    /// Rejected `(base, candidate)` transitions. Keyed by the base so a
    /// rejection is never re-proposed from the same state, and a different state
    /// still gets its own chance.
    rejected: Vec<(ScadRecipe, ScadRecipe)>,
    geometry_tree: Option<GeometryValueTree>,
    /// `true` while the current recipe is the delegated geometry tree's value.
    geometry_current: bool,
    evaluations: u32,
    proposals: u32,
    /// A fatal candidate/adapter/artifact error was recorded; the search stops
    /// and must not claim completed minimization.
    fatal: bool,
}

#[derive(Debug)]
struct Certified {
    recipe: ScadRecipe,
    case: ScriptedCase,
    complexity: RecipeComplexity,
    geometry_current: bool,
}

fn contour_recipe(case: &GeometryCase) -> Recipe2d {
    if case.polygons.is_empty() {
        Recipe2d::Union(Vec::new())
    } else {
        Recipe2d::Polygon {
            polygons: case.polygons.clone(),
        }
    }
}

fn jitter(max_abs_units: i64, raw: u32, eighths: i64) -> i64 {
    let base = max_abs_units.saturating_mul(eighths) / 8;
    let span = (max_abs_units / 8).max(1);
    base + i64::from(raw) % span
}

fn units(value: i64) -> Option<u32> {
    u32::try_from(value.max(1)).ok()
}

/// Deterministic controlled CSG templates for one topology class.
fn csg_templates(class: TopologyClass, max_abs_units: i64, raw: &[u32; 6]) -> Option<Recipe2d> {
    let m = max_abs_units;
    match class {
        TopologyClass::Empty => Some(Recipe2d::Union(Vec::new())),
        TopologyClass::Convex => {
            let width = jitter(m, raw[0], 3);
            let height = jitter(m, raw[1], 3);
            let shifted = jitter(m, raw[2], 0).max(1);
            Some(Recipe2d::Intersection(vec![
                Recipe2d::Rectangle {
                    size_units: [units(width)?, units(height)?],
                },
                Recipe2d::Translate {
                    offset_units: [shifted, shifted],
                    child: Box::new(Recipe2d::Rectangle {
                        size_units: [units(width)?, units(height)?],
                    }),
                },
            ]))
        }
        TopologyClass::Concave => {
            let width = jitter(m, raw[0], 3);
            let height = jitter(m, raw[1], 3);
            let arm_width = (width / 4).max(2);
            let arm_height = (height / 4).max(2);
            Some(Recipe2d::Union(vec![
                Recipe2d::Rectangle {
                    size_units: [units(width)?, units(height)?],
                },
                Recipe2d::Translate {
                    offset_units: [width - arm_width / 2, 0],
                    child: Box::new(Recipe2d::Rectangle {
                        size_units: [units(arm_width)?, units(arm_height)?],
                    }),
                },
            ]))
        }
        TopologyClass::Holed => {
            let width = jitter(m, raw[0], 3);
            let height = jitter(m, raw[1], 3);
            let radius = (width.min(height) / 4).max(2);
            let facets = u16::try_from(8 + raw[2] % 8).ok()?;
            Some(Recipe2d::Difference {
                base: Box::new(Recipe2d::Rectangle {
                    size_units: [units(width)?, units(height)?],
                }),
                cuts: vec![Recipe2d::Translate {
                    // A `circle` is centred on its own origin, so the translate
                    // is the intended hole centre, not its lower corner.
                    offset_units: [width / 2, height / 2],
                    child: Box::new(Recipe2d::Disk {
                        radius_units: units(radius)?,
                        facets,
                    }),
                }],
            })
        }
        TopologyClass::Disconnected => {
            let width = jitter(m, raw[0], 3);
            let height = jitter(m, raw[1], 3);
            let gap = jitter(m, raw[2], 0).max(1);
            Some(Recipe2d::Union(vec![
                Recipe2d::Rectangle {
                    size_units: [units(width)?, units(height)?],
                },
                Recipe2d::Translate {
                    offset_units: [width + gap, 0],
                    child: Box::new(Recipe2d::Rectangle {
                        size_units: [units((width / 2).max(1))?, units((height / 2).max(1))?],
                    }),
                },
            ]))
        }
        TopologyClass::NonStarShaped => {
            // An H: two full-height arms joined by one middle bar. The gap
            // above and below the bar gives the shape an empty visibility
            // kernel (the left arm demands x <= arm, the right demands
            // x >= width - arm).
            let arm = (m / 8).max(2);
            let width = arm * 4;
            let height = arm * 4;
            let bar = (arm / 2).max(1);
            Some(Recipe2d::Union(vec![
                Recipe2d::Rectangle {
                    size_units: [units(arm)?, units(height)?],
                },
                Recipe2d::Translate {
                    offset_units: [width - arm, 0],
                    child: Box::new(Recipe2d::Rectangle {
                        size_units: [units(arm)?, units(height)?],
                    }),
                },
                Recipe2d::Translate {
                    offset_units: [0, (height - bar) / 2],
                    child: Box::new(Recipe2d::Rectangle {
                        size_units: [units(width)?, units(bar)?],
                    }),
                },
            ]))
        }
        TopologyClass::NearDegenerate => {
            // An explicit leaf with two independent near-degenerate features:
            // a one-unit neck between two non-adjacent walls, and an
            // almost-collinear vertex (a unit off the line through its
            // neighbours). Neither is a radial/star-shaped family property.
            let unit = (m / 16).max(64);
            let ring = slicer_ir::Polygon {
                points: vec![
                    [0, 0],
                    [5 * unit, 0],
                    [10 * unit, 1],
                    [10 * unit, 6 * unit],
                    [5 * unit + 1, 6 * unit],
                    [5 * unit + 1, 3 * unit],
                    [5 * unit, 3 * unit],
                    [5 * unit, 6 * unit],
                    [0, 6 * unit],
                ]
                .into_iter()
                .map(|[x, y]| slicer_ir::Point2 { x, y })
                .collect(),
            };
            Some(Recipe2d::Polygon {
                polygons: vec![slicer_ir::ExPolygon {
                    contour: ring,
                    holes: Vec::new(),
                }],
            })
        }
        TopologyClass::Thin => None,
    }
}

fn class_has_csg(class: TopologyClass) -> bool {
    class != TopologyClass::Thin
}

/// Deterministic solid templates. Every `SolidWitness` has at least two
/// construction variants so a run reaches primitives, transforms, extrusions,
/// intersections and Booleans rather than one shape per witness.
fn solid_recipe(witness: SolidWitness, max_abs_units: i64, raw: &[u32; 5]) -> Option<Recipe3d> {
    let m = max_abs_units;
    let variant = raw[0] % 3;
    match witness {
        SolidWitness::ClosedSolid => match variant {
            0 => Some(Recipe3d::Box {
                size_units: [
                    units(jitter(m, raw[1], 2))?,
                    units(jitter(m, raw[2], 2))?,
                    units(jitter(m, raw[3], 2))?,
                ],
            }),
            1 => Some(Recipe3d::Cylinder {
                radius_units: units(jitter(m, raw[1], 1))?,
                height_units: units(jitter(m, raw[2], 2))?,
                facets: u16::try_from(8 + raw[3] % 8).ok()?,
            }),
            _ => {
                let width = jitter(m, raw[1], 2);
                let depth = jitter(m, raw[2], 2);
                let height = jitter(m, raw[3], 1);
                let shifted = jitter(m, raw[4], 0).max(1);
                Some(Recipe3d::Intersection(vec![
                    Recipe3d::LinearExtrude {
                        height_units: units(height)?,
                        profile: Box::new(Recipe2d::Rectangle {
                            size_units: [units(width)?, units(depth)?],
                        }),
                    },
                    Recipe3d::Translate {
                        offset_units: [shifted, shifted, 0],
                        child: Box::new(Recipe3d::LinearExtrude {
                            height_units: units((height * 2).max(1))?,
                            profile: Box::new(Recipe2d::Rectangle {
                                size_units: [units(width)?, units(depth)?],
                            }),
                        }),
                    },
                ]))
            }
        },
        SolidWitness::Cavity => {
            let side = jitter(m, raw[1], 2);
            let inner = (side / 2).max(2);
            let inset = (side - inner) / 2;
            let cut = Recipe3d::Translate {
                offset_units: [inset, inset, inset],
                child: Box::new(Recipe3d::Box {
                    size_units: [units(inner)?, units(inner)?, units(inner)?],
                }),
            };
            let base = if variant == 1 {
                // An extruded planar base is an orthogonal cell too, so the
                // occupancy model still describes the converted solid.
                Recipe3d::LinearExtrude {
                    height_units: units(side)?,
                    profile: Box::new(Recipe2d::Rectangle {
                        size_units: [units(side)?, units(side)?],
                    }),
                }
            } else {
                Recipe3d::Box {
                    size_units: [units(side)?, units(side)?, units(side)?],
                }
            };
            let scene = Recipe3d::Difference {
                base: Box::new(base),
                cuts: vec![cut],
            };
            Some(if variant == 2 {
                Recipe3d::QuarterTurnZ {
                    turns: 1,
                    child: Box::new(scene),
                }
            } else {
                scene
            })
        }
        SolidWitness::Bridge => {
            let pillar = jitter(m, raw[1], 2);
            let gap = (m / 8).max(1);
            let height = jitter(m, raw[2], 2);
            let beam = (m / 8).max(1);
            let scene = Recipe3d::Union(vec![
                Recipe3d::Box {
                    size_units: [units(pillar)?, units(pillar)?, units(height)?],
                },
                Recipe3d::Translate {
                    offset_units: [pillar + gap, 0, 0],
                    child: Box::new(Recipe3d::Box {
                        size_units: [units(pillar)?, units(pillar)?, units(height)?],
                    }),
                },
                Recipe3d::Translate {
                    offset_units: [0, 0, height],
                    child: Box::new(Recipe3d::Box {
                        size_units: [
                            units(pillar + pillar + gap)?,
                            units(pillar)?,
                            units(beam)?,
                        ],
                    }),
                },
            ]);
            // A quarter turn would move the span onto the Y axis, which the
            // designated X-oriented span witness does not scan, so the bridge
            // keeps its orientation and varies its dimensions instead.
            Some(scene)
        }
        SolidWitness::Overhang => {
            let pillar = jitter(m, raw[1], 2);
            let height = jitter(m, raw[2], 2);
            let extension = (m / 8).max(1);
            let plate = (m / 16).max(1);
            Some(Recipe3d::Union(vec![
                Recipe3d::Box {
                    size_units: [units(pillar)?, units(pillar)?, units(height)?],
                },
                Recipe3d::Translate {
                    offset_units: [0, 0, height],
                    child: Box::new(Recipe3d::Box {
                        size_units: [
                            units(pillar + extension)?,
                            units(pillar)?,
                            units(plate)?,
                        ],
                    }),
                },
            ]))
        }
        SolidWitness::SeparatedBodies => {
            let first = jitter(m, raw[1], 2);
            let second = jitter(m, raw[2], 1).max(1);
            let gap = jitter(m, raw[3], 0).max(1);
            let height = jitter(m, raw[4], 1).max(1);
            let second_body = Recipe3d::Translate {
                offset_units: [first + gap, 0, 0],
                child: Box::new(Recipe3d::Box {
                    size_units: [units(second)?, units(second)?, units(height)?],
                }),
            };
            let scene = Recipe3d::Union(vec![
                Recipe3d::Box {
                    size_units: [units(first)?, units(first)?, units(height)?],
                },
                second_body,
            ]);
            Some(if variant == 2 {
                // A quarter turn keeps the bodies disjoint and moves the
                // separation onto the Y axis, which the slice-component check
                // still sees.
                Recipe3d::QuarterTurnZ {
                    turns: 1,
                    child: Box::new(scene),
                }
            } else {
                scene
            })
        }
    }
}

/// Build a certified strategy for one scripted domain.
///
/// # Errors
/// Returns [`ScriptedError::Domain`] for inconsistent bounds, geometry domains
/// or adapter capacities, so an impossible request is rejected before
/// generation rather than becoming an all-rejected population.
pub fn scripted_cases(
    domain: ScriptedDomain,
    options: ScadEvalOptions,
) -> Result<ScriptedStrategy, ScriptedError> {
    let recipe_bounds = domain.bounds;
    check_bounds(&recipe_bounds)?;
    validate_domain(&domain, &options)?;
    let source = match domain.solid_witness {
        None => {
            let geometry = geometry_cases(domain.geometry.clone())?;
            // The parameter range must be wide enough to exercise the jitter
            // spans, otherwise every generated dimension would land in its first
            // few units.
            let mode = (
                0_u32..4096,
                0_u32..4096,
                0_u32..4096,
                0_u32..4096,
                0_u32..4096,
                0_u32..4096,
            )
                .prop_map(|(a, b, c, d, e, f)| [a, b, c, d, e, f])
                .boxed();
            Source::Contours { geometry, mode }
        }
        Some(witness) => {
            let mode = (
                0_u32..4096,
                0_u32..4096,
                0_u32..4096,
                0_u32..4096,
                0_u32..4096,
            )
                .prop_map(|(a, b, c, d, e)| [a, b, c, d, e])
                .boxed();
            Source::Solid { witness, mode }
        }
    };
    Ok(ScriptedStrategy {
        domain,
        options,
        source,
    })
}

fn validate_domain(domain: &ScriptedDomain, options: &ScadEvalOptions) -> Result<(), ScriptedError> {
    // The constituent planar domain is validated for both branches, so an
    // impossible geometry capacity is rejected at construction rather than
    // becoming an all-rejected population.
    super::super::domains::geometry_cases(domain.geometry.clone())?;
    if domain.solid_witness.is_some() {
        // The bounded solid families are built from convex rectangular
        // profiles. A request for a different constituent planar topology would
        // be silently ignored, so it is rejected before sampling instead.
        if domain.geometry.class != super::super::domains::TopologyClass::Convex
            || domain.geometry.min_holes > 0
            || domain.geometry.min_components > 1
        {
            return Err(ScriptedError::Domain(
                super::super::domains::DomainError::UnsupportedDomain(
                    "solid families use convex rectangular constituent profiles; \
                     request TopologyClass::Convex with min_holes = 0 and min_components <= 1"
                        .into(),
                ),
            ));
        }
    }
    let capacity = super::required_adapter_capacity(&domain.bounds);
    if options.limits.max_ast_nodes < capacity.0 {
        return Err(invalid_bounds(format!(
            "adapter max_ast_nodes {} is below the {}-node emitter bound",
            options.limits.max_ast_nodes, capacity.0
        )));
    }
    if options.limits.max_ast_depth < capacity.1 {
        return Err(invalid_bounds(format!(
            "adapter max_ast_depth {} is below the {}-level emitter bound",
            options.limits.max_ast_depth, capacity.1
        )));
    }
    if options.limits.max_geometry_nodes < capacity.2 {
        return Err(invalid_bounds(format!(
            "adapter max_geometry_nodes {} is below the {}-node recipe bound",
            options.limits.max_geometry_nodes, capacity.2
        )));
    }
    if options.extrude_height_mm.is_some() {
        return Err(invalid_bounds(
            "a scripted domain owns its extrusion height; options.extrude_height_mm must be None",
        ));
    }
    if domain.geometry.bounds.max_abs_units < 16 {
        return Err(invalid_bounds(
            "the geometry coordinate bound must be at least 16 canonical units",
        ));
    }
    if domain.geometry.bounds.max_vertices == 0 {
        return Err(invalid_bounds("max_vertices must be positive"));
    }
    if domain.geometry.bounds.max_components == 0
        && domain.geometry.class != super::super::domains::TopologyClass::Empty
    {
        return Err(invalid_bounds(
            "max_components must be positive for a non-empty class",
        ));
    }
    if options.limits.evaluation_steps == 0
        || options.limits.max_source_bytes == 0
        || options.limits.max_ast_nodes == 0
        || options.limits.max_ast_depth == 0
        || options.limits.max_geometry_nodes == 0
        || options.limits.max_facets == 0
        || options.limits.max_vertices == 0
        || options.limits.max_triangles == 0
        || options.limits.worker_stack_bytes == 0
        || !options.limits.max_abs_coordinate_mm.is_finite()
        || options.limits.max_abs_coordinate_mm <= 0.0
    {
        return Err(invalid_bounds(
            "adapter limits must be finite and non-zero (0 is not 'unlimited')",
        ));
    }
    if options.limits.evaluation_steps == u64::MAX {
        return Err(invalid_bounds(
            "adapter evaluation_steps must be finite (u64::MAX is not 'unlimited')",
        ));
    }
    if !(slicer_scad::MIN_WORKER_STACK_BYTES..=slicer_scad::MAX_WORKER_STACK_BYTES)
        .contains(&options.limits.worker_stack_bytes)
    {
        return Err(invalid_bounds(
            "adapter worker_stack_bytes is outside the adapter's measured safe range",
        ));
    }
    if options.limits.max_facets < u32::from(domain.bounds.max_facets) {
        return Err(invalid_bounds(
            "adapter max_facets is below the recipe max_facets",
        ));
    }
    if options.limits.max_source_bytes < domain.bounds.max_source_bytes {
        return Err(invalid_bounds(
            "adapter max_source_bytes is below the recipe max_source_bytes",
        ));
    }
    let envelope_mm = domain.geometry.bounds.max_abs_units as f64 / 10_000.0;
    if options.limits.max_abs_coordinate_mm < envelope_mm {
        return Err(invalid_bounds(
            "adapter max_abs_coordinate_mm is below the domain's canonical-unit envelope",
        ));
    }
    Ok(())
}

thread_local! {
    /// Candidate evaluations on this thread, for the render-pause probe.
    static RENDER_PROBES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Pause on evaluation `N` of this thread when a test sets
/// `PNP_SCRIPTED_PAUSE_AFTER`. `PNP_SCRIPTED_PAUSE_BEFORE_RENDER` is the
/// equivalent of `N = 1`.
fn pause_at_render_probe() {
    let requested = std::env::var("PNP_SCRIPTED_PAUSE_AFTER")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| std::env::var_os("PNP_SCRIPTED_PAUSE_BEFORE_RENDER").map(|_| 1));
    let Some(requested) = requested else {
        return;
    };
    let current = RENDER_PROBES.with(|count| {
        let next = count.get() + 1;
        count.set(next);
        next
    });
    if current == requested {
        // test-quality: fault-injection seam for the supervised render-kill tests
        std::thread::sleep(std::time::Duration::from_secs(120));
    }
}

fn evaluate_certified(
    domain: &ScriptedDomain,
    options: &ScadEvalOptions,
    recipe: &ScadRecipe,
) -> Result<ScriptedCase, ScriptedError> {
    record_evaluation();
    let bounds = domain.bounds;
    // Persist the recipe/bounds/options/engine identity before emission and
    // before any adapter call, so an abort leaves a readable input set.
    let mut attempt = super::artifacts::prepare_attempt(recipe, &bounds, options)?;
    let source = match super::emit_scad(recipe, &bounds) {
        Ok(source) => source,
        Err(error) => {
            if let Some(attempt) = attempt {
                attempt.fail("failed", &format!("emission: {error}"))?;
            }
            return Err(error);
        }
    };
    if let Some(attempt) = attempt.as_mut() {
        attempt.record_source(&source)?;
    }
    // Fault-injection seam for the supervision contract: the exact window
    // between a complete, readable input set and the adapter call. No production
    // path sets these variables; they let a supervised child be killed while a
    // generation or a shrink render is pending, so the retained `prepared`
    // attempt can be inspected.
    pause_at_render_probe();
    let output = match evaluate_recipe(recipe, options, &bounds) {
        Ok(output) => output,
        Err(error) => {
            if let Some(attempt) = attempt {
                attempt.fail("failed", &error.to_string())?;
            }
            return Err(error);
        }
    };
    let certification = match domain.solid_witness {
        None => certify_contours(recipe, &output, &domain.geometry).map(|_| ()),
        Some(witness) => certify_solid(recipe, &output, witness).map(|_| ()),
    };
    match certification {
        Ok(()) => {
            if let Some(attempt) = attempt {
                attempt.finish(&output)?;
            }
        }
        Err(error) => {
            if let Some(attempt) = attempt {
                attempt.fail("rejected", &error.to_string())?;
            }
            return Err(error);
        }
    }
    Ok(ScriptedCase {
        recipe: recipe.clone(),
        recipe_bounds: bounds,
        source,
        options: options.clone(),
        output,
    })
}

/// Conversion-topology rejections of an otherwise valid recipe.
///
/// `InvalidContours`/`InvalidMesh`/range failures mean the emitted program's
/// *converted* geometry is unusable. That candidate is discarded and counted; it
/// is never exposed as a valid case. Parse, evaluation, budget, partial,
/// degraded, approximate, resource and unclassified outcomes stay fatal: those
/// describe the engine or the generator lying about a program the emitter
/// believes is well formed.
fn error_is_rejection(error: &ScriptedError) -> bool {
    // Only a genuine input rejection is expected: an invalid *bound* or an
    // arithmetic overflow is a harness/generator defect, not a discarded sample.
    matches!(
        error,
        ScriptedError::Domain(
            super::super::domains::DomainError::InvalidInput(_)
                | super::super::domains::DomainError::UnsupportedDomain(_)
        )
    ) || is_conversion_rejection(error)
}

fn is_conversion_rejection(error: &ScriptedError) -> bool {
    matches!(
        error,
        ScriptedError::Adapter(scad)
            if matches!(
                scad.kind,
                slicer_scad::ScadErrorKind::InvalidContours
                    | slicer_scad::ScadErrorKind::InvalidMesh
                    | slicer_scad::ScadErrorKind::CoordinateOutOfRange
                    | slicer_scad::ScadErrorKind::NonFiniteCoordinate
            )
    )
}

enum GeneratorFailure {
    Rejected(String),
}

struct Proposal {
    recipe: ScadRecipe,
    geometry_tree: Option<GeometryValueTree>,
    geometry_current: bool,
}

impl ScriptedStrategy {
    fn max_abs_units(&self) -> i64 {
        self.domain.geometry.bounds.max_abs_units
    }

    fn propose(&self, runner: &mut TestRunner) -> Result<Proposal, GeneratorFailure> {
        match &self.source {
            Source::Solid { witness, mode } => {
                let raw = mode
                    .new_tree(runner)
                    .map_err(|reason| GeneratorFailure::Rejected(reason.to_string()))?
                    .current();
                let recipe = solid_recipe(*witness, self.max_abs_units(), &raw).ok_or_else(|| {
                    GeneratorFailure::Rejected("solid template out of range".into())
                })?;
                Ok(Proposal {
                    recipe: ScadRecipe::Solid(recipe),
                    geometry_tree: None,
                    geometry_current: false,
                })
            }
            Source::Contours { geometry, mode } => {
                // Family selection is part of the generated value, not a
                // retry side effect: an always-valid explicit leaf must not
                // starve the CSG templates out of every tree.
                if class_has_csg(self.domain.geometry.class) {
                    let raw = mode
                        .new_tree(runner)
                        .map_err(|reason| GeneratorFailure::Rejected(reason.to_string()))?
                        .current();
                    if raw[0] % 3 != 0 {
                        let template =
                            csg_templates(self.domain.geometry.class, self.max_abs_units(), &raw)
                                .ok_or_else(|| {
                                    GeneratorFailure::Rejected(
                                        "no CSG template for this class".into(),
                                    )
                                })?;
                        return Ok(Proposal {
                            recipe: ScadRecipe::Contours(template),
                            geometry_tree: None,
                            geometry_current: false,
                        });
                    }
                }
                let tree = geometry
                    .new_tree(runner)
                    .map_err(|reason| GeneratorFailure::Rejected(reason.to_string()))?;
                let recipe = ScadRecipe::Contours(contour_recipe(&tree.current()));
                Ok(Proposal {
                    recipe,
                    geometry_tree: Some(tree),
                    geometry_current: true,
                })
            }
        }
    }
}

impl Strategy for ScriptedStrategy {
    type Tree = ScriptedValueTree;
    type Value = ScriptedCase;

    fn new_tree(&self, runner: &mut TestRunner) -> Result<Self::Tree, Reason> {
        let budget = self.domain.bounds.max_evaluations;
        let mut reasons = Vec::new();
        let mut used = 0_u32;
        for _attempt in 0..MAX_INITIAL_PROPOSALS {
            if used >= budget {
                set_shrink_completed(false);
                break;
            }
            let proposal = match self.propose(runner) {
                Ok(proposal) => proposal,
                Err(GeneratorFailure::Rejected(reason)) => {
                    record_reason(reason.clone());
                    reasons.push(reason);
                    continue;
                }
            };
            used += 1;
            match evaluate_certified(&self.domain, &self.options, &proposal.recipe) {
                Ok(case) => {
                    let complexity = recipe_complexity(&proposal.recipe).map_err(|error| {
                        record_generator_fault(error.to_string());
                        Reason::from(error.to_string())
                    })?;
                    return Ok(ScriptedValueTree {
                        domain: self.domain.clone(),
                        options: self.options.clone(),
                        recipe: proposal.recipe,
                        case,
                        complexity,
                        history: Vec::new(),
                        rejected: Vec::new(),
                        geometry_tree: proposal.geometry_tree,
                        geometry_current: proposal.geometry_current,
                        evaluations: used,
                        proposals: 0,
                        fatal: false,
                    });
                }
                Err(error) if error_is_rejection(&error) => {
                    let message = format!("candidate rejected: {error}");
                    record_reason(message.clone());
                    reasons.push(message);
                }
                Err(error) => {
                    let message = format!("scripted generation failed: {error}");
                    record_generator_fault(message.clone());
                    return Err(Reason::from(message));
                }
            }
        }
        let message = format!(
            "Too many rejected scripted candidates after {MAX_INITIAL_PROPOSALS} proposals: {}",
            reasons.join("; ")
        );
        Err(Reason::from(message))
    }
}

impl ScriptedValueTree {
    fn certified(&self) -> Certified {
        Certified {
            recipe: self.recipe.clone(),
            case: self.case.clone(),
            complexity: self.complexity,
            geometry_current: self.geometry_current,
        }
    }

    /// Accept a candidate if it strictly decreases [`RecipeComplexity`] and
    /// still certifies. `None` means the evaluation budget is exhausted.
    fn consider(&mut self, candidate: ScadRecipe, geometry_source: bool) -> Option<bool> {
        if self.proposals >= self.domain.geometry.bounds.max_shrink_candidates {
            set_shrink_completed(false);
            return None;
        }
        self.proposals += 1;
        if self.evaluations >= self.domain.bounds.max_evaluations {
            set_shrink_completed(false);
            return None;
        }
        if self
            .rejected
            .iter()
            .any(|(base, rejected)| *base == self.recipe && *rejected == candidate)
        {
            return Some(false);
        }
        let complexity = match recipe_complexity(&candidate) {
            Ok(complexity) => complexity,
            Err(error) => {
                record_generator_fault(error.to_string());
                return Some(false);
            }
        };
        if complexity >= self.complexity {
            return Some(false);
        }
        self.evaluations += 1;
        match evaluate_certified(&self.domain, &self.options, &candidate) {
            Ok(case) => {
                self.history.push(self.certified());
                self.recipe = candidate;
                self.case = case;
                self.complexity = complexity;
                self.geometry_current = geometry_source;
                Some(true)
            }
            Err(error) if error_is_rejection(&error) => {
                // A domain-invalid or conversion-collapsed candidate is an
                // ordinary rejected proposal; it is never exposed.
                self.rejected.push((self.recipe.clone(), candidate));
                record_reason("candidate rejected by the domain, witness or conversion check");
                Some(false)
            }
            Err(error) => {
                // Adapter, artifact and accounting failures are generator
                // failures, never ordinary discarded samples.
                let message = format!("scripted candidate failed: {error}");
                record_generator_fault(message);
                self.rejected.push((self.recipe.clone(), candidate));
                set_shrink_completed(false);
                self.fatal = true;
                None
            }
        }
    }
}

/// Candidate simplifications of a solid recipe, structure first.
fn solid_candidates(recipe: &Recipe3d) -> Vec<Recipe3d> {
    solid_candidates_inner(recipe, true)
}

/// `allow_translation` is false for a Boolean cut: moving a cut can change whether
/// it stays inside the base, which is a *generator* failure (the adapter rejects
/// the emitted valid recipe) rather than an expected domain rejection.
fn solid_candidates_inner(recipe: &Recipe3d, allow_translation: bool) -> Vec<Recipe3d> {
    let mut out = Vec::new();
    match recipe {
        Recipe3d::Box { size_units } => {
            for axis in 0..3 {
                let mut size = *size_units;
                if size[axis] > 1 {
                    size[axis] = (size[axis] / 2).max(1);
                    out.push(Recipe3d::Box { size_units: size });
                }
            }
        }
        Recipe3d::Cylinder {
            radius_units,
            height_units,
            facets,
        } => {
            if *facets > 3 {
                out.push(Recipe3d::Cylinder {
                    radius_units: *radius_units,
                    height_units: *height_units,
                    facets: (*facets / 2).max(3),
                });
            }
            if *radius_units > 1 {
                out.push(Recipe3d::Cylinder {
                    radius_units: (*radius_units / 2).max(1),
                    height_units: *height_units,
                    facets: *facets,
                });
            }
            if *height_units > 1 {
                out.push(Recipe3d::Cylinder {
                    radius_units: *radius_units,
                    height_units: (*height_units / 2).max(1),
                    facets: *facets,
                });
            }
        }
        Recipe3d::LinearExtrude {
            height_units,
            profile,
        } => {
            if *height_units > 1 {
                out.push(Recipe3d::LinearExtrude {
                    height_units: (*height_units / 2).max(1),
                    profile: profile.clone(),
                });
            }
            for candidate in two_d_candidates_inner(profile, allow_translation) {
                out.push(Recipe3d::LinearExtrude {
                    height_units: *height_units,
                    profile: Box::new(candidate),
                });
            }
        }
        Recipe3d::Translate {
            offset_units,
            child,
        } => {
            if allow_translation {
                out.push((**child).clone());
            }
            if allow_translation && offset_units.iter().any(|offset| *offset != 0) {
                out.push(Recipe3d::Translate {
                    offset_units: offset_units.map(|offset| offset / 2),
                    child: child.clone(),
                });
            }
            for candidate in solid_candidates_inner(child, allow_translation) {
                out.push(Recipe3d::Translate {
                    offset_units: *offset_units,
                    child: Box::new(candidate),
                });
            }
        }
        Recipe3d::QuarterTurnZ { turns, child } => {
            out.push((**child).clone());
            if *turns != 0 {
                out.push(Recipe3d::QuarterTurnZ {
                    turns: 0,
                    child: Box::new((**child).clone()),
                });
            }
        }
        Recipe3d::Union(children) => {
            if children.len() == 1 {
                out.push(children[0].clone());
            } else if !children.is_empty() {
                let mut shorter = children.clone();
                shorter.pop();
                out.push(Recipe3d::Union(shorter));
            }
            for (index, child) in children.iter().enumerate() {
                for candidate in solid_candidates(child) {
                    let mut replaced = children.clone();
                    replaced[index] = candidate;
                    out.push(Recipe3d::Union(replaced));
                }
            }
        }
        Recipe3d::Difference { base, cuts } => {
            if cuts.is_empty() {
                out.push((**base).clone());
            } else {
                let mut shorter = cuts.clone();
                shorter.pop();
                out.push(Recipe3d::Difference {
                    base: base.clone(),
                    cuts: shorter,
                });
            }
            for (index, cut) in cuts.iter().enumerate() {
                for candidate in solid_candidates_inner(cut, false) {
                    let mut replaced = cuts.clone();
                    replaced[index] = candidate;
                    out.push(Recipe3d::Difference {
                        base: base.clone(),
                        cuts: replaced,
                    });
                }
            }
            for candidate in solid_candidates(base) {
                out.push(Recipe3d::Difference {
                    base: Box::new(candidate),
                    cuts: cuts.clone(),
                });
            }
        }
        Recipe3d::Intersection(children) => {
            if children.len() == 1 {
                out.push(children[0].clone());
            } else if !children.is_empty() {
                let mut shorter = children.clone();
                shorter.pop();
                out.push(Recipe3d::Intersection(shorter));
            }
            for (index, child) in children.iter().enumerate() {
                for candidate in solid_candidates(child) {
                    let mut replaced = children.clone();
                    replaced[index] = candidate;
                    out.push(Recipe3d::Intersection(replaced));
                }
            }
        }
    }
    out
}

/// Candidate simplifications of a planar recipe.
fn two_d_candidates(recipe: &Recipe2d) -> Vec<Recipe2d> {
    two_d_candidates_inner(recipe, true)
}

/// `allow_translation` is false for a Boolean cut; see `solid_candidates_inner`.
fn two_d_candidates_inner(recipe: &Recipe2d, allow_translation: bool) -> Vec<Recipe2d> {
    let mut out = Vec::new();
    match recipe {
        Recipe2d::Rectangle { size_units } => {
            for axis in 0..2 {
                let mut size = *size_units;
                if size[axis] > 1 {
                    size[axis] = (size[axis] / 2).max(1);
                    out.push(Recipe2d::Rectangle { size_units: size });
                }
            }
        }
        Recipe2d::Disk {
            radius_units,
            facets,
        } => {
            if *facets > 3 {
                out.push(Recipe2d::Disk {
                    radius_units: *radius_units,
                    facets: (*facets / 2).max(3),
                });
            }
            if *radius_units > 1 {
                out.push(Recipe2d::Disk {
                    radius_units: (*radius_units / 2).max(1),
                    facets: *facets,
                });
            }
        }
        Recipe2d::Polygon { polygons } => {
            // Optional vertices are mostly removed by the delegated geometry
            // tree; this candidate keeps the leaf legal by only dropping a
            // trailing vertex of a component that still validates.
            for component in (0..polygons.len()).rev() {
                let mut candidate = polygons.clone();
                let ring = &mut candidate[component].contour.points;
                if ring.len() > 3 {
                    ring.pop();
                    out.push(Recipe2d::Polygon {
                        polygons: candidate,
                    });
                }
            }
        }
        Recipe2d::Translate {
            offset_units,
            child,
        } => {
            if allow_translation {
                out.push((**child).clone());
            }
            if allow_translation && offset_units.iter().any(|offset| *offset != 0) {
                out.push(Recipe2d::Translate {
                    offset_units: offset_units.map(|offset| offset / 2),
                    child: child.clone(),
                });
            }
            for candidate in two_d_candidates_inner(child, allow_translation) {
                out.push(Recipe2d::Translate {
                    offset_units: *offset_units,
                    child: Box::new(candidate),
                });
            }
        }
        Recipe2d::QuarterTurn { turns, child } => {
            out.push((**child).clone());
            if *turns != 0 {
                out.push(Recipe2d::QuarterTurn {
                    turns: 0,
                    child: Box::new((**child).clone()),
                });
            }
        }
        Recipe2d::Union(children) => {
            if children.len() == 1 {
                out.push(children[0].clone());
            } else if !children.is_empty() {
                let mut shorter = children.clone();
                shorter.pop();
                out.push(Recipe2d::Union(shorter));
            }
            for (index, child) in children.iter().enumerate() {
                for candidate in two_d_candidates(child) {
                    let mut replaced = children.clone();
                    replaced[index] = candidate;
                    out.push(Recipe2d::Union(replaced));
                }
            }
        }
        Recipe2d::Difference { base, cuts } => {
            if cuts.is_empty() {
                out.push((**base).clone());
            } else {
                let mut shorter = cuts.clone();
                shorter.pop();
                out.push(Recipe2d::Difference {
                    base: base.clone(),
                    cuts: shorter,
                });
            }
            for (index, cut) in cuts.iter().enumerate() {
                for candidate in two_d_candidates_inner(cut, false) {
                    let mut replaced = cuts.clone();
                    replaced[index] = candidate;
                    out.push(Recipe2d::Difference {
                        base: base.clone(),
                        cuts: replaced,
                    });
                }
            }
            for candidate in two_d_candidates(base) {
                out.push(Recipe2d::Difference {
                    base: Box::new(candidate),
                    cuts: cuts.clone(),
                });
            }
        }
        Recipe2d::Intersection(children) => {
            if children.len() == 1 {
                out.push(children[0].clone());
            } else if !children.is_empty() {
                let mut shorter = children.clone();
                shorter.pop();
                out.push(Recipe2d::Intersection(shorter));
            }
            for (index, child) in children.iter().enumerate() {
                for candidate in two_d_candidates(child) {
                    let mut replaced = children.clone();
                    replaced[index] = candidate;
                    out.push(Recipe2d::Intersection(replaced));
                }
            }
        }
    }
    out
}

impl ValueTree for ScriptedValueTree {
    type Value = ScriptedCase;

    fn current(&self) -> ScriptedCase {
        // Cached: no evaluation and no I/O on this path.
        self.case.clone()
    }

    fn simplify(&mut self) -> bool {
        if self.geometry_current {
            if let Some(mut tree) = self.geometry_tree.take() {
                let mut accepted = false;
                let mut exhausted_budget = false;
                while tree.simplify() {
                    let candidate = ScadRecipe::Contours(contour_recipe(&tree.current()));
                    match self.consider(candidate, true) {
                        Some(true) => {
                            accepted = true;
                            break;
                        }
                        Some(false) => {
                            // Keep the delegated tree in step with the certified
                            // value rather than leaking an unaccepted state.
                            tree.complicate();
                        }
                        None => {
                            tree.complicate();
                            exhausted_budget = true;
                            break;
                        }
                    }
                }
                self.geometry_tree = Some(tree);
                if accepted {
                    return true;
                }
                if exhausted_budget || self.fatal {
                    return false;
                }
            }
        }
        let candidates = match &self.recipe {
            ScadRecipe::Contours(recipe) => two_d_candidates(recipe)
                .into_iter()
                .map(ScadRecipe::Contours)
                .collect::<Vec<_>>(),
            ScadRecipe::Solid(recipe) => solid_candidates(recipe)
                .into_iter()
                .map(ScadRecipe::Solid)
                .collect::<Vec<_>>(),
        };
        for candidate in candidates {
            match self.consider(candidate, false) {
                Some(true) => return true,
                Some(false) => continue,
                None => return false,
            }
        }
        if self.fatal {
            return false;
        }
        set_shrink_completed(true);
        false
    }

    fn complicate(&mut self) -> bool {
        let Some(previous) = self.history.pop() else {
            return false;
        };
        if self.geometry_current {
            if let Some(tree) = self.geometry_tree.as_mut() {
                if !tree.complicate() {
                    self.history.push(previous);
                    return false;
                }
            }
        }
        // The transition we are undoing passed the property (Proptest asked to
        // complicate back), so it must never be proposed again from this base:
        // re-proposing it would loop until the shrink iteration budget ran out.
        self.rejected
            .push((previous.recipe.clone(), self.recipe.clone()));
        self.recipe = previous.recipe;
        self.case = previous.case;
        self.complexity = previous.complexity;
        self.geometry_current = previous.geometry_current;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::property_testing::domains::{BoundaryRule, GeometryBounds, GeometryDomain};
    use proptest::test_runner::Config;
    use crate::test_support::property_testing::scripted::RecipeBounds;

    fn fixture_domain() -> ScriptedDomain {
        // exhaustive: pins the fatal-shrink unit fixture deliberately
        ScriptedDomain {
            // exhaustive: pins the fatal-shrink unit fixture deliberately
            geometry: GeometryDomain {
                class: TopologyClass::Holed,
                min_holes: 0,
                min_components: 1,
                boundary: BoundaryRule::Strict,
                // exhaustive: pins the fatal-shrink unit fixture deliberately
                bounds: GeometryBounds {
                    max_abs_units: 40_000,
                    max_vertices: 32,
                    max_holes: 2,
                    max_components: 2,
                    max_shrink_candidates: 8,
                },
            },
            solid_witness: None,
            // exhaustive: pins the fatal-shrink unit fixture deliberately
            bounds: RecipeBounds {
                max_nodes: 24,
                max_depth: 6,
                max_facets: 16,
                max_source_bytes: 16 * 1024,
                max_evaluations: 32,
            },
        }
    }

    fn fixture_options(bounds: &RecipeBounds) -> ScadEvalOptions {
        let (ast_nodes, ast_depth, geometry_nodes) = super::super::required_adapter_capacity(bounds);
        ScadEvalOptions {
            parameters: Vec::new(),
            extrude_height_mm: None,
            // exhaustive: pins the fatal-shrink unit fixture's adapter limits
            limits: slicer_scad::ScadLimits {
                evaluation_steps: 5_000_000,
                max_source_bytes: bounds.max_source_bytes,
                max_ast_nodes: ast_nodes,
                max_ast_depth: ast_depth,
                max_geometry_nodes: geometry_nodes,
                max_facets: u32::from(bounds.max_facets),
                max_vertices: 100_000,
                max_triangles: 200_000,
                max_abs_coordinate_mm: 4.0,
                worker_stack_bytes: slicer_scad::ScadLimits::default().worker_stack_bytes,
            },
            random_seed: Some(7),
        }
    }

    #[test]
    fn scripted_fatal_shrink_error_keeps_incomplete_status() {
        let domain = fixture_domain();
        let options = fixture_options(&domain.bounds);
        let strategy = scripted_cases(domain, options).expect("fixture strategy");
        let mut runner = TestRunner::new(Config {
            cases: 1,
            failure_persistence: None,
            ..Config::default()
        });
        let mut tree = strategy.new_tree(&mut runner).expect("fixture case");
        // Redirect attempt journaling to a blocked path *after* the initial tree
        // so the failure lands inside a shrink candidate, not in generation.
        let blocked = std::env::temp_dir().join(format!(
            "pnp-scripted-fatal-shrink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&blocked);
        std::fs::create_dir_all(&blocked).expect("blocked root");
        std::fs::write(blocked.join("attempts"), b"not a directory").expect("block attempts");
        set_attempt_root(Some(blocked.clone()), Some(1));
        reset_generator_telemetry();
        let progressed = tree.simplify();
        let completed = last_shrink_completed();
        let fault = take_generator_fault();
        set_attempt_root(None, None);
        let _ = std::fs::remove_dir_all(&blocked);
        assert!(
            !progressed,
            "a fatal shrink candidate must stop the search"
        );
        assert_eq!(
            completed,
            Some(false),
            "a fatal shrink error must not claim completed minimization"
        );
        assert!(fault.is_some(), "the fatal generator fault is retained");
    }
}
