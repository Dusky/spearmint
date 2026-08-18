//! Drawing, as the player does it.
//!
//! Everything the player builds sits on the tile grid (spec 2.3). Emitters already did:
//! they are tile-anchored and their sprites fill a tile exactly. Walls only appeared to,
//! because a stroke used to snap its two *endpoints* to tile centres and then
//! interpolate between them in cell space — so horizontal and vertical drags landed on
//! the grid by accident, and diagonals laid down a smooth off-grid band.
//!
//! Here a stroke is a line over *tiles*, and every tile it touches is filled completely.
//! A tile is either entirely one element or entirely not, and a diagonal wall comes out
//! as a staircase of whole tiles.
//!
//! This lives in the core rather than in the browser bridge because it mutates the
//! world, and because `sim-wasm` is a `cdylib` that the test binaries cannot link.

use crate::chunk::TILE_CELLS;
use crate::elements::ElementId;
use crate::field::CellField;

/// Paints a line of whole tiles, inclusive of both ends.
///
/// Erasing is the same call with `EMPTY`: half-erasing a wall would reintroduce exactly
/// the off-grid state this exists to prevent.
pub fn stroke<F: CellField + ?Sized>(
    field: &mut F,
    from: (i32, i32),
    to: (i32, i32),
    id: ElementId,
) {
    // Bresenham over tiles. Integer only — the sim has no floats anywhere (spec 3.1).
    let (mut x, mut y) = from;
    let step_x = if from.0 < to.0 { 1 } else { -1 };
    let step_y = if from.1 < to.1 { 1 } else { -1 };
    let run = (to.0 - from.0).abs();
    let rise = -(to.1 - from.1).abs();
    let mut error = run + rise;

    loop {
        tile(field, x, y, id);
        if (x, y) == to {
            break;
        }
        let doubled = 2 * error;
        if doubled >= rise {
            error += rise;
            x += step_x;
        }
        if doubled <= run {
            error += run;
            y += step_y;
        }
    }
}

/// Fills one tile completely.
pub fn tile<F: CellField + ?Sized>(field: &mut F, tile_x: i32, tile_y: i32, id: ElementId) {
    let left = tile_x * TILE_CELLS as i32;
    let top = tile_y * TILE_CELLS as i32;
    for row in 0..TILE_CELLS as i32 {
        for column in 0..TILE_CELLS as i32 {
            let (x, y) = (left + column, top + row);
            field.set(x, y, id);
            // Writing a cell is not a move, so nothing else reports it. Without this a
            // stroke into a settled region leaves it asleep and the new matter hangs
            // there — the same reason `entities::emit` marks what it emits.
            field.mark_active(x, y);
        }
    }
}
