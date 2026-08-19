//! Product in, currency out.
//!
//! Currency is matter (spec 5.1): a press presses what falls into it into nuggets,
//! the nuggets are cells like any other, and the player's balance is what a machine is
//! holding. So the things worth testing are the exchange rate, the refusal to grind its
//! own output, and the accounting — matter may leave the world, but never quietly.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{paint, scene, World, EMPTY};

const CELLS: i32 = TILE_CELLS as i32;

/// A press set into a pocket of wall, which is how one is meant to be used: a
/// machine is not matter (spec 4.1), so material falls straight through an
/// open-bottomed press and slides out of an unwalled one.
fn hopper(world: &mut World, tile_x: i32, tile_y: i32, structure: u8) {
    world.place(common::press(tile_x, tile_y));
    let field = world.field_mut();
    paint::stroke(field, (tile_x - 1, tile_y + 1), (tile_x + 1, tile_y + 1), structure);
    paint::stroke(field, (tile_x - 1, tile_y), (tile_x - 1, tile_y), structure);
    paint::stroke(field, (tile_x + 1, tile_y), (tile_x + 1, tile_y), structure);
}

/// A press with side walls only — no floor. Residue is real matter now (spec 5.3), so
/// a press has to have somewhere to put it, the same way it needs a vault under it for
/// gold. These tests run the press for thousands of ticks, so unlike `hopper` — which
/// deliberately seals a fed charge in place to inspect it mid-press — they need an
/// outlet or the body fills with its own byproduct and jams.
fn chute(world: &mut World, tile_x: i32, tile_y: i32, structure: u8) {
    world.place(common::press(tile_x, tile_y));
    let field = world.field_mut();
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

/// How many ticks it takes a press to get through what it used to in `ticks`.
///
/// The "run it until the machine has worked through everything" tests care about the
/// end state, not the clock, so they scale with the press's pacing rather than
/// hard-coding a tick count that silently stops being enough when pacing is tuned.
fn pressing_ticks(ticks: u64) -> u64 {
    let rules = common::rules();
    let interval = rules
        .entities
        .get(common::press_kind())
        .expect("press")
        .interval as u64;
    ticks * interval
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
    let structure = rules.elements.id_of("structure").expect("structure");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let per = rules
        .entities
        .get(common::press_kind())
        .expect("press")
        .gold_per as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure);

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
    let structure = rules.elements.id_of("structure").expect("structure");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let per = rules
        .entities
        .get(common::press_kind())
        .expect("press")
        .gold_per as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure);

    feed(&mut world, 4, 6, per - 1, wet);
    world.step_many(20);
    assert_eq!(world.collected(), 0, "one grain short is not a nugget");

    // The missing grain arrives later, and the machine remembers the rest.
    feed(&mut world, 4, 6, 1, wet);
    world.step_many(20);
    assert_eq!(world.collected(), 1, "the bank carried across ticks");
}

/// The rule that keeps a press from starving itself: it takes only product, so the sand
/// and water it depends on flow past untouched and go on reacting.
#[test]
fn raw_material_is_left_alone() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure);

    let before = world.count_of(sand);
    feed(&mut world, 4, 6, CELLS, sand);
    world.step_many(40);

    assert_eq!(
        world.count_of(sand),
        before + CELLS as usize,
        "unwashed sand is not the press's business"
    );
    assert_eq!(world.collected(), 0);
}

/// Walls have no value, so they are not pressable, and a press set into one does not
/// chew its way out.
#[test]
fn a_press_leaves_structure_alone() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure);

    feed(&mut world, 4, 6, CELLS, structure);
    let before = world.count_of(structure);
    world.step_many(20);

    // A press that ate walls would be a demolition tool, and erase already is one.
    assert_eq!(world.count_of(structure), before);
    assert_eq!(world.collected(), 0);
}

/// A machine must not grind its own output back into nothing.
#[test]
fn a_press_never_eats_currency() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure);

    feed(&mut world, 4, 6, CELLS, gold);
    world.step_many(200);

    assert_eq!(count_in_body(&world, 4, 6, gold), CELLS, "it ate its own money");
    // Not stored, either: a press is where gold is made, not where it is kept
    // (spec 5.1). Storage is a vault, and there is none here.
    assert_eq!(world.stored(), 0);
}

/// The other output. What does not complete a nugget is not destroyed — it is
/// converted (spec 5.3), and residue is what wet sand converts into.
#[test]
fn a_press_leaves_residue_behind() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let residue = rules.elements.id_of("residue").expect("residue");
    let per = rules
        .entities
        .get(common::press_kind())
        .expect("press")
        .gold_per as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure);

    feed(&mut world, 4, 6, per, wet);
    world.step_many(20);

    // Every grain that did not complete the nugget became residue, not nothing.
    assert_eq!(count_in_body(&world, 4, 6, residue), per - 1);
}

/// A press must not grind its own byproduct back through itself — residue has no
/// value, so `is_pressable` already refuses it, but that is worth pinning directly
/// rather than trusting it as a side effect of another rule.
#[test]
fn a_press_never_re_presses_its_own_residue() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let residue = rules.elements.id_of("residue").expect("residue");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure);

    feed(&mut world, 4, 6, CELLS, residue);
    world.step_many(200);

    assert_eq!(count_in_body(&world, 4, 6, residue), CELLS, "it ate its own byproduct");
}

