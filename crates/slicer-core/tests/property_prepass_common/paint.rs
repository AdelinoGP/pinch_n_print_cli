//! Independent orthogonal-cell model for `H-PAINT-COMPOSE`.
//!
//! `compose_variants` (`crates/slicer-core/src/algos/paint_segmentation/compose_variants.rs`)
//! partitions one layer's total contour into disjoint variant chains. The model
//! here never asks the system under test — or Clipper — for an expected
//! partition. It builds a cell grid on an **even** coordinate lattice so every
//! cell midpoint is an integer grid point, derives each cell's canonical chain
//! from the declared labels, and then classifies every returned polygon set with
//! `oracles::point_in_region` at the midpoint.
//!
//! Even coordinates are load-bearing: `point_in_region` takes integer points,
//! and every produced boundary lies on an input boundary, so a midpoint can
//! never land on a boundary.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_core::algos::paint_segmentation::compose_variants::{
    compose_variants, ChainKey, SemanticOutput,
};
use slicer_ir::{ExPolygon, PaintSemantic, PaintValue, Point2, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

/// One declared axis of the compose fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Axis {
    /// Namespace spelling.
    pub name: &'static str,
    /// Declared region-split priority.
    pub priority: u32,
    /// Legal values for this axis, in declared order.
    pub values: &'static [ComposeValueKind],
}

/// One legal paint value for the compose fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeValueKind {
    /// `Flag(true)`.
    FlagTrue,
    /// `ToolIndex(0)`.
    ToolZero,
    /// `ToolIndex(1)`.
    ToolOne,
    /// `Custom("alpha")`.
    CustomAlpha,
}

impl ComposeValueKind {
    fn to_paint_value(self) -> PaintValue {
        match self {
            ComposeValueKind::FlagTrue => PaintValue::Flag(true),
            ComposeValueKind::ToolZero => PaintValue::ToolIndex(0),
            ComposeValueKind::ToolOne => PaintValue::ToolIndex(1),
            ComposeValueKind::CustomAlpha => PaintValue::Custom("alpha".to_owned()),
        }
    }
}

/// The three legal compose axes. `material` and `fuzzy_skin` carry the real core
/// priorities; the community axis sits at the floor and ties lexically with
/// nothing, so priority-versus-name disagreement and multi-value axes are both
/// reachable.
pub const AXES: [Axis; 3] = [
    Axis {
        name: "material",
        priority: 100,
        values: &[ComposeValueKind::ToolZero, ComposeValueKind::ToolOne],
    },
    Axis {
        name: "fuzzy_skin",
        priority: 200,
        values: &[ComposeValueKind::FlagTrue],
    },
    Axis {
        name: "com.example.alpha",
        priority: 1100,
        values: &[ComposeValueKind::CustomAlpha],
    },
];

/// One generated fixture: a rectangular total split into a cell grid, with one
/// label set per `(layer, cell)`.
#[derive(Debug, Clone)]
pub struct ComposeCase {
    /// Even positive width in canonical units.
    pub width: i64,
    /// Even positive height in canonical units.
    pub height: i64,
    /// Strictly increasing interior X cut points (even, inside `(0, width)`).
    pub xs: Vec<i64>,
    /// Strictly increasing interior Y cut points (even, inside `(0, height)`).
    pub ys: Vec<i64>,
    /// Declared axes (a subset of [`AXES`]).
    pub axes: Vec<Axis>,
    /// Per layer, per cell, one optional value index per declared axis.
    pub labels: Vec<Vec<Vec<Option<usize>>>>,
}

/// One open cell of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Left edge.
    pub x0: i64,
    /// Right edge.
    pub x1: i64,
    /// Bottom edge.
    pub y0: i64,
    /// Top edge.
    pub y1: i64,
}

impl ComposeCase {
    /// The grid's X boundaries, including both outer edges.
    #[must_use]
    pub fn x_boundaries(&self) -> Vec<i64> {
        let mut values = vec![0, self.width];
        values.extend(self.xs.iter().copied());
        values.sort_unstable();
        values.dedup();
        values
    }

    /// The grid's Y boundaries, including both outer edges.
    #[must_use]
    pub fn y_boundaries(&self) -> Vec<i64> {
        let mut values = vec![0, self.height];
        values.extend(self.ys.iter().copied());
        values.sort_unstable();
        values.dedup();
        values
    }

