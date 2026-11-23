//! Native-only typed SCAD recipe properties.
//!
//! Recipes are typed, bounded ASTs. Emission is a pure function of the recipe
//! plus its [`RecipeBounds`]; evaluation goes through the public
//! [`slicer_scad::evaluate`] adapter and is never a second renderer. A
//! successful render is stimulus, not a correctness oracle: every exposed
//! value is independently validated against the requested [`ScriptedDomain`].
//!
//! Recipe integral dimensions and coordinates are **canonical units**
//! (1 unit = 100 nm = 10⁻⁴ mm), never OrcaSlicer nanometres. Emission converts
//! to exact millimetre decimal text without narrowing through `f32`.

use serde::{Deserialize, Serialize};

use super::domains::DomainError;

mod artifacts;
mod emit;
mod reference;
mod strategy;

pub use artifacts::{replay_scripted_case, write_scripted_artifacts};
pub use emit::{emit_scad, evaluate_recipe};
pub use strategy::{scripted_cases, ScriptedStrategy, ScriptedValueTree};

pub(crate) use strategy::{
    last_shrink_completed, reset_generator_telemetry, set_attempt_root, take_generator_fault,
};

/// A bounded typed planar recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Recipe2d {
    /// An axis-aligned rectangle of the given canonical-unit size.
    Rectangle {
        /// Width and height in canonical units, both positive.
        size_units: [u32; 2],
    },
    /// A regular faceted disk of the given radius and facet count.
    Disk {
        /// Radius in canonical units, positive.
        radius_units: u32,
        /// Explicit facet count, at least 3.
        facets: u16,
    },
    /// An explicit, already validated polygon set (outer contours and holes).
    ///
    /// Coordinates are canonical units; arbitrary non-star-shaped and holed
    /// shapes are preserved rather than reduced to radial vertices.
    Polygon {
        /// Outer components with their holes, in canonical units.
        polygons: Vec<slicer_ir::ExPolygon>,
    },
    /// Translate a child in the XY plane.
    Translate {
        /// X and Y offsets in canonical units.
        offset_units: [i64; 2],
        /// Translated child.
        child: Box<Recipe2d>,
    },
    /// Rotate a child by whole quarter turns around Z.
    QuarterTurn {
        /// Turn count in `0..=3`.
        turns: u8,
        /// Rotated child.
        child: Box<Recipe2d>,
    },
    /// Boolean union of the children.
    Union(Vec<Recipe2d>),
    /// Boolean difference of `base` minus every cut.
    Difference {
        /// Base shape.
        base: Box<Recipe2d>,
        /// Subtracted shapes.
        cuts: Vec<Recipe2d>,
    },
    /// Boolean intersection of the children.
    Intersection(Vec<Recipe2d>),
}

/// A bounded typed solid recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Recipe3d {
    /// An axis-aligned box of the given canonical-unit size.
    Box {
        /// X, Y and Z extents in canonical units, all positive.
        size_units: [u32; 3],
    },
    /// A faceted cylinder standing on Z=0.
    Cylinder {
        /// Radius in canonical units, positive.
        radius_units: u32,
        /// Height in canonical units, positive.
        height_units: u32,
        /// Explicit facet count, at least 3.
        facets: u16,
    },
    /// Extrude a planar recipe from Z=0 to the given height.
    LinearExtrude {
        /// Extrusion height in canonical units, positive.
        height_units: u32,
        /// Extruded planar profile.
        profile: Box<Recipe2d>,
    },
    /// Translate a child in space.
    Translate {
        /// X, Y and Z offsets in canonical units.
        offset_units: [i64; 3],
        /// Translated child.
        child: Box<Recipe3d>,
    },
    /// Rotate a child by whole quarter turns around Z.
    QuarterTurnZ {
        /// Turn count in `0..=3`.
        turns: u8,
        /// Rotated child.
        child: Box<Recipe3d>,
    },
    /// Boolean union of the children.
    Union(Vec<Recipe3d>),
    /// Boolean difference of `base` minus every cut.
    Difference {
        /// Base solid.
        base: Box<Recipe3d>,
        /// Subtracted solids.
        cuts: Vec<Recipe3d>,
    },
    /// Boolean intersection of the children.
    Intersection(Vec<Recipe3d>),
}

