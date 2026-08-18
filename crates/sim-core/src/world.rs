//! The simulation state and its public surface.

use crate::elements::{ElementId, ElementTable};
use crate::grid::Grid;
use crate::hash::Hasher;
use crate::step;

/// A simulated region and everything needed to advance it.
///
/// Note what is absent: no clock, no file handles, no renderer, no allocator tricks.
/// Spec 8.3 makes the core's isolation a hard architectural requirement, because the
/// same crate has to compile to wasm for the client and to native for server-side
/// replay verification, and both must produce identical results.
#[derive(Clone, Debug)]
pub struct World {
    grid: Grid,
    table: ElementTable,
    seed: u64,
    tick: u64,
}

impl World {
    pub fn new(width: u32, height: u32, seed: u64, table: ElementTable) -> World {
        World {
            grid: Grid::new(width, height),
            table,
            seed,
            tick: 0,
        }
    }

    /// Advances exactly one tick.
    ///
    /// The fixed timestep lives in the host, not here (spec 3.1): the client runs an
    /// accumulator so ticks stay decoupled from render frames, and the server runs this
    /// as fast as it likes when verifying a replay. The core just knows how to take one
    /// step.
    pub fn step(&mut self) {
        step::step(&mut self.grid, &self.table, self.seed, self.tick);
        self.tick += 1;
    }

    pub fn step_many(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step();
        }
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub const fn width(&self) -> u32 {
        self.grid.width()
    }

    pub const fn height(&self) -> u32 {
        self.grid.height()
    }

    pub const fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
    }

    pub const fn table(&self) -> &ElementTable {
        &self.table
    }

    /// The raw cell bytes. This is the view the renderer will upload as a texture.
    pub fn cells(&self) -> &[ElementId] {
        self.grid.cells()
    }

    pub fn count_of(&self, id: ElementId) -> usize {
        self.grid.count_of(id)
    }

    /// A fingerprint of the entire world state.
    ///
    /// This is what the determinism tests compare, and the shape the signed checkpoints
    /// of spec 8.3 will take.
    pub fn hash(&self) -> u64 {
        let mut hasher = Hasher::new();
        hasher.write_u32(self.grid.width());
        hasher.write_u32(self.grid.height());
        hasher.write_u64(self.tick);
        hasher.write_u64(self.seed);
        hasher.write_bytes(self.grid.cells());
        hasher.finish()
    }
}
