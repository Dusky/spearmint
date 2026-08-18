//! The one licensed sink.
//!
//! Spec 1.1 says particles are never destroyed outside the physics rules themselves. A
//! collector is such a rule: it consumes what gravity feeds it and pays for it. Which
//! makes accounting the thing to test — matter may leave the world, but never quietly.

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

/// Fills the bottom row inside a collector, which is where it takes from first.
fn fill_body(world: &mut World, tile_x: i32, tile_y: i32, id: u8) {
    let y = (tile_y + 1) * CELLS - 1;
    for offset in 0..CELLS {
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

#[test]
fn a_collector_eats_what_falls_into_it() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let rate = rules
        .entities
        .get(common::collector_kind())
        .expect("collector")
        .rate as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    fill_body(&mut world, 4, 6, wet);
    assert_eq!(count_in_body(&world, 4, 6, wet), CELLS);

    world.step();

    // Rate-limited: a collector works through what it holds over several ticks, so
    // throughput is a property of the machine rather than of the tick.
    assert_eq!(count_in_body(&world, 4, 6, wet), CELLS - rate);
    assert_eq!(world.collected(), rate as u64);
}

#[test]
fn a_collector_pays_only_for_product() {
    let rules = common::rules_without_reactions();
    let sand = rules.elements.id_of("sand").expect("sand");
    let wall = rules.elements.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    fill_body(&mut world, 4, 6, sand);
    world.step_many(20);

    // Raw sand is worth nothing, and feeding it in destroys it anyway. That is the
    // point: routing unwashed material into a collector is a visible waste.
    assert_eq!(world.count_of(sand), 0, "the sand should have been eaten");
    assert_eq!(world.collected(), 0, "raw sand should pay nothing");
}

#[test]
fn a_collector_refuses_solids() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall);

    fill_body(&mut world, 4, 6, wall);
    let before = world.count_of(wall);
    world.step_many(20);

    // A collector that ate walls would be a demolition tool, and erase already is one.
    assert_eq!(world.count_of(wall), before);
    assert_eq!(world.collected(), 0);
}

#[test]
fn an_unfed_collector_reads_as_blocked() {
    let rules = common::rules_without_reactions();
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let definition = rules
        .entities
        .get(common::collector_kind())
        .expect("collector");
    let mut world = scene::arena(200, 200, 1, &rules);
    let collector = common::collector(4, 6);
    world.place(collector);

    assert!(
        collector.is_blocked(definition, world.field()),
        "nothing has fallen in yet"
    );
    fill_body(&mut world, 4, 6, wet);
    assert!(!collector.is_blocked(definition, world.field()));
}

/// The accounting property, and the reason a sink is allowed at all: every cell that
/// leaves the world is on the books.
#[test]
fn everything_destroyed_is_accounted_for() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let wet = table.id_of("wetSand").expect("wetSand");
    let wall = rules.elements.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 5, &rules);
    hopper(&mut world, 4, 12, wall);

    // A column of product above the collector, falling in under gravity.
    for tile_y in 4..12 {
        paint::stroke(world.field_mut(), (4, tile_y), (4, tile_y), wet);
    }

    let before: usize = table.iter().map(|element| world.count_of(element.id)).sum();
    world.step_many(600);
    let after: usize = table.iter().map(|element| world.count_of(element.id)).sum();

    // wetSand is the only thing worth anything, so gold is also the cell count here —
    // which is what makes this a conservation check rather than a vibe.
    assert!(world.collected() > 0, "nothing reached the collector");
    assert_eq!(
        before - after,
        world.collected() as usize,
        "cells left the world without being collected"
    );
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

    // A walled chute with a floor, filled with product, and no machine yet.
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

    // Now put a collector under it. Everything above has to come down.
    world.place(common::collector(4, 11));
    world.step_many(2_000);

    assert_eq!(
        world.count_of(wet),
        0,
        "the collector hollowed out the pile and then starved under it"
    );
}
