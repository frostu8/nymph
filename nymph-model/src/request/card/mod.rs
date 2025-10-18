//! Card endpoint request models.

pub mod inventory;

use serde::{Deserialize, Serialize};

/// List cards endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListCardsQuery {
    /// Search query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The query's page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// How many results should be returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}
