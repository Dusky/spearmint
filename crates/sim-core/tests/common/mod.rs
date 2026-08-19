//! Shared test fixtures.
//!
//! Each test binary compiles this module separately, so anything a given binary does
//! not use looks dead to it — hence the allows.

use sim_core::{ElementTable, Entity, EntityKind, Rules};

/// The real element data, compiled in. Tests run against the file the game ships, not
/// a fixture, so a bad edit to it fails the suite.
#[allow(dead_code)]
pub const ELEMENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/elements.json"
));

/// Machines, as opposed to matter.
#[allow(dead_code)]
pub const ENTITIES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/entities.json"
));

pub fn rules() -> Rules {
    Rules::load(ELEMENTS_JSON, ENTITIES_JSON).expect("the data files should load")
}

/// The emitter's kind id, read from the data rather than assumed — ids belong to the
/// data file (spec 3.2).
#[allow(dead_code)]
pub fn emitter_kind() -> EntityKind {
    rules()
        .entities
        .id_of("emitter")
        .expect("an `emitter` entity should be defined")
}

/// The press's kind id, read from the data for the same reason.
#[allow(dead_code)]
pub fn press_kind() -> EntityKind {
    rules()
        .entities
        .id_of("press")
        .expect("a `press` entity should be defined")
}

/// A press on a tile. It works on whatever it can press, so it carries no element.
#[allow(dead_code)]
pub fn press(tile_x: i32, tile_y: i32) -> Entity {
    sized(Entity::new(press_kind(), tile_x, tile_y, sim_core::EMPTY))
}

/// The vault's kind id.
#[allow(dead_code)]
pub fn vault_kind() -> EntityKind {
    rules()
        .entities
        .id_of("vault")
        .expect("a `vault` entity should be defined")
}

/// A vault over a marked-out region of tiles.
#[allow(dead_code)]
pub fn vault(tile_x: i32, tile_y: i32, width: u32, height: u32) -> Entity {
    Entity::sized(vault_kind(), tile_x, tile_y, width, height)
}

/// The burner's kind id.
#[allow(dead_code)]
pub fn burner_kind() -> EntityKind {
    rules()
        .entities
        .id_of("burner")
        .expect("a `burner` entity should be defined")
}

/// A burner on a tile. Its input is fixed by the data (residue), not chosen here.
#[allow(dead_code)]
pub fn burner(tile_x: i32, tile_y: i32) -> Entity {
    sized(Entity::new(burner_kind(), tile_x, tile_y, sim_core::EMPTY))
}

/// The compactor's kind id.
#[allow(dead_code)]
pub fn compactor_kind() -> EntityKind {
    rules()
        .entities
        .id_of("compactor")
        .expect("a `compactor` entity should be defined")
}

/// A compactor on a tile. Its input is fixed by the data (burntResidue), not chosen here.
#[allow(dead_code)]
pub fn compactor(tile_x: i32, tile_y: i32) -> Entity {
    sized(Entity::new(
        compactor_kind(),
        tile_x,
        tile_y,
        sim_core::EMPTY,
    ))
}

/// The belt's kind id.
#[allow(dead_code)]
pub fn belt_kind() -> EntityKind {
    rules()
        .entities
        .id_of("belt")
        .expect("a `belt` entity should be defined")
}

/// A belt on a tile, conveying in `direction` (`1` or `-1`).
#[allow(dead_code)]
pub fn belt(tile_x: i32, tile_y: i32, direction: i8) -> Entity {
    sized(Entity::belt(belt_kind(), tile_x, tile_y, direction))
}

/// The filter's kind id.
#[allow(dead_code)]
pub fn filter_kind() -> EntityKind {
    rules()
        .entities
        .id_of("filter")
        .expect("a `filter` entity should be defined")
}

/// A filter on a tile, conveying in `direction` and letting `element` drop through.
#[allow(dead_code)]
pub fn filter(tile_x: i32, tile_y: i32, direction: i8, element: u8) -> Entity {
    sized(Entity::filter(
        filter_kind(),
        tile_x,
        tile_y,
        direction,
        element,
    ))
}

fn sized(mut entity: Entity) -> Entity {
    let rules = rules();
    if let Some(definition) = rules.entities.get(entity.kind) {
        entity.size_from(definition);
    }
    entity
}

/// An emitter on a tile, emitting one element.
///
/// Sized from its type here, which `World::place` would otherwise do — so a test can
/// hit-test one without placing it first.
#[allow(dead_code)]
pub fn emitter(tile_x: i32, tile_y: i32, element: u8) -> Entity {
    sized(Entity::new(emitter_kind(), tile_x, tile_y, element))
}

#[allow(dead_code)]
pub fn table() -> ElementTable {
    rules().elements
}

/// Rules with the reactions stripped out.
///
/// Several invariants — per-element conservation most of all — only hold when nothing
/// is transmuting. Those tests are about movement, so they run without chemistry.
#[allow(dead_code)]
pub fn rules_without_reactions() -> Rules {
    Rules {
        elements: table(),
        reactions: Default::default(),
        entities: rules().entities,
    }
}
