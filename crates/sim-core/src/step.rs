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
use crate::reactions::ReactionTable;
use crate::rng;

/// Salts keeping a cell's independent decisions from correlating with each other.
const SALT_ROW_DIRECTION: u32 = 1;
const SALT_DIAGONAL: u32 = 2;
const SALT_LATERAL: u32 = 3;
const SALT_REACT_RIGHT: u32 = 4;
const SALT_REACT_DOWN: u32 = 5;

/// Advances one tick over any storage.
///
/// Generic so the flat grid and the chunked map run this exact code rather than two
/// copies of it — otherwise comparing them would only prove that two tick loops agree,
/// not that chunking is transparent.
pub fn step<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    reactions: &ReactionTable,
    seed: u64,
    tick: u64,
) {
    field.begin_tick();
    let Some(bounds) = field.bounds() else {
        return;
    };

    let mut spans: Vec<(i32, i32)> = Vec::new();

    // Bottom row upward, so a particle that falls into an already-visited row cannot be
    // picked up and moved a second time in the same tick.
    for y in (bounds.min_y..=bounds.max_y).rev() {
        // Alternating the scan direction stops material drifting consistently one way,
        // which a fixed left-to-right sweep would cause. The choice is hashed on the row
        // alone, so it does not depend on how much of the row is being visited.
        let left_to_right = rng::coin_flip(seed, tick, 0, y, SALT_ROW_DIRECTION);

        spans.clear();
        field.active_spans(y, &mut spans);
        if spans.is_empty() {
            continue;
        }

        // Spans are ascending and disjoint, so walking them in order — or in reverse,
        // reversed within each — visits exactly the cells a full sweep would, in the
        // same relative order. Skipped cells cannot move, and cost nothing to skip:
        // randomness is a position hash, so passing over a cell consumes nothing.
        for span_index in 0..spans.len() {
            let (span_min, span_max) = if left_to_right {
                spans[span_index]
            } else {
                spans[spans.len() - 1 - span_index]
            };

            for offset in 0..=(span_max - span_min) {
                let x = if left_to_right {
                    span_min + offset
                } else {
                    span_max - offset
                };

                if field.is_moved(x, y) {
                    continue;
                }

                let Some(mut id) = field.get(x, y) else {
                    continue;
                };
                if id == EMPTY {
                    continue;
                }

                // Reactions first, so a cell that changes still moves as what it became.
                if !reactions.is_empty() {
                    id = react(field, reactions, x, y, id, seed, tick);
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
                    State::Gas => step_gas(field, table, element, x, y, seed, tick),
                }
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
    if try_move(field, table, element, x, y, x, y + 1, displaces) {
        return;
    }
    let (first, second) = diagonal_order(seed, tick, x, y);
    if try_move(field, table, element, x, y, x + first, y + 1, displaces) {
        return;
    }
    try_move(field, table, element, x, y, x + second, y + 1, displaces);
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
    if try_move(field, table, element, x, y, x, y + 1, displaces) {
        return;
    }
    let (first, second) = diagonal_order(seed, tick, x, y);
    if try_move(field, table, element, x, y, x + first, y + 1, displaces) {
        return;
    }
    if try_move(field, table, element, x, y, x + second, y + 1, displaces) {
        return;
    }

    // Submerged liquid does not flow sideways.
    //
    // Without this, a cell deep inside a body of water still shuffles laterally, and
    // every such move leaves a void behind. Gravity refills voids from above at the
    // same rate they are created, so the body settles at about three-quarters density
    // forever instead of packing. Restricting lateral flow to the surface lets the bulk
    // fill in, and leaves spreading — which only the surface does anyway — intact.
    if field
        .get(x, y - 1)
        .and_then(|above| table.get(above))
        .is_some_and(|above| matches!(above.state, State::Liquid))
    {
        return;
    }

    // Surface liquid disperses several cells per tick rather than one.
    //
    // At one cell per tick a pour takes as long to spread as the tank is wide, and the
    // surface visibly lags behind where it should be. Scanning outward and taking the
    // furthest clear cell is how the genre gets liquid to find its level promptly.
    let first = if rng::coin_flip(seed, tick, x, y, SALT_LATERAL) {
        1
    } else {
        -1
    };
    for direction in [first, -first] {
        if let Some(target) = furthest_clear(field, table, element, x, y, direction, 1, displaces) {
            if try_move(field, table, element, x, y, target, y, displaces) {
                return;
            }
        }
    }
}

/// Rises, slips diagonally upward, and spreads along whatever is holding it down.
///
/// A liquid turned upside down, deliberately: a gas is a fluid that falls the other way,
/// and writing it as the mirror keeps the two honest about being one idea. The surface
/// rule mirrors too — a gas with more gas directly above it is "submerged" from its own
/// point of view and does not flow sideways, which is what lets a cloud pack against a
/// ceiling instead of churning under it forever.
fn step_gas<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    element: &Element,
    x: i32,
    y: i32,
    seed: u64,
    tick: u64,
) {
    if try_move(field, table, element, x, y, x, y - 1, rises_through) {
        return;
    }
    let (first, second) = diagonal_order(seed, tick, x, y);
    for dx in [first, second] {
        if try_move(field, table, element, x, y, x + dx, y - 1, rises_through) {
            return;
        }
    }

    if field
        .get(x, y + 1)
        .and_then(|below| table.get(below))
        .is_some_and(|below| matches!(below.state, State::Gas))
    {
        return;
    }

    let first = if rng::coin_flip(seed, tick, x, y, SALT_LATERAL) {
        1
    } else {
        -1
    };
    for direction in [first, -first] {
        if let Some(target) =
            furthest_clear(field, table, element, x, y, direction, -1, rises_through)
        {
            if try_move(field, table, element, x, y, target, y, rises_through) {
                return;
            }
        }
    }
}

/// How far a fluid may travel sideways in one tick.
const DISPERSION: i32 = 5;

/// Scans outward and reports the furthest cell the fluid could occupy, stopping early at
/// the first place it could leave the row from — water should drop into a gap rather
/// than run past it, and a gas should escape up a chimney rather than run past that.
///
/// `along` is the way this fluid travels vertically: `1` for a liquid, `-1` for a gas.
#[allow(clippy::too_many_arguments)]
fn furthest_clear<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    element: &Element,
    x: i32,
    y: i32,
    direction: i32,
    along: i32,
    yields: Yields,
) -> Option<i32> {
    let mut furthest = None;
    for step in 1..=DISPERSION {
        let candidate = x + direction * step;
        if !can_occupy(field, table, element, candidate, y, yields) {
            break;
        }
        furthest = Some(candidate);
        // Somewhere to go: stop here rather than running past the gap.
        if can_occupy(field, table, element, candidate, y + along, yields) {
            break;
        }
    }
    furthest
}

/// Whether `element` could move into this cell — empty, or a fluid that yields to it.
fn can_occupy<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    element: &Element,
    x: i32,
    y: i32,
    yields: Yields,
) -> bool {
    let Some(occupant) = field.get(x, y) else {
        return false;
    };
    if occupant == EMPTY {
        return true;
    }
    table
        .get(occupant)
        .is_some_and(|target| yields(element, target))
}

