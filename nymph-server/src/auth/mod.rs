//! Service authentication.

pub mod api_key;
pub mod token;

pub use api_key::ApiKeyAuthentication;
pub use token::{Claims, ClaimsBuilder, Sub, TokenAuthentication};

use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts},
};

use derive_more::Deref;

use http::request::Parts;

use sqlx::FromRow;

use crate::app::{AppError, AppErrorKind, AppState};

/// An authenticated user.
#[derive(Clone, Debug, FromRow)]
pub struct AuthenticatedUser {
    /// The ID of the authenticated user.
    pub id: i32,
    /// The user's display name.
    pub display_name: String,
    /// The user if they are managed.
    pub managed: bool,
}

/// Authentication guard.
///
/// This doesn't care how a user gets authenticated, just that they eventually
/// will be authenticated.
#[derive(Clone, Debug, Deref)]
pub struct Authentication(AuthenticatedUser);

impl<S> FromRequestParts<S> for Authentication
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let api_key = parts
            .extract_with_state::<ApiKeyAuthentication, S>(state)
            .await
            .map(|api_key| Authentication(api_key.user.clone()));

        match api_key {
            Ok(api_key) => Ok(api_key),
            Err(err) if matches!(err.kind(), AppErrorKind::Unauthenticated) => {
                // try token auth
                parts
                    .extract_with_state::<TokenAuthentication, S>(state)
                    .await
                    .map(|token| Authentication(token.user.clone()))
            }
            Err(err) => Err(err),
        }
    }
}
