//! Sparse chunked storage — the infinite canvas of spec 2.1.
//!
//! Chunks are defined in **tiles**, not pixels (spec 2.4), so tile and chunk alignment
//! can never drift apart. A chunk that has never held anything is not stored at all and
//! reads as empty, which is what makes an unbounded world affordable.
//!
//! **Chunking here is storage, and nothing else.** The tick still sweeps global rows
//! bottom-up across the whole live region, exactly as the flat grid does, so chunk
//! layout cannot influence the result. That is deliberate: it makes this milestone
//! provably transparent, and it isolates the genuinely risky change — sleeping, where
//! what gets visited stops being a function of the world alone — into the next one.
//!
//! Chunks are never surfaced to the player: no grid lines, no counters, no limits.

use std::cell::Cell;
use std::collections::BTreeMap;

use crate::elements::{ElementId, EMPTY};
use crate::field::{Bounds, CellField};

/// Cells per tile edge. Odd on purpose: every tile has a true centre cell, which
/// matters for rotation, symmetry and teleporter endpoints (spec 2.3). Final tile size
/// is still open, so it appears exactly once.
pub const TILE_CELLS: u32 = 9;

/// Tiles per chunk edge (spec 2.4's suggested starting point; tune after profiling).
pub const CHUNK_TILES: u32 = 16;

/// Cells per chunk edge.
pub const CHUNK_CELLS: u32 = TILE_CELLS * CHUNK_TILES;

const CHUNK_AREA: usize = (CHUNK_CELLS * CHUNK_CELLS) as usize;

/// A chunk's position on the infinite canvas, in chunks.
pub type ChunkCoord = (i32, i32);

#[derive(Clone, Debug)]
struct Chunk {
    cells: Vec<ElementId>,
    moved: Vec<u64>,
    /// Something moved in this chunk during the tick now running.
    dirty: bool,
    /// Something moved in it during the previous tick.
    was_dirty: bool,
    /// It is being simulated this tick.
    awake: bool,
}

impl Chunk {
    fn new() -> Chunk {
        Chunk {
            cells: vec![EMPTY; CHUNK_AREA],
            moved: vec![0; CHUNK_AREA.div_ceil(64)],
            // A chunk that has just come into existence has never been simulated, so it
            // must run before it can be trusted to be at rest.
            dirty: true,
            was_dirty: true,
            awake: true,
        }
    }

    fn is_vacant(&self) -> bool {
        self.cells.iter().all(|&cell| cell == EMPTY)
    }
}

/// Chunks keyed by position.
///
/// Storage is a `Vec`, with a `BTreeMap` from position to slot. A `BTreeMap` rather
/// than a hash map because spec 3.1 forbids hash-map iteration anywhere that could
/// reach the simulation, and ordered keys make every traversal reproducible without
/// having to remember to sort.
#[derive(Clone, Debug, Default)]
pub struct ChunkMap {
    chunks: Vec<Chunk>,
    slots: BTreeMap<ChunkCoord, usize>,
    /// Awake chunk columns per chunk row, rebuilt each tick. Ordered, so the sweep
    /// visits them in a reproducible order.
    awake_rows: BTreeMap<i32, Vec<i32>>,
    /// Off by default: every chunk ticks, which is the behaviour the flat reference
    /// world is compared against.
    sleeping: bool,
    /// Chunks within this many chunks of the viewport stay awake regardless (spec 2.4).
    viewport: Option<Bounds>,
    viewport_margin: i32,
    /// The last slot resolved.
    ///
    /// The tick loop walks a row and reads each cell's neighbours, so consecutive
    /// lookups nearly always land in the same chunk. Without this, every cell access is
    /// a tree descent and the chunked world runs roughly an order of magnitude slower
    /// than the flat one. `Cell` because reads resolve slots too, and they take `&self`.
    cache: Cell<Option<(ChunkCoord, usize)>>,
}

/// Splits a world coordinate into a chunk coordinate and an offset inside it.
///
/// Floor division and Euclidean remainder, not truncation — with `/` and `%`, cells at
/// x = -1 and x = 0 would land in the same chunk and the world would fold over on
/// itself at the origin.
#[inline]
pub const fn split(coordinate: i32) -> (i32, u32) {
    let size = CHUNK_CELLS as i32;
    (
        coordinate.div_euclid(size),
        coordinate.rem_euclid(size) as u32,
    )
}

#[inline]
const fn offset(local_x: u32, local_y: u32) -> usize {
    (local_y * CHUNK_CELLS + local_x) as usize
}

impl ChunkMap {
    pub fn new() -> ChunkMap {
        ChunkMap::default()
    }

    /// Enables viewport-gated sleeping (spec 2.4).
    ///
    /// A chunk sleeps only when it is **quiescent**: nothing moved in it or in any
    /// neighbour last tick. That is what makes sleeping free of consequence — ticking a
    /// settled chunk produces no change, so skipping it produces no difference. The
    /// viewport gate layered on top can only keep *more* chunks awake, never fewer, so
    /// it costs CPU and cannot alter the world.
    ///
    /// This is the distinction that separates sleeping from eviction. Eviction discards
    /// state and is therefore observable, and its interaction with replay verification
    /// is still open (spec 2.4). Sleeping is not observable, and does not wait on that.
    pub fn set_sleeping(&mut self, enabled: bool) {
        self.sleeping = enabled;
    }

