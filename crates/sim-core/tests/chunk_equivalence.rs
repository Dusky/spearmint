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
use sim_core::field::CellField;
use sim_core::scene;

/// Compares cell by cell over the scene's own extent, so a mismatch names a position
/// rather than just a differing hash.
fn assert_same_world(width: u32, height: u32, seed: u64, ticks: u64) {
    let rules = common::rules_without_reactions();

    let mut chunked = scene::sandbox(width, height, seed, &rules);
    let mut flat = scene::sandbox_flat(width, height, seed, &rules);

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
    let rules = common::rules_without_reactions();
    let world = scene::sandbox(width, height, 7, &rules);
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
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let mut world = scene::sandbox(96, 72, 5, &rules);
    world.step_many(300);

    let before = world.hash();
    let counts: Vec<usize> = table.iter().map(|e| world.count_of(e.id)).collect();

    world.field_mut().compact();

    assert_eq!(world.hash(), before, "compaction changed the world");
    let after: Vec<usize> = table.iter().map(|e| world.count_of(e.id)).collect();
    assert_eq!(counts, after);

    // And it must still simulate identically afterwards.
    let mut untouched = scene::sandbox(96, 72, 5, &rules);
    untouched.step_many(300);
    world.step_many(100);
    untouched.step_many(100);
    assert_eq!(world.hash(), untouched.hash());
}

/// Compaction is the one pass that carries state *down* a column rather than working
/// cell-locally, and it walks `bounds()` — which is not the same rectangle in a chunked
/// field as in a flat one. That combination is exactly the shape of thing that could
/// diverge, and the shipped sandbox would never catch it: it contains structure, sand
/// and water, none of which compact, so the pass never fires there at all.
#[test]
fn matches_the_flat_world_with_a_pile_deep_enough_to_compact() {
    let rules = common::rules_without_reactions();
    let (width, height, seed) = (96, 120, 0x0005_1105_u64);
    let burnt = rules.elements.id_of("burntResidue").expect("burntResidue");
    let structure = rules.elements.id_of("structure").expect("structure");
    let element = rules.elements.get(burnt).expect("burntResidue");
    let depth = element.compaction_load / element.density * 3;

    let mut chunked = scene::sandbox(width, height, seed, &rules);
    let mut flat = scene::sandbox_flat(width, height, seed, &rules);

    // Two shafts: one against the left wall and one out in the middle, so the sweep is
    // exercised both where a chunk boundary is near and where it is not.
    let build = |field: &mut dyn CellField| {
        for x in [8, 53] {
            let floor = height as i32 - 2;
            for y in (floor - depth - 1)..=floor {
                field.set(x - 1, y, structure);
                field.set(x + 1, y, structure);
            }
            for offset in 0..depth {
                field.set(x, floor - offset, burnt);
            }
        }
    };
    build(chunked.field_mut());
    build(flat.field_mut());

    chunked.step_many(300);
    flat.step_many(300);

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            assert_eq!(
                chunked.get(x, y),
                flat.get(x, y),
                "differed at ({x}, {y}) after compacting, seed {seed:#x}"
            );
        }
    }
    assert_eq!(chunked.hash(), flat.hash(), "canonical hashes differ after compacting");

    // And the scene has to have actually compacted, or this proves nothing.
    let fuel = rules.elements.id_of("fuel").expect("fuel");
    let converted = (0..height as i32)
        .flat_map(|y| (0..width as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| flat.get(x, y) == fuel)
        .count();
    assert!(converted > 0, "the pile should have compacted something");
}
