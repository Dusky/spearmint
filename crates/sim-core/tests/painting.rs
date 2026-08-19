//! Drawing lands on the tile grid.
//!
//! Everything the player builds is tile-aligned (spec 2.3). The bug this file pins:
//! a stroke used to snap only its endpoints and interpolate between them in cell space,
//! so a diagonal drag laid down a smooth band of half-filled tiles instead of a
//! staircase of whole ones.

mod common;

use sim_core::chunk::TILE_CELLS;
use sim_core::elements::EMPTY;
use sim_core::field::CellField;
use sim_core::{paint, scene, World};

const CELLS: i32 = TILE_CELLS as i32;

/// How many of a tile's cells hold `id`. A tile is only ever 0 or all of them.
fn count_in_tile<F: CellField + ?Sized>(field: &F, tile_x: i32, tile_y: i32, id: u8) -> i32 {
    let left = tile_x * CELLS;
    let top = tile_y * CELLS;
    let mut found = 0;
    for row in 0..CELLS {
        for column in 0..CELLS {
            if field.get(left + column, top + row) == Some(id) {
                found += 1;
            }
        }
    }
    found
}

/// Asserts every tile in a region is either untouched or completely filled with `id`.
fn assert_tiles_whole<F: CellField + ?Sized>(
    field: &F,
    tiles: (i32, i32, i32, i32),
    id: u8,
    what: &str,
) {
    let (x0, y0, x1, y1) = tiles;
    for tile_y in y0..=y1 {
        for tile_x in x0..=x1 {
            let found = count_in_tile(field, tile_x, tile_y, id);
            assert!(
                found == 0 || found == CELLS * CELLS,
                "{what}: tile ({tile_x}, {tile_y}) is {found}/{} filled — a partial tile",
                CELLS * CELLS
            );
        }
    }
}

#[test]
fn a_diagonal_stroke_leaves_no_partial_tile() {
    let rules = common::rules();
    let structure = rules.elements.id_of("structure").expect("structure");
    // An empty world, not the arena: the arena's boundary frame is one cell thick and
    // deliberately off-grid, which would be indistinguishable from a partial tile here.
    let mut world = World::new(1, rules.clone());

    // A shallow diagonal and a steep one: Bresenham steps differently along each axis,
    // and the old code was wrong on both.
    paint::stroke(world.field_mut(), (2, 2), (14, 8), structure);
    paint::stroke(world.field_mut(), (20, 2), (24, 18), structure);

    assert_tiles_whole(world.field_mut(), (0, 0, 30, 24), structure, "diagonal stroke");
}

#[test]
fn a_stroke_covers_every_tile_between_its_ends() {
    let rules = common::rules();
    let structure = rules.elements.id_of("structure").expect("structure");
    // An empty world, not the arena: the arena's boundary frame is one cell thick and
    // deliberately off-grid, which would be indistinguishable from a partial tile here.
    let mut world = World::new(1, rules.clone());

    paint::stroke(world.field_mut(), (3, 5), (9, 5), structure);

    let full = CELLS * CELLS;
    for tile_x in 3..=9 {
        assert_eq!(
            count_in_tile(world.field_mut(), tile_x, 5, structure),
            full,
            "tile ({tile_x}, 5) should be filled end to end, endpoints included"
        );
    }
    assert_eq!(count_in_tile(world.field_mut(), 2, 5, structure), 0);
    assert_eq!(count_in_tile(world.field_mut(), 10, 5, structure), 0);
}

#[test]
fn a_single_tile_stroke_fills_exactly_one_tile() {
    let rules = common::rules();
    let structure = rules.elements.id_of("structure").expect("structure");
    // An empty world, not the arena: the arena's boundary frame is one cell thick and
    // deliberately off-grid, which would be indistinguishable from a partial tile here.
    let mut world = World::new(1, rules.clone());

    paint::stroke(world.field_mut(), (6, 6), (6, 6), structure);

    assert_eq!(count_in_tile(world.field_mut(), 6, 6, structure), CELLS * CELLS);
    assert_eq!(count_in_tile(world.field_mut(), 7, 6, structure), 0);
    assert_eq!(count_in_tile(world.field_mut(), 6, 7, structure), 0);
}

#[test]
fn erasing_clears_whole_tiles() {
    let rules = common::rules();
    let structure = rules.elements.id_of("structure").expect("structure");
    // An empty world, not the arena: the arena's boundary frame is one cell thick and
    // deliberately off-grid, which would be indistinguishable from a partial tile here.
    let mut world = World::new(1, rules.clone());

    paint::stroke(world.field_mut(), (2, 2), (14, 8), structure);
    // Across the diagonal, not along it, so the erase and the wall disagree about
    // direction — which is where a half-cleared tile would show up.
    paint::stroke(world.field_mut(), (2, 8), (14, 2), EMPTY);

    assert_tiles_whole(world.field_mut(), (0, 0, 16, 10), structure, "after erasing");
}

/// Drawing writes cells rather than moving them, and only movement marks a chunk dirty.
/// Without an explicit wake, powder drawn into a chunk that has already settled hangs in
/// the air until something else happens to disturb its neighbourhood.
///
/// A freshly allocated chunk starts awake, so the sand has to land in a chunk that
/// already exists and has gone to sleep — otherwise this passes either way and pins
/// nothing.
#[test]
fn drawing_wakes_a_sleeping_chunk() {
    let rules = common::rules_without_reactions();
    let structure = rules.elements.id_of("structure").expect("structure");
    let sand = rules.elements.id_of("sand").expect("sand");
    let mut world = scene::arena(200, 200, 7, &rules);

    // A pedestal, so the chunk the sand lands in is allocated and settled before the
    // draw. Tiles are 9 cells and chunks 16 tiles, so both of these sit in chunk (0, 0).
    paint::stroke(world.field_mut(), (3, 6), (7, 6), structure);
    world.set_sleeping(true);
    world.step_many(30);

    let before = world.content_hash();
    world.step();
    assert_eq!(
        world.content_hash(),
        before,
        "the world should be at rest before the draw"
    );

    paint::stroke(world.field_mut(), (5, 3), (5, 3), sand);
    // Measured after the draw: the sand itself changes the world, so the question is
    // whether it then *moves*.
    let drawn = world.content_hash();
    world.step_many(5);

    assert_ne!(
        world.content_hash(),
        drawn,
        "sand drawn into a sleeping chunk never started falling"
    );
}
