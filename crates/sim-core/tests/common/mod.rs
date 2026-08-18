//! Shared test fixtures.

use sim_core::ElementTable;

/// The real element data, compiled in. Tests run against the file the game ships, not
/// a fixture, so a bad edit to it fails the suite.
pub const ELEMENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/elements.json"
));

pub fn table() -> ElementTable {
    ElementTable::from_json(ELEMENTS_JSON).expect("data/elements.json should load")
}
