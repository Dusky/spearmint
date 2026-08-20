//! Burning: the first thing in the factory that reads temperature (spec 3.3, 5.3).
//!
//! Residue has no machine that converts it any more. It burns where it is hot enough,
//! and `flammability` is only the chance per tick that an eligible cell actually goes —
//! so *how long* a cell spends hot is what decides throughput. That is residence time,
//! and it comes out of the layout rather than out of a number on a machine.
//!
//! The kiln is what makes that buildable. It is `belt` with one field changed, so the
//! interesting claims are thermal, not mechanical: that a kiln heats what it carries,
//! that a plain belt provably does not, and that neither of them stops to do it.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::heat::AMBIENT_TEMPERATURE;
use sim_core::{paint, scene, Entity, World};

const CELLS: i32 = TILE_CELLS as i32;

/// A cell resting on the arena's floor, held at `temperature` for `ticks`.
///
/// Two details this has to get right or it tests nothing. Residue is a powder, so it
/// needs a floor or it simply falls; and the floor is matter, so it would otherwise
/// conduct the forced heat straight back out and leave the cell below its threshold by
/// the time `transition` reads it. Forcing both every tick — as a heater re-forces its
/// own footprint — leaves no gradient, so the cell is at exactly the stated temperature
/// when it is tested. These tests are about the threshold, not about conduction.
fn cook(world: &mut World, x: i32, floor_y: i32, temperature: i16, ticks: u64) -> u8 {
    let y = floor_y - 1;
    for _ in 0..ticks {
        world.field_mut().set_temperature(x, y, temperature);
        world.field_mut().set_temperature(x, floor_y, temperature);
        world.step();
    }
    world.get(x, y)
}

/// The row of the arena's bottom wall, which everything in these tests rests on.
const fn floor_of(height: i32) -> i32 {
    height - 1
}

/// Above its ignition point, residue becomes burnt residue. This is the whole
/// replacement for the burner.
#[test]
fn residue_burns_above_its_ignition_point() {
    let rules = common::rules_without_reactions();
    let residue_id = rules.elements.id_of("residue").expect("residue");
    let residue = rules.elements.get(residue_id).expect("residue");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let ignition = residue.ignition_point as i16;

    let mut world = scene::arena(60, 60, 1, &rules);
    let (x, floor) = (30, floor_of(60));
    world.field_mut().set(x, floor - 1, residue_id);

    assert_eq!(
        cook(&mut world, x, floor, ignition, 400),
        burnt,
        "residue held at its ignition point should burn"
    );
}

/// One degree below it, it never does — however long you wait. The threshold is a
/// threshold, not a bias.
#[test]
fn residue_below_its_ignition_point_never_burns() {
    let rules = common::rules_without_reactions();
    let residue_id = rules.elements.id_of("residue").expect("residue");
    let ignition = rules.elements.get(residue_id).expect("residue").ignition_point as i16;

    let mut world = scene::arena(60, 60, 1, &rules);
    let (x, floor) = (30, floor_of(60));
    world.field_mut().set(x, floor - 1, residue_id);

    assert_eq!(
        cook(&mut world, x, floor, ignition - 1, 2000),
        residue_id,
        "one degree under the threshold is under the threshold"
    );
}

/// Burning is one-way. What it produces must not itself burn, or the chain would eat
/// its own output and residue would never reach a compactor.
#[test]
fn what_burning_produces_does_not_itself_burn() {
    let rules = common::rules_without_reactions();
    let burnt_id = rules.elements.id_of("burntResidue").expect("burntResidue");
    let burnt = rules.elements.get(burnt_id).expect("burntResidue");
    assert_eq!(burnt.burns_into, sim_core::EMPTY, "burnt residue should not burn again");

    let mut world = scene::arena(60, 60, 1, &rules);
    let (x, floor) = (30, floor_of(60));
    world.field_mut().set(x, floor - 1, burnt_id);

    // Hot, but below its 1300K melting point, so melting cannot be what saves it.
    assert_eq!(cook(&mut world, x, floor, 1200, 1000), burnt_id);
}

/// Dwell time is the throughput dial. A cell that spends twice as long hot should burn
/// substantially more often — which is the property that makes belt *length* mean
/// something, and the reason `flammability` is a probability rather than a flag.
#[test]
fn longer_in_the_heat_burns_more_of_it() {
    let rules = common::rules_without_reactions();
    let residue_id = rules.elements.id_of("residue").expect("residue");
    let ignition = rules.elements.get(residue_id).expect("residue").ignition_point as i16;
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");

    // One cell is a coin flip; a row of them is a rate. Each draws on its own
    // coordinates, so a row of 40 is 40 independent trials of the same dwell.
    let burned_after = |ticks: u64| {
        let mut world = scene::arena(80, 80, 1, &rules);
        let floor = floor_of(80);
        let y = floor - 1;
        for x in 20..60 {
            world.field_mut().set(x, y, residue_id);
        }
        for _ in 0..ticks {
            for x in 20..60 {
                world.field_mut().set_temperature(x, y, ignition);
                world.field_mut().set_temperature(x, floor, ignition);
            }
            world.step();
        }
        (20..60).filter(|&x| world.get(x, y) == burnt).count()
    };

    let brief = burned_after(4);
    let long = burned_after(40);
    assert!(
        long > brief,
        "ten times the dwell should burn more, not the same: {brief} then {long}"
    );
    assert!(brief < 40, "a brief pass should not convert the whole batch: {brief}");
}

