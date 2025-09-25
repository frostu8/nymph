//! Card functions.

use std::iter;

use textdistance::{Algorithm, Levenshtein};
use tracing::instrument;
use twilight_model::{
    application::interaction::Interaction,
    channel::message::{
        Component, Embed, MessageFlags,
        component::{ActionRow, Button, ButtonStyle},
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};
use twilight_util::builder::{InteractionResponseDataBuilder, embed::EmbedBuilder};

use crate::{
    commands::Context,
    config::Config,
    models::card::{self, Card},
};

/// Responds to an interaction with card information.
#[instrument(skip(cx, interaction))]
pub async fn show_card(cx: &Context, interaction: &Interaction, card: &Card) -> anyhow::Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        anyhow::bail!("missing guild id in interaction");
    };

    // find associated cards
    let downgrade = card::get_downgrade_of(&cx.db, guild_id, card.id()).await?;
    let upgrade = card::get_upgrade_of(&cx.db, guild_id, card.id()).await?;

    // create component list
    let mut components = Vec::with_capacity(2);

    if let Some(downgrade) = downgrade.as_ref() {
        components.push(Component::Button(Button {
            custom_id: Some(format!("show_card:{}", downgrade.id())),
            disabled: false,
            emoji: None,
            label: Some(String::from("--")),
            style: ButtonStyle::Danger,
            url: None,
            sku_id: None,
        }));
    }

    if let Some(upgrade) = upgrade.as_ref() {
        components.push(Component::Button(Button {
            custom_id: Some(format!("show_card:{}", upgrade.id())),
            disabled: false,
            emoji: None,
            label: Some(String::from("++")),
            style: ButtonStyle::Success,
            url: None,
            sku_id: None,
        }));
    }

    // create embed for card
    let embed = display_card(&cx.config, &card);

    // create response
    let response_data = InteractionResponseDataBuilder::new()
        .flags(MessageFlags::EPHEMERAL)
        .embeds(iter::once(embed));

    let response_data = if components.len() > 0 {
        response_data
            .components(iter::once(Component::ActionRow(ActionRow { components })))
            .build()
    } else {
        response_data.build()
    };

    cx.client
        .interaction(cx.application_id)
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response_data),
            },
        )
        .await?;
    Ok(())
}

/// Responds to an interaction with a not found error message.
pub async fn show_not_found(
    cx: &Context,
    interaction: &Interaction,
    name: impl AsRef<str>,
) -> anyhow::Result<()> {
    // Get a new not found message!
    let accent = cx.config.accent.select_not_found();
    let message = format!(
        "-# {}\nThe card `{}` does not exist.",
        accent,
        name.as_ref()
    );

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

    let timestamp =
        Timestamp::from_micros(card.updated_at().and_utc().timestamp_micros()).expect("valid time");

    let mut embed = EmbedBuilder::new()
        .title(formatted_title)
        .description(card.content())
        .timestamp(timestamp);

    if let Some(color) = category.and_then(|c| c.color) {
        embed = embed.color(color);
    }

    embed.build()
}
