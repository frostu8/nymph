//! User API responses.

use serde::{Deserialize, Serialize};

/// A response from `POST /users/{user.id}`. This endpoint (secured by mTLS)
/// allows the Discord bot to query as another user.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserProxyResponse {
    /// A signed JWT that allows the bot to proxy as a user.
    ///
    /// These typically have very short lifetimes (15 mins).
    pub token: String,
}
