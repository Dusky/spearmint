//! Product in, currency out.
//!
//! Currency is matter (spec 5.1): a collector presses what falls into it into nuggets,
//! the nuggets are cells like any other, and the player's balance is what a machine is
//! holding. So the things worth testing are the exchange rate, the refusal to grind its
//! own output, and the accounting — matter may leave the world, but never quietly.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{paint, scene, World};

const CELLS: i32 = TILE_CELLS as i32;

/// A collector set into a pocket of wall, which is how one is meant to be used: a
/// machine is not matter (spec 4.1), so material falls straight through an
/// open-bottomed collector and slides out of an unwalled one.
fn hopper(world: &mut World, tile_x: i32, tile_y: i32, wall: u8) {
    world.place(common::collector(tile_x, tile_y));
    let field = world.field_mut();
    paint::stroke(field, (tile_x - 1, tile_y + 1), (tile_x + 1, tile_y + 1), wall);
    paint::stroke(field, (tile_x - 1, tile_y), (tile_x - 1, tile_y), wall);
    paint::stroke(field, (tile_x + 1, tile_y), (tile_x + 1, tile_y), wall);
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

/// The exchange rate, and the shape of pillar 2: several grains go in, one nugget comes
/// out, in the place the last grain was.
#[test]
fn product_is_pressed_into_nuggets() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let per = rules
        .entities
        .get(common::collector_kind())
        .expect("collector")
        .gold_per as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    feed(&mut world, 4, 6, per, wet);
    world.step_many(20);

    assert_eq!(count_in_body(&world, 4, 6, wet), 0, "all of it should be eaten");
    assert_eq!(count_in_body(&world, 4, 6, gold), 1, "exactly one nugget");
    assert_eq!(world.collected(), 1, "one nugget ever minted");
}

/// A partial nugget's worth of product is banked, not lost or rounded up.
#[test]
fn a_part_load_mints_nothing_yet() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let per = rules
        .entities
        .get(common::collector_kind())
        .expect("collector")
        .gold_per as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    feed(&mut world, 4, 6, per - 1, wet);
    world.step_many(20);
    assert_eq!(world.collected(), 0, "one grain short is not a nugget");

    // The missing grain arrives later, and the machine remembers the rest.
    feed(&mut world, 4, 6, 1, wet);
    world.step_many(20);
    assert_eq!(world.collected(), 1, "the bank carried across ticks");
}

#[test]
fn raw_material_mints_nothing_and_is_still_eaten() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    feed(&mut world, 4, 6, CELLS, sand);
    world.step_many(40);

    // Routing unwashed material into a collector destroys it for nothing. That is the
    // cost of getting the routing wrong, and it should be visible.
    assert_eq!(world.count_of(sand), 0);
    assert_eq!(world.collected(), 0);
}

#[test]
fn a_collector_refuses_solids() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    feed(&mut world, 4, 6, CELLS, wall);
    let before = world.count_of(wall);
    world.step_many(20);

    // A collector that ate walls would be a demolition tool, and erase already is one.
    assert_eq!(world.count_of(wall), before);
    assert_eq!(world.collected(), 0);
}

/// A machine must not grind its own output back into nothing.
#[test]
fn a_collector_never_eats_currency() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    feed(&mut world, 4, 6, CELLS, gold);
    world.step_many(200);

    assert_eq!(count_in_body(&world, 4, 6, gold), CELLS, "it ate its own money");
    assert_eq!(world.stored(), CELLS as u64);
}

/// Full of money and unable to work is a state worth showing, and the sprite already
/// draws it.
#[test]
fn a_collector_full_of_gold_reads_as_blocked() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let definition = rules
        .entities
        .get(common::collector_kind())
        .expect("collector");
    let mut world = scene::arena(200, 200, 1, &rules);
    let collector = common::collector(4, 6);
    hopper(&mut world, 4, 6, wall);

    assert!(
        collector.is_blocked(definition, world.field(), &rules.elements),
        "nothing has fallen in yet"
    );

    feed(&mut world, 4, 6, CELLS, gold);
    assert!(
        collector.is_blocked(definition, world.field(), &rules.elements),
        "a body holding only money has nothing left to work on"
    );

    // Product landing on top of the money gives it something to do again. Checked
    // without stepping, because the machine would eat it within a few ticks.
    let top = 6 * CELLS;
    for offset in 0..CELLS {
        world.field_mut().set(4 * CELLS + offset, top, wet);
    }
    assert!(!collector.is_blocked(definition, world.field(), &rules.elements));
}

