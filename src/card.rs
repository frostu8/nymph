//! Card functions.

use chrono::Local;
use textdistance::{Algorithm, Levenshtein};
use twilight_model::{channel::message::Embed, util::Timestamp};
use twilight_util::builder::embed::EmbedBuilder;

use crate::{config::Config, models::card::Card};

/// Sorts the results of a card search.
pub fn sort_results(
    cards: impl IntoIterator<Item = String>,
    query: impl AsRef<str>,
    limit: usize,
) -> Vec<String> {
    let query = query.as_ref();

    // results that start with the query are prioritized
    let mut top = Vec::new();
    let mut bottom = Vec::new();

    for card in cards {
        if card.starts_with(query) {
            top.push(card);
        } else {
            bottom.push(card);
        }
    }

    // sort by lexicographic score
    let textdistance = Levenshtein::default();
    let sorter = |a: &String, b: &String| {
        let a = textdistance.for_str(a, query).val();
        let b = textdistance.for_str(b, query).val();
        a.cmp(&b)
    };

    top.sort_unstable_by(&sorter);
    bottom.sort_unstable_by(&sorter);

    // combine and limit
    top.into_iter().chain(bottom).take(limit).collect()
}

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