fn diagonal_order(seed: u64, tick: u64, x: i32, y: i32) -> (i32, i32) {
    if rng::coin_flip(seed, tick, x, y, SALT_DIAGONAL) {
        (1, -1)
    } else {
        (-1, 1)
    }
}

/// What counts as a cell yielding to something moving into it.
///
/// Passed in rather than baked into `try_move` because sinking and rising are mirror
/// images of one rule — the denser of two fluids ends up lower — and which comparison
/// says so depends on which way the mover is going.
type Yields = fn(&Element, &Element) -> bool;

/// Moves the contents of one cell into another if the target yields, and reports
/// whether it happened.
#[allow(clippy::too_many_arguments)]
fn try_move<F: CellField + ?Sized>(
    field: &mut F,
    table: &ElementTable,
    mover: &Element,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    yields: Yields,
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
        if !yields(mover, target_element) {
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
/// Fluids yield to anything denser, which is what makes sand sink through water without
/// a single rule naming either of them. Powders yield the same way to a denser powder —
/// gold settling to the bottom of a mixed pile is this rule, not a rule about gold.
fn displaces(mover: &Element, target: &Element) -> bool {
    matches!(target.state, State::Liquid | State::Gas | State::Powder)
        && mover.density > target.density
}

/// Whether `mover` can rise through `target`.
///
/// The same rule as `displaces` seen from underneath: of two fluids, the denser ends up
/// lower. Sinking asks whether the mover is heavier than what is beneath it; rising asks
/// whether it is lighter than what is above. Steam under water needs the second, and
/// `displaces` answers it backwards — which would leave a gas trapped wherever it was
/// made.
fn rises_through(mover: &Element, target: &Element) -> bool {
    matches!(target.state, State::Liquid | State::Gas | State::Powder)
        && mover.density < target.density
}

/// Reacts a cell with its right and lower neighbours, returning what it ended up as.
///
/// Only two of the four neighbours are checked, so each adjacent pair is considered
/// exactly once per tick rather than twice.
///
/// Note what is *not* here: no temperature gate, no catalyst, no rate table. Yield is
/// whatever the geometry produces, because a reaction can only happen where reactants
/// actually touch (spec 3.3). Contact area is not a parameter — it is the mechanism.
fn react<F: CellField + ?Sized>(
    field: &mut F,
    reactions: &ReactionTable,
    x: i32,
    y: i32,
    id: crate::elements::ElementId,
    seed: u64,
    tick: u64,
) -> crate::elements::ElementId {
    let mut current = id;

    for (dx, dy, salt) in [(1, 0, SALT_REACT_RIGHT), (0, 1, SALT_REACT_DOWN)] {
        let (nx, ny) = (x + dx, y + dy);
        let Some(neighbour) = field.get(nx, ny) else {
            continue;
        };
        if neighbour == EMPTY {
            continue;
        }
        let Some((reaction, flipped)) = reactions.between(current, neighbour) else {
            continue;
        };

        // Reactants in contact make this cell chemically active even if nothing moves,
        // which has to keep its chunk awake or the reaction would never get another
        // chance to fire.
        field.mark_active(x, y);
        field.mark_active(nx, ny);

        // Probability is Q16.16, so the draw is out of 65536.
        let roll = rng::below(seed, tick, x, y, salt, 1 << 16);
        if i64::from(roll) >= i64::from(reaction.probability.raw()) {
            continue;
        }

        let (here, there) = if flipped {
            (reaction.products[1], reaction.products[0])
        } else {
            (reaction.products[0], reaction.products[1])
        };
        field.set(x, y, here);
        field.set(nx, ny, there);
        current = here;
    }

    current
}
