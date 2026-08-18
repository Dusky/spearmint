//! Machines, as opposed to matter.
//!
//! Entities are a separate layer from the particle grid (spec 4.1): a cell holds sand or
//! water, never a machine. They sit on the tile grid (spec 2.3) rather than on cells, so
//! hit-testing one is a tile comparison and its sprite fills a tile exactly.
//!
//! **There is one entity type here, not one per machine.** A belt and a teleporter are
//! rows in `data/entities.json` and one arm of `tick` each — not a new struct, a new
//! `Vec` on `World`, a new set of accessors, and a new pile of exports. That plumbing
//! grows linearly with machine count if you let it, and this is the point of the design.
//!
//! Everything descriptive lives in data, following the same principle as elements
//! (spec 3.2). Only behaviour needs code.

use crate::chunk::TILE_CELLS;
use crate::elements::{DataError, ElementId, ElementTable, State, EMPTY};
use crate::field::CellField;
use crate::json::{self, Json};
use crate::rng;

/// Which kind of machine this is, declared in the data file and permanent thereafter —
/// ids end up in saves, exactly as element ids do.
pub type EntityKind = u8;

/// What a machine does. The one part of an entity that cannot be data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Behaviour {
    /// Introduces matter into the world. Spawners are the game's only input (spec 3.4).
    Emit,
    /// Takes matter out of the world and pays for it. The one licensed sink: spec 1.1
    /// allows the physics rules themselves to destroy a particle, and a machine that
    /// consumes what falls into it is such a rule.
    Collect,
}

impl Behaviour {
    fn parse(text: &str) -> Option<Behaviour> {
        match text {
            "emit" => Some(Behaviour::Emit),
            "collect" => Some(Behaviour::Collect),
            _ => None,
        }
    }
}

/// One placed machine.
///
/// Deliberately small: everything shared by machines of a kind lives in the type, not in
/// each instance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entity {
    pub kind: EntityKind,
    pub tile_x: i32,
    pub tile_y: i32,
    /// What it works on — the element an emitter emits. `EMPTY` where it means nothing.
    pub element: ElementId,
}

impl Entity {
    pub const fn new(kind: EntityKind, tile_x: i32, tile_y: i32, element: ElementId) -> Entity {
        Entity {
            kind,
            tile_x,
            tile_y,
            element,
        }
    }

    pub const fn left(&self) -> i32 {
        self.tile_x * TILE_CELLS as i32
    }

    pub const fn top(&self) -> i32 {
        self.tile_y * TILE_CELLS as i32
    }

    /// The row an emitter's material appears in: immediately below its footprint, so
    /// the sprite can fill the tile without emitted matter appearing inside it.
    pub fn mouth(&self, definition: &EntityType) -> i32 {
        (self.tile_y + definition.height_tiles as i32) * TILE_CELLS as i32
    }

    /// The cells inside the machine, which is where a collector takes from.
    ///
    /// A machine is not matter (spec 4.1), so nothing rests on top of one: material
    /// falls straight *through* the tile. A collector therefore eats what is inside it
    /// rather than what is stacked above it. Build it into a floor and it is a hopper;
    /// leave the bottom open and product falls past, which is a routing mistake the
    /// player can watch happen.
    pub fn body(&self, definition: &EntityType) -> (i32, i32, i32, i32) {
        let left = self.left();
        let top = self.top();
        (
            left,
            top,
            left + definition.width_tiles as i32 * TILE_CELLS as i32 - 1,
            top + definition.height_tiles as i32 * TILE_CELLS as i32 - 1,
        )
    }

    /// Whether this covers a tile, accounting for a footprint wider than one.
    pub fn covers(&self, definition: &EntityType, tile_x: i32, tile_y: i32) -> bool {
        tile_x >= self.tile_x
            && tile_y >= self.tile_y
            && tile_x < self.tile_x + definition.width_tiles as i32
            && tile_y < self.tile_y + definition.height_tiles as i32
    }

