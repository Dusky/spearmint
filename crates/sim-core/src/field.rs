//! The storage interface the physics rules are written against.
//!
//! There are two implementations: the flat `Grid` and the chunked `ChunkMap`. They
//! share this trait so they also share one copy of the rules — which is what makes the
//! equivalence test meaningful. If each had its own tick loop, comparing them would
//! prove the two loops agree, not that chunking is transparent.

use crate::elements::ElementId;

/// An inclusive region of cells to sweep.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl Bounds {
    pub const fn width(&self) -> i64 {
        self.max_x as i64 - self.min_x as i64 + 1
    }

    pub const fn height(&self) -> i64 {
        self.max_y as i64 - self.min_y as i64 + 1
    }
}

pub trait CellField {
    /// The contents of a cell, or `None` where the world does not extend.
    ///
    /// `None` is an impassable boundary, not empty space — a fixed grid has edges, and
    /// nothing may fall through them.
    fn get(&self, x: i32, y: i32) -> Option<ElementId>;

    /// Places an element. Storage that grows on demand allocates here.
    fn set(&mut self, x: i32, y: i32, id: ElementId);

    /// Exchanges two cells. Movement is always a swap, so no rule can create or destroy
    /// a particle (spec 1.1).
    fn swap(&mut self, ax: i32, ay: i32, bx: i32, by: i32);

    fn is_moved(&self, x: i32, y: i32) -> bool;

    fn mark_moved(&mut self, x: i32, y: i32);

    /// Called once before each tick's sweep, to clear per-tick state and work out what
    /// needs simulating.
    fn begin_tick(&mut self);

    /// The region to sweep this tick, or `None` when there is nothing to simulate.
    fn bounds(&self) -> Option<Bounds>;

    /// The x-ranges worth visiting on row `y`, ascending and non-overlapping, appended
    /// to `out`.
    ///
    /// Cells outside them must be *guaranteed* not to move this tick — this is where
    /// storage skips settled regions, and a wrong answer here is a silent divergence
    /// rather than a crash. Storage with nothing to skip returns the whole row.
    fn active_spans(&self, y: i32, out: &mut Vec<(i32, i32)>);
}