/// A typed scripted recipe in exactly one dimensionality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScadRecipe {
    /// A planar recipe; its converted result must be a contour forest.
    Contours(Recipe2d),
    /// A solid recipe; its converted result must be a triangle mesh.
    Solid(Recipe3d),
}

/// The independently checked structural promise a solid recipe must keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolidWitness {
    /// One closed, positive-volume body with no required optional structure.
    ClosedSolid,
    /// A nonzero enclosed void.
    Cavity,
    /// A positive unsupported span above a gap.
    Bridge,
    /// A positive unsupported span beyond its supporting footprint.
    Overhang,
    /// At least two disjoint positive-volume bodies.
    SeparatedBodies,
}

/// Finite recipe limits applied before generation and at emission time.
///
/// The default is a safe helper base only; live property profiles must supply
/// their measured bounds explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBounds {
    /// Maximum recipe nodes, including leaves and Boolean operands.
    pub max_nodes: usize,
    /// Maximum recipe nesting depth; a lone leaf has depth 1.
    pub max_depth: usize,
    /// Maximum explicit facet count of any single faceted primitive.
    pub max_facets: u16,
    /// Maximum emitted source length in UTF-8 bytes.
    pub max_source_bytes: usize,
    /// Maximum candidate evaluations per value tree, retries and shrink
    /// proposals included.
    pub max_evaluations: u32,
}

impl Default for RecipeBounds {
    /// Bounded helper defaults; live profiles must pass measured values.
    ///
    /// These are the smallest bounds that still express every required 2D
    /// topology family and every solid witness, so they are a safety floor
    /// rather than a tested profile.
    fn default() -> Self {
        Self {
            max_nodes: 24,
            max_depth: 6,
            max_facets: 32,
            max_source_bytes: 16 * 1024,
            max_evaluations: 64,
        }
    }
}

/// A requested scripted domain: converted geometry plus an optional solid
/// witness and finite recipe limits.
#[derive(Debug, Clone)]
pub struct ScriptedDomain {
    /// For contour recipes, the converted final contour domain. For solid
    /// recipes, the bounded constituent planar profile domain.
    pub geometry: super::domains::GeometryDomain,
    /// `None` selects [`ScadRecipe::Contours`]; `Some` selects
    /// [`ScadRecipe::Solid`] and names the witness the resulting scene must
    /// keep.
    pub solid_witness: Option<SolidWitness>,
    /// Finite recipe limits.
    pub bounds: RecipeBounds,
}

/// A certified scripted case: the recipe, its fixed inputs, and the evaluated
/// adapter output that satisfied the domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptedCase {
    /// The recipe that produced `source`.
    pub recipe: ScadRecipe,
    /// The bounds used for emission and evaluation accounting.
    pub recipe_bounds: RecipeBounds,
    /// The exact emitted source.
    pub source: slicer_scad::ScadSource,
    /// The exact adapter options supplied for this evaluation.
    pub options: slicer_scad::ScadEvalOptions,
    /// The full adapter output, including provenance and diagnostics.
    pub output: slicer_scad::ScadEvalOutput,
}

/// Every failure the scripted helper surface can return.
#[derive(Debug)]
pub enum ScriptedError {
    /// The requested domain, recipe or bounds are inconsistent.
    Domain(DomainError),
    /// The adapter rejected the emitted source, or produced unacceptable
    /// diagnostics, partial output or non-exact geometry.
    Adapter(slicer_scad::ScadError),
    /// An artifact could not be written, read, or trusted.
    Artifact(String),
    /// A retained case no longer reproduces its recorded inputs.
    ReplayMismatch(String),
}

impl std::fmt::Display for ScriptedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(error) => write!(f, "scripted domain: {error}"),
            Self::Adapter(error) => write!(f, "scripted adapter: {error}"),
            Self::Artifact(message) => write!(f, "scripted artifact: {message}"),
            Self::ReplayMismatch(message) => write!(f, "scripted replay mismatch: {message}"),
        }
    }
}

impl std::error::Error for ScriptedError {}

impl From<DomainError> for ScriptedError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<slicer_scad::ScadError> for ScriptedError {
    fn from(error: slicer_scad::ScadError) -> Self {
        Self::Adapter(error)
    }
}

