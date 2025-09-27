//! Interaction dispatch.

use anyhow::{Context as _, Error};

use tracing::instrument;

use twilight_model::{
    application::{
        command::{CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType},
        interaction::{
            Interaction, InteractionData, InteractionType,
            application_command::{CommandData, CommandOptionValue},
            message_component::MessageComponentInteractionData,
        },
    },
    channel::message::{AllowedMentions, MessageFlags},
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{
    card::{show_card, show_not_found, show_unauthorized, sort_results, update_card},
    commands::Context,
    models::{card, user},
};

/// Handles an interaction.
#[instrument(skip(cx, interaction))]
pub async fn interaction(cx: Context, mut interaction: Box<InteractionCreate>) {
    match interaction.kind {
        InteractionType::ApplicationCommandAutocomplete => {
            let data = interaction.0.data.take();
            let Some(InteractionData::ApplicationCommand(data)) = data else {
                tracing::error!("failed to get interaction payload");
                return;
            };

            if let Err(err) = autocomplete(cx, &interaction, data).await {
                tracing::error!(?interaction, "{:?}", err);
            }
        }
        InteractionType::ApplicationCommand => {
            let data = interaction.0.data.take();
            let Some(InteractionData::ApplicationCommand(data)) = data else {
                tracing::error!("failed to get interaction payload");
                return;
            };

            if let Err(err) = slash_command(cx, &interaction, data).await {
                tracing::error!(?interaction, "{:?}", err);
            }
        }
        InteractionType::MessageComponent => {
            let data = interaction.0.data.take();
            let Some(InteractionData::MessageComponent(data)) = data else {
                tracing::error!("failed to get interaction payload");
                return;
            };

            if let Err(err) = message_component(cx, &interaction, data).await {
                tracing::error!(?interaction, "{:?}", err);
            }
        }
        // ignore other payloads
        _ => (),
    }
}

async fn slash_command(
    cx: Context,
    interaction: &Interaction,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        anyhow::bail!("missing guild id in interaction");
    };
    let Some(user_id) = interaction
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref().map(|u| u.id))
    else {
        anyhow::bail!("missing user id in interaciton");
    };

    match data.name.as_str() {
        "s" => {
            let name = data
                .options
                .iter()
                .find(|option| option.name == "name")
                .and_then(|option| match option.value {
                    CommandOptionValue::String(ref value) => Some(value),
                    _ => None,
                })
                .ok_or_else(|| Error::msg("invalid command payload"))?;
            let name = name.to_uppercase();

            match card::fetch(&cx.db, user_id, guild_id, &name).await? {
                Some(card) if card.public() || card.owned => {
                    show_card(&cx, &interaction, &card).await?;
                }
                Some(_card) => show_unauthorized(&cx, &interaction, &name).await?,
                None => show_not_found(&cx, &interaction, &name).await?,
            }
        }
        "grant" => {
            let name = data
                .options
                .iter()
                .find(|option| option.name == "name")
                .and_then(|option| match option.value {
                    CommandOptionValue::String(ref value) => Some(value),
                    _ => None,
                })
                .ok_or_else(|| Error::msg("invalid command payload"))?;
            let name = name.to_uppercase();

            let target_user_id = data
                .options
                .iter()
                .find(|option| option.name == "user")
                .and_then(|option| match option.value {
                    CommandOptionValue::User(id) => Some(id),
                    _ => None,
                })
                .ok_or_else(|| Error::msg("invalid command payload"))?;
            let target_user = cx.cache.user(target_user_id).expect("cached user");

            // disallow granting cards to bots!!!
            if target_user.bot {
                let is_current_bot = cx
                    .cache
                    .current_user()
                    .map(|current_user| current_user.id == target_user.id)
                    .unwrap_or(false);

                let message = if is_current_bot {
                    format!("-# {}", cx.config.accent.self_grant)
                } else {
                    format!(
                        "User <@{}> is a bot. Unfortunately, automatons do not have the higher thought required to appreciate game design.",
                        target_user.id
                    )
                };

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
                                    .allowed_mentions(AllowedMentions::default())
                                    .build(),
                            ),
                        },
                    )
                    .await?;
                return Ok(()); // bail early
            }

            let res = user::grant_card(&cx.db, target_user_id, guild_id, &name).await?;
            if res.rows_affected() > 0 {
                // the operation was successful
                let message = format!("Gave card `{}` to user <@{}>!", name, target_user.id);

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
                                    .allowed_mentions(AllowedMentions::default())
                                    .build(),
                            ),
                        },
                    )
                    .await?;
            } else {
                show_not_found(&cx, &interaction, &name).await?;
            }
        }
        _ => tracing::warn!(?interaction, "unknown interaction"),
    }

    Ok(())
}

async fn autocomplete(
    cx: Context,
    interaction: &Interaction,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        anyhow::bail!("missing guild id in interaction");
    };
    let Some(user_id) = interaction
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref().map(|u| u.id))
    else {
        anyhow::bail!("missing user id in interaciton");
    };

    match data.name.as_str() {
        "s" | "grant" => {
            let name = data
                .options
                .iter()
                .find(|option| option.name == "name")
                .and_then(|option| match option.value {
                    CommandOptionValue::Focused(ref value, CommandOptionType::String) => {
                        Some(value)
                    }
                    _ => None,
                });

            let Some(name) = name else {
                // invalid command payload
                anyhow::bail!("invalid command payload");
            };

            // make search query uppercase
            let name = name.to_ascii_uppercase();

            // get cards with name
            // search with administrator permissions if using grant
            let cards = if data.name.as_str() == "grant" {
                card::search(&cx.db, guild_id, &name).await?
            } else {
                card::search_visible(&cx.db, user_id, guild_id, &name).await?
            };
            let cards = sort_results(cards, &name, 8);

            // map into choices
            let choices = cards.into_iter().map(|name| CommandOptionChoice {
                name_localizations: None,
                value: CommandOptionChoiceValue::String(name.clone()),
                name,
            });

            cx.client
                .interaction(cx.application_id)
                .create_response(
                    interaction.id,
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
                        data: Some(
                            InteractionResponseDataBuilder::new()
                                .choices(choices)
                                .build(),
                        ),
                    },
                )
                .await?;
        }
        _ => tracing::warn!(?interaction, "unknown interaction"),
    }

    Ok(())
}

async fn message_component(
    cx: Context,
    interaction: &Interaction,
    data: Box<MessageComponentInteractionData>,
) -> anyhow::Result<()> {
    // Currently, the only interactable components this bot attaches are simple
    // buttons that show cards.
    let custom_id = data.custom_id.as_str();

    if let Some(card_id) = custom_id.strip_prefix("update_with_card:") {
        // this is a show card button!
        // parse the card id
        let card_id = card_id.parse::<i32>().context("malformed card id")?;

        // fetch card
        let card = card::get_by_id(&cx.db, card_id).await?;
        update_card(&cx, &interaction, &card).await?;
    }

    Ok(())
}
