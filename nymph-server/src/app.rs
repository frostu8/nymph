//! Nymph general application items.

use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;

use anyhow::Error;

use axum::RequestExt as _;
use axum::extract::rejection::{FormRejection, JsonRejection, QueryRejection};
use axum::extract::{FromRequestParts, Request};
use axum::middleware::Next;
use axum::{
    Form, Json,
    extract::{FromRequest, Query},
    response::{IntoResponse, Response},
};

use http::{HeaderValue, StatusCode, header};

use nymph_model::{Error as ApiError, ErrorCode};

use serde::de::DeserializeOwned;
use sqlx::{SqlitePool, pool::PoolOptions};

use derive_more::{Deref, Display, From};

use jsonwebtoken::{
    DecodingKey, EncodingKey,
    errors::{Error as JwtError, ErrorKind as JwtErrorKind},
};

use rand::{Rng as _, SeedableRng as _, rngs::StdRng};

use base16::encode_lower;

use crate::config::ServerConfig;

/// Shared server state.
///
/// Cheaply cloneable.
#[derive(Clone)]
pub struct AppState {
    /// The port the server is binded to.
    pub port: u16,
    /// A database connection pool.
    pub db: SqlitePool,
    /// The secret signing keys for tokens.
    ///
    /// This is randomly generated on app startup. This means that when the
    /// daemon restarts, old JWTs will be rejected.
    pub keys: Arc<SigningKeys>,
}

impl AppState {
    /// Creates a new `AppState`.
    ///
    /// See [`Config`] to learn more on what the options do.
    pub async fn new(config: ServerConfig) -> Result<AppState, Error> {
        let ServerConfig { port, .. } = config;

        // get url
        let Some(database_url) = config.database_url.as_ref() else {
            return Err(Error::msg("`DATABASE_URL` not present"));
        };

        // establish database connection
        let pool = PoolOptions::new().connect(database_url).await?;

        // randomly generate JWT secret
        let keys = match config.signing_key.as_ref() {
            Some(key) => Arc::from(SigningKeys::new(key)?),
            None => Arc::from(SigningKeys::new_random()),
        };

        Ok(AppState {
            port,
            db: pool,
            keys,
        })
    }
}

impl Debug for AppState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerState")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

/// App REST headers.
pub async fn app_rest_headers(request: Request, next: Next) -> Response {
    let mut res = next.run(request).await;

    //let hsts_time = 60 * 60 * 24;

    // apply additional headers for REST safety
    res.headers_mut().extend([
        (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        (
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("frame-ancestors 'none'"),
        ),
        (
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
        (header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")),
        // (
        //     header::STRICT_TRANSPORT_SECURITY,
        //     HeaderValue::try_from(format!("max-age={}", hsts_time)).expect("valid hsts time"),
        // ),
    ]);

    res
}

/// Selective body extractor.
#[derive(Deref)]
pub struct Payload<T>(pub T);

impl<S, T> FromRequest<S> for Payload<T>
where
    T: DeserializeOwned + 'static,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // switch on content type
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppErrorKind::MissingContentType)?;

        match content_type {
            "application/x-www-form-urlencoded" => {
                let AppForm(form) = req.extract_with_state::<AppForm<T>, _, _>(state).await?;
                Ok(Payload(form))
            }
            "application/json" => {
                let AppJson(json) = req.extract_with_state::<AppJson<T>, _, _>(state).await?;
                Ok(Payload(json))
            }
            mime => Err(AppErrorKind::UnsupportedContentType(mime.to_owned()).into()),
        }
    }
}

/// App Query extractor.
#[derive(Deref, FromRequestParts)]
#[from_request(via(Query), rejection(AppError))]
pub struct AppQuery<T>(pub T);

/// App Form extractor and responder.
#[derive(Deref, FromRequest)]
#[from_request(via(Form), rejection(AppError))]
pub struct AppForm<T>(pub T);

impl<T> IntoResponse for AppForm<T>
where
    Form<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        Form(self.0).into_response()
    }
}

/// App JSON extractor and responder.
#[derive(Deref, FromRequest)]
#[from_request(via(Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

impl<T> IntoResponse for AppJson<T>
where
    Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}

/// An app error.
#[derive(Debug)]
pub struct AppError {
    kind: AppErrorKind,
    /// An optional override message.
    message: Option<String>,
}

impl AppError {
    /// Checks if an error is internal.
    pub fn is_internal(&self) -> bool {
        self.kind.is_internal()
    }

