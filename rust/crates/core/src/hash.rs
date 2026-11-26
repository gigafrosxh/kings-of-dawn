//! Stable state hash.
//!
//! [`StateHash`] is FNV-1a (128 bit) over canonical bytes. It is a
//! reproducibility fingerprint for replays, desync checks and save
//! integrity, **not** a cryptographic hash: it detects divergence and
//! corruption, not deliberate tampering.

use std::fmt;

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};

const OFFSET_BASIS: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// Incremental FNV-1a/128 hasher.
#[derive(Debug, Clone)]
pub struct StateHasher {
    state: u128,
}

impl Default for StateHasher {
    fn default() -> Self {
        Self {
            state: OFFSET_BASIS,
        }
    }
}

impl StateHasher {
    /// Fresh hasher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= u128::from(b);
            self.state = self.state.wrapping_mul(PRIME);
        }
    }

    /// Final digest.
    pub fn finish(&self) -> StateHash {
        StateHash(self.state)
    }
}

/// 128-bit fingerprint of canonical bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateHash(pub u128);

impl StateHash {
    /// Hash of raw bytes.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut h = StateHasher::new();
        h.update(bytes);
        h.finish()
    }

    /// Hash of a value's canonical encoding.
    pub fn of<T: Encode + ?Sized>(value: &T) -> Self {
        let mut enc = Encoder::new();
        value.encode(&mut enc);
        Self::of_bytes(enc.as_bytes())
    }
}

impl fmt::Display for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl Encode for StateHash {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.0);
    }
}

impl Decode for StateHash {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self(dec.get()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_128_reference_vectors() {
        assert_eq!(StateHash::of_bytes(b"").0, OFFSET_BASIS);
        assert_eq!(
            StateHash::of_bytes(b"a").0,
            0xd228_cb69_6f1a_8caf_7891_2b70_4e4a_8964
        );
    }

    #[test]
    fn display_is_fixed_width_hex() {
        assert_eq!(StateHash(1).to_string().len(), 32);
    }
}
