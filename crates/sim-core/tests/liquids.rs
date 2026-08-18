//! Liquid behaviour, and the one artifact that made water unusable.
//!
//! Spec 3.5 is still open — how liquids find their level is a design question. What is
//! settled is narrower: a body of liquid must actually be a body of liquid.

mod common;

use sim_core::scene;

fn tank(width: u32, height: u32, spawn_tile_x: i32) -> sim_core::World {
    let rules = common::rules_without_reactions();
    let water = rules.elements.id_of("water").expect("water");
    let mut world = scene::arena(width, height, 1, &rules);
    world.place(common::emitter(spawn_tile_x, 0, water));
    world
}

/// The regression this file exists for.
///
/// Submerged liquid used to keep flowing sideways, and every lateral move left a void
/// behind. Gravity refilled voids at the same rate they were created, so a body of
/// water settled at roughly three-quarters density — permanently, in a stable lattice
/// of holes. It read as a rendering fault rather than as a fluid.
#[test]
fn a_body_of_liquid_packs_solid() {
    let rules = common::rules_without_reactions();
    let water = rules.elements.id_of("water").expect("water");
    let mut world = tank(60, 60, 3);
    world.step_many(10_000);

    let mut filled = 0;
    let mut holes = 0;
    for y in 40..58 {
        for x in 2..57 {
            match world.get(x, y) {
                cell if cell == water => filled += 1,
                0 => holes += 1,
                _ => {}
            }
        }
    }

    assert!(filled > 0, "the tank never filled");
    assert_eq!(
        holes, 0,
        "{holes} holes in {filled} cells of submerged water"
    );
}

/// Restricting lateral flow to the surface must not stop water finding its level —
/// that is the whole point of it flowing at all. With multi-cell dispersion it should
/// level essentially exactly, not merely approximately.
#[test]
fn liquid_still_finds_its_level() {
    let rules = common::rules_without_reactions();
    let water = rules.elements.id_of("water").expect("water");
    let mut world = tank(80, 40, 0);
    world.step_many(6_000);

    let depth = |x: i32| (1..39).filter(|&y| world.get(x, y) == water).count();

    // Poured in at the far left; the far end must be within a few cells of the near end.
    let near = depth(8);
    let far = depth(74);
    assert!(
        near > 10,
        "the tank barely filled: {near} deep at the source"
    );
    assert!(
        near.abs_diff(far) <= 1,
        "water did not level out: {near} deep at the source, {far} at the far end"
    );
}
