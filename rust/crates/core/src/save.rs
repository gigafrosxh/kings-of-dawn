//! Versioned, checksummed save container.
//!
//! Layout (container version 1, all integers little-endian):
//!
//! | Field              | Encoding                                   |
//! |--------------------|--------------------------------------------|
//! | magic              | 8 bytes `KODSAVE\0`                        |
//! | container version  | `u16` (= 1)                                |
//! | schema id          | string (`u32` length + UTF-8)              |
//! | schema version     | `u32`                                      |
//! | state hash         | `u128`, [`Simulation::state_hash`]         |
//! | payload            | bytes (`u32` length), [`Simulation::encode_state`] |
//! | checksum           | `u128`, FNV-1a/128 over all previous bytes |
//!
//! The SQLite metadata store and snapshot compression (docs/ARCHITECTURE.md)
//! wrap this container later; the container itself stays the unit of
//! integrity and versioning.

use std::fmt;

use crate::codec::{DecodeError, Decoder, Encoder};
use crate::domain::Domain;
use crate::hash::StateHash;
use crate::sim::Simulation;

/// File magic.
pub const MAGIC: [u8; 8] = *b"KODSAVE\0";
/// Current container version.
pub const CONTAINER_VERSION: u16 = 1;

/// Save loading failure. Loading never yields a partially restored state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// Not a Kings of Dawn save.
    BadMagic,
    /// Container written by an unknown (newer) version.
    UnsupportedContainer(u16),
    /// Save belongs to a different domain schema.
    SchemaMismatch {
        /// Expected schema id.
        expected: &'static str,
        /// Found schema id.
        found: String,
    },
    /// Checksum mismatch: the file is corrupted or truncated.
    Corrupted,
    /// Payload decodes, but its state hash differs from the stored one.
    StateHashMismatch {
        /// Hash stored in the file.
        stored: StateHash,
        /// Hash of the restored state.
        restored: StateHash,
    },
    /// Structurally invalid content.
    Decode(DecodeError),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => f.write_str("not a Kings of Dawn save"),
            Self::UnsupportedContainer(v) => write!(f, "unsupported save container version {v}"),
            Self::SchemaMismatch { expected, found } => {
                write!(f, "save schema {found}, expected {expected}")
            }
            Self::Corrupted => f.write_str("save checksum mismatch"),
            Self::StateHashMismatch { stored, restored } => {
                write!(f, "state hash {restored} differs from stored {stored}")
            }
            Self::Decode(err) => write!(f, "invalid save: {err}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<DecodeError> for SaveError {
    fn from(err: DecodeError) -> Self {
        Self::Decode(err)
    }
}

/// Metadata readable without restoring the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    /// Domain schema id.
    pub schema_id: String,
    /// Domain schema version the payload was written with.
    pub schema_version: u32,
    /// State hash at save time.
    pub state_hash: StateHash,
    /// Raw state payload.
    pub payload: Vec<u8>,
}

impl SaveFile {
    /// Serialises a simulation.
    pub fn write<D: Domain>(sim: &Simulation<D>) -> Vec<u8> {
        let mut payload = Encoder::new();
        sim.encode_state(&mut payload);
        let mut enc = Encoder::new();
        enc.put_raw(&MAGIC);
        enc.put(&CONTAINER_VERSION);
        enc.put(D::SCHEMA_ID);
        enc.put(&D::SCHEMA_VERSION);
        enc.put(&sim.state_hash());
        enc.put(&payload.finish());
        let checksum = StateHash::of_bytes(enc.as_bytes());
        enc.put(&checksum);
        enc.finish()
    }

    /// Parses and integrity-checks the container without decoding the payload.
    pub fn parse(bytes: &[u8]) -> Result<Self, SaveError> {
        if bytes.len() < MAGIC.len() || bytes[..MAGIC.len()] != MAGIC {
            return Err(SaveError::BadMagic);
        }
        if bytes.len() < MAGIC.len() + 2 + 16 {
            return Err(SaveError::Corrupted);
        }
        let (body, checksum_bytes) = bytes.split_at(bytes.len() - 16);
        let mut dec = Decoder::new(&body[MAGIC.len()..]);
        let version: u16 = dec.get()?;
        if version != CONTAINER_VERSION {
            return Err(SaveError::UnsupportedContainer(version));
        }
        let stored = u128::from_le_bytes(checksum_bytes.try_into().expect("16 bytes"));
        if StateHash::of_bytes(body) != StateHash(stored) {
            return Err(SaveError::Corrupted);
        }
        let file = Self {
            schema_id: dec.get()?,
            schema_version: dec.get()?,
            state_hash: dec.get()?,
            payload: dec.get()?,
        };
        dec.finish()?;
        Ok(file)
    }

    /// Parses, verifies and restores a simulation.
    pub fn load<D: Domain>(bytes: &[u8]) -> Result<Simulation<D>, SaveError> {
        let file = Self::parse(bytes)?;
        if file.schema_id != D::SCHEMA_ID {
            return Err(SaveError::SchemaMismatch {
                expected: D::SCHEMA_ID,
                found: file.schema_id,
            });
        }
        let sim = Simulation::<D>::decode_state(&file.payload, file.schema_version)?;
        // A migrated state legitimately hashes differently from the old one.
        if file.schema_version == D::SCHEMA_VERSION {
            let restored = sim.state_hash();
            if restored != file.state_hash {
                return Err(SaveError::StateHashMismatch {
                    stored: file.state_hash,
                    restored,
                });
            }
        }
        Ok(sim)
    }
}
