use std::sync::Arc;

use nymph_bot::{commands::InteractionContext, config::Config, dispatch, http::Client as DbClient};

use twilight_cache_inmemory::{InMemoryCacheBuilder, ResourceType};
use twilight_gateway::{
    ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client;
use twilight_model::gateway::payload::incoming::GuildCreate;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // load config
    let config = Arc::new(Config::load("nymph-bot.toml")?);

    tracing::info!("connecting to api...");

    // setup database
    let db_client = DbClient::new(&config.api)?;

    // setup discord connection
    let token = config.general.discord_token.clone();
    //let intents = Intents::empty();
    let intents = Intents::GUILDS;

    let shard_config = ConfigBuilder::new(token.clone(), intents).build();

    // setup cache
    let cache_config = InMemoryCacheBuilder::new()
        .resource_types(ResourceType::MEMBER | ResourceType::USER | ResourceType::USER_CURRENT);
    let cache = Arc::new(cache_config.build());

    // setup client
    let client = Arc::new(Client::new(token));
    let application = client.current_user_application().await?.model().await?;

    if let Some(owner) = application.owner {
        tracing::info!("application id: {}, owner: {}", application.id, owner.name);
    } else {
        tracing::info!("application id: {}", application.id);
    }

    let interaction = client.interaction(application.id);

    let mut shard = Shard::with_config(ShardId::ONE, shard_config);

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            tracing::warn!(source = ?item.unwrap_err(), "error receiving event");

            continue;
        };

        cache.update(&event);

        tracing::trace!(?event, "received event");

        match event {
            Event::Ready(ready) => {
                tracing::info!(
                    "serving bot as {}#{} in {} guilds",
                    ready.user.name,
                    ready.user.discriminator(),
                    ready.guilds.len()
                );

                // create commands
                interaction
                    .set_global_commands(&nymph_bot::commands::commands())
                    .await?;
            }
            Event::GuildCreate(guild) => match guild.as_ref() {
                GuildCreate::Available(guild) => tracing::info!("guild: {}", guild.name),
                _ => (),
            },
            Event::InteractionCreate(interaction) => {
                let interaction = interaction.0;

                // setup command context
                let cx = InteractionContext {
                    interaction,
                    config: config.clone(),
                    client: client.clone(),
                    cache: cache.clone(),
                    db_client: db_client.clone(),
                    application_id: application.id,
                };

                tokio::spawn(dispatch::interaction(cx));
            }
            _ => (),
        }
    }

    Ok(())
}
