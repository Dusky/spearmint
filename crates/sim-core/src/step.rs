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
use crate::grid::Grid;
use crate::rng;

/// Salts keeping a cell's independent decisions from correlating with each other.
const SALT_ROW_DIRECTION: u32 = 1;
const SALT_DIAGONAL: u32 = 2;
const SALT_LATERAL: u32 = 3;

pub fn step(grid: &mut Grid, table: &ElementTable, seed: u64, tick: u64) {
    grid.clear_moved();

    // Bottom row upward, so a particle that falls into an already-visited row cannot be
    // picked up and moved a second time in the same tick.
    for y in (0..grid.height() as i32).rev() {
        // Alternating the scan direction stops material drifting consistently one way,
        // which a fixed left-to-right sweep would cause. The choice is hashed, so it is
        // reproducible.
        let left_to_right = rng::coin_flip(seed, tick, 0, y, SALT_ROW_DIRECTION);

        for column in 0..grid.width() as i32 {
            let x = if left_to_right {
                column
            } else {
                grid.width() as i32 - 1 - column
            };

            let Some(index) = grid.index(x, y) else {
                continue;
            };
            if grid.is_moved(index) {
                continue;
            }

            let id = grid.cells()[index];
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
                State::Powder => step_powder(grid, table, element, x, y, seed, tick),
                State::Liquid => step_liquid(grid, table, element, x, y, seed, tick),
                // No gas element exists yet. Rules arrive with one, not before.
                State::Gas => {}
            }
        }
    }
}

/// Falls straight down, then slides diagonally. It will not flow sideways on the level,
/// which is what makes a powder heap instead of spreading.
fn step_powder(
    grid: &mut Grid,
    table: &ElementTable,
    element: &Element,
    x: i32,
    y: i32,
    seed: u64,
    tick: u64,
) {
    if try_move(grid, table, element, x, y, x, y + 1) {
        return;
    }
    let (first, second) = diagonal_order(seed, tick, x, y);
    if try_move(grid, table, element, x, y, x + first, y + 1) {
        return;
    }
    try_move(grid, table, element, x, y, x + second, y + 1);
}

/// Falls, slides diagonally, and spreads sideways to find its level.
fn step_liquid(
    grid: &mut Grid,
    table: &ElementTable,
    element: &Element,
    x: i32,
    y: i32,
    seed: u64,
    tick: u64,
) {
    if try_move(grid, table, element, x, y, x, y + 1) {
        return;
    }
    let (first, second) = diagonal_order(seed, tick, x, y);
    if try_move(grid, table, element, x, y, x + first, y + 1) {
        return;
    }
    if try_move(grid, table, element, x, y, x + second, y + 1) {
        return;
    }

    // One cell per tick. Slower to level out than a multi-cell scan, and cheaper and
    // simpler to reason about; revisit when flow rate is something the game cares about.
    let lateral = if rng::coin_flip(seed, tick, x, y, SALT_LATERAL) {
        1
    } else {
        -1
    };
    if try_move(grid, table, element, x, y, x + lateral, y) {
        return;
    }
    try_move(grid, table, element, x, y, x - lateral, y);
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
fn try_move(
    grid: &mut Grid,
    table: &ElementTable,
    mover: &Element,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> bool {
    // Outside the grid is immovable boundary, not empty space.
    let Some(to_index) = grid.index(to_x, to_y) else {
        return false;
    };
    let Some(from_index) = grid.index(from_x, from_y) else {
        return false;
    };
    if grid.is_moved(to_index) {
        return false;
    }

    let target = grid.cells()[to_index];
    if target != EMPTY {
        let Some(target_element) = table.get(target) else {
            return false;
        };
        if !displaces(mover, target_element) {
            return false;
        }
    }

    grid.swap(from_index, to_index);
    // Both cells changed hands. Marking each keeps this tick's later visits from
    // moving either one again.
    grid.mark_moved(to_index);
    grid.mark_moved(from_index);
    true
}

/// Whether `mover` can push through `target`.
///
/// Only fluids yield, and only to something denser — which is what makes sand sink
/// through water without a single rule naming either of them.
fn displaces(mover: &Element, target: &Element) -> bool {
    matches!(target.state, State::Liquid | State::Gas) && mover.density > target.density
}
