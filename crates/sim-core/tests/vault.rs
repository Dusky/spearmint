//! A vault is a place, not a machine.
//!
//! The player digs a pit, walls it, and declares the inside a vault; gold that lands in
//! one is money and gold anywhere else is not (spec 5.1). Its footprint is whatever was
//! marked out, so the tests here are mostly about the region — that it can be any size,
//! that it holds what falls in, and that spending reaches into it and nowhere else.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::{paint, scene, World};

const CELLS: i32 = TILE_CELLS as i32;

/// Fills a tile's worth of cells with an element, the way the draw tool would.
fn fill(world: &mut World, tile_x: i32, tile_y: i32, id: u8) {
    paint::stroke(world.field_mut(), (tile_x, tile_y), (tile_x, tile_y), id);
}

#[test]
fn a_vault_is_whatever_size_the_player_marked_out() {
    let rules = common::rules_without_reactions();
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(400, 400, 1, &rules);

    world.place(common::vault(4, 6, 3, 2));

    let vault = world.entities()[0];
    assert_eq!((vault.width_tiles, vault.height_tiles), (3, 2));
    assert!(vault.covers(4, 6) && vault.covers(6, 7), "the whole region");
    assert!(!vault.covers(7, 6) && !vault.covers(4, 8), "and no further");

    // Every tile of it holds money, not just the first.
    for tile_x in 4..7 {
        for tile_y in 6..8 {
            fill(&mut world, tile_x, tile_y, gold);
        }
    }
    assert_eq!(world.stored(), (3 * 2 * CELLS * CELLS) as u64);
}

/// The rule the whole design rests on, now that storage is a thing you build.
#[test]
fn only_gold_inside_a_vault_counts() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(400, 400, 1, &rules);

    world.place(common::vault(4, 6, 1, 1));
    fill(&mut world, 4, 6, gold);

    // A pile lying on the arena floor, and another sitting inside a press — neither
    // is in a vault, so neither is money.
    fill(&mut world, 12, 20, gold);
    world.place(common::press(20, 6));
    paint::stroke(world.field_mut(), (20, 7), (20, 7), wall);
    fill(&mut world, 20, 6, gold);

    assert_eq!(world.stored(), (CELLS * CELLS) as u64, "only the vault");
    assert_eq!(
        world.count_of(gold),
        (3 * CELLS * CELLS) as usize,
        "the rest is still gold, and still in the world"
    );
}

#[test]
fn spending_takes_the_exact_amount_or_nothing() {
    let rules = common::rules_without_reactions();
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(400, 400, 1, &rules);
    world.place(common::vault(4, 6, 1, 1));
    fill(&mut world, 4, 6, gold);

    let held = world.stored();
    assert_eq!(world.spend(held + 1), 0);
    assert_eq!(world.stored(), held, "a refused purchase costs nothing");

    assert_eq!(world.spend(4), 4);
    assert_eq!(world.stored(), held - 4);

    assert_eq!(world.spend(held - 4), held - 4);
    assert_eq!(world.stored(), 0, "the vault is empty");
}

#[test]
fn spending_never_reaches_gold_outside_a_vault() {
    let rules = common::rules_without_reactions();
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(400, 400, 1, &rules);
    world.place(common::vault(4, 6, 1, 1));
    fill(&mut world, 12, 20, gold);

    let loose = world.count_of(gold);
    assert_eq!(world.spend(1), 0, "the vault is empty");
    assert_eq!(world.count_of(gold), loose, "the loose pile is untouched");
}

/// The whole point of separating minting from storage: a press over an open pit
/// drops its nuggets into the vault below, and gravity is the only transport involved.
#[test]
fn nuggets_fall_from_a_press_into_the_vault_below() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let mut world = scene::arena(400, 400, 3, &rules);

    // A chute down to a press, and a walled pit under it declared a vault.
    paint::stroke(world.field_mut(), (3, 6), (3, 9), wall);
    paint::stroke(world.field_mut(), (5, 6), (5, 9), wall);
    paint::stroke(world.field_mut(), (3, 10), (3, 12), wall);
    paint::stroke(world.field_mut(), (5, 10), (5, 12), wall);
    paint::stroke(world.field_mut(), (3, 13), (5, 13), wall);
    world.place(common::press(4, 9));
    world.place(common::vault(4, 10, 1, 3));

    for tile_y in 6..9 {
        fill(&mut world, 4, tile_y, wet);
    }
    world.step_many(2_000);

    let minted = world.collected();
    assert!(minted > 0, "nothing was pressed");
    assert_eq!(
        world.stored(),
        minted,
        "every nugget should have fallen into the vault"
    );
}

/// The mechanism the vault design leans on: a denser powder sinks through a lighter
/// one, the same rule that already lets sand sink through water, extended from fluids
/// to powders (spec 5.1). Gold poured on top of a wetSand-filled pit should end up
/// underneath it, not sitting on top where it fell.
#[test]
fn gold_sinks_below_a_lighter_powder_it_is_poured_onto() {
    let rules = common::rules_without_reactions();
    let wall = rules.elements.id_of("wall").expect("wall");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(400, 400, 1, &rules);

    // A four-tile-deep, walled and floored pit.
    paint::stroke(world.field_mut(), (3, 6), (3, 9), wall);
    paint::stroke(world.field_mut(), (5, 6), (5, 9), wall);
    paint::stroke(world.field_mut(), (3, 10), (5, 10), wall);

    // Three tiles of wet sand at the bottom, gold poured on top of it last.
    for tile_y in 7..10 {
        fill(&mut world, 4, tile_y, wet);
    }
    fill(&mut world, 4, 6, gold);

    world.step_many(3_000);

    // The bottom tile should now be gold, not wet sand — it sank through everything
    // lighter that used to be underneath it.
    let mut gold_at_bottom = 0;
    for row in 0..CELLS {
        for col in 0..CELLS {
            if world.get(4 * CELLS + col, 9 * CELLS + row) == gold {
                gold_at_bottom += 1;
            }
        }
    }
    assert!(
        gold_at_bottom > (CELLS * CELLS) / 2,
        "gold should have settled to the bottom of the pit, found {gold_at_bottom} cells there"
    );
}
