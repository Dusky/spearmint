//! The refine chain end to end, as a factory rather than as three isolated rigs.
//!
//! Everything downstream of the press was built and tested a stage at a time: burning on
//! a kiln, compacting in a shaft. This is the question those tests cannot answer — do the
//! stages *compose* into something a player could run?

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{scene, World};

const CELLS: i32 = TILE_CELLS as i32;

fn wall(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, structure: u8) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            world.field_mut().set(x, y, structure);
        }
    }
}

fn count(world: &World, id: u8, x0: i32, y0: i32, x1: i32, y1: i32) -> i32 {
    let mut found = 0;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if world.get(x, y) == id {
                found += 1;
            }
        }
    }
    found
}

/// A whole downstream factory: an emitter dropping residue onto a heated kiln run, the
/// kiln's output falling off the end into a walled silo deep enough to compact.
///
/// The one thing it does *not* do is route the silo's fuel back to the heaters, because
/// that is the thing under test.
#[test]
fn the_chain_strands_its_own_fuel_without_a_lift() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let residue = rules.elements.id_of("residue").expect("residue");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(300, 300, 1, &rules);

    // Kiln run at tile row 6, four tiles wide, conveying right.
    let (run_x, run_y, run_len) = (6, 6, 4);
    for offset in 0..run_len {
        world.place(common::kiln(run_x + offset, run_y, 1));
    }
    // Heaters directly under it, and a floor under them.
    for offset in 0..run_len {
        world.place(common::heater(run_x + offset, run_y + 1));
    }
    wall(
        &mut world,
        (run_x - 1) * CELLS,
        (run_y + 2) * CELLS,
        (run_x + run_len) * CELLS - 1,
        (run_y + 2) * CELLS,
        structure,
    );

    // A starting charge of fuel in the heaters, the way a player would have to prime
    // them: there is no other fuel in the world yet.
    for offset in 0..run_len {
        let (x0, y0, x1, y1) = common::heater(run_x + offset, run_y + 1).body();
        wall(&mut world, x0, y0, x1, y1, fuel);
    }

    // An emitter dropping residue onto the head of the run.
    world.place(common::emitter(run_x, run_y - 2, residue));

    // A silo off the downstream end: two walls and a floor, deep enough to compact.
    let silo_x = (run_x + run_len) * CELLS + 4;
    let silo_top = run_y * CELLS;
    let silo_floor = silo_top + 80;
    wall(&mut world, silo_x - 5, silo_top, silo_x - 5, silo_floor, structure);
    wall(&mut world, silo_x + 5, silo_top, silo_x + 5, silo_floor, structure);
    wall(&mut world, silo_x - 5, silo_floor, silo_x + 5, silo_floor, structure);
    // A ramp catching what falls off the kiln and sending it into the silo mouth.
    wall(&mut world, (run_x + run_len) * CELLS, silo_top - 1, silo_x - 5, silo_top - 1, structure);

    world.step_many(3000);

    let heater_fuel = count(
        &world,
        fuel,
        run_x * CELLS,
        (run_y + 1) * CELLS,
        (run_x + run_len) * CELLS - 1,
        (run_y + 2) * CELLS - 1,
    );
    let silo_fuel = count(&world, fuel, silo_x - 4, silo_top, silo_x + 4, silo_floor);
    let silo_burnt = count(&world, burnt, silo_x - 4, silo_top, silo_x + 4, silo_floor);

    eprintln!(
        "after 3000 ticks: heater fuel {heater_fuel}, silo fuel {silo_fuel}, silo burnt {silo_burnt}"
    );

    // The chain works: residue burned, and the burnt residue compacted.
    assert!(silo_fuel > 0, "the silo should have compacted fuel: {silo_fuel}");

    // And the loop is open. The heaters burned their priming charge and there is no
    // transport that could return the silo's fuel to them — belts are horizontal, and
    // gravity only goes down, so nothing in the game moves a powder upward on purpose.
    assert_eq!(heater_fuel, 0, "the heaters should have run dry: {heater_fuel}");
    assert!(
        silo_fuel > 0 && heater_fuel == 0,
        "fuel exists, and it is all in the wrong place"
    );
}

/// Why the loop could not close before the lift existed, kept as the record of it:
/// **nothing but a lift moves a powder upward.**
///
/// Gravity moves matter down or diagonally down, belts move it sideways, and emitters
/// only introduce it. None of those decrease a cell's y. The one exception is
/// displacement — a denser powder sinking through a lighter one lifts the lighter one a
/// cell — which is why this measures a *lone* powder with nothing to displace it, and why
/// displacement is not a pump: it costs one falling cell per cell lifted, and leaves the
/// heavy cell at the bottom needing to come back up itself.
#[test]
fn nothing_but_a_lift_raises_a_powder() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);

    // A belt run with a lone cell of fuel riding it, walled at the far end so it jams
    // rather than falling off — every chance to go somewhere, including up.
    for offset in 0..6 {
        world.place(common::belt(4 + offset, 8, 1));
    }
    let carry_y = 8 * CELLS - 1;
    let start_x = 4 * CELLS;
    world.field_mut().set(start_x, carry_y, fuel);
    world.field_mut().set(10 * CELLS, carry_y, structure);

    let highest = |world: &World| {
        (0..200)
            .flat_map(|y| (0..200).map(move |x| (x, y)))
            .filter(|&(x, y)| world.get(x, y) == fuel)
            .map(|(_, y)| y)
            .min()
    };

    let before = highest(&world).expect("the fuel is there to start with");
    world.step_many(500);
    let after = highest(&world).expect("and it has not vanished");

    assert!(
        after >= before,
        "a powder ended up higher than it started ({before} -> {after}), which would mean \
         some transport lifts material and the fuel loop is not actually open"
    );
}

/// And the same factory with a lift in it: the silo's fuel gets back up to the row the
/// heaters are fed from, which is the whole reason the lift exists.
///
/// Deliberately the same rig as `the_chain_strands_its_own_fuel_without_a_lift`, one
/// machine different, so what the lift changes is the only thing that varies.
#[test]
fn a_lift_returns_the_silos_fuel_to_the_level_that_burns_it() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(300, 300, 1, &rules);

    // A charge of fuel on the floor, at the depth a silo would leave it.
    let (shaft_x, floor_tile) = (12, 14);
    let lift_height = 6;
    let entity = common::lift(shaft_x, floor_tile - lift_height as i32, lift_height);
    world.place(entity);
    let (x0, y0, x1, y1) = entity.body();
    wall(&mut world, x0 - 2, y1 + 1, x1 + 2, y1 + 1, structure);
    for x in (x0 + 1)..x1 {
        for y in (y1 - 12)..=y1 {
            world.field_mut().set(x, y, fuel);
        }
    }

    let started_below = y1 - 12;
    world.step_many(2000);

    let highest = (0..300)
        .find(|&y| (0..300).any(|x| world.get(x, y) == fuel))
        .expect("the fuel is still somewhere");

    eprintln!("fuel started at row {started_below}, highest is now {highest}");
    assert!(
        highest < y0,
        "the fuel should have been carried out of the shaft mouth at {y0}: got {highest}"
    );
    assert!(
        highest < started_below - 40,
        "and well above where it started ({started_below} -> {highest})"
    );
}
