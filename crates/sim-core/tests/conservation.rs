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

    for seed in [3, 0x4f2a11, 0xfeed_face] {
        let mut world = scene::sandbox(96, 72, seed, &table);

        let before: Vec<(u8, usize)> = table
            .iter()
            .map(|element| (element.id, world.count_of(element.id)))
            .collect();
        let empty_before = world.count_of(EMPTY);

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
        assert_eq!(
            world.count_of(EMPTY),
            empty_before,
            "seed {seed:#x}: void count changed"
        );
    }
}

/// Movement is always a swap, so the total cell count is invariant too — a weaker
/// claim than the census, but it fails differently if indexing goes wrong.
#[test]
fn cell_count_is_invariant() {
    let table = common::table();
    let mut world = scene::sandbox(64, 48, 11, &table);
    let cells = world.cells().len();

    world.step_many(1_000);

    assert_eq!(world.cells().len(), cells);
    assert_eq!(cells, 64 * 48);
}

/// Solids do not move. Sand may pile against a wall, but no wall may end up somewhere
/// it was not placed.
#[test]
fn solids_never_move() {
    let table = common::table();
    let wall = table.id_of("wall").expect("wall element");
    let mut world = scene::sandbox(80, 60, 0x1234, &table);

    let placed: Vec<usize> = world
        .cells()
        .iter()
        .enumerate()
        .filter(|(_, &cell)| cell == wall)
        .map(|(index, _)| index)
        .collect();
    assert!(!placed.is_empty());

    world.step_many(2_000);

    let after: Vec<usize> = world
        .cells()
        .iter()
        .enumerate()
        .filter(|(_, &cell)| cell == wall)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(placed, after, "a solid moved");
}
