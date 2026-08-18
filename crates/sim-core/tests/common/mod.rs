//! Shared test fixtures.
//!
//! Each test binary compiles this module separately, so anything a given binary does
//! not use looks dead to it — hence the allows.

use sim_core::{ElementTable, Rules};

/// The real element data, compiled in. Tests run against the file the game ships, not
/// a fixture, so a bad edit to it fails the suite.
#[allow(dead_code)]
pub const ELEMENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/elements.json"
));

#[allow(dead_code)]
pub fn rules() -> Rules {
    Rules::from_json(ELEMENTS_JSON).expect("data/elements.json should load")
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
    }
}
