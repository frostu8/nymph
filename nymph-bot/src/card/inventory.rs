//! Inventory interactions.

use anyhow::{Context as _, Error};

use nymph_model::{Error as ApiError, ErrorCode};

use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    channel::message::{AllowedMentions, MessageFlags},
    http::interaction::{InteractionResponse, InteractionResponseType},
    user::User,
};

use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::InteractionContext;

use super::show_not_found;

use derive_more::{Display, Error};

/// Represents both `/grant` and `/revoke`, which are opposite inventory
/// modifications.
pub async fn command_transfer_card(cx: InteractionContext, data: CommandData) -> Result<(), Error> {
    let guild_id = cx
        .guild_id
        .ok_or_else(|| Error::msg("missing guild id in interaction"))?;
    let caller = cx
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref())
        .ok_or_else(|| Error::msg("missing user in interaction"))?;

    let options = InventoryTransferOptions::try_from(&data)?;

    let is_current_user = cx
        .cache
        .current_user()
        .map(|current_user| current_user.id == options.target_user.id)
        .unwrap_or(false);

    if !options.target_user.bot {
        // fetch requested card
        let card = cx
            .db_client
            .list_cards(guild_id)
            .find(&options.name)
            .execute()
            .await
            .context("failed to fetch card")?
            .into_iter()
            // only find exact matches
            .find(|card| card.name == options.name);

        let Some(card) = card else {
            tracing::debug!(
                "/{}: failed to find card w/ name `{}`",
                data.name,
                options.name
            );
            show_not_found(&cx, &options.name).await?;

            return Ok(());
        };

        // fetch user information
        let user = cx.db_client.get_discord_user(&options.target_user).await?;

        if options.kind == InventoryTransferType::Grant {
            match cx
                .db_client
                .proxy_for(&caller)
                .grant_card_to_user(user.id, card.id)
                .execute()
                .await
            {
                Ok(card) => {
                    // the operation was successful
                    let message = format!(
                        "Granted card `{}` to user <@{}>!",
                        card.name, options.target_user.id,
                    );

                    cx.client
                        .interaction(cx.application_id)
                        .create_response(
                            cx.id,
                            &cx.token,
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

                    Ok(())
                }
                Err(err) if err.is::<ApiError>() => {
                    match err.downcast_ref::<ApiError>().unwrap().code {
                        ErrorCode::InvalidTransfer => {
                            // user already owns the card!
                            let message = format!(
                                "User <@{}> already owns card `{}`!",
                                options.target_user.id, card.name,
                            );

                            cx.client
                                .interaction(cx.application_id)
                                .create_response(
                                    cx.id,
                                    &cx.token,
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

                            Ok(())
                        }
                        _ => Err(err),
                    }
                }
                Err(err) => Err(err),
            }
        } else {
            match cx
                .db_client
                .proxy_for(&caller)
                .revoke_card_from_user(user.id, card.id)
                .execute()
                .await
            {
                Ok(card) => {
                    // the operation was successful
                    let message = format!(
                        "Revoked card `{}` from user <@{}>!",
                        card.name, options.target_user.id,
                    );

                    cx.client
                        .interaction(cx.application_id)
                        .create_response(
                            cx.id,
                            &cx.token,
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

                    Ok(())
                }
                Err(err) if err.is::<ApiError>() => {
                    match err.downcast_ref::<ApiError>().unwrap().code {
                        ErrorCode::InvalidTransfer => {
                            // user already owns the card!
                            let message = format!(
                                "User <@{}> does not own card `{}`!",
                                options.target_user.id, card.name,
                            );

                            cx.client
                                .interaction(cx.application_id)
                                .create_response(
                                    cx.id,
                                    &cx.token,
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

                            Ok(())
                        }
                        _ => Err(err),
                    }
                }
                Err(err) => Err(err),
            }
        }
    } else {
        let message = if is_current_user {
            format!("-# {}", cx.config.accent.self_grant)
        } else {
            format!(
                "User <@{}> is a bot. Unfortunately, automatons do not have the higher thought required to appreciate game design.",
                options.target_user.id
            )
        };

        cx.client
            .interaction(cx.application_id)
            .create_response(
                cx.id,
                &cx.token,
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
            .await
            .map(|_| ())
            .map_err(From::from)
    }
}

#[derive(Debug, Display, Error)]
#[display("invalid command payload")]
struct InvalidCommandPayload;

#[derive(Debug)]
struct InventoryTransferOptions {
    pub name: String,
    pub target_user: User,
    pub kind: InventoryTransferType,
}

#[derive(Debug, PartialEq, Eq)]
enum InventoryTransferType {
    Grant,
    Revoke,
}

impl TryFrom<&CommandData> for InventoryTransferOptions {
    type Error = InvalidCommandPayload;

    fn try_from(value: &CommandData) -> Result<Self, Self::Error> {
        let resolved = value.resolved.as_ref().ok_or(InvalidCommandPayload)?;

        let name = value
            .options
            .iter()
            .find(|option| option.name == "name")
            .and_then(|option| match option.value {
                CommandOptionValue::String(ref value) => Some(value),
                _ => None,
            })
            .ok_or(InvalidCommandPayload)?
            .to_ascii_uppercase();

        let target_user = value
            .options
            .iter()
            .find(|option| option.name == "user")
            .and_then(|option| match option.value {
                CommandOptionValue::User(id) => Some(id),
                _ => None,
            })
            .and_then(|user_id| resolved.users.get(&user_id))
            .cloned()
            .ok_or(InvalidCommandPayload)?;

        let kind = match value.name.as_str() {
            "grant" => InventoryTransferType::Grant,
            "revoke" => InventoryTransferType::Revoke,
            _ => return Err(InvalidCommandPayload),
        };

        Ok(InventoryTransferOptions {
            name,
            target_user,
            kind,
        })
    }
}
