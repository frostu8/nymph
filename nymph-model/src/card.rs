//! Card data models.

use std::str::FromStr;

use chrono::NaiveDateTime;

use derive_more::{Display, Error};

use serde::{Deserialize, Serialize};

use super::Id;

/// A single card.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Card {
    /// The unique identifier of the card.
    pub id: i32,
    /// The guild the card belongs to.
    pub guild_id: Id,
    /// The card's name.
    pub name: String,
    /// The card's category, if it belongs to a category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
    /// The card's visibility status.
    pub visibility: Visibility,
    /// The card's content in Markdown.
    pub content: String,
    /// Whether or not the card is usually hidden from the user.
    ///
    /// Only appears when the user has permission to view hidden cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// The card's upgrades.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrades: Option<Vec<Card>>,
    /// The card's downgrade.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downgrade: Option<Box<Card>>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Card visibility.
///
/// This determines how the card appears to users that do not own the card.
/// Users that own the card can see *all* information detailed by a card.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    /// The card, its existence and its details are unable to be seen by users
    /// that do not own the card.
    Private,
    /// The card's existence may be seen by users that do not own the card, but
    /// they cannot query for the card's information.
    Hidden,
    /// The card, its existence and its details are all able to be seen by
    /// users that do not own the card.
    Public,
}

impl Visibility {
    /// Creates a string representation of the visibility that can be used to
    /// get back the visibility with [`FromStr`].
    pub fn to_str(&self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Hidden => "hidden",
            Visibility::Public => "public",
        }
    }

    /// Checks if the visibility is [`Visibility::Public`].
    pub fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }
}

impl TryFrom<String> for Visibility {
    type Error = NoSuchVisibility;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for Visibility {
    type Error = NoSuchVisibility;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for Visibility {
    type Err = NoSuchVisibility;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "private" => Ok(Visibility::Private),
            "hidden" => Ok(Visibility::Hidden),
            "public" => Ok(Visibility::Public),
            _ => Err(NoSuchVisibility(s.to_string())),
        }
    }
}

#[derive(Clone, Debug, Display, Error)]
#[display("no such visibility \"{_0}\" exists")]
pub struct NoSuchVisibility(#[error(not(source))] String);
