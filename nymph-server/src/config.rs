//! Server configuration options.

use std::path::Path;

use anyhow::Error;

use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
    value::Uncased,
};
use serde::{Deserialize, Serialize};

/// The default port the server is hosted on.
pub const DEFAULT_PORT: u16 = 4000;

/// Server configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub server: ServerConfig,
}

impl Config {
    /// Reads the config from the environment.
    pub fn load(config_path: impl AsRef<Path>) -> Result<Config, Error> {
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file(config_path))
            .merge(Env::prefixed("NYMPH_"))
            .merge(
                Env::raw()
                    .only(&["DATABASE_URL", "PORT"])
                    .map(|k| Uncased::from(format!("SERVER.{}", k))),
            )
            .extract()
            .map_err(Error::from)
    }
}

/// Server config.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ServerConfig {
    /// The port the server is binded to.
    pub port: u16,
    /// The database url the server will connect to.
    #[serde(default)]
    pub database_url: Option<String>,
    /// The signing key used to sign JWTs.
    #[serde(default)]
    pub signing_key: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: DEFAULT_PORT,
            database_url: None,
            signing_key: None,
        }
    }
}
