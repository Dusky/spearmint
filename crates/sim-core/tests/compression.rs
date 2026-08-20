//! Compacting: the second thing in the factory that reads physics instead of a machine
//! (spec 3.3's pressure, after temperature).
//!
//! Burnt residue has no machine that converts it any more. It compacts under the weight
//! of whatever is standing on it, and `hardness` decides how readily — so **depth is the
//! dial**, the way a kiln's length is the dial for dwell.
//!
//! Load is derived, never stored: recomputed every tick from the arrangement of matter
//! alone. That is what these tests pin down — that it comes from the column above, that
//! a gap in that column cuts it off, and that more of it converts faster rather than
//! merely converting at all.

mod common;

use sim_core::field::CellField;
use sim_core::{compress, scene, World};

/// A walled shaft one cell wide, filled from the bottom with `depth` cells of `id`.
///
/// One cell wide on purpose: a column is exactly what load is defined over, so a shaft
/// makes the quantity under test the only thing varying. The walls stop the pile
/// slumping sideways, which is what a powder does the moment it has anywhere to go.
fn shaft(world: &mut World, x: i32, floor_y: i32, depth: i32, id: u8, structure: u8) {
    for y in (floor_y - depth - 2)..=floor_y {
        world.field_mut().set(x - 1, y, structure);
        world.field_mut().set(x + 1, y, structure);
    }
    world.field_mut().set(x, floor_y, structure);
    for offset in 1..=depth {
        world.field_mut().set(x, floor_y - offset, id);
    }
}

/// Load is the summed density of the contiguous column standing on a cell — not
/// counting the cell itself.
#[test]
fn load_is_the_weight_of_the_column_above() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt_id = rules.elements.id_of("burntResidue").expect("burntResidue");
    let density = rules.elements.get(burnt_id).expect("burntResidue").density;

    let mut world = scene::arena(120, 120, 1, &rules);
    let (x, floor) = (60, 100);
    let depth = 10;
    shaft(&mut world, x, floor, depth, burnt_id, structure);

    // The bottom cell carries the nine above it, and not itself.
    assert_eq!(
        compress::load_at(world.field(), &rules.elements, x, floor - 1),
        i64::from(density) * i64::from(depth - 1),
    );
    // And the topmost carries nothing, with only empty space over it.
    assert_eq!(
        compress::load_at(world.field(), &rules.elements, x, floor - depth),
        0,
        "nothing is standing on the top of the pile"
    );
}

/// A gap in the column cuts the load off. A shelf with air under it presses on nothing,
/// and this is also what keeps the chunked and flat worlds agreeing — above the topmost
/// matter every column is empty, so both start from zero however far their bounds reach.
#[test]
fn a_gap_in_the_column_carries_no_load() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let density = i64::from(rules.elements.get(burnt).expect("burntResidue").density);

    let mut world = scene::arena(120, 120, 1, &rules);
    let (x, floor) = (60, 100);
    for y in 60..=floor {
        world.field_mut().set(x - 1, y, structure);
        world.field_mut().set(x + 1, y, structure);
    }
    world.field_mut().set(x, floor, structure);

    // Two cells resting on the floor, then a gap, then a long stack hung above it.
    world.field_mut().set(x, floor - 1, burnt);
    world.field_mut().set(x, floor - 2, burnt);
    for y in (floor - 30)..=(floor - 5) {
        world.field_mut().set(x, y, burnt);
    }

    assert_eq!(
        compress::load_at(world.field(), &rules.elements, x, floor - 1),
        density,
        "only the one cell actually touching it counts, not the stack across the gap"
    );
}

/// The threshold is a threshold. Under it, nothing compacts however long you wait.
#[test]
fn a_shallow_pile_never_compacts() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let element = rules.elements.get(burnt).expect("burntResidue");

    // One cell short of the threshold, by the loader's own numbers rather than a
    // hand-copied depth — so retuning the data cannot silently make this test vacuous.
    let depth = element.compaction_load / element.density;
    assert!(depth > 1, "the threshold should be deeper than a single cell");

    let mut world = scene::arena(200, 200, 1, &rules);
    let (x, floor) = (100, 180);
    shaft(&mut world, x, floor, depth, burnt, structure);
    world.step_many(600);

    let converted = (1..=depth).filter(|o| world.get(x, floor - o) == fuel).count();
    assert_eq!(converted, 0, "nothing under the threshold should have compacted");
}

/// Deep enough, and it does — and deeper still does it faster, which is the property
/// that makes depth a dial with a range rather than a switch with two positions.
#[test]
fn deeper_piles_compact_and_compact_faster() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let element = rules.elements.get(burnt).expect("burntResidue");
    let threshold_depth = element.compaction_load / element.density;

    let converted_after = |depth: i32, ticks: u64| {
        let mut world = scene::arena(300, 300, 1, &rules);
        let (x, floor) = (150, 280);
        shaft(&mut world, x, floor, depth, burnt, structure);
        world.step_many(ticks);
        (1..=depth).filter(|o| world.get(x, floor - o) == fuel).count()
    };

    let shallow = converted_after(threshold_depth + 4, 120);
    let deep = converted_after(threshold_depth * 3, 120);

    assert!(shallow > 0, "past the threshold something should compact: {shallow}");
    assert!(
        deep > shallow,
        "three times the depth should compact more in the same time, not the same: \
         {shallow} then {deep}"
    );
}

/// What compacting produces must not itself compact, or a silo would eat its own output
/// and the chain would never end.
#[test]
fn what_compacting_produces_does_not_itself_compact() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    assert_eq!(
        rules.elements.get(fuel).expect("fuel").compacts_into,
        sim_core::EMPTY,
        "fuel should be the end of the chain"
    );

    let mut world = scene::arena(300, 300, 1, &rules);
    let (x, floor) = (150, 280);
    shaft(&mut world, x, floor, 90, fuel, structure);
    world.step_many(300);

    let intact = (1..=90).filter(|o| world.get(x, floor - o) == fuel).count();
    assert_eq!(intact, 90, "every cell of fuel should still be fuel");
}

/// Weight is weight, wherever it comes from. A lid of structure piled on a pile that is
/// itself too shallow should push it over the line — which is what makes "build a heavier
/// roof" a real alternative to "dig a deeper hole".
#[test]
fn weight_from_anything_counts() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let element = rules.elements.get(burnt).expect("burntResidue");
    let shallow = element.compaction_load / element.density - 4;

    let mut world = scene::arena(200, 200, 1, &rules);
    let (x, floor) = (100, 180);
    shaft(&mut world, x, floor, shallow + 30, burnt, structure);
    // Replace the upper part of the charge with structure: same column height, but a
    // lid of something heavier than the powder under it.
    for offset in (shallow + 1)..=(shallow + 30) {
        world.field_mut().set(x, floor - offset, structure);
    }
    world.step_many(300);

    let converted = (1..=shallow).filter(|o| world.get(x, floor - o) == fuel).count();
    assert!(converted > 0, "a heavy lid should compact what is under it: {converted}");
}
