use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use octocrab::Octocrab;
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::config::Repository;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assignee: Option<serde_json::Value>,
    pub assignees: Vec<serde_json::Value>,
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

#[async_trait]
pub trait GitHubClient {
    async fn get_open_issues(&self, repo: &Repository) -> Result<Vec<Issue>>;
    async fn comment_on_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        comment: &str,
    ) -> Result<()>;
    async fn get_rate_limit(&self) -> Result<u32>;
}

pub struct OctocrabClient {
    client: Octocrab,
    reqwest_client: reqwest::Client,
    #[allow(dead_code)]
    token: String,
    #[allow(dead_code)]
    username: String,
}

impl OctocrabClient {
    pub fn new(token: String, username: String) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token.clone())
            .build()
            .context("Failed to build GitHub client")?;

        let mut headers = header::HeaderMap::new();
        let auth_value = format!("token {}", token);
        let mut auth_header = header::HeaderValue::from_str(&auth_value)?;
        auth_header.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_header);

        let reqwest_client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("gh-issues-bot")
            .build()?;

        Ok(Self {
            client,
            reqwest_client,
            token,
            username,
        })
    }
}

#[async_trait]
impl GitHubClient for OctocrabClient {
    async fn get_open_issues(&self, repo: &Repository) -> Result<Vec<Issue>> {
        // Build the URL with query parameters
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues?state=open&per_page=100",
            repo.owner, repo.repo
        );

        // Send the request
        let response = self.reqwest_client.get(&url).send().await?;

        // Check for success
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "GitHub API request failed with status: {}",
                response.status()
            ));
        }

        // Parse the response
        let issues: Vec<Issue> = response.json().await?;

        // Filter issues that are not already assigned and match our criteria
        let filtered_issues = issues
            .into_iter()
            .filter(|issue| {
                // Skip issues that are already assigned
                if issue.assignee.is_some() || !issue.assignees.is_empty() {
                    return false;
                }

                // Make sure the issue has all required labels
                if !repo.labels.is_empty() {
                    let issue_label_names: Vec<String> =
                        issue.labels.iter().map(|l| l.name.clone()).collect();

                    // Check if all required labels are present
                    for required_label in &repo.labels {
                        if !issue_label_names.contains(required_label) {
                            return false;
                        }
                    }
                }

                // Skip issues with excluded labels
                if !repo.exclude_labels.is_empty() {
                    let issue_label_names: Vec<String> =
                        issue.labels.iter().map(|l| l.name.clone()).collect();

                    for exclude_label in &repo.exclude_labels {
                        if issue_label_names.contains(exclude_label) {
                            return false;
                        }
                    }
                }

                // Apply title regex filter if specified
                if let Some(ref regex_str) = repo.title_regex {
                    if let Ok(regex) = regex::Regex::new(regex_str) {
                        return regex.is_match(&issue.title);
                    }
                }

                true
            })
            .collect();

        Ok(filtered_issues)
    }

    async fn comment_on_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        comment: &str,
    ) -> Result<()> {
        self.client
            .issues(owner, repo)
            .create_comment(issue_number, comment)
            .await?;

        Ok(())
    }

    async fn get_rate_limit(&self) -> Result<u32> {
        let url = "https://api.github.com/rate_limit";
        let response = self.reqwest_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "GitHub API rate_limit request failed with status: {}",
                response.status()
            ));
        }

        let rate_limit: serde_json::Value = response.json().await?;
        let remaining = rate_limit["resources"]["core"]["remaining"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(remaining)
    }
}
