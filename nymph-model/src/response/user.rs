//! User API responses.

use serde::{Deserialize, Serialize};

use crate::{Id, user::User};

/// A response from `POST /users/discord`. This endpoint allows the Discord bot
/// to update a discord user's details without querying for their id and such
/// beforehand, and also allows the bot to pose as them in requests.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateDiscordUserResponse {
    /// The user.
    pub user: User,
    /// The discord ID of the updated user.
    pub discord_id: Id,
    /// A signed JWT that allows the bot to proxy as a user.
    ///
    /// Only returned if `generate_token` was raised in the request. These
    /// typically have very short lifetimes (15 mins).
    pub access_token: Option<String>,
}
