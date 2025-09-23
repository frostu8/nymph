//! Card model.

use chrono::NaiveDateTime;

use sqlx::{Error, Executor, FromRow, Postgres};
use twilight_model::id::{Id, marker::GuildMarker};

/// A single card.
#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "PascalCase")]
pub struct Card {
    id: i32,
    guild_id: i64,
    name: String,
    content: String,
    inserted_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl Card {
    /// The id of the card.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The guild id the card belongs to.
    pub fn guild_id(&self) -> Id<GuildMarker> {
        u64::try_from(self.guild_id)
            .ok()
            .and_then(|id| Id::new_checked(id))
            .expect("id out of bounds")
    }

    /// The name of the card.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The description of the card.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The timestamp of when the card was created.
    pub fn inserted_at(&self) -> NaiveDateTime {
        self.inserted_at
    }

    /// The timestamp of when the card was last updated.
    pub fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
}

/// Gets a list of cards by name, returning the names of each card.
pub async fn search<'e, E>(db: E, query: &str) -> Result<Vec<String>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, (String,)>("SELECT Name FROM Card WHERE Name LIKE CONCAT('%', $1, '%')")
        .bind(query)
        .fetch_all(db)
        .await
        .map(|result| result.into_iter().map(|(s,)| s).collect())
}
