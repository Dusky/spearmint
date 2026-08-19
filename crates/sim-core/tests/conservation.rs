//! Spec 1.1: particles are never created, destroyed, or summarised outside the physics
//! rules themselves. The spec calls this constraint load-bearing, so it gets a test
//! rather than a convention.
//!
//! Milestone 1 has no spawners and no reactions, so in a closed box every element's
//! census must be exactly what it started as — forever. This is what catches a
//! displacement rule that drops a particle, long before anyone notices sand quietly
//! evaporating out of a factory.

mod common;

use sim_core::{scene, ElementTable, EMPTY};

/// The scene places walls, sand and water; anything else in the roster is legitimately
/// absent and is still covered by the conservation assertions.
///
/// Takes the census rather than a world, so it serves both the flat and chunked ones.
fn assert_present(elements: &ElementTable, census: &[(u8, usize)], seed: u64) {
    for name in ["structure", "sand", "water"] {
        let id = elements.id_of(name).expect("element should be defined");
        let count = census
            .iter()
            .find(|(element, _)| *element == id)
            .map(|(_, count)| *count)
            .unwrap_or(0);
        assert!(count > 0, "seed {seed:#x}: {name} absent from the scene");
    }
}

#[test]
fn nothing_is_created_or_destroyed() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;

    // Several seeds on the flat world, which simulates the same rules for a fraction
    // of the cost; one chunked run below covers the storage.
    for seed in [3, 0x4f2a11, 0xfeed_face] {
        let mut world = scene::sandbox_flat(96, 72, seed, &rules);

        let before: Vec<(u8, usize)> = table
            .iter()
            .map(|element| (element.id, world.count_of(element.id)))
            .collect();

        // The materials whose movement this is about must actually be present, or the
        // test would pass vacuously. Elements the scene never places — reaction
        // products, for one — are still conserved below, at a count of zero.
        assert_present(&rules.elements, &before, seed);

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
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let mut world = scene::sandbox_flat(96, 72, 3, &rules);

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
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let structure = table.id_of("structure").expect("structure element");
    let mut world = scene::sandbox(80, 60, 0x1234, &rules);

    let placed: Vec<(i32, i32)> = (0..60)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .filter(|&(x, y)| world.get(x, y) == structure)
        .collect();
    assert!(!placed.is_empty());

    world.step_many(2_000);

    let after: Vec<(i32, i32)> = (0..60)
        .flat_map(|y| (0..80).map(move |x| (x, y)))
        .filter(|&(x, y)| world.get(x, y) == structure)
        .collect();
    assert_eq!(placed, after, "a solid moved");
}

/// Chunked storage must conserve as well — and it is the one that can genuinely lose a
/// particle, since a swap across a chunk boundary writes into two separate arrays.
#[test]
fn chunked_storage_conserves_across_boundaries() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    // Wide enough to straddle several chunks, so boundary swaps happen constantly.
    let mut world = scene::sandbox(sim_core::chunk::CHUNK_CELLS * 2 + 40, 120, 0x4f2a11, &rules);

    let before: Vec<(u8, usize)> = table
        .iter()
        .map(|element| (element.id, world.count_of(element.id)))
        .collect();
    assert_present(&rules.elements, &before, 0x4f2a11);

    world.step_many(3_000);

    for (id, expected) in before {
        assert_eq!(world.count_of(id), expected, "element {id} census changed");
    }
}
