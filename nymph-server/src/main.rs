use std::{io, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Error;

use axum::{
    Router,
    extract::{MatchedPath, Request},
    middleware::{Next, from_fn},
    response::Response,
    routing::{delete, get, post},
};

use axum_server::Handle;
use clap::Parser as _;

use nymph_server::{
    app::{AppError, AppState, random_signing_key},
    cli::{Args, run_command},
    config::Config,
    routes,
};

use tokio::{main, select, signal};

use tower_http::{compression::CompressionLayer, trace::TraceLayer};

#[main]
async fn main() -> Result<(), Error> {
    sqlx::any::install_default_drivers();
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::fmt()
        .with_writer(io::stderr)
        .init();

    let args = Args::parse();

    // load config
    let config_path = args.config.unwrap_or_else(|| PathBuf::from("./nymph.toml"));
    let mut config = Config::load(config_path)?;

    // check for development defaults
    if config.server.signing_key.is_none() {
        let signing_key = random_signing_key();

        // print keys
        tracing::warn!("Using development secret: {}", signing_key);
        tracing::warn!("Set a `SIGNING_KEY` option for production!");

        config.server.signing_key = Some(signing_key);
    }

    let state = AppState::new(config.server).await?;
    let db = state.db.clone();

    // Execute command if it exists
    if let Some(command) = args.command {
        return run_command(&command, &state).await;
    }

    let addr: SocketAddr = ([0, 0, 0, 0], state.port).into();

    // Build router
    let router = Router::<AppState>::new()
        .nest(
            "/guilds/{guild_id}/cards",
            Router::<AppState>::new()
                .route("/", get(routes::card::list))
                .route("/{id}", get(routes::card::show)),
        )
        .nest(
            "/users",
            Router::<AppState>::new()
                .route("/discord", post(routes::user::discord))
                .nest(
                    "/{user_id}",
                    Router::<AppState>::new()
                        .route("/cards", get(routes::card::inventory::list))
                        .route("/cards", post(routes::card::inventory::grant))
                        .route("/cards/{card_id}", delete(routes::card::inventory::revoke)),
                ),
        )
        .layer(from_fn(nymph_server::app::app_rest_headers))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request| {
                    let method = req.method();
                    let uri = req.uri();

                    // axum automatically adds this extension.
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|matched_path| matched_path.as_str());

                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                // By default `TraceLayer` will log 5xx responses but we're doing our specific
                // logging of errors so disable that
                .on_failure(()),
        )
        .layer(from_fn(log_app_errors))
        .layer(CompressionLayer::new())
        .with_state(state);

    // Setup cancellation task for server
    let handle = Handle::new();

    // Start cancellation task
    tokio::spawn(shutdown_signal(handle.clone()));

    // Serve HTTP
    tracing::info!("listening on {} (http)", addr);

    axum_server::bind(addr)
        .handle(handle)
        .serve(router.into_make_service())
        .await?;

    // Close Sql connection
    db.close().await;

    tracing::info!("graceful shutdown complete!");

    Ok(())
}

// Stolen from: https://github.com/tokio-rs/axum/blob/main/examples/error-handling/src/main.rs
// Our middleware is responsible for logging error details internally
async fn log_app_errors(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    // If the response contains an AppError Extension, log it.
    if let Some(err) = response.extensions().get::<Arc<AppError>>() {
        tracing::error!(?err, "an unexpected error occurred inside a handler");
    }
    response
}

// Stolen from: https://github.com/maxcountryman/tower-sessions-stores/tree/main/sqlx-store
// Lol
async fn shutdown_signal(handle: Handle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    select! {
        _ = ctrl_c => { handle.shutdown() }
        _ = terminate => { handle.shutdown() }
    }
}
