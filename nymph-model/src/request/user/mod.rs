//! API user request models.

use serde::{Deserialize, Serialize};

use crate::Id;

/// Request body for the `POST /users/discord` endpoint.
///
/// Allows the bot to update a Discord user's information.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateDiscordUserRequest {
    /// The discord ID of the user.
    ///
    /// Proxy requests can only be made for discord ID authenticated users.
    pub discord_id: Id,
    /// The user's current username.
    pub display_name: String,
    /// Whether or not to generate a token for use in proxy.
    pub generate_token: bool,
}
