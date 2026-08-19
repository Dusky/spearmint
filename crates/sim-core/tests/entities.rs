//! Machine placement, hit-testing and removal.
//!
//! Emitter count is the game's only hard limit on production (spec 3.4), which makes
//! being able to take one back matter more than it looks: without removal, spending
//! every slot on one material soft-locks the run.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::scene;

#[test]
fn entity_types_load_from_data() {
    let rules = common::rules();
    assert_eq!(
        rules.entities.len(),
        8,
        "the emitter, the press, the vault, the burner, the compactor, the belt, the \
         filter, and the heater"
    );

    let vault = rules
        .entities
        .get(common::vault_kind())
        .expect("the vault should be defined");
    assert_eq!(vault.behaviour, sim_core::Behaviour::Store);

    let press = rules
        .entities
        .get(common::press_kind())
        .expect("the press should be defined");
    assert_eq!(press.behaviour, sim_core::Behaviour::Press);
    assert!(press.rate > 0, "a press that eats nothing is useless");

    let emitter = rules
        .entities
        .get(common::emitter_kind())
        .expect("the emitter should be defined");
    assert_eq!(emitter.name, "emitter");
    assert_eq!(emitter.behaviour, sim_core::Behaviour::Emit);
    assert_eq!(emitter.width_tiles, 1);
    assert!(emitter.rate > 0, "an emitter that emits nothing is useless");
}

#[test]
fn an_entity_occupies_its_footprint() {
    let entity = common::emitter(3, 4, 2);

    assert_eq!(entity.left(), 3 * TILE_CELLS as i32);
    assert_eq!(entity.top(), 4 * TILE_CELLS as i32);
    // It emits from the row below its own tile, so its sprite can fill the tile without
    // the emitted matter appearing inside it.
    assert_eq!(entity.mouth(), 5 * TILE_CELLS as i32);

    assert!(entity.covers(3, 4));
    assert!(!entity.covers(3, 5));
    assert!(!entity.covers(4, 4));
}

#[test]
fn machines_are_found_by_position_including_negative_tiles() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(200, 200, 1, &rules);

    world.place(common::emitter(2, 3, sand));
    world.place(common::emitter(-4, -2, sand));

    assert_eq!(world.entity_at(2, 3), Some(0));
    assert_eq!(world.entity_at(-4, -2), Some(1));
    assert_eq!(world.entity_at(0, 0), None);
}

/// The bug this milestone started from: five emitters spent on sand, no way to place
/// water, no way to undo, and the run unwinnable.
#[test]
fn removal_frees_the_slot_completely() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let water = rules.elements.id_of("water").expect("water");
    let mut world = scene::arena(200, 200, 1, &rules);

    for tile in 0..5 {
        world.place(common::emitter(tile, 1, sand));
    }
    assert_eq!(world.count_of_kind(common::emitter_kind()), 5);

    let index = world.entity_at(2, 1).expect("a machine on that tile");
    assert!(world.remove(index));
    assert_eq!(world.count_of_kind(common::emitter_kind()), 4);

    world.place(common::emitter(2, 1, water));
    assert_eq!(world.count_of_kind(common::emitter_kind()), 5);
    let replaced = world.entity_at(2, 1).expect("the new machine");
    assert_eq!(world.entities()[replaced].element, water);
}

#[test]
fn removing_something_that_is_not_there_is_harmless() {
    let rules = common::rules();
    let mut world = scene::arena(60, 60, 1, &rules);
    assert!(!world.remove(0));
    assert!(!world.remove(99));
}

#[test]
fn a_removed_emitter_stops_emitting() {
    let rules = common::rules();
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(90, 90, 1, &rules);
    world.place(common::emitter(2, 1, sand));

    world.step_many(200);
    let while_running = world.count_of(sand);
    assert!(while_running > 0, "the emitter never emitted");

    assert!(world.remove(0));
    world.step_many(400);

    // What it already made stays — removal takes the machine, not the matter.
    assert_eq!(
        world.count_of(sand),
        while_running,
        "sand kept appearing after the emitter was removed"
    );
}

