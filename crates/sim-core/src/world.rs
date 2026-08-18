//! The simulation state and its public surface.
//!
//! Two worlds exist during Milestone 2a. `World` is the real one, backed by sparse
//! chunks on an infinite canvas. `FlatWorld` is a fixed rectangle, kept as the
//! reference implementation the chunked one is measured against — the equivalence test
//! is what licenses everything built on chunking later, and it needs an oracle.
//!
//! Note what is absent from both: no clock, no file handles, no renderer. Spec 8.3
//! makes the core's isolation a hard architectural requirement, because the same crate
//! has to compile to wasm for the client and to native for server-side replay
//! verification, and both must produce identical results.

use crate::chunk::ChunkMap;
use crate::elements::{ElementId, ElementTable, EMPTY};
use crate::field::CellField;
use crate::grid::Grid;
use crate::hash::Hasher;
use crate::step;

/// Hashes the contents of a field, keyed by absolute position.
///
/// Canonical rather than structural: it walks global rows and skips empty cells, so two
/// worlds holding the same material agree regardless of how that material is stored or
/// which chunks happen to be allocated. This is what makes a flat world and a chunked
/// one directly comparable.
fn canonical_hash<F: CellField + ?Sized>(field: &F, tick: u64, seed: u64) -> u64 {
    let mut hasher = Hasher::new();
    hasher.write_u64(tick);
    hasher.write_u64(seed);

    if let Some(bounds) = field.bounds() {
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let id = field.get(x, y).unwrap_or(EMPTY);
                if id == EMPTY {
                    continue;
                }
                hasher.write_u32(x as u32);
                hasher.write_u32(y as u32);
                hasher.write_u8(id);
            }
        }
    }
    hasher.finish()
}

/// The simulated world: sparse chunks, unbounded extent (spec 2.1).
#[derive(Clone, Debug)]
pub struct World {
    field: ChunkMap,
    table: ElementTable,
    seed: u64,
    tick: u64,
}

impl World {
    pub fn new(seed: u64, table: ElementTable) -> World {
        World {
            field: ChunkMap::new(),
            table,
            seed,
            tick: 0,
        }
    }

    /// Advances exactly one tick.
    ///
    /// The fixed timestep lives in the host, not here (spec 3.1): the client runs an
    /// accumulator so ticks stay decoupled from render frames, and the server runs this
    /// as fast as it likes when verifying a replay.
    pub fn step(&mut self) {
        step::step(&mut self.field, &self.table, self.seed, self.tick);
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

    pub const fn table(&self) -> &ElementTable {
        &self.table
    }

    pub const fn field(&self) -> &ChunkMap {
        &self.field
    }

    pub fn field_mut(&mut self) -> &mut ChunkMap {
        &mut self.field
    }

    pub fn get(&self, x: i32, y: i32) -> ElementId {
        self.field.get(x, y).unwrap_or(EMPTY)
    }

    pub fn count_of(&self, id: ElementId) -> usize {
        self.field.count_of(id)
    }

    /// A fingerprint of the whole world, and the shape the signed checkpoints of spec
    /// 8.3 will take.
    pub fn hash(&self) -> u64 {
        canonical_hash(&self.field, self.tick, self.seed)
    }
}

/// The reference implementation: a fixed rectangle with impassable edges.
///
/// Retained as a test oracle. Its bounded edges are also what make the closed-box
/// conservation test meaningful — on an infinite canvas, "nothing escaped" needs
/// somewhere for things to escape to.
#[derive(Clone, Debug)]
pub struct FlatWorld {
    field: Grid,
    table: ElementTable,
    seed: u64,
    tick: u64,
}

impl FlatWorld {
    pub fn new(width: u32, height: u32, seed: u64, table: ElementTable) -> FlatWorld {
        FlatWorld {
            field: Grid::new(width, height),
            table,
            seed,
            tick: 0,
        }
    }

    pub fn step(&mut self) {
        step::step(&mut self.field, &self.table, self.seed, self.tick);
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

    pub const fn width(&self) -> u32 {
        self.field.width()
    }

    pub const fn height(&self) -> u32 {
        self.field.height()
    }

    pub const fn field(&self) -> &Grid {
        &self.field
    }

    pub fn field_mut(&mut self) -> &mut Grid {
        &mut self.field
    }

    pub fn get(&self, x: i32, y: i32) -> ElementId {
        self.field.get(x, y).unwrap_or(EMPTY)
    }

    /// The raw cell bytes. This is the view the renderer will upload as a texture.
    pub fn cells(&self) -> &[ElementId] {
        self.field.cells()
    }

    pub fn count_of(&self, id: ElementId) -> usize {
        self.field.count_of(id)
    }

    /// Comparable with `World::hash` — same canonical form, so the two can be compared
    /// directly despite storing their cells completely differently.
    pub fn hash(&self) -> u64 {
        canonical_hash(&self.field, self.tick, self.seed)
    }

    /// The original Milestone 1 fingerprint: every cell byte in storage order, empties
    /// included.
    ///
    /// Kept because the golden hash is pinned against it. `hash` had to become
    /// canonical so a flat world and a chunked one could be compared, and re-pinning
    /// the golden at the same time as changing the rules' plumbing would have thrown
    /// away the one check that the plumbing change altered no behaviour.
    pub fn structural_hash(&self) -> u64 {
        let mut hasher = Hasher::new();
        hasher.write_u32(self.field.width());
        hasher.write_u32(self.field.height());
        hasher.write_u64(self.tick);
        hasher.write_u64(self.seed);
        hasher.write_bytes(self.field.cells());
        hasher.finish()
    }
}
