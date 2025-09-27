//! User model and inventories.

use sqlx::{
    Error, Executor,
    postgres::{PgQueryResult, Postgres},
};

/// Gives a card to a user.
pub async fn grant_card<'e, E>(
    db: E,
    user_id: impl Into<u64>,
    guild_id: impl Into<u64>,
    name: impl AsRef<str>,
) -> Result<PgQueryResult, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let user_id = user_id.into() as i64;
    let guild_id = guild_id.into() as i64;

    sqlx::query(
        r#"
        INSERT INTO
            ownership
            (owner_id, card_id, owned)
        SELECT
            $2 AS owner_id,
            c.id AS card_id,
            TRUE AS owned
        FROM
            card AS c
        WHERE
            c.guild_id = $1 AND
            c.name = $3
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(name.as_ref())
    .execute(db)
    .await
}
