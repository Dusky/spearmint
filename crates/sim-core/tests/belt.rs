//! Conveying, and filtering (spec 4.1-4.3).
//!
//! A belt holds cargo up with real solid matter written into its own footprint at
//! placement, not a rule `step.rs` has to know about — so what is worth testing is that
//! this actually behaves like ground (nothing ever falls through it, regardless of
//! density), that cargo advances exactly one cell a tick even across a tile boundary,
//! and that a filter diverts only the element it names while conveying everything else.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::scene;

const CELLS: i32 = TILE_CELLS as i32;

/// A belt shifts a single riding particle exactly one cell per tick, and never more.
#[test]
fn a_belt_shifts_cargo_one_cell_a_tick() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(200, 200, 1, &rules);

    let entity = common::belt(2, 1, 1);
    world.place(entity);

    let carry_y = entity.top() - 1;
    let start_x = entity.left();
    world.field_mut().set(start_x, carry_y, sand);

    world.step();
    assert_eq!(
        world.get(start_x, carry_y),
        sim_core::EMPTY,
        "cargo should have left its cell"
    );
    assert_eq!(
        world.get(start_x + 1, carry_y),
        sand,
        "cargo should have advanced exactly one cell"
    );
    assert_eq!(
        world.get(start_x + 2, carry_y),
        sim_core::EMPTY,
        "and no further than that"
    );

    world.step();
    assert_eq!(world.get(start_x + 1, carry_y), sim_core::EMPTY);
    assert_eq!(
        world.get(start_x + 2, carry_y),
        sand,
        "a second tick advances it one more cell"
    );
}

/// A belt facing the other way conveys leftward instead.
#[test]
fn a_belt_conveys_in_its_declared_direction() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(200, 200, 1, &rules);

    let entity = common::belt(5, 1, -1);
    world.place(entity);

    let carry_y = entity.top() - 1;
    let start_x = entity.left() + TILE_CELLS as i32 - 1;
    world.field_mut().set(start_x, carry_y, sand);

    world.step();
    assert_eq!(world.get(start_x, carry_y), sim_core::EMPTY);
    assert_eq!(world.get(start_x - 1, carry_y), sand);
}

/// A run of several belt tiles shifts a whole loaded stretch of its row by exactly one
/// cell, including across the seam between two tiles — the case that would double-move
/// cargo if entities were not processed downstream-first (see `entities::convey`).
#[test]
fn a_belt_run_conveys_without_cascading_across_tiles() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(200, 200, 1, &rules);

    // Three belt tiles in a row, tiles 0..3, all facing right.
    for tile in 0..3 {
        world.place(common::belt(tile, 1, 1));
    }

    let carry_y = CELLS - 1;
    // A run of cargo straddling the seam between tile 0 (columns 0..9) and tile 1
    // (columns 9..18): columns 5..11.
    let loaded: Vec<i32> = (5..11).collect();
    for &x in &loaded {
        world.field_mut().set(x, carry_y, sand);
    }

    world.step();

    // The whole block should have shifted by exactly one cell: the trailing edge
    // vacates, the interior stays filled (each cell's old occupant moved on, but the
    // cell behind it moved in — so it reads as still-occupied), and the leading edge
    // gains exactly one new cell, no more.
    let first = *loaded.first().expect("non-empty");
    let last = *loaded.last().expect("non-empty");
    assert_eq!(
        world.get(first, carry_y),
        sim_core::EMPTY,
        "trailing edge should have vacated"
    );
    for x in (first + 1)..=(last + 1) {
        assert_eq!(
            world.get(x, carry_y),
            sand,
            "column {x} should hold cargo after the shift"
        );
    }
    assert_eq!(
        world.get(last + 2, carry_y),
        sim_core::EMPTY,
        "the leading edge must not have advanced two cells in one tick"
    );
    assert_eq!(
        world.count_of(sand),
        loaded.len(),
        "conveying must not create or destroy cargo"
    );
}

