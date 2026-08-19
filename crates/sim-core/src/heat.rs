//! Temperature: a real per-cell quantity that conducts between touching matter and can
//! carry an element across its melting or boiling point (spec 5.3).
//!
//! Deliberately separate from `step.rs`'s movement sweep. That sweep's `is_moved`
//! bookkeeping exists to stop a particle being carried along a second time in one
//! tick — a concern specific to movement, and unrelated to heat. Keeping this a
//! distinct pass, run once per tick, means neither has to reason about the other.
//!
//! Heat only exists where matter does: it does not conduct through empty space, and an
//! empty cell never reads as anything but ambient. That sidesteps diffusing across the
//! unbounded empty canvas (spec 2.1), and it is also the more physically honest
//! reading of "conductivity" — two things exchange heat by touching, not by sharing a
//! void.

use crate::elements::{ElementTable, EMPTY};
use crate::field::CellField;
use crate::fixed::Fixed;

/// Room temperature, in whole Kelvin. What any cell reads as until something heats or
/// cools it, including storage that has never been allocated.
pub const AMBIENT_TEMPERATURE: i16 = 293;

/// Conducts heat between touching matter, then applies any melting or boiling
/// transition the result crosses.
///
/// Walks `field.bounds()` the same way `world.rs`'s hashing already does — the whole
/// stored region, empty cells included, skipped cheaply rather than tracked separately.
/// For each occupied cell, exchanges heat with its right and lower neighbour if that
/// neighbour is also occupied: the same "each adjacent pair considered exactly once"
/// contact pattern `step.rs`'s `react` uses, for the same reason.
pub fn step<F: CellField + ?Sized>(field: &mut F, elements: &ElementTable) {
    let Some(bounds) = field.bounds() else {
        return;
    };

    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let Some(id) = field.get(x, y) else { continue };
            if id == EMPTY {
                continue;
            }

            for (dx, dy) in [(1, 0), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                let Some(neighbour_id) = field.get(nx, ny) else { continue };
                if neighbour_id == EMPTY {
                    continue;
                }
                exchange(field, elements, x, y, nx, ny);
            }

            // Re-read: the exchanges above may just have changed this cell's own
            // temperature, and that is exactly the value a transition checks against.
            transition(field, elements, x, y);
        }
    }
}

/// Moves heat between two touching cells, proportional to their temperature
/// difference and however well the *worse* conductor of the pair carries it — an
/// insulating neighbour blocks heat even when the other side conducts perfectly, which
/// is what makes an insulator actually insulate rather than just slowing things down.
fn exchange<F: CellField + ?Sized>(
    field: &mut F,
    elements: &ElementTable,
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
) {
    let (Some(a_element), Some(b_element)) = (
        field.get(ax, ay).and_then(|id| elements.get(id)),
        field.get(bx, by).and_then(|id| elements.get(id)),
    ) else {
        return;
    };

    let a_temp = field.temperature(ax, ay);
    let b_temp = field.temperature(bx, by);
    if a_temp == b_temp {
        return;
    }

    let (hot_temp, cold_temp, hot_is_a) = if a_temp > b_temp {
        (a_temp, b_temp, true)
    } else {
        (b_temp, a_temp, false)
    };
    let diff = i32::from(hot_temp) - i32::from(cold_temp);

    let conductivity = a_element.thermal_conductivity.min(b_element.thermal_conductivity);
    // Floored, and clamped to *half* the difference — the two cells meet in the middle
    // at equilibrium, so half is the most that can ever move between them. Clamping to
    // the whole difference would let a good conductor (gold's is 3.17, well above 1.0)
    // swap the two temperatures outright and flip which side is hotter.
    let transferred = Fixed::from_int(diff)
        .saturating_mul(conductivity)
        .floor_to_int()
        .clamp(0, diff / 2);
    if transferred == 0 {
        return;
    }

    let new_hot = hot_temp - transferred as i16;
    let new_cold = cold_temp + transferred as i16;
    let (new_a, new_b) = if hot_is_a { (new_hot, new_cold) } else { (new_cold, new_hot) };
    field.set_temperature(ax, ay, new_a);
    field.set_temperature(bx, by, new_b);
    field.mark_active(ax, ay);
    field.mark_active(bx, by);
}

/// Boiling first: a substance hot enough to boil already crossed its melting point on
/// the way there, so the more-transformed transition is the one that should win.
fn transition<F: CellField + ?Sized>(field: &mut F, elements: &ElementTable, x: i32, y: i32) {
    let Some(id) = field.get(x, y) else { return };
    let Some(element) = elements.get(id) else { return };
    let temperature = i32::from(field.temperature(x, y));

    let target = if element.boils_into != EMPTY && temperature >= element.boiling_point {
        element.boils_into
    } else if element.melts_into != EMPTY && temperature >= element.melting_point {
        element.melts_into
    } else {
        return;
    };

    // A change of identity in place, not a move — the temperature already sitting here
    // is the correct temperature for whatever this becomes, so nothing else to update.
    field.set(x, y, target);
    field.mark_active(x, y);
}
