//! Service authentication.

pub mod token;

pub use token::{Claims, ClaimsBuilder, Sub, TokenAuthentication};

use chrono::{DateTime, Utc};

use sqlx::FromRow;

use crate::app::{AppError, AppState};

async fn get_or_create_bot_user(state: &AppState, cname: impl AsRef<str>) -> Result<Sub, AppError> {
    // try to fetch user
    let id = sqlx::query_as::<_, (i32,)>(
        r#"
        SELECT id
        FROM user u, mtls_auth ma
        WHERE
            u.id = ma.user_id
            AND ma.common_name = $1
        "#,
    )
    .bind(cname.as_ref())
    .fetch_optional(&state.db)
    .await?;

    if let Some((id,)) = id {
        Ok(Sub::from(id))
    } else {
        #[derive(Debug, FromRow)]
        #[allow(dead_code)]
        struct User {
            id: i32,
            display_name: String,
            bot: bool,
            inserted_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let mut tx = state.db.begin().await?;

        let now = Utc::now();

        // create new user
        let new_user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO user (display_name, bot, inserted_at, updated_at)
            VALUES ($1, TRUE, $2, $2)
            RETURNING *
            "#,
        )
        .bind(cname.as_ref())
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        tracing::info!(
            ?new_user,
            "created user for mtls authenticated client `{}`",
            cname.as_ref()
        );

        // create mtls auth entry
        sqlx::query(
            r#"
            INSERT INTO mtls_auth (user_id, common_name)
            VALUES ($1, $2)
            "#,
        )
        .bind(new_user.id)
        .bind(cname.as_ref())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Sub::from(new_user.id))
    }
}
