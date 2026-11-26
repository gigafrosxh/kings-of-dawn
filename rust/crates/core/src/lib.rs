//! Deterministic simulation kernel of Kings of Dawn.
//!
//! This crate is the authoritative, engine-independent foundation every
//! simulation domain builds on (issue #8, T03). It runs headless and contains
//! no Unreal, rendering, wall-clock or floating-point dependencies.
//!
//! Building blocks:
//! - [`id`]: typed, stable, totally ordered identifiers.
//! - [`fixed`]: decimal fixed-point numbers with defined rounding.
//! - [`time`]: tick counter and astronomical proleptic Gregorian dates.
//! - [`rng`]: seedable, serialisable pseudo random generator.
//! - [`codec`]: canonical little-endian binary encoding (state hash + saves).
//! - [`hash`]: stable 128-bit state hash.
//! - [`domain`]: the contract a simulation domain implements (commands,
//!   queries, events, tick phases).
//! - [`sim`]: the tick scheduler that validates and applies commands
//!   atomically and in a stable order.
//! - [`save`]: versioned, checksummed save container.
//! - [`replay`]: command logs and state-hash verified replays.
//!
//! See `docs/decisions/ADR-0003-deterministic-simulation-core.md` for the rationale.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic, clippy::float_cmp)]
#![warn(missing_docs)]

pub mod codec;
pub mod domain;
pub mod fixed;
pub mod hash;
pub mod id;
pub mod replay;
pub mod rng;
pub mod save;
pub mod sim;
pub mod time;

#[cfg(feature = "fixtures")]
pub mod fixture;

pub use codec::{Decode, DecodeError, Decoder, Encode, Encoder};
pub use domain::{CommandEnvelope, CommandError, Domain, Phase, TickContext};
pub use fixed::{Fixed, FixedError};
pub use hash::StateHash;
pub use id::{ActorId, IdAllocator};
pub use replay::{Replay, ReplayError, ReplayLog};
pub use rng::DetRng;
pub use save::{SaveError, SaveFile};
pub use sim::{CommandOutcome, Simulation, SubmitError, TickReport};
pub use time::{SimDate, Tick};

/// Stable project identity for headless tooling and smoke checks.
pub const PROJECT_NAME: &str = "Kings of Dawn";

#[cfg(test)]
mod tests {
    use super::PROJECT_NAME;

    #[test]
    fn project_identity_is_available_headless() {
        assert_eq!(PROJECT_NAME, "Kings of Dawn");
    }
}
