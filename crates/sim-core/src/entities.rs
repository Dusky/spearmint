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
    /// Holds currency. A vault does nothing each tick — it is a region the player has
    /// declared, and what makes it work is that gold inside one counts as money
    /// (spec 5.1). Physics keeps the gold in; the walls are the player's problem.
    Store,
    /// Presses product into currency. The one licensed sink: spec 1.1 allows the physics
    /// rules themselves to destroy a particle, and a machine that consumes what it is
    /// fed is such a rule.
    Press,
    /// Turns one specific element into whatever it is declared to refine into, reading
    /// `EntityType::input` and that element's `refined_into` (spec 5.3). Unlike a press,
    /// nothing is banked: one cell in is one cell out, because there is no threshold to
    /// accumulate toward.
    ///
    /// The compactor is the only one. Residue used to have a burner here and now burns
    /// wherever it is hot enough instead (`Element::burns_into`), which is the same
    /// conversion moved out of a machine and into the physics — so this behaviour is
    /// still generic, but only one stage of the chain runs on it.
    Refine,
    /// Conveys whatever rests on top of it, and — when the instance names an element
    /// (`Entity.element`, the same field an emitter uses) — drops that one element
    /// straight through instead. A filter is not a second behaviour; it is this, tuned
    /// per instance (spec 4.3).
    ///
    /// The entity's own footprint is filled with a real, indestructible structural
    /// element at placement (`World::place`), so gravity already holds cargo up — this
    /// behaviour only ever touches the row directly above its own body.
    Belt,
    /// Burns fuel to hold its own footprint at `EntityType.heat_output`, so the
    /// generic conduction pass (`heat.rs`) can carry that into whatever touches it
    /// (spec 5.3). No banking: while it has fuel it forces its footprint hot, and once
    /// it runs out conduction alone cools it back toward ambient — no special-cased
    /// cooldown anywhere.
    Heater,
}

/// Where a machine looks for the material it works on.
///
/// A data-declared field rather than a second `Behaviour`, for the same reason a filter
/// turned out to be a belt with one more field: a machine that takes what falls into it
/// and a piston that crushes what is heaped under it do the identical thing to whatever
/// they find, and differ only in where they look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reach {
    /// Inside its own footprint — material that has fallen in. Gravity is the conveyor,
    /// and a machine is not matter (spec 4.1), so material falls straight through.
    Body,
    /// The tile directly beneath it: a head that comes down on whatever is under the
    /// machine. Fed by piling material *under* it rather than dropping material *into*
    /// it, which is a different thing to build around.
    Below,
}

impl Reach {
    fn parse(text: &str) -> Option<Reach> {
        match text {
            "body" => Some(Reach::Body),
            "below" => Some(Reach::Below),
            _ => None,
        }
    }
}

