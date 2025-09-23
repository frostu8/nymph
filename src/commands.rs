//! Command suite.

use std::ops::Deref;
use std::sync::Arc;

use twilight_http::Client;
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::InteractionContextType,
    },
    id::{Id, marker::ApplicationMarker},
    oauth::ApplicationIntegrationType,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

/// Command context.
///
/// Drills some useful things to the command endpoint.
#[derive(Clone, Debug)]
pub struct Context {
    /// HTTP Client used to respond to interactions.
    pub client: Arc<Client>,
    pub application_id: Id<ApplicationMarker>,
}

impl Deref for Context {
    type Target = Client;

    fn deref(&self) -> &Client {
        &self.client
    }
}

/// Returns a list of commands the bot offers.
pub fn commands() -> [Command; 1] {
    [CommandBuilder::new(
        "show",
        "Displays full information about a card publicly",
        CommandType::ChatInput,
    )
    .integration_types([ApplicationIntegrationType::GuildInstall])
    .contexts([InteractionContextType::Guild])
    .option(
        StringBuilder::new("name", "The name of the card")
            .autocomplete(true)
            .required(true),
    )
    .build()]
}
