//! Bot configuration.

use std::{collections::HashMap, path::Path};

use figment::{
    Figment,
    providers::{Env, Format as _, Toml},
};
use serde::{Deserialize, Deserializer, de::Error as _};

/// The main configuration struct.
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// The token the bot uses.
    pub discord_token: String,
    /// The database url the bot shall connect to.
    pub database_url: String,
    /// Contains set information.
    #[serde(default)]
    pub category: HashMap<String, CategoryConfig>,
}

impl Config {
    /// Loads a config from the environment and a given config path.
    pub fn load(config_path: impl AsRef<Path>) -> Result<Config, figment::Error> {
        Figment::new()
            .merge(Toml::file(config_path))
            .merge(Env::prefixed("NYMPH_"))
            .merge(Env::raw().only(&["DISCORD_TOKEN", "DATABASE_URL"]))
            .extract()
    }
}

/// Describes a set.
#[derive(Deserialize, Debug, Clone)]
pub struct CategoryConfig {
    /// Added to the beginning of the card's title.
    #[serde(default)]
    pub prefix: Option<String>,
    /// Added to the end of the card's title.
    #[serde(default)]
    pub suffix: Option<String>,
    /// Overrides the embed color.
    #[serde(deserialize_with = "deser_hex_color")]
    #[serde(default)]
    pub color: Option<u32>,
}

fn deser_hex_color<'de, D>(deser: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(color) = Option::<String>::deserialize(deser)? else {
        return Ok(None);
    };
    let color = color.strip_prefix("#").unwrap_or(&color);
    u32::from_str_radix(color, 16)
        .map(Some)
        .map_err(|e| D::Error::custom(e))
}
