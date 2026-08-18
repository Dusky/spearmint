//! Cell storage.
//!
//! One byte per cell in a flat `Vec`, which is the shape spec 9 wants at the wasm
//! boundary: a typed array the renderer can view directly without copying per frame.

use crate::elements::{ElementId, EMPTY};
use crate::field::{Bounds, CellField};

/// Cells outside the grid behave as immovable boundary, so a closed world stays closed
/// and nothing falls out of it. Milestone 2 replaces this with real chunk neighbours.
#[derive(Clone, Debug)]
pub struct Grid {
    width: u32,
    height: u32,
    cells: Vec<ElementId>,
    /// One bit per cell: this cell's contents already moved this tick, and must not be
    /// carried along again by a later visit.
    moved: Vec<u64>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Grid {
        let count = width as usize * height as usize;
        Grid {
            width,
            height,
            cells: vec![EMPTY; count],
            moved: vec![0; count.div_ceil(64)],
        }
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn cells(&self) -> &[ElementId] {
        &self.cells
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        if self.in_bounds(x, y) {
            Some(y as usize * self.width as usize + x as usize)
        } else {
            None
        }
    }

    pub fn count_of(&self, id: ElementId) -> usize {
        self.cells.iter().filter(|&&cell| cell == id).count()
    }

    fn swap_indices(&mut self, a: usize, b: usize) {
        self.cells.swap(a, b);
    }

    fn moved_at(&self, index: usize) -> bool {
        self.moved[index / 64] & (1 << (index % 64)) != 0
    }

    fn mark_moved_at(&mut self, index: usize) {
        self.moved[index / 64] |= 1 << (index % 64);
    }
}

/// The reference implementation. A fixed rectangle with impassable edges — which is
/// what makes the closed-box conservation test meaningful, and what the chunked world
/// is measured against.
impl CellField for Grid {
    fn get(&self, x: i32, y: i32) -> Option<ElementId> {
        self.index(x, y).map(|index| self.cells[index])
    }

    fn set(&mut self, x: i32, y: i32, id: ElementId) {
        if let Some(index) = self.index(x, y) {
            self.cells[index] = id;
        }
    }

    fn swap(&mut self, ax: i32, ay: i32, bx: i32, by: i32) {
        if let (Some(a), Some(b)) = (self.index(ax, ay), self.index(bx, by)) {
            self.swap_indices(a, b);
        }
    }

    fn is_moved(&self, x: i32, y: i32) -> bool {
        self.index(x, y).is_some_and(|index| self.moved_at(index))
    }

    fn mark_moved(&mut self, x: i32, y: i32) {
        if let Some(index) = self.index(x, y) {
            self.mark_moved_at(index);
        }
    }

    fn clear_moved(&mut self) {
        self.moved.fill(0);
    }

    fn bounds(&self) -> Option<Bounds> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        Some(Bounds {
            min_x: 0,
            min_y: 0,
            max_x: self.width as i32 - 1,
            max_y: self.height as i32 - 1,
        })
    }
}
