//! Endpoints to manage what cards a user owns.

use axum::{
    debug_handler,
    extract::{Path, State},
};

use nymph_model::{
    card::Card,
    request::card::inventory::{GrantRequest, ListInventoryQuery},
};

use sqlx::{Executor, Sqlite, sqlite::SqliteQueryResult};

use super::CardResult;

use crate::{
    app::{AppError, AppErrorKind, AppJson, AppQuery, AppState, Payload},
    auth::TokenAuthentication,
    routes::{Pagination, card::get_card},
};

/// Lists all cards belonging to a user.
#[debug_handler]
pub async fn list(
    Path((user_id,)): Path<(i32,)>,
    AppQuery(query): AppQuery<ListInventoryQuery>,
    State(state): State<AppState>,
    authorization: TokenAuthentication,
) -> Result<AppJson<Vec<Card>>, AppError> {
    // TODO: finer grained permissions
    // if this is the authorized user, they can always list their own cards
    if authorization.sub.get() == user_id {
        return Err(AppErrorKind::InsufficientPermissions.into());
    }

    let results = if let Some(guild_id) = query.guild_id {
        sqlx::query_as::<_, CardResult>(
            r#"
            SELECT
                c.id, c.guild_id, c.name, c.category_name, c.content,
                c.visibility, c.inserted_at, c.updated_at
            FROM
                card c, ownership o
            WHERE
                o.card_id = c.id
                AND o.owner_id = $1
                AND c.guild_id = $2
            "#,
        )
        .bind(authorization.sub.get())
        .bind(guild_id.get() as i64)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, CardResult>(
            r#"
            SELECT
                c.id, c.guild_id, c.name, c.category_name, c.content,
                c.visibility, c.inserted_at, c.updated_at
            FROM
                card c, ownership o
            WHERE
                o.card_id = c.id
                AND o.owner_id = $1
            "#,
        )
        .bind(authorization.sub.get())
        .fetch_all(&state.db)
        .await?
    };

    let results: Vec<_> = results.into_iter().map(|card| Card::from(card)).collect();

    // Paginate cards
    Ok(AppJson(
        Pagination::new(results)
            .limit(25)
            .paginate(query.page.unwrap_or(1), query.count.unwrap_or(25))?
            .to_owned(),
    ))
}

/// Adds a card to a user's inventory.
#[debug_handler]
pub async fn grant(
    Path((user_id,)): Path<(i32,)>,
    State(state): State<AppState>,
    authorization: TokenAuthentication,
    Payload(request): Payload<GrantRequest>,
) -> Result<AppJson<Card>, AppError> {
    // TODO: finer grained permissions
    if !authorization.proxy {
        return Err(AppErrorKind::InsufficientPermissions.into());
    }

    let res = update_ownership(&state.db, user_id, request.card_id, true).await?;
    let card = get_card(&state, request.card_id, &authorization).await?;

    if res.rows_affected() > 0 {
        Ok(AppJson(card))
    } else {
        Err(
            AppError::from(AppErrorKind::InvalidTransfer(card.name.to_owned())).with_message(
                format!(
                    "Card `{}` cannot be granted because user already owns that card.",
                    &card.name
                ),
            ),
        )
    }
}

/// Removes a card from a user's inventory.
#[debug_handler]
pub async fn revoke(
    Path((user_id, card_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    authorization: TokenAuthentication,
) -> Result<AppJson<Card>, AppError> {
    // TODO: finer grained permissions
    if !authorization.proxy {
        return Err(AppErrorKind::InsufficientPermissions.into());
    }

    update_ownership(&state.db, user_id, card_id, false).await?;

    // fetch card
    let res = update_ownership(&state.db, user_id, card_id, true).await?;
    let card = get_card(&state, card_id, &authorization).await?;

    if res.rows_affected() > 0 {
        Ok(AppJson(card))
    } else {
        Err(
            AppError::from(AppErrorKind::InvalidTransfer(card.name.to_owned())).with_message(
                format!(
                    "Card `{}` cannot be revoked because user does not own that card.",
                    &card.name
                ),
            ),
        )
    }
}

async fn update_ownership<'c, E>(
    db: E,
    owner_id: i32,
    card_id: i32,
    owned: bool,
) -> Result<SqliteQueryResult, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    sqlx::query(
        r#"
        INSERT INTO ownership (owner_id, card_id, owned)
        VALUES ($1, $2, $3)
        ON CONFLICT (owner_id, card_id) DO UPDATE
        SET owned = $3
        WHERE NOT owned = $3
        "#,
    )
    .bind(owner_id)
    .bind(card_id)
    .bind(owned)
    .execute(db)
    .await
}
