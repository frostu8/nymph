//! Nymph data representations.

pub mod card;
pub mod error;
pub mod request;
pub mod response;
pub mod user;

pub use error::{ApiError, ErrorCode};

use std::num::NonZeroU64;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use derive_more::{Deref, DerefMut, From, Into};

/// A container used to serialize and deserialize large ids.
///
/// Because Discord snowflakes approach sizes of integer not representable by
/// Javascript's usual JSON parsing utilities, they are encoded as string
/// atoms.
#[derive(Clone, Copy, Debug, From, Into, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct Id(NonZeroU64);

impl Id {
    /// Creates a new `Id`.
    ///
    /// Returns `None` if the id is 0.
    pub fn new(inner: u64) -> Option<Id> {
        NonZeroU64::new(inner).map(|id| Id(id))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|s| s.parse::<u64>().map_err(|e| D::Error::custom(e)))
            .and_then(|id| NonZeroU64::new(id).ok_or_else(|| D::Error::custom("id is 0")))
            .map(|id| Id(id))
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.get().to_string().serialize(serializer)
    }
}
