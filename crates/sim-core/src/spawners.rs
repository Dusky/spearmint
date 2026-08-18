//! Particle spawners — the only source of matter, and the game's one hard limit.
//!
//! Spec 3.4 makes spawner count the sole constraint on production: getting more out of
//! the same input is the entire game. Everything else the player builds only rearranges
//! what these emit.
//!
//! Emission is a pure function of seed, tick and position, like everything else in the
//! sim, so a replay reproduces the same input stream exactly.

use crate::elements::{ElementId, EMPTY};
use crate::field::CellField;
use crate::rng;

#[derive(Clone, Copy, Debug)]
pub struct Spawner {
    /// Left edge of the emitting line, in cells.
    pub x: i32,
    pub y: i32,
    /// How many cells wide the emitter is.
    pub width: u32,
    pub element: ElementId,
    /// Cells emitted per tick.
    pub rate: u32,
}

/// Emits from every spawner, in order.
///
/// A spawner cannot force material into an occupied cell — back-pressure is physical,
/// exactly as it is on belts (spec 4.2). A clogged machine starves itself, and nothing
/// special-cases that.
pub fn emit<F: CellField + ?Sized>(field: &mut F, spawners: &[Spawner], seed: u64, tick: u64) {
    for (index, spawner) in spawners.iter().enumerate() {
        if spawner.width == 0 || spawner.element == EMPTY {
            continue;
        }
        for grain in 0..spawner.rate {
            // Salted by spawner and by grain, so two spawners at the same tick do not
            // emit in lockstep and one spawner's grains do not stack on one column.
            let salt = (index as u32)
                .wrapping_mul(31)
                .wrapping_add(grain)
                .wrapping_add(101);
            let offset = rng::below(seed, tick, spawner.x, spawner.y, salt, spawner.width);
            let x = spawner.x + offset as i32;
            if field.get(x, spawner.y) == Some(EMPTY) {
                field.set(x, spawner.y, spawner.element);
                field.mark_active(x, spawner.y);
            }
        }
    }
}
