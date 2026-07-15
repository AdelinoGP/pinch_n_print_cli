//! Sparse spatial lookup for f32 millimeter coordinates.
//!
//! OrcaSlicer's template uses `SparsePointGrid<T, Locator>` with a separate
//! locator type. PnP uses a concrete [`Point2`] extractor for now; a single
//! monomorphisation is sufficient for the current callers.

use std::collections::HashMap;

/// A two-dimensional point in the grid's f32 millimeter coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    /// Horizontal coordinate in millimeters.
    pub x: f32,
    /// Vertical coordinate in millimeters.
    pub y: f32,
}

/// A sparse grid that indexes copied payloads by the cell containing their location.
pub struct SparsePointGrid<T, F> {
    cell_size: f32,
    locator: F,
    cells: HashMap<(i64, i64), Vec<T>>,
}

impl<T, F> SparsePointGrid<T, F>
where
    T: Copy,
    F: Fn(&T) -> Point2,
{
    /// Creates a grid. The cell size is stored verbatim, as in OrcaSlicer.
    pub fn new(cell_size: f32, locator: F) -> Self {
        Self {
            cell_size,
            locator,
            cells: HashMap::new(),
        }
    }

    /// Inserts an item into the cell containing its located point.
    pub fn insert(&mut self, item: T) {
        if self.cell_size == 0.0 {
            return;
        }

        let point = (self.locator)(&item);
        let cell = Self::cell_for(point, self.cell_size);
        self.cells.entry(cell).or_default().push(item);
    }

    /// Returns copied items from every cell touched by the query radius.
    /// Callers apply the exact distance check when needed.
    pub fn get_nearby(&self, query: Point2, radius: f32) -> Vec<T> {
        if self.cell_size == 0.0 || radius < 0.0 {
            return Vec::new();
        }

        let x_min =
            ((f64::from(query.x) - f64::from(radius)) / f64::from(self.cell_size)).floor() as i64;
        let x_max =
            ((f64::from(query.x) + f64::from(radius)) / f64::from(self.cell_size)).floor() as i64;
        let y_min =
            ((f64::from(query.y) - f64::from(radius)) / f64::from(self.cell_size)).floor() as i64;
        let y_max =
            ((f64::from(query.y) + f64::from(radius)) / f64::from(self.cell_size)).floor() as i64;
        let mut nearby = Vec::new();
        for x in x_min..=x_max {
            for y in y_min..=y_max {
                if let Some(items) = self.cells.get(&(x, y)) {
                    nearby.extend(items.iter().copied());
                }
            }
        }
        nearby
    }

    fn cell_for(point: Point2, cell_size: f32) -> (i64, i64) {
        (
            (f64::from(point.x) / f64::from(cell_size)).floor() as i64,
            (f64::from(point.y) / f64::from(cell_size)).floor() as i64,
        )
    }
}
