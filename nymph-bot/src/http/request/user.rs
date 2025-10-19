//! User-related queries and requests.

use std::num::NonZeroU64;

use http::Method;

use nymph_model::{
    request::user::UpdateDiscordUserRequest, response::user::UpdateDiscordUserResponse,
};

use twilight_model::id::{Id, marker::UserMarker};

use crate::http::Client;

use anyhow::Error;

/// Proxies for a Discord user using the bot.
#[derive(Debug)]
pub struct UpdateDiscordUser {
    client: Client,
    discord_id: Id<UserMarker>,
    display_name: String,
    generate_token: bool,
}

impl UpdateDiscordUser {
    /// Creates a new `UpdateDiscordUser`.
    pub fn new(client: Client, discord_id: Id<UserMarker>, display_name: String) -> Self {
        UpdateDiscordUser {
            client,
            discord_id,
            display_name,
            generate_token: false,
        }
    }

    /// Generates a token.
    pub fn generate_token(self, generate_token: bool) -> Self {
        UpdateDiscordUser {
            generate_token,
            ..self
        }
    }

    /// Sends the request.
    pub async fn execute(self) -> Result<UpdateDiscordUserResponse, Error> {
        let UpdateDiscordUser {
            client,
            discord_id,
            display_name,
            generate_token,
        } = self;

        let request = client
            .request(Method::POST, "/users/discord")
            .json(&UpdateDiscordUserRequest {
                discord_id: NonZeroU64::from(discord_id).into(),
                display_name: display_name,
                generate_token: generate_token,
            })
            .send_privileged()
            .await?;

        // get response and cache
        let res = request.json::<UpdateDiscordUserResponse>().await?;
        client.update_cache(&res).await;

        Ok(res)
    }
}
