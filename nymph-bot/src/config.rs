//! Bot configuration.

use std::{collections::HashMap, path::Path};

use figment::{
    Figment,
    providers::{Env, Format as _, Toml},
    value::Uncased,
};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Deserializer, de::Error as _};

/// The main configuration struct.
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    /// API access configuration.
    pub api: ApiConfig,
    /// Accent text configuration.
    pub accent: AccentTextConfig,
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
            .merge(Env::raw().only(&["DISCORD_TOKEN", "API_KEY"]).map(|k| {
                if k == "DISCORD_TOKEN" {
                    Uncased::from("GENERAL.DISCORD_TOKEN")
                } else if k == "API_KEY" {
                    Uncased::from("API.KEY")
                } else {
                    k.into()
                }
            }))
            .extract()
    }
}

/// General bot settings.
#[derive(Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    /// The token the bot uses.
    pub discord_token: String,
    /// The default color of embeds.
    #[serde(deserialize_with = "deser_hex_color")]
    pub embed_color: u32,
}

/// API connectivity config.
#[derive(Deserialize, Debug, Clone)]
pub struct ApiConfig {
    /// The API endpoint.
    pub endpoint: String,
    /// The API key
    pub key: String,
    /// How many times the bot should refresh.
    #[serde(default = "token_refresh_retries_default")]
    pub token_refresh_retries: u32,
}

fn token_refresh_retries_default() -> u32 {
    5
}

/// Configuration for accent text that appears in certain states or actions.
#[derive(Deserialize, Debug, Clone)]
pub struct AccentTextConfig {
    /// Accent text for when a user attempts to type /inv without owning any
    /// cards.
    pub no_cards_owned: String,
    /// The accent text displayed in the bizarre case an admin attempts to
    /// grant a card to the bot.
    pub self_grant: String,
    /// Accent text for when users attempt to show a card that doesn't exist.
    pub not_found: Vec<String>,
    /// Accent text for when users attempt to access a card they cannot access.
    pub unauthorized: Vec<String>,
}

impl AccentTextConfig {
    /// Selects a not found text.
    pub fn select_not_found(&self) -> &str {
        let mut rng = rand::rng();
        self.not_found.choose(&mut rng).expect("at least one line")
    }

    /// Selects an accent text displayed when a user attempts to view a card
    /// they are unable to access.
    pub fn select_unauthorized(&self) -> &str {
        let mut rng = rand::rng();
        self.unauthorized
            .choose(&mut rng)
            .expect("at least one line")
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
    #[serde(deserialize_with = "deser_hex_color_optional")]
    #[serde(default)]
    pub color: Option<u32>,
}

impl CategoryConfig {
    /// Formats the title of cards that belong to this category.
    pub fn format_title(&self, title: impl AsRef<str>) -> String {
        let title = title.as_ref();

        match (self.prefix.as_ref(), self.suffix.as_ref()) {
            (Some(prefix), Some(suffix)) => {
                format!("{} `{}` {}", prefix, title, suffix)
            }
            (Some(prefix), None) => format!("{} `{}`", prefix, title),
            (None, Some(suffix)) => format!("`{}` {}", title, suffix),
            (None, None) => format!("`{}`", title),
        }
    }
}

fn deser_hex_color<'de, D>(deser: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let color = String::deserialize(deser)?;
    let color = color.strip_prefix("#").unwrap_or(&color);
    u32::from_str_radix(color, 16).map_err(|e| D::Error::custom(e))
}

fn deser_hex_color_optional<'de, D>(deser: D) -> Result<Option<u32>, D::Error>
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
