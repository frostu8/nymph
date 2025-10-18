//! API user inventory request models.

use serde::{Deserialize, Serialize};

use crate::Id;

/// List cards owned by user endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListInventoryQuery {
    /// Filter by guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Id>,
    /// The query's page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// How many results should be returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

/// A request for granting a card.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GrantRequest {
    /// The ID of the card to grant.
    pub card_id: i32,
}
