//! Pressure: the weight of the column standing on a cell, and what enough of it does.
//!
//! The counterpart to `heat.rs`, and deliberately its mirror. Heat is a stored quantity
//! that conducts; load is a *derived* one, recomputed every tick from the arrangement of
//! matter alone. That difference is the point — a cell's temperature is history, but its
//! load is nothing but where it is right now, so there is nothing to store, nothing to
//! swap when matter moves, and nothing to add to a save file.
//!
//! Spec 3.3 names pressure among the things reaction yield depends on. This is that
//! factor: burnt residue compacts into fuel under enough overburden, so **depth is the
//! dial**, the same way a kiln's length is the dial for dwell time.
//!
//! Unrelated to `ChunkMap::compact`, which drops empty chunks: that is storage
//! housekeeping and this is a physical rule about matter. The one here is spelled
//! `crush` in code for exactly that reason, and only the data fields keep the domain
//! word — burnt residue does compact into fuel, whatever the allocator calls its own
//! tidying.
//!
//! Load accumulates only through *contiguous* matter. A gap resets it to nothing, which
//! is both the physically honest reading — a shelf with air under it presses on nothing
//! — and what keeps the chunked and flat worlds agreeing: above the topmost matter in
//! any column both are empty, so both start from zero however far up their bounds reach.

use crate::elements::{ElementTable, EMPTY};
use crate::field::CellField;
use crate::fixed::{Fixed, FRACTIONAL_BITS};
use crate::rng;

/// Continues the salt numbering in `step.rs` and `heat.rs` — the salt space is global to
/// `rng`, so a value reused across files would correlate two unrelated draws.
const SALT_COMPACT: u32 = 7;

/// Compacts anything standing under enough weight.
///
/// One top-to-bottom sweep carrying a running load per column, so the whole field costs
/// a single pass rather than a walk up the column at every cell.
pub fn step<F: CellField + ?Sized>(field: &mut F, elements: &ElementTable, seed: u64, tick: u64) {
    let Some(bounds) = field.bounds() else {
        return;
    };

    // One accumulator per column, carried down the sweep. i64 because an unbounded
    // canvas puts no ceiling on how tall a column can get, and `beltStructure`'s density
    // is 999999 — a few thousand cells of it would overflow an i32.
    let width = bounds.width() as usize;
    let mut load: Vec<i64> = vec![0; width];

    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let column = (x - bounds.min_x) as usize;
            let Some(id) = field.get(x, y) else {
                load[column] = 0;
                continue;
            };
            if id == EMPTY {
                load[column] = 0;
                continue;
            }
            let Some(element) = elements.get(id) else {
                load[column] = 0;
                continue;
            };

            // What is already on this cell, before it adds its own weight to the total
            // for whatever is under it.
            crush(field, element, x, y, load[column], seed, tick);

            // Re-read rather than reusing `element`: a cell that just compacted is a
            // denser thing than it was a moment ago, and whatever is under it should
            // feel the weight of what is actually standing there now, not of what used
            // to be. Compacting fuel is denser than the burnt residue it came from, so
            // the difference is real and would otherwise lag by a tick.
            let settled = field.get(x, y).and_then(|id| elements.get(id));
            load[column] =
                load[column].saturating_add(settled.map_or(0, |e| i64::from(e.density)));
        }
    }
}

/// Rolls for one cell under `load`.
///
/// The chance per tick scales with how far past its threshold the load is, tempered by
/// how hard the material is — `hardness`, a field parsed and stored since Milestone 1
/// and read by nothing until now. So a deeper pile compacts *faster* rather than merely
/// compacting at all, which is what makes depth a dial with a range instead of a switch
/// with two positions.
///
/// Deliberately probabilistic, for the same reason burning is: without it, the bottom of
/// a deep enough pile would convert the instant it arrived, and how long material spends
/// under load would mean nothing.
fn crush<F: CellField + ?Sized>(
    field: &mut F,
    element: &crate::elements::Element,
    x: i32,
    y: i32,
    load: i64,
    seed: u64,
    tick: u64,
) {
    if element.compacts_into == EMPTY {
        return;
    }
    let threshold = i64::from(element.compaction_load);
    if threshold <= 0 || load < threshold {
        return;
    }

    // Under load and not yet compacted still counts as something happening here — the
    // same reason `react` and the ignition branch of `heat.rs` mark themselves active.
    // A settled pile moves nothing, so nothing else would keep its chunk awake and the
    // roll would never come round again.
    field.mark_active(x, y);

    // All integer, in Q16.16 raw units, and never through `Fixed::from_int` — a load is
    // tens of thousands and Fixed tops out around 32767, so the ratio is taken first and
    // only the ratio is ever a Fixed quantity.
    //
    // The excess is clamped to the threshold *before* the shift rather than the ratio
    // being clamped after. The two are equivalent, since the ratio saturates at 1.0
    // either way — but `load` is a saturating accumulator, and shifting a saturated i64
    // left by 16 would overflow. Clamping first means the shift is bounded by the
    // threshold, which is only ever an i32.
    let excess = (load - threshold).min(threshold);
    let ratio = (excess << FRACTIONAL_BITS) / threshold;
    let softness = i64::from(Fixed::ONE.raw() - element.hardness.raw()).max(0);
    let chance = (softness * ratio) >> FRACTIONAL_BITS;

    let roll = rng::below(seed, tick, x, y, SALT_COMPACT, 1 << FRACTIONAL_BITS);
    if i64::from(roll) < chance {
        field.set(x, y, element.compacts_into);
    }
}

/// The weight bearing down on one cell: the contiguous column of matter directly above
/// it, by density. For the inspector and for tests — the tick loop accumulates this as
/// it sweeps rather than asking cell by cell.
pub fn load_at<F: CellField + ?Sized>(field: &F, elements: &ElementTable, x: i32, y: i32) -> i64 {
    let Some(bounds) = field.bounds() else {
        return 0;
    };
    let mut total: i64 = 0;
    let mut above = y - 1;
    while above >= bounds.min_y {
        match field.get(x, above).and_then(|id| elements.get(id)) {
            Some(element) => total = total.saturating_add(i64::from(element.density)),
            None => break,
        }
        above -= 1;
    }
    total
}