/// Lexicographic recipe cost: nodes, explicit polygon vertices plus facets,
/// and the checked sum of absolute integral parameters.
pub type RecipeComplexity = (usize, usize, u128);

fn overflow() -> ScriptedError {
    ScriptedError::Domain(DomainError::ArithmeticOverflow)
}

/// Count recipe nodes, explicit planar vertices plus facet counts, and the
/// checked sum of absolute integral parameters.
///
/// # Errors
/// Returns [`ScriptedError::Domain`] with [`DomainError::ArithmeticOverflow`]
/// rather than wrapping when a coordinate sum leaves representable range.
pub fn recipe_complexity(recipe: &ScadRecipe) -> Result<RecipeComplexity, ScriptedError> {
    let mut cost = Cost::default();
    match recipe {
        ScadRecipe::Contours(recipe) => cost.two_d(recipe)?,
        ScadRecipe::Solid(recipe) => cost.three_d(recipe)?,
    }
    Ok((cost.nodes, cost.second, cost.sum))
}

#[derive(Default)]
struct Cost {
    nodes: usize,
    second: usize,
    sum: u128,
}

impl Cost {
    fn enter(&mut self) -> Result<(), ScriptedError> {
        self.nodes = self.nodes.checked_add(1).ok_or_else(overflow)?;
        Ok(())
    }

    fn add_unit(&mut self, value: i128) -> Result<(), ScriptedError> {
        self.sum = self
            .sum
            .checked_add(value.unsigned_abs())
            .ok_or_else(overflow)?;
        Ok(())
    }

    fn add_count(&mut self, count: usize) -> Result<(), ScriptedError> {
        self.second = self.second.checked_add(count).ok_or_else(overflow)?;
        self.add_unit(
            i128::try_from(count).map_err(|_| {
                ScriptedError::Domain(DomainError::InvalidBounds("count exceeds i128".into()))
            })?,
        )
    }

    fn add_facets(&mut self, facets: u16) -> Result<(), ScriptedError> {
        self.second = self
            .second
            .checked_add(usize::from(facets))
            .ok_or_else(overflow)?;
        self.add_unit(i128::from(facets))
    }

    fn two_d(&mut self, recipe: &Recipe2d) -> Result<(), ScriptedError> {
        self.enter()?;
        match recipe {
            Recipe2d::Rectangle { size_units } => {
                for extent in size_units {
                    self.add_unit(i128::from(*extent))?;
                }
            }
            Recipe2d::Disk {
                radius_units,
                facets,
            } => {
                self.add_unit(i128::from(*radius_units))?;
                self.add_facets(*facets)?;
            }
            Recipe2d::Polygon { polygons } => {
                for polygon in polygons {
                    for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
                        self.add_count(ring.points.len())?;
                        for point in &ring.points {
                            self.add_unit(i128::from(point.x))?;
                            self.add_unit(i128::from(point.y))?;
                        }
                    }
                }
            }
            Recipe2d::Translate {
                offset_units,
                child,
            } => {
                for offset in offset_units {
                    self.add_unit(i128::from(*offset))?;
                }
                self.two_d(child)?;
            }
            Recipe2d::QuarterTurn { turns, child } => {
                self.add_unit(i128::from(*turns))?;
                self.two_d(child)?;
            }
            Recipe2d::Union(children) | Recipe2d::Intersection(children) => {
                self.add_count(children.len())?;
                for child in children {
                    self.two_d(child)?;
                }
            }
            Recipe2d::Difference { base, cuts } => {
                self.add_count(cuts.len())?;
                self.two_d(base)?;
                for cut in cuts {
                    self.two_d(cut)?;
                }
            }
        }
        Ok(())
    }

    fn three_d(&mut self, recipe: &Recipe3d) -> Result<(), ScriptedError> {
        self.enter()?;
        match recipe {
            Recipe3d::Box { size_units } => {
                for extent in size_units {
                    self.add_unit(i128::from(*extent))?;
                }
            }
            Recipe3d::Cylinder {
                radius_units,
                height_units,
                facets,
            } => {
                self.add_unit(i128::from(*radius_units))?;
                self.add_unit(i128::from(*height_units))?;
                self.add_facets(*facets)?;
            }
            Recipe3d::LinearExtrude {
                height_units,
                profile,
            } => {
                self.add_unit(i128::from(*height_units))?;
                self.two_d(profile)?;
            }
            Recipe3d::Translate {
                offset_units,
                child,
            } => {
                for offset in offset_units {
                    self.add_unit(i128::from(*offset))?;
                }
                self.three_d(child)?;
            }
            Recipe3d::QuarterTurnZ { turns, child } => {
                self.add_unit(i128::from(*turns))?;
                self.three_d(child)?;
            }
            Recipe3d::Union(children) | Recipe3d::Intersection(children) => {
                self.add_count(children.len())?;
                for child in children {
                    self.three_d(child)?;
                }
            }
            Recipe3d::Difference { base, cuts } => {
                self.add_count(cuts.len())?;
                self.three_d(base)?;
                for cut in cuts {
                    self.three_d(cut)?;
                }
            }
        }
        Ok(())
    }
}

