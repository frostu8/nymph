//! User editing and authorization.

use crate::{
    app::{AppError, AppErrorKind, AppJson, AppState},
    auth::{Authentication, Claims},
};

use axum::{debug_handler, extract::State};

use chrono::{DateTime, TimeDelta, Utc};

use sqlx::{Acquire as _, FromRow};

use nymph_model::{request::user::UserProxyRequest, response::user::UserProxyResponse};

/// Updates user information, returning an access token so the bot can proxy.
///
/// **This is a secured endpoint.** Clients must use mTLS to access this
/// endpoint.
#[debug_handler]
pub async fn proxy_for(
    State(state): State<AppState>,
    auth: Authentication,
    AppJson(request): AppJson<UserProxyRequest>,
) -> Result<AppJson<UserProxyResponse>, AppError> {
    if !auth.managed {
        return Err(AppErrorKind::Forbidden.into());
    }

    #[derive(Debug, FromRow)]
    #[allow(dead_code)]
    struct User {
        id: i32,
        display_name: String,
        managed: bool,
        inserted_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let mut conn = state.db.acquire().await?;

    let now = Utc::now();

    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.id, u.display_name, u.managed, u.inserted_at, u.updated_at
        FROM user u, discord_auth da
        WHERE
            u.id = da.user_id
            AND da.discord_id = $1
        "#,
    )
    .bind(request.discord_id.get() as i64)
    .fetch_optional(&mut *conn)
    .await?;

    let user = match user {
        // check if we need to update the display name
        Some(user) if user.display_name != request.display_name => {
            tracing::info!(
                ?user,
                new = { &request.display_name },
                "proxy: updating stale display name",
            );

            sqlx::query(
                r#"
                UPDATE user
                SET display_name = $2, updated_at = $3
                WHERE id = $1
                "#,
            )
            .bind(user.id)
            .bind(&request.display_name)
            .bind(now)
            .execute(&mut *conn)
            .await?;

            user
        }
        Some(user) => user,
        // create a new user
        None => {
            let mut tx = conn.begin().await?;

            let user = sqlx::query_as::<_, User>(
                r#"
                INSERT INTO user (display_name, inserted_at, updated_at)
                VALUES ($1, $2, $2)
                RETURNING id, display_name, managed, inserted_at, updated_at
                "#,
            )
            .bind(&request.display_name)
            .bind(now)
            .fetch_one(&mut *tx)
            .await?;

            tracing::info!(?user, "proxy: creating new user");

            // create discord auth
            sqlx::query(
                r#"
                INSERT INTO discord_auth (user_id, discord_id, inserted_at)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(user.id)
            .bind(request.discord_id.get() as i64)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            user
        }
    };

    // create claims
    let claims = Claims::builder(user.id).exp(TimeDelta::minutes(15)).build();
    let token = claims.encode(&state.keys)?;

    Ok(AppJson(UserProxyResponse { token }))
}
