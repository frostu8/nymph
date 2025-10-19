//! Interaction dispatch.

use tracing::instrument;

use twilight_model::application::interaction::{
    Interaction, InteractionData, InteractionType, application_command::CommandData,
    message_component::MessageComponentInteractionData,
};

use crate::commands::InteractionContext;

/// The limit for autocomplete entries.
pub const AUTOCOMPLETE_ENTRY_LEN: usize = 25;

/// Handles an interaction.
#[instrument(skip(cx))]
pub async fn interaction(mut cx: InteractionContext) {
    match cx.kind {
        InteractionType::ApplicationCommand => {
            let data = cx.interaction.data.take();
            let Some(InteractionData::ApplicationCommand(data)) = data else {
                tracing::error!("failed to get interaction payload");
                return;
            };

            if let Err(err) = slash_command(cx, *data).await {
                for err in err.chain() {
                    tracing::error!("{:?}", err);
                }
            }
        }
        InteractionType::ApplicationCommandAutocomplete => {
            let data = cx.interaction.data.take();
            let Some(InteractionData::ApplicationCommand(data)) = data else {
                tracing::error!("failed to get interaction payload");
                return;
            };

            if let Err(err) = autocomplete(cx, *data).await {
                for err in err.chain() {
                    tracing::error!("{:?}", err);
                }
            }
        }
        /*
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
        */
        // ignore other payloads
        _ => (),
    }
}

async fn slash_command(cx: InteractionContext, data: CommandData) -> anyhow::Result<()> {
    match data.name.as_str() {
        "s" => crate::card::command_show(cx, data).await?,
        "grant" | "revoke" => crate::card::command_transfer_card(cx, data).await?,
        /*
                "sl" => {
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

                    // fetch card from client
                    let card = cx
                        .db_client
                        .list_cards(guild_id)
                        .find(&name)
                        .show_hidden(true)
                        .await?
                        .into_iter()
                        // only find exact matches
        query_as::<_, CardResult>(
                r#"
                .find(|card| card.name == name);

            match card {
                Some(card) => show_card_editor(&cx, &interaction, &card).await?,
                None => show_not_found(&cx, &interaction, &name).await?,
            }
        }
        "inv" => {
            let cards = card::get_owned(&cx.db, user_id, guild_id).await?;

            if cards.len() > 0 {
                show_card_list(&cx, &interaction, cards.into_iter().as_ref()).await?;
            } else {
                let message = format!(
                    "-# {}\nYou do not have any cards.",
                    cx.config.accent.no_cards_owned
                );

                cx.client
                    .interaction(cx.application_id)
                    .create_response(
                        interaction.id,dca06d299e4eb237905ba3f49b5d6012f41819581caea529bd4435cc4b4cf6d6898c2c2e82a667abca515822db9d7aff749b65f35a7d80756baed0418c31e375dec4627dd70f627d7e587be38bd076c511fd40581878c28b75e96a79b7442c1d95293d31e506f318299c5170844dc99c2fc774db1e80fe943db0200758a180921242b5a7f362b5de2ff0654d926956232c3699fac91e0b31ef883d0aa547166d5a292c8f2baddb9387f35950d625ec0dea109356c21f07754ee2d76d61ccfc1d74a6871efaf112589c1bfdd01dfb85912d8bc623ec21958b7a8a6e23b248f48783e68cb51a4e78c85607fe8cd8f21519c5f135a48944c40a0a010aab5b115c1e
                        &interaction.token,
                        &InteractionResponse {
                            kind: InteractionResponseType::ChannelMessageWithSource,
                            data: Some(13  6367
                                InteractionResponseDataBuilder::new()
                                    .flags(MessageFlags::EPHEMERAL)
                                    .content(message)
                                    .allowed_mentions(AllowedMentions::default())
                                    .build(),
                            ),
                        },
                    )
                    .await?;
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

            // validate grant before giving card
            if let Err(err) = validate_grant(&cx, target_user_id) {
                let message = match err {
                    GrantTargetError::IsCurrentUser => {
                        format!("-# {}", cx.config.accent.self_grant)
                    }
                    GrantTargetError::IsBot => {
                        format!(
                            "User <@{}> is a bot. Unfortunately, automatons do not have the higher thought required to appreciate game design.",
                            target_user_id
                        )
                    }
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

            match user::grant_card(&cx.db, target_user_id, guild_id, &name).await {
                Ok(pg) if pg.rows_affected() > 0 => {
                    // the operation was successful
                    let message = format!("Granted card `{}` to user <@{}>!", name, target_user_id);

                    cx.client
                        .interaction(cx.application_id)
                        .create_response(
                            interaction.id,
                            &interaction.token,
                            &InteractionResponse {
                                kind: InteractionResponseType::ChannelMessageWithSource,
                                data: Some(
                                    InteractionResponseDataBuilder::new()
                                        //.flags(MessageFlags::EPHEMERAL)
                                        .content(message)
                                        .allowed_mentions(AllowedMentions::default())
                                        .build(),
                                ),
                            },
                        )
                        .await?;
                }
                Ok(_pg) => {
                    show_not_found(&cx, &interaction, &name).await?;
                }
                Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
                    // user already owns the card!
                    let message =
                        format!("User <@{}> already owns card `{}`!", target_user_id, name);

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
                }
                Err(err) => return Err(err.into()),
            }
        }
        "revoke" => {
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

            // validate grant before giving card
            if let Err(err) = validate_grant(&cx, target_user_id) {
                let message = match err {
                    GrantTargetError::IsCurrentUser => {
                        format!("-# {}", cx.config.accent.self_grant)
                    }
                    GrantTargetError::IsBot => {
                        format!(
                            "User <@{}> is a bot. Unfortunately, automatons do not have the higher thought required to appreciate game design.",
                            target_user_id
                        )
                    }
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
                return Ok(());
            }

            // check if card exists
            if card::get(&cx.db, guild_id, &name).await?.is_none() {
                show_not_found(&cx, &interaction, &name).await?;
                return Ok(());
            }

            match user::revoke_card(&cx.db, target_user_id, guild_id, &name).await {
                Ok(pg) if pg.rows_affected() > 0 => {
                    // the operation was successful
                    let message =
                        format!("Revoked card `{}` from user <@{}>!", name, target_user_id);

                    cx.client
                        .interaction(cx.application_id)
                        .create_response(
                            interaction.id,
                            &interaction.token,
                            &InteractionResponse {
                                kind: InteractionResponseType::ChannelMessageWithSource,
                                data: Some(
                                    InteractionResponseDataBuilder::new()
                                        //.flags(MessageFlags::EPHEMERAL)
                                        .content(message)
                                        .allowed_mentions(AllowedMentions::default())
                                        .build(),
                                ),
                            },
                        )
                        .await?;
                }
                Ok(_pg) => {
                    // we already know if the card exists, so if we fail to
                    // delete rows its because the card isn't real
                    let message =
                        format!("User <@{}> does not own card `{}`!", target_user_id, name);

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
                }
                Err(err) => return Err(err.into()),
            }
        }
        */
        _ => tracing::warn!(?cx.interaction, "unknown interaction"),
    }

    Ok(())
}

