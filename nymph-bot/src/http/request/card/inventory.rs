//! Card inventory transfers and manipulation.

use anyhow::Error;

use http::Method;
use nymph_model::{card::Card, request::card::inventory::GrantRequest};

use crate::http::Client;

/// Grants a card to a user.
#[derive(Debug)]
pub struct GrantCard {
    client: Client,
    user_id: i32,
    card_id: i32,
}

impl GrantCard {
    /// Creates a new `GrantCard`.
    pub fn new(client: Client, user_id: i32, card_id: i32) -> GrantCard {
        GrantCard {
            client,
            user_id,
            card_id,
        }
    }

    /// Sends the request.
    pub async fn execute(self) -> Result<Card, Error> {
        let GrantCard {
            client,
            user_id,
            card_id,
        } = self;

        let request = client
            .request(Method::POST, format!("/users/{}/cards", user_id))
            .json(&GrantRequest { card_id })
            .send()
            .await?;

        Ok(request.json().await?)
    }
}
