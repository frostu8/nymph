//! Card functions.

use std::iter;

use chrono::Local;
use textdistance::{Algorithm, Levenshtein};
use twilight_model::{
    application::interaction::Interaction,
    channel::message::{Embed, MessageFlags},
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};
use twilight_util::builder::{InteractionResponseDataBuilder, embed::EmbedBuilder};

use crate::{
    commands::Context,
    config::Config,
    models::card::{self, Card},
};

/// Responds to an interaction with card information fetched by its `name`.
///
/// Does not do anything if the function returns an [`Err`].
pub async fn show_card(
    cx: &Context,
    interaction: &Interaction,
    name: impl AsRef<str>,
) -> anyhow::Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        anyhow::bail!("missing guild id in interaction");
    };

    let name = name.as_ref().to_uppercase();

    match card::get(&cx.db, guild_id, &name).await? {
        Some(card) => {
            let embed = display_card(&cx.config, &card);
            cx.client
                .interaction(cx.application_id)
                .create_response(
                    interaction.id,
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(
                            InteractionResponseDataBuilder::new()
                                .flags(MessageFlags::EPHEMERAL)
                                .embeds(iter::once(embed))
                                .build(),
                        ),
                    },
                )
                .await?;
        }
        None => {
            // Get a new not found message!
            let accent = cx.config.accent.select_not_found();
            let message = format!("-# {}\nThe card `{}` does not exist.", accent, name);

            cx.client
                .interaction(cx.application_id)
                .create_response(
                    interaction.id,
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(
                            InteractionResponseDataBuilder::new()
                                .flags(MessageFlags::EPHEMERAL)
                                .content(message)
                                .build(),
                        ),
                    },
                )
                .await?;
        }
    }

    Ok(())
}

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
