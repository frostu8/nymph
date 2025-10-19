//! Nymph API client.

use super::request::user::UpdateDiscordUser;

use anyhow::Error;

use std::num::NonZeroU64;
use std::sync::Arc;

use derive_more::{Deref, Display, Error};

use crate::config::ApiConfig;

use crate::http::request::card::inventory::GrantCard;
use crate::http::request::card::{GetCard, ListCards};

use moka::future::Cache;

use http::{HeaderName, HeaderValue, Method, header};

use nymph_model::{
    Error as ApiError, ErrorCode, response::user::UpdateDiscordUserResponse, user::User as DbUser,
};

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
#[derive(Clone, Debug)]
pub struct Client {
    http: reqwest::Client,
    state: Arc<ClientState>,
    user_cache: Cache<Id<UserMarker>, CachedUser>,
    proxy_for: Option<User>,
}

#[derive(Debug)]
struct ClientState {
    endpoint: String,
    api_key: String,
    token_refresh_retries: u32,
}

/// A cached user.
#[derive(Clone, Debug, Deref, PartialEq, Eq, Hash)]
pub struct CachedUser {
    /// The user.
    #[deref]
    pub user: DbUser,
    /// The access token of the user.
    pub access_token: Option<String>,
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
            token_refresh_retries: config.token_refresh_retries,
        };

        Ok(Client {
            http,
            state: Arc::new(state),
            user_cache: Cache::new(10_000),
            proxy_for: None,
        })
    }

    /// Gets a user, trying first from the cache, and then submitting a request
    /// to get them from the API.
    pub async fn get_discord_user(&self, user: &User) -> Result<DbUser, Error> {
        if let Some(user) = self.user_cache.get(&user.id).await {
            Ok(user.user.clone())
        } else {
            self.update_discord_user(user.id, &user.name)
                .execute()
                .await
                .map(|res| res.user.clone())
                .map_err(From::from)
        }
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

    /// Grants a card to a user.
    pub fn grant_card_to_user(&self, user_id: i32, card_id: i32) -> GrantCard {
        GrantCard::new(self.clone(), user_id, card_id)
    }

    /// Updates a Discord user's information.
    pub fn update_discord_user(
        &self,
        discord_id: Id<UserMarker>,
        display_name: impl Into<String>,
    ) -> UpdateDiscordUser {
        UpdateDiscordUser::new(self.clone(), discord_id, display_name.into())
    }

    /// Makes a generic request to the server.
    pub(super) fn request(&self, method: Method, url: impl AsRef<str>) -> Request {
        Request::new(self.clone(), method, url)
    }

    /// Updates the user cache with a result from the `/users/discord`
    /// endpoint.
    pub(super) async fn update_cache(&self, res: &UpdateDiscordUserResponse) {
        let discord_id = Id::<UserMarker>::from(NonZeroU64::from(res.discord_id));
        let cached_user = self.user_cache.get(&discord_id).await;

        let access_token = res
            .access_token
            .to_owned()
            .or(cached_user.and_then(|user| user.access_token));

        self.user_cache
            .insert(
                discord_id,
                CachedUser {
                    user: res.user.clone(),
                    access_token,
                },
            )
            .await;
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

    /// Makes a general request to the API as the bot.
    ///
    /// This bypasses any possible proxying.
    pub async fn send_privileged(self) -> Result<reqwest::Response, Error> {
        let mut request = self.request.build()?;

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

    /// Makes a general request to the API.
    pub async fn send(mut self) -> Result<reqwest::Response, Error> {
        let token_refresh_retries = self.client.state.token_refresh_retries;

        if self.client.proxy_for.is_some() {
            let mut request = self.request.build()?;
            let user = self.client.proxy_for.take().unwrap();

            for _ in 0..token_refresh_retries {
                // try to get bearer token
                let token = if let Some(token) = self
                    .client
                    .user_cache
                    .get(&user.id)
                    .await
                    .and_then(|user| user.access_token)
                {
                    token
                } else {
                    // fetch bearer token from internet
                    self.client
                        .update_discord_user(user.id, user.name.clone())
                        .generate_token(true)
                        .execute()
                        .await?
                        .access_token
                        .ok_or_else(|| Error::msg("server refused to give access token"))?
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
                        self.client.user_cache.invalidate(&user.id).await;
                    } else {
                        return Err(error.into());
                    }
                }
            }

            Err(TokenRefreshError.into())
        } else {
            self.send_privileged().await
        }
    }
}

/// A marker type for a client requesting as the bot.
#[derive(Debug)]
pub struct Bot;

/// A marker type for a client requesting as a proxy.
#[derive(Debug)]
pub struct Proxy;

/// The token failed to refresh.
#[derive(Debug, Display, Error)]
pub struct TokenRefreshError;
