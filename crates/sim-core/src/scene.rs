//! A deterministic starting world, shared by the tests and the verification harness.
//!
//! Not game content. Milestone 1 has no spawners, so everything the world will ever
//! contain has to be placed at tick zero — which is also what makes the conservation
//! test meaningful: a closed box with a known census.
//!
//! Both the in-process tests and the `sim-hash` binary build the world from here. If
//! they built it separately the cross-process comparison would prove nothing.

use crate::elements::ElementTable;
use crate::rng;
use crate::world::World;

/// Builds a closed box with some structure in it and a charge of sand and water.
///
/// Placement is hashed from the seed rather than drawn from a stream, so the same seed
/// gives the same world on any machine, in any order.
pub fn sandbox(width: u32, height: u32, seed: u64, table: &ElementTable) -> World {
    let wall = table
        .id_of("wall")
        .expect("scene requires a `wall` element");
    let sand = table
        .id_of("sand")
        .expect("scene requires a `sand` element");
    let water = table
        .id_of("water")
        .expect("scene requires a `water` element");

    let mut world = World::new(width, height, seed, table.clone());
    let grid = world.grid_mut();
    let right = width as i32 - 1;
    let bottom = height as i32 - 1;

    // A one-cell frame. Nothing enters or leaves.
    grid.fill_rect(0, 0, right, 0, wall);
    grid.fill_rect(0, bottom, right, bottom, wall);
    grid.fill_rect(0, 0, 0, bottom, wall);
    grid.fill_rect(right, 0, right, bottom, wall);

    // Two ledges and a divider, so material has to find its way around something
    // rather than settling into one flat layer.
    let third = right / 3;
    let two_thirds = 2 * right / 3;
    grid.fill_rect(2, bottom / 2, third, bottom / 2, wall);
    grid.fill_rect(two_thirds, bottom / 3, right - 2, bottom / 3, wall);
    grid.fill_rect(third + 4, bottom / 2, third + 4, bottom - 1, wall);

    // A charge of material across the top third, mixed by position hash.
    let fill_bottom = bottom / 3;
    for y in 1..fill_bottom {
        for x in 1..right {
            if grid.get(x, y) != Some(crate::elements::EMPTY) {
                continue;
            }
            // Coarse blocks rather than per-cell noise, so the two materials arrive in
            // pockets that have to interact instead of pre-mixed static.
            let draw = rng::below(seed, 0, x >> 3, y >> 3, 0, 100);
            let id = if draw < 45 {
                sand
            } else if draw < 80 {
                water
            } else {
                continue;
            };
            grid.set(x, y, id);
        }
    }

    world
}
