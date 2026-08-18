//! Cell storage.
//!
//! One byte per cell in a flat `Vec`, which is the shape spec 9 wants at the wasm
//! boundary: a typed array the renderer can view directly without copying per frame.

use crate::elements::{ElementId, EMPTY};

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

    /// `None` outside the grid — callers must decide what the boundary means rather
    /// than silently reading empty space.
    pub fn get(&self, x: i32, y: i32) -> Option<ElementId> {
        self.index(x, y).map(|index| self.cells[index])
    }

    /// Returns false if the position is outside the grid.
    pub fn set(&mut self, x: i32, y: i32, id: ElementId) -> bool {
        match self.index(x, y) {
            Some(index) => {
                self.cells[index] = id;
                true
            }
            None => false,
        }
    }

    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, id: ElementId) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, id);
            }
        }
    }

    pub fn count_of(&self, id: ElementId) -> usize {
        self.cells.iter().filter(|&&cell| cell == id).count()
    }

    /// Exchanges two cells. Movement is *always* a swap and never a write-plus-clear,
    /// so no rule can create or destroy a particle by accident (spec 1.1).
    pub fn swap(&mut self, a: usize, b: usize) {
        self.cells.swap(a, b);
    }

    pub fn is_moved(&self, index: usize) -> bool {
        self.moved[index / 64] & (1 << (index % 64)) != 0
    }

    pub fn mark_moved(&mut self, index: usize) {
        self.moved[index / 64] |= 1 << (index % 64);
    }

    pub fn clear_moved(&mut self) {
        self.moved.fill(0);
    }
}
