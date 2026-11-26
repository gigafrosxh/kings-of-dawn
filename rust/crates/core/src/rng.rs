//! Deterministic pseudo random numbers.
//!
//! [`DetRng`] is xoshiro256** seeded through SplitMix64. Its full state is
//! part of the saved simulation state, so a loaded save continues with the
//! identical random sequence. Domains derive independent sub-streams with
//! [`DetRng::fork`] so that adding draws in one domain does not shift the
//! sequence of another.

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use crate::fixed::{Fixed, SCALE};

/// Seedable, serialisable random generator. Not cryptographically secure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetRng {
    s: [u64; 4],
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl DetRng {
    /// Generator for a session seed.
    pub fn from_seed(seed: u64) -> Self {
        let mut sm = seed;
        let s = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        // xoshiro must not start from the all-zero state; SplitMix64 cannot
        // produce four zero outputs in a row, but guard anyway.
        if s == [0; 4] {
            Self { s: [1, 0, 0, 0] }
        } else {
            Self { s }
        }
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform integer in `0..bound` without modulo bias. `bound` must be > 0.
    ///
    /// # Panics
    /// Panics when `bound == 0`.
    pub fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "DetRng::below requires bound > 0");
        // Lemire's nearly divisionless method.
        let mut m = u128::from(self.next_u64()) * u128::from(bound);
        if (m as u64) < bound {
            let threshold = bound.wrapping_neg() % bound;
            while (m as u64) < threshold {
                m = u128::from(self.next_u64()) * u128::from(bound);
            }
        }
        (m >> 64) as u64
    }

    /// Uniform fixed-point value in `[0, 1)` with full 10^-9 resolution.
    pub fn unit(&mut self) -> Fixed {
        Fixed::from_raw(i128::from(self.below(SCALE as u64)))
    }

    /// `true` with probability `p` (clamped to `[0, 1]`).
    pub fn chance(&mut self, p: Fixed) -> bool {
        self.unit() < p.clamp_to(Fixed::ZERO, Fixed::ONE)
    }

    /// Independent sub-stream identified by `stream`; does not advance `self`.
    pub fn fork(&self, stream: u64) -> Self {
        let mut sm = self.s[0]
            ^ self.s[1].rotate_left(13)
            ^ self.s[2].rotate_left(29)
            ^ self.s[3].rotate_left(43);
        let mut stream_state = stream;
        sm ^= splitmix64(&mut stream_state);
        Self::from_seed(splitmix64(&mut sm))
    }
}

impl Encode for DetRng {
    fn encode(&self, enc: &mut Encoder) {
        for word in self.s {
            enc.put(&word);
        }
    }
}

impl Decode for DetRng {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let s = [dec.get()?, dec.get()?, dec.get()?, dec.get()?];
        if s == [0; 4] {
            return Err(DecodeError::Invalid("all-zero rng state"));
        }
        Ok(Self { s })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = DetRng::from_seed(42);
        let mut b = DetRng::from_seed(42);
        let mut c = DetRng::from_seed(43);
        let sa: Vec<u64> = (0..32).map(|_| a.next_u64()).collect();
        let sb: Vec<u64> = (0..32).map(|_| b.next_u64()).collect();
        let sc: Vec<u64> = (0..32).map(|_| c.next_u64()).collect();
        assert_eq!(sa, sb);
        assert_ne!(sa, sc);
    }

    #[test]
    fn splitmix_matches_reference_vectors() {
        // Reference outputs of SplitMix64 for seed 0 (Vigna's splitmix64.c).
        let mut state = 0u64;
        assert_eq!(splitmix64(&mut state), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(&mut state), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(splitmix64(&mut state), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn known_answer_is_stable_across_platforms() {
        // Golden values guard against accidental algorithm changes, which would
        // invalidate every stored replay.
        let mut rng = DetRng::from_seed(0);
        let first: Vec<u64> = (0..3).map(|_| rng.next_u64()).collect();
        assert_eq!(first, GOLDEN_SEED0);
    }

    // Cross-checked against an independent Python implementation of
    // SplitMix64 + xoshiro256** (Vigna) when introduced.
    const GOLDEN_SEED0: [u64; 3] = [
        11_091_344_671_253_066_420,
        13_793_997_310_169_335_082,
        1_900_383_378_846_508_768,
    ];

    #[test]
    fn below_stays_in_range_and_codec_roundtrips() {
        let mut rng = DetRng::from_seed(7);
        for bound in [1u64, 2, 3, 10, 1_000_000_007] {
            for _ in 0..200 {
                assert!(rng.below(bound) < bound);
            }
        }
        let bytes = rng.to_canonical_bytes();
        let mut back = DetRng::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(back.next_u64(), rng.next_u64());
    }

    #[test]
    fn fork_is_pure_and_distinct() {
        let base = DetRng::from_seed(9);
        let mut f1 = base.fork(1);
        let mut f1b = base.fork(1);
        let mut f2 = base.fork(2);
        assert_eq!(f1.next_u64(), f1b.next_u64());
        assert_ne!(f1.next_u64(), f2.next_u64());
        assert_eq!(base, DetRng::from_seed(9));
    }

    #[test]
    fn chance_bounds() {
        let mut rng = DetRng::from_seed(1);
        assert!((0..100).all(|_| !rng.chance(Fixed::ZERO)));
        assert!((0..100).all(|_| rng.chance(Fixed::ONE)));
    }
}
