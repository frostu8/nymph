//! Nymph data representations.

use std::{
    fmt::{self, Display, Formatter},
    num::NonZeroU64,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use derive_more::{Deref, DerefMut, Error, From, Into};

pub mod card;
pub mod request;
pub mod response;

/// A container used to serialize and deserialize large ids.
///
/// Because Discord snowflakes approach sizes of integer not representable by
/// Javascript's usual JSON parsing utilities, they are encoded as string
/// atoms.
#[derive(Clone, Copy, Debug, From, Into, Deref, DerefMut, PartialEq, Eq)]
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

/// API error.
#[derive(Clone, Debug, Deserialize, Serialize, Error)]
pub struct Error {
    /// An API error code.
    pub code: ErrorCode,
    /// A user-friendly message of the error.
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

/// An API error code.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(from = "u32", into = "u32")]
pub enum ErrorCode {
    /// The request consisted of malformed JSON.
    MalformedJson,
    /// The request had a well-formed body, but the data was otherwise
    /// unexpected.
    InvalidData,
    /// The server refuses to serve that content type.
    UnsupportedContentType,
    /// The resource was not found.
    NotFound,
    /// The authenticated user does not have access to this resource.
    Forbidden,
    /// The card is hidden to the user.
    Hidden,
    /// The card is already owned by the user.
    AlreadyOwned,
    /// The card is not owned by the user.
    Unowned,
    /// The user is unauthorized.
    Unauthenticated,
    /// The user's credentials have expired or are otherwise bad.
    BadCredentials,
    /// The user does not have the right permissions to access a resource.
    InsufficientPermissions,
    /// An internal server error occured.
    ///
    /// This is a bug, usually.
    InternalServerError,
    /// Any other error code.
    Other(u32),
}

impl From<u32> for ErrorCode {
    fn from(value: u32) -> Self {
        match value {
            4000 => ErrorCode::MalformedJson,
            4001 => ErrorCode::InvalidData,
            4002 => ErrorCode::UnsupportedContentType,
            4003 => ErrorCode::NotFound,
            4004 => ErrorCode::Unauthenticated,
            4005 => ErrorCode::Forbidden,
            4006 => ErrorCode::Hidden,
            4007 => ErrorCode::InsufficientPermissions,
            4008 => ErrorCode::AlreadyOwned,
            4009 => ErrorCode::Unowned,
            4010 => ErrorCode::BadCredentials,
            5000 => ErrorCode::InternalServerError,
            other => ErrorCode::Other(other),
        }
    }
}

impl From<ErrorCode> for u32 {
    fn from(value: ErrorCode) -> Self {
        match value {
            ErrorCode::MalformedJson => 4000,
            ErrorCode::InvalidData => 4001,
            ErrorCode::UnsupportedContentType => 4002,
            ErrorCode::NotFound => 4003,
            ErrorCode::Unauthenticated => 4004,
            ErrorCode::Forbidden => 4005,
            ErrorCode::Hidden => 4006,
            ErrorCode::InsufficientPermissions => 4007,
            ErrorCode::AlreadyOwned => 4008,
            ErrorCode::Unowned => 4009,
            ErrorCode::BadCredentials => 4010,
            ErrorCode::InternalServerError => 5000,
            ErrorCode::Other(other) => other,
        }
    }
}
