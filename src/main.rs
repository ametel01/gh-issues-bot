mod bot;
mod config;
mod github;
mod persistence;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use log::info;
use std::path::PathBuf;

use crate::bot::Bot;
use crate::config::Config;
use crate::github::OctocrabClient;
use crate::persistence::FilePersistence;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the bot
    Run {
        /// Path to config file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// Directory to store state
        #[arg(short, long, value_name = "DIR", default_value = ".gh-issues-bot")]
        data_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment variables from .env file (if it exists)
    dotenv().ok();

    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Parse command line arguments
    let args = Args::parse();

    match args.command {
        Commands::Run { config, data_dir } => {
            run_bot(config, data_dir).await?;
        }
    }

    Ok(())
}

async fn run_bot(config_path: Option<PathBuf>, data_dir: PathBuf) -> Result<()> {
    // Load configuration
    let config = match config_path {
        Some(path) => Config::from_file(&path)
            .with_context(|| format!("Failed to load config from {}", path.display()))?,
        None => Config::from_env().context("Failed to load config from environment")?,
    };

    // Initialize GitHub client
    let github_client = OctocrabClient::new(config.auth_token.clone(), config.user_login.clone())
        .context("Failed to initialize GitHub client")?;

    // Initialize persistence
    let persistence = FilePersistence::new(&data_dir)
        .await
        .context("Failed to initialize persistence")?;

    // Initialize and run bot
    let mut bot = Bot::new(config.clone(), github_client, persistence);
    bot.initialize().await?;

    info!("Bot initialized successfully");
    info!(
        "Watching for issues in {} repositories",
        config.repositories.len()
    );

    bot.start().await?;

    Ok(())
}
