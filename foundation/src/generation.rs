//! Bounded slot and generation components.

use core::fmt;
use core::num::NonZeroU32;

/// A nonzero generation that changes whenever reusable storage is republished.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Generation(NonZeroU32);

impl Generation {
    /// The first valid generation.
    pub const INITIAL: Self = Self(NonZeroU32::MIN);

    /// Creates a generation, rejecting zero.
    pub const fn new(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the stored nonzero value.
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// Advances the generation without wrapping or reusing zero.
    pub const fn checked_next(self) -> Option<Self> {
        match self.get().checked_add(1) {
            Some(value) => Self::new(value),
            None => None,
        }
    }
}

impl Default for Generation {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl fmt::Display for Generation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(formatter)
    }
}

impl TryFrom<u32> for Generation {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

impl From<Generation> for u32 {
    fn from(value: Generation) -> Self {
        value.get()
    }
}

/// A bounded index into caller-owned or runtime-owned slot storage.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SlotIndex(u32);

impl SlotIndex {
    /// Creates a slot index.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the stored zero-based index.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SlotIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<u32> for SlotIndex {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<SlotIndex> for u32 {
    fn from(value: SlotIndex) -> Self {
        value.get()
    }
}

#[cfg(test)]
mod tests {
    use super::{Generation, SlotIndex};

    #[test]
    fn generations_never_wrap() {
        assert_eq!(Generation::new(0), None);
        assert_eq!(
            Generation::INITIAL.checked_next().map(Generation::get),
            Some(2)
        );
        let maximum = Generation::new(u32::MAX).expect("the maximum value is nonzero");
        assert_eq!(maximum.checked_next(), None);
    }

    #[test]
    fn slot_index_preserves_zero() {
        assert_eq!(SlotIndex::default().get(), 0);
        assert_eq!(SlotIndex::new(42).get(), 42);
    }
}
