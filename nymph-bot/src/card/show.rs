//! The show command.
//!
//! See [`command_show`].

use std::iter;

use nymph_model::{Error as ApiError, ErrorCode, card::Card};

use twilight_util::builder::InteractionResponseDataBuilder;

use super::{display_card, show_not_found, show_unauthorized};

use twilight_model::{
    application::interaction::application_command::{CommandData, CommandOptionValue},
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
};

use crate::commands::InteractionContext;

use anyhow::Error;

/// `/s`, shows a card to a user.
pub async fn command_show(cx: InteractionContext, data: CommandData) -> anyhow::Result<()> {
    let guild_id = cx
        .guild_id
        .ok_or_else(|| Error::msg("missing guild id in interaction"))?;

    let name = data
        .options
        .iter()
        .find(|option| option.name == "name")
        .and_then(|option| match option.value {
            CommandOptionValue::String(ref value) => Some(value),
            _ => None,
        })
        .ok_or_else(|| Error::msg("invalid command payload"))?;
    let name = name.to_ascii_uppercase();

    let card = cx
        .db_client
        .list_cards(guild_id)
        .find(&name)
        .execute()
        .await?
        .into_iter()
        // only find exact matches
        .find(|card| card.name == name);

    let Some(Card { id, .. }) = card else {
        // confidently say no card exists
        tracing::debug!("/s: failed to find card w/ name `{}`", name);
        show_not_found(&cx, &name).await?;

        return Ok(());
    };

    match show_card(&cx, id).await {
        Ok(resp) => cx
            .client
            .interaction(cx.application_id)
            .create_response(
                cx.id,
                &cx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    ..resp
                },
            )
            .await
            .map(|_| ())
            .map_err(From::from),
        Err(err) if err.is::<ApiError>() => match err.downcast_ref::<ApiError>().unwrap().code {
            ErrorCode::Hidden => {
                tracing::debug!(?err, "/s: card is hidden");
                show_unauthorized(&cx, &name).await.map_err(From::from)
            }
            ErrorCode::Forbidden => {
                tracing::debug!(?err, "/s: card is private");
                show_not_found(&cx, &name).await.map_err(From::from)
            }
            _ => Err(err),
        },
        Err(err) => Err(err),
    }
}

/// Creates an [`InteractionResponse`] for showing a card
///
/// By default, `kind` is
/// [`InteractionResponseType::ChannelMessageWithSource`], but may be
/// reconfigured to update existing replies.
pub async fn show_card(cx: &InteractionContext, id: i32) -> anyhow::Result<InteractionResponse> {
    let guild_id = cx
        .guild_id
        .ok_or_else(|| Error::msg("missing guild id in interaction"))?;
    let caller = cx
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref())
        .ok_or_else(|| Error::msg("missing member in interaction"))?;

    let card = cx
        .db_client
        .proxy_for(&caller)
        .get_card(guild_id, id)
        .execute()
        .await?;

    tracing::debug!(?card, "/s: got card");

    // build card
    let card = display_card(&cx, &card)?;

    Ok(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .components(iter::once(card.into()))
                .flags(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2)
                .build(),
        ),
    })
}
