//! The browser bridge.
//!
//! Deliberately raw: `extern "C"` exports over integers and a pointer into linear
//! memory, with a hand-written JS binding on the other side. The interface is small
//! enough that `wasm-bindgen` would add a toolchain and a dependency tree to save
//! very little.
//!
//! All platform coupling lives here. `sim-core` stays free of it, so the same crate
//! still compiles native for server-side replay verification (spec 8.3).
//!
//! Coordinates are world cells. The canvas is rendered at cell resolution and scaled up
//! by the page, so one pixel here is one cell.

use std::cell::RefCell;

use sim_core::chunk::TILE_CELLS;
use sim_core::field::CellField;
use sim_core::{paint, scene, ElementTable, Entity, Rules, World};

mod sprites;

/// Compiled in, so the module needs no fetch to start and the sim and the client agree
/// on element ids by construction — the client reads the same file.
const ELEMENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/elements.json"
));

/// Machines, as opposed to matter. Same story: compiled in, and the client reads the
/// same file for its tool list and its sprite preview.
const ENTITIES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/entities.json"
));

struct State {
    world: World,
    table: ElementTable,
    sprites: sprites::SpriteTable,
    /// RGBA, one pixel per cell, reused between frames.
    frame: Vec<u8>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with_state<R>(default: R, body: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|cell| match cell.borrow_mut().as_mut() {
        Some(state) => body(state),
        None => default,
    })
}

/// Creates the world. Returns 1 on success, 0 if the element data failed to load.
#[no_mangle]
pub extern "C" fn sim_init(seed: u32, width: u32, height: u32) -> u32 {
    let Ok(rules) = Rules::load(ELEMENTS_JSON, ENTITIES_JSON) else {
        return 0;
    };
    let Ok(sprites) = sprites::SpriteTable::from_json(ENTITIES_JSON) else {
        return 0;
    };
    let table = rules.elements.clone();
    let world = scene::arena(width.max(8), height.max(8), u64::from(seed), &rules);
    STATE.with(|cell| {
        *cell.borrow_mut() = Some(State {
            world,
            table,
            sprites,
            frame: Vec::new(),
        });
    });
    1
}

/// Advances the simulation. The host owns the fixed-timestep accumulator (spec 3.1),
/// so this just runs the number of ticks it is told to.
#[no_mangle]
pub extern "C" fn sim_step(ticks: u32) {
    with_state((), |state| state.world.step_many(u64::from(ticks)));
}

#[no_mangle]
pub extern "C" fn sim_tick() -> u32 {
    with_state(0, |state| state.world.tick() as u32)
}

/// Renders a window of the world into linear memory as RGBA and returns its address.
///
/// Rendering happens here rather than in JavaScript because the world is chunked and
/// sparse; reproducing that layout across the boundary would mean shipping the chunk
/// map to the client every frame.
///
/// The returned pointer is only valid until the next call — the buffer is reused, and
/// growing it can move it.
#[no_mangle]
pub extern "C" fn sim_render(origin_x: i32, origin_y: i32, width: u32, height: u32) -> *const u8 {
    STATE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(state) = borrow.as_mut() else {
            return std::ptr::null();
        };

        let needed = width as usize * height as usize * 4;
        if state.frame.len() != needed {
            state.frame.resize(needed, 0);
        }

        for row in 0..height {
            for column in 0..width {
                let x = origin_x + column as i32;
                let y = origin_y + row as i32;
                let id = state.world.get(x, y);

                let (red, green, blue) = match state.table.get(id) {
                    Some(element) => {
                        // Per-cell colour variance, hashed from position so a cell does
                        // not shimmer between frames.
                        let spread = i32::from(element.color_variance);
                        let shift = if spread == 0 {
                            0
                        } else {
                            let noise =
                                sim_core::rng::below(0x9e37, 0, x, y, 7, (spread * 2 + 1) as u32);
                            noise as i32 - spread
                        };
                        (
                            clamp(i32::from(element.color[0]) + shift),
                            clamp(i32::from(element.color[1]) + shift),
                            clamp(i32::from(element.color[2]) + shift),
                        )
                    }
                    // The void. Matches the client's --sim-void token.
                    None => (0x07, 0x08, 0x06),
                };

                let offset = (row as usize * width as usize + column as usize) * 4;
                state.frame[offset] = red;
                state.frame[offset + 1] = green;
                state.frame[offset + 2] = blue;
                state.frame[offset + 3] = 255;
            }
        }

        draw_entities(state, origin_x, origin_y, width, height);
        state.frame.as_ptr()
    })
}

