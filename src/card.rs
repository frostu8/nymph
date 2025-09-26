//! Card functions.

use std::iter;

use textdistance::{Algorithm, Levenshtein};
use tracing::instrument;
use twilight_model::{
    application::interaction::Interaction,
    channel::message::{
        Component, MessageFlags,
        component::{ActionRow, Button, ButtonStyle},
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};
use twilight_util::builder::{
    InteractionResponseDataBuilder,
    message::{ContainerBuilder, TextDisplayBuilder},
};

use crate::{
    commands::Context,
    models::card::{self, Card},
};

/// Responds to an interaction with card information.
#[instrument(skip(cx, interaction))]
pub async fn show_card(cx: &Context, interaction: &Interaction, card: &Card) -> anyhow::Result<()> {
    let response = make_card(cx, interaction, card).await?;
    cx.client
        .interaction(cx.application_id)
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response),
            },
        )
        .await?;
    Ok(())
}

/// Responds to an interaction with card information by updating the original
/// message of the interaction.
#[instrument(skip(cx, interaction))]
pub async fn update_card(
    cx: &Context,
    interaction: &Interaction,
    card: &Card,
) -> anyhow::Result<()> {
    let response = make_card(cx, interaction, card).await?;
    cx.client
        .interaction(cx.application_id)
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::UpdateMessage,
                data: Some(response),
            },
        )
        .await?;
    Ok(())
}

/// Creates a response to an interaction requesting a card.
async fn make_card(
    cx: &Context,
    interaction: &Interaction,
    card: &Card,
) -> anyhow::Result<InteractionResponseData> {
    let Some(guild_id) = interaction.guild_id else {
        anyhow::bail!("missing guild id in interaction");
    };
    let category = card.category_name().and_then(|n| cx.config.category.get(n));
    let color = category.and_then(|c| c.color);

    // find associated cards
    let downgrade = card::get_downgrade_of(&cx.db, guild_id, card.id()).await?;
    let upgrade = card::get_upgrade_of(&cx.db, guild_id, card.id()).await?;

    // create the card action row
    let mut action_row = ActionRow {
        id: None,
        components: Vec::with_capacity(2),
    };

    // if we have found a downgrade or upgrade card, push the respective button
    // to the end of the components list
    if let Some(downgrade) = downgrade.as_ref() {
        action_row.components.push(Component::Button(Button {
            id: None,
            custom_id: Some(format!("update_with_card:{}", downgrade.id())),
            disabled: false,
            emoji: None,
            label: Some(String::from("--")),
            style: ButtonStyle::Danger,
            url: None,
            sku_id: None,
        }));
    }

    if let Some(upgrade) = upgrade.as_ref() {
        action_row.components.push(Component::Button(Button {
            id: None,
            custom_id: Some(format!("update_with_card:{}", upgrade.id())),
            disabled: false,
            emoji: None,
            label: Some(String::from("++")),
            style: ButtonStyle::Success,
            url: None,
            sku_id: None,
        }));
    }

    // append any category prefixes/suffixes to title
    let formatted_title = match category {
        Some(category) => match (category.prefix.as_ref(), category.suffix.as_ref()) {
            (Some(prefix), Some(suffix)) => format!("# {} `{}` {}", prefix, card.name(), suffix),
            (Some(prefix), None) => format!("# {} `{}`", prefix, card.name()),
            (None, Some(suffix)) => format!("# `{}` {}", card.name(), suffix),
            (None, None) => format!("# `{}`", card.name()),
        },
        None => format!("# `{}`", card.name()),
    };

    // build card body
    let body = format!("{}\n{}", formatted_title, card.content());

    //let timestamp =
    //    Timestamp::from_micros(card.updated_at().and_utc().timestamp_micros()).expect("valid time");

    let mut card_container = ContainerBuilder::new()
        .accent_color(color)
        .spoiler(false)
        .component(TextDisplayBuilder::new(body).build())
        .build();
    // add action row only if there are buttons to add
    if action_row.components.len() > 0 {
        card_container
            .components
            .push(Component::ActionRow(action_row));
    }

    // create response
    Ok(InteractionResponseDataBuilder::new()
        .components(iter::once(Component::Container(card_container)))
        .flags(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2)
        .build())
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

/// Responds to an interaction with an unauthorized error message.
pub async fn show_unauthorized(
    cx: &Context,
    interaction: &Interaction,
    name: impl AsRef<str>,
) -> anyhow::Result<()> {
    let accent = cx.config.accent.select_unauthorized();
    let message = format!(
        "-# {}\nThe card `{}` is hidden to you.",
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
