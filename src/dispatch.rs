//! Interaction dispatch.

use std::iter;

use tracing::instrument;
use twilight_model::{
    application::{
        command::{CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType},
        interaction::{
            InteractionData, InteractionType,
            application_command::{CommandData, CommandOptionValue},
        },
    },
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{
    card::{display_card, sort_results},
    commands::Context,
    models::card,
};

/// Handles an interaction.
#[instrument(skip(cx))]
pub async fn interaction(cx: Context, mut interaction: Box<InteractionCreate>) {
    match interaction.kind {
        InteractionType::ApplicationCommandAutocomplete => {
            let data = interaction.0.data.take();
            let Some(InteractionData::ApplicationCommand(data)) = data else {
                tracing::error!("failed to get interaction payload");
                return;
            };

            if let Err(err) = autocomplete(cx, interaction, data).await {
                tracing::error!("{:?}", err);
            }
        }
        InteractionType::ApplicationCommand => {
            let data = interaction.0.data.take();
            let Some(InteractionData::ApplicationCommand(data)) = data else {
                tracing::error!("failed to get interaction payload");
                return;
            };

            if let Err(err) = slash_command(cx, interaction, data).await {
                tracing::error!("{}", err);
            }
        }
        // ignore other payloads
        _ => (),
    }
}

async fn slash_command(
    cx: Context,
    interaction: Box<InteractionCreate>,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        anyhow::bail!("missing guild id in interaction");
    };

    match data.name.as_str() {
        "show" | "s" => {
            let name = data
                .options
                .iter()
                .find(|option| option.name == "name")
                .and_then(|option| match option.value {
                    CommandOptionValue::String(ref value) => Some(value),
                    _ => None,
                });

            let Some(name) = name else {
                // invalid command payload
                anyhow::bail!("invalid command payload");
            };

            match card::get(&cx.db, guild_id, name).await? {
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
        }
        _ => (),
    }

    Ok(())
}

async fn autocomplete(
    cx: Context,
    interaction: Box<InteractionCreate>,
    data: Box<CommandData>,
) -> anyhow::Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        anyhow::bail!("missing guild id in interaction");
    };

    match data.name.as_str() {
        // run show autocomplete
        "show" | "s" => {
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
            let cards = card::search(&cx.db, guild_id, &name).await?;
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
        _ => (),
    }

    Ok(())
}
