//! Tick scheduler.

use std::collections::BTreeMap;
use std::fmt;

use crate::codec::{Decode, DecodeError, Decoder, Encoder};
use crate::domain::{Cadence, CommandEnvelope, CommandError, Domain, Phase, TickContext};
use crate::hash::{StateHash, StateHasher};
use crate::id::ActorId;
use crate::rng::DetRng;
use crate::time::{SimDate, Tick};

/// Version of the kernel's own state layout (around the domain state).
pub const KERNEL_STATE_VERSION: u32 = 1;

/// Why a command could not be queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitError {
    /// `execute_at` lies before the next tick to be processed.
    InPast {
        /// Requested tick.
        execute_at: Tick,
        /// Next tick the scheduler will process.
        next_tick: Tick,
    },
    /// `sequence` is not greater than the last one accepted from this actor.
    SequenceNotIncreasing {
        /// Issuer.
        actor: ActorId,
        /// Last accepted sequence.
        last: u64,
    },
}

impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InPast {
                execute_at,
                next_tick,
            } => write!(f, "command for {execute_at} submitted at {next_tick}"),
            Self::SequenceNotIncreasing { actor, last } => {
                write!(f, "sequence of {actor} must exceed {last}")
            }
        }
    }
}

impl std::error::Error for SubmitError {}

/// Result of one queued command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// Validated and applied.
    Applied,
    /// Rejected; state unchanged.
    Rejected(CommandError),
}

/// What happened during one tick.
#[derive(Debug, Clone)]
pub struct TickReport<D: Domain> {
    /// Processed tick.
    pub tick: Tick,
    /// Its date.
    pub date: SimDate,
    /// Commands executed in this tick, in execution order.
    pub commands: Vec<(ActorId, u64, CommandOutcome)>,
    /// Events from commands and phases, in emission order.
    pub events: Vec<D::Event>,
}

type QueueKey = (Tick, ActorId, u64);

/// Deterministic simulation of one domain.
///
/// Everything that influences future ticks (domain state, clock, RNG, command
/// queue, per-actor sequence numbers) lives here and is covered by
/// [`state_hash`](Self::state_hash) and saves.
#[derive(Debug, Clone)]
pub struct Simulation<D: Domain> {
    seed: u64,
    start_date: SimDate,
    next_tick: Tick,
    rng: DetRng,
    last_sequence: BTreeMap<ActorId, u64>,
    queue: BTreeMap<QueueKey, D::Command>,
    state: D::State,
}

impl<D: Domain> Simulation<D> {
    /// New session at tick 0 = `start_date`.
    pub fn new(seed: u64, start_date: SimDate, state: D::State) -> Self {
        Self {
            seed,
            start_date,
            next_tick: Tick(0),
            rng: DetRng::from_seed(seed),
            last_sequence: BTreeMap::new(),
            queue: BTreeMap::new(),
            state,
        }
    }

    /// Session seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Date of tick 0.
    pub fn start_date(&self) -> SimDate {
        self.start_date
    }

    /// Next tick that [`step`](Self::step) will process.
    pub fn next_tick(&self) -> Tick {
        self.next_tick
    }

    /// Date of the next tick.
    pub fn current_date(&self) -> SimDate {
        self.date_of(self.next_tick)
    }

    /// Read access to the domain state.
    pub fn state(&self) -> &D::State {
        &self.state
    }

    /// Number of queued commands.
    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Last accepted sequence number of `actor`, if any.
    pub fn last_sequence(&self, actor: ActorId) -> Option<u64> {
        self.last_sequence.get(&actor).copied()
    }

    /// Answers a read-only query.
    pub fn query(&self, query: &D::Query) -> D::Answer {
        D::query(&self.state, query)
    }

    fn date_of(&self, tick: Tick) -> SimDate {
        self.start_date
            .add_days(tick.0)
            .expect("simulation ran beyond the supported calendar range")
    }

    /// Queues a command. Validation happens when the command executes, against
    /// the state of that tick.
    pub fn submit(&mut self, envelope: CommandEnvelope<D::Command>) -> Result<(), SubmitError> {
        if envelope.execute_at < self.next_tick {
            return Err(SubmitError::InPast {
                execute_at: envelope.execute_at,
                next_tick: self.next_tick,
            });
        }
        if let Some(&last) = self.last_sequence.get(&envelope.actor)
            && envelope.sequence <= last
        {
            return Err(SubmitError::SequenceNotIncreasing {
                actor: envelope.actor,
                last,
            });
        }
        self.last_sequence.insert(envelope.actor, envelope.sequence);
        self.queue.insert(
            (envelope.execute_at, envelope.actor, envelope.sequence),
            envelope.command,
        );
        Ok(())
    }

