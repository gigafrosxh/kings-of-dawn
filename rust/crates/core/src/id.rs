//! Typed, stable identifiers.
//!
//! IDs are opaque `u32` values that are never reused within a save. They are
//! totally ordered, so iterating `BTreeMap<SomeId, _>` is deterministic, and
//! they are the only references that may cross the FFI boundary or be stored
//! in saves (no pointers, no hash-map positions).
//!
//! Domains declare their own ID types with [`define_id!`](crate::define_id).

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};

/// Declares a typed `u32` identifier with ordering, codec and `Display`.
///
/// ```
/// kod_core::define_id!(
///     /// Identifier of a province.
///     ProvinceId, "P"
/// );
/// let id = ProvinceId::new(7);
/// assert_eq!(id.to_string(), "P7");
/// ```
#[macro_export]
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            /// Wraps a raw value.
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            /// Raw numeric value.
            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, concat!($prefix, "{}"), self.0)
            }
        }

        impl $crate::codec::Encode for $name {
            fn encode(&self, enc: &mut $crate::codec::Encoder) {
                enc.put(&self.0);
            }
        }

        impl $crate::codec::Decode for $name {
            fn decode(dec: &mut $crate::codec::Decoder<'_>) -> Result<Self, $crate::codec::DecodeError> {
                Ok(Self(dec.get::<u32>()?))
            }
        }

        impl From<$name> for u32 {
            fn from(id: $name) -> u32 {
                id.0
            }
        }
    };
}

define_id!(
    /// Issuer of a command: a player seat, an AI controller or the system.
    ///
    /// `ActorId::SYSTEM` (0) is reserved for engine-internal commands.
    ActorId,
    "A"
);

impl ActorId {
    /// Reserved actor for engine-internal commands.
    pub const SYSTEM: Self = Self::new(0);
}

/// Monotonic allocator for one ID space. Part of the saved state, so IDs stay
/// stable across save/load and are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdAllocator {
    next: u32,
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self::starting_at(1)
    }
}

impl IdAllocator {
    /// Allocator whose first ID is `first` (0 is commonly reserved).
    pub const fn starting_at(first: u32) -> Self {
        Self { next: first }
    }

    /// Next raw value that [`allocate`](Self::allocate) would return.
    pub const fn peek(&self) -> u32 {
        self.next
    }

    /// Returns a fresh raw ID, or `None` once the space is exhausted.
    pub fn allocate(&mut self) -> Option<u32> {
        let id = self.next;
        self.next = self.next.checked_add(1)?;
        Some(id)
    }
}

impl Encode for IdAllocator {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.next);
    }
}

impl Decode for IdAllocator {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self { next: dec.get()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_is_monotonic_and_bounded() {
        let mut ids = IdAllocator::default();
        assert_eq!(ids.allocate(), Some(1));
        assert_eq!(ids.allocate(), Some(2));
        let mut last = IdAllocator::starting_at(u32::MAX);
        assert_eq!(last.allocate(), None);
    }

    #[test]
    fn ids_order_and_display() {
        assert!(ActorId::new(2) > ActorId::SYSTEM);
        assert_eq!(ActorId::new(5).to_string(), "A5");
    }
}
