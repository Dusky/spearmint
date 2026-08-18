//! Milestone 2a's gate: chunked storage must be *transparent*.
//!
//! Spec 3.1 warns that determinism must be proven before anything is built on top of
//! it. Chunking is the first thing built on top of the sim, so it gets the same
//! treatment Milestone 1 got: a world stored in sparse chunks and a world stored in one
//! flat array must produce identical results, tick for tick.
//!
//! This is only meaningful because both run the *same* tick loop, generic over the
//! `CellField` trait. Two implementations of the rules agreeing would prove much less.
//!
//! Note what the equivalence rests on: the chunked sweep visits whole chunks, so it
//! covers empty cells the flat sweep never sees. Those cost nothing and change nothing
//! — an empty cell is skipped without consuming any randomness, because randomness is a
//! position hash rather than a stream. With a streaming PRNG every one of those extra
//! visits would shift the sequence and the two worlds would diverge immediately.

mod common;

use sim_core::chunk::CHUNK_CELLS;
use sim_core::scene;

/// Compares cell by cell over the scene's own extent, so a mismatch names a position
/// rather than just a differing hash.
fn assert_same_world(width: u32, height: u32, seed: u64, ticks: u64) {
    let table = common::table();

    let mut chunked = scene::sandbox(width, height, seed, &table);
    let mut flat = scene::sandbox_flat(width, height, seed, &table);

    chunked.step_many(ticks);
    flat.step_many(ticks);

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            assert_eq!(
                chunked.get(x, y),
                flat.get(x, y),
                "differed at ({x}, {y}) after {ticks} ticks, seed {seed:#x}, {width}x{height}"
            );
        }
    }

    assert_eq!(
        chunked.hash(),
        flat.hash(),
        "canonical hashes differ at {width}x{height}, seed {seed:#x}"
    );
}

#[test]
fn matches_the_flat_world_inside_one_chunk() {
    assert_same_world(96, 72, 0x4f2a11, 600);
}

/// The case that matters: material crossing chunk boundaries in both axes.
#[test]
fn matches_the_flat_world_across_several_chunks() {
    let width = CHUNK_CELLS * 2 + 40;
    let height = CHUNK_CELLS + 60;
    assert_same_world(width, height, 7, 400);

    // The scene must genuinely span more than one chunk, or this proves nothing.
    let table = common::table();
    let world = scene::sandbox(width, height, 7, &table);
    assert!(
        world.field().chunk_count() >= 6,
        "expected a multi-chunk scene, got {}",
        world.field().chunk_count()
    );
}

/// A scene sized to land material exactly on a chunk seam.
#[test]
fn matches_the_flat_world_on_a_chunk_seam() {
    assert_same_world(CHUNK_CELLS, CHUNK_CELLS, 0xbeef, 400);
    assert_same_world(CHUNK_CELLS + 1, CHUNK_CELLS + 1, 0xbeef, 400);
}

#[test]
fn matches_across_seeds() {
    for seed in [0, 1, 0xdead_beef, u64::MAX] {
        assert_same_world(80, 60, seed, 200);
    }
}

/// Chunk allocation is an internal optimisation and must never be observable. Dropping
/// chunks that hold nothing has to leave the world identical.
#[test]
fn compaction_is_unobservable() {
    let table = common::table();
    let mut world = scene::sandbox(96, 72, 5, &table);
    world.step_many(300);

    let before = world.hash();
    let counts: Vec<usize> = table.iter().map(|e| world.count_of(e.id)).collect();

    world.field_mut().compact();

    assert_eq!(world.hash(), before, "compaction changed the world");
    let after: Vec<usize> = table.iter().map(|e| world.count_of(e.id)).collect();
    assert_eq!(counts, after);

    // And it must still simulate identically afterwards.
    let mut untouched = scene::sandbox(96, 72, 5, &table);
    untouched.step_many(300);
    world.step_many(100);
    untouched.step_many(100);
    assert_eq!(world.hash(), untouched.hash());
}