    /// Processes one tick: due commands in stable order, then all phases.
    pub fn step(&mut self) -> TickReport<D> {
        let tick = self.next_tick;
        let date = self.date_of(tick);
        let mut ctx = TickContext {
            tick,
            date,
            cadence: Cadence::of(date),
            rng: &mut self.rng,
        };
        let mut events = Vec::new();
        let mut commands = Vec::new();

        while let Some(entry) = self.queue.first_entry() {
            let (at, actor, sequence) = *entry.key();
            if at > tick {
                break;
            }
            let command = entry.remove();
            let outcome = match D::validate(&self.state, actor, &command) {
                Ok(()) => {
                    D::apply(&mut self.state, &mut ctx, actor, &command, &mut events);
                    CommandOutcome::Applied
                }
                Err(err) => CommandOutcome::Rejected(err),
            };
            commands.push((actor, sequence, outcome));
        }

        for phase in Phase::ORDER {
            D::run_phase(&mut self.state, phase, &mut ctx, &mut events);
        }

        self.next_tick = tick.next();
        TickReport {
            tick,
            date,
            commands,
            events,
        }
    }

    /// Steps until `next_tick() == target`; returns the number of ticks run.
    pub fn run_until(&mut self, target: Tick) -> u64 {
        let mut ran = 0;
        while self.next_tick < target {
            self.step();
            ran += 1;
        }
        ran
    }

    /// Fingerprint of the complete simulation state.
    pub fn state_hash(&self) -> StateHash {
        let mut enc = Encoder::new();
        self.encode_state(&mut enc);
        let mut hasher = StateHasher::new();
        hasher.update(D::SCHEMA_ID.as_bytes());
        hasher.update(&D::SCHEMA_VERSION.to_le_bytes());
        hasher.update(enc.as_bytes());
        hasher.finish()
    }

    /// Canonical encoding of the full state (save payload and hash input).
    pub fn encode_state(&self, enc: &mut Encoder) {
        enc.put(&KERNEL_STATE_VERSION);
        enc.put(&self.seed);
        enc.put(&self.start_date);
        enc.put(&self.next_tick);
        enc.put(&self.rng);
        enc.put(&self.last_sequence);
        enc.put(&(self.queue.len() as u64));
        self.encode_queue(enc);
        let mut domain = Encoder::new();
        domain.put(&self.state);
        enc.put(&domain.finish());
    }

    fn encode_queue(&self, enc: &mut Encoder) {
        for ((at, actor, seq), cmd) in &self.queue {
            enc.put(at);
            enc.put(actor);
            enc.put(seq);
            enc.put(cmd);
        }
    }

    /// Inverse of [`encode_state`](Self::encode_state). `domain_version` is the
    /// schema version the payload was written with; older versions go through
    /// [`Domain::migrate`].
    pub fn decode_state(bytes: &[u8], domain_version: u32) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(bytes);
        let kernel_version: u32 = dec.get()?;
        if kernel_version != KERNEL_STATE_VERSION {
            return Err(DecodeError::Invalid("unsupported kernel state version"));
        }
        let seed = dec.get()?;
        let start_date = dec.get()?;
        let next_tick = dec.get()?;
        let rng = dec.get()?;
        let last_sequence = dec.get()?;
        let queue_len = dec.get::<u64>()? as usize;
        let mut queue = BTreeMap::new();
        for _ in 0..queue_len {
            let key: QueueKey = (dec.get()?, dec.get()?, dec.get()?);
            if key.0 < next_tick {
                return Err(DecodeError::Invalid("queued command in the past"));
            }
            if queue.last_key_value().is_some_and(|(last, _)| *last >= key) {
                return Err(DecodeError::UnsortedMap);
            }
            queue.insert(key, dec.get()?);
        }
        let domain_bytes: Vec<u8> = dec.get()?;
        dec.finish()?;
        let state = if domain_version == D::SCHEMA_VERSION {
            D::State::from_canonical_bytes(&domain_bytes)?
        } else if domain_version < D::SCHEMA_VERSION {
            D::migrate(domain_version, &domain_bytes)?
        } else {
            return Err(DecodeError::Invalid(
                "save written by a newer schema version",
            ));
        };
        Ok(Self {
            seed,
            start_date,
            next_tick,
            rng,
            last_sequence,
            queue,
            state,
        })
    }
}
