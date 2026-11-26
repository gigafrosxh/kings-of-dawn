//! Canonical binary encoding.
//!
//! One encoding is used both for the state hash and for save payloads, so two
//! states hash equal exactly when they serialise to identical bytes.
//!
//! Rules (format version 1):
//! - Integers are little-endian with fixed width (`u8`..`u128`, `i8`..`i128`).
//! - `bool` is one byte, `0` or `1`; any other value is rejected on decode.
//! - Sequences, strings and maps carry a `u32` element/byte count prefix.
//! - Strings are UTF-8.
//! - `Option<T>` is a `u8` tag (`0` = None, `1` = Some) followed by `T`.
//! - Maps are only supported as `BTreeMap`, encoded in ascending key order.
//!   Hash maps are deliberately not encodable: their iteration order is not
//!   deterministic.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Error produced while decoding canonical bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Input ended before the value was complete.
    UnexpectedEof {
        /// Bytes that were still required.
        needed: usize,
    },
    /// An enum tag or bool byte was not a known value.
    InvalidTag {
        /// Name of the type being decoded.
        ty: &'static str,
        /// The offending tag.
        tag: u32,
    },
    /// A string was not valid UTF-8.
    InvalidUtf8,
    /// A map was not stored in strictly ascending key order.
    UnsortedMap,
    /// A length prefix exceeds the remaining input.
    LengthOutOfRange,
    /// Bytes remained after decoding a complete top-level value.
    TrailingBytes {
        /// Number of unread bytes.
        remaining: usize,
    },
    /// A decoded value violates a type invariant.
    Invalid(&'static str),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { needed } => {
                write!(f, "unexpected end of input ({needed} bytes missing)")
            }
            Self::InvalidTag { ty, tag } => write!(f, "invalid tag {tag} for {ty}"),
            Self::InvalidUtf8 => f.write_str("invalid UTF-8 string"),
            Self::UnsortedMap => f.write_str("map keys not strictly ascending"),
            Self::LengthOutOfRange => f.write_str("length prefix exceeds input"),
            Self::TrailingBytes { remaining } => write!(f, "{remaining} trailing bytes"),
            Self::Invalid(msg) => write!(f, "invalid value: {msg}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Append-only byte sink for canonical encoding.
#[derive(Debug, Default, Clone)]
pub struct Encoder {
    buf: Vec<u8>,
}

impl Encoder {
    /// Creates an empty encoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes raw bytes without a length prefix.
    pub fn put_raw(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Writes a `u32` length prefix.
    ///
    /// # Panics
    /// Panics when `len` exceeds `u32::MAX`; such collections are a logic error.
    pub fn put_len(&mut self, len: usize) {
        let len = u32::try_from(len).expect("collection longer than u32::MAX");
        self.put_raw(&len.to_le_bytes());
    }

    /// Encodes a value.
    pub fn put<T: Encode + ?Sized>(&mut self, value: &T) {
        value.encode(self);
    }

    /// Returns the encoded bytes.
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    /// Bytes written so far.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }
}

/// Cursor over canonical bytes.
#[derive(Debug, Clone)]
pub struct Decoder<'a> {
    input: &'a [u8],
}

impl<'a> Decoder<'a> {
    /// Creates a decoder over `input`.
    pub fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    /// Reads `n` raw bytes.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.input.len() < n {
            return Err(DecodeError::UnexpectedEof {
                needed: n - self.input.len(),
            });
        }
        let (head, tail) = self.input.split_at(n);
        self.input = tail;
        Ok(head)
    }

    /// Reads a fixed-size array.
    pub fn take_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// Reads a `u32` length prefix and checks it against the remaining input,
    /// assuming every element occupies at least `min_elem_size` bytes.
    pub fn take_len(&mut self, min_elem_size: usize) -> Result<usize, DecodeError> {
        let len = u32::decode(self)? as usize;
        if len.saturating_mul(min_elem_size) > self.input.len() {
            return Err(DecodeError::LengthOutOfRange);
        }
        Ok(len)
    }

    /// Decodes a value.
    pub fn get<T: Decode>(&mut self) -> Result<T, DecodeError> {
        T::decode(self)
    }

    /// Remaining unread bytes.
    pub fn remaining(&self) -> usize {
        self.input.len()
    }

    /// Fails when unread bytes remain.
    pub fn finish(self) -> Result<(), DecodeError> {
        if self.input.is_empty() {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes {
                remaining: self.input.len(),
            })
        }
    }
}

/// Canonical encoding of a value.
pub trait Encode {
    /// Appends the canonical bytes of `self`.
    fn encode(&self, enc: &mut Encoder);

    /// Convenience: canonical bytes of `self`.
    fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut enc = Encoder::new();
        self.encode(&mut enc);
        enc.finish()
    }
}

/// Canonical decoding of a value.
pub trait Decode: Sized {
    /// Reads a value from the decoder.
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError>;

    /// Decodes a complete value; trailing bytes are an error.
    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(bytes);
        let value = Self::decode(&mut dec)?;
        dec.finish()?;
        Ok(value)
    }
}

macro_rules! int_codec {
    ($($t:ty),*) => {$(
        impl Encode for $t {
            fn encode(&self, enc: &mut Encoder) {
                enc.put_raw(&self.to_le_bytes());
            }
        }
        impl Decode for $t {
            fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
                Ok(<$t>::from_le_bytes(dec.take_array()?))
            }
        }
    )*};
}