/// Tops every heater in the row back up to a full body of fuel.
///
/// A heater conducts through whatever is standing in its own footprint, and `burn_fuel`
/// empties that footprint from the bottom up — so the row that touches whatever is
/// above it is the first row to go hollow, and a heater left to burn down stops heating
/// long before it stops having fuel. In play a heater is fed continuously, by belt or
/// by gravity; here that feed is this call. Without it these tests measure a heater
/// going out, which is not what they are about.
fn refuel(world: &mut World, tile_x: i32, tile_y: i32, tiles: i32, fuel: u8) {
    for offset in 0..tiles {
        let (x0, y0, x1, y1) = common::heater(tile_x + offset, tile_y).body();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if world.get(x, y) == sim_core::EMPTY {
                    world.field_mut().set(x, y, fuel);
                }
            }
        }
    }
}

/// Lays a run of `tiles` conveyors, walls both ends so cargo stays on the run, loads
/// the carry row with `cargo`, and puts a fed row of heaters underneath.
fn hot_run(
    world: &mut World,
    make: impl Fn(i32, i32, i8) -> Entity,
    tiles: i32,
    structure: u8,
    cargo: u8,
    fuel: u8,
) -> (i32, i32, i32) {
    let (tile_x, tile_y) = (4, 6);
    for offset in 0..tiles {
        world.place(make(tile_x + offset, tile_y, 1));
    }

    // Heaters directly beneath, so their top row touches the run's bottom row.
    for offset in 0..tiles {
        world.place(common::heater(tile_x + offset, tile_y + 1));
    }
    refuel(world, tile_x, tile_y + 1, tiles, fuel);

    // A floor under the heaters, so the fuel packed into them has something to rest on.
    paint::stroke(
        world.field_mut(),
        (tile_x, tile_y + 2),
        (tile_x + tiles - 1, tile_y + 2),
        structure,
    );

    let carry_y = tile_y * CELLS - 1;
    let first_x = tile_x * CELLS;
    let last_x = (tile_x + tiles) * CELLS - 1;

    // Walls at both ends, and two cells deep. One cell deep is not enough: cargo
    // resting on a ledge with open air beside it slips diagonally off the end, which is
    // ordinary powder behaviour and drained the run dry the first time this was built.
    // Cargo jams against these rather than leaving, which keeps the test about heating
    // rather than about where the material went.
    for y in carry_y..=carry_y + 2 {
        world.field_mut().set(first_x - 1, y, structure);
        world.field_mut().set(last_x + 1, y, structure);
    }
    for x in first_x..=last_x {
        world.field_mut().set(x, carry_y, cargo);
    }
    (carry_y, first_x, last_x)
}

/// The claim the kiln exists for: cargo riding a kiln gets hot, and cargo riding a
/// plain belt does not — same geometry, same heaters, same cargo, one field different
/// in the data.
#[test]
fn a_kiln_heats_what_it_carries_and_a_belt_does_not() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let residue = rules.elements.id_of("residue").expect("residue");
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let fuel = rules.elements.id_of("fuel").expect("fuel");

    let run = |make: fn(i32, i32, i8) -> Entity| {
        let mut world = scene::arena(200, 200, 1, &rules);
        let (carry_y, first_x, last_x) = hot_run(&mut world, make, 3, structure, residue, fuel);
        for _ in 0..250 {
            refuel(&mut world, 4, 7, 3, fuel);
            world.step();
        }

        let hottest = (first_x..=last_x)
            .map(|x| world.field().temperature(x, carry_y))
            .max()
            .expect("the run is not empty");
        let converted = (first_x..=last_x).filter(|&x| world.get(x, carry_y) == burnt).count();
        (hottest, converted)
    };

    let (kiln_heat, kiln_burned) = run(common::kiln);
    let (belt_heat, belt_burned) = run(common::belt);

    assert!(
        kiln_heat > 800,
        "a kiln should carry its heaters' heat into its cargo: {kiln_heat}K"
    );
    assert!(kiln_burned > 0, "and that heat should actually burn some of it");

    assert_eq!(
        belt_heat, AMBIENT_TEMPERATURE,
        "beltStructure conducts 0.0, so a belt's cargo should never warm at all: {belt_heat}K"
    );
    assert_eq!(belt_burned, 0, "and none of it should burn");
}

/// A kiln is a belt. Whatever it does thermally, it must convey exactly the way a belt
/// conveys — `convey_one` was not touched, and this is what says so.
#[test]
fn a_kiln_conveys_exactly_like_a_belt() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");

    let carried = |make: fn(i32, i32, i8) -> Entity| {
        let mut world = scene::arena(200, 200, 1, &rules);
        let entity = make(2, 1, 1);
        world.place(entity);
        let carry_y = entity.top() - 1;
        let start_x = entity.left();
        world.field_mut().set(start_x, carry_y, sand);
        world.step_many(12);
        (start_x..start_x + CELLS)
            .find(|&x| world.get(x, carry_y) == sand)
            .map(|x| x - start_x)
    };

    assert_eq!(
        carried(common::kiln),
        carried(common::belt),
        "a kiln and a belt should move cargo the same distance in the same time"
    );
    assert!(carried(common::kiln).is_some_and(|moved| moved > 0), "and it should move");
}
