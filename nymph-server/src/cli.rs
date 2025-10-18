//! Nymph server command-line interface.

use std::path::PathBuf;

use chrono::Utc;
use clap::{Parser, Subcommand};

use anyhow::Error;

use crate::{
    app::AppState,
    auth::api_key::{generate_key, hash_key},
};

/// The command line arguments.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// Subcommands.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Operational commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    CreateApiKey(CreateApiKey),
}

/// Creates an API key.
#[derive(clap::Args, Debug)]
pub struct CreateApiKey {
    /// Changes the name of the user the API key is attributed to.
    ///
    /// By default, this user is named `nymph`.
    #[arg(short, long, default_value = "nymph")]
    pub name: String,
}

/// Runs a command.
pub async fn run_command(command: &Command, state: &AppState) -> Result<(), Error> {
    match command {
        Command::CreateApiKey(command) => create_api_key(command, state).await,
    }
}

async fn create_api_key(command: &CreateApiKey, state: &AppState) -> Result<(), Error> {
    let mut tx = state.db.begin().await?;

    let now = Utc::now();

    // try to fetch user with name
    let id = sqlx::query_as::<_, (i32,)>(
        r#"
        SELECT
            u.id
        FROM
            user u
        WHERE
            u.display_name = $1
            AND u.managed = TRUE
        "#,
    )
    .bind(&command.name)
    .fetch_optional(&mut *tx)
    .await?;

    let id = match id {
        Some((id,)) => id,
        None => {
            // create new user
            let (id,) = sqlx::query_as::<_, (i32,)>(
                r#"
                INSERT INTO user (display_name, managed, inserted_at, updated_at)
                VALUES ($1, TRUE, $2, $2)
                RETURNING *
                "#,
            )
            .bind(&command.name)
            .bind(now)
            .fetch_one(&mut *tx)
            .await?;

            id
        }
    };

    // generate api token
    let api_key = generate_key();
    let hash = hash_key(&api_key);

    sqlx::query(
        r#"
        INSERT INTO api_auth (user_id, hash, inserted_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(id)
    .bind(hash)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // export key
    println!("{}", api_key);

    Ok(())
}