/// Full of money and unable to work is a state worth showing, and the sprite already
/// draws it.
#[test]
fn a_press_full_of_gold_reads_as_blocked() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let definition = rules
        .entities
        .get(common::press_kind())
        .expect("press");
    let mut world = scene::arena(200, 200, 1, &rules);
    let press = common::press(4, 6);
    hopper(&mut world, 4, 6, structure);

    assert!(
        press.is_blocked(definition, world.field(), &rules.elements),
        "nothing has fallen in yet"
    );

    feed(&mut world, 4, 6, CELLS, gold);
    assert!(
        press.is_blocked(definition, world.field(), &rules.elements),
        "a body holding only money has nothing left to work on"
    );

    // Product landing on top of the money gives it something to do again. Checked
    // without stepping, because the machine would eat it within a few ticks.
    let top = 6 * CELLS;
    for offset in 0..CELLS {
        world.field_mut().set(4 * CELLS + offset, top, wet);
    }
    assert!(!press.is_blocked(definition, world.field(), &rules.elements));
}

/// The accounting property, and the reason a sink is allowed at all — but now a
/// stronger one than "leaves are on the books". A press converts rather than destroys
/// (spec 5.3): every physical cell it takes becomes either a nugget or residue, never
/// nothing, so the total cell count is conserved across pressing. Only spending removes
/// matter from the world outright, which is a separate action this test does not touch.
#[test]
fn pressing_conserves_every_cell() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let wet = table.id_of("wetSand").expect("wetSand");
    let gold = table.id_of("gold").expect("gold");
    let residue = table.id_of("residue").expect("residue");
    let structure = table.id_of("structure").expect("structure");
    let mut world = scene::arena(200, 200, 5, &rules);
    chute(&mut world, 4, 12, structure);
    // Walls the rest of the way up, so the product has nowhere to heap except into
    // the machine.
    paint::stroke(world.field_mut(), (3, 8), (3, 11), structure);
    paint::stroke(world.field_mut(), (5, 8), (5, 11), structure);

    for tile_y in 8..12 {
        paint::stroke(world.field_mut(), (4, tile_y), (4, tile_y), wet);
    }
    let product = world.count_of(wet);
    let before: usize = table.iter().map(|element| world.count_of(element.id)).sum();

    world.step_many(pressing_ticks(3_000));

    let after: usize = table.iter().map(|element| world.count_of(element.id)).sum();
    let minted = world.collected() as usize;
    assert!(minted > 0, "nothing reached the press");
    assert_eq!(before, after, "pressing destroyed matter instead of converting it");

    // Every grain is either still wet sand or was converted into exactly one nugget or
    // one cell of residue — the accounting spec 1.1 demands.
    //
    // Note what is *not* asserted: that the chute emptied. A paced press is a real
    // throughput limit rather than a race, so product arriving faster than it can be
    // pressed carries on past and lands below — deliberate, and documented on `press`
    // itself. Requiring every grain to be pressed would be requiring the machine to be
    // infinitely fast, which is the opposite of what pacing is for.
    assert_eq!(
        world.count_of(gold) + world.count_of(residue) + world.count_of(wet),
        product,
        "a grain went somewhere that is neither nugget, residue, nor still-unpressed"
    );

    // The nuggets are all still there — in the press, which is not storage. Getting
    // them somewhere that counts is the vault's job, and gravity's.
    assert_eq!(world.count_of(gold), minted, "a nugget went missing");
    assert_eq!(world.stored(), 0, "a press is not a vault");
}

/// A settled pile stops moving, and a chunk where nothing moves goes to sleep. The
/// press eats regardless — machines tick whether or not their chunk does — so
/// without waking what it takes from, it hollows out the product resting inside it and
/// then starves under a pile that never collapses.
#[test]
fn a_press_wakes_the_pile_resting_on_it() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let mut world = scene::arena(200, 200, 5, &rules);

    // A solid floor, so the pile settles exactly as it would with no press involved.
    paint::stroke(world.field_mut(), (3, 12), (5, 12), structure);
    paint::stroke(world.field_mut(), (3, 8), (3, 11), structure);
    paint::stroke(world.field_mut(), (5, 8), (5, 11), structure);
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
        "the pile should be at rest before the press arrives"
    );

    // Only now dig the press's outlet — residue is real matter (spec 5.3) and needs
    // somewhere to go, the same way gold needs a vault under it.
    paint::stroke(world.field_mut(), (4, 12), (4, 12), EMPTY);
    world.place(common::press(4, 11));
    world.step_many(pressing_ticks(3_000));

    // The column has to have come *down*, which is the thing waking is for. Counting
    // pressed grains would not show it: a paced press lets some product fall straight
    // through (see `pressing_conserves_every_cell`), so "not all of it was pressed" is
    // normal and would mask the real failure. A pile that never collapsed leaves wet
    // sand stranded up where it started, and that is what this looks for.
    let stranded = (8 * CELLS..12 * CELLS)
        .flat_map(|y| (3 * CELLS..6 * CELLS).map(move |x| (x, y)))
        .filter(|&(x, y)| world.get(x, y) == wet)
        .count();
    assert_eq!(
        stranded, 0,
        "{stranded} cells never came down: the press hollowed out the pile resting \
         inside it and then starved under a column that never collapsed"
    );
    assert!(world.collected() > 0, "the press never got anything at all");
}
