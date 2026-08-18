//! Spec 3.1: same inputs, byte-identical output, every machine, every run.
//!
//! This is the milestone's reason for existing. Determinism gates cross-device sync,
//! replays, and all of the anti-cheat design (8.2, 8.3), and spec 10 is explicit that
//! nothing should be built on top of the sim until it is proven.

mod common;

use sim_core::scene;

/// The tick count spec 10 asks for.
const TICKS: u64 = 10_000;

/// These tests are about the *rules*, so they run on the flat reference world: it
/// sweeps only the cells the scene occupies, where the chunked world sweeps whole
/// chunks and takes several times as long for an identical result. That the two agree
/// is established separately, and thoroughly, in `chunk_equivalence.rs`.
fn hash_after(seed: u64, width: u32, height: u32, ticks: u64) -> u64 {
    let table = common::table();
    let mut world = scene::sandbox_flat(width, height, seed, &table);
    world.step_many(ticks);
    world.hash()
}

/// The flat reference world, fingerprinted the way Milestone 1 pinned it.
fn flat_structural_hash_after(seed: u64, width: u32, height: u32, ticks: u64) -> u64 {
    let table = common::table();
    let mut world = scene::sandbox_flat(width, height, seed, &table);
    world.step_many(ticks);
    world.structural_hash()
}

#[test]
fn identical_across_runs_in_one_process() {
    // Several seeds rather than one: spec 10 asks for a test that passes *reliably*,
    // and a single lucky seed does not establish that.
    for seed in [1, 0x4f2a11, u64::MAX, 0] {
        let first = hash_after(seed, 64, 48, TICKS);
        let second = hash_after(seed, 64, 48, TICKS);
        assert_eq!(
            first, second,
            "seed {seed:#x} produced {first:016x} then {second:016x}"
        );
    }
}

#[test]
fn identical_across_threads() {
    // Catches anything that leaks in from the environment rather than the seed —
    // thread-local state, address-dependent behaviour, lazily initialised randomness.
    let seed = 0xa5a5_1234;
    let expected = hash_after(seed, 96, 72, TICKS);

    let handle = std::thread::spawn(move || hash_after(seed, 96, 72, TICKS));
    let from_thread = handle.join().expect("worker thread should not panic");

    assert_eq!(expected, from_thread, "hash differed on another thread");
}

#[test]
fn identical_across_grid_sizes_and_seeds() {
    for (width, height) in [(32, 24), (61, 47), (128, 40)] {
        for seed in [7, 0xdead_beef] {
            let first = hash_after(seed, width, height, 2_000);
            let second = hash_after(seed, width, height, 2_000);
            assert_eq!(first, second, "seed {seed:#x} at {width}x{height}");
        }
    }
}

/// Stepping is a pure function of the state, so reaching tick N in one go must equal
/// reaching it in two halves. If this fails, something is accumulating state between
/// ticks that the hash cannot see.
#[test]
fn stepping_is_resumable() {
    let table = common::table();
    let seed = 0x51ce;

    let mut straight = scene::sandbox_flat(80, 60, seed, &table);
    straight.step_many(4_000);

    let mut halted = scene::sandbox_flat(80, 60, seed, &table);
    halted.step_many(1_500);
    halted.step_many(2_500);

    assert_eq!(straight.hash(), halted.hash());
    assert_eq!(straight.tick(), halted.tick());
}

/// A pinned fingerprint, so a change to the rules has to be a decision rather than an
/// accident. If this fails and the change was intended, re-pin it in the same commit
/// that changes the rules — never on its own.
#[test]
fn golden_hash_is_unchanged() {
    const GOLDEN_SEED: u64 = 0x4f2a11;
    // Pinned in Milestone 1, against the flat reference world and its structural
    // fingerprint. Deliberately unchanged by the move to chunked storage: if chunking
    // is transparent, this number must not move.
    const GOLDEN: u64 = 0xb87b_be3a_234b_320d;

    let actual = flat_structural_hash_after(GOLDEN_SEED, 128, 96, TICKS);
    assert_eq!(
        actual, GOLDEN,
        "world hash changed: expected {GOLDEN:016x}, got {actual:016x}"
    );
}

/// The chunked world carries the same guarantee. One case rather than a sweep of them:
/// it costs several times as much per tick, and `chunk_equivalence.rs` already pins it
/// to the flat world cell for cell.
#[test]
fn the_chunked_world_is_deterministic_too() {
    let table = common::table();

    let run = |seed: u64| {
        let mut world = scene::sandbox(96, 72, seed, &table);
        world.step_many(TICKS);
        world.hash()
    };

    let seed = 0x4f2a11;
    assert_eq!(run(seed), run(seed));

    // And across a process's threads, where anything ambient would show up.
    let expected = run(seed);
    let from_thread = std::thread::spawn(move || {
        let table = common::table();
        let mut world = scene::sandbox(96, 72, seed, &table);
        world.step_many(TICKS);
        world.hash()
    })
    .join()
    .expect("worker thread should not panic");
    assert_eq!(expected, from_thread);
}
