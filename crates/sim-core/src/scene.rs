//! A deterministic starting world, shared by the tests and the verification harness.
//!
//! Not game content. Milestone 1 has no spawners, so everything the world will ever
//! contain has to be placed at tick zero — which is also what makes the conservation
//! test meaningful: a closed box with a known census.
//!
//! One placement routine feeds both the flat and the chunked builder. If they laid out
//! their worlds separately, the equivalence test between them could pass or fail for
//! setup reasons rather than for anything to do with chunking.

use crate::elements::{ElementId, ElementTable, EMPTY};
use crate::field::CellField;
use crate::rng;
use crate::rules::Rules;
use crate::world::{FlatWorld, World};

/// Places the sandbox: a closed box with some structure in it and a charge of sand and
/// water, hashed from the seed rather than drawn from a stream.
fn build(width: u32, height: u32, seed: u64, table: &ElementTable, field: &mut dyn CellField) {
    let structure = table
        .id_of("structure")
        .expect("scene requires a `structure` element");
    let sand = table
        .id_of("sand")
        .expect("scene requires a `sand` element");
    let water = table
        .id_of("water")
        .expect("scene requires a `water` element");

    let right = width as i32 - 1;
    let bottom = height as i32 - 1;

    let fill = |field: &mut dyn CellField, x0: i32, y0: i32, x1: i32, y1: i32, id: ElementId| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                field.set(x, y, id);
            }
        }
    };

    // A one-cell frame. Nothing enters or leaves — which on an infinite canvas is the
    // only reason the chunked world stays bounded too.
    fill(field, 0, 0, right, 0, structure);
    fill(field, 0, bottom, right, bottom, structure);
    fill(field, 0, 0, 0, bottom, structure);
    fill(field, right, 0, right, bottom, structure);

    // Two ledges and a divider, so material has to find its way around something rather
    // than settling into one flat layer.
    let third = right / 3;
    let two_thirds = 2 * right / 3;
    fill(field, 2, bottom / 2, third, bottom / 2, structure);
    fill(field, two_thirds, bottom / 3, right - 2, bottom / 3, structure);
    fill(field, third + 4, bottom / 2, third + 4, bottom - 1, structure);

    // A charge of material across the top third, mixed by position hash.
    let fill_bottom = bottom / 3;
    for y in 1..fill_bottom {
        for x in 1..right {
            if field.get(x, y) != Some(EMPTY) {
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
            field.set(x, y, id);
        }
    }
}

/// The sandbox on chunked storage.
pub fn sandbox(width: u32, height: u32, seed: u64, rules: &Rules) -> World {
    let mut world = World::new(seed, rules.clone());
    build(width, height, seed, &rules.elements, world.field_mut());
    world
}

/// The same sandbox on the flat reference storage.
pub fn sandbox_flat(width: u32, height: u32, seed: u64, rules: &Rules) -> FlatWorld {
    let mut world = FlatWorld::new(width, height, seed, rules.clone());
    build(width, height, seed, &rules.elements, world.field_mut());
    world
}

/// An empty arena for the player to build in: walls on all four sides, nothing inside.
///
/// The world is infinite (spec 2.1) and nothing yet holds it up (spec 3.6), so material
/// with no floor beneath it falls forever, allocating chunks as it goes. Enclosing the
/// playable area sidesteps that until terrain answers it properly.
pub fn arena(width: u32, height: u32, seed: u64, rules: &Rules) -> World {
    let structure = rules
        .elements
        .id_of("structure")
        .expect("arena requires a `structure` element");
    let mut world = World::new(seed, rules.clone());
    let field = world.field_mut();

    let right = width as i32 - 1;
    let bottom = height as i32 - 1;
    for x in 0..=right {
        field.set(x, 0, structure);
        field.set(x, bottom, structure);
    }
    for y in 0..=bottom {
        field.set(0, y, structure);
        field.set(right, y, structure);
    }

    world
}