impl Behaviour {
    fn parse(text: &str) -> Option<Behaviour> {
        match text {
            "emit" => Some(Behaviour::Emit),
            "press" => Some(Behaviour::Press),
            "store" => Some(Behaviour::Store),
            "refine" => Some(Behaviour::Refine),
            "belt" => Some(Behaviour::Belt),
            "heater" => Some(Behaviour::Heater),
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
    /// Footprint in tiles, per instance rather than per type.
    ///
    /// A machine is the size its data says, but a vault is whatever the player marked
    /// out — a pit they dug and then declared. Zero means "as the type says" and is
    /// filled in when the entity is placed, so a caller that does not care never has to
    /// say.
    pub width_tiles: u32,
    pub height_tiles: u32,
    /// Value taken in but not yet pressed into a nugget, in points.
    ///
    /// Currency is matter (spec 5.1), so it can only be minted a whole cell at a time.
    /// A machine that has swallowed seven grains of an eight-grain nugget is holding
    /// those seven grains' worth here until the eighth arrives.
    pub bank: u32,
    /// Which way a belt conveys: `1` for increasing x, `-1` for decreasing. Meaningless
    /// off a `Behaviour::Belt`, same as `bank` is meaningless off a press.
    pub direction: i8,
    /// Whether this machine runs at all. A disabled machine does nothing, of any
    /// behaviour, while physics carries on around it untouched.
    ///
    /// For working on a running factory: pausing the emitter feeding the part you are
    /// rebuilding beats deleting and replacing it, which loses where it was pointed.
    pub enabled: bool,
    /// How much this machine does per action, overriding its type's `rate`. Zero means
    /// "as the type says" and is filled in at placement, the same convention
    /// `width_tiles`/`height_tiles` already use, so nothing downstream has to ask which
    /// of instance or type is authoritative.
    pub rate: u32,
}

impl Entity {
    pub const fn new(kind: EntityKind, tile_x: i32, tile_y: i32, element: ElementId) -> Entity {
        Entity {
            kind,
            tile_x,
            tile_y,
            element,
            width_tiles: 0,
            height_tiles: 0,
            bank: 0,
            direction: 1,
            enabled: true,
            rate: 0,
        }
    }

    /// An entity with a footprint the player chose, for the ones that have one.
    pub const fn sized(
        kind: EntityKind,
        tile_x: i32,
        tile_y: i32,
        width_tiles: u32,
        height_tiles: u32,
    ) -> Entity {
        Entity {
            kind,
            tile_x,
            tile_y,
            element: EMPTY,
            width_tiles,
            height_tiles,
            bank: 0,
            direction: 1,
            enabled: true,
            rate: 0,
        }
    }

    /// A belt, conveying in the direction given (`1` or `-1`).
    pub const fn belt(kind: EntityKind, tile_x: i32, tile_y: i32, direction: i8) -> Entity {
        Entity {
            kind,
            tile_x,
            tile_y,
            element: EMPTY,
            width_tiles: 0,
            height_tiles: 0,
            bank: 0,
            direction,
            enabled: true,
            rate: 0,
        }
    }

    /// A filter, conveying in the direction given and letting `element` fall through
    /// its underside instead. Chosen per instance, the same way an emitter's element
    /// is — one "filter" tool, tuned to whatever the player wants at placement.
    pub const fn filter(
        kind: EntityKind,
        tile_x: i32,
        tile_y: i32,
        direction: i8,
        element: ElementId,
    ) -> Entity {
        Entity {
            kind,
            tile_x,
            tile_y,
            element,
            width_tiles: 0,
            height_tiles: 0,
            bank: 0,
            direction,
            enabled: true,
            rate: 0,
        }
    }

    /// Fills in whatever the instance left as zero from the type. Called once, when the
    /// entity is placed, so everything downstream can read the instance and never the
    /// type.
    pub fn defaults_from(&mut self, definition: &EntityType) {
        if self.width_tiles == 0 {
            self.width_tiles = definition.width_tiles;
        }
        if self.height_tiles == 0 {
            self.height_tiles = definition.height_tiles;
        }
        if self.rate == 0 {
            self.rate = definition.rate;
        }
    }

    /// How much this machine does per action.
    ///
    /// Reads the instance, falling back to the type — `defaults_from` normally settles
    /// this at placement, so the fallback only covers an entity that was never placed,
    /// which is a thing tests construct.
    pub fn effective_rate(&self, definition: &EntityType) -> u32 {
        if self.rate == 0 {
            definition.rate
        } else {
            self.rate
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
    pub fn mouth(&self) -> i32 {
        (self.tile_y + self.height_tiles as i32) * TILE_CELLS as i32
    }

    /// The cells inside the machine, which is what a press works on.
    ///
    /// A machine is not matter (spec 4.1), so nothing rests on top of one: material
    /// falls straight *through* the tile. A press therefore works on what is inside it
    /// rather than on what is stacked above it, and anything it does not press — or
    /// cannot keep up with — carries on falling. Gravity is the conveyor.
    pub fn body(&self) -> (i32, i32, i32, i32) {
        let left = self.left();
        let top = self.top();
        (
            left,
            top,
            left + self.width_tiles as i32 * TILE_CELLS as i32 - 1,
            top + self.height_tiles as i32 * TILE_CELLS as i32 - 1,
        )
    }

    /// The cells a machine works on, given where its type says it reaches.
    ///
    /// `Body` is its own footprint; `Below` is the one tile row directly beneath it,
    /// as wide as the machine.
    pub fn reach_area(&self, reach: Reach) -> (i32, i32, i32, i32) {
        match reach {
            Reach::Body => self.body(),
            Reach::Below => {
                let left = self.left();
                let top = self.mouth();
                (
                    left,
                    top,
                    left + self.width_tiles as i32 * TILE_CELLS as i32 - 1,
                    top + TILE_CELLS as i32 - 1,
                )
            }
        }
    }

    /// Whether this covers a tile, accounting for a footprint wider than one.
    pub fn covers(&self, tile_x: i32, tile_y: i32) -> bool {
        tile_x >= self.tile_x
            && tile_y >= self.tile_y
            && tile_x < self.tile_x + self.width_tiles as i32
            && tile_y < self.tile_y + self.height_tiles as i32
    }

    /// The machine cannot do its job, which for an emitter means every cell it would
    /// emit into is occupied and for a press means there is nothing in it to press.
    ///
    /// Not a failure state — back-pressure is physical here as it is on belts (spec
    /// 4.2), and a machine that has backed up into its own input, or is being fed
    /// nothing at all, is worth showing.
    pub fn is_blocked<F: CellField + ?Sized>(
        &self,
        definition: &EntityType,
        field: &F,
        elements: &ElementTable,
    ) -> bool {
        match definition.behaviour {
            Behaviour::Emit => {
                let mouth = self.mouth();
                let left = self.left() + definition.mouth_offset as i32;
                let span = definition.mouth_width as i32;
                (0..span).all(|offset| field.get(left + offset, mouth) != Some(EMPTY))
            }
            Behaviour::Press => {
                let (x0, y0, x1, y1) = self.body();
                (y0..=y1).all(|y| (x0..=x1).all(|x| !is_pressable(field.get(x, y), elements)))
            }
            Behaviour::Refine => {
                let (x0, y0, x1, y1) = self.reach_area(definition.reach);
                (y0..=y1).all(|y| {
                    (x0..=x1).all(|x| !is_refinable(field.get(x, y), elements, definition.input))
                })
            }
            // A vault is a place, not a process. It cannot be blocked.
            Behaviour::Store => false,
            // Blocked means jammed: cargo is resting on it that cannot advance, because
            // the cell it would move into is occupied. An empty belt is not blocked —
            // it has nothing to be blocked on.
            Behaviour::Belt => {
                let (x0, y0, x1, _) = self.body();
                let carry_y = y0 - 1;
                let direction = i32::from(self.direction);
                (x0..=x1).any(|x| {
                    field.get(x, carry_y).is_some_and(|id| id != EMPTY)
                        && field.get(x + direction, carry_y) != Some(EMPTY)
                })
            }
            // Blocked means idle: no fuel anywhere in the body to burn. Pure id
            // equality against `input` — a heater does not consult the element table,
            // unlike Press and Refine, because there is no further property to check.
            Behaviour::Heater => {
                let (x0, y0, x1, y1) = self.body();
                (y0..=y1).all(|y| (x0..=x1).all(|x| field.get(x, y) != Some(definition.input)))
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
    /// Cells emitted per action, for emitters; cells pressed per action, for presses;
    /// cells of fuel consumed per action, for a heater.
    pub rate: u32,
    /// Ticks between actions. `1` is every tick, which is what everything did before
    /// pacing existed and remains the default.
    ///
    /// One field rather than a bespoke throttle per behaviour: a belt shoving cargo, an
    /// emitter emitting and a press pressing are all "the machine acts", and they all
    /// wanted slowing down at once. `tick % interval` is a pure function of the tick, so
    /// this cannot reach determinism (spec 3.1) — every machine of a kind acts on the
    /// same beat, which is also the predictable thing for a factory.
    pub interval: u32,
    /// Points a press must take in to make one cell of currency.
    pub gold_per: u32,
    /// The one element a `Refine` machine acts on, or a `Heater`'s fuel. `EMPTY` for
    /// anything else.
    pub input: ElementId,
    /// Where an emitter's opening actually is, in cells from the tile's left edge —
    /// matching the funnel the sprite draws rather than the full tile width. Defaults
    /// to the whole width, so anything that does not declare these keeps today's
    /// behaviour.
    pub mouth_offset: u32,
    pub mouth_width: u32,
    /// What a `Behaviour::Belt` entity's own body is made of — the indestructible
    /// element `World::place` fills its footprint with, so gravity holds cargo up using
    /// physics that already exists rather than an entity-aware exception in `step.rs`.
    /// `EMPTY` for anything that is not a belt.
    ///
    /// Named for the machine's frame rather than "structure", which is now also the
    /// name of the player's building material — two different things a field called
    /// `structure` could plausibly mean.
    pub chassis: ElementId,
    /// The whole-Kelvin temperature a `Heater` holds its own footprint at while it has
    /// fuel (spec 5.3). Meaningless for anything else.
    pub heat_output: i16,
    /// Where this machine looks for what it works on. Defaults to its own body, which
    /// is what every machine did before a piston needed to reach underneath itself.
    pub reach: Reach,
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
    pub fn from_json(source: &str, elements: &ElementTable) -> Result<EntityTable, DataError> {
        let document = json::parse(source).map_err(DataError::Json)?;
        let Some(entries) = document.get("entities").and_then(Json::as_array) else {
            return Ok(EntityTable::default());
        };

        let mut table = EntityTable::default();
        for entry in entries {
            let definition = parse_entity(entry, elements)?;
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

fn parse_entity(entry: &Json<'_>, elements: &ElementTable) -> Result<EntityType, DataError> {
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
    let behaviour = Behaviour::parse(behaviour_text).ok_or_else(|| bad("behaviour"))?;

    let width_tiles = field("widthTiles")?
        .as_i32()
        .filter(|value| *value > 0)
        .ok_or_else(|| bad("widthTiles"))? as u32;

    Ok(EntityType {
        id,
        name: field("name")?
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| bad("name"))?
            .to_owned(),
        behaviour,
        tool: field("tool")?
            .as_str()
            .ok_or_else(|| bad("tool"))?
            .to_owned(),
        width_tiles,
        height_tiles: field("heightTiles")?
            .as_i32()
            .filter(|value| *value > 0)
            .ok_or_else(|| bad("heightTiles"))? as u32,
        // Optional: only emitters use it.
        rate: entry.get("rate").and_then(Json::as_i32).unwrap_or(1).max(0) as u32,
        // Optional: absent means every tick, which is what everything did before
        // pacing existed. Floored at 1 — a zero interval would divide by zero.
        interval: entry
            .get("interval")
            .and_then(Json::as_i32)
            .unwrap_or(1)
            .max(1) as u32,
        // Optional: only presses use it, and a zero would mint infinitely.
        gold_per: entry
            .get("goldPer")
            .and_then(Json::as_i32)
            .unwrap_or(1)
            .max(1) as u32,
        // Optional: only Refine machines declare one. Resolved directly against the
        // already-complete element table — unlike `residue` on Element, entities load
        // after elements finish, so there is no forward-reference problem to solve.
        input: match entry.get("input").and_then(Json::as_str) {
            Some(name) => elements.id_of(name).ok_or_else(|| bad("input"))?,
            None => EMPTY,
        },
        // Optional: absent means the whole tile is the opening, which is today's
        // behaviour and is correct for anything that is not an emitter.
        mouth_offset: entry
            .get("mouthOffset")
            .and_then(Json::as_i32)
            .filter(|value| *value >= 0)
            .unwrap_or(0) as u32,
        mouth_width: match entry.get("mouthWidth").and_then(Json::as_i32) {
            Some(value) if value > 0 => value as u32,
            Some(_) => return Err(bad("mouthWidth")),
            None => width_tiles * TILE_CELLS,
        },
        // Required for a belt, since without it World::place would have nothing to fill
        // its footprint with and gravity would have nothing to hold cargo up on.
        // Absent — and meaningless — for anything else.
        chassis: match entry.get("chassis").and_then(Json::as_str) {
            Some(name) => elements.id_of(name).ok_or_else(|| bad("chassis"))?,
            None if behaviour == Behaviour::Belt => return Err(bad("chassis")),
            None => EMPTY,
        },
        // Optional: absent means a machine works on what falls into it, which is what
        // everything did before a piston needed to reach under itself.
        reach: match entry.get("reach").and_then(Json::as_str) {
            Some(text) => Reach::parse(text).ok_or_else(|| bad("reach"))?,
            None => Reach::Body,
        },
        // Required for a heater, the same way `chassis` is required for a belt.
        // Meaningless — and absent — for anything else.
        heat_output: match entry.get("heatOutput").and_then(Json::as_i32) {
            Some(value) if (i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&value) => {
                value as i16
            }
            Some(_) => return Err(bad("heatOutput")),
            None if behaviour == Behaviour::Heater => return Err(bad("heatOutput")),
            None => 0,
        },
    })
}

/// Whether a press would take this cell.
///
/// **Only what it can actually press.** Everything else — walls, water, unwashed sand,
/// and the nuggets it has already made — falls straight through and is none of its
/// business. That is what keeps a press from competing with the reaction it depends on:
/// a machine that ate reactants would starve itself of product by consuming the sand and
/// water before they could meet.
fn is_pressable(cell: Option<ElementId>, elements: &ElementTable) -> bool {
    let Some(id) = cell else {
        return false;
    };
    let Some(element) = elements.get(id) else {
        return false;
    };
    // Currency has no value by definition, so the check for it is belt and braces: a
    // machine must never grind its own output back into nothing.
    id != EMPTY && element.value > 0 && !element.currency
}

/// Runs every machine, in placement order, and returns how many nuggets were pressed
/// this tick.
///
/// This is the one place behaviour dispatches. Adding a belt means adding an arm here
/// and a row in the data file, and touching nothing else.
pub fn tick<F: CellField + ?Sized>(
    field: &mut F,
    entities: &mut [Entity],
    types: &EntityTable,
    elements: &ElementTable,
    seed: u64,
    tick: u64,
) -> u64 {
    // Belts run as their own pass, in a deliberate order, rather than inline in the
    // loop below with everything else — see `convey`'s doc for why order matters here
    // and nowhere else in this function.
    convey(field, entities, types, tick);

    let mut minted = 0;
    for (index, entity) in entities.iter_mut().enumerate() {
        let Some(definition) = types.get(entity.kind) else {
            continue;
        };
        // A disabled machine does nothing, whatever it is — one check rather than an
        // arm in each behaviour. Physics carries on around it untouched: what it
        // already made stays, and matter still falls through it.
        if !entity.enabled || !acts_this_tick(definition, tick) {
            continue;
        }
        match definition.behaviour {
            Behaviour::Emit => emit(field, entity, definition, elements, seed, tick, index),
            Behaviour::Press => minted += press(field, entity, definition, elements),
            Behaviour::Refine => refine(field, entity, definition, elements),
            Behaviour::Heater => heater(field, entity, definition),
            // A vault holds what falls into it and does nothing else. Gravity is the
            // mechanism; the walls are the player's. A belt was already handled above.
            Behaviour::Store | Behaviour::Belt => {}
        }
    }
    minted
}

/// Whether a machine of this kind acts on this tick, or is between actions.
///
/// Pure arithmetic on the tick, never on accumulated state, so it cannot drift and
/// cannot depend on how a world was reached — a replay lands on the same beat.
fn acts_this_tick(definition: &EntityType, tick: u64) -> bool {
    definition.interval <= 1 || tick.is_multiple_of(definition.interval as u64)
}

/// Runs every belt and filter for this tick, downstream tile first.
///
/// Every other behaviour here works on its own footprint alone, so processing order
/// between entities never matters. A belt does not: it shoves cargo out across its own
/// boundary into whatever tile is next, and if that tile also conveys this same tick,
/// running it *after* would shove the same cargo a second time. Ordering every belt by
/// how far downstream it is — furthest first — means a tile always resolves before the
/// neighbour that might hand cargo to it, so nothing this tick moves more than once.
fn convey<F: CellField + ?Sized>(
    field: &mut F,
    entities: &[Entity],
    types: &EntityTable,
    tick: u64,
) {
    let mut belts: Vec<usize> = entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| {
            entity.enabled
                && types.get(entity.kind).is_some_and(|definition| {
                    definition.behaviour == Behaviour::Belt && acts_this_tick(definition, tick)
                })
        })
        .map(|(index, _)| index)
        .collect();

    // Sorting by position times the negated direction puts the furthest-downstream
    // tile first regardless of which way it faces: a rightward belt's highest x sorts
    // first, a leftward belt's lowest x sorts first.
    belts.sort_by_key(|&index| {
        let entity = &entities[index];
        entity.left() as i64 * -i64::from(entity.direction)
    });

    for index in belts {
        convey_one(field, &entities[index]);
    }
}

/// Shoves whatever rests on top of one belt or filter tile one cell downstream, or —
/// for a filter, when the cargo matches `entity.element` — drops it straight down
/// through the tile's own solid body instead.
///
/// A filter is not a distinct behaviour; it is a belt whose instance names which
/// element it lets through, the same field an emitter's instance already uses to say
/// what it emits (`EMPTY` means a plain belt that lets nothing through). Choosing this
/// per instance rather than per machine type is what lets one "filter" tool be tuned to
/// gold, sand, or anything else at placement, the same way one "emitter" tool is.
///
/// Acts only on the row directly above the entity's own footprint; the footprint itself
/// is solid structure (`World::place`), so this never has to hold anything up itself.
/// Walked from the downstream column backward within this one tile so a short line of
/// cargo inside a single tile's row cannot cascade several cells in one tick, the same
/// reason `press` and `refine` work bottom-up.
fn convey_one<F: CellField + ?Sized>(field: &mut F, entity: &Entity) {
    let (x0, y0, x1, _) = entity.body();
    let carry_y = y0 - 1;
    let direction = i32::from(entity.direction);

    let columns: Vec<i32> = if direction >= 0 {
        (x0..=x1).rev().collect()
    } else {
        (x0..=x1).collect()
    };

    for x in columns {
        let Some(id) = field.get(x, carry_y) else {
            continue;
        };
        if id == EMPTY {
            continue;
        }

        if entity.element != EMPTY && id == entity.element {
            let drop_y = entity.mouth();
            if field.get(x, drop_y) == Some(EMPTY) {
                field.set(x, carry_y, EMPTY);
                field.set(x, drop_y, id);
                field.mark_active(x, carry_y);
                field.mark_active(x, drop_y);
            }
            continue;
        }

        let target_x = x + direction;
        if field.get(target_x, carry_y) == Some(EMPTY) {
            field.swap(x, carry_y, target_x, carry_y);
            field.mark_active(x, carry_y);
            field.mark_active(target_x, carry_y);
        }
    }
}

/// Whether a `Refine` machine of this `input` would take this cell.
///
/// Checked against the element table rather than trusting `input` alone, the same way
/// `is_pressable` does not trust `currency` alone — a machine whose input happens to
/// name something with no declared `refined_into` should read as blocked, not quietly
/// delete what it is fed.
fn is_refinable(cell: Option<ElementId>, elements: &ElementTable, input: ElementId) -> bool {
    let Some(id) = cell else {
        return false;
    };
    if id != input {
        return false;
    }
    elements.get(id).is_some_and(|element| element.refined_into != EMPTY)
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
    let left = entity.left() + definition.mouth_offset as i32;
    let mouth = entity.mouth();
    let span = definition.mouth_width;

    for grain in 0..entity.effective_rate(definition) {
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

/// Presses product into money, and returns how many nuggets it made.
///
/// A press takes only what it can press and banks each cell's declared value. Every
/// `goldPer` points, the cell it is working on becomes currency instead of empty space —
/// eight grains in, one nugget where the eighth was, which is what pillar 2 looks like
/// as an object.
///
/// Everything it cannot press falls through untouched, so a press never competes with
/// the reaction feeding it, and `rate` is a real throughput limit rather than a race:
/// product arriving faster than it can be pressed carries on past.
///
/// Bottom row up, left to right, up to `rate` cells a tick — no randomness, because
/// there is nothing here for it to decide.
fn press<F: CellField + ?Sized>(
    field: &mut F,
    entity: &mut Entity,
    definition: &EntityType,
    elements: &ElementTable,
) -> u64 {
    let (x0, y0, x1, y1) = entity.body();
    let Some(currency) = elements.iter().find(|element| element.currency) else {
        return 0;
    };

    let mut minted = 0;
    let mut taken = 0;
    // Material that has fallen furthest into the machine is the material about to fall
    // out of it, so that is what gets pressed first.
    for y in (y0..=y1).rev() {
        for x in x0..=x1 {
            if taken == entity.effective_rate(definition) {
                return minted;
            }
            let cell = field.get(x, y);
            if !is_pressable(cell, elements) {
                continue;
            }
            // `is_pressable` already confirmed this resolves, so the id is real.
            let element = cell.and_then(|id| elements.get(id)).expect("pressable cell");

            entity.bank += element.value;
            taken += 1;

            if entity.bank >= definition.gold_per {
                entity.bank -= definition.gold_per;
                field.set(x, y, currency.id);
                minted += 1;
            } else {
                // Converted, not destroyed (spec 5.3): what does not complete a nugget
                // becomes residue rather than vanishing. Only spending ever removes
                // matter from the world outright.
                field.set(x, y, element.residue);
            }
            // Neither writing nor clearing a cell is a move, so nothing else reports it
            // — and a pile that stops being told it is settling stops feeding the
            // machine.
            field.mark_active(x, y);
        }
    }
    minted
}

/// Turns cells of `definition.input` into their declared `refined_into`, up to `rate`
/// a tick.
///
/// One cell in, one cell out — unlike `press`, there is no threshold to bank toward, so
/// nothing here accumulates across ticks. Bottom row up, same reason `press` reads that
/// way: the material that has fallen furthest in is the material about to fall out.
fn refine<F: CellField + ?Sized>(
    field: &mut F,
    entity: &Entity,
    definition: &EntityType,
    elements: &ElementTable,
) {
    let (x0, y0, x1, y1) = entity.reach_area(definition.reach);

    let mut taken = 0;
    for y in (y0..=y1).rev() {
        for x in x0..=x1 {
            if taken == entity.effective_rate(definition) {
                return;
            }
            if !is_refinable(field.get(x, y), elements, definition.input) {
                continue;
            }
            // `is_refinable` already confirmed the id resolves and has a target.
            let target = elements.get(definition.input).expect("refinable cell").refined_into;

            field.set(x, y, target);
            // Writing a cell is not a move, so nothing else reports it — and a pile
            // that stops being told it is settling stops feeding the machine.
            field.mark_active(x, y);
            taken += 1;
        }
    }
}

/// Burns up to `rate` cells of fuel a tick and, if it burned any, holds its own
/// footprint at `heat_output`.
///
/// No banking, the same reason `refine` has none — there is nothing to accumulate
/// toward, since "hot" is not a threshold. Fed or not is decided fresh every tick: a
/// heater that just ran dry simply stops forcing its footprint hot, and the generic
/// conduction pass in `heat.rs` alone carries it back toward ambient afterward — this
/// function has no cooldown logic of its own to have.
fn heater<F: CellField + ?Sized>(field: &mut F, entity: &Entity, definition: &EntityType) {
    if burn_fuel(field, entity, definition) == 0 {
        return;
    }
    let (x0, y0, x1, y1) = entity.body();
    for y in y0..=y1 {
        for x in x0..=x1 {
            field.set_temperature(x, y, definition.heat_output);
        }
    }
}

/// Deletes up to `rate` cells of `definition.input` from the body and reports how many
/// it took. Bottom row up, same reason `press` and `refine` read that way.
fn burn_fuel<F: CellField + ?Sized>(
    field: &mut F,
    entity: &Entity,
    definition: &EntityType,
) -> u32 {
    let (x0, y0, x1, y1) = entity.body();

    let mut burned = 0;
    for y in (y0..=y1).rev() {
        for x in x0..=x1 {
            if burned == entity.effective_rate(definition) {
                return burned;
            }
            if field.get(x, y) != Some(definition.input) {
                continue;
            }
            // Eaten as valueless input (spec 1.1) — a heater has no byproduct, unlike
            // a press or a refiner, so this is a real deletion rather than a
            // conversion.
            field.set(x, y, EMPTY);
            field.mark_active(x, y);
            burned += 1;
        }
    }
    burned
}