/// Recipe shape accounting shared by emission, construction and shrinking.
#[derive(Default)]
pub(crate) struct Shape {
    /// Total recipe nodes.
    pub(crate) nodes: usize,
    /// Maximum nesting depth.
    pub(crate) depth: usize,
    /// Sum of explicit planar vertices.
    pub(crate) vertices: usize,
    /// Largest explicit facet count.
    pub(crate) facets: u16,
    /// Conservative absolute canonical-unit envelope.
    pub(crate) envelope: i128,
}

pub(crate) fn invalid_bounds(message: impl Into<String>) -> ScriptedError {
    ScriptedError::Domain(DomainError::InvalidBounds(message.into()))
}

pub(crate) fn invalid_input(message: impl Into<String>) -> ScriptedError {
    ScriptedError::Domain(DomainError::InvalidInput(message.into()))
}

impl Shape {
    fn nest(mut self, offset: i128) -> Result<Shape, ScriptedError> {
        self.nodes = self.nodes.checked_add(1).ok_or_else(overflow)?;
        self.depth = self.depth.checked_add(1).ok_or_else(overflow)?;
        self.envelope = self
            .envelope
            .checked_add(offset.abs())
            .ok_or_else(overflow)?;
        Ok(self)
    }
}

/// Compute the bounded shape of a recipe.
pub(crate) fn shape(recipe: &ScadRecipe) -> Result<Shape, ScriptedError> {
    match recipe {
        ScadRecipe::Contours(recipe) => shape2d(recipe),
        ScadRecipe::Solid(recipe) => shape3d(recipe),
    }
}

fn polygon_shape(polygons: &[slicer_ir::ExPolygon]) -> Result<Shape, ScriptedError> {
    let mut shape = Shape {
        nodes: 1,
        depth: 1,
        ..Shape::default()
    };
    let mut extent: i128 = 0;
    for polygon in polygons {
        for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
            shape.vertices = shape
                .vertices
                .checked_add(ring.points.len())
                .ok_or_else(overflow)?;
            for point in &ring.points {
                extent = extent
                    .max(i128::from(point.x).abs())
                    .max(i128::from(point.y).abs());
            }
        }
    }
    shape.envelope = extent;
    Ok(shape)
}

fn shape2d(recipe: &Recipe2d) -> Result<Shape, ScriptedError> {
    let mut shape = match recipe {
        Recipe2d::Rectangle { size_units } => Shape {
            nodes: 1,
            depth: 1,
            envelope: i128::from(size_units[0]).max(i128::from(size_units[1])),
            ..Shape::default()
        },
        Recipe2d::Disk {
            radius_units,
            facets,
        } => Shape {
            nodes: 1,
            depth: 1,
            facets: *facets,
            envelope: i128::from(*radius_units),
            ..Shape::default()
        },
        Recipe2d::Polygon { polygons } => polygon_shape(polygons)?,
        Recipe2d::Translate {
            offset_units,
            child,
        } => {
            let offset = i128::from(offset_units[0]).abs() + i128::from(offset_units[1]).abs();
            shape2d(child)?.nest(offset)?
        }
        Recipe2d::QuarterTurn { child, .. } => shape2d(child)?.nest(0)?,
        Recipe2d::Union(children) | Recipe2d::Intersection(children) => {
            let mut shape = Shape {
                nodes: 1,
                depth: 1,
                ..Shape::default()
            };
            for child in children {
                shape = shape.merged_child(shape2d(child)?)?;
            }
            shape
        }
        Recipe2d::Difference { base, cuts } => {
            let mut shape = Shape {
                nodes: 1,
                depth: 1,
                ..Shape::default()
            };
            shape = shape.merged_child(shape2d(base)?)?;
            for cut in cuts {
                shape = shape.merged_child(shape2d(cut)?)?;
            }
            shape
        }
    };
    shape.nodes = shape.nodes.max(1);
    shape.depth = shape.depth.max(1);
    Ok(shape)
}

