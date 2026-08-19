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

/// Whether a machine is working, has backed up, or has been switched off.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Running,
    Blocked,
    /// Switched off by the player. Distinct from `Blocked` on purpose: blocked is the
    /// factory telling you something is wrong, off is you telling the factory.
    Disabled,
}

#[derive(Clone, Debug)]
struct Rule {
    element: Option<String>,
    element_state: Option<State>,
    status: Option<Status>,
    /// Which row of a multi-tile footprint this is art for, counting from the top.
    /// `None` matches every row, which is what a one-tile machine wants and what every
    /// sprite meant before anything was taller than a tile.
    row: Option<u32>,
    /// One or more pixel maps, each row-major, `SPRITE_SIZE` rows of `SPRITE_SIZE`
    /// bytes. More than one is animation: which frame is drawn is the caller's
    /// business (`SpriteTable::pick` takes a frame index), not this rule's — a rule
    /// only knows what it looks like, never when.
    frames: Vec<Vec<u8>>,
}

impl Rule {
    fn matches(
        &self,
        element: &str,
        element_state: Option<State>,
        status: Status,
        row: u32,
    ) -> bool {
        self.element.as_deref().is_none_or(|want| want == element)
            && self
                .element_state
                .is_none_or(|want| element_state == Some(want))
            && self.status.is_none_or(|want| want == status)
            && self.row.is_none_or(|want| want == row)
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

    /// The pixel map to draw, or `None` if nothing matches. `frame` is a monotonic
    /// counter, not an index — wrapped here so the caller never needs to know how many
    /// frames a rule has.
    #[allow(clippy::too_many_arguments)]
    pub fn pick(
        &self,
        kind: EntityKind,
        element: &str,
        element_state: Option<State>,
        status: Status,
        frame: usize,
        row: u32,
    ) -> Option<&[u8]> {
        self.sets
            .get(usize::from(kind))?
            .iter()
            .find(|rule| rule.matches(element, element_state, status, row))
            .map(|rule| rule.frames[frame % rule.frames.len()].as_slice())
    }
}

/// The ink at one position of a pixel map.
pub fn ink_at(pixels: &[u8], x: usize, y: usize) -> Ink {
    if x >= SPRITE_SIZE || y >= SPRITE_SIZE {
        return Ink::None;
    }
    ink_of(pixels[y * SPRITE_SIZE + x])
}

/// One frame's worth of rows, whichever shape the JSON used.
fn parse_frame(rows: &[Json<'_>]) -> Result<Vec<u8>, DataError> {
    let bad = |field: &'static str| DataError::BadField { field };
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
    Ok(pixels)
}

fn parse_rule(sprite: &Json<'_>) -> Result<Rule, DataError> {
    let bad = |field: &'static str| DataError::BadField { field };

    let outer = sprite
        .get("pixels")
        .and_then(Json::as_array)
        .ok_or(DataError::MissingField { field: "pixels" })?;

    // A single frame is an array of SPRITE_SIZE row-strings. Animation is an array of
    // those — told apart by whether the first element is itself an array.
    let frames = if outer.first().is_some_and(|first| first.as_array().is_some()) {
        outer
            .iter()
            .map(|frame| {
                let rows = frame.as_array().ok_or_else(|| bad("pixels"))?;
                parse_frame(rows)
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![parse_frame(outer)?]
    };

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
        Some("disabled") => Some(Status::Disabled),
        Some(_) => return Err(bad("status")),
        None => None,
    };

    let row = match when.and_then(|when| when.get("row")).map(Json::as_i32) {
        Some(Some(value)) if value >= 0 => Some(value as u32),
        Some(_) => return Err(bad("row")),
        None => None,
    };

    Ok(Rule {
        element: text("element").map(str::to_owned),
        element_state,
        status,
        row,
        frames,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::elements::ElementTable;
    use sim_core::entities::EntityTable;

    const ELEMENTS_JSON: &str = include_str!("../../../data/elements.json");
    const ENTITIES_JSON: &str = include_str!("../../../data/entities.json");

    /// The shipped animation data actually parses, and its two frames are distinct —
    /// the multi-frame `pixels` shape (an array of arrays) is otherwise only exercised
    /// by hand-written fixtures, never by the real file.
    #[test]
    fn the_shipped_belt_animation_has_two_distinct_frames() {
        let elements = ElementTable::from_json(ELEMENTS_JSON).expect("elements");
        let entities = EntityTable::from_json(ENTITIES_JSON, &elements).expect("entities");
        let table = SpriteTable::from_json(ENTITIES_JSON).expect("sprites");
        let belt_kind = entities.id_of("belt").expect("belt kind");

        let frame0 = table.pick(belt_kind, "", None, Status::Running, 0, 0);
        let frame1 = table.pick(belt_kind, "", None, Status::Running, 1, 0);
        assert!(frame0.is_some() && frame1.is_some(), "the running sprite should resolve");
        assert_ne!(frame0, frame1, "the two animation frames should differ");
        // The frame index wraps rather than panicking on an out-of-range counter.
        assert_eq!(table.pick(belt_kind, "", None, Status::Running, 2, 0), frame0);

        assert!(
            table.pick(belt_kind, "", None, Status::Blocked, 0, 0).is_some(),
            "the single-frame blocked sprite should still resolve"
        );
    }

    /// A machine taller than one tile draws different art per row — otherwise it is one
    /// tile stamped twice, which is no good for a piston that needs a gantry and a ram.
    #[test]
    fn the_shipped_compactor_draws_a_different_sprite_per_row() {
        let elements = ElementTable::from_json(ELEMENTS_JSON).expect("elements");
        let entities = EntityTable::from_json(ENTITIES_JSON, &elements).expect("entities");
        let table = SpriteTable::from_json(ENTITIES_JSON).expect("sprites");

        let kind = entities.id_of("compactor").expect("compactor kind");
        let definition = entities.get(kind).expect("compactor");
        assert!(definition.height_tiles > 1, "this test needs a multi-tile machine");

        let gantry = table.pick(kind, "", None, Status::Running, 0, 0);
        let ram = table.pick(kind, "", None, Status::Running, 0, 1);
        assert!(gantry.is_some() && ram.is_some(), "both rows should resolve");
        assert_ne!(gantry, ram, "the two rows should not be the same tile twice");

        // The ram strokes; the gantry it hangs from does not, or the whole machine
        // would read as sliding.
        assert_ne!(
            ram,
            table.pick(kind, "", None, Status::Running, 1, 1),
            "the ram should animate"
        );
        assert_eq!(
            gantry,
            table.pick(kind, "", None, Status::Running, 1, 0),
            "the gantry should hold still"
        );
    }
}