    /// Where the player is looking, in cells. Anything within `margin_chunks` of it
    /// stays awake.
    pub fn set_viewport(&mut self, viewport: Option<Bounds>, margin_chunks: i32) {
        self.viewport = viewport;
        self.viewport_margin = margin_chunks.max(0);
    }

    /// How many chunks ran this tick. Exposed so a test can prove that sleeping
    /// actually happened rather than passing vacuously.
    pub fn awake_chunk_count(&self) -> usize {
        self.chunks.iter().filter(|chunk| chunk.awake).count()
    }

    fn viewport_chunks(&self) -> Option<(i32, i32, i32, i32)> {
        let viewport = self.viewport?;
        let (min_x, _) = split(viewport.min_x);
        let (min_y, _) = split(viewport.min_y);
        let (max_x, _) = split(viewport.max_x);
        let (max_y, _) = split(viewport.max_y);
        let margin = self.viewport_margin;
        Some((
            min_x - margin,
            min_y - margin,
            max_x + margin,
            max_y + margin,
        ))
    }

    /// Number of chunks currently held. An internal detail, exposed for tests.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunk_coords(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.slots.keys().copied()
    }

    /// Resolves a chunk's slot, remembering it for the next lookup.
    fn slot(&self, coord: ChunkCoord) -> Option<usize> {
        if let Some((cached_coord, slot)) = self.cache.get() {
            if cached_coord == coord {
                return Some(slot);
            }
        }
        let slot = *self.slots.get(&coord)?;
        self.cache.set(Some((coord, slot)));
        Some(slot)
    }

    fn locate(&self, x: i32, y: i32) -> (ChunkCoord, usize) {
        let (chunk_x, local_x) = split(x);
        let (chunk_y, local_y) = split(y);
        ((chunk_x, chunk_y), offset(local_x, local_y))
    }

    /// Resolves a chunk's slot, creating the chunk if it is not there yet.
    fn slot_or_create(&mut self, coord: ChunkCoord) -> usize {
        if let Some(slot) = self.slot(coord) {
            return slot;
        }
        let slot = self.chunks.len();
        self.chunks.push(Chunk::new());
        self.slots.insert(coord, slot);
        self.cache.set(Some((coord, slot)));
        slot
    }

    pub fn count_of(&self, id: ElementId) -> usize {
        if id == EMPTY {
            // Empty space outside every stored chunk is unbounded, so counting it is
            // meaningless. Callers wanting a census should ask about real elements.
            return 0;
        }
        self.chunks
            .iter()
            .map(|chunk| chunk.cells.iter().filter(|&&cell| cell == id).count())
            .sum()
    }

    /// Drops chunks that hold nothing, so a region that empties out stops costing
    /// memory. Allocation is never observable in the world state — reading a missing
    /// chunk and reading an empty one give the same answer.
    pub fn compact(&mut self) {
        let mut kept = Vec::with_capacity(self.chunks.len());
        let mut slots = BTreeMap::new();
        // Rebuild in key order, so the surviving layout depends only on which chunks
        // remain and never on the order they were first touched.
        for (&coord, &slot) in &self.slots {
            if self.chunks[slot].is_vacant() {
                continue;
            }
            slots.insert(coord, kept.len());
            kept.push(self.chunks[slot].clone());
        }
        self.chunks = kept;
        self.slots = slots;
        self.cache.set(None);
    }
}

impl CellField for ChunkMap {
    /// Always `Some`. The canvas is infinite (spec 2.1), so there is no boundary to
    /// report — an unallocated cell is empty space, not a wall.
    fn get(&self, x: i32, y: i32) -> Option<ElementId> {
        let (coord, index) = self.locate(x, y);
        Some(match self.slot(coord) {
            Some(slot) => self.chunks[slot].cells[index],
            None => EMPTY,
        })
    }

    fn set(&mut self, x: i32, y: i32, id: ElementId) {
        let (coord, index) = self.locate(x, y);
        // Writing empty into a chunk that does not exist would allocate a chunk to
        // store nothing.
        if id == EMPTY && self.slot(coord).is_none() {
            return;
        }
        let slot = self.slot_or_create(coord);
        self.chunks[slot].cells[index] = id;
    }

    fn swap(&mut self, ax: i32, ay: i32, bx: i32, by: i32) {
        let (a_coord, a_index) = self.locate(ax, ay);
        let (b_coord, b_index) = self.locate(bx, by);

        if a_coord == b_coord {
            let slot = self.slot_or_create(a_coord);
            self.chunks[slot].cells.swap(a_index, b_index);
            self.chunks[slot].dirty = true;
            return;
        }

        // Across a chunk boundary. Read both, then write both — two chunks cannot be
        // borrowed mutably at once, and going through values keeps that from mattering.
        let a_value = self.get(ax, ay).unwrap_or(EMPTY);
        let b_value = self.get(bx, by).unwrap_or(EMPTY);
        if a_value == b_value {
            return;
        }
        self.set(ax, ay, b_value);
        self.set(bx, by, a_value);
        for coord in [a_coord, b_coord] {
            let slot = self.slot_or_create(coord);
            self.chunks[slot].dirty = true;
        }
    }

