//! Command logs and verified replays.
//!
//! A [`ReplayLog`] holds the initial save, every accepted command together
//! with the tick before which it was submitted, and state-hash checkpoints.
//! [`ReplayLog::verify`] re-runs the session headless and fails at the first
//! checkpoint whose hash diverges.

use std::fmt;

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use crate::domain::{CommandEnvelope, Domain};
use crate::hash::StateHash;
use crate::save::{SaveError, SaveFile};
use crate::sim::{Simulation, SubmitError, TickReport};
use crate::time::Tick;

/// Current replay log format version.
pub const REPLAY_FORMAT_VERSION: u32 = 2;

/// Replay failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// The embedded initial save could not be loaded.
    InitialSave(SaveError),
    /// A recorded command was refused by the scheduler during replay.
    Submit {
        /// Tick before which it was submitted.
        tick: Tick,
        /// Scheduler error.
        error: SubmitError,
    },
    /// State diverged from the recording.
    Divergence {
        /// Checkpoint tick (state after processing up to, excluding, this tick).
        tick: Tick,
        /// Recorded hash.
        expected: StateHash,
        /// Replayed hash.
        actual: StateHash,
    },
    /// Log is structurally invalid.
    Malformed(&'static str),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitialSave(err) => write!(f, "initial save: {err}"),
            Self::Submit { tick, error } => write!(f, "command at {tick} refused: {error}"),
            Self::Divergence {
                tick,
                expected,
                actual,
            } => {
                write!(f, "divergence at {tick}: expected {expected}, got {actual}")
            }
            Self::Malformed(what) => write!(f, "malformed replay: {what}"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Recorded session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayLog<C> {
    /// Save of the starting state.
    pub initial_save: Vec<u8>,
    /// `(submitted before tick, command)` in submission order.
    pub commands: Vec<(Tick, CommandEnvelope<C>)>,
    /// `(tick, hash)`, strictly ascending. A checkpoint at tick `t` is the
    /// state hash once every command submitted before tick `t` is processed
    /// has been queued, i.e. directly before stepping `t` (or at the end of
    /// the recording for `t == end_tick`). Format v2; v1 hashed before late
    /// submissions and could disagree with `verify`.
    pub checkpoints: Vec<(Tick, StateHash)>,
    /// `next_tick()` at the end of the recording.
    pub end_tick: Tick,
}

impl<C: Encode> Encode for ReplayLog<C> {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&REPLAY_FORMAT_VERSION);
        enc.put(&self.initial_save);
        enc.put(&self.commands);
        enc.put(&self.checkpoints);
        enc.put(&self.end_tick);
    }
}

impl<C: Decode> Decode for ReplayLog<C> {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let version: u32 = dec.get()?;
        if version != REPLAY_FORMAT_VERSION {
            return Err(DecodeError::Invalid("unsupported replay format version"));
        }
        Ok(Self {
            initial_save: dec.get()?,
            commands: dec.get()?,
            checkpoints: dec.get()?,
            end_tick: dec.get()?,
        })
    }
}

impl<C: Clone> ReplayLog<C> {
    /// Final recorded hash, if a checkpoint was taken at the end.
    pub fn final_hash(&self) -> Option<StateHash> {
        self.checkpoints
            .last()
            .filter(|(t, _)| *t == self.end_tick)
            .map(|(_, h)| *h)
    }

    /// Re-runs the log headless and compares every checkpoint.
    /// Returns the final simulation on success.
    pub fn verify<D: Domain<Command = C>>(&self) -> Result<Simulation<D>, ReplayError> {
        let mut sim = SaveFile::load::<D>(&self.initial_save).map_err(ReplayError::InitialSave)?;
        let start = sim.next_tick();
        if self.end_tick < start {
            return Err(ReplayError::Malformed("end before start"));
        }
        if self.commands.windows(2).any(|w| w[0].0 > w[1].0)
            || self.checkpoints.windows(2).any(|w| w[0].0 >= w[1].0)
        {
            return Err(ReplayError::Malformed("entries not in tick order"));
        }
        let mut commands = self.commands.iter().peekable();
        let mut checkpoints = self.checkpoints.iter().peekable();
        loop {
            let now = sim.next_tick();
            while let Some((tick, envelope)) = commands.next_if(|(t, _)| *t == now) {
                sim.submit(envelope.clone())
                    .map_err(|error| ReplayError::Submit { tick: *tick, error })?;
            }
            if let Some(&&(tick, expected)) = checkpoints.peek() {
                if tick < now {
                    return Err(ReplayError::Malformed("checkpoint outside recorded range"));
                }
                if tick == now {
                    let actual = sim.state_hash();
                    if actual != expected {
                        return Err(ReplayError::Divergence {
                            tick,
                            expected,
                            actual,
                        });
                    }
                    checkpoints.next();
                }
            }
            if now == self.end_tick {
                break;
            }
            sim.step();
        }
        if commands.next().is_some() || checkpoints.next().is_some() {
            return Err(ReplayError::Malformed("entries after end tick"));
        }
        Ok(sim)
    }
}

/// Records a session into a [`ReplayLog`].
#[derive(Debug)]
pub struct Replay<D: Domain> {
    sim: Simulation<D>,
    log: ReplayLog<D::Command>,
    checkpoint_every: u64,
    /// A checkpoint is due for the current tick (taken before stepping it).
    checkpoint_due: bool,
}

impl<D: Domain> Replay<D> {
    /// Starts recording; takes a checkpoint every `checkpoint_every` ticks
    /// (0 = only at start and end).
    pub fn new(sim: Simulation<D>, checkpoint_every: u64) -> Self {
        let start = sim.next_tick();
        let log = ReplayLog {
            initial_save: SaveFile::write(&sim),
            commands: Vec::new(),
            checkpoints: Vec::new(),
            end_tick: start,
        };
        Self {
            sim,
            log,
            checkpoint_every,
            checkpoint_due: true,
        }
    }

    /// The recorded simulation.
    pub fn sim(&self) -> &Simulation<D> {
        &self.sim
    }

    /// Submits and records an accepted command.
    pub fn submit(&mut self, envelope: CommandEnvelope<D::Command>) -> Result<(), SubmitError> {
        self.sim.submit(envelope.clone())?;
        self.log.commands.push((self.sim.next_tick(), envelope));
        Ok(())
    }

    /// Steps the simulation and takes due checkpoints.
    pub fn step(&mut self) -> TickReport<D> {
        if self.checkpoint_due {
            self.log
                .checkpoints
                .push((self.sim.next_tick(), self.sim.state_hash()));
        }
        let report = self.sim.step();
        let now = self.sim.next_tick();
        self.checkpoint_due =
            self.checkpoint_every > 0 && now.0.is_multiple_of(self.checkpoint_every);
        report
    }

    /// Ends the recording with a final checkpoint.
    pub fn finish(mut self) -> (Simulation<D>, ReplayLog<D::Command>) {
        // The end checkpoint includes commands submitted after the last step.
        let now = self.sim.next_tick();
        self.log.checkpoints.push((now, self.sim.state_hash()));
        self.log.end_tick = now;
        (self.sim, self.log)
    }
}
