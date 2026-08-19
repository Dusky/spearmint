//! One cell in, one cell out.
//!
//! A burner and a compactor are the same mechanism — `Behaviour::Refine` — reading two
//! different rows of data: which element they accept, and what it becomes. So the
//! things worth testing are the accepted input, the refusal to touch anything else
//! (including its own output), and that the chain end to end conserves matter the same
//! way pressing does.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{paint, scene, Entity, World};

const CELLS: i32 = TILE_CELLS as i32;

/// A machine sealed into a pocket of wall — left, right, and floored — so a fed charge
/// stays put to be inspected mid-tick rather than falling anywhere.
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

#[test]
fn a_burner_turns_residue_into_burnt_residue() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let residue = rules.elements.id_of("residue").expect("residue");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let rate = rules
        .entities
        .get(common::burner_kind())
        .expect("burner")
        .rate as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::burner(4, 6));

    feed(&mut world, 4, 6, CELLS, residue);
    world.step();

    assert_eq!(count_in_body(&world, 4, 6, burnt), rate, "one tick, one rate's worth");
    assert_eq!(count_in_body(&world, 4, 6, residue), CELLS - rate);
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
    let residue = rules.elements.id_of("residue").expect("residue");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let definition = rules.entities.get(common::burner_kind()).expect("burner");
    let (rate, interval) = (definition.rate as i32, definition.interval as u64);
    assert!(
        interval > 1,
        "this test needs a paced machine, or it cannot tell pacing from its absence"
    );

    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::burner(4, 6));
    feed(&mut world, 4, 6, CELLS, residue);

    // Tick 0 is on the beat.
    world.step();
    assert_eq!(count_in_body(&world, 4, 6, burnt), rate);

    // Every tick up to the next beat must change nothing at all.
    world.step_many(interval - 1);
    assert_eq!(
        count_in_body(&world, 4, 6, burnt),
        rate,
        "the machine kept working between beats"
    );

    // And the next beat lands, finishing the row. Less than a full rate's worth is
    // left by then — one row holds CELLS cells, and the first beat already took `rate`
    // of them — so this checks the beat happened, not that it moved a full load.
    world.step();
    assert_eq!(count_in_body(&world, 4, 6, burnt), CELLS);
    assert!(rate < CELLS, "a single beat should not be able to finish the whole row");
}

#[test]
fn a_compactor_turns_burnt_residue_into_fuel() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let rate = rules
        .entities
        .get(common::compactor_kind())
        .expect("compactor")
        .rate as i32;
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::compactor(4, 6));

    feed(&mut world, 4, 6, CELLS, burnt);
    world.step();

    assert_eq!(count_in_body(&world, 4, 6, fuel), rate);
    assert_eq!(count_in_body(&world, 4, 6, burnt), CELLS - rate);
}

/// A burner is not a demolition tool, and not a compactor: it works on residue and
/// nothing else, including the raw product a press would recognise and the burnt
/// residue one stage further down the chain.
#[test]
fn a_burner_leaves_everything_else_alone() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let sand = rules.elements.id_of("sand").expect("sand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");

    for other in [structure, sand, gold, burnt] {
        let mut world = scene::arena(200, 200, 1, &rules);
        hopper(&mut world, 4, 6, structure, common::burner(4, 6));
        feed(&mut world, 4, 6, CELLS, other);
        world.step_many(20);
        assert_eq!(
            count_in_body(&world, 4, 6, other),
            CELLS,
            "a burner touched element {other}, which is not its input"
        );
    }
}

#[test]
fn an_idle_burner_reads_as_blocked() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let residue = rules.elements.id_of("residue").expect("residue");
    let definition = rules.entities.get(common::burner_kind()).expect("burner");
    let mut world = scene::arena(200, 200, 1, &rules);
    let burner = common::burner(4, 6);
    hopper(&mut world, 4, 6, structure, burner);

    assert!(
        burner.is_blocked(definition, world.field(), &rules.elements),
        "nothing has fallen in yet"
    );
    feed(&mut world, 4, 6, CELLS, residue);
    assert!(!burner.is_blocked(definition, world.field(), &rules.elements));
}

/// The chain end to end, tested the way `press.rs`'s accounting test is: not by routing
/// through gravity between three separate machines, but by conservation. Each stage is
/// exactly one cell in for one cell out, so residue fed in should equal fuel that comes
/// out, with nothing lost or gained along the way.
#[test]
fn the_chain_conserves_every_cell_from_residue_to_fuel() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let structure = table.id_of("structure").expect("structure");
    let residue = table.id_of("residue").expect("residue");
    let burnt = table.id_of("burntResidue").expect("burntResidue");
    let fuel = table.id_of("fuel").expect("fuel");

    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::burner(4, 6));
    hopper(&mut world, 4, 9, structure, common::compactor(4, 9));

    feed(&mut world, 4, 6, CELLS, residue);
    world.step_many(20);
    assert_eq!(count_in_body(&world, 4, 6, residue), 0, "the burner should have kept up");
    let burnt_count = count_in_body(&world, 4, 6, burnt);
    assert_eq!(burnt_count, CELLS, "every grain became burnt residue");

    // Move what the burner made into the compactor directly — this test is about the
    // chain's conservation, not about routing burnt residue between two machines by
    // hand, which is a drawing exercise `vault.rs` already covers for one machine.
    for offset in 0..CELLS {
        let id = world.get(4 * CELLS + offset, 6 * CELLS + (CELLS - 1));
        world.field_mut().set(4 * CELLS + offset, 9 * CELLS + (CELLS - 1), id);
        world.field_mut().set(4 * CELLS + offset, 6 * CELLS + (CELLS - 1), 0);
    }
    world.step_many(20);

    assert_eq!(count_in_body(&world, 4, 9, burnt), 0, "the compactor should have kept up");
    assert_eq!(
        count_in_body(&world, 4, 9, fuel),
        burnt_count,
        "every cell of burnt residue should have become exactly one cell of fuel"
    );
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
    let residue = rules.elements.id_of("residue").expect("residue");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::burner(4, 6));
    feed(&mut world, 4, 6, CELLS, residue);

    assert!(world.retune(0, |entity| entity.enabled = false), "there is a machine at 0");
    world.step_many(200);
    assert_eq!(
        count_in_body(&world, 4, 6, burnt),
        0,
        "a switched-off machine refined something anyway"
    );
    assert_eq!(
        count_in_body(&world, 4, 6, residue),
        CELLS,
        "and its input should still be sitting there untouched"
    );

    world.retune(0, |entity| entity.enabled = true);
    world.step_many(200);
    assert_eq!(
        count_in_body(&world, 4, 6, burnt),
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
    let residue = rules.elements.id_of("residue").expect("residue");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let definition = rules.entities.get(common::burner_kind()).expect("burner");
    assert!(definition.rate > 1, "this test needs room to slow the machine down");

    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, structure, common::burner(4, 6));
    feed(&mut world, 4, 6, CELLS, residue);

    world.retune(0, |entity| entity.rate = 1);
    world.step();
    assert_eq!(
        count_in_body(&world, 4, 6, burnt),
        1,
        "one beat at a retuned rate of 1 should convert exactly one cell"
    );

    // Zero is not "off" — it means defer to the type, which is why placement can use it
    // as "unset" without a separate flag.
    world.retune(0, |entity| entity.rate = 0);
    world.step_many(definition.interval as u64);
    assert_eq!(
        count_in_body(&world, 4, 6, burnt),
        1 + definition.rate as i32,
        "a rate of zero should fall back to the type's"
    );
}
