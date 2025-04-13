use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub auth_token: String,
    pub user_login: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_cooldown_hours")]
    pub cooldown_hours: u32,
    #[serde(default)]
    pub comment_templates: Vec<String>,
    pub repositories: Vec<Repository>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Repository {
    pub owner: String,
    pub repo: String,
    pub labels: Vec<String>,
    #[serde(default)]
    pub title_regex: Option<String>,
    #[serde(default)]
    pub exclude_labels: Vec<String>,
}

fn default_poll_interval() -> u64 {
    45
}

fn default_max_retries() -> u32 {
    3
}

fn default_cooldown_hours() -> u32 {
    24
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).context("Failed to read config file")?;

        let mut config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        // If no comment templates provided, add some defaults
        if config.comment_templates.is_empty() {
            config.comment_templates = vec![
                "Hi, I'd love to take this one!".to_string(),
                "This looks interesting, may I work on it?".to_string(),
                "I'd like to contribute to this issue, thanks!".to_string(),
            ];
        }

        Ok(config)
    }

    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let auth_token =
            std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable not set")?;

        let user_login = std::env::var("GITHUB_USERNAME")
            .context("GITHUB_USERNAME environment variable not set")?;

        // This is a minimal config from environment variables
        // For full configuration, use a config file
        Ok(Config {
            auth_token,
            user_login,
            poll_interval_secs: default_poll_interval(),
            max_retries: default_max_retries(),
            cooldown_hours: default_cooldown_hours(),
            comment_templates: vec![
                "Hi, I'd love to take this one!".to_string(),
                "This looks interesting, may I work on it?".to_string(),
                "I'd like to contribute to this issue, thanks!".to_string(),
            ],
            repositories: vec![],
        })
    }
}
