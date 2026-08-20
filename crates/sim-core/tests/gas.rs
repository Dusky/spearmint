//! Gas: the third state of matter, and phase changes that run both ways.
//!
//! `State::Gas` was an empty match arm for the whole life of the project, and
//! `heat.rs` has been able to boil things into `boils_into` since heat landed — with
//! nothing in the roster that was a gas or declared a boiling target, so none of it
//! could ever fire. This file covers what happens now that it can.

mod common;

use sim_core::field::CellField;
use sim_core::heat::AMBIENT_TEMPERATURE;
use sim_core::{scene, ElementId, World, EMPTY};

fn id(name: &str) -> ElementId {
    common::table()
        .id_of(name)
        .unwrap_or_else(|| panic!("{name}"))
}

/// An arena with no chemistry in it. Every test here is about movement or about a phase
/// change, and a sand/water reaction firing in the background would only add noise.
fn arena(width: u32, height: u32) -> World {
    scene::arena(width, height, 1, &common::rules_without_reactions())
}

/// Where the only cell of `element` is, if there is exactly one.
fn sole_cell(world: &World, element: ElementId) -> Option<(i32, i32)> {
    let bounds = world.field().bounds()?;
    let mut found = None;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            if world.get(x, y) == element {
                assert!(found.is_none(), "expected a single cell of the element");
                found = Some((x, y));
            }
        }
    }
    found
}

/// Hot enough that it will not condense the moment it exists.
///
/// Steam below its boiling point turns back into water, correctly and immediately, so a
/// test about how a gas *moves* has to put it in the world at a temperature it can
/// survive at — the same way a test about water does not start it above boiling.
const HOT: i16 = 500;

fn count(world: &World, element: ElementId) -> usize {
    let Some(bounds) = world.field().bounds() else {
        return 0;
    };
    let mut total = 0;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            if world.get(x, y) == element {
                total += 1;
            }
        }
    }
    total
}

/// The sweep-order hazard, and the reason to pin it.
///
/// Rows are visited bottom-first so a falling particle cannot be picked up twice in one
/// tick. Something moving *up* runs with that grain rather than against it: without the
/// `moved` bitmap it would rise into a not-yet-visited row, be found again, and ride the
/// sweep to the ceiling in a single tick. `try_move` marks both cells, which is what
/// stops it — and which is easy to break without noticing.
#[test]
fn a_gas_rises_exactly_one_cell_per_tick() {
    let steam = id("steam");
    let mut world = arena(40, 40);
    world.field_mut().set(20, 30, steam);
    world.field_mut().set_temperature(20, 30, HOT);

    for expected in (1..=8).map(|elapsed| 30 - elapsed) {
        world.step();
        let (x, y) = sole_cell(&world, steam).expect("the steam should still exist");
        assert_eq!(
            y, expected,
            "steam should climb one row per tick, not several"
        );
        assert_eq!(
            x, 20,
            "and should not wander sideways while it has room above"
        );
    }
}

/// Buoyancy is `displaces` seen from underneath.
///
/// `displaces` asks whether the mover is *heavier* than what it is moving into, which is
/// right for sand sinking through water and exactly backwards for steam under it. Before
/// `rises_through`, a gas made at the bottom of a tank would have stayed there forever.
#[test]
fn a_gas_bubbles_up_through_a_liquid_and_the_liquid_takes_its_place() {
    let (steam, water) = (id("steam"), id("water"));
    let mut world = arena(40, 40);

    // A short column of water with one cell of steam underneath it.
    for y in 26..=30 {
        world.field_mut().set(20, y, water);
    }
    world.field_mut().set(20, 31, steam);
    world.field_mut().set_temperature(20, 31, HOT);

    world.step();

    assert_eq!(
        world.get(20, 30),
        steam,
        "the steam should have risen into the water"
    );
    assert_eq!(
        world.get(20, 31),
        water,
        "and the water should have taken its place"
    );
    // A swap, so nothing was created or destroyed doing it (spec 1.1).
    assert_eq!(
        count(&world, water),
        5,
        "water should be conserved by the swap"
    );
    assert_eq!(count(&world, steam), 1, "and so should the steam");
}

