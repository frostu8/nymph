//! Card functions.

use twilight_model::{channel::message::Embed, util::Timestamp};
use twilight_util::builder::embed::EmbedBuilder;

use crate::models::card::Card;

/// Displays a card as an embed.
pub fn display_card(card: &Card) -> Embed {
    let formatted_title = format!("`{}`", card.name());
    let timestamp =
        Timestamp::from_micros(card.updated_at().and_utc().timestamp_micros()).expect("valid time");

    EmbedBuilder::new()
        .title(formatted_title)
        .description(card.content())
        .timestamp(timestamp)
        .build()
}