    /// Attachs an override message to the error.
    pub fn with_message(self, message: impl Into<String>) -> AppError {
        AppError {
            message: Some(message.into()),
            ..self
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(message) = self.message.as_ref() {
            f.write_str(message)
        } else {
            Display::fmt(&self.kind, f)
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            AppErrorKind::Json(err) => Some(err),
            AppErrorKind::InvalidAuthorization(err) => Some(err),
            AppErrorKind::Database(err) => Some(err),
            _ => None,
        }
    }
}

impl<T> From<T> for AppError
where
    AppErrorKind: From<T>,
{
    fn from(value: T) -> Self {
        AppError {
            kind: AppErrorKind::from(value),
            message: None,
        }
    }
}

#[derive(Debug, Display, From)]
pub enum AppErrorKind {
    /// The request's query params were malformed or unexpected.
    #[display("{_0}")]
    Query(QueryRejection),
    /// The request's urlencoded body was malformed or unexpected.
    #[display("{_0}")]
    Form(FormRejection),
    /// The request's JSON body was malformed or unexpected.
    #[display("{_0}")]
    Json(JsonRejection),
    /// A data field's value is out of range.
    #[from(ignore)]
    FieldOutOfRange(String),
    /// The card cannot be added to the user's inventory because they already
    /// own the card, or the card cannot be removed from the user's inventory
    /// because they do not have the card.
    #[display("Card `{_0}` cannot be transferred.`")]
    #[from(ignore)]
    InvalidTransfer(String),
    /// A request sent a payload without a MIME type.
    MissingContentType,
    /// A request sent a payload with a MIME type the server refused to serve.
    #[from(ignore)]
    UnsupportedContentType(String),
    /// The resource wasn't found.
    #[from(ignore)]
    #[display("Resource not found")]
    NotFound,
    /// The resource is forbidden to the authenticator.
    #[display("The resource is forbidden")]
    Forbidden,
    /// The card cannot be viewed.
    #[from(ignore)]
    #[display("The card `{_0}` is hidden")]
    Hidden(String),
    /// The user does not have the right permissions to access or update this
    /// resource.
    #[display("Not enough permissions")]
    InsufficientPermissions,
    /// Authorization is invalid.
    #[display("{_0}")]
    InvalidAuthorization(JwtError),
    /// Authorization is missing.
    #[display("Request unauthenticated")]
    Unauthorized,
    /// Missing mTLS certificate for secured route.
    #[display("Missing mTLS certificate for secured route")]
    MissingCertificate,
    /// Missing or invalid common name in the certificate.
    #[display("Unexpected or missing common name")]
    InvalidCommonName,
    /// An internal database error happened that was unhandled.
    #[display("{_0}")]
    Database(sqlx::Error),
}

impl AppErrorKind {
    /// Checks if an error is internal.
    pub fn is_internal(&self) -> bool {
        matches!(
            self,
            AppErrorKind::Database(_)
                | AppErrorKind::Json(JsonRejection::BytesRejection(_))
                | AppErrorKind::Form(FormRejection::BytesRejection(_))
        )
    }
}

impl IntoResponse for AppError {
    fn into_response(mut self) -> Response {
        let (status, mut error, internal_error) = match self.kind {
            // QUERY errors
            AppErrorKind::Query(QueryRejection::FailedToDeserializeQueryString(error)) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::InvalidData,
                    message: error.to_string(),
                },
                None,
            ),
            // FORM errors
            AppErrorKind::Form(FormRejection::FailedToDeserializeForm(error)) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::InvalidData,
                    message: error.to_string(),
                },
                None,
            ),
            AppErrorKind::Form(FormRejection::FailedToDeserializeFormBody(error)) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::InvalidData,
                    message: error.to_string(),
                },
                None,
            ),
            AppErrorKind::Form(FormRejection::InvalidFormContentType(_)) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::UnsupportedContentType,
                    message: "No supported content type.".into(),
                },
                None,
            ),
            // JSON errors
            AppErrorKind::Json(JsonRejection::JsonDataError(error)) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::InvalidData,
                    message: error.to_string(),
                },
                None,
            ),
            AppErrorKind::Json(JsonRejection::JsonSyntaxError(error)) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::MalformedJson,
                    message: error.to_string(),
                },
                None,
            ),
            AppErrorKind::Json(JsonRejection::MissingJsonContentType(_)) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::UnsupportedContentType,
                    message: "No supported content type.".into(),
                },
                None,
            ),
            // Card management errors
            AppErrorKind::InvalidTransfer(name) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::AlreadyOwned,
                    message: format!("Card `{}` cannot be transferred.", name),
                },
                None,
            ),
            // Other request errors
            AppErrorKind::FieldOutOfRange(name) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::InvalidData,
                    message: format!("Field `{}`'s value is out of range.", name),
                },
                None,
            ),
            AppErrorKind::UnsupportedContentType(mime) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::NotFound,
                    message: format!("Unrecognized MIME type: {}.", mime),
                },
                None,
            ),
            AppErrorKind::MissingContentType => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    code: ErrorCode::NotFound,
                    message: "Missing request content type.".into(),
                },
                None,
            ),
            AppErrorKind::NotFound => (
                StatusCode::NOT_FOUND,
                ApiError {
                    code: ErrorCode::NotFound,
                    message: "The resource was not found.".into(),
                },
                None,
            ),
            AppErrorKind::Forbidden => (
                StatusCode::FORBIDDEN,
                ApiError {
                    code: ErrorCode::Forbidden,
                    message: "This resource is forbidden.".into(),
                },
                None,
            ),
            AppErrorKind::Hidden(card_name) => (
                StatusCode::FORBIDDEN,
                ApiError {
                    code: ErrorCode::Hidden,
                    message: format!("The card `{}` is hidden to you.", card_name),
                },
                None,
            ),
            AppErrorKind::InsufficientPermissions => (
                StatusCode::FORBIDDEN,
                ApiError {
                    code: ErrorCode::InsufficientPermissions,
                    message: "You don't have the permissions to do this.".into(),
                },
                None,
            ),
            AppErrorKind::InvalidAuthorization(err) => (
                StatusCode::UNAUTHORIZED,
                ApiError {
                    code: ErrorCode::BadCredentials,
                    message: if matches!(
                        err.kind(),
                        JwtErrorKind::ExpiredSignature | JwtErrorKind::InvalidSignature
                    ) {
                        "User credentials have expired.".into()
                    } else {
                        "Access token verification failed.".into()
                    },
                },
                None,
            ),
            AppErrorKind::Unauthorized
            | AppErrorKind::MissingCertificate
            | AppErrorKind::InvalidCommonName => (
                StatusCode::UNAUTHORIZED,
                ApiError {
                    code: ErrorCode::Unauthorized,
                    message: "Request is unauthorized.".into(),
                },
                None,
            ),
            // create a generic internal error
            error_kind => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError {
                    code: ErrorCode::InternalServerError,
                    message: "An internal server error occured.".into(),
                },
                Some(AppError {
                    kind: error_kind,
                    message: self.message.take(),
                }),
            ),
        };

        if let Some(message) = self.message {
            error.message = message;
        }

        let mut response = (status, AppJson(error)).into_response();
        if let Some(error) = internal_error {
            response.extensions_mut().insert(Arc::new(error));
        }
        response
    }
}

