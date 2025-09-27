//! Command suite.

use std::ops::Deref;
use std::sync::Arc;

use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client;
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::InteractionContextType,
    },
    guild::Permissions,
    id::{Id, marker::ApplicationMarker},
    oauth::ApplicationIntegrationType,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder, UserBuilder};

use sqlx::PgPool;

use crate::config::Config;

/// Command context.
///
/// Drills some useful things to the command endpoint.
#[derive(Clone, Debug)]
pub struct Context {
    pub config: Arc<Config>,
    /// HTTP Client used to respond to interactions.
    pub client: Arc<Client>,
    pub cache: Arc<InMemoryCache>,
    pub db: PgPool,
    pub application_id: Id<ApplicationMarker>,
}

impl Deref for Context {
    type Target = Client;

    fn deref(&self) -> &Client {
        &self.client
    }
}

/// Returns a list of commands the bot offers.
pub fn commands() -> [Command; 3] {
    [
        CommandBuilder::new(
            "s",
            "Displays full information about a card privately",
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
