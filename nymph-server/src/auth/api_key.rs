//! API key authentication.

use axum::extract::{FromRef, FromRequestParts};

use http::{header::HeaderName, request::Parts};

use crate::app::{AppError, AppErrorKind, AppState};

use super::AuthenticatedUser;

use sha2::{Digest as _, Sha256};

use base16::encode_lower;

use rand::{
    Rng,
    distr::{Alphanumeric, SampleString},
};

pub const X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");

/// API key authentication.
#[derive(Clone, Debug)]
pub struct ApiKeyAuthentication {
    pub user: AuthenticatedUser,
}

impl<S> FromRequestParts<S> for ApiKeyAuthentication
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // if the result was cached, simply return the cached value
        if let Some(auth) = parts.extensions.get::<ApiKeyAuthentication>() {
            return Ok(auth.clone());
        }

        let key = parts
            .headers
            .get(X_API_KEY)
            .and_then(|s| s.to_str().ok())
            .map(|s| s.trim());

        if let Some(key) = key {
            let state = AppState::from_ref(state);

            // hash token
            let hash = hash_key(key);

            // search database for record
            let user = sqlx::query_as::<_, AuthenticatedUser>(
                r#"
                SELECT
                    u.id, u.display_name, u.managed
                FROM
                    user u, api_auth aa
                WHERE
                    u.id = aa.user_id
                    AND hash = $1
                "#,
            )
            .bind(hash)
            .fetch_optional(&state.db)
            .await?;

            match user {
                Some(user) => {
                    let auth = ApiKeyAuthentication { user };

                    // cache toe xtensions
                    parts.extensions.insert(auth.clone());

                    Ok(auth)
                }
                // api key matches nothing
                None => Err(AppErrorKind::Unauthenticated.into()),
            }
        } else {
            Err(AppErrorKind::Unauthenticated.into())
        }
    }
}

/// Generates a new API key.
pub fn generate_key() -> String {
    generate_key_with(&mut rand::rng())
}

/// Generates a new API key.
pub fn generate_key_with<R>(rng: &mut R) -> String
where
    R: Rng,
{
    Alphanumeric::default().sample_string(rng, 64)
}

/// Hashes an API key.
pub fn hash_key(key: impl AsRef<str>) -> String {
    let mut hasher = Sha256::new();

    hasher.update(key.as_ref());

    let result = hasher.finalize();

    encode_lower(&result)
}
