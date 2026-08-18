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

use sim_core::field::CellField;
use sim_core::{scene, ElementTable, Rules, Spawner, World};

/// Compiled in, so the module needs no fetch to start and the sim and the client agree
/// on element ids by construction — the client reads the same file.
const ELEMENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/elements.json"
));

struct State {
    world: World,
    table: ElementTable,
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
    let Ok(rules) = Rules::from_json(ELEMENTS_JSON) else {
        return 0;
    };
    let table = rules.elements.clone();
    let world = scene::arena(width.max(8), height.max(8), u64::from(seed), &rules);
    STATE.with(|cell| {
        *cell.borrow_mut() = Some(State {
            world,
            table,
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

        state.frame.as_ptr()
    })
}

fn clamp(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Paints a line of cells, as a drag does. `half_width` of 0 is a single cell.
#[no_mangle]
pub extern "C" fn sim_paint_line(x0: i32, y0: i32, x1: i32, y1: i32, id: u32, half_width: u32) {
    with_state((), |state| {
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        let half = half_width as i32;
        for step in 0..=steps {
            // Integer interpolation — the sim has no floats anywhere (spec 3.1).
            let x = x0 + (x1 - x0) * step / steps;
            let y = y0 + (y1 - y0) * step / steps;
            for dy in -half..=half {
                for dx in -half..=half {
                    state.world.field_mut().set(x + dx, y + dy, id as u8);
                }
            }
        }
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

/// Places a spawner. Spawner count is the game's only hard input cap (spec 3.4).
#[no_mangle]
pub extern "C" fn sim_add_spawner(x: i32, y: i32, width: u32, element: u32, rate: u32) {
    with_state((), |state| {
        state.world.add_spawner(Spawner {
            x,
            y,
            width: width.max(1),
            element: element as u8,
            rate: rate.max(1),
        });
    });
}

#[no_mangle]
pub extern "C" fn sim_spawner_count() -> u32 {
    with_state(0, |state| state.world.spawners().len() as u32)
}

#[no_mangle]
pub extern "C" fn sim_clear_spawners() {
    with_state((), |state| state.world.clear_spawners());
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
