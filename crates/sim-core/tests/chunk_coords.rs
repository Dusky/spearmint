//! Chunk coordinate maths.
//!
//! The infinite canvas extends in every direction, so negative coordinates are ordinary
//! and not an edge case. Truncating division would put x = -1 and x = 0 in the same
//! chunk and fold the world over on itself at the origin, which is exactly the kind of
//! bug that shows up as an unreproducible desync months later.

use sim_core::chunk::{split, ChunkMap, CHUNK_CELLS};
use sim_core::field::CellField;

#[test]
fn splits_positive_coordinates() {
    assert_eq!(split(0), (0, 0));
    assert_eq!(split(1), (0, 1));
    assert_eq!(split(CHUNK_CELLS as i32 - 1), (0, CHUNK_CELLS - 1));
    assert_eq!(split(CHUNK_CELLS as i32), (1, 0));
    assert_eq!(split(CHUNK_CELLS as i32 + 5), (1, 5));
}

#[test]
fn splits_negative_coordinates_without_folding_at_the_origin() {
    assert_eq!(split(-1), (-1, CHUNK_CELLS - 1));
    assert_eq!(split(-(CHUNK_CELLS as i32)), (-1, 0));
    assert_eq!(split(-(CHUNK_CELLS as i32) - 1), (-2, CHUNK_CELLS - 1));

    // Every coordinate in a run must map to a distinct (chunk, offset) pair.
    let mut seen = std::collections::BTreeSet::new();
    for coordinate in -400..400 {
        assert!(seen.insert(split(coordinate)), "collision at {coordinate}");
    }
}

#[test]
fn stores_and_reads_across_the_origin() {
    let mut map = ChunkMap::new();
    let positions = [
        (-500, -500),
        (-1, -1),
        (0, 0),
        (1, 1),
        (500, 500),
        (-1, 500),
    ];

    for (index, &(x, y)) in positions.iter().enumerate() {
        map.set(x, y, index as u8 + 1);
    }
    for (index, &(x, y)) in positions.iter().enumerate() {
        assert_eq!(map.get(x, y), Some(index as u8 + 1), "at ({x}, {y})");
    }
}

#[test]
fn unwritten_space_reads_as_empty_without_allocating() {
    let mut map = ChunkMap::new();
    assert_eq!(map.get(10_000, -10_000), Some(sim_core::EMPTY));
    assert_eq!(map.chunk_count(), 0);

    // Writing empty into nothing must not conjure a chunk to hold it.
    map.set(10_000, -10_000, sim_core::EMPTY);
    assert_eq!(map.chunk_count(), 0);

    map.set(10_000, -10_000, 1);
    assert_eq!(map.chunk_count(), 1);
}

#[test]
fn swaps_across_a_chunk_boundary() {
    let mut map = ChunkMap::new();
    let seam = CHUNK_CELLS as i32;

    map.set(seam - 1, 0, 2);
    map.set(seam, 0, 3);
    assert_eq!(
        map.chunk_count(),
        2,
        "the two cells should be in two chunks"
    );

    map.swap(seam - 1, 0, seam, 0);

    assert_eq!(map.get(seam - 1, 0), Some(3));
    assert_eq!(map.get(seam, 0), Some(2));
}

#[test]
fn bounds_cover_every_stored_chunk() {
    let mut map = ChunkMap::new();
    assert_eq!(map.bounds(), None, "an empty world has nothing to sweep");

    map.set(0, 0, 1);
    map.set(-1, -1, 1);
    let bounds = map.bounds().expect("bounds");

    assert!(bounds.min_x <= -1 && bounds.min_y <= -1);
    assert!(bounds.max_x >= 0 && bounds.max_y >= 0);
    assert_eq!(bounds.width(), 2 * i64::from(CHUNK_CELLS));
    assert_eq!(bounds.height(), 2 * i64::from(CHUNK_CELLS));
}
