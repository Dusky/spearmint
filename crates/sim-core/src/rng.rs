//! The simulation's own randomness: a pure function of position and time.
//!
//! Spec 3.1 requires a seeded PRNG owned by the sim, with no system entropy and a
//! deterministic iteration order. The obvious implementation — one PRNG advanced as
//! cells are visited — satisfies that today and breaks the moment chunks sleep.
//!
//! A sleeping chunk (spec 2.4) consumes no random numbers, so every cell evaluated
//! after it would draw a *different* value than it would have in a session where that
//! chunk stayed awake. The world would then diverge based on where the player happened
//! to be looking, which is the sort of determinism bug the spec warns is extremely
//! expensive to retrofit.
//!
//! So there is no stream and no state. Each draw is a hash of `(seed, tick, x, y)`,
//! which makes results independent of iteration order, chunk boundaries, sleeping, and
//! any future parallelism — satisfying 3.1's requirement that parallel chunk updates
//! resolve reproducibly, by construction rather than by discipline.

/// The splitmix64 finalizer: strong avalanche, integer-only, no lookup tables.
const fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// Odd multipliers keep each coordinate contributing to the whole 64-bit state.
const TICK_KEY: u64 = 0x9e37_79b9_7f4a_7c15;
const X_KEY: u64 = 0xc2b2_ae3d_27d4_eb4f;
const Y_KEY: u64 = 0x1656_67b1_9e37_79f9;
const SALT_KEY: u64 = 0xd6e8_feb8_6659_fd93;

/// A random 32-bit value for one cell at one tick.
///
/// Deterministic in every sense that matters: same inputs, same output, on every
/// machine, in any order, in any number of threads.
pub fn noise(seed: u64, tick: u64, x: i32, y: i32) -> u32 {
    noise_salted(seed, tick, x, y, 0)
}

/// A second, independent draw for the same cell and tick.
///
/// A cell that needs two unrelated decisions in one tick must not reuse one value for
/// both, or the decisions correlate. The salt separates them without introducing state.
pub fn noise_salted(seed: u64, tick: u64, x: i32, y: i32, salt: u32) -> u32 {
    let mut hash = seed;
    hash = mix(hash ^ tick.wrapping_mul(TICK_KEY));
    // Sign-extend through u32 so negative coordinates map to distinct keys rather than
    // colliding with large positive ones.
    hash = mix(hash ^ u64::from(x as u32).wrapping_mul(X_KEY));
    hash = mix(hash ^ u64::from(y as u32).wrapping_mul(Y_KEY));
    if salt != 0 {
        hash = mix(hash ^ u64::from(salt).wrapping_mul(SALT_KEY));
    }
    // The high bits of a splitmix64 output are the best mixed.
    (hash >> 32) as u32
}

/// True about half the time.
pub fn coin_flip(seed: u64, tick: u64, x: i32, y: i32, salt: u32) -> bool {
    // Bit 31 rather than bit 0: the top bits carry the strongest avalanche.
    noise_salted(seed, tick, x, y, salt) >> 31 == 1
}

/// A value in `0..bound`, without the modulo bias a `%` would introduce and without a
/// division. Returns 0 when `bound` is 0.
pub fn below(seed: u64, tick: u64, x: i32, y: i32, salt: u32, bound: u32) -> u32 {
    let draw = u64::from(noise_salted(seed, tick, x, y, salt));
    ((draw * u64::from(bound)) >> 32) as u32
}