fn shape3d(recipe: &Recipe3d) -> Result<Shape, ScriptedError> {
    let shape = match recipe {
        Recipe3d::Box { size_units } => Shape {
            nodes: 1,
            depth: 1,
            envelope: i128::from(size_units[0])
                .max(i128::from(size_units[1]))
                .max(i128::from(size_units[2])),
            ..Shape::default()
        },
        Recipe3d::Cylinder {
            radius_units,
            height_units,
            facets,
        } => Shape {
            nodes: 1,
            depth: 1,
            facets: *facets,
            envelope: i128::from(*radius_units).max(i128::from(*height_units)),
            ..Shape::default()
        },
        Recipe3d::LinearExtrude {
            height_units,
            profile,
        } => {
            let offset = i128::from(*height_units);
            shape2d(profile)?.nest(offset)?
        }
        Recipe3d::Translate {
            offset_units,
            child,
        } => {
            let offset = i128::from(offset_units[0]).abs()
                + i128::from(offset_units[1]).abs()
                + i128::from(offset_units[2]).abs();
            shape3d(child)?.nest(offset)?
        }
        Recipe3d::QuarterTurnZ { child, .. } => shape3d(child)?.nest(0)?,
        Recipe3d::Union(children) | Recipe3d::Intersection(children) => {
            let mut shape = Shape {
                nodes: 1,
                depth: 1,
                ..Shape::default()
            };
            for child in children {
                shape = shape.merged_child(shape3d(child)?)?;
            }
            shape
        }
        Recipe3d::Difference { base, cuts } => {
            let mut shape = Shape {
                nodes: 1,
                depth: 1,
                ..Shape::default()
            };
            shape = shape.merged_child(shape3d(base)?)?;
            for cut in cuts {
                shape = shape.merged_child(shape3d(cut)?)?;
            }
            shape
        }
    };
    let mut shape = shape;
    shape.nodes = shape.nodes.max(1);
    shape.depth = shape.depth.max(1);
    Ok(shape)
}

impl Shape {
    fn merged_child(mut self, child: Shape) -> Result<Shape, ScriptedError> {
        self.nodes = self.nodes.checked_add(child.nodes).ok_or_else(overflow)?;
        self.depth = self.depth.max(child.depth.checked_add(1).ok_or_else(overflow)?);
        self.vertices = self
            .vertices
            .checked_add(child.vertices)
            .ok_or_else(overflow)?;
        self.facets = self.facets.max(child.facets);
        self.envelope = self.envelope.max(child.envelope);
        Ok(self)
    }
}

/// Validate that a recipe is bounded and structurally legal, independent of any
/// domain. Primitive dimensions must be positive, turn counts in `0..=3`, and
/// every faceted primitive must have at least three facets.
pub(crate) fn validate_recipe(recipe: &ScadRecipe, bounds: &RecipeBounds) -> Result<Shape, ScriptedError> {
    check_bounds(bounds)?;
    let shape = shape(recipe)?;
    if shape.nodes > bounds.max_nodes {
        return Err(invalid_bounds(format!(
            "recipe has {} nodes, above max_nodes {}",
            shape.nodes, bounds.max_nodes
        )));
    }
    if shape.depth > bounds.max_depth {
        return Err(invalid_bounds(format!(
            "recipe depth {} exceeds max_depth {}",
            shape.depth, bounds.max_depth
        )));
    }
    if shape.facets > bounds.max_facets {
        return Err(invalid_bounds(format!(
            "recipe facet count {} exceeds max_facets {}",
            shape.facets, bounds.max_facets
        )));
    }
    validate_primitive_values(recipe)?;
    Ok(shape)
}