    /// Cells in `(row, column)` order: rows bottom-up.
    #[must_use]
    pub fn cells(&self) -> Vec<Cell> {
        let xs = self.x_boundaries();
        let ys = self.y_boundaries();
        let mut cells = Vec::new();
        for row in 0..ys.len().saturating_sub(1) {
            for column in 0..xs.len().saturating_sub(1) {
                cells.push(Cell {
                    x0: xs[column],
                    x1: xs[column + 1],
                    y0: ys[row],
                    y1: ys[row + 1],
                });
            }
        }
        cells
    }

    /// Independently derived axis order: ascending `(priority, name)`.
    #[must_use]
    pub fn axis_order(&self) -> Vec<&'static str> {
        let mut pairs: Vec<(u32, &'static str)> = self
            .axes
            .iter()
            .map(|axis| (axis.priority, axis.name))
            .collect();
        pairs.sort();
        pairs.into_iter().map(|(_, name)| name).collect()
    }

    /// The chain a cell carries on one layer, in canonical axis order.
    #[must_use]
    pub fn expected_chain(&self, layer: usize, cell_index: usize) -> ChainKey {
        let order = self.axis_order();
        let mut chain = Vec::new();
        for name in order {
            let axis_index = self
                .axes
                .iter()
                .position(|axis| axis.name == name)
                .expect("axis order only names declared axes");
            if let Some(value_index) = self.labels[layer][cell_index][axis_index] {
                chain.push((
                    name.to_owned(),
                    self.axes[axis_index].values[value_index].to_paint_value(),
                ));
            }
        }
        chain
    }

    /// Every chain key the composer must emit.
    ///
    /// `compose_variants` enumerates every subset of the **supplied** semantic
    /// outputs and inserts a key for each, whether or not that subset owns any
    /// geometry. The supplied outputs are one per `(axis, declared value)`, so
    /// the key set is the cross-product `∏ (1 + K_i)` in canonical axis order.
    /// Geometry ownership is checked separately, cell by cell.
    #[must_use]
    pub fn expected_keys(&self) -> BTreeSet<ChainKey> {
        let mut keys: BTreeSet<ChainKey> = BTreeSet::new();
        keys.insert(Vec::new());
        for name in self.axis_order() {
            let axis = self
                .axes
                .iter()
                .find(|axis| axis.name == name)
                .expect("axis order only names declared axes");
            let mut extended = BTreeSet::new();
            for chain in &keys {
                extended.insert(chain.clone());
                for value in axis.values {
                    let mut with = chain.clone();
                    with.push((name.to_owned(), value.to_paint_value()));
                    extended.insert(with);
                }
            }
            keys = extended;
        }
        keys
    }
}

/// Build the total contour and the per-`(axis, value)` polygon outputs the
/// system under test consumes.
///
/// Outputs are supplied in canonical axis order (the documented caller
/// contract), with values in declared order.
pub fn inputs(case: &ComposeCase) -> (Vec<Vec<ExPolygon>>, Vec<SemanticOutput>) {
    let total = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 {
                    x: case.width,
                    y: 0,
                },
                Point2 {
                    x: case.width,
                    y: case.height,
                },
                Point2 {
                    x: 0,
                    y: case.height,
                },
            ],
        },
        holes: Vec::new(),
    };
    let totals = vec![vec![total.clone()]; case.labels.len()];

    let cells = case.cells();
    let order = case.axis_order();
    let mut outputs = Vec::new();
    for name in order {
        let axis_index = case
            .axes
            .iter()
            .position(|axis| axis.name == name)
            .expect("axis order only names declared axes");
        let axis = &case.axes[axis_index];
        for (value_index, value) in axis.values.iter().enumerate() {
            let mut per_layer = Vec::new();
            for layer in 0..case.labels.len() {
                let mut polygons = Vec::new();
                for (cell_index, cell) in cells.iter().enumerate() {
                    if case.labels[layer][cell_index][axis_index] == Some(value_index) {
                        polygons.push(rect(case, *cell));
                    }
                }
                per_layer.push(polygons);
            }
            outputs.push(SemanticOutput {
                semantic: paint_semantic(axis.name),
                value: value.to_paint_value(),
                per_layer,
            });
        }
    }
    (totals, outputs)
}

fn paint_semantic(name: &str) -> PaintSemantic {
    match name {
        "material" => PaintSemantic::Material,
        "fuzzy_skin" => PaintSemantic::FuzzySkin,
        other => PaintSemantic::Custom(other.to_owned()),
    }
}