/// Signing keys.
#[derive(Clone)]
pub struct SigningKeys {
    /// The encoding key.
    pub encoding: EncodingKey,
    /// The decoding key.
    pub decoding: DecodingKey,
    is_random: bool,
}

impl SigningKeys {
    /// Creates a new set of `SigningKeys` from a base64 secret.
    pub fn new(secret: impl Into<String>) -> Result<SigningKeys, JwtError> {
        let secret = secret.into();

        Ok(SigningKeys {
            encoding: EncodingKey::from_base64_secret(&secret)?,
            decoding: DecodingKey::from_base64_secret(&secret)?,
            is_random: false,
        })
    }

    /// Creates a new set of random `SigningKeys`.
    pub fn new_random() -> SigningKeys {
        let secret = random_signing_key();
        let keys = SigningKeys::new(secret).expect("valid format HMAC keys");

        SigningKeys {
            is_random: true,
            ..keys
        }
    }

    /// If the keys were randomly generated at runtime.
    pub fn is_random(&self) -> bool {
        self.is_random
    }
}

impl Debug for SigningKeys {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SigningKeys").finish_non_exhaustive()
    }
}

/// Creates a random HMAC signing key and returns it as a [`String`]
pub fn random_signing_key() -> String {
    let mut rng = StdRng::from_os_rng();
    let mut bytes = [0u8; 256];
    rng.fill(&mut bytes);

    encode_lower(&bytes)
}
