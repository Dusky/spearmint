//! Milestone 2b's gate: sleeping must be *unobservable*.
//!
//! This is the milestone where determinism is genuinely at risk, because what gets
//! simulated stops being a function of the world alone. So it gets the same treatment
//! chunking got: a world where regions sleep must be identical to one where nothing
//! sleeps, tick for tick.
//!
//! The design that makes that possible is that a chunk sleeps only when **quiescent** —
//! nothing moved in it or in any neighbour last tick. Ticking a settled chunk produces
//! no change, so skipping it produces no difference. The viewport gate of spec 2.4 sits
//! on top and can only keep *more* chunks awake, so it spends CPU and cannot alter the
//! world.
//!
//! That is also why sleeping does not inherit eviction's problem. Eviction discards
//! state and is observable; sleeping preserves it exactly and is not.

mod common;

use sim_core::chunk::CHUNK_CELLS;
use sim_core::field::{Bounds, CellField};
use sim_core::scene;

fn run(width: u32, height: u32, seed: u64, ticks: u64, sleeping: bool) -> (u64, usize) {
    let rules = common::rules_without_reactions();
    let mut world = scene::sandbox(width, height, seed, &rules);
    world.set_sleeping(sleeping);
    world.step_many(ticks);
    (world.hash(), world.awake_chunk_count())
}

#[test]
fn sleeping_does_not_change_the_world() {
    for seed in [3, 0x4f2a11, 0xdead_beef] {
        let (asleep, _) = run(96, 72, seed, 3_000, true);
        let (awake, _) = run(96, 72, seed, 3_000, false);
        assert_eq!(
            asleep, awake,
            "sleeping changed the world at seed {seed:#x}"
        );
    }
}

/// Across chunk seams, where a sleeping chunk has to be woken by a neighbour's activity
/// rather than by its own.
#[test]
fn sleeping_does_not_change_a_multi_chunk_world() {
    let width = CHUNK_CELLS * 2 + 40;
    let height = CHUNK_CELLS + 60;
    for seed in [7, 0xfeed_face] {
        let (asleep, _) = run(width, height, seed, 1_500, true);
        let (awake, _) = run(width, height, seed, 1_500, false);
        assert_eq!(asleep, awake, "seed {seed:#x}");
    }
}

/// Without this the tests above could pass by never sleeping at all.
///
/// Powders only: a sand heap reaches a fixed point, so its chunks go quiet. Liquids
/// currently do not — see `liquid_worlds_never_settle` below, which is why this scene
/// is deliberately dry.
#[test]
fn chunks_actually_sleep_once_the_world_settles() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let structure = table.id_of("structure").expect("structure");
    let sand = table.id_of("sand").expect("sand");

    let mut world = sim_core::World::new(9, rules.clone());
    world.set_sleeping(true);

    // Three chunks wide, two tall. Enclosed on all sides: on an infinite canvas a bare
    // floor is not enough, because material slides off the ends and falls forever,
    // allocating chunks as it goes and never coming to rest.
    let width = CHUNK_CELLS as i32 * 3;
    let height = CHUNK_CELLS as i32 * 2;
    for x in 0..width {
        world.field_mut().set(x, 0, structure);
        world.field_mut().set(x, height - 1, structure);
    }
    for y in 0..height {
        world.field_mut().set(0, y, structure);
        world.field_mut().set(width - 1, y, structure);
    }
    // Sand in the left-hand chunk only, so the right-hand ones have nothing to do.
    for x in 4..(CHUNK_CELLS as i32 - 4) {
        for y in 10..40 {
            world.field_mut().set(x, y, sand);
        }
    }

    let total = world.field().chunk_count();
    assert!(total >= 6, "expected a multi-chunk world, got {total}");

    world.step_many(20);
    let early = world.awake_chunk_count();

    // Long enough for the sand to come to rest.
    world.step_many(4_000);
    let settled = world.awake_chunk_count();

    assert!(
        settled < early,
        "nothing went to sleep: {early} chunks awake early, {settled} after settling"
    );
    assert!(
        settled < total,
        "every chunk stayed awake ({settled} of {total})"
    );
}

