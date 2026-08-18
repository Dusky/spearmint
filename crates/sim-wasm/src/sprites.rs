//! Entity sprites, read from `data/entities.json`.
//!
//! Art is data. Editing a machine's look is a data edit, not a Rust edit, which is what
//! makes iterating on it bearable — and it means `npm run sprites` can render the whole
//! set from the same source the game uses.
//!
//! Sprites live here and never in `sim-core`, which has no rendering by hard
//! architectural requirement (spec 8.3).
//!
//! A sprite is chosen by the first rule that matches, so rules are ordered most specific
//! first. Matching on the *state* of the element a machine handles — powder, liquid, gas
//! — is the useful part: a new liquid gets the tank silhouette without anyone drawing
//! anything, while a `element` rule can still single one out.

use sim_core::chunk::TILE_CELLS;
use sim_core::elements::{DataError, State};
use sim_core::entities::EntityKind;
use sim_core::json::{self, Json};

/// A sprite is as wide and tall as a tile (spec 2.3).
pub const SPRITE_SIZE: usize = TILE_CELLS as usize;

/// What each character in a pixel map means. Roles rather than colours, so one sprite
/// serves every element by being tinted at draw time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ink {
    /// Transparent — the world shows through.
    None,
    Shell,
    ShellDark,
    Trim,
    /// The opening, which takes the colour of whatever is being handled.
    Mouth,
}

const fn ink_of(character: u8) -> Ink {
    match character {
        b'#' => Ink::Shell,
        b'=' => Ink::ShellDark,
        b'\'' => Ink::Trim,
        b'o' => Ink::Mouth,
        _ => Ink::None,
    }
}

/// Colour per ink. The casing is neutral so machines read as built rather than as
/// material; only the opening takes the element's colour.
pub fn colour_of(ink: Ink, element_colour: [u8; 3]) -> Option<[u8; 3]> {
    match ink {
        Ink::None => None,
        Ink::Shell => Some([0x6B, 0x69, 0x63]),
        Ink::ShellDark => Some([0x3A, 0x39, 0x36]),
        Ink::Trim => Some([0x8B, 0x89, 0x82]),
        Ink::Mouth => Some(element_colour),
    }
}

/// Whether a machine is working or has backed up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Running,
    Blocked,
}

#[derive(Clone, Debug)]
struct Rule {
    element: Option<String>,
    element_state: Option<State>,
    status: Option<Status>,
    /// Row-major, `SPRITE_SIZE` rows of `SPRITE_SIZE` bytes.
    pixels: Vec<u8>,
}

impl Rule {
    fn matches(&self, element: &str, element_state: Option<State>, status: Status) -> bool {
        self.element.as_deref().is_none_or(|want| want == element)
            && self
                .element_state
                .is_none_or(|want| element_state == Some(want))
            && self.status.is_none_or(|want| want == status)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpriteTable {
    /// Indexed by entity kind.
    sets: Vec<Vec<Rule>>,
}

impl SpriteTable {
    pub fn from_json(source: &str) -> Result<SpriteTable, DataError> {
        let document = json::parse(source).map_err(DataError::Json)?;
        let Some(entries) = document.get("entities").and_then(Json::as_array) else {
            return Ok(SpriteTable::default());
        };

        let mut table = SpriteTable::default();
        for entry in entries {
            let kind = entry
                .get("id")
                .and_then(Json::as_u8)
                .ok_or(DataError::MissingField { field: "id" })?;

            let mut rules = Vec::new();
            if let Some(sprites) = entry.get("sprites").and_then(Json::as_array) {
                for sprite in sprites {
                    rules.push(parse_rule(sprite)?);
                }
            }

            let index = usize::from(kind);
            if index >= table.sets.len() {
                table.sets.resize(index + 1, Vec::new());
            }
            table.sets[index] = rules;
        }
        Ok(table)
    }

    /// The pixel map to draw, or `None` if nothing matches.
    pub fn pick(
        &self,
        kind: EntityKind,
        element: &str,
        element_state: Option<State>,
        status: Status,
    ) -> Option<&[u8]> {
        self.sets
            .get(usize::from(kind))?
            .iter()
            .find(|rule| rule.matches(element, element_state, status))
            .map(|rule| rule.pixels.as_slice())
    }
}

/// The ink at one position of a pixel map.
pub fn ink_at(pixels: &[u8], x: usize, y: usize) -> Ink {
    if x >= SPRITE_SIZE || y >= SPRITE_SIZE {
        return Ink::None;
    }
    ink_of(pixels[y * SPRITE_SIZE + x])
}

fn parse_rule(sprite: &Json<'_>) -> Result<Rule, DataError> {
    let bad = |field: &'static str| DataError::BadField { field };

    let rows = sprite
        .get("pixels")
        .and_then(Json::as_array)
        .ok_or(DataError::MissingField { field: "pixels" })?;
    if rows.len() != SPRITE_SIZE {
        return Err(bad("pixels"));
    }

    let mut pixels = Vec::with_capacity(SPRITE_SIZE * SPRITE_SIZE);
    for row in rows {
        let line = row.as_str().ok_or_else(|| bad("pixels"))?;
        // A short or long row would silently shear the art, so it is an error.
        if line.len() != SPRITE_SIZE {
            return Err(bad("pixels"));
        }
        pixels.extend_from_slice(line.as_bytes());
    }

    // An absent `when` matches everything, which is how a fallback is written.
    let when = sprite.get("when");
    let text = |field: &str| when.and_then(|when| when.get(field)).and_then(Json::as_str);

    let element_state = match text("elementState") {
        Some(name) => Some(parse_state(name).ok_or_else(|| bad("elementState"))?),
        None => None,
    };
    let status = match text("status") {
        Some("running") => Some(Status::Running),
        Some("blocked") => Some(Status::Blocked),
        Some(_) => return Err(bad("status")),
        None => None,
    };

    Ok(Rule {
        element: text("element").map(str::to_owned),
        element_state,
        status,
        pixels,
    })
}

fn parse_state(name: &str) -> Option<State> {
    match name {
        "solid" => Some(State::Solid),
        "powder" => Some(State::Powder),
        "liquid" => Some(State::Liquid),
        "gas" => Some(State::Gas),
        _ => None,
    }
}
