//! Nymph API client.

use super::request::user::UserProxy;

use anyhow::Error;

use std::sync::Arc;

use derive_more::{Display, Error};

use crate::config::ApiConfig;

use crate::http::request::card::{GetCard, ListCards};

use dashmap::DashMap;

use http::{HeaderName, HeaderValue, Method, header};

use nymph_model::ErrorCode;
use nymph_model::{Error as ApiError, response::user::UserProxyResponse};

use serde::Serialize;
use twilight_model::id::marker::GuildMarker;
use twilight_model::{
    id::{Id, marker::UserMarker},
    user::User,
};

/// A client used to access the HTTP API.
///
/// Cheaply cloneable, as it uses an `Arc` to track internal state and manage
/// connections.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    state: Arc<ClientState>,
    token_refresh_retries: u32,
    proxy_for: Option<User>,
}

#[derive(Debug)]
struct ClientState {
    endpoint: String,
    api_key: String,
    token_store: DashMap<Id<UserMarker>, String>,
}

impl Client {
    /// Creates a new client.
    pub fn new(config: &ApiConfig) -> Result<Client, Error> {
        let http = reqwest::Client::builder()
            .use_rustls_tls()
            .deflate(true)
            .build()?;

        let state = ClientState {
            endpoint: config.endpoint.to_owned(),
            api_key: config.key.to_owned(),
            token_store: DashMap::new(),
        };

        Ok(Client {
            http,
            state: Arc::new(state),
            proxy_for: None,
            token_refresh_retries: config.token_refresh_retries,
        })
    }

    /// Proxies as a user.
    ///
    /// Creates a copy of the client that can be used to proxy for a user.
    pub fn proxy_for(&self, user: User) -> Client {
        Client {
            proxy_for: Some(user),
            ..self.clone()
        }
    }

    /// Gets a single card in a guild.
    pub fn get_card(&self, guild_id: Id<GuildMarker>, id: i32) -> GetCard {
        GetCard::new(self.clone(), guild_id, id)
    }

    /// Lists all avaialble cards in a guild.
    pub fn list_cards(&self, guild_id: Id<GuildMarker>) -> ListCards {
        ListCards::new(self.clone(), guild_id)
    }

    pub(super) fn request(&self, method: Method, url: impl AsRef<str>) -> Request {
        Request::new(self.clone(), method, url)
    }

    /// Creates a proxy for a discord user.
    pub(super) fn create_proxy(
        &self,
        discord_id: Id<UserMarker>,
        display_name: impl Into<String>,
    ) -> UserProxy {
        UserProxy::new(self.clone(), discord_id, display_name.into())
    }

    /// Sets up a proxy to be used in requests for a Discord user.
    ///
    /// Returns the resulting string if the proxy was successful.
    pub(super) async fn cache_proxy(&self, user: &User) -> Result<String, Error> {
        let display_name = user.name.to_string();

        let UserProxyResponse { token, .. } = self.create_proxy(user.id, display_name).await?;

        // cache token for later
        // make sure this is a String so SecUtf8 can take ownership of the buf
        self.state.token_store.insert(user.id, token.clone());

        Ok(token)
    }
}

/// A HTTP client request.
#[derive(Debug)]
pub struct Request {
    client: Client,
    request: reqwest::RequestBuilder,
}

impl Request {
    /// Creates a new `Request`.
    ///
    /// The url is appended to the API endpoint, and headers are set before
    /// sending the request.
    pub fn new(client: Client, method: Method, url: impl AsRef<str>) -> Request {
        let url = format!("{}{}", client.state.endpoint, url.as_ref());

        Request {
            request: client.http.request(method, url),
            client,
        }
    }

    /// Serrializes the body into the request.
    pub fn json<T>(self, json: &T) -> Request
    where
        T: Serialize + ?Sized,
    {
        Request {
            request: self.request.json(json),
            ..self
        }
    }

    /// Makes a general request to the API.
    pub async fn send(mut self) -> Result<reqwest::Response, Error> {
        let mut request = self.request.build()?;
        let use_proxy = !request.headers().contains_key(header::AUTHORIZATION);

        if use_proxy && self.client.proxy_for.is_some() {
            let user = self.client.proxy_for.take().unwrap();

            // TODO: magic number
            for _ in 0..self.client.token_refresh_retries {
                // try to get bearer token
                let token = if let Some(token) = self.client.state.token_store.get(&user.id) {
                    token.clone()
                } else {
                    // fetch bearer token from internet
                    self.client.cache_proxy(&user).await?
                };

                request.headers_mut().insert(
                    header::AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", token))?,
                );

                // request with token
                let res = self
                    .client
                    .http
                    .execute(request.try_clone().expect("cloneable request"))
                    .await?;

                if res.status().is_success() {
                    // short circuit with success value
                    return Ok(res);
                } else {
                    let error = res.json::<ApiError>().await?;

                    if error.code == ErrorCode::BadCredentials {
                        // retry request after getting new credentials
                        self.client.state.token_store.remove(&user.id);
                    } else {
                        return Err(error.into());
                    }
                }
            }

            Err(TokenRefreshError.into())
        } else {
            request.headers_mut().insert(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.client.state.api_key).expect("valid api key"),
            );

            let res = self.client.http.execute(request).await?;

            if res.status().is_success() {
                Ok(res)
            } else {
                Err(res.json::<ApiError>().await?.into())
            }
        }
    }
}

/// The token failed to refresh.
#[derive(Debug, Display, Error)]
pub struct TokenRefreshError;
