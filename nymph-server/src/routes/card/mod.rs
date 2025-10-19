//! Card routes.

pub mod inventory;

use std::iter;

use axum::{
    debug_handler,
    extract::{Path, State},
};

use sqlx::FromRow;

use chrono::NaiveDateTime;

use nymph_model::{
    Id,
    card::{Card, Visibility},
    request::card::ListCardsQuery,
};

use textdistance::{Algorithm as _, Levenshtein};

use crate::{
    app::{AppError, AppErrorKind, AppJson, AppQuery, AppState},
    auth::Authentication,
    routes::Pagination,
};

#[derive(FromRow)]
struct CardResult {
    id: i32,
    guild_id: i64,
    name: String,
    category_name: Option<String>,
    #[sqlx(try_from = "String")]
    visibility: Visibility,
    content: String,
    owned: bool,
    inserted_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl From<CardResult> for Card {
    fn from(value: CardResult) -> Self {
        Card {
            id: value.id,
            // TODO: maybe not panic when getting arbitrary data?
            guild_id: Id::new(value.guild_id as u64).expect("valid id"),
            name: value.name,
            category_name: value.category_name,
            content: value.content,
            hidden: Some(!value.owned && value.visibility != Visibility::Public),
            visibility: value.visibility,
            upgrades: None,
            downgrade: None,
            created_at: value.inserted_at,
            updated_at: value.updated_at,
        }
    }
}

/// Lists all cards in a guilds with optional query params.
#[debug_handler]
pub async fn list(
    AppQuery(query): AppQuery<ListCardsQuery>,
    State(state): State<AppState>,
    Path((guild_id,)): Path<(i64,)>,
    auth: Authentication,
) -> Result<AppJson<Vec<Card>>, AppError> {
    let results = if let Some(search) = query.query.as_ref() {
        sqlx::query_as::<_, CardResult>(
            r#"
            SELECT
                c.id, c.guild_id, c.name, c.category_name, c.content,
                c.visibility, c.inserted_at, c.updated_at,
                COALESCE(o.owned, FALSE) AS owned
            FROM
                card c
            LEFT OUTER JOIN
                ownership AS o
                ON o.card_id = c.id AND o.owner_id = $1
            WHERE
                c.guild_id = $2
                AND c.name LIKE CONCAT('%', $3, '%')
            "#,
        )
        .bind(auth.id)
        .bind(guild_id)
        .bind(&search)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, CardResult>(
            r#"
            SELECT
                c.id, c.guild_id, c.name, c.category_name, c.content,
                c.visibility, c.inserted_at, c.updated_at,
                COALESCE(o.owned, FALSE) AS owned
            FROM
                card c
            LEFT OUTER JOIN
                ownership AS o
                ON o.card_id = c.id AND o.owner_id = $1
            WHERE
                c.guild_id = $2
            "#,
        )
        .bind(auth.id)
        .bind(guild_id)
        .fetch_all(&state.db)
        .await?
    };

    let results = results.into_iter().map(Card::from);

    // TODO: skip hidden results if the user doesn't have permissions

    let results: Vec<_> = if let Some(search) = query.query.as_ref() {
        sort_query_results(results, search).collect()
    } else {
        results.collect()
    };

    Ok(AppJson(
        Pagination::new(results)
            .limit(25)
            .paginate(query.page.unwrap_or(1), query.count.unwrap_or(25))?
            .to_owned(),
    ))
}

/// Gets a card by its ID.
#[debug_handler]
pub async fn show(
    State(state): State<AppState>,
    Path((guild_id, id)): Path<(i64, i32)>,
    auth: Authentication,
) -> Result<AppJson<Card>, AppError> {
    // fetch main card
    let card = sqlx::query_as::<_, CardResult>(
        r#"
        SELECT
            c.id, c.guild_id, c.name, c.category_name, c.content, c.visibility,
            c.inserted_at, c.updated_at, COALESCE(o.owned, FALSE) AS owned
        FROM
            card c
        LEFT OUTER JOIN
            ownership AS o
            ON o.card_id = c.id AND o.owner_id = $1
        WHERE
            c.id = $3
            AND c.guild_id = $2
        "#,
    )
    .bind(auth.id)
    .bind(guild_id)
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .map(Card::from);

    if let Some(card) = card {
        // TODO: reveal hidden cards if user has perms
        let hidden = card.hidden.unwrap_or_default();

        match card.visibility.into() {
            Visibility::Hidden if hidden => Err(AppErrorKind::Hidden(card.name).into()),
            Visibility::Private if hidden => Err(AppErrorKind::Forbidden.into()),
            // Public cards are always viewable
            _ => Ok(AppJson(
                preload_card(&state, &auth, Card::from(card)).await?,
            )),
        }
    } else {
        Err(AppError::from(AppErrorKind::NotFound)
            .with_message(format!("The card of id {} does not exist.", id)))
    }
}

/// Preloads card information from an already fetched card.
pub async fn preload_card(
    state: &AppState,
    auth: &Authentication,
    mut card: Card,
) -> Result<Card, AppError> {
    // Fetch all the upgrades for the card
    let upgrades = sqlx::query_as::<_, CardResult>(
        r#"
        SELECT
            c.id, c.guild_id, c.name, c.category_name, c.content,
            c.visibility, c.inserted_at, c.updated_at,
            COALESCE(o.owned, FALSE) AS owned
        FROM
            card c
        LEFT OUTER JOIN
            ownership AS o
            ON o.card_id = c.id AND o.owner_id = $1
        WHERE
            c.previous_id = $2
        "#,
    )
    .bind(auth.id)
    .bind(card.id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .filter(|card| card.owned || matches!(card.visibility.into(), Visibility::Public))
    .map(|card| Card::from(card))
    .collect::<Vec<_>>();

    // Fetch the downgrade for the card
    let downgrade = sqlx::query_as::<_, CardResult>(
        r#"
        SELECT
            down.id,
            down.guild_id,
            down.name,
            down.category_name,
            down.content,
            down.visibility,
            down.inserted_at,
            down.updated_at,
            COALESCE(o.owned, FALSE) AS owned
        FROM
            card down, card up
        LEFT OUTER JOIN
            ownership AS o
            ON o.card_id = down.id AND o.owner_id = $1
        WHERE
            down.id = up.previous_id
            AND up.id = $2
    "#,
    )
    .bind(auth.id)
    .bind(card.id)
    .fetch_optional(&state.db)
    .await?;

    // Apply to card
    if upgrades.len() > 0 {
        card.upgrades = Some(upgrades);
    }

    if let Some(downgrade) = downgrade {
        if downgrade.owned || matches!(downgrade.visibility.into(), Visibility::Public) {
            card.downgrade = Some(Box::new(Card::from(downgrade)));
        }
    }

    Ok(card)
}

/// Lower-level request handler given simply a card id.
pub async fn get_card(state: &AppState, id: i32, auth: &Authentication) -> Result<Card, AppError> {
    // fetch main card
    let card = sqlx::query_as::<_, CardResult>(
        r#"
        SELECT
            c.id, c.guild_id, c.name, c.category_name, c.content, c.visibility,
            c.inserted_at, c.updated_at, COALESCE(o.owned, FALSE) AS owned
        FROM
            card c
        LEFT OUTER JOIN
            ownership AS o
            ON o.card_id = c.id AND o.owner_id = $1
        WHERE
            c.id = $2
        "#,
    )
    .bind(auth.id)
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    match card {
        Some(card) => Ok(preload_card(state, auth, Card::from(card)).await?),
        None => Err(AppError::from(AppErrorKind::NotFound)
            .with_message(format!("The card of id {} does not exist.", id))),
    }
}

fn sort_query_results(
    cards: impl IntoIterator<Item = Card>,
    query: impl AsRef<str>,
) -> impl Iterator<Item = Card> {
    let query = query.as_ref();

    // results that start with the query are prioritized
    let mut exact_match = None;

    let mut top = Vec::new();
    let mut bottom = Vec::new();

    for card in cards {
        if card.name == query {
            exact_match = Some(card);
        } else if card.name.starts_with(query) {
            top.push(card);
        } else {
            bottom.push(card);
        }
    }

    // sort by lexicographic score
    let textdistance = Levenshtein::default();
    let sorter = |a: &Card, b: &Card| {
        let a = textdistance.for_str(&a.name, query).val();
        let b = textdistance.for_str(&b.name, query).val();
        a.cmp(&b)
    };

    top.sort_unstable_by(&sorter);
    bottom.sort_unstable_by(&sorter);

    iter::once(exact_match)
        .filter_map(std::convert::identity)
        .chain(top)
        .chain(bottom)
}