/// Buoyancy must not become "moves through anything".
///
/// A sealed chamber makes both halves of that checkable at once: released at the bottom,
/// the gas must end up at the top of the chamber — it rose rather than sank — and it must
/// still be inside it. An open lid would not do, because a gas slipping diagonally around
/// a short one and carrying on up is correct behaviour, not a leak.
#[test]
fn a_gas_rises_to_the_top_of_a_sealed_chamber_and_no_further() {
    let steam = id("steam");
    // Built from `beltStructure`, whose thermal conductivity is 0.0, so the chamber
    // cannot cool its contents. A `structure` chamber would work too and then condense
    // the steam away mid-test — correct physics, and it would leave this test asserting
    // something about heat rather than about movement.
    let inert = id("beltStructure");
    let mut world = arena(40, 40);

    // Walls from (18,18) to (22,24); the interior is x 19..=21, y 19..=23.
    for y in 18..=24 {
        for x in 18..=22 {
            if y == 18 || y == 24 || x == 18 || x == 22 {
                world.field_mut().set(x, y, inert);
            }
        }
    }

    world.field_mut().set(20, 23, steam);
    world.field_mut().set_temperature(20, 23, HOT);
    world.step_many(60);

    let (x, y) = sole_cell(&world, steam).expect("the steam should still be in there");
    assert_eq!(y, 19, "it should have risen to the top of the chamber");
    assert!(
        (19..=21).contains(&x),
        "and must not have passed through a wall to get there: x={x}"
    );
}

/// The round's one subtle correctness property.
///
/// A cell sitting exactly on a threshold has a transition available in both directions.
/// If both used `>=`, it would boil, condense, boil, condense — changing identity every
/// tick forever, never settling and never letting its chunk sleep. `>=` going up and `<`
/// coming down leaves exactly one cell of hysteresis, which is enough.
#[test]
fn a_cell_parked_on_its_threshold_settles_instead_of_oscillating() {
    let (steam, water) = (id("steam"), id("water"));
    let boiling = i16::try_from(common::table().get(water).expect("water").boiling_point)
        .expect("a boiling point that fits");

    let mut world = arena(40, 40);
    // Boxed into a pocket of `beltStructure`, whose thermal conductivity is 0.0. That is
    // what isolates the question: a `structure` box would bleed the cell's heat away
    // within a tick or two and it would cross the threshold for an ordinary reason,
    // proving nothing about what happens while it is sitting exactly on it.
    let inert = id("beltStructure");
    for y in 19..=21 {
        for x in 19..=21 {
            world.field_mut().set(x, y, inert);
        }
    }
    // All eight neighbours, not just the four orthogonal ones: a liquid that cannot fall
    // straight down slides diagonally instead, and would simply leave.
    world.field_mut().set(20, 20, water);
    world.field_mut().set_temperature(20, 20, boiling);

    world.step();
    let settled = world.get(20, 20);
    assert_eq!(
        settled, steam,
        "at exactly the boiling point it should be steam"
    );

    // And it stays steam. With nothing to conduct into, the temperature cannot drift, so
    // any change here would be the two branches undoing each other rather than physics.
    for _ in 0..500 {
        world.step();
        assert_eq!(
            world.field().temperature(20, 20),
            boiling,
            "the pocket is inert, so the temperature should not have moved at all"
        );
        assert_eq!(
            world.get(20, 20),
            settled,
            "a cell parked on its threshold should not flip identity every tick"
        );
    }
}

/// The whole water cycle, with nothing scripting it.
///
/// Heat a pool past boiling and steam appears, rises, sheds its warmth into the cold
/// arena wall it ends up against, condenses, and falls back as water. Every step of that
/// is a rule that already existed applied to one new element.
#[test]
fn boiled_water_rises_condenses_and_comes_back_down() {
    let (steam, water) = (id("steam"), id("water"));
    let mut world = arena(40, 40);

    let pool: Vec<(i32, i32)> = (10..=16)
        .flat_map(|x| (30..=32).map(move |y| (x, y)))
        .collect();
    for &(x, y) in &pool {
        world.field_mut().set(x, y, water);
        world.field_mut().set_temperature(x, y, 500);
    }
    let charge = pool.len();

    // It boils promptly — every cell is already well past the threshold.
    world.step();
    assert!(count(&world, steam) > 0, "hot water should boil into steam");

    // The steam climbs. Somewhere in the top half of the arena within a few seconds of
    // simulated time is a claim about it rising, without pinning a rate.
    let mut reached_the_top = false;
    for _ in 0..120 {
        world.step();
        let bounds = world.field().bounds().expect("a non-empty world");
        for y in bounds.min_y..20 {
            for x in bounds.min_x..=bounds.max_x {
                if world.get(x, y) == steam {
                    reached_the_top = true;
                }
            }
        }
        if reached_the_top {
            break;
        }
    }
    assert!(
        reached_the_top,
        "steam should reach the upper half of the arena"
    );

    // And it comes back. The wall it is resting against sits at ambient, so it cools
    // through the boiling point and condenses.
    let mut condensed = false;
    for _ in 0..4000 {
        world.step();
        if count(&world, steam) == 0 && count(&world, water) > 0 {
            condensed = true;
            break;
        }
    }
    assert!(
        condensed,
        "steam against a cold wall should condense back to water"
    );

    // Matter is conserved across both phase changes: a transition is a change of
    // identity in place, never a creation or a deletion.
    assert_eq!(
        count(&world, water),
        charge,
        "every cell that boiled should have come back"
    );
}

