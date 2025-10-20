//! Nymph data representations.

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use derive_more::Error;

/// API error.
#[derive(Clone, Debug, Deserialize, Serialize, Error)]
pub struct ApiError {
    /// An API error code.
    pub code: ErrorCode,
    /// A user-friendly message of the error.
    pub message: String,
}

impl Display for ApiError {
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
    InvalidTransfer,
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
            4008 => ErrorCode::InvalidTransfer,
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
            ErrorCode::InvalidTransfer => 4008,
            ErrorCode::BadCredentials => 4010,
            ErrorCode::InternalServerError => 5000,
            ErrorCode::Other(other) => other,
        }
    }
}
