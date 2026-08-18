//! The Particle Factory simulation core.
//!
//! Headless and self-contained: no rendering, no platform APIs, no I/O, no
//! dependencies. Spec 8.3 makes that isolation a hard architectural requirement, since
//! the same crate compiles to wasm for the client and to native for server-side replay
//! verification, and the two must agree byte for byte.
//!
//! Determinism (spec 3.1) is structural here, not a convention to be remembered:
//!
//! - **No floats.** Fixed-point only, enforced by lint and by a test that reads the
//!   source. Float rounding differs across machines; byte-identical output cannot.
//! - **No random state.** Randomness is a pure hash of position and tick, so results do
//!   not depend on visit order, chunk boundaries, sleeping, or threading.
//! - **No hash-map iteration.** Element data is stored in ordered vectors.
//! - **No system entropy, no clock.** Everything comes from the seed.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_debug_implementations)]

pub mod chunk;
pub mod elements;
pub mod field;
pub mod fixed;
pub mod grid;
pub mod hash;
pub mod json;
pub mod reactions;
pub mod rng;
pub mod rules;
pub mod scene;
pub mod spawners;
pub mod step;
pub mod world;

pub use chunk::ChunkMap;
pub use elements::{DataError, Element, ElementId, ElementTable, State, EMPTY};
pub use field::{Bounds, CellField};
pub use fixed::Fixed;
pub use grid::Grid;
pub use reactions::{Reaction, ReactionTable};
pub use rules::Rules;
pub use spawners::Spawner;
pub use world::{FlatWorld, World};
