/// Spatial cell index for 2D line segments.
use crate::algos::paint_segmentation::triangle_intersect::Line;
use slicer_ir::slice_ir::BoundingBox2;
use slicer_ir::Point2;

/// Grid of cells covering a 2D bounding box, for spatial line queries.
pub struct EdgeGrid {
    bbox: BoundingBox2,
    cell_size: i64,
    cols: usize,
    rows: usize,
}

impl EdgeGrid {
    /// Create a new grid covering `bbox` with cells of size `cell_size`.
    pub fn new(bbox: BoundingBox2, cell_size: i64) -> Self {
        assert!(cell_size > 0);
        let dx = (bbox.max.x - bbox.min.x).max(1);
        let dy = (bbox.max.y - bbox.min.y).max(1);
        let cols = ((dx + cell_size - 1) / cell_size) as usize;
        let rows = ((dy + cell_size - 1) / cell_size) as usize;
        Self {
            bbox,
            cell_size,
            cols,
            rows,
        }
    }

    /// Number of columns in the grid.
    pub fn cols(&self) -> usize {
        self.cols
    }
    /// Number of rows in the grid.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Call `visitor` with each cell index that `line` passes through.
    pub fn visit_cells_intersecting_line(&self, line: &Line, visitor: &mut dyn FnMut(usize)) {
        let to_grid = |v: Point2| -> (i64, i64) {
            let col = ((v.x - self.bbox.min.x) / self.cell_size).clamp(0, self.cols as i64 - 1);
            let row = ((v.y - self.bbox.min.y) / self.cell_size).clamp(0, self.rows as i64 - 1);
            (col, row)
        };

        let (c0, r0) = to_grid(line.start);
        let (c1, r1) = to_grid(line.end);

        let min_c = c0.min(c1);
        let max_c = c0.max(c1);
        let min_r = r0.min(r1);
        let max_r = r0.max(r1);

        for r in min_r..=max_r {
            for c in min_c..=max_c {
                visitor(r as usize * self.cols + c as usize);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bbox(x0: i64, y0: i64, x1: i64, y1: i64) -> BoundingBox2 {
        BoundingBox2 {
            min: Point2 { x: x0, y: y0 },
            max: Point2 { x: x1, y: y1 },
        }
    }

    #[test]
    fn single_cell() {
        let grid = EdgeGrid::new(bbox(0, 0, 100, 100), 100);
        let line = Line {
            start: Point2 { x: 10, y: 10 },
            end: Point2 { x: 50, y: 50 },
        };
        let mut cells = Vec::new();
        grid.visit_cells_intersecting_line(&line, &mut |i| cells.push(i));
        assert_eq!(cells, vec![0]);
    }

    #[test]
    fn multi_cell() {
        let grid = EdgeGrid::new(bbox(0, 0, 400, 400), 100);
        let line = Line {
            start: Point2 { x: 50, y: 50 },
            end: Point2 { x: 350, y: 350 },
        };
        let mut cells = Vec::new();
        grid.visit_cells_intersecting_line(&line, &mut |i| cells.push(i));
        assert!(cells.len() > 1);
    }
}
