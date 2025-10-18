//! Card-related queries and requests.

use futures_util::future::BoxFuture;

use http::Method;

use nymph_model::{card::Card, request::card::ListCardsQuery};

use twilight_model::id::{Id, marker::GuildMarker};

use crate::http::Client;

use anyhow::Error;

/// Lists all cards in a guild.
#[derive(Debug)]
pub struct ListCards {
    client: Client,
    guild_id: Id<GuildMarker>,
    query: Option<String>,
    page: Option<u32>,
    count: Option<u32>,
}

impl ListCards {
    /// Creates a new `ListCards`.
    pub fn new(client: Client, guild_id: Id<GuildMarker>) -> ListCards {
        ListCards {
            client,
            guild_id,
            query: None,
            page: None,
            count: None,
        }
    }

    /// Searches the guild with a search term.
    pub fn search(self, query: impl Into<String>) -> ListCards {
        ListCards {
            query: Some(query.into()),
            ..self
        }
    }

    pub fn page(self, page: u32) -> ListCards {
        ListCards {
            page: Some(page),
            ..self
        }
    }

    pub fn count(self, count: u32) -> ListCards {
        ListCards {
            count: Some(count),
            ..self
        }
    }

    /// Finds either 1 or 0 cards in a guild.
    pub fn find(self, name: impl Into<String>) -> ListCards {
        ListCards {
            query: Some(name.into()),
            count: Some(1),
            ..self
        }
    }
}

impl IntoFuture for ListCards {
    type Output = Result<Vec<Card>, Error>;
    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let query = serde_urlencoded::ser::to_string(&ListCardsQuery {
                query: self.query,
                page: self.page,
                count: self.count,
            })?;

            let mut url = format!("/guilds/{}/cards", self.guild_id);
            if query.len() > 0 {
                url = format!("{}?{}", url, query);
            }

            let request = self.client.request(Method::GET, url).send().await?;

            Ok(request.json().await?)
        })
    }
}

/// Gets a card by its id.
pub struct GetCard {
    client: Client,
    guild_id: Id<GuildMarker>,
    id: i32,
}

impl GetCard {
    /// Create a new `GetCard`.
    pub fn new(client: Client, guild_id: Id<GuildMarker>, id: i32) -> GetCard {
        GetCard {
            client,
            guild_id,
            id,
        }
    }
}

impl IntoFuture for GetCard {
    type Output = Result<Card, Error>;
    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let request = self
                .client
                .request(
                    Method::GET,
                    format!("/guilds/{}/cards/{}", self.guild_id, self.id),
                )
                .send()
                .await?;

            Ok(request.json().await?)
        })
    }
}
