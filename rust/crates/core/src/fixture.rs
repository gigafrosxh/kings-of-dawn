//! Test fixture domain (feature `fixtures`).
//!
//! A deliberately small ledger that exercises every kernel contract:
//! authorisation, validation, atomic application, RNG use inside phases,
//! monthly cadence and fixed-point rounding. It is **not** game content; it
//! exists for kernel tests, the replay/save regression fixtures and the FFI
//! roundtrip of #7 until real domains replace it.

use std::collections::BTreeMap;

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use crate::domain::{CommandError, Domain, Phase, TickContext};
use crate::fixed::Fixed;
use crate::id::{ActorId, IdAllocator};
use crate::time::Tick;

crate::define_id!(
    /// Ledger account.
    AccountId,
    "acct"
);

/// Maximum account name length in bytes.
pub const MAX_NAME_LEN: usize = 64;

/// One account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Owning actor; only the owner may debit or close it.
    pub owner: ActorId,
    /// Unique display name.
    pub name: String,
    /// Balance, never negative.
    pub balance: Fixed,
}

/// Ledger state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerState {
    /// Accounts by stable id.
    pub accounts: BTreeMap<AccountId, Account>,
    /// Account id allocator.
    pub ids: IdAllocator,
    /// Monthly interest rate on positive balances.
    pub monthly_interest: Fixed,
    /// Daily probability of a windfall of 1 unit to a random account.
    pub windfall_chance: Fixed,
}

impl Default for LedgerState {
    fn default() -> Self {
        Self {
            accounts: BTreeMap::new(),
            ids: IdAllocator::default(),
            monthly_interest: Fixed::from_raw(2_500_000), // 0.0025
            windfall_chance: Fixed::from_raw(100_000_000), // 0.1
        }
    }
}

/// Ledger commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerCommand {
    /// Opens an account owned by the issuer with an initial deposit.
    Open {
        /// Unique name.
        name: String,
        /// Initial deposit (>= 0).
        deposit: Fixed,
    },
    /// Moves `amount` from an account of the issuer to another account.
    Transfer {
        /// Debited account (owned by the issuer).
        from: AccountId,
        /// Credited account.
        to: AccountId,
        /// Amount (> 0).
        amount: Fixed,
    },
    /// Closes an empty account of the issuer.
    Close {
        /// Account to close.
        account: AccountId,
    },
}

/// Ledger events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerEvent {
    /// Account opened.
    Opened(AccountId),
    /// Transfer booked.
    Transferred {
        /// From.
        from: AccountId,
        /// To.
        to: AccountId,
        /// Amount.
        amount: Fixed,
    },
    /// Account closed.
    Closed(AccountId),
    /// Monthly interest booked.
    InterestPaid {
        /// Account.
        account: AccountId,
        /// Amount.
        amount: Fixed,
    },
    /// Random windfall.
    Windfall {
        /// Account.
        account: AccountId,
        /// Tick.
        tick: Tick,
    },
}

/// Ledger queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerQuery {
    /// Balance of an account.
    Balance(AccountId),
    /// Sum of all balances.
    Total,
}

/// Ledger domain marker.
#[derive(Debug, Clone, Copy)]
pub struct Ledger;

impl Domain for Ledger {
    const SCHEMA_ID: &'static str = "kod.fixture.ledger";
    const SCHEMA_VERSION: u32 = 1;

    type State = LedgerState;
    type Command = LedgerCommand;
    type Event = LedgerEvent;
    type Query = LedgerQuery;
    type Answer = Option<Fixed>;

    fn validate(
        state: &LedgerState,
        actor: ActorId,
        command: &LedgerCommand,
    ) -> Result<(), CommandError> {
        match command {
            LedgerCommand::Open { name, deposit } => {
                if name.is_empty() || name.len() > MAX_NAME_LEN {
                    return Err(CommandError::InvalidArgument("name length".into()));
                }
                if deposit.is_negative() {
                    return Err(CommandError::InvalidArgument("negative deposit".into()));
                }
                if state.accounts.values().any(|a| a.name == *name) {
                    return Err(CommandError::Conflict(format!("name {name} taken")));
                }
                if state.ids.peek() == u32::MAX {
                    return Err(CommandError::Conflict("account ids exhausted".into()));
                }
                Ok(())
            }
            LedgerCommand::Transfer { from, to, amount } => {
                let src = state
                    .accounts
                    .get(from)
                    .ok_or_else(|| CommandError::UnknownTarget(from.to_string()))?;
                let dst = state
                    .accounts
                    .get(to)
                    .ok_or_else(|| CommandError::UnknownTarget(to.to_string()))?;
                if src.owner != actor {
                    return Err(CommandError::Unauthorized);
                }
                if from == to || *amount <= Fixed::ZERO {
                    return Err(CommandError::InvalidArgument(
                        "transfer amount or target".into(),
                    ));
                }
                if src.balance < *amount {
                    return Err(CommandError::Conflict("insufficient balance".into()));
                }
                dst.balance
                    .checked_add(*amount)
                    .map_err(|_| CommandError::Conflict("balance overflow".into()))?;
                Ok(())
            }
            LedgerCommand::Close { account } => {
                let acc = state
                    .accounts
                    .get(account)
                    .ok_or_else(|| CommandError::UnknownTarget(account.to_string()))?;
                if acc.owner != actor {
                    return Err(CommandError::Unauthorized);
                }
                if acc.balance != Fixed::ZERO {
                    return Err(CommandError::Conflict("balance not zero".into()));
                }
                Ok(())
            }
        }
    }

