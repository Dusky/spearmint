//! Machine mechanics that are not about any one machine: the beat a machine acts on,
//! switching one off, and overriding its rate per instance.
//!
//! These used to live in `refine.rs` and were tested through the burner, because it had
//! the simplest geometry of anything that converted a cell. Both conversion machines are
//! gone now — residue burns and burnt residue compacts, neither of them a machine — so
//! the heater stands in: it is the surviving machine whose work is countable one cell at
//! a time, since `burn_fuel` takes a fixed number of cells per beat and leaves the rest.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{paint, scene, Entity, World};

const CELLS: i32 = TILE_CELLS as i32;

/// A machine sealed into a pocket of structure — left, right, and floored — so a fed
/// charge stays put to be inspected rather than falling anywhere.
fn hopper(world: &mut World, tile_x: i32, tile_y: i32, structure: u8, machine: Entity) {
    world.place(machine);
    let field = world.field_mut();
    paint::stroke(field, (tile_x - 1, tile_y + 1), (tile_x + 1, tile_y + 1), structure);
    paint::stroke(field, (tile_x - 1, tile_y), (tile_x - 1, tile_y), structure);
    paint::stroke(field, (tile_x + 1, tile_y), (tile_x + 1, tile_y), structure);
}

/// Puts `count` cells along the bottom row inside a machine, which is where it takes
/// from first.
fn feed(world: &mut World, tile_x: i32, tile_y: i32, count: i32, id: u8) {
    let y = (tile_y + 1) * CELLS - 1;
    for offset in 0..count {
        world.field_mut().set(tile_x * CELLS + offset, y, id);
    }
}

fn count_in_body(world: &World, tile_x: i32, tile_y: i32, id: u8) -> i32 {
    let mut found = 0;
    for y in tile_y * CELLS..(tile_y + 1) * CELLS {
        for x in tile_x * CELLS..(tile_x + 1) * CELLS {
            if world.get(x, y) == id {
                found += 1;
            }
        }
    }
    found
}

/// A machine acts on its own beat, not every tick.
///
/// Playtesting found the whole factory roughly six times too fast; `interval` is half
/// the answer (the host halving its tick rate is the other half). What matters here is
/// that the ticks *between* actions genuinely do nothing — a machine that quietly kept
/// working between beats would look paced while running at the old speed.
#[test]
fn a_machine_only_acts_on_its_interval() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let definition = rules.entities.get(common::heater_kind()).expect("heater");
    let (rate, interval) = (definition.rate as i32, definition.interval as u64);
    assert!(
        interval > 1,
        "this test needs a paced machine, or it cannot tell pacing from its absence"
    );
    assert!(rate < CELLS, "a single beat should not be able to finish the whole row");

    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::heater(4, 6));
    feed(&mut world, 4, 6, CELLS, fuel);

    // Tick 0 is on the beat.
    world.step();
    assert_eq!(count_in_body(&world, 4, 6, fuel), CELLS - rate);

    // Every tick up to the next beat must change nothing at all.
    world.step_many(interval - 1);
    assert_eq!(
        count_in_body(&world, 4, 6, fuel),
        CELLS - rate,
        "the machine kept working between beats"
    );

    // And the next beat lands.
    world.step();
    assert_eq!(count_in_body(&world, 4, 6, fuel), CELLS - rate * 2);
}

/// A disabled machine does nothing at all, and picks straight back up when re-enabled.
///
/// For working on a running factory: pausing the part you are rebuilding beats deleting
/// and replacing it. So what matters is that it is genuinely inert while off — not
/// merely slower — and that switching it back on needs no other repair.
#[test]
fn a_disabled_machine_does_nothing_until_switched_back_on() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::heater(4, 6));
    feed(&mut world, 4, 6, CELLS, fuel);

    assert!(world.retune(0, |entity| entity.enabled = false), "there is a machine at 0");
    world.step_many(200);
    assert_eq!(
        count_in_body(&world, 4, 6, fuel),
        CELLS,
        "a switched-off machine consumed its input anyway"
    );

    world.retune(0, |entity| entity.enabled = true);
    world.step_many(200);
    assert_eq!(
        count_in_body(&world, 4, 6, fuel),
        0,
        "switching it back on should need no other repair"
    );
}

/// A per-instance rate overrides the type's, so two machines of one kind can run at
/// different speeds. Zero keeps meaning "as the type says", which is what makes the
/// field safe to default.
#[test]
fn a_per_instance_rate_overrides_the_types() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let definition = rules.entities.get(common::heater_kind()).expect("heater");
    assert!(definition.rate > 1, "this test needs room to slow the machine down");

    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::heater(4, 6));
    feed(&mut world, 4, 6, CELLS, fuel);

    world.retune(0, |entity| entity.rate = 1);
    world.step();
    assert_eq!(
        count_in_body(&world, 4, 6, fuel),
        CELLS - 1,
        "one beat at a retuned rate of 1 should consume exactly one cell"
    );

    // Zero is not "off" — it means defer to the type, which is why placement can use it
    // as "unset" without a separate flag.
    world.retune(0, |entity| entity.rate = 0);
    world.step_many(definition.interval as u64);
    assert_eq!(
        count_in_body(&world, 4, 6, fuel),
        CELLS - 1 - definition.rate as i32,
        "a rate of zero should fall back to the type's"
    );
}