fn rect(_case: &ComposeCase, cell: Cell) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: cell.x0,
                    y: cell.y0,
                },
                Point2 {
                    x: cell.x1,
                    y: cell.y0,
                },
                Point2 {
                    x: cell.x1,
                    y: cell.y1,
                },
                Point2 {
                    x: cell.x0,
                    y: cell.y1,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// An integer grid point strictly inside a cell.
fn midpoint(cell: Cell) -> Point2 {
    Point2 {
        x: (cell.x0 + cell.x1) / 2,
        y: (cell.y0 + cell.y1) / 2,
    }
}

/// Check one composed result against the independent cell model.
///
/// Returns the first discrepancy. `guard` receives nothing: the checker is
/// deterministic so a control can call it directly.
pub fn verify(
    case: &ComposeCase,
    composed: &[BTreeMap<ChainKey, Vec<ExPolygon>>],
) -> Result<(), String> {
    if composed.len() != case.labels.len() {
        return Err(format!(
            "compose returned {} layers, expected {}",
            composed.len(),
            case.labels.len()
        ));
    }

    let cells = case.cells();
    for (layer_index, layer_map) in composed.iter().enumerate() {
        // --- Key set (ordered vectors, compared as a set of vectors) ------
        let actual_keys: BTreeSet<ChainKey> = layer_map.keys().cloned().collect();
        let expected_keys = case.expected_keys();
        if actual_keys != expected_keys {
            return Err(format!(
                "layer {layer_index}: chain key set {actual_keys:?} != expected {expected_keys:?}"
            ));
        }

        // --- Every chain's polygons stay inside the declared extent -------
        for (chain, polygons) in layer_map {
            for polygon in polygons {
                for point in &polygon.contour.points {
                    if point.x < 0 || point.x > case.width || point.y < 0 || point.y > case.height {
                        return Err(format!(
                            "chain {chain:?} leaves the total extent at {point:?}"
                        ));
                    }
                }
                for hole in &polygon.holes {
                    for point in &hole.points {
                        if point.x < 0
                            || point.x > case.width
                            || point.y < 0
                            || point.y > case.height
                        {
                            return Err(format!(
                                "chain {chain:?} hole leaves the total extent at {point:?}"
                            ));
                        }
                    }
                }
            }
        }

        // --- Exact cell ownership -----------------------------------------
        let mut observed_painted = 0usize;
        let mut observed_base = 0usize;
        for (cell_index, cell) in cells.iter().enumerate() {
            let point = midpoint(*cell);
            let expected_chain = case.expected_chain(layer_index, cell_index);
            let mut owners = Vec::new();
            for (chain, polygons) in layer_map {
                match point_in_region(point, polygons) {
                    Ok(PointLocation::Inside) => owners.push(chain.clone()),
                    Ok(PointLocation::Outside) => {}
                    Ok(PointLocation::Boundary) => {
                        return Err(format!(
                            "cell {cell:?} midpoint {point:?} landed on a boundary of {chain:?}"
                        ))
                    }
                    Err(error) => {
                        return Err(format!("point_in_region failed on {chain:?}: {error:?}"))
                    }
                }
            }
            if owners.len() != 1 {
                return Err(format!(
                    "cell {cell:?} is owned by {} chains: {owners:?}",
                    owners.len()
                ));
            }
            if owners[0] != expected_chain {
                return Err(format!(
                    "cell {cell:?} owner {:?} != expected {:?}",
                    owners[0], expected_chain
                ));
            }
            if expected_chain.is_empty() {
                observed_base += 1;
            } else {
                observed_painted += 1;
            }
        }
        let _ = (observed_painted, observed_base);
    }
    Ok(())
}

/// Run the system under test and verify it.
pub fn observe_and_verify(case: &ComposeCase) -> Result<(), String> {
    let (totals, outputs) = inputs(case);
    let composed = compose_variants(&totals, &outputs);
    verify(case, &composed)
}

/// Strategy for one generated compose fixture.
pub fn strategy(
    max_layers: usize,
    max_semantics: usize,
    _max_cells: usize,
    max_abs_units: i64,
) -> impl Strategy<Value = ComposeCase> {
    // Even coordinates keep every cell midpoint on an integer grid point.
    let max_side = ((max_abs_units / 2).clamp(2, 12)) * 2;
    let axis_count = max_semantics.clamp(1, AXES.len());
    let axes = proptest::sample::subsequence(AXES.to_vec(), 1..=axis_count);
    (
        1..=max_layers.max(1),
        (1i64..=3, 1i64..=3),
        axes,
        proptest::collection::vec(0u8..4, 0..=4),
        proptest::collection::vec(0u8..4, 0..=4),
    )
        .prop_flat_map(move |(layers, (cols, rows), axes, xs_raw, ys_raw)| {
            let width = (cols * 2).max(2);
            let height = (rows * 2).max(2);
            let _ = max_side;
            // Interior cuts are even as well, so every cell midpoint is an
            // integer grid point that cannot sit on a cut or on the outer edge.
            let xs: Vec<i64> = xs_raw
                .into_iter()
                .map(|value| i64::from(value) * 2)
                .filter(|value| *value > 0 && *value < width)
                .collect();
            let ys: Vec<i64> = ys_raw
                .into_iter()
                .map(|value| i64::from(value) * 2)
                .filter(|value| *value > 0 && *value < height)
                .collect();
            let cells = (xs.len() + 1) * (ys.len() + 1);
            // Every label is an independent `None | Some(value)` per axis.
            let label = proptest::collection::vec(0u8..=2u8, cells * axes.len() * layers);
            let axes = axes.clone();
            let xs_for_case = xs.clone();
            let ys_for_case = ys.clone();
            label.prop_map(move |choices| {
                let mut labels = Vec::with_capacity(layers);
                let mut cursor = 0;
                for _ in 0..layers {
                    let mut layer = Vec::with_capacity(cells);
                    for _ in 0..cells {
                        let mut cell = Vec::with_capacity(axes.len());
                        for axis in &axes {
                            let choice = choices[cursor];
                            cursor += 1;
                            // Reserve 0 for "unpainted"; 1..=2 index the axis
                            // values (an axis with one value treats 2 as 0).
                            let value_index = match choice {
                                0 => None,
                                n => Some(usize::from(n - 1) % axis.values.len().max(1)),
                            };
                            cell.push(value_index);
                        }
                        layer.push(cell);
                    }
                    labels.push(layer);
                }
                // Keep every generated layer observing at least one painted
                // cell; an all-BASE draw would make the painted half of the
                // checker vacuous for that layer. The BASE-only mode keeps its
                // own deterministic witness.
                for layer in &mut labels {
                    if layer.iter().all(|cell| cell.iter().all(Option::is_none)) {
                        if let Some(first) = layer.first_mut() {
                            if let Some(slot) = first.first_mut() {
                                *slot = Some(0);
                            }
                        }
                    }
                }
                ComposeCase {
                    width,
                    height,
                    xs: xs_for_case.clone(),
                    ys: ys_for_case.clone(),
                    axes: axes.clone(),
                    labels,
                }
            })
        })
        .boxed()
}

/// Registry entry point for `H-PAINT-COMPOSE`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new("H-PAINT-COMPOSE", "proptest_paint_composition_cells")
        .map_err(|error| error.to_string())?;
    let max_layers = usize::try_from(run.bound("max_layers").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_layers does not fit a usize"))?;
    let max_semantics = usize::try_from(run.bound("max_semantics").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_semantics does not fit a usize"))?;
    let max_cells = usize::try_from(run.bound("max_cells").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_cells does not fit a usize"))?;
    let max_abs_units = i64::try_from(run.bound("max_abs_units").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_abs_units does not fit an i64"))?;

    // Widest grid this profile can generate is 4 x 4 cells (four interior cuts
    // on either axis); the bound must admit it.
    let widest_cells = 16usize;
    if widest_cells > max_cells {
        return Err(format!(
            "the generator can produce {widest_cells} cells but max_cells is {max_cells}"
        ));
    }

    // Rare deterministic witness: BASE-only must yield exactly one chain.
    let base_only = ComposeCase {
        width: 40,
        height: 40,
        xs: vec![20],
        ys: vec![],
        axes: vec![AXES[0], AXES[1]],
        labels: vec![vec![vec![None, None], vec![None, None]]],
    };
    observe_and_verify(&base_only)?;
    let (totals, outputs) = inputs(&base_only);
    let composed = compose_variants(&totals, &outputs);
    if composed[0].len() != base_only.expected_keys().len() {
        return Err(String::from(
            "a BASE-only layer must still enumerate every supplied subset",
        ));
    }
    let base_polygons = composed[0]
        .get(&Vec::new())
        .ok_or_else(|| String::from("a BASE-only layer must contain the empty chain"))?;
    if base_polygons.is_empty() {
        return Err(String::from(
            "a BASE-only layer must put the whole total on the empty chain",
        ));
    }
    for (chain, polygons) in &composed[0] {
        if !chain.is_empty() && !polygons.is_empty() {
            return Err(format!(
                "BASE-only layer gave nonempty geometry to {chain:?}"
            ));
        }
    }

    let generated = strategy(max_layers, max_semantics, max_cells.max(16), max_abs_units);
    run.check(&generated, |case: ComposeCase, _guard: &mut CaseGuard| {
        observe_and_verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}