    fn apply(
        state: &mut LedgerState,
        _ctx: &mut TickContext<'_>,
        actor: ActorId,
        command: &LedgerCommand,
        events: &mut Vec<LedgerEvent>,
    ) {
        match command {
            LedgerCommand::Open { name, deposit } => {
                let id = AccountId::new(state.ids.allocate().expect("validated"));
                state.accounts.insert(
                    id,
                    Account {
                        owner: actor,
                        name: name.clone(),
                        balance: *deposit,
                    },
                );
                events.push(LedgerEvent::Opened(id));
            }
            LedgerCommand::Transfer { from, to, amount } => {
                let src = state.accounts.get_mut(from).expect("validated");
                src.balance = src.balance.checked_sub(*amount).expect("validated");
                let dst = state.accounts.get_mut(to).expect("validated");
                dst.balance = dst.balance.checked_add(*amount).expect("validated");
                events.push(LedgerEvent::Transferred {
                    from: *from,
                    to: *to,
                    amount: *amount,
                });
            }
            LedgerCommand::Close { account } => {
                state.accounts.remove(account);
                events.push(LedgerEvent::Closed(*account));
            }
        }
    }

    fn run_phase(
        state: &mut LedgerState,
        phase: Phase,
        ctx: &mut TickContext<'_>,
        events: &mut Vec<LedgerEvent>,
    ) {
        match phase {
            Phase::DueRulesAndEvents => {
                if !state.accounts.is_empty() && ctx.rng.chance(state.windfall_chance) {
                    let index = ctx.rng.below(state.accounts.len() as u64) as usize;
                    let (&id, account) = state
                        .accounts
                        .iter_mut()
                        .nth(index)
                        .expect("index in range");
                    account.balance = account.balance.saturating_add(Fixed::ONE);
                    events.push(LedgerEvent::Windfall {
                        account: id,
                        tick: ctx.tick,
                    });
                }
            }
            Phase::AccountingAndBudget if ctx.cadence.month_start => {
                for (&id, account) in &mut state.accounts {
                    if account.balance > Fixed::ZERO {
                        let interest = account
                            .balance
                            .checked_mul(state.monthly_interest)
                            .unwrap_or(Fixed::ZERO);
                        account.balance = account.balance.saturating_add(interest);
                        events.push(LedgerEvent::InterestPaid {
                            account: id,
                            amount: interest,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn query(state: &LedgerState, query: &LedgerQuery) -> Option<Fixed> {
        match query {
            LedgerQuery::Balance(id) => state.accounts.get(id).map(|a| a.balance),
            LedgerQuery::Total => state
                .accounts
                .values()
                .try_fold(Fixed::ZERO, |sum, a| sum.checked_add(a.balance).ok()),
        }
    }
}

impl Encode for Account {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.owner);
        enc.put(&self.name);
        enc.put(&self.balance);
    }
}

impl Decode for Account {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            owner: dec.get()?,
            name: dec.get()?,
            balance: dec.get()?,
        })
    }
}

impl Encode for LedgerState {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.accounts);
        enc.put(&self.ids);
        enc.put(&self.monthly_interest);
        enc.put(&self.windfall_chance);
    }
}

impl Decode for LedgerState {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            accounts: dec.get()?,
            ids: dec.get()?,
            monthly_interest: dec.get()?,
            windfall_chance: dec.get()?,
        })
    }
}

impl Encode for LedgerCommand {
    fn encode(&self, enc: &mut Encoder) {
        match self {
            Self::Open { name, deposit } => {
                enc.put(&0u8);
                enc.put(name);
                enc.put(deposit);
            }
            Self::Transfer { from, to, amount } => {
                enc.put(&1u8);
                enc.put(from);
                enc.put(to);
                enc.put(amount);
            }
            Self::Close { account } => {
                enc.put(&2u8);
                enc.put(account);
            }
        }
    }
}

impl Decode for LedgerCommand {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        match dec.get::<u8>()? {
            0 => Ok(Self::Open {
                name: dec.get()?,
                deposit: dec.get()?,
            }),
            1 => Ok(Self::Transfer {
                from: dec.get()?,
                to: dec.get()?,
                amount: dec.get()?,
            }),
            2 => Ok(Self::Close {
                account: dec.get()?,
            }),
            tag => Err(DecodeError::InvalidTag {
                ty: "LedgerCommand",
                tag: tag.into(),
            }),
        }
    }
}

impl Encode for LedgerEvent {
    fn encode(&self, enc: &mut Encoder) {
        match self {
            Self::Opened(id) => {
                enc.put(&0u8);
                enc.put(id);
            }
            Self::Transferred { from, to, amount } => {
                enc.put(&1u8);
                enc.put(from);
                enc.put(to);
                enc.put(amount);
            }
            Self::Closed(id) => {
                enc.put(&2u8);
                enc.put(id);
            }
            Self::InterestPaid { account, amount } => {
                enc.put(&3u8);
                enc.put(account);
                enc.put(amount);
            }
            Self::Windfall { account, tick } => {
                enc.put(&4u8);
                enc.put(account);
                enc.put(tick);
            }
        }
    }
}

impl Decode for LedgerEvent {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        match dec.get::<u8>()? {
            0 => Ok(Self::Opened(dec.get()?)),
            1 => Ok(Self::Transferred {
                from: dec.get()?,
                to: dec.get()?,
                amount: dec.get()?,
            }),
            2 => Ok(Self::Closed(dec.get()?)),
            3 => Ok(Self::InterestPaid {
                account: dec.get()?,
                amount: dec.get()?,
            }),
            4 => Ok(Self::Windfall {
                account: dec.get()?,
                tick: dec.get()?,
            }),
            tag => Err(DecodeError::InvalidTag {
                ty: "LedgerEvent",
                tag: tag.into(),
            }),
        }
    }
}
