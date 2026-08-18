//! FNV-1a 64, used to fingerprint a world state.
//!
//! Hand-rolled on purpose. `std::collections::hash_map::DefaultHasher` is seeded from
//! `RandomState`, which draws system entropy per process — it would produce a different
//! hash for identical worlds in two processes and fail the cross-process determinism
//! test for a reason that has nothing to do with the simulation.
//!
//! FNV-1a is not cryptographic and does not need to be. It needs to be stable across
//! machines, processes, and compiler versions, and to change when any byte changes.

const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy, Debug)]
pub struct Hasher(u64);

impl Default for Hasher {
    fn default() -> Self {
        Hasher::new()
    }
}

impl Hasher {
    pub const fn new() -> Self {
        Hasher(OFFSET_BASIS)
    }

    pub fn write_u8(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(PRIME);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u8(byte);
        }
    }

    /// Little-endian, explicitly, so the hash does not depend on host byte order.
    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Little-endian, explicitly, so the hash does not depend on host byte order.
    pub fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub const fn finish(self) -> u64 {
        self.0
    }
}