async fn autocomplete(cx: InteractionContext, data: CommandData) -> anyhow::Result<()> {
    match data.name.as_str() {
        "s" => crate::card::autocomplete(&cx, data).await?,
        _ => tracing::warn!(?cx.interaction, "unknown interaction"),
    }

    Ok(())
}

async fn message_component(
    cx: InteractionContext,
    interaction: &Interaction,
    data: Box<MessageComponentInteractionData>,
) -> anyhow::Result<()> {
    // Currently, the only interactable components this bot attaches are simple
    // buttons that show cards.
    /*
    let custom_id = data.custom_id.as_str();

    let user_id = interaction
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref())
        .map(|u| u.id)
        .ok_or_else(|| anyhow::Error::msg("missing member in interaction"))?;

    if let Some(card_id) = custom_id.strip_prefix("show_card:") {
        // this is a show card button!
        // parse the card id
        let card_id = card_id.parse::<i32>().context("malformed card id")?;

        // fetch card
        let card = card::fetch_by_id(&cx.db, user_id, card_id).await?;
        show_card(&cx, &interaction, &card).await?;
    }

    if let Some(card_id) = custom_id.strip_prefix("update_with_card:") {
        // like show card, but this one updates the interaction that sent this
        let card_id = card_id.parse::<i32>().context("malformed card id")?;

        // fetch card
        let card = card::fetch_by_id(&cx.db, user_id, card_id).await?;
        update_card(&cx, &interaction, &card).await?;
    }

    if let Some(card_id) = custom_id.strip_prefix("change_visibility:") {
        // changes the visibility of a card to what was specified
        let card_id = card_id.parse::<i32>().context("malformed card id")?;

        let visibility = data
            .values
            .iter()
            .nth(0)
            .context("missing visibility")?
            .parse::<Visibility>()?;

        card::update_visibility(&cx.db, card_id, visibility).await?;

        // re-fetch card and present with the editor
        let card = card::fetch_by_id(&cx.db, user_id, card_id).await?;
        show_card_editor(&cx, &interaction, &card).await?;
    }
    */

    Ok(())
}