fn check_facets(facets: u16) -> Result<(), ScriptedError> {
    if facets < 3 {
        return Err(invalid_input("a faceted primitive needs at least 3 facets"));
    }
    Ok(())
}

fn check_turns(turns: u8) -> Result<(), ScriptedError> {
    if turns > 3 {
        return Err(invalid_input("quarter turns must be in 0..=3"));
    }
    Ok(())
}

fn check_positive(value: u32, label: &str) -> Result<(), ScriptedError> {
    if value == 0 {
        return Err(invalid_input(format!("{label} must be positive")));
    }
    Ok(())
}

fn validate_primitive_values(recipe: &ScadRecipe) -> Result<(), ScriptedError> {
    match recipe {
        ScadRecipe::Contours(recipe) => validate2d(recipe),
        ScadRecipe::Solid(recipe) => validate3d(recipe),
    }
}

fn validate2d(recipe: &Recipe2d) -> Result<(), ScriptedError> {
    match recipe {
        Recipe2d::Rectangle { size_units } => {
            check_positive(size_units[0], "rectangle width")?;
            check_positive(size_units[1], "rectangle height")
        }
        Recipe2d::Disk {
            radius_units,
            facets,
        } => {
            check_positive(*radius_units, "disk radius")?;
            check_facets(*facets)
        }
        Recipe2d::Polygon { polygons } => {
            if polygons
                .iter()
                .flat_map(|p| std::iter::once(&p.contour).chain(p.holes.iter()))
                .any(|ring| ring.points.len() < 3)
            {
                return Err(invalid_input("a polygon ring needs at least 3 vertices"));
            }
            Ok(())
        }
        Recipe2d::Translate { child, .. } => validate2d(child),
        Recipe2d::QuarterTurn { turns, child } => {
            check_turns(*turns)?;
            validate2d(child)
        }
        Recipe2d::Union(children) | Recipe2d::Intersection(children) => {
            for child in children {
                validate2d(child)?;
            }
            Ok(())
        }
        Recipe2d::Difference { base, cuts } => {
            validate2d(base)?;
            for cut in cuts {
                validate2d(cut)?;
            }
            Ok(())
        }
    }
}

fn validate3d(recipe: &Recipe3d) -> Result<(), ScriptedError> {
    match recipe {
        Recipe3d::Box { size_units } => {
            check_positive(size_units[0], "box width")?;
            check_positive(size_units[1], "box depth")?;
            check_positive(size_units[2], "box height")
        }
        Recipe3d::Cylinder {
            radius_units,
            height_units,
            facets,
        } => {
            check_positive(*radius_units, "cylinder radius")?;
            check_positive(*height_units, "cylinder height")?;
            check_facets(*facets)
        }
        Recipe3d::LinearExtrude {
            height_units,
            profile,
        } => {
            check_positive(*height_units, "extrusion height")?;
            validate2d(profile)
        }
        Recipe3d::Translate { child, .. } => validate3d(child),
        Recipe3d::QuarterTurnZ { turns, child } => {
            check_turns(*turns)?;
            validate3d(child)
        }
        Recipe3d::Union(children) | Recipe3d::Intersection(children) => {
            for child in children {
                validate3d(child)?;
            }
            Ok(())
        }
        Recipe3d::Difference { base, cuts } => {
            validate3d(base)?;
            for cut in cuts {
                validate3d(cut)?;
            }
            Ok(())
        }
    }
}

/// Check a [`RecipeBounds`] value on its own.
pub(crate) fn check_bounds(bounds: &RecipeBounds) -> Result<(), ScriptedError> {
    if bounds.max_nodes == 0 {
        return Err(invalid_bounds("max_nodes must be positive"));
    }
    if bounds.max_depth == 0 {
        return Err(invalid_bounds("max_depth must be positive"));
    }
    if bounds.max_facets < 3 {
        return Err(invalid_bounds("max_facets must be at least 3"));
    }
    if bounds.max_source_bytes < 32 {
        return Err(invalid_bounds("max_source_bytes must be at least 32"));
    }
    if bounds.max_evaluations == 0 {
        return Err(invalid_bounds("max_evaluations must be positive"));
    }
    Ok(())
}

