//! Card functions.

use chrono::Local;
use twilight_model::{channel::message::Embed, util::Timestamp};
use twilight_util::builder::embed::EmbedBuilder;

use crate::{config::Config, models::card::Card};

/// Displays a card as an embed.
pub fn display_card(config: &Config, card: &Card) -> Embed {
    let category = card.category_name().and_then(|n| config.category.get(n));

    // append any set prefixes/suffixes
    let formatted_title = match category {
        Some(category) => match (category.prefix.as_ref(), category.suffix.as_ref()) {
            (Some(prefix), Some(suffix)) => format!("{} `{}` {}", prefix, card.name(), suffix),
            (Some(prefix), None) => format!("{} `{}`", prefix, card.name()),
            (None, Some(suffix)) => format!("`{}` {}", card.name(), suffix),
            (None, None) => format!("`{}`", card.name()),
        },
        None => format!("`{}`", card.name()),
    };

    let timestamp = Timestamp::from_micros(
        card.updated_at()
            .and_local_timezone(Local)
            .unwrap()
            .timestamp_micros(),
    )
    .expect("valid time");

    let mut embed = EmbedBuilder::new()
        .title(formatted_title)
        .description(card.content())
        .timestamp(timestamp);

    if let Some(color) = category.and_then(|c| c.color) {
        embed = embed.color(color);
    }

    embed.build()
}
