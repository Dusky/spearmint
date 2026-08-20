//! One cell in, one cell out.
//!
//! `Behaviour::Refine` is now one machine rather than two. The burner used to be the
//! other, and is gone: residue burns wherever it is hot enough (see `ignition.rs`)
//! rather than being converted on demand by a box. So everything here is the
//! compactor — which means the mechanics that used to be tested through the burner,
//! because it was the simpler geometry, are tested through a piston instead.
//!
//! What is worth testing is the accepted input, the refusal to touch anything else
//! (including its own output), the beat it acts on, and that switching one off or
//! retuning it does exactly what it says.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{paint, scene, World};

const CELLS: i32 = TILE_CELLS as i32;

/// The tile a compactor placed at `tile_y` crushes against: it reaches *below* itself,
/// so material heaped on the anvil sits in the tile directly under the machine.
fn under(tile_y: i32, height_tiles: i32) -> i32 {
    tile_y + height_tiles
}

/// A compactor with an anvil beneath it and walls either side, so a fed charge stays
/// put to be inspected rather than falling out from under the ram.
fn press_rig(world: &mut World, tile_x: i32, tile_y: i32, structure: u8, height_tiles: i32) -> i32 {
    let anvil = under(tile_y, height_tiles) + 1;
    world.place(common::compactor(tile_x, tile_y));
    let field = world.field_mut();
    paint::stroke(field, (tile_x - 1, anvil), (tile_x + 1, anvil), structure);
    paint::stroke(field, (tile_x - 1, anvil - 1), (tile_x - 1, anvil - 1), structure);
    paint::stroke(field, (tile_x + 1, anvil - 1), (tile_x + 1, anvil - 1), structure);
    anvil - 1
}

/// Puts `count` cells along the bottom row of a tile, which is where a machine takes
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

/// The compactor is a piston: it crushes what is piled *under* it, not what falls into
/// it. Two tiles tall, so the tile it works on is the one below both of them.
#[test]
fn a_compactor_crushes_what_is_piled_under_it() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let definition = rules.entities.get(common::compactor_kind()).expect("compactor");
    let rate = definition.rate as i32;
    assert_eq!(definition.reach, sim_core::Reach::Below, "this test is about the reach");

    let mut world = scene::arena(200, 200, 1, &rules);
    let floor = press_rig(&mut world, 4, 6, structure, definition.height_tiles as i32);

    feed(&mut world, 4, floor, CELLS, burnt);
    world.step();

    assert_eq!(count_in_body(&world, 4, floor, fuel), rate, "it crushes what is under it");
    assert_eq!(count_in_body(&world, 4, floor, burnt), CELLS - rate);
    // And it leaves its own body alone — the ram works below, not inside.
    assert_eq!(count_in_body(&world, 4, 6, fuel), 0, "the ram works below, not inside");
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
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let definition = rules.entities.get(common::compactor_kind()).expect("compactor");
    let (rate, interval) = (definition.rate as i32, definition.interval as u64);
    assert!(
        interval > 1,
        "this test needs a paced machine, or it cannot tell pacing from its absence"
    );

    let mut world = scene::arena(200, 200, 1, &rules);
    let floor = press_rig(&mut world, 4, 6, structure, definition.height_tiles as i32);
    feed(&mut world, 4, floor, CELLS, burnt);

    // Tick 0 is on the beat.
    world.step();
    assert_eq!(count_in_body(&world, 4, floor, fuel), rate);

    // Every tick up to the next beat must change nothing at all.
    world.step_many(interval - 1);
    assert_eq!(
        count_in_body(&world, 4, floor, fuel),
        rate,
        "the machine kept working between beats"
    );

    // And the next beat lands, finishing the row. Less than a full rate's worth is
    // left by then — one row holds CELLS cells, and the first beat already took `rate`
    // of them — so this checks the beat happened, not that it moved a full load.
    world.step();
    assert_eq!(count_in_body(&world, 4, floor, fuel), CELLS);
    assert!(rate < CELLS, "a single beat should not be able to finish the whole row");
}

/// A compactor is not a demolition tool: it works on burnt residue and nothing else,
/// including the residue one stage *up* the chain — which now needs heat rather than a
/// machine — and the fuel one stage down that it makes itself.
#[test]
fn a_compactor_leaves_everything_else_alone() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let sand = rules.elements.id_of("sand").expect("sand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let residue = rules.elements.id_of("residue").expect("residue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let height = rules
        .entities
        .get(common::compactor_kind())
        .expect("compactor")
        .height_tiles as i32;

    for other in [structure, sand, gold, residue, fuel] {
        let mut world = scene::arena(200, 200, 1, &rules);
        let floor = press_rig(&mut world, 4, 6, structure, height);
        feed(&mut world, 4, floor, CELLS, other);
        world.step_many(20);
        assert_eq!(
            count_in_body(&world, 4, floor, other),
            CELLS,
            "a compactor touched element {other}, which is not its input"
        );
    }
}

#[test]
fn an_idle_compactor_reads_as_blocked() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let definition = rules.entities.get(common::compactor_kind()).expect("compactor");
    let mut world = scene::arena(200, 200, 1, &rules);
    let compactor = common::compactor(4, 6);
    let floor = press_rig(&mut world, 4, 6, structure, definition.height_tiles as i32);

    assert!(
        compactor.is_blocked(definition, world.field(), &rules.elements),
        "nothing is heaped under it yet"
    );
    feed(&mut world, 4, floor, CELLS, burnt);
    assert!(!compactor.is_blocked(definition, world.field(), &rules.elements));
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
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let height = rules
        .entities
        .get(common::compactor_kind())
        .expect("compactor")
        .height_tiles as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    let floor = press_rig(&mut world, 4, 6, structure, height);
    feed(&mut world, 4, floor, CELLS, burnt);

    assert!(world.retune(0, |entity| entity.enabled = false), "there is a machine at 0");
    world.step_many(200);
    assert_eq!(
        count_in_body(&world, 4, floor, fuel),
        0,
        "a switched-off machine refined something anyway"
    );
    assert_eq!(
        count_in_body(&world, 4, floor, burnt),
        CELLS,
        "and its input should still be sitting there untouched"
    );

    world.retune(0, |entity| entity.enabled = true);
    world.step_many(200);
    assert_eq!(
        count_in_body(&world, 4, floor, fuel),
        CELLS,
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
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let definition = rules.entities.get(common::compactor_kind()).expect("compactor");
    assert!(definition.rate > 1, "this test needs room to slow the machine down");

    let mut world = scene::arena(200, 200, 1, &rules);
    let floor = press_rig(&mut world, 4, 6, structure, definition.height_tiles as i32);
    feed(&mut world, 4, floor, CELLS, burnt);

    world.retune(0, |entity| entity.rate = 1);
    world.step();
    assert_eq!(
        count_in_body(&world, 4, floor, fuel),
        1,
        "one beat at a retuned rate of 1 should convert exactly one cell"
    );

    // Zero is not "off" — it means defer to the type, which is why placement can use it
    // as "unset" without a separate flag.
    world.retune(0, |entity| entity.rate = 0);
    world.step_many(definition.interval as u64);
    assert_eq!(
        count_in_body(&world, 4, floor, fuel),
        1 + definition.rate as i32,
        "a rate of zero should fall back to the type's"
    );
}
