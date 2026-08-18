//! One tick of the simulation.
//!
//! Rules dispatch on an element's `state`, never on its identity. Spec 3.2 requires
//! that adding an element means editing JSON rather than writing a match arm, and that
//! only holds if the physics is written against powders and liquids rather than against
//! sand and water.
//!
//! Determinism comes from two properties, both structural rather than incidental:
//! the scan order is fixed, and every random choice is a pure hash of position and tick
//! (see `rng`), so nothing depends on how many cells were visited beforehand.

use crate::elements::{Element, ElementTable, State, EMPTY};
use crate::field::CellField;
use crate::rng;

/// Salts keeping a cell's independent decisions from correlating with each other.
const SALT_ROW_DIRECTION: u32 = 1;
const SALT_DIAGONAL: u32 = 2;
const SALT_LATERAL: u32 = 3;

/// Advances one tick over any storage.
///
/// Generic so the flat grid and the chunked map run this exact code rather than two
/// copies of it — otherwise comparing them would only prove that two tick loops agree,
/// not that chunking is transparent.
pub fn step<F: CellField + ?Sized>(field: &mut F, table: &ElementTable, seed: u64, tick: u64) {
    field.clear_moved();
    let Some(bounds) = field.bounds() else {
        return;
    };

    // Bottom row upward, so a particle that falls into an already-visited row cannot be
    // picked up and moved a second time in the same tick.
    for y in (bounds.min_y..=bounds.max_y).rev() {
        // Alternating the scan direction stops material drifting consistently one way,
        // which a fixed left-to-right sweep would cause. The choice is hashed on the row
        // alone, so it does not depend on how wide the swept region happens to be.
        let left_to_right = rng::coin_flip(seed, tick, 0, y, SALT_ROW_DIRECTION);

        for column in 0..bounds.width() as i32 {
            let x = if left_to_right {
                bounds.min_x + column
            } else {
                bounds.max_x - column
            };

            if field.is_moved(x, y) {
                continue;
            }

            let Some(id) = field.get(x, y) else {
                continue;
            };
            if id == EMPTY {
                continue;
            }
            // An id with no definition is left alone rather than assumed inert — it
            // means the data file and the world disagree, which is worth noticing.
            let Some(element) = table.get(id) else {
                continue;
            };

            match element.state {
                State::Solid => {}
                State::Powder => step_powder(field, table, element, x, y, seed, tick),
                State::Liquid => step_liquid(field, table, element, x, y, seed, tick),
                // No gas element exists yet. Rules arrive with one, not before.
                State::Gas => {}
            }
        }
    }
}

/// Falls straight down, then slides diagonally. It will not flow sideways on the level,
/// which is what makes a powder heap instead of spreading.
fn step_powder<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    element: &Element,
    x: i32,
    y: i32,
    seed: u64,
    tick: u64,
) {
    if try_move(field, table, element, x, y, x, y + 1) {
        return;
    }
    let (first, second) = diagonal_order(seed, tick, x, y);
    if try_move(field, table, element, x, y, x + first, y + 1) {
        return;
    }
    try_move(field, table, element, x, y, x + second, y + 1);
}

/// Falls, slides diagonally, and spreads sideways to find its level.
fn step_liquid<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    element: &Element,
    x: i32,
    y: i32,
    seed: u64,
    tick: u64,
) {
    if try_move(field, table, element, x, y, x, y + 1) {
        return;
    }
    let (first, second) = diagonal_order(seed, tick, x, y);
    if try_move(field, table, element, x, y, x + first, y + 1) {
        return;
    }
    if try_move(field, table, element, x, y, x + second, y + 1) {
        return;
    }

    // One cell per tick. Slower to level out than a multi-cell scan, and cheaper and
    // simpler to reason about; revisit when flow rate is something the game cares about.
    let lateral = if rng::coin_flip(seed, tick, x, y, SALT_LATERAL) {
        1
    } else {
        -1
    };
    if try_move(field, table, element, x, y, x + lateral, y) {
        return;
    }
    try_move(field, table, element, x, y, x - lateral, y);
}

fn diagonal_order(seed: u64, tick: u64, x: i32, y: i32) -> (i32, i32) {
    if rng::coin_flip(seed, tick, x, y, SALT_DIAGONAL) {
        (1, -1)
    } else {
        (-1, 1)
    }
}

/// Moves the contents of one cell into another if the target yields, and reports
/// whether it happened.
fn try_move<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    mover: &Element,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> bool {
    // `None` is impassable boundary, not empty space — a fixed grid has edges.
    let Some(target) = field.get(to_x, to_y) else {
        return false;
    };
    if field.get(from_x, from_y).is_none() {
        return false;
    }
    if field.is_moved(to_x, to_y) {
        return false;
    }

    if target != EMPTY {
        let Some(target_element) = table.get(target) else {
            return false;
        };
        if !displaces(mover, target_element) {
            return false;
        }
    }

    field.swap(from_x, from_y, to_x, to_y);
    // Both cells changed hands. Marking each keeps this tick's later visits from
    // moving either one again.
    field.mark_moved(to_x, to_y);
    field.mark_moved(from_x, from_y);
    true
}

/// Whether `mover` can push through `target`.
///
/// Only fluids yield, and only to something denser — which is what makes sand sink
/// through water without a single rule naming either of them.
fn displaces(mover: &Element, target: &Element) -> bool {
    matches!(target.state, State::Liquid | State::Gas) && mover.density > target.density
}
