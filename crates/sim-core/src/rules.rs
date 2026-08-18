//! Everything the data file defines, loaded together.
//!
//! Elements and reactions are always read from the same source and always travel
//! together, so they get one type rather than two parameters threaded through
//! everything (spec 3.2).

use crate::elements::{DataError, ElementTable};
use crate::reactions::ReactionTable;

#[derive(Clone, Debug, Default)]
pub struct Rules {
    pub elements: ElementTable,
    pub reactions: ReactionTable,
}

impl Rules {
    pub fn from_json(source: &str) -> Result<Rules, DataError> {
        let elements = ElementTable::from_json(source)?;
        let reactions = ReactionTable::from_json(source, &elements)?;
        Ok(Rules {
            elements,
            reactions,
        })
    }
}
