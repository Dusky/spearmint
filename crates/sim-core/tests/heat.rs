//! Temperature: a real per-cell quantity that conducts between touching matter, and a
//! heater that finally gives fuel a consumer (spec 5.3).

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::heat::AMBIENT_TEMPERATURE;
use sim_core::{paint, scene, Entity, World};

const CELLS: i32 = TILE_CELLS as i32;

/// A machine sealed into a pocket of wall — left, right, and floored — so a fed charge
/// stays put and stays in thermal contact with something, rather than radiating into
/// open (and thus thermally silent) space. The same shape `refine.rs`'s own `hopper`
/// uses.
fn hopper(world: &mut World, tile_x: i32, tile_y: i32, wall: u8, machine: Entity) {
    world.place(machine);
    let field = world.field_mut();
    paint::stroke(field, (tile_x - 1, tile_y + 1), (tile_x + 1, tile_y + 1), wall);
    paint::stroke(field, (tile_x - 1, tile_y), (tile_x - 1, tile_y), wall);
    paint::stroke(field, (tile_x + 1, tile_y), (tile_x + 1, tile_y), wall);
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

/// Conduction moves heat toward equality and never past it, even when one element's
/// conductivity is above 1.0 (gold's is 3.17) — the case the clamp in `heat::exchange`
/// exists for. Both cells sit on a `beltStructure` floor: gold is a powder and needs
/// solid ground under it, but `beltStructure`'s own conductivity is `0.0` (spec 4.3 —
/// it is meant to be thermally inert, an indestructible belt floor lava cannot melt),
/// so the floor itself cannot become a second, uncontrolled heat path and confuse what
/// this test is isolating.
#[test]
fn conduction_never_overshoots_past_equality() {
    let rules = common::rules();
    let gold = rules.elements.id_of("gold").expect("gold");
    let structure = rules.elements.id_of("beltStructure").expect("beltStructure");
    let mut world = scene::arena(200, 200, 1, &rules);

    let (hot_x, cold_x, y) = (10, 11, 10);
    // A floor under both, and one cell wider on each side — a powder that cannot fall
    // straight down slides diagonally instead, so a single-cell-wide floor is not
    // enough to actually pin it in place.
    for x in (hot_x - 1)..=(cold_x + 1) {
        world.field_mut().set(x, y + 1, structure);
    }
    world.field_mut().set(hot_x, y, gold);
    world.field_mut().set(cold_x, y, gold);
    world.field_mut().set_temperature(hot_x, y, 2000);
    world.field_mut().set_temperature(cold_x, y, AMBIENT_TEMPERATURE);

    world.step();

    let hot_after = world.field().temperature(hot_x, y);
    let cold_after = world.field().temperature(cold_x, y);
    assert!(hot_after >= cold_after, "heat must not flip which side is hotter");
    assert!(
        (AMBIENT_TEMPERATURE..=2000).contains(&hot_after)
            && (AMBIENT_TEMPERATURE..=2000).contains(&cold_after),
        "neither side may end up outside the original range: hot={hot_after} cold={cold_after}"
    );
}

/// Heat does not conduct through empty space: two occupied cells separated by one
/// empty cell stay thermally independent. Floored on `beltStructure`, not wall — wall
/// conducts, and a contiguous strip of it under both cells would quietly become a
/// second path between them, defeating the point of this test.
#[test]
fn heat_does_not_cross_empty_space() {
    let rules = common::rules();
    let gold = rules.elements.id_of("gold").expect("gold");
    let structure = rules.elements.id_of("beltStructure").expect("beltStructure");
    let mut world = scene::arena(200, 200, 1, &rules);

    // Well clear of x = 0: `scene::arena` runs its own wall down that column, and a
    // cell placed there would be touching it — heat's way out, and nothing to do with
    // the gap this test is about.
    let (y, left, right) = (10, 10, 12);
    for x in (left - 1)..=(right + 1) {
        world.field_mut().set(x, y + 1, structure);
    }
    world.field_mut().set(left, y, gold);
    world.field_mut().set(right, y, gold);
    // The cell between them stays empty.
    world.field_mut().set_temperature(left, y, 2000);
    world.field_mut().set_temperature(right, y, AMBIENT_TEMPERATURE);

    world.step_many(10);

    assert_eq!(world.field().temperature(left, y), 2000, "nothing touches this cell");
    assert_eq!(
        world.field().temperature(right, y),
        AMBIENT_TEMPERATURE,
        "nothing touches this cell either"
    );
}

/// A fed heater holds its own footprint hot and consumes fuel; with none available, it
/// reads as blocked and does nothing.
///
/// Checked at the body's centre column, top row — the one point in a walled pocket that
/// touches nothing but other body cells the heater itself just set, so it is not also
/// bleeding heat sideways into the pocket wall within the same tick it was set. That
/// bleed is real and correct (conduction runs the instant something touches a cooler
/// neighbour); it is just not what this test is checking.
#[test]
fn a_fed_heater_holds_its_footprint_hot_and_burns_fuel() {
    let rules = common::rules();
    let wall = rules.elements.id_of("wall").expect("wall");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let definition = rules.entities.get(common::heater_kind()).expect("heater");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall, common::heater(4, 6));

    assert!(
        common::heater(4, 6).is_blocked(definition, world.field(), &rules.elements),
        "no fuel yet"
    );

    for offset in 0..CELLS {
        world.field_mut().set(4 * CELLS + offset, (6 + 1) * CELLS - 1, fuel);
    }

    let entity = common::heater(4, 6);
    assert!(
        !entity.is_blocked(definition, world.field(), &rules.elements),
        "fuel is loaded now"
    );

    world.step();

    assert_eq!(
        count_in_body(&world, 4, 6, fuel),
        CELLS - definition.rate as i32,
        "one tick, one rate's worth burned"
    );
    let interior = (4 * CELLS + CELLS / 2, 6 * CELLS);
    assert_eq!(
        world.field().temperature(interior.0, interior.1),
        definition.heat_output,
        "an interior cell, untouched by the surrounding wall, should read exactly hot"
    );
}