int_codec!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl Encode for bool {
    fn encode(&self, enc: &mut Encoder) {
        enc.put_raw(&[u8::from(*self)]);
    }
}

impl Decode for bool {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        match u8::decode(dec)? {
            0 => Ok(false),
            1 => Ok(true),
            tag => Err(DecodeError::InvalidTag {
                ty: "bool",
                tag: tag.into(),
            }),
        }
    }
}

impl Encode for str {
    fn encode(&self, enc: &mut Encoder) {
        enc.put_len(self.len());
        enc.put_raw(self.as_bytes());
    }
}

impl Encode for String {
    fn encode(&self, enc: &mut Encoder) {
        self.as_str().encode(enc);
    }
}

impl Decode for String {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let len = dec.take_len(1)?;
        let bytes = dec.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| DecodeError::InvalidUtf8)
    }
}

impl<T: Encode> Encode for [T] {
    fn encode(&self, enc: &mut Encoder) {
        enc.put_len(self.len());
        for item in self {
            item.encode(enc);
        }
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, enc: &mut Encoder) {
        self.as_slice().encode(enc);
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let len = dec.take_len(1)?;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(T::decode(dec)?);
        }
        Ok(out)
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, enc: &mut Encoder) {
        match self {
            None => enc.put(&0u8),
            Some(v) => {
                enc.put(&1u8);
                v.encode(enc);
            }
        }
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        match u8::decode(dec)? {
            0 => Ok(None),
            1 => Ok(Some(T::decode(dec)?)),
            tag => Err(DecodeError::InvalidTag {
                ty: "Option",
                tag: tag.into(),
            }),
        }
    }
}

impl<K: Encode, V: Encode> Encode for BTreeMap<K, V> {
    fn encode(&self, enc: &mut Encoder) {
        enc.put_len(self.len());
        for (k, v) in self {
            k.encode(enc);
            v.encode(enc);
        }
    }
}

impl<K: Decode + Ord, V: Decode> Decode for BTreeMap<K, V> {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let len = dec.take_len(1)?;
        let mut out = BTreeMap::new();
        for _ in 0..len {
            let k = K::decode(dec)?;
            let v = V::decode(dec)?;
            if out.last_key_value().is_some_and(|(last, _)| *last >= k) {
                return Err(DecodeError::UnsortedMap);
            }
            out.insert(k, v);
        }
        Ok(out)
    }
}

impl<T: Encode> Encode for BTreeSet<T> {
    fn encode(&self, enc: &mut Encoder) {
        enc.put_len(self.len());
        for v in self {
            v.encode(enc);
        }
    }
}

impl<T: Decode + Ord> Decode for BTreeSet<T> {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let len = dec.take_len(1)?;
        let mut out = BTreeSet::new();
        for _ in 0..len {
            let v = T::decode(dec)?;
            if out.last().is_some_and(|last| *last >= v) {
                return Err(DecodeError::UnsortedMap);
            }
            out.insert(v);
        }
        Ok(out)
    }
}

impl<A: Encode, B: Encode> Encode for (A, B) {
    fn encode(&self, enc: &mut Encoder) {
        self.0.encode(enc);
        self.1.encode(enc);
    }
}

impl<A: Decode, B: Decode> Decode for (A, B) {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok((A::decode(dec)?, B::decode(dec)?))
    }
}

impl<T: Encode + ?Sized> Encode for &T {
    fn encode(&self, enc: &mut Encoder) {
        (**self).encode(enc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_are_little_endian() {
        assert_eq!(0x0102_0304u32.to_canonical_bytes(), vec![4, 3, 2, 1]);
        assert_eq!((-1i16).to_canonical_bytes(), vec![0xff, 0xff]);
    }

    #[test]
    fn roundtrip_composites() {
        let mut map = BTreeMap::new();
        map.insert(3u32, Some("drei".to_string()));
        map.insert(1u32, None);
        let value = (map, vec![true, false]);
        let bytes = value.to_canonical_bytes();
        let back =
            <(BTreeMap<u32, Option<String>>, Vec<bool>)>::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(back, value);
    }

    #[test]
    fn rejects_invalid_bool_and_trailing_bytes() {
        assert_eq!(
            bool::from_canonical_bytes(&[2]),
            Err(DecodeError::InvalidTag { ty: "bool", tag: 2 })
        );
        assert_eq!(
            u8::from_canonical_bytes(&[1, 2]),
            Err(DecodeError::TrailingBytes { remaining: 1 })
        );
    }

    #[test]
    fn rejects_unsorted_map_and_huge_length() {
        // len 2, keys 5 then 1
        let mut enc = Encoder::new();
        enc.put_len(2);
        enc.put(&5u8);
        enc.put(&0u8);
        enc.put(&1u8);
        enc.put(&0u8);
        assert_eq!(
            BTreeMap::<u8, u8>::from_canonical_bytes(enc.as_bytes()),
            Err(DecodeError::UnsortedMap)
        );
        assert_eq!(
            Vec::<u8>::from_canonical_bytes(&u32::MAX.to_le_bytes()),
            Err(DecodeError::LengthOutOfRange)
        );
    }

    #[test]
    fn truncated_input_reports_eof() {
        assert!(matches!(
            u64::from_canonical_bytes(&[1, 2, 3]),
            Err(DecodeError::UnexpectedEof { needed: 5 })
        ));
    }
}