    fn mark_active(&mut self, x: i32, y: i32) {
        let (coord, _) = self.locate(x, y);
        let slot = self.slot_or_create(coord);
        self.chunks[slot].dirty = true;
    }

    fn is_moved(&self, x: i32, y: i32) -> bool {
        let (coord, index) = self.locate(x, y);
        self.slot(coord)
            .is_some_and(|slot| self.chunks[slot].moved[index / 64] & (1 << (index % 64)) != 0)
    }

    fn mark_moved(&mut self, x: i32, y: i32) {
        let (coord, index) = self.locate(x, y);
        let slot = self.slot_or_create(coord);
        self.chunks[slot].moved[index / 64] |= 1 << (index % 64);
    }

    /// Clears per-tick state and decides which chunks run.
    ///
    /// A chunk is awake if anything moved in it last tick, or in any of its eight
    /// neighbours — material crossing a boundary has to find the receiving chunk
    /// running. The rule is deliberately conservative: it can only over-simulate, and
    /// over-simulating is invisible, where under-simulating is a silent divergence.
    ///
    /// Why quiescence is sufficient: whether a cell *can* move is a function of its
    /// neighbours' contents alone. Randomness only picks between candidate directions,
    /// and every candidate is tried, so a cell that could not move last tick cannot
    /// move this tick from a different draw. If nothing moved, nothing was marked, and
    /// visit order had no effect either.
    fn begin_tick(&mut self) {
        for chunk in &mut self.chunks {
            chunk.moved.fill(0);
            chunk.was_dirty = chunk.dirty;
            chunk.dirty = false;
        }

        self.awake_rows.clear();

        if !self.sleeping {
            // Everything runs. This is the behaviour the flat reference is measured
            // against, and the default.
            for chunk in &mut self.chunks {
                chunk.awake = true;
            }
            for &(chunk_x, chunk_y) in self.slots.keys() {
                self.awake_rows.entry(chunk_y).or_default().push(chunk_x);
            }
            return;
        }

        let viewport = self.viewport_chunks();
        let dirty: std::collections::BTreeSet<ChunkCoord> = self
            .slots
            .iter()
            .filter(|(_, &slot)| self.chunks[slot].was_dirty)
            .map(|(&coord, _)| coord)
            .collect();

        for (&(chunk_x, chunk_y), &slot) in &self.slots {
            let near_activity =
                (-1..=1).any(|dy| (-1..=1).any(|dx| dirty.contains(&(chunk_x + dx, chunk_y + dy))));
            let in_view = viewport.is_some_and(|(min_x, min_y, max_x, max_y)| {
                chunk_x >= min_x && chunk_x <= max_x && chunk_y >= min_y && chunk_y <= max_y
            });

            let awake = near_activity || in_view;
            self.chunks[slot].awake = awake;
            if awake {
                self.awake_rows.entry(chunk_y).or_default().push(chunk_x);
            }
        }
    }

    /// Awake chunk columns on the chunk row containing `y`, merged into contiguous
    /// cell spans.
    fn active_spans(&self, y: i32, out: &mut Vec<(i32, i32)>) {
        let (chunk_y, _) = split(y);
        let Some(columns) = self.awake_rows.get(&chunk_y) else {
            return;
        };

        let size = CHUNK_CELLS as i32;
        for &chunk_x in columns {
            let start = chunk_x * size;
            let end = start + size - 1;
            match out.last_mut() {
                // Neighbouring chunks make one continuous run; merging them keeps the
                // sweep from restarting every 144 cells.
                Some((_, previous_end)) if *previous_end + 1 == start => *previous_end = end,
                _ => out.push((start, end)),
            }
        }
    }

    /// The bounding box of every stored chunk.
    ///
    /// Sweeping whole chunks means visiting empty cells beyond the material, which
    /// costs a little and changes nothing: an empty cell is skipped without consuming
    /// anything, because randomness is a position hash rather than a stream. That is
    /// precisely why a chunked sweep and a flat one produce identical worlds.
    fn bounds(&self) -> Option<Bounds> {
        let mut coords = self.slots.keys();
        let &(first_x, first_y) = coords.next()?;
        let (mut min_x, mut max_x) = (first_x, first_x);
        let (mut min_y, mut max_y) = (first_y, first_y);
        for &(chunk_x, chunk_y) in coords {
            min_x = min_x.min(chunk_x);
            max_x = max_x.max(chunk_x);
            min_y = min_y.min(chunk_y);
            max_y = max_y.max(chunk_y);
        }
        let size = CHUNK_CELLS as i32;
        Some(Bounds {
            min_x: min_x * size,
            min_y: min_y * size,
            max_x: max_x * size + size - 1,
            max_y: max_y * size + size - 1,
        })
    }
}