/// Melting produces something the player keeps.
///
/// `moltenSand` used to be a dead end — its own data note said so. With `freezesInto` it
/// sets as glass wherever it stopped flowing, which is what makes melting sand a
/// production step rather than a way to lose it.
#[test]
fn molten_sand_sets_as_glass_once_it_cools_and_stays_glass() {
    let (sand, molten, glass) = (id("sand"), id("moltenSand"), id("glass"));
    let melting = common::table().get(sand).expect("sand").melting_point;

    let mut world = arena(40, 40);
    // On the arena floor, so it has somewhere to rest while it cools.
    world.field_mut().set(20, 38, sand);

    // Held above the melting point for a few ticks rather than set hot once. A lone cell
    // touching the cold floor sheds most of its heat within a tick — conduction runs
    // before the transition does — so a single write would cool back below the line
    // before it ever melted. This is what a heater does, stated directly.
    let mut melted = false;
    for _ in 0..12 {
        world
            .field_mut()
            .set_temperature(20, 38, melting as i16 + 400);
        world.step();
        if count(&world, molten) == 1 {
            melted = true;
        }
    }
    assert!(melted, "sand held past its melting point should melt");

    let mut set = false;
    for _ in 0..6000 {
        world.step();
        if count(&world, glass) == 1 {
            set = true;
            break;
        }
    }
    assert!(set, "molten sand should set as glass once it cools");

    // Glass is the end of the road at ambient: it must not melt straight back, and sand
    // must not come back either — melting is meant to be a one-way upgrade.
    world.step_many(500);
    assert_eq!(count(&world, glass), 1, "cooled glass should stay glass");
    assert_eq!(count(&world, sand), 0, "and must not turn back into sand");

    let (x, y) = sole_cell(&world, glass).expect("the glass");
    assert!(
        world.field().temperature(x, y) < melting as i16,
        "which only means anything if it really has cooled below its melting point"
    );
}

/// The whole roster, dropped into a world at room temperature, and what it settles into.
///
/// Every threshold is a comparison against a number in the data file, and a typo in
/// either direction would transform the world on tick one. This checks the two claims
/// that must hold for all of them at once — and covers elements that do not exist yet,
/// since it reads the roster rather than a list written here.
///
/// Note what it does *not* claim: that nothing changes. `moltenSand` correctly becomes
/// glass at room temperature, because room temperature is a long way below its melting
/// point — an element that declares a cooling transition is supposed to take it.
#[test]
fn the_whole_roster_settles_at_room_temperature() {
    let rules = common::rules_without_reactions();
    let mut world = arena(60, 40);

    let mut placed = Vec::new();
    for (index, element) in rules.elements.iter().enumerate() {
        if element.id == EMPTY {
            continue;
        }
        // Spread along the floor, two apart so neighbours cannot react or pile together.
        let x = 5 + index as i32 * 4;
        world.field_mut().set(x, 38, element.id);
        placed.push(element.clone());
    }
    assert!(placed.len() > 8, "this should cover the whole roster");

    // Measured against a baseline rather than against `placed.len()`, because the arena
    // builds its own walls out of `structure` and those are cells too.
    let census = |world: &World| -> Vec<usize> {
        rules
            .elements
            .iter()
            .map(|element| count(world, element.id))
            .collect()
    };
    let before = census(&world);

    world.step_many(200);
    let after = census(&world);

    // Nothing may vanish. A transition changes identity in place, so the total is the
    // same however many of them fired.
    assert_eq!(
        after.iter().sum::<usize>(),
        before.iter().sum::<usize>(),
        "matter must be conserved: a phase change is a change of identity, not a deletion"
    );

    // And anything untouched by the cooling rules must be not just present but exactly as
    // it was — it may have fallen, it may not have become something else. That means
    // skipping both ends of every cooling transition: `moltenSand` is expected to lose a
    // cell, and `glass` and `water` are expected to gain the ones it and `steam` lose.
    let mut involved = std::collections::BTreeSet::new();
    for element in rules.elements.iter() {
        for target in [element.condenses_into, element.freezes_into] {
            if target != EMPTY {
                involved.insert(element.id);
                involved.insert(target);
            }
        }
    }

    for (index, element) in rules.elements.iter().enumerate() {
        if involved.contains(&element.id) {
            continue;
        }
        assert_eq!(
            after[index], before[index],
            "{} is not part of any cooling transition and should be unchanged at ambient",
            element.name
        );
    }

    assert_eq!(
        world.field().temperature(5, 38),
        AMBIENT_TEMPERATURE,
        "and nothing should have heated itself"
    );
}
