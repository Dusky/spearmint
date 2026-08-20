//! Going up — spec 11's first open question, and the one that stopped the refine chain
//! closing its own loop.
//!
//! Nothing else in the game decreases a cell's y: gravity moves matter down, belts move
//! it sideways, emitters only introduce it. So fuel compacted at the bottom of a silo
//! could never reach the heaters above it, and the factory could not run on what it
//! made. What matters here is that a lift genuinely raises *the same particles*, that it
//! only climbs while it is fed, and that a belt underneath feeds it with no special case
//! at either end.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{scene, World};

const CELLS: i32 = TILE_CELLS as i32;

fn highest(world: &World, id: u8, x0: i32, x1: i32, y0: i32, y1: i32) -> Option<i32> {
    (y0..=y1)
        .find(|&y| (x0..=x1).any(|x| world.get(x, y) == id))
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

/// A fed shaft carries its column up and steps the top cell out of the mouth.
#[test]
fn a_fed_lift_raises_what_is_in_it() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);
    let (tile_x, tile_y, height) = (5, 8, 3);
    let entity = common::lift(tile_x, tile_y, height);
    world.place(entity);
    let (x0, y0, x1, y1) = entity.body();

    // A floor under the shaft, and a charge of fuel resting on it. The outermost
    // columns are the shaft's own walls, written at placement, so the charge goes in
    // the interior.
    for x in x0..=x1 {
        world.field_mut().set(x, y1 + 1, structure);
    }
    for x in (x0 + 1)..x1 {
        for y in (y1 - 5)..=y1 {
            world.field_mut().set(x, y, fuel);
        }
    }

    let before = highest(&world, fuel, x0, x1, y0 - 20, y1).expect("fuel is in the shaft");
    world.step_many(200);
    let after = highest(&world, fuel, x0, x1, y0 - 20, y1).expect("and it has not vanished");

    assert!(
        after < before,
        "a fed lift should have raised its column ({before} -> {after})"
    );
    assert!(
        after < y0,
        "and some of it should have stepped out of the mouth: mouth at {y0}, highest {after}"
    );
}

/// Nothing is created or destroyed doing it — every step in `raise` is a move, and
/// spec 1.1 makes that load-bearing rather than incidental.
#[test]
fn lifting_conserves_every_cell() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);
    let entity = common::lift(5, 8, 3);
    world.place(entity);
    let (x0, _, x1, y1) = entity.body();
    for x in x0..=x1 {
        world.field_mut().set(x, y1 + 1, structure);
    }
    for x in (x0 + 1)..x1 {
        for y in (y1 - 5)..=y1 {
            world.field_mut().set(x, y, fuel);
        }
    }

    let before = count(&world, fuel, 0, 0, 199, 199);
    world.step_many(400);
    let after = count(&world, fuel, 0, 0, 199, 199);
    assert_eq!(before, after, "the lift lost or invented fuel");
}

/// An unfed lift delivers what is already in it rather than holding it back. It is an
/// elevator, not a valve — refusing to carry the last of a load because no more is
/// coming would be a bug wearing the costume of a feature.
#[test]
fn an_unfed_lift_still_delivers_what_is_already_in_it() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);
    let entity = common::lift(5, 8, 3);
    world.place(entity);
    let (x0, y0, x1, y1) = entity.body();
    for x in x0..=x1 {
        world.field_mut().set(x, y1 + 1, structure);
    }
    let charge = 3 * (x1 - x0 - 1);
    for x in (x0 + 1)..x1 {
        for y in (y1 - 2)..=y1 {
            world.field_mut().set(x, y, fuel);
        }
    }

    world.step_many(600);

    assert!(
        count(&world, fuel, x0, y0 - 30, x1 + 4, y0 - 1) > 0,
        "an unfed lift should still have delivered what it was already holding"
    );
    assert_eq!(
        count(&world, fuel, 0, 0, 199, 199),
        charge,
        "and every cell of the charge should still exist somewhere"
    );
}

