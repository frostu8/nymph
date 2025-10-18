//! Card functions and instrumentation.

mod editor;
mod show;

pub use show::command_show;

use std::fmt::Debug;
use std::iter;

use anyhow::Error;

use nymph_model::card::{Card, Visibility};

use tracing::instrument;

use twilight_model::{
    application::{
        command::{CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType},
        interaction::application_command::{CommandData, CommandOptionValue},
    },
    channel::message::{
        Component, MessageFlags,
        component::{ActionRow, Button, ButtonStyle, Container, SelectMenuType},
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
};

use twilight_util::builder::{
    InteractionResponseDataBuilder,
    message::{
        ButtonBuilder, ContainerBuilder, SectionBuilder, SelectMenuBuilder,
        SelectMenuOptionBuilder, TextDisplayBuilder,
    },
};

use crate::commands::InteractionContext;

/// Autocompletes a `/s` command.
pub async fn autocomplete(cx: &InteractionContext, data: CommandData) -> anyhow::Result<()> {
    let guild_id = cx
        .guild_id
        .ok_or_else(|| Error::msg("missing guild id in interaction"))?;
    let user = cx
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref())
        .ok_or_else(|| Error::msg("missing user in interaction"))?;

    let name = data
        .options
        .iter()
        .find(|option| option.name == "name")
        .and_then(|option| match option.value {
            CommandOptionValue::Focused(ref value, CommandOptionType::String) => Some(value),
            _ => None,
        })
        .ok_or_else(|| Error::msg("invalid command payload"))?;

    // make search query uppercase
    let name = name.to_ascii_uppercase();

    // search card
    let choices = cx
        .db_client
        .proxy_for(user.clone())
        .list_cards(guild_id)
        .search(name)
        .await?
        .into_iter()
        .filter(|card| !card.hidden.unwrap_or(false))
        .map(|card| CommandOptionChoice {
            name_localizations: None,
            value: CommandOptionChoiceValue::String(card.name.clone()),
            name: card.name,
        });

    cx.client
        .interaction(cx.application_id)
        .create_response(
            cx.id,
            &cx.token,
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

    Ok(())
}

/// Responds to an interaction with card information and detailed administrator
/// information and settings.
#[instrument(skip(cx))]
async fn show_card_editor(cx: &InteractionContext, card: &Card) -> anyhow::Result<()> {
    let card_container = display_card_admin(cx, card).await?;

    let response = InteractionResponseDataBuilder::new()
        .components(iter::once(Component::Container(card_container)))
        .flags(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2)
        .build();

    cx.client
        .interaction(cx.application_id)
        .create_response(
            cx.id,
            &cx.token,
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response),
            },
        )
        .await?;
    Ok(())
}

/// Responds to an interaction with a list of card information.
#[instrument(skip(cx))]
async fn show_card_list<'c, I>(cx: &InteractionContext, cards: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = &'c Card> + Debug,
{
    // Each card becomes a section of a message component
    let components = cards.into_iter().map(|card| {
        // Build card detail
        let category = card
            .category_name
            .as_ref()
            .and_then(|n| cx.config.category.get(n));
        let formatted_title = category
            .map(|c| c.format_title(&card.name))
            .unwrap_or_else(|| format!("`{}`", card.name));

        let body = format!("## {}", formatted_title);

        // Create button to show card
        let button = ButtonBuilder::new(ButtonStyle::Secondary)
            .custom_id(format!("show_card:{}", card.id))
            .label("View")
            .build();

        // Show card
        Component::Section(
            SectionBuilder::new(button)
                .component(TextDisplayBuilder::new(body).build())
                .build(),
        )
    });

    // Put these all in an embed container
    let mut card_container = ContainerBuilder::new()
        .accent_color(Some(cx.config.general.embed_color))
        .spoiler(false)
        .build();
    card_container.components.extend(components);

    let response = InteractionResponseDataBuilder::new()
        .components(iter::once(Component::Container(card_container)))
        .flags(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2)
        .build();

    cx.client
        .interaction(cx.application_id)
        .create_response(
            cx.id,
            &cx.token,
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response),
            },
        )
        .await?;
    Ok(())
}

/// Creates a card container populated with the information of the card, and a
/// set of admin settings.
async fn display_card_admin(cx: &InteractionContext, card: &Card) -> anyhow::Result<Container> {
    // create the base card container
    let mut card_container = display_card(cx, card)?;

    // create a new action row for admin widgets
    let mut action_row = ActionRow {
        id: None,
        components: Vec::with_capacity(1),
    };

    // add the visibility adjustment menu
    let visibility_selector = SelectMenuBuilder::new(
        format!("change_visibility:{}", card.id),
        SelectMenuType::Text,
    )
    .option(
        SelectMenuOptionBuilder::new("Private", Visibility::Private.to_str())
            .description("Card details and its existence is hidden to users that do not own the card")
            .default(card.visibility == Visibility::Private),
    )
    .option(
        SelectMenuOptionBuilder::new("Hidden", Visibility::Hidden.to_str())
            .description("Card details are hidden, but its existence is known to users that do not own the card")
            .default(card.visibility == Visibility::Hidden),
    )
    .option(
        SelectMenuOptionBuilder::new("Public", Visibility::Public.to_str())
            .description("Card details and its existence is known to users that do not own the card")
            .default(card.visibility == Visibility::Public),
    )
    .build();

    action_row.components.push(visibility_selector.into());

    // finalize
    card_container.components.push(action_row.into());

    Ok(card_container)
}

/// Creates a card container populated with the information of the card.
fn display_card(cx: &InteractionContext, card: &Card) -> anyhow::Result<Container> {
    let category = card
        .category_name
        .as_ref()
        .and_then(|n| cx.config.category.get(n));
    let color = category.and_then(|c| c.color);

    // create the card action row
    let mut action_row = ActionRow {
        id: None,
        components: Vec::with_capacity(2),
    };

    // if we have found a downgrade or upgrade card, push the respective button
    // to the end of the components list
    if let Some(downgrade) = card.downgrade.as_ref() {
        action_row.components.push(Component::Button(Button {
            id: None,
            custom_id: Some(format!("update_with_card:{}", downgrade.id)),
            disabled: false,
            emoji: None,
            label: Some(String::from("--")),
            style: ButtonStyle::Danger,
            url: None,
            sku_id: None,
        }));
    }

    if let Some(upgrade) = card.upgrades.as_ref().and_then(|upgrades| upgrades.first()) {
        action_row.components.push(Component::Button(Button {
            id: None,
            custom_id: Some(format!("update_with_card:{}", upgrade.id)),
            disabled: false,
            emoji: None,
            label: Some(String::from("++")),
            style: ButtonStyle::Success,
            url: None,
            sku_id: None,
        }));
    }

    // append any category prefixes/suffixes to title
    let formatted_title = category
        .map(|c| c.format_title(&card.name))
        .unwrap_or_else(|| format!("`{}`", card.name));

    // build card body
    let body = format!("# {}\n{}", formatted_title, card.content);

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
    Ok(card_container)
}

/// Responds to an interaction with a not found error message.
async fn show_not_found(cx: &InteractionContext, name: impl AsRef<str>) -> anyhow::Result<()> {
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
            cx.id,
            &cx.token,
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
async fn show_unauthorized(cx: &InteractionContext, name: impl AsRef<str>) -> anyhow::Result<()> {
    let accent = cx.config.accent.select_unauthorized();
    let message = format!(
        "-# {}\nThe card `{}` is hidden to you.",
        accent,
        name.as_ref()
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
                        .build(),
                ),
            },
        )
        .await?;

    Ok(())
}
