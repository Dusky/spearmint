//! The element table, loaded from data.
//!
//! Spec 3.2: element definitions live in data files, not code. Adding an element means
//! editing JSON, not writing a match arm — which is only true if the simulation rules
//! dispatch on an element's *state* rather than its identity. Nothing in this crate
//! matches on "sand".

use crate::fixed::Fixed;
use crate::json::{self, Json, JsonError};

/// One byte per cell. Id 0 is reserved for empty space and never appears in the table.
pub type ElementId = u8;

/// Empty space. Not an element: it has no definition and no behaviour.
pub const EMPTY: ElementId = 0;

/// The largest id the table can hold, bounded by the one-byte cell.
pub const MAX_ELEMENT_ID: ElementId = u8::MAX;

/// What an element does, which is all the simulation rules are allowed to know about it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Never moves.
    Solid,
    /// Falls, and heaps — it will slide diagonally but not flow sideways.
    Powder,
    /// Falls, and flows sideways to find its level.
    Liquid,
    /// Rises. No gas element exists yet; the state is here because the schema has it.
    Gas,
}

impl State {
    fn parse(text: &str) -> Option<State> {
        match text {
            "solid" => Some(State::Solid),
            "powder" => Some(State::Powder),
            "liquid" => Some(State::Liquid),
            "gas" => Some(State::Gas),
            _ => None,
        }
    }
}

/// The full schema from spec 3.2. Several fields are unused by Milestone 1's rules and
/// are loaded anyway: the data file is the contract, and a field that is parsed and
/// validated now cannot be quietly wrong later.
#[derive(Clone, Debug)]
pub struct Element {
    pub id: ElementId,
    pub name: String,
    pub state: State,
    /// Relative, unitless. Decides what displaces what.
    pub density: i32,
    /// Kelvin.
    pub melting_point: i32,
    /// Kelvin.
    pub boiling_point: i32,
    pub thermal_conductivity: Fixed,
    /// Straight RGB, matching the client's palette.
    pub color: [u8; 3],
    /// Per-channel spread applied when rendering, so a field of one element is not flat.
    pub color_variance: u8,
    pub flammability: Fixed,
    pub hardness: Fixed,
    /// What a press pays per cell of it, in points. Zero for everything that is not a
    /// product — and since a press only takes what it can press, zero also means the
    /// machine ignores it entirely.
    pub value: u32,
    /// Whether this *is* money. A press turns points into currency and never takes it
    /// back, and currency inside a vault is the player's balance (spec 5.1).
    pub currency: bool,
    /// What a press leaves behind when it takes a cell of this but does not complete a
    /// nugget. `EMPTY` means it is simply destroyed, which is the default for anything
    /// that has not declared a byproduct.
    ///
    /// This is what makes pressing conversion rather than destruction (spec 5.3): eight
    /// grains in, eight cells out — one nugget and seven residue — and only spending
    /// ever removes matter from the world outright.
    pub residue: ElementId,
    /// What a machine built to refine this turns it into — residue burns into burnt
    /// residue, which compacts into fuel (spec 5.3). `EMPTY` means nothing refines it.
    ///
    /// Generic on purpose: a burner and a compactor are the same behaviour reading this
    /// field on whatever they are fed, not two hand-written machines.
    pub refined_into: ElementId,
}

/// Elements indexed by id. A `Vec` rather than a map: spec 3.1 forbids hash-map
/// iteration in the sim, and direct indexing is what the tick loop wants anyway.
#[derive(Clone, Debug, Default)]
pub struct ElementTable {
    slots: Vec<Option<Element>>,
}

impl ElementTable {
    pub fn from_json(source: &str) -> Result<ElementTable, DataError> {
        let document = json::parse(source).map_err(DataError::Json)?;
        let elements = document
            .get("elements")
            .and_then(Json::as_array)
            .ok_or(DataError::MissingField { field: "elements" })?;

        let mut table = ElementTable::default();
        for entry in elements {
            let element = parse_element(entry)?;
            let index = usize::from(element.id);
            if index >= table.slots.len() {
                table.slots.resize(index + 1, None);
            }
            if table.slots[index].is_some() {
                return Err(DataError::DuplicateId(element.id));
            }
            table.slots[index] = Some(element);
        }

        if table.slots.iter().all(Option::is_none) {
            return Err(DataError::Empty);
        }

        // A second pass to resolve name-referencing fields, the same shape reactions.rs
        // uses for reactants and products: every id has to exist before any name can be
        // looked up, so this cannot be done inline with the loop above.
        for entry in elements {
            let id = field_id(entry)?;

            if let Some(name) = entry.get("residue").and_then(Json::as_str) {
                let residue_id = table
                    .id_of(name)
                    .ok_or(DataError::BadField { field: "residue" })?;
                table.slots[usize::from(id)]
                    .as_mut()
                    .expect("just inserted above")
                    .residue = residue_id;
            }
            if let Some(name) = entry.get("refinedInto").and_then(Json::as_str) {
                let refined_id = table
                    .id_of(name)
                    .ok_or(DataError::BadField { field: "refinedInto" })?;
                table.slots[usize::from(id)]
                    .as_mut()
                    .expect("just inserted above")
                    .refined_into = refined_id;
            }
        }

        Ok(table)
    }

