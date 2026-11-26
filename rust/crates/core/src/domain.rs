//! The contract between the kernel and a simulation domain.
//!
//! A domain owns its state and defines its commands, events and queries. The
//! kernel ([`Simulation`](crate::sim::Simulation)) owns time, ordering,
//! randomness, persistence and hashing.
//!
//! Command contract (docs/SIMULATION.md): commands are authorised and
//! validated by [`Domain::validate`] against the current state without side
//! effects, then applied by [`Domain::apply`], which must not fail. A rejected
//! command therefore never changes state, and an accepted command is applied
//! completely within one tick (atomicity).

use std::fmt;

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use crate::id::ActorId;
use crate::rng::DetRng;
use crate::time::{SimDate, Tick};

/// Fixed order of simulation phases within one tick (docs/SIMULATION.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Due rules and scheduled events.
    DueRulesAndEvents,
    /// Demography and labour supply.
    Demography,
    /// Production.
    Production,
    /// Trade and stocks.
    TradeAndStocks,
    /// Prices and exchange rates.
    PricesAndFx,
    /// Bookings and budgets.
    AccountingAndBudget,
    /// Supply and conflicts.
    SupplyAndConflicts,
    /// Political mood and AI planning (AI plans for the next tick).
    PoliticsAndAi,
    /// Statistics and snapshots.
    StatisticsAndSnapshot,
}

impl Phase {
    /// All phases in execution order.
    pub const ORDER: [Phase; 9] = [
        Phase::DueRulesAndEvents,
        Phase::Demography,
        Phase::Production,
        Phase::TradeAndStocks,
        Phase::PricesAndFx,
        Phase::AccountingAndBudget,
        Phase::SupplyAndConflicts,
        Phase::PoliticsAndAi,
        Phase::StatisticsAndSnapshot,
    ];
}

/// Calendar boundaries reached by the current tick; domains use them for
/// weekly, monthly and yearly cadences instead of counting ticks themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cadence {
    /// Monday (ISO week start).
    pub week_start: bool,
    /// First day of a month.
    pub month_start: bool,
    /// 1 January.
    pub year_start: bool,
}

impl Cadence {
    /// Cadence flags of `date`.
    pub fn of(date: SimDate) -> Self {
        Self {
            week_start: date.iso_weekday() == 1,
            month_start: date.day() == 1,
            year_start: date.day() == 1 && date.month() == 1,
        }
    }
}

/// Per-tick context handed to domain code.
#[derive(Debug)]
pub struct TickContext<'a> {
    /// Tick being processed.
    pub tick: Tick,
    /// Calendar date of that tick.
    pub date: SimDate,
    /// Calendar boundaries of that date.
    pub cadence: Cadence,
    /// The only permitted source of randomness.
    pub rng: &'a mut DetRng,
}

/// Why a command was rejected. Codes are stable and exposed over the FFI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// The issuing actor may not perform this command.
    Unauthorized,
    /// A referenced entity does not exist.
    UnknownTarget(String),
    /// An argument is out of range or malformed.
    InvalidArgument(String),
    /// The command conflicts with the current state.
    Conflict(String),
}

impl CommandError {
    /// Stable numeric code (1..=4).
    pub fn code(&self) -> u32 {
        match self {
            Self::Unauthorized => 1,
            Self::UnknownTarget(_) => 2,
            Self::InvalidArgument(_) => 3,
            Self::Conflict(_) => 4,
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized => f.write_str("unauthorized"),
            Self::UnknownTarget(what) => write!(f, "unknown target: {what}"),
            Self::InvalidArgument(what) => write!(f, "invalid argument: {what}"),
            Self::Conflict(what) => write!(f, "conflict: {what}"),
        }
    }
}

impl std::error::Error for CommandError {}

/// A command with its issuer and scheduling key.
///
/// Commands due in the same tick execute in ascending
/// `(execute_at, actor, sequence)` order, independent of submission order or
/// thread timing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEnvelope<C> {
    /// Issuer.
    pub actor: ActorId,
    /// Per-actor sequence number; must strictly increase per actor.
    pub sequence: u64,
    /// Tick in which the command executes.
    pub execute_at: Tick,
    /// Domain payload.
    pub command: C,
}

impl<C: Encode> Encode for CommandEnvelope<C> {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.actor);
        enc.put(&self.sequence);
        enc.put(&self.execute_at);
        enc.put(&self.command);
    }
}

impl<C: Decode> Decode for CommandEnvelope<C> {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            actor: dec.get()?,
            sequence: dec.get()?,
            execute_at: dec.get()?,
            command: dec.get()?,
        })
    }
}

/// A simulation domain plugged into the kernel.
pub trait Domain {
    /// Stable schema identifier stored in saves and replays, e.g. `"kod.world"`.
    const SCHEMA_ID: &'static str;
    /// Version of the state encoding; bump on every incompatible change.
    const SCHEMA_VERSION: u32;

    /// Authoritative state. Must only contain ordered collections.
    type State: Encode + Decode + Clone + fmt::Debug;
    /// Commands (player, AI or system intents).
    type Command: Encode + Decode + Clone + fmt::Debug;
    /// Facts emitted while applying commands and phases.
    type Event: Encode + Decode + Clone + fmt::Debug + PartialEq;
    /// Read-only queries.
    type Query;
    /// Query answers.
    type Answer;

    /// Authorises and validates `command` for `actor`. Must be side-effect free.
    fn validate(
        state: &Self::State,
        actor: ActorId,
        command: &Self::Command,
    ) -> Result<(), CommandError>;

    /// Applies a command that passed [`validate`](Self::validate) in the same
    /// tick and state. Must not fail and must apply the command completely.
    fn apply(
        state: &mut Self::State,
        ctx: &mut TickContext<'_>,
        actor: ActorId,
        command: &Self::Command,
        events: &mut Vec<Self::Event>,
    );

    /// Runs one phase of the tick. Default: nothing.
    fn run_phase(
        state: &mut Self::State,
        phase: Phase,
        ctx: &mut TickContext<'_>,
        events: &mut Vec<Self::Event>,
    ) {
        let _ = (state, phase, ctx, events);
    }

    /// Answers a read-only query.
    fn query(state: &Self::State, query: &Self::Query) -> Self::Answer;

    /// Upgrades a state payload written with an older `from_version`.
    /// Default: unsupported.
    fn migrate(from_version: u32, payload: &[u8]) -> Result<Self::State, DecodeError> {
        let _ = (from_version, payload);
        Err(DecodeError::Invalid("no migration path for schema version"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadence_flags() {
        let c = Cadence::of(SimDate::new(2024, 1, 1).unwrap()); // Monday
        assert_eq!(
            c,
            Cadence {
                week_start: true,
                month_start: true,
                year_start: true
            }
        );
        let c = Cadence::of(SimDate::new(2024, 1, 2).unwrap());
        assert_eq!(c, Cadence::default());
    }

    #[test]
    fn phase_order_is_total_and_unique() {
        let mut sorted = Phase::ORDER;
        sorted.sort();
        assert_eq!(sorted, Phase::ORDER);
    }
}
