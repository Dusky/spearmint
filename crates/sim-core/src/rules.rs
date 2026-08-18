//! Everything the data files define, loaded together.
//!
//! Elements, reactions and entity types always travel together, so they get one type
//! rather than three parameters threaded through everything (spec 3.2).

use crate::elements::{DataError, ElementTable};
use crate::entities::EntityTable;
use crate::reactions::ReactionTable;

#[derive(Clone, Debug, Default)]
pub struct Rules {
    pub elements: ElementTable,
    pub reactions: ReactionTable,
    pub entities: EntityTable,
}

impl Rules {
    /// Loads matter from `elements.json` and machines from `entities.json`.
    pub fn load(elements_json: &str, entities_json: &str) -> Result<Rules, DataError> {
        let elements = ElementTable::from_json(elements_json)?;
        let reactions = ReactionTable::from_json(elements_json, &elements)?;
        let entities = EntityTable::from_json(entities_json)?;
        Ok(Rules {
            elements,
            reactions,
            entities,
        })
    }

    /// Matter only, for tests and tools that do not care about machines.
    pub fn from_json(elements_json: &str) -> Result<Rules, DataError> {
        Rules::load(elements_json, "{}")
    }
}