    pub fn get(&self, id: ElementId) -> Option<&Element> {
        self.slots.get(usize::from(id))?.as_ref()
    }

    /// Looks an element up by name. For tests and tooling — the tick loop uses ids.
    pub fn id_of(&self, name: &str) -> Option<ElementId> {
        self.iter()
            .find(|element| element.name == name)
            .map(|element| element.id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.slots.iter().flatten()
    }

    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The id an entry declares, for the second pass that resolves names against ids.
fn field_id(entry: &Json<'_>) -> Result<ElementId, DataError> {
    entry
        .get("id")
        .and_then(Json::as_u8)
        .ok_or(DataError::MissingField { field: "id" })
}

fn parse_element(entry: &Json<'_>) -> Result<Element, DataError> {
    let field = |name: &'static str| {
        entry
            .get(name)
            .ok_or(DataError::MissingField { field: name })
    };
    let bad = |name: &'static str| DataError::BadField { field: name };

    let id = field("id")?.as_u8().ok_or_else(|| bad("id"))?;
    if id == EMPTY {
        return Err(DataError::ReservedId);
    }

    let name = field("name")?.as_str().ok_or_else(|| bad("name"))?;
    if name.is_empty() {
        return Err(bad("name"));
    }

    let state_text = field("state")?.as_str().ok_or_else(|| bad("state"))?;
    let state = State::parse(state_text).ok_or_else(|| bad("state"))?;

    Ok(Element {
        id,
        name: name.to_owned(),
        state,
        density: field("density")?.as_i32().ok_or_else(|| bad("density"))?,
        melting_point: field("melting_point")?
            .as_i32()
            .ok_or_else(|| bad("melting_point"))?,
        boiling_point: field("boiling_point")?
            .as_i32()
            .ok_or_else(|| bad("boiling_point"))?,
        thermal_conductivity: field("thermal_conductivity")?
            .as_fixed()
            .ok_or_else(|| bad("thermal_conductivity"))?,
        color: parse_color(field("color")?.as_str().ok_or_else(|| bad("color"))?)?,
        color_variance: field("color_variance")?
            .as_u8()
            .ok_or_else(|| bad("color_variance"))?,
        flammability: field("flammability")?
            .as_fixed()
            .ok_or_else(|| bad("flammability"))?,
        hardness: field("hardness")?
            .as_fixed()
            .ok_or_else(|| bad("hardness"))?,
        // Optional: most elements are worth nothing, and saying so in every entry
        // would be noise.
        value: entry
            .get("value")
            .map(|value| value.as_i32().filter(|value| *value >= 0).ok_or(bad("value")))
            .transpose()?
            .unwrap_or(0) as u32,
        currency: entry
            .get("currency")
            .map(|flag| flag.as_bool().ok_or(bad("currency")))
            .transpose()?
            .unwrap_or(false),
        // Resolved against the other elements' names in a second pass, once every id
        // exists — see `ElementTable::from_json`. Left as EMPTY here regardless of what
        // the entry says, so this function never depends on parse order (spec 3.2).
        residue: EMPTY,
        refined_into: EMPTY,
    })
}

/// `"#RRGGBB"`, the same form the client's design tokens use.
fn parse_color(text: &str) -> Result<[u8; 3], DataError> {
    let digits = text.strip_prefix('#').ok_or(DataError::BadColor)?;
    if digits.len() != 6 {
        return Err(DataError::BadColor);
    }
    let bytes = digits.as_bytes();
    let mut channels = [0u8; 3];
    for (index, channel) in channels.iter_mut().enumerate() {
        let high = hex_digit(bytes[index * 2]).ok_or(DataError::BadColor)?;
        let low = hex_digit(bytes[index * 2 + 1]).ok_or(DataError::BadColor)?;
        *channel = (high << 4) | low;
    }
    Ok(channels)
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataError {
    Json(JsonError),
    MissingField {
        field: &'static str,
    },
    BadField {
        field: &'static str,
    },
    /// Id 0 means empty space and cannot be given to an element.
    ReservedId,
    DuplicateId(ElementId),
    /// A reaction named an element that is not defined.
    UnknownElement {
        field: &'static str,
    },
    BadColor,
    /// The file parsed but defined no elements.
    Empty,
}

impl core::fmt::Display for DataError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DataError::Json(error) => {
                write!(
                    formatter,
                    "invalid JSON at byte {}: {:?}",
                    error.offset, error.kind
                )
            }
            DataError::MissingField { field } => write!(formatter, "missing field `{field}`"),
            DataError::BadField { field } => write!(formatter, "invalid field `{field}`"),
            DataError::ReservedId => formatter.write_str("id 0 is reserved for empty space"),
            DataError::DuplicateId(id) => write!(formatter, "duplicate element id {id}"),
            DataError::UnknownElement { field } => {
                write!(formatter, "field `{field}` names an undefined element")
            }
            DataError::BadColor => formatter.write_str("color must be \"#RRGGBB\""),
            DataError::Empty => formatter.write_str("no elements defined"),
        }
    }
}

impl std::error::Error for DataError {}