/// The rule the whole design rests on: gold on the floor is not money.
#[test]
fn only_gold_inside_a_machine_counts() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    feed(&mut world, 4, 6, CELLS, gold);
    // A pile of the same size lying on the arena floor, nowhere near a machine.
    paint::stroke(world.field_mut(), (12, 20), (12, 20), gold);

    assert_eq!(world.stored(), CELLS as u64, "only what the machine holds");
    assert!(
        world.count_of(gold) > world.stored() as usize,
        "the loose pile is still gold, and still in the world"
    );
}

#[test]
fn spending_takes_the_exact_amount_or_nothing() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    feed(&mut world, 4, 6, CELLS, gold);
    let held = world.stored();

    // A partial charge is not a purchase.
    assert_eq!(world.spend(held + 1), 0);
    assert_eq!(world.stored(), held, "a refused purchase costs nothing");

    assert_eq!(world.spend(4), 4);
    assert_eq!(world.stored(), held - 4);

    assert_eq!(world.spend(held - 4), held - 4);
    assert_eq!(world.stored(), 0, "the machine is empty");
}

/// Spending reaches into machines only. Loose gold is not spendable, which is the same
/// rule as `only_gold_inside_a_machine_counts` seen from the other side.
#[test]
fn spending_never_reaches_gold_on_the_floor() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);
    paint::stroke(world.field_mut(), (12, 20), (12, 20), gold);

    let loose = world.count_of(gold);
    assert_eq!(world.spend(1), 0, "nothing is being held");
    assert_eq!(world.count_of(gold), loose, "the loose pile is untouched");
}

/// The accounting property, and the reason a sink is allowed at all: every cell that
/// leaves the world is either destroyed as valueless input or pressed into a nugget.
#[test]
fn everything_that_leaves_is_accounted_for() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let wet = table.id_of("wetSand").expect("wetSand");
    let wall = table.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 5, &rules);
    hopper(&mut world, 4, 12, wall);
    // A chute, so the product has nowhere to heap except into the machine.
    paint::stroke(world.field_mut(), (3, 8), (3, 11), wall);
    paint::stroke(world.field_mut(), (5, 8), (5, 11), wall);

    for tile_y in 8..12 {
        paint::stroke(world.field_mut(), (4, tile_y), (4, tile_y), wet);
    }
    let product = world.count_of(wet);
    let before: usize = table.iter().map(|element| world.count_of(element.id)).sum();

    world.step_many(3_000);

    let after: usize = table.iter().map(|element| world.count_of(element.id)).sum();
    let minted = world.collected() as usize;
    assert!(minted > 0, "nothing reached the collector");
    assert_eq!(world.count_of(wet), 0, "the chute did not empty");
    assert_eq!(
        before - after,
        product - minted,
        "cells left the world without being eaten or minted"
    );

    // And spending is the other way out, on the same books.
    let held = world.stored();
    assert_eq!(held, minted as u64, "every nugget is still in the machine");
    world.spend(held);
    let spent: usize = table.iter().map(|element| world.count_of(element.id)).sum();
    assert_eq!(after - spent, held as usize);
}

/// A settled pile stops moving, and a chunk where nothing moves goes to sleep. The
/// collector eats regardless — machines tick whether or not their chunk does — so
/// without waking what it takes from, it hollows out the product resting inside it and
/// then starves under a pile that never collapses.
#[test]
fn a_collector_wakes_the_pile_resting_on_it() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let mut world = scene::arena(200, 200, 5, &rules);

    paint::stroke(world.field_mut(), (3, 12), (5, 12), wall);
    paint::stroke(world.field_mut(), (3, 8), (3, 11), wall);
    paint::stroke(world.field_mut(), (5, 8), (5, 11), wall);
    for tile_y in 8..12 {
        paint::stroke(world.field_mut(), (4, tile_y), (4, tile_y), wet);
    }

    world.set_sleeping(true);
    world.step_many(300);
    let settled = world.content_hash();
    world.step();
    assert_eq!(
        world.content_hash(),
        settled,
        "the pile should be at rest before the collector arrives"
    );

    world.place(common::collector(4, 11));
    world.step_many(3_000);

    assert_eq!(
        world.count_of(wet),
        0,
        "the collector hollowed out the pile and then starved under it"
    );
}
