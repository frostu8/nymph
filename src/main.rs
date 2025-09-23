use std::env;
use std::sync::Arc;

use nymph::{commands::Context, dispatch};
use sqlx::PgPool;
use twilight_gateway::{
    ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // setup database
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // setup discord connection
    let token = env::var("DISCORD_TOKEN")?;
    let intents = Intents::empty();

    let config = ConfigBuilder::new(token.clone(), intents).build();

    // setup client
    let client = Arc::new(Client::new(token));
    let application = client.current_user_application().await?.model().await?;

    let interaction = client.interaction(application.id);

    let mut shard = Shard::with_config(ShardId::ONE, config);

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            tracing::warn!(source = ?item.unwrap_err(), "error receiving event");

            continue;
        };

        tracing::debug!(?event, "received event");

        match event {
            Event::Ready(_ready) => {
                // create commands
                interaction
                    .set_global_commands(&nymph::commands::commands())
                    .await?;
            }
            Event::InteractionCreate(interaction) => {
                // setup command context
                let cx = Context {
                    client: client.clone(),
                    db: pool.clone(),
                    application_id: application.id,
                };

                tokio::spawn(dispatch::interaction(cx, interaction));
            }
            _ => (),
        }
    }

    Ok(())
}