/// A heater has no cooldown logic of its own: while fed it forces its footprint hot,
/// and once it is not, whatever touching matter got heated cools back toward ambient
/// through the same generic conduction pass alone.
///
/// The probe is a cell of `wall` sitting inside the body, not the body itself and not
/// a powder. Both alternatives measure the wrong thing: once the fuel is burned the
/// body is *empty*, and an empty cell is not eligible for conduction (heat only exists
/// where matter does), so it keeps whatever it was last set to forever; and a powder
/// probe simply falls out of the cell being watched the moment the fuel under it
/// disappears, leaving that same stale reading behind. A solid stays put and conducts.
#[test]
fn matter_touching_an_unfed_heater_cools_through_ordinary_conduction() {
    let rules = common::rules();
    let wall = rules.elements.id_of("wall").expect("wall");
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let definition = rules.entities.get(common::heater_kind()).expect("heater");
    let mut world = scene::arena(200, 200, 1, &rules);
    hopper(&mut world, 4, 6, wall, common::heater(4, 6));

    // A slab of wall behind the pocket's left side, well beyond the single tile
    // `hopper` paints — a heat sink with enough mass not to saturate over the hundreds
    // of ticks this test runs, which one tile's worth would.
    for y in (CELLS)..(11 * CELLS) {
        for x in (0..(4 * CELLS)).rev() {
            world.field_mut().set(x, y, wall);
        }
    }

    // The probe: a solid at the body's top-left, touching the pocket wall, so once
    // heated it has somewhere to keep losing heat to.
    let probe = (4 * CELLS, 6 * CELLS);
    world.field_mut().set(probe.0, probe.1, wall);

    // Fuel along the body's bottom row, resting on the pocket floor so it stays put.
    for offset in 0..CELLS {
        world.field_mut().set(4 * CELLS + offset, (6 + 1) * CELLS - 1, fuel);
    }
    let rate = definition.rate as i32;
    let ticks_to_exhaust = ((CELLS + rate - 1) / rate) as u64;
    world.step_many(ticks_to_exhaust);

    let heated = world.field().temperature(probe.0, probe.1);
    assert!(heated > AMBIENT_TEMPERATURE, "the probe should have picked up real heat: {heated}");
    assert_eq!(count_in_body(&world, 4, 6, fuel), 0, "all the fed fuel should be gone by now");

    world.step_many(300);

    let cooled = world.field().temperature(probe.0, probe.1);
    assert!(
        cooled < heated,
        "with the heater unfed, ordinary conduction alone should have cooled it further: \
         heated={heated} cooled={cooled}"
    );
}

/// Sand held above its melting point becomes `moltenSand` — the real shipped
/// transition, not a fixture (this repo's own testing convention: run against the file
/// the game ships). Floored on `beltStructure`, not wall — wall conducts, and would
/// pull the temperature back below the melting point on the very tick this test means
/// to check it on, before the transition ever gets evaluated.
#[test]
fn sand_melts_into_molten_sand_above_its_melting_point() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let molten = rules.elements.id_of("moltenSand").expect("moltenSand");
    let structure = rules.elements.id_of("beltStructure").expect("beltStructure");
    let melting_point = rules.elements.get(sand).expect("sand").melting_point;
    let mut world = scene::arena(200, 200, 1, &rules);

    let (x, y) = (10, 10);
    // Three wide: a powder that cannot fall straight down slides diagonally instead.
    for floor_x in (x - 1)..=(x + 1) {
        world.field_mut().set(floor_x, y + 1, structure);
    }
    world.field_mut().set(x, y, sand);
    world.field_mut().set_temperature(x, y, melting_point as i16);

    world.step();

    assert_eq!(world.get(x, y), molten, "sand at its melting point should have melted");
}

/// Below the melting point, sand stays sand — the transition is a threshold, not a
/// gradual drift.
#[test]
fn sand_below_its_melting_point_stays_sand() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let wall = rules.elements.id_of("wall").expect("wall");
    let mut world = scene::arena(200, 200, 1, &rules);

    let (x, y) = (10, 10);
    for floor_x in (x - 1)..=(x + 1) {
        world.field_mut().set(floor_x, y + 1, wall);
    }
    world.field_mut().set(x, y, sand);
    world.field_mut().set_temperature(x, y, AMBIENT_TEMPERATURE);

    world.step_many(5);

    assert_eq!(world.get(x, y), sand);
}
