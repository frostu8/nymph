//! Card-related queries and requests.

pub mod inventory;

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

    /// Sets the page to explore.
    pub fn page(self, page: u32) -> ListCards {
        ListCards {
            page: Some(page),
            ..self
        }
    }

    /// Sets the count of entries to return.
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

    /// Sends the request.
    pub async fn execute(self) -> Result<Vec<Card>, Error> {
        let ListCards {
            client,
            guild_id,
            query,
            page,
            count,
        } = self;

        let request = client
            .request(Method::GET, format!("/guilds/{}/cards", guild_id))
            .query(&ListCardsQuery { query, page, count })
            .send()
            .await?;

        Ok(request.json().await?)
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

    /// Sends the request.
    pub async fn execute(self) -> Result<Card, Error> {
        let GetCard {
            client,
            guild_id,
            id,
        } = self;

        let request = client
            .request(Method::GET, format!("/guilds/{}/cards/{}", guild_id, id))
            .send()
            .await?;

        Ok(request.json().await?)
    }
}