/// A blocked mouth stalls the shaft rather than crushing anything into it.
///
/// This is why the test above cannot ask for the shaft to *empty*: with nothing carrying
/// the output away, the first row out sits on the mouth and the lift correctly stops.
/// Backpressure, and the reason a lift wants a belt at the top as well as the bottom.
#[test]
fn a_blocked_mouth_stalls_the_shaft() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);
    let entity = common::lift(5, 8, 3);
    world.place(entity);
    let (x0, y0, x1, y1) = entity.body();
    for x in x0..=x1 {
        world.field_mut().set(x, y1 + 1, structure);
        // A lid right across the mouth: nowhere for anything to step out to.
        world.field_mut().set(x, y0 - 1, structure);
    }
    let charge = 3 * (x1 - x0 - 1);
    for x in (x0 + 1)..x1 {
        for y in (y1 - 2)..=y1 {
            world.field_mut().set(x, y, fuel);
        }
    }

    world.step_many(400);

    assert_eq!(
        count(&world, fuel, x0, y0, x1, y1),
        charge,
        "a lift with nowhere to put its load should have held all of it, intact"
    );
    let definition = rules.entities.get(common::lift_kind()).expect("lift");
    assert!(
        entity.is_blocked(definition, world.field(), &rules.elements),
        "and it should read as blocked, so the player can see why it stopped"
    );
}

/// The one coupling in `raise`, pinned: its cancelling pass assumes a powder falls
/// exactly one cell a tick, and exists to undo precisely that. With nothing at all under
/// the shaft, a load should therefore hang where the machine holds it rather than
/// sinking — which is what an elevator does, and is also the assertion that fails loudly
/// if gravity ever accelerates instead of lifts quietly becoming down-escalators.
#[test]
fn a_lift_holds_its_load_over_a_void() {
    let rules = common::rules_without_reactions();
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);
    let entity = common::lift(5, 8, 3);
    world.place(entity);
    let (x0, y0, x1, y1) = entity.body();

    // Deliberately no floor: the only thing between this charge and the bottom of the
    // arena is the machine.
    let lowest_before = y1;
    for x in (x0 + 1)..x1 {
        world.field_mut().set(x, y1, fuel);
    }

    world.step_many(40);

    let lowest_after = (y0..200)
        .rev()
        .find(|&y| ((x0 + 1)..x1).any(|x| world.get(x, y) == fuel))
        .expect("the charge has not vanished");
    assert!(
        lowest_after <= lowest_before,
        "the load sank from {lowest_before} to {lowest_after}, so the cancelling pass is \
         no longer cancelling a tick of fall"
    );
}

/// A belt running directly under a lift feeds it, with no special case at either end:
/// a belt's carry row *is* the bottom row of a lift standing on it.
#[test]
fn a_belt_underneath_feeds_a_lift_with_no_special_case() {
    let rules = common::rules_without_reactions();
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);
    // Belt at tile row 11; its carry row is the last row of tile row 10, which is where
    // a lift whose lowest tile is row 10 has its floor.
    let (tile_x, belt_row) = (5, 11);
    for offset in 0..4 {
        world.place(common::belt(tile_x - 3 + offset, belt_row, 1));
    }
    let entity = common::lift(tile_x, belt_row - 3, 3);
    world.place(entity);
    let (x0, y0, x1, y1) = entity.body();
    assert_eq!(y1, belt_row * CELLS - 1, "the lift's floor should be the belt's carry row");

    // Cargo upstream on the belt, which will be carried under the lift and up it.
    let carry_y = y1;
    for x in ((tile_x - 3) * CELLS)..x0 {
        world.field_mut().set(x, carry_y, fuel);
    }

    world.step_many(600);

    let lifted = count(&world, fuel, x0, y0 - 12, x1, y0 - 1);
    assert!(lifted > 0, "the belt should have fed the lift and the lift raised it: {lifted}");

    // And the belt must not have walked off with the shaft. A belt's carry row is the
    // lift's bottom row, so before machine structure was made un-carryable the belt
    // conveyed the lift's own walls away a cell at a time.
    let chassis = rules.elements.id_of("beltStructure").expect("beltStructure");
    let walls = (y0..y1).filter(|&y| world.get(x0, y) == chassis).count()
        + (y0..y1).filter(|&y| world.get(x1, y) == chassis).count();
    assert_eq!(
        walls,
        2 * (y1 - y0) as usize,
        "the belt underneath carried off part of the shaft's walls"
    );
}
