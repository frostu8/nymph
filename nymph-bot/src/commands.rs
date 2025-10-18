//! Command suite.

use std::sync::Arc;

use twilight_cache_inmemory::InMemoryCache;

use twilight_http::Client;

use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::{Interaction, InteractionContextType},
    },
    guild::Permissions,
    id::{Id, marker::ApplicationMarker},
    oauth::ApplicationIntegrationType,
};

use twilight_util::builder::command::{CommandBuilder, StringBuilder, UserBuilder};

use crate::{config::Config, http::Client as DbClient};

use derive_more::Deref;

/// Command context.
///
/// Drills some useful things to the command endpoint.
#[derive(Clone, Debug, Deref)]
pub struct InteractionContext {
    /// The interaction this request is responding to.
    #[deref]
    pub interaction: Interaction,
    /// HTTP Client used to respond to interactions.
    pub client: Arc<Client>,
    /// HTTP Client used to make requests to the database.
    pub db_client: DbClient,
    pub cache: Arc<InMemoryCache>,
    pub config: Arc<Config>,
    pub application_id: Id<ApplicationMarker>,
}

/// Returns a list of commands the bot offers.
pub fn commands() -> [Command; 5] {
    [
        CommandBuilder::new(
            "s",
            "Displays information about a card privately",
            CommandType::ChatInput,
        )
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .contexts([InteractionContextType::Guild])
        .option(
            StringBuilder::new("name", "The name of the card")
                .autocomplete(true)
                .required(true),
        )
        .build(),
        CommandBuilder::new(
            "sl",
            "Displays additional administrator information about a card",
            CommandType::ChatInput,
        )
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .contexts([InteractionContextType::Guild])
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .option(
            StringBuilder::new("name", "The name of the card")
                .autocomplete(true)
                .required(true),
        )
        .build(),
        CommandBuilder::new(
            "inv",
            "Displays all cards that have been granted to you",
            CommandType::ChatInput,
        )
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .contexts([InteractionContextType::Guild])
        .build(),
        CommandBuilder::new(
            "grant",
            "Grants a card to a member, allowing them to view it with /s",
            CommandType::ChatInput,
        )
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .contexts([InteractionContextType::Guild])
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .option(UserBuilder::new("user", "The member to give the card to").required(true))
        .option(
            StringBuilder::new("name", "The name of the card")
                .autocomplete(true)
                .required(true),
        )
        .build(),
        CommandBuilder::new(
            "revoke",
            "Takes a card from a member",
            CommandType::ChatInput,
        )
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .contexts([InteractionContextType::Guild])
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .option(UserBuilder::new("user", "The member to take from").required(true))
        .option(
            StringBuilder::new("name", "The name of the card")
                .autocomplete(true)
                .required(true),
        )
        .build(),
    ]
}
