//! API user request models.

use serde::{Deserialize, Serialize};

use crate::Id;

/// A request body for proxying a user.
///
/// Represents the `POST /users/{user.id}` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserProxyRequest {
    /// The discord ID of the user.
    ///
    /// Proxy requests can only be made for discord ID authenticated users.
    pub discord_id: Id,
    /// The user's current username.
    pub display_name: String,
}