    /// The machine cannot do its job, which for an emitter means every cell it would
    /// emit into is occupied and for a collector means there is nothing to eat.
    ///
    /// Not a failure state — back-pressure is physical here as it is on belts (spec
    /// 4.2), and a machine that has backed up into its own input, or is being fed
    /// nothing at all, is worth showing.
    pub fn is_blocked<F: CellField + ?Sized>(&self, definition: &EntityType, field: &F) -> bool {
        match definition.behaviour {
            Behaviour::Emit => {
                let mouth = self.mouth(definition);
                let span = definition.width_tiles as i32 * TILE_CELLS as i32;
                (0..span).all(|offset| field.get(self.left() + offset, mouth) != Some(EMPTY))
            }
            Behaviour::Collect => {
                let (x0, y0, x1, y1) = self.body(definition);
                (y0..=y1).all(|y| {
                    (x0..=x1).all(|x| field.get(x, y).unwrap_or(EMPTY) == EMPTY)
                })
            }
        }
    }
}

/// Everything about a kind of machine that is not its behaviour.
#[derive(Clone, Debug)]
pub struct EntityType {
    pub id: EntityKind,
    pub name: String,
    pub behaviour: Behaviour,
    /// Which client tool places it, so the tool list can be built from data.
    pub tool: String,
    pub width_tiles: u32,
    pub height_tiles: u32,
    /// Cells emitted per tick, for emitters.
    pub rate: u32,
}

/// Entity types indexed by id — a `Vec`, never a map, so iteration order cannot reach
/// the simulation (spec 3.1).
#[derive(Clone, Debug, Default)]
pub struct EntityTable {
    slots: Vec<Option<EntityType>>,
}

impl EntityTable {
    /// Reads the `entities` array. Its absence is not an error: a world with no machines
    /// is a perfectly good world, and Milestone 1 had one.
    pub fn from_json(source: &str) -> Result<EntityTable, DataError> {
        let document = json::parse(source).map_err(DataError::Json)?;
        let Some(entries) = document.get("entities").and_then(Json::as_array) else {
            return Ok(EntityTable::default());
        };

        let mut table = EntityTable::default();
        for entry in entries {
            let definition = parse_entity(entry)?;
            let index = usize::from(definition.id);
            if index >= table.slots.len() {
                table.slots.resize(index + 1, None);
            }
            if table.slots[index].is_some() {
                return Err(DataError::DuplicateId(definition.id));
            }
            table.slots[index] = Some(definition);
        }
        Ok(table)
    }

    pub fn get(&self, kind: EntityKind) -> Option<&EntityType> {
        self.slots.get(usize::from(kind))?.as_ref()
    }

