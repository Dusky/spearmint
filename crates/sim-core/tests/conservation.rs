//! Spec 1.1: particles are never created, destroyed, or summarised outside the physics
//! rules themselves. The spec calls this constraint load-bearing, so it gets a test
//! rather than a convention.
//!
//! Milestone 1 has no spawners and no reactions, so in a closed box every element's
//! census must be exactly what it started as — forever. This is what catches a
//! displacement rule that drops a particle, long before anyone notices sand quietly
//! evaporating out of a factory.

mod common;

use sim_core::{scene, EMPTY};

#[test]
fn nothing_is_created_or_destroyed() {
    let table = common::table();

    // Several seeds on the flat world, which simulates the same rules for a fraction
    // of the cost; one chunked run below covers the storage.
    for seed in [3, 0x4f2a11, 0xfeed_face] {
        let mut world = scene::sandbox_flat(96, 72, seed, &table);

        let before: Vec<(u8, usize)> = table
            .iter()
            .map(|element| (element.id, world.count_of(element.id)))
            .collect();

        // Every element must actually be present, or the test would pass vacuously.
        for (id, count) in &before {
            assert!(
                *count > 0,
                "seed {seed:#x}: element {id} absent from the scene"
            );
        }

        world.step_many(10_000);

        for (id, expected) in before {
            assert_eq!(
                world.count_of(id),
                expected,
                "seed {seed:#x}: element {id} census changed"
            );
        }
    }
}

/// The flat world additionally has a fixed number of cells, so the void is countable
/// and must balance too.
#[test]
fn the_flat_world_conserves_its_void_as_well() {
    let table = common::table();
    let mut world = scene::sandbox_flat(96, 72, 3, &table);

    let before: Vec<(u8, usize)> = table
        .iter()
        .map(|element| (element.id, world.count_of(element.id)))
        .collect();
    let empty_before = world.count_of(EMPTY);

    world.step_many(10_000);

    for (id, expected) in before {
        assert_eq!(world.count_of(id), expected, "element {id} census changed");
    }
    assert_eq!(world.count_of(EMPTY), empty_before, "void count changed");
    assert_eq!(world.cells().len(), 96 * 72, "the grid changed size");
}

/// Solids do not move. Sand may pile against a wall, but no wall may end up somewhere
/// it was not placed.
#[test]
fn solids_never_move() {
    let table = common::table();
    let wall = table.id_of("wall").expect("wall element");
    let mut world = scene::sandbox(80, 60, 0x1234, &table);

    let placed: Vec<(i32, i32)> = (0..60)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .filter(|&(x, y)| world.get(x, y) == wall)
        .collect();
    assert!(!placed.is_empty());

    world.step_many(2_000);

    let after: Vec<(i32, i32)> = (0..60)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .filter(|&(x, y)| world.get(x, y) == wall)
        .collect();
    assert_eq!(placed, after, "a solid moved");
}

/// Chunked storage must conserve as well — and it is the one that can genuinely lose a
/// particle, since a swap across a chunk boundary writes into two separate arrays.
#[test]
fn chunked_storage_conserves_across_boundaries() {
    let table = common::table();
    // Wide enough to straddle several chunks, so boundary swaps happen constantly.
    let mut world = scene::sandbox(sim_core::chunk::CHUNK_CELLS * 2 + 40, 120, 0x4f2a11, &table);

    let before: Vec<(u8, usize)> = table
        .iter()
        .map(|element| (element.id, world.count_of(element.id)))
        .collect();
    for (id, count) in &before {
        assert!(*count > 0, "element {id} absent from the scene");
    }

    world.step_many(3_000);

    for (id, expected) in before {
        assert_eq!(world.count_of(id), expected, "element {id} census changed");
    }
}