/// The conservative AST capacity an emitter-bounded recipe can need.
pub(crate) const AST_NODES_PER_RECIPE_NODE: usize = 8;
/// Fixed AST headroom for the wrapper statements emission adds.
pub(crate) const AST_NODE_HEADROOM: usize = 16;
/// Depth multiplier for the wrapper statements emission adds per level.
pub(crate) const AST_DEPTH_PER_RECIPE_DEPTH: usize = 2;
/// Fixed AST depth headroom.
pub(crate) const AST_DEPTH_HEADROOM: usize = 8;

/// The adapter AST/geometry capacity this recipe bound can require.
///
/// A polygon leaf contributes one AST node per vertex, so the structural node
/// count alone is not a bound. Every AST node needs at least one source byte,
/// which makes `max_source_bytes` a safe upper bound on AST nodes; the
/// structural estimate is kept as a floor so a small source bound cannot
/// starve a deep recipe.
pub(crate) fn required_adapter_capacity(bounds: &RecipeBounds) -> (usize, usize, usize) {
    let structural = bounds
        .max_nodes
        .saturating_mul(AST_NODES_PER_RECIPE_NODE)
        .saturating_add(AST_NODE_HEADROOM);
    let nodes = structural.max(bounds.max_source_bytes);
    (
        nodes,
        bounds
            .max_depth
            .saturating_mul(AST_DEPTH_PER_RECIPE_DEPTH)
            .saturating_add(AST_DEPTH_HEADROOM),
        // A polygon leaf expands to several evaluated CSG nodes (one per ring
        // and Boolean wrapper), so the structural recipe count is not a bound
        // either: every evaluated node comes from a statement, and every
        // statement occupies at least one source byte.
        nodes,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle(width: u32, height: u32) -> Recipe2d {
        Recipe2d::Rectangle {
            size_units: [width, height],
        }
    }

    #[test]
    fn scripted_recipe_complexity_is_checked_and_lexicographic() {
        let leaf = ScadRecipe::Contours(rectangle(4, 3));
        let nested = ScadRecipe::Contours(Recipe2d::Translate {
            offset_units: [2, -5],
            child: Box::new(rectangle(4, 3)),
        });
        assert_eq!(recipe_complexity(&leaf).unwrap(), (1, 0, 7));
        assert_eq!(recipe_complexity(&nested).unwrap(), (2, 0, 14));
        assert!(recipe_complexity(&leaf).unwrap() < recipe_complexity(&nested).unwrap());

        let disk = ScadRecipe::Contours(Recipe2d::Disk {
            radius_units: 3,
            facets: 8,
        });
        assert_eq!(recipe_complexity(&disk).unwrap(), (1, 8, 11));
    }

    #[test]
    fn scripted_recipe_bounds_reject_inconsistent_capacities() {
        let bounds = RecipeBounds::default();
        let mut zero_nodes = bounds;
        zero_nodes.max_nodes = 0;
        assert!(check_bounds(&zero_nodes).is_err());

        let mut few_facets = bounds;
        few_facets.max_facets = 2;
        assert!(check_bounds(&few_facets).is_err());

        let too_deep = RecipeBounds {
            max_depth: 1,
            ..bounds
        };
        let nested = ScadRecipe::Contours(Recipe2d::Translate {
            offset_units: [1, 0],
            child: Box::new(rectangle(2, 2)),
        });
        assert!(validate_recipe(&nested, &too_deep).is_err());
        assert!(validate_recipe(&nested, &bounds).is_ok());
    }

    #[test]
    fn scripted_recipe_rejects_zero_dimensions_and_extra_turns() {
        let bounds = RecipeBounds::default();
        let zero = ScadRecipe::Contours(rectangle(0, 3));
        assert!(validate_recipe(&zero, &bounds).is_err());
        let turns = ScadRecipe::Contours(Recipe2d::QuarterTurn {
            turns: 4,
            child: Box::new(rectangle(2, 2)),
        });
        assert!(validate_recipe(&turns, &bounds).is_err());
        let few = ScadRecipe::Solid(Recipe3d::Cylinder {
            radius_units: 2,
            height_units: 2,
            facets: 2,
        });
        assert!(validate_recipe(&few, &bounds).is_err());
    }
}
