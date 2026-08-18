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

/// The collector's kind id, read from the data for the same reason.
#[allow(dead_code)]
pub fn collector_kind() -> EntityKind {
    rules()
        .entities
        .id_of("collector")
        .expect("a `collector` entity should be defined")
}

/// A collector on a tile. It eats whatever falls in, so it carries no element.
#[allow(dead_code)]
pub fn collector(tile_x: i32, tile_y: i32) -> Entity {
    Entity::new(collector_kind(), tile_x, tile_y, sim_core::EMPTY)
}

/// An emitter on a tile, emitting one element.
#[allow(dead_code)]
pub fn emitter(tile_x: i32, tile_y: i32, element: u8) -> Entity {
    Entity::new(emitter_kind(), tile_x, tile_y, element)
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