    pub fn id_of(&self, name: &str) -> Option<EntityKind> {
        self.iter()
            .find(|definition| definition.name == name)
            .map(|definition| definition.id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &EntityType> {
        self.slots.iter().flatten()
    }

    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn parse_entity(entry: &Json<'_>) -> Result<EntityType, DataError> {
    let field = |name: &'static str| {
        entry
            .get(name)
            .ok_or(DataError::MissingField { field: name })
    };
    let bad = |name: &'static str| DataError::BadField { field: name };

    let id = field("id")?.as_u8().ok_or_else(|| bad("id"))?;
    if id == 0 {
        // Zero means "no entity", as it means "no element" for cells.
        return Err(DataError::ReservedId);
    }

    let behaviour_text = field("behaviour")?
        .as_str()
        .ok_or_else(|| bad("behaviour"))?;

    Ok(EntityType {
        id,
        name: field("name")?
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| bad("name"))?
            .to_owned(),
        behaviour: Behaviour::parse(behaviour_text).ok_or_else(|| bad("behaviour"))?,
        tool: field("tool")?
            .as_str()
            .ok_or_else(|| bad("tool"))?
            .to_owned(),
        width_tiles: field("widthTiles")?
            .as_i32()
            .filter(|value| *value > 0)
            .ok_or_else(|| bad("widthTiles"))? as u32,
        height_tiles: field("heightTiles")?
            .as_i32()
            .filter(|value| *value > 0)
            .ok_or_else(|| bad("heightTiles"))? as u32,
        // Optional: only emitters use it.
        rate: entry.get("rate").and_then(Json::as_i32).unwrap_or(1).max(0) as u32,
    })
}

/// Runs every machine, in placement order, and returns what the collectors earned this
/// tick.
///
/// This is the one place behaviour dispatches. Adding a belt means adding an arm here
/// and a row in the data file, and touching nothing else.
pub fn tick<F: CellField + ?Sized>(
    field: &mut F,
    entities: &[Entity],
    types: &EntityTable,
    elements: &ElementTable,
    seed: u64,
    tick: u64,
) -> u64 {
    let mut earned = 0;
    for (index, entity) in entities.iter().enumerate() {
        let Some(definition) = types.get(entity.kind) else {
            continue;
        };
        match definition.behaviour {
            Behaviour::Emit => emit(field, entity, definition, elements, seed, tick, index),
            Behaviour::Collect => earned += collect(field, entity, definition, elements),
        }
    }
    earned
}

/// Introduces matter. A machine cannot force material into an occupied cell, so a
/// clogged factory starves itself and nothing special-cases that.
#[allow(clippy::too_many_arguments)]
fn emit<F: CellField + ?Sized>(
    field: &mut F,
    entity: &Entity,
    definition: &EntityType,
    elements: &ElementTable,
    seed: u64,
    tick: u64,
    index: usize,
) {
    let Some(element) = elements.get(entity.element) else {
        return;
    };
    // Emitters emit matter that moves. A solid would appear one cell at a time and sit
    // there, which is not emission — it is drawing, and there is a tool for that.
    if entity.element == EMPTY || element.state == State::Solid {
        return;
    }
    let left = entity.left();
    let mouth = entity.mouth(definition);
    let span = definition.width_tiles * TILE_CELLS;

    for grain in 0..definition.rate {
        // Salted by machine and by grain, so two emitters do not fire in lockstep and
        // one machine's grains do not stack up in a single column.
        let salt = (index as u32)
            .wrapping_mul(31)
            .wrapping_add(grain)
            .wrapping_add(101);
        let offset = rng::below(seed, tick, left, mouth, salt, span);
        let x = left + offset as i32;
        if field.get(x, mouth) == Some(EMPTY) {
            field.set(x, mouth, entity.element);
            field.mark_active(x, mouth);
        }
    }
}

/// Takes matter out of the world, and returns what it was worth.
///
/// A collector eats whatever falls into its mouth and pays each element's declared
/// value, which is zero for everything that is not product. That is the whole reason
/// routing matters: dumping unwashed sand into a collector destroys it for nothing.
///
/// Left to right, up to `rate` cells a tick — no randomness, because there is nothing
/// here for it to decide.
fn collect<F: CellField + ?Sized>(
    field: &mut F,
    entity: &Entity,
    definition: &EntityType,
    elements: &ElementTable,
) -> u64 {
    let (x0, y0, x1, y1) = entity.body(definition);

    let mut earned = 0;
    let mut taken = 0;
    // Bottom row first: material that has fallen furthest into the machine is the
    // material about to fall out of it.
    for y in (y0..=y1).rev() {
        for x in x0..=x1 {
            if taken == definition.rate {
                return earned;
            }
            let Some(id) = field.get(x, y) else {
                continue;
            };
            let Some(element) = elements.get(id) else {
                continue;
            };
            // Solids are structure, not throughput. A collector that ate walls would be
            // a demolition tool, and erase already is one.
            if id == EMPTY || element.state == State::Solid {
                continue;
            }

            field.set(x, y, EMPTY);
            // Removing a cell is not a move, so nothing else reports it — and a pile
            // that stops being told it is settling stops feeding the collector.
            field.mark_active(x, y);
            earned += u64::from(element.value);
            taken += 1;
        }
    }
    earned
}