/// A downstream jam backs cargo up rather than losing it, and is reported as blocked.
#[test]
fn a_blocked_belt_backs_up_instead_of_losing_cargo() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let wall = rules.elements.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 1, &rules);

    let entity = common::belt(2, 1, 1);
    let definition = rules.entities.get(common::belt_kind()).expect("belt");
    world.place(entity);

    let carry_y = entity.top() - 1;
    let (x0, y0, x1, _) = entity.body();
    // Wall off both the carrying row and the row below it, on both sides of the tile:
    // a single-cell sliver would still leave a diagonal escape at cargo's outer edge
    // (powder slides diagonally, spec 2.2), so a real pocket needs both rows walled.
    for &x in &[x0 - 1, x1 + 1] {
        world.field_mut().set(x, carry_y, wall);
        world.field_mut().set(x, y0, wall);
    }

    // Load the whole carrying row.
    for x in x0..=x1 {
        world.field_mut().set(x, carry_y, sand);
    }

    let before = world.count_of(sand);
    world.step_many(20);
    assert_eq!(world.count_of(sand), before, "a jam must not lose cargo");
    assert!(
        entity.is_blocked(definition, world.field(), &rules.elements),
        "cargo resting against the wall should read as blocked"
    );
}

/// Solid ground, not a special case: nothing rests inside a belt's own footprint, ever
/// — including something denser than everything else, which would otherwise sink
/// through a powder floor (spec's density-sinking rule only ever applies between
/// non-solid states).
#[test]
fn nothing_falls_through_a_belts_own_body() {
    let rules = common::rules();
    let gold = rules.elements.id_of("gold").expect("gold");
    let structure = rules
        .elements
        .id_of("beltStructure")
        .expect("beltStructure");
    let mut world = scene::arena(200, 200, 1, &rules);

    let entity = common::belt(2, 1, 1);
    world.place(entity);

    let (x0, y0, x1, y1) = entity.body();
    for y in y0..=y1 {
        for x in x0..=x1 {
            assert_eq!(
                world.get(x, y),
                structure,
                "a belt's footprint should be solid at placement"
            );
        }
    }

    // Gold is denser than anything else in the game — if density sinking ever reached
    // solid ground, this is what would expose it.
    let carry_y = y0 - 1;
    world.field_mut().set(entity.left(), carry_y, gold);
    world.step_many(50);

    for y in y0..=y1 {
        for x in x0..=x1 {
            assert_eq!(
                world.get(x, y),
                structure,
                "the belt's body must stay solid"
            );
        }
    }
}

/// A filter lets its declared element drop straight through while carrying everything
/// else on, matching Sandustry's mechanic (spec 4.3).
#[test]
fn a_filter_drops_its_element_and_conveys_the_rest() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let gold = rules.elements.id_of("gold").expect("gold");
    let wall = rules.elements.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 1, &rules);

    let entity = common::filter(2, 1, 1, gold);
    world.place(entity);

    let carry_y = entity.top() - 1;
    let gold_x = entity.left() + 2;
    let sand_x = entity.left() + 5;
    world.field_mut().set(gold_x, carry_y, gold);
    world.field_mut().set(sand_x, carry_y, sand);
    // A floor one row below the mouth, wide enough to block a diagonal slide off either
    // side, so the dropped gold has somewhere to land and this test can check exactly
    // where — otherwise it is still an ordinary powder and keeps sliding through the
    // physics phase that runs later in the same tick.
    for x in (gold_x - 1)..=(gold_x + 1) {
        world.field_mut().set(x, entity.mouth() + 1, wall);
    }

    world.step();

    // Gold drops straight through to the row below the filter's own footprint.
    assert_eq!(
        world.get(gold_x, carry_y),
        sim_core::EMPTY,
        "gold should have left the carrying row"
    );
    assert_eq!(
        world.get(gold_x, entity.mouth()),
        gold,
        "gold should have dropped through"
    );

    // Sand, not the filtered element, keeps conveying instead.
    assert_eq!(world.get(sand_x, carry_y), sim_core::EMPTY);
    assert_eq!(
        world.get(sand_x + 1, carry_y),
        sand,
        "sand should still be conveyed, not dropped"
    );
}

/// A plain belt has no declared element, so it never diverts anything — everything
/// conveys.
#[test]
fn a_plain_belt_lets_nothing_through() {
    let rules = common::rules();
    let gold = rules.elements.id_of("gold").expect("gold");
    let mut world = scene::arena(200, 200, 1, &rules);

    let entity = common::belt(2, 1, 1);
    world.place(entity);

    let carry_y = entity.top() - 1;
    let x = entity.left();
    world.field_mut().set(x, carry_y, gold);

    world.step();

    assert_eq!(
        world.get(x, entity.mouth()),
        sim_core::EMPTY,
        "a plain belt must not drop anything through"
    );
    assert_eq!(
        world.get(x + 1, carry_y),
        gold,
        "it should simply convey instead"
    );
}
