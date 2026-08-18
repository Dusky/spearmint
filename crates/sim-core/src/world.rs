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
use crate::elements::{ElementId, EMPTY};
use crate::entities::{self, Entity, EntityKind};
use crate::field::{Bounds, CellField};
use crate::grid::Grid;
use crate::hash::Hasher;
use crate::rules::Rules;
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
    content_hash_into(&mut hasher, field);
    hasher.finish()
}

/// Hashes cell contents alone — no tick, no seed.
///
/// `canonical_hash` folds in the tick, which is right for a checkpoint but useless for
/// asking "did anything change?", since it differs every tick by construction. That
/// distinction is easy to miss and reduces such a test to measuring the clock.
fn content_hash<F: CellField + ?Sized>(field: &F) -> u64 {
    let mut hasher = Hasher::new();
    content_hash_into(&mut hasher, field);
    hasher.finish()
}

fn content_hash_into<F: CellField + ?Sized>(hasher: &mut Hasher, field: &F) {
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
}

/// The simulated world: sparse chunks, unbounded extent (spec 2.1).
#[derive(Clone, Debug)]
pub struct World {
    field: ChunkMap,
    rules: Rules,
    entities: Vec<Entity>,
    seed: u64,
    tick: u64,
}

impl World {
    pub fn new(seed: u64, rules: Rules) -> World {
        World {
            field: ChunkMap::new(),
            rules,
            entities: Vec::new(),
            seed,
            tick: 0,
        }
    }

    /// Places a machine. Counting them against a cap is the economy's job, not the
    /// sim's (spec 3.4).
    pub fn place(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// The machine covering this tile, if any.
    pub fn entity_at(&self, tile_x: i32, tile_y: i32) -> Option<usize> {
        self.entities.iter().position(|entity| {
            self.rules
                .entities
                .get(entity.kind)
                .is_some_and(|definition| entity.covers(definition, tile_x, tile_y))
        })
    }

    /// Removes a machine, freeing its slot completely.
    ///
    /// Placement is never a purchase that can be wasted: gold buys *capacity* (spec
    /// 3.4), and where a machine sits within that capacity is a layout decision the
    /// player can take back at no cost. Charging for a misplacement would make players
    /// hoard slots rather than experiment, which is backwards for a game whose loop is
    /// iterating on machine geometry — and without removal, spending every slot on one
    /// material soft-locks the run.
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.entities.len() {
            return false;
        }
        self.entities.remove(index);
        true
    }

    /// How many machines of a kind are placed, for a cap to be enforced against.
    pub fn count_of_kind(&self, kind: EntityKind) -> usize {
        self.entities
            .iter()
            .filter(|entity| entity.kind == kind)
            .count()
    }

    pub fn clear_entities(&mut self) {
        self.entities.clear();
    }

    /// Advances exactly one tick.
    ///
    /// The fixed timestep lives in the host, not here (spec 3.1): the client runs an
    /// accumulator so ticks stay decoupled from render frames, and the server runs this
    /// as fast as it likes when verifying a replay.
    pub fn step(&mut self) {
        // Machines act first, then everything moves. Emitters are the only source of
        // matter (spec 3.4), so this is the whole input side of the game.
        entities::tick(
            &mut self.field,
            &self.entities,
            &self.rules.entities,
            &self.rules.elements,
            self.seed,
            self.tick,
        );
        step::step(
            &mut self.field,
            &self.rules.elements,
            &self.rules.reactions,
            self.seed,
            self.tick,
        );
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

    pub const fn rules(&self) -> &Rules {
        &self.rules
    }

    pub const fn field(&self) -> &ChunkMap {
        &self.field
    }

    pub fn field_mut(&mut self) -> &mut ChunkMap {
        &mut self.field
    }

    /// Enables viewport-gated sleeping (spec 2.4). Off by default.
    pub fn set_sleeping(&mut self, enabled: bool) {
        self.field.set_sleeping(enabled);
    }

    /// Where the player is looking, in cells, plus how many chunks beyond it to keep
    /// awake.
    pub fn set_viewport(&mut self, viewport: Option<Bounds>, margin_chunks: i32) {
        self.field.set_viewport(viewport, margin_chunks);
    }

    /// How many chunks ran during the last tick.
    pub fn awake_chunk_count(&self) -> usize {
        self.field.awake_chunk_count()
    }

    pub fn get(&self, x: i32, y: i32) -> ElementId {
        self.field.get(x, y).unwrap_or(EMPTY)
    }

    pub fn count_of(&self, id: ElementId) -> usize {
        self.field.count_of(id)
    }

    /// A fingerprint of the whole world, and the shape the signed checkpoints of spec
    /// 8.3 will take. Includes the tick, so it differs every tick even at rest.
    pub fn hash(&self) -> u64 {
        canonical_hash(&self.field, self.tick, self.seed)
    }

    /// A fingerprint of the cell contents alone. Use this to ask whether anything
    /// actually moved; `hash` cannot answer that, since it folds in the tick.
    pub fn content_hash(&self) -> u64 {
        content_hash(&self.field)
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
    rules: Rules,
    entities: Vec<Entity>,
    seed: u64,
    tick: u64,
}

impl FlatWorld {
    pub fn new(width: u32, height: u32, seed: u64, rules: Rules) -> FlatWorld {
        FlatWorld {
            field: Grid::new(width, height),
            rules,
            entities: Vec::new(),
            seed,
            tick: 0,
        }
    }

    pub fn place(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn step(&mut self) {
        entities::tick(
            &mut self.field,
            &self.entities,
            &self.rules.entities,
            &self.rules.elements,
            self.seed,
            self.tick,
        );
        step::step(
            &mut self.field,
            &self.rules.elements,
            &self.rules.reactions,
            self.seed,
            self.tick,
        );
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

    /// Cell contents alone, ignoring the tick. See `World::content_hash`.
    pub fn content_hash(&self) -> u64 {
        content_hash(&self.field)
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