fn clamp(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Draws entity sprites over the cells.
///
/// After the cell pass, and skipping any pixel where matter already sits, so piling sand
/// over a machine buries it. Spec 4.3 wants burial to be visible; drawing in this order
/// gets that for free rather than as a special case.
fn draw_entities(state: &mut State, origin_x: i32, origin_y: i32, width: u32, height: u32) {
    for index in 0..state.world.entities().len() {
        let entity = state.world.entities()[index];
        let Some(definition) = state.world.rules().entities.get(entity.kind).cloned() else {
            continue;
        };

        let status = if entity.is_blocked(&definition, state.world.field()) {
            sprites::Status::Blocked
        } else {
            sprites::Status::Running
        };

        let element = state.table.get(entity.element);
        let element_name = element.map_or(String::new(), |element| element.name.clone());
        let element_state = element.map(|element| element.state);
        let element_colour = element.map_or([0x98, 0x96, 0x8F], |element| element.color);

        let Some(pixels) = state
            .sprites
            .pick(entity.kind, &element_name, element_state, status)
            .map(<[u8]>::to_vec)
        else {
            continue;
        };

        // A footprint wider than one tile repeats the sprite across it. Bespoke art for
        // large machines can come when something actually needs it.
        for tile_y in 0..definition.height_tiles as i32 {
            for tile_x in 0..definition.width_tiles as i32 {
                blit(
                    state,
                    &pixels,
                    element_colour,
                    entity.left() + tile_x * TILE_CELLS as i32,
                    entity.top() + tile_y * TILE_CELLS as i32,
                    origin_x,
                    origin_y,
                    width,
                    height,
                );
            }
        }
    }
}

/// Draws one pixel map into the frame, skipping pixels where matter already sits.
#[allow(clippy::too_many_arguments)]
fn blit(
    state: &mut State,
    pixels: &[u8],
    element_colour: [u8; 3],
    tile_left: i32,
    tile_top: i32,
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
) {
    for row in 0..sprites::SPRITE_SIZE {
        for column in 0..sprites::SPRITE_SIZE {
            let Some(colour) =
                sprites::colour_of(sprites::ink_at(pixels, column, row), element_colour)
            else {
                continue;
            };

            let world_x = tile_left + column as i32;
            let world_y = tile_top + row as i32;
            // Matter wins: a buried machine is hidden by what buried it.
            if state.world.get(world_x, world_y) != 0 {
                continue;
            }

            let screen_x = world_x - origin_x;
            let screen_y = world_y - origin_y;
            if screen_x < 0 || screen_y < 0 {
                continue;
            }
            let (screen_x, screen_y) = (screen_x as u32, screen_y as u32);
            if screen_x >= width || screen_y >= height {
                continue;
            }

            let offset = (screen_y as usize * width as usize + screen_x as usize) * 4;
            state.frame[offset] = colour[0];
            state.frame[offset + 1] = colour[1];
            state.frame[offset + 2] = colour[2];
            state.frame[offset + 3] = 255;
        }
    }
}

/// Paints a stroke of whole tiles, as a drag does.
///
/// Coordinates are tiles here, not cells: everything the player builds sits on the tile
/// grid (spec 2.3). The rule itself lives in `sim_core::paint`.
#[no_mangle]
pub extern "C" fn sim_paint_tiles(
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    id: u32,
) {
    with_state((), |state| {
        paint::stroke(
            state.world.field_mut(),
            (tile_x0, tile_y0),
            (tile_x1, tile_y1),
            id as u8,
        );
    });
}

/// The element at a cell, for alt-click material picking.
#[no_mangle]
pub extern "C" fn sim_get(x: i32, y: i32) -> u32 {
    with_state(0, |state| u32::from(state.world.get(x, y)))
}

/// How many cells hold this element.
#[no_mangle]
pub extern "C" fn sim_count(id: u32) -> u32 {
    with_state(0, |state| state.world.count_of(id as u8) as u32)
}

#[no_mangle]
pub extern "C" fn sim_chunk_count() -> u32 {
    with_state(0, |state| state.world.field().chunk_count() as u32)
}

#[no_mangle]
pub extern "C" fn sim_awake_chunk_count() -> u32 {
    with_state(0, |state| state.world.awake_chunk_count() as u32)
}

/// Enables viewport-gated sleeping (spec 2.4).
#[no_mangle]
pub extern "C" fn sim_set_sleeping(enabled: u32) {
    with_state((), |state| state.world.set_sleeping(enabled != 0));
}

/// Places a machine on a tile. Returns 1 if it went down.
///
/// Generic over kind: a belt or a teleporter needs no new export here, only a row in
/// `data/entities.json` and a behaviour arm in the core.
#[no_mangle]
pub extern "C" fn sim_place_entity(kind: u32, tile_x: i32, tile_y: i32, element: u32) -> u32 {
    with_state(0, |state| {
        // One machine per tile.
        if state.world.entity_at(tile_x, tile_y).is_some() {
            return 0;
        }
        state
            .world
            .place(Entity::new(kind as u8, tile_x, tile_y, element as u8));
        1
    })
}

/// The machine covering this world cell, or -1. For hit-testing a click or a hover.
#[no_mangle]
pub extern "C" fn sim_entity_at(x: i32, y: i32) -> i32 {
    with_state(-1, |state| {
        let tile_x = x.div_euclid(TILE_CELLS as i32);
        let tile_y = y.div_euclid(TILE_CELLS as i32);
        state
            .world
            .entity_at(tile_x, tile_y)
            .map_or(-1, |index| index as i32)
    })
}

/// Removes a machine, freeing its slot in full. Returns 1 if one was there.
#[no_mangle]
pub extern "C" fn sim_remove_entity(index: u32) -> u32 {
    with_state(0, |state| u32::from(state.world.remove(index as usize)))
}

#[no_mangle]
pub extern "C" fn sim_entity_count() -> u32 {
    with_state(0, |state| state.world.entities().len() as u32)
}

/// How many of one kind are placed, which is what a cap is enforced against.
#[no_mangle]
pub extern "C" fn sim_entity_count_of_kind(kind: u32) -> u32 {
    with_state(0, |state| state.world.count_of_kind(kind as u8) as u32)
}

/// Where a machine sits, so the client can outline it. `i32::MIN` for a bad index.
#[no_mangle]
pub extern "C" fn sim_entity_tile_x(index: u32) -> i32 {
    with_state(i32::MIN, |state| {
        state
            .world
            .entities()
            .get(index as usize)
            .map_or(i32::MIN, |entity| entity.tile_x)
    })
}

#[no_mangle]
pub extern "C" fn sim_entity_tile_y(index: u32) -> i32 {
    with_state(i32::MIN, |state| {
        state
            .world
            .entities()
            .get(index as usize)
            .map_or(i32::MIN, |entity| entity.tile_y)
    })
}

/// Cells where the two reactants of a reaction are touching.
///
/// This is contact area in the literal sense — not a modelled parameter but a count of
/// where the reaction can actually happen, which is what makes yield a property of the
/// machine's shape (spec 3.3).
#[no_mangle]
pub extern "C" fn sim_contact_area() -> u32 {
    with_state(0, |state| {
        let reactions = &state.world.rules().reactions;
        if reactions.is_empty() {
            return 0;
        }
        let Some(bounds) = state.world.field().bounds() else {
            return 0;
        };

        let mut contacts = 0u32;
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let here = state.world.get(x, y);
                if here == 0 {
                    continue;
                }
                // Right and below only, so each touching pair is counted once.
                for (dx, dy) in [(1, 0), (0, 1)] {
                    let there = state.world.get(x + dx, y + dy);
                    if there != 0 && reactions.between(here, there).is_some() {
                        contacts += 1;
                    }
                }
            }
        }
        contacts
    })
}
