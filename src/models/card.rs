//! Card model.

use chrono::NaiveDateTime;

use sqlx::{Error, Executor, FromRow, Postgres};

use derive_more::Deref;

use twilight_model::id::{Id, marker::GuildMarker};

/// A single card.
#[derive(Clone, Debug, FromRow)]
pub struct Card {
    id: i32,
    guild_id: i64,
    name: String,
    category_name: Option<String>,
    content: String,
    public: bool,
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

    /// The name of the set the card belongs to.
    pub fn category_name(&self) -> Option<&str> {
        self.category_name.as_deref()
    }

    /// The description of the card.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The public visibility of the card.
    ///
    /// `true` means a user does not need to own a card to view it. `false` is
    /// default behavior.
    pub fn public(&self) -> bool {
        self.public
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

/// A single card with ownership details, returned by [`fetch`].
#[derive(Clone, Debug, Deref, FromRow)]
pub struct FetchResult {
    #[sqlx(flatten)]
    #[deref]
    pub card: Card,
    pub owned: bool,
}

/// Fetches a single card as a user.
pub async fn fetch<'e, E>(
    db: E,
    user_id: impl Into<u64>,
    guild_id: impl Into<u64>,
    name: impl AsRef<str>,
) -> Result<Option<FetchResult>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let guild_id = guild_id.into() as i64;
    let user_id = user_id.into() as i64;

    sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.guild_id,
            c.name,
            c.category_name,
            c.content,
            c.public,
            c.inserted_at,
            c.updated_at,
            COALESCE(o.owned, FALSE) AS owned
        FROM
            card AS c
        LEFT OUTER JOIN
            ownership AS o
            ON o.card_id = c.id AND o.owner_id = $2
        WHERE
            c.guild_id = $1 AND
            c.name = $3
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(name.as_ref())
    .fetch_optional(db)
    .await
}

/// Fetches a single card by name in a guild.
pub async fn get<'e, E>(
    db: E,
    guild_id: impl Into<u64>,
    name: impl AsRef<str>,
) -> Result<Option<Card>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let guild_id = guild_id.into() as i64;

    sqlx::query_as(
        r#"
        SELECT
            id, guild_id, name, category_name, content, public, inserted_at,
            updated_at
        FROM
            card
        WHERE
            guild_id = $1 AND
            name = $2
        "#,
    )
    .bind(guild_id)
    .bind(name.as_ref())
    .fetch_optional(db)
    .await
}

/// Fetches a single card by id.
pub async fn get_by_id<'e, E>(db: E, id: i32) -> Result<Card, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
        SELECT
            id, guild_id, name, category_name, content, public, inserted_at,
            updated_at
        FROM
            card
        WHERE
            id = $1
        "#,
    )
    .bind(id)
    .fetch_one(db)
    .await
}

/// Gets a list of cards by name, returning the names of each card.
pub async fn search<'e, E>(
    db: E,
    guild_id: impl Into<u64>,
    query: impl AsRef<str>,
) -> Result<Vec<String>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let guild_id = guild_id.into() as i64;

    sqlx::query_as::<_, (String,)>(
        "SELECT name FROM card WHERE guild_id = $1 AND name LIKE CONCAT('%', $2, '%')",
    )
    .bind(guild_id)
    .bind(query.as_ref())
    .fetch_all(db)
    .await
    .map(|result| result.into_iter().map(|(s,)| s).collect())
}

/// Fetches the upgrade of a card.
pub async fn get_upgrade_of<'e, E>(
    db: E,
    guild_id: impl Into<u64>,
    id: i32,
) -> Result<Option<Card>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let guild_id = guild_id.into() as i64;

    sqlx::query_as(
        r#"
        SELECT
            id, guild_id, name, category_name, content, inserted_at, updated_at
        FROM
            card
        WHERE
            guild_id = $1 AND
            previous_id = $2
        "#,
    )
    .bind(guild_id)
    .bind(id)
    .fetch_optional(db)
    .await
}

/// Fetches the downgrade of a card.
pub async fn get_downgrade_of<'e, E>(
    db: E,
    guild_id: impl Into<u64>,
    id: i32,
) -> Result<Option<Card>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let guild_id = guild_id.into() as i64;

    sqlx::query_as(
        r#"
        SELECT
            down.id,
            down.guild_id,
            down.name,
            down.category_name,
            down.content,
            down.inserted_at,
            down.updated_at
        FROM
            card AS down
        JOIN
            card AS up
            ON down.id = up.previous_id
        WHERE
            down.guild_id = $1 AND
            up.id = $2
        "#,
    )
    .bind(guild_id)
    .bind(id)
    .fetch_optional(db)
    .await
}