/// A settled world must stop changing entirely — the strongest form of the claim, and
/// what sleeping ultimately rests on.
#[test]
fn a_settled_powder_world_reaches_a_fixed_point() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let structure = table.id_of("structure").expect("structure");
    let sand = table.id_of("sand").expect("sand");

    let mut world = sim_core::World::new(3, rules.clone());
    world.set_sleeping(true);
    // A closed box. An open floor would let the heap spread off its ends and fall
    // forever down an infinite canvas.
    for x in 0..80 {
        world.field_mut().set(x, 0, structure);
        world.field_mut().set(x, 60, structure);
    }
    for y in 0..=60 {
        world.field_mut().set(0, y, structure);
        world.field_mut().set(79, y, structure);
    }
    for x in 20..40 {
        for y in 10..30 {
            world.field_mut().set(x, y, sand);
        }
    }

    world.step_many(3_000);
    let settled = world.content_hash();
    world.step_many(500);

    // Contents, not `hash` — that one folds in the tick and so differs every tick even
    // when nothing has moved.
    assert_eq!(
        world.content_hash(),
        settled,
        "a powder world never came to rest"
    );
}

/// Documents a real limitation rather than hiding it.
///
/// Liquids never reach a fixed point. A void trapped inside a body of water random-walks
/// forever: water never rises, so the void cannot escape upward, and a lateral move
/// costs nothing, so water shuffles around it indefinitely. Sleeping is correct in the
/// presence of this, but it cannot engage — any chunk holding water stays dirty, and by
/// the neighbour rule it keeps the chunks around it awake too.
///
/// Fixing it means deciding how liquids find their level, which is a design question
/// with gameplay consequences (spec 3.3 counts pressure and residence time among the
/// things yield depends on). Flipping this test to `assert_eq` is the check that a
/// future liquid model actually terminates.
#[test]
fn liquid_worlds_never_settle() {
    let rules = common::rules_without_reactions();
    let mut world = scene::sandbox(60, 40, 5, &rules);
    world.set_sleeping(true);

    world.step_many(16_000);
    let long_settled = world.content_hash();
    world.step_many(1);

    assert_ne!(
        world.content_hash(),
        long_settled,
        "liquids now reach a fixed point — good news; update this test and check \
         whether chunks holding water can sleep"
    );
}

/// A sleeping chunk must wake when material arrives from a neighbour, or particles
/// would pile up against an invisible wall at the seam.
#[test]
fn a_neighbour_wakes_a_sleeping_chunk() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let sand = table.id_of("sand").expect("sand");
    let seam = CHUNK_CELLS as i32;

    let mut world = sim_core::World::new(1, rules.clone());
    world.set_sleeping(true);

    // A floor across two chunks, with sand sitting above it on the left one only.
    for x in 0..(seam + 20) {
        world
            .field_mut()
            .set(x, seam - 1, table.id_of("structure").unwrap());
    }
    world.field_mut().set(seam - 1, seam - 4, sand);

    // Let the right-hand chunk settle into sleep, then drop sand toward the seam.
    world.step_many(200);

    // The sand should have fallen and come to rest on the floor, not stopped in mid-air
    // at the chunk edge.
    let resting = (0..seam).any(|y| world.get(seam - 1, y) == sand);
    assert!(resting, "the sand vanished");
    assert_eq!(
        world.get(seam - 1, seam - 2),
        sand,
        "sand did not settle onto the floor"
    );
}

/// The viewport gate keeps chunks awake; it must never put one to sleep.
#[test]
fn the_viewport_only_ever_keeps_chunks_awake() {
    let width = CHUNK_CELLS * 3;
    let height = CHUNK_CELLS * 2;
    let rules = common::rules_without_reactions();

    let settle = |viewport: Option<Bounds>| {
        let mut world = scene::sandbox(width, height, 11, &rules);
        world.set_sleeping(true);
        world.set_viewport(viewport, 1);
        world.step_many(6_000);
        (world.hash(), world.awake_chunk_count())
    };

    let (no_view_hash, no_view_awake) = settle(None);
    let (viewed_hash, viewed_awake) = settle(Some(Bounds {
        min_x: 0,
        min_y: 0,
        max_x: CHUNK_CELLS as i32,
        max_y: CHUNK_CELLS as i32,
    }));

    assert_eq!(no_view_hash, viewed_hash, "the viewport changed the world");
    assert!(
        viewed_awake >= no_view_awake,
        "the viewport put chunks to sleep: {viewed_awake} awake with a viewport, \
         {no_view_awake} without"
    );
}

/// Sleeping must not cost conservation either — a chunk that stops ticking still holds
/// its particles (spec 2.4: freezing, not abstraction).
#[test]
fn sleeping_conserves_everything() {
    let rules = common::rules_without_reactions();
    let table = &rules.elements;
    let mut world = scene::sandbox(CHUNK_CELLS * 2, CHUNK_CELLS, 0x4f2a11, &rules);
    world.set_sleeping(true);

    let before: Vec<(u8, usize)> = table
        .iter()
        .map(|element| (element.id, world.count_of(element.id)))
        .collect();

    world.step_many(6_000);

    for (id, expected) in before {
        assert_eq!(world.count_of(id), expected, "element {id} census changed");
    }
}