#[test]
fn an_emitter_reports_being_blocked() {
    let rules = common::rules();
    let definition = rules.entities.get(common::emitter_kind()).expect("emitter");
    let sand = rules.elements.id_of("sand").expect("sand");
    let structure = rules.elements.id_of("structure").expect("structure");
    let mut world = scene::arena(90, 90, 1, &rules);

    let entity = common::emitter(2, 1, sand);
    world.place(entity);
    assert!(
        !entity.is_blocked(definition, world.field(), &rules.elements),
        "nothing is in the way yet"
    );

    // Wall off every cell it would emit into.
    for offset in 0..TILE_CELLS as i32 {
        world
            .field_mut()
            .set(entity.left() + offset, entity.mouth(), structure);
    }
    assert!(entity.is_blocked(definition, world.field(), &rules.elements));

    // Blocked means it produces nothing, rather than forcing matter into a wall.
    let before = world.count_of(sand);
    world.step_many(200);
    assert_eq!(world.count_of(sand), before);
}

/// An emitter emits from where its sprite actually draws an opening, not the full tile
/// (spec 8.3): sim-core cannot read the art, so `mouthOffset`/`mouthWidth` are the data
/// that keeps physics and pixels agreeing, authored by hand rather than derived.
#[test]
fn an_emitter_only_emits_and_blocks_within_its_mouth() {
    let rules = common::rules();
    let definition = rules.entities.get(common::emitter_kind()).expect("emitter");
    let sand = rules.elements.id_of("sand").expect("sand");
    let structure = rules.elements.id_of("structure").expect("structure");
    assert!(
        definition.mouth_width < TILE_CELLS,
        "this test needs the emitter's mouth to be narrower than the tile, or it \
         cannot tell mouth-aware behaviour apart from the old full-width behaviour"
    );

    let mut world = scene::arena(90, 90, 1, &rules);
    let entity = common::emitter(2, 1, sand);
    world.place(entity);

    // Wall off everything outside the mouth, leaving the mouth itself clear.
    let mouth_left = entity.left() + definition.mouth_offset as i32;
    let mouth_right = mouth_left + definition.mouth_width as i32;
    for x in entity.left()..entity.left() + TILE_CELLS as i32 {
        if x < mouth_left || x >= mouth_right {
            world.field_mut().set(x, entity.mouth(), structure);
        }
    }
    assert!(
        !entity.is_blocked(definition, world.field(), &rules.elements),
        "the mouth itself is still clear"
    );

    let before = world.count_of(sand);
    world.step_many(50);
    assert!(world.count_of(sand) > before, "it should still be emitting through the mouth");

    // Sand that lands must have landed within the mouth's columns, never beside it.
    for x in entity.left()..entity.left() + TILE_CELLS as i32 {
        if world.get(x, entity.mouth()) == sand {
            assert!(
                x >= mouth_left && x < mouth_right,
                "sand appeared at x={x}, outside the mouth [{mouth_left}, {mouth_right})"
            );
        }
    }
}

/// A `structure` emitter is nonsense: a solid would appear one cell at a time and sit there,
/// which is drawing, not emission. Refused in the simulation rather than only in the UI,
/// so nothing else can reintroduce it.
#[test]
fn solids_cannot_be_emitted() {
    let rules = common::rules();
    let structure = rules.elements.id_of("structure").expect("structure");
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(90, 90, 1, &rules);

    world.place(common::emitter(2, 1, structure));
    world.step_many(300);
    let structures_before = world.count_of(structure);

    // The arena's own walls are all that should exist; the emitter added none.
    world.step_many(300);
    assert_eq!(world.count_of(structure), structures_before, "a solid was emitted");

    // And a sand emitter beside it still works, so this is not just a dead world.
    world.place(common::emitter(5, 1, sand));
    world.step_many(200);
    assert!(world.count_of(sand) > 0);
}
