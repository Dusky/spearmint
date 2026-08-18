//! Reactions between adjacent elements.
//!
//! Data-driven, like elements (spec 3.2): a reaction names its reactants and products
//! by element name, and adding one means editing JSON. Nothing here matches on "sand".
//!
//! Spec 3.3 is the reason this exists at all — reactions have no fixed conversion
//! ratio. Yield is whatever the layout produces, because a reaction can only happen
//! where two reactants are actually touching. Contact area, residence time and mixing
//! are not modelled parameters; they are consequences of the machine the player drew.

use crate::elements::{DataError, ElementId, ElementTable};
use crate::fixed::Fixed;
use crate::json::{self, Json};

#[derive(Clone, Debug)]
pub struct Reaction {
    /// The two elements that must be adjacent.
    pub reactants: [ElementId; 2],
    /// What each becomes. Two products for two reactants, so a reaction changes
    /// composition without changing how many cells hold matter.
    pub products: [ElementId; 2],
    /// Chance per adjacent pair per tick, 0..1.
    pub probability: Fixed,
}

#[derive(Clone, Debug, Default)]
pub struct ReactionTable {
    reactions: Vec<Reaction>,
}

impl ReactionTable {
    /// Reads the `reactions` array. Its absence is not an error — a world with no
    /// reactions is a perfectly good world, and Milestone 1 had one.
    pub fn from_json(source: &str, elements: &ElementTable) -> Result<ReactionTable, DataError> {
        let document = json::parse(source).map_err(DataError::Json)?;
        let Some(entries) = document.get("reactions").and_then(Json::as_array) else {
            return Ok(ReactionTable::default());
        };

        let mut reactions = Vec::new();
        for entry in entries {
            reactions.push(parse_reaction(entry, elements)?);
        }
        Ok(ReactionTable { reactions })
    }

    pub fn is_empty(&self) -> bool {
        self.reactions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.reactions.len()
    }

    /// The reaction between two elements, if there is one, along with which side of it
    /// `first` sits on.
    pub fn between(&self, first: ElementId, second: ElementId) -> Option<(&Reaction, bool)> {
        self.reactions.iter().find_map(|reaction| {
            if reaction.reactants[0] == first && reaction.reactants[1] == second {
                Some((reaction, false))
            } else if reaction.reactants[1] == first && reaction.reactants[0] == second {
                Some((reaction, true))
            } else {
                None
            }
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Reaction> {
        self.reactions.iter()
    }
}

fn parse_reaction(entry: &Json<'_>, elements: &ElementTable) -> Result<Reaction, DataError> {
    let names = |field: &'static str| -> Result<[ElementId; 2], DataError> {
        let list = entry
            .get(field)
            .and_then(Json::as_array)
            .ok_or(DataError::MissingField { field })?;
        if list.len() != 2 {
            // Two-in, two-out keeps cell count invariant. Anything else needs a
            // decision about what happens to the difference.
            return Err(DataError::BadField { field });
        }
        let mut ids = [0; 2];
        for (index, item) in list.iter().enumerate() {
            let name = item.as_str().ok_or(DataError::BadField { field })?;
            ids[index] = elements
                .id_of(name)
                .ok_or(DataError::UnknownElement { field })?;
        }
        Ok(ids)
    };

    Ok(Reaction {
        reactants: names("reactants")?,
        products: names("products")?,
        probability: entry.get("probability").and_then(Json::as_fixed).ok_or(
            DataError::MissingField {
                field: "probability",
            },
        )?,
    })
}
