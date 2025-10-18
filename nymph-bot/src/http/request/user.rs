//! User-related queries and requests.

use std::num::NonZeroU64;

use futures_util::future::BoxFuture;

use http::Method;

use nymph_model::{request::user::UserProxyRequest, response::user::UserProxyResponse};

use twilight_model::id::{Id, marker::UserMarker};

use crate::http::Client;

use anyhow::Error;

/// Proxies for a Discord user using the bot.
#[derive(Debug)]
pub struct UserProxy {
    client: Client,
    discord_id: Id<UserMarker>,
    display_name: String,
}

impl UserProxy {
    /// Creates a new `UserProxy`.
    pub fn new(client: Client, discord_id: Id<UserMarker>, display_name: String) -> UserProxy {
        UserProxy {
            client,
            discord_id,
            display_name,
        }
    }
}

impl IntoFuture for UserProxy {
    type Output = Result<UserProxyResponse, Error>;
    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let request = self
                .client
                .request(Method::POST, "/users/proxy")
                .json(&UserProxyRequest {
                    discord_id: NonZeroU64::from(self.discord_id).into(),
                    display_name: self.display_name,
                })
                .send()
                .await?;

            Ok(request.json().await?)
        })
    }
}
