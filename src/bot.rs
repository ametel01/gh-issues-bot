use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use log::{debug, info, warn};
use rand::{seq::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;
use tokio::time;

use crate::config::{Config, Repository};
use crate::github::{GitHubClient, Issue};
use crate::persistence::Persistence;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveIssue {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
    pub issue_url: String,
    pub requested_at: DateTime<Utc>,
    pub timeout: DateTime<Utc>,
}

pub struct Bot<T: GitHubClient, P: Persistence> {
    config: Config,
    github_client: T,
    persistence: P,
    active_issue: Arc<Mutex<Option<ActiveIssue>>>,
    processed_issues: Arc<Mutex<HashSet<u64>>>,
}

impl<T: GitHubClient, P: Persistence> Bot<T, P> {
    pub fn new(config: Config, github_client: T, persistence: P) -> Self {
        Self {
            config,
            github_client,
            persistence,
            active_issue: Arc::new(Mutex::new(None)),
            processed_issues: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Load state from persistence
        if let Ok(active) = self.persistence.load_active_issue().await {
            let mut lock = self.active_issue.lock().unwrap();
            *lock = active;
        }

        if let Ok(processed) = self.persistence.load_processed_issues().await {
            let mut lock = self.processed_issues.lock().unwrap();
            *lock = processed;
        }

        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting GitHub issue assignment bot");
        info!("Monitoring {} repositories", self.config.repositories.len());

        let mut interval = time::interval(StdDuration::from_secs(self.config.poll_interval_secs));

        loop {
            interval.tick().await;

            // Add some jitter to appear more human-like
            let jitter = thread_rng().gen_range(0..30);
            time::sleep(StdDuration::from_secs(jitter)).await;

            if let Err(e) = self.poll_repositories().await {
                warn!("Error during polling: {}", e);
            }
        }
    }

    async fn poll_repositories(&self) -> Result<()> {
        // Check if we're currently waiting for an assignment
        {
            let active_lock = self.active_issue.lock().unwrap();
            if let Some(ref active) = *active_lock {
                // Still waiting on this issue
                if Utc::now() < active.timeout {
                    debug!(
                        "Waiting for assignment on issue #{} in {}/{}",
                        active.issue_number, active.repo_owner, active.repo_name
                    );
                    return Ok(());
                }

                // Timeout has expired
                info!(
                    "Assignment request for issue #{} in {}/{} has timed out",
                    active.issue_number, active.repo_owner, active.repo_name
                );
            }
        }

        // Check rate limits before making requests
        let remaining = self.github_client.get_rate_limit().await?;
        debug!("GitHub API rate limit: {} remaining", remaining);

        if remaining < 50 {
            warn!(
                "GitHub API rate limit is low: {} remaining. Waiting for reset.",
                remaining
            );
            return Ok(());
        }

        // No active issue or timeout expired, so we can look for a new issue
        for repo in &self.config.repositories {
            match self.process_repository(repo).await {
                Ok(true) => {
                    // Successfully processed an issue, stop for this cycle
                    return Ok(());
                }
                Ok(false) => {
                    // No eligible issues found
                    continue;
                }
                Err(e) => {
                    warn!(
                        "Error processing repository {}/{}: {}",
                        repo.owner, repo.repo, e
                    );
                }
            }
        }

        debug!("No eligible issues found in this cycle");
        Ok(())
    }

    async fn process_repository(&self, repo: &Repository) -> Result<bool> {
        info!("Checking for issues in {}/{}", repo.owner, repo.repo);
        
        let issues = self.github_client.get_open_issues(repo).await?;
        debug!("Found {} issues in {}/{}", issues.len(), repo.owner, repo.repo);
        
        // Process issues sorted by creation date (oldest first to be fair)
        let mut sorted_issues = issues;
        sorted_issues.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        // Get a copy of the processed issues set
        let processed = {
            let processed_lock = self.processed_issues.lock().unwrap();
            processed_lock.clone()
        };
        
        // Find first eligible issue
        for issue in sorted_issues {
            // Skip already processed issues
            if processed.contains(&issue.id) {
                continue;
            }
            
            // Found an eligible issue
            info!("Found eligible issue: #{} - {}", issue.number, issue.title);
            
            // Try to comment on the issue
            if let Err(e) = self.request_assignment(&repo.owner, &repo.repo, &issue).await {
                warn!("Failed to request assignment: {}", e);
                continue;
            }
            
            // Update our state
            self.mark_issue_as_active(&repo.owner, &repo.repo, &issue).await?;
            
            return Ok(true);
        }
        
        Ok(false)
    }

    async fn request_assignment(&self, owner: &str, repo: &str, issue: &Issue) -> Result<()> {
        // Choose a random comment template
        let mut rng = thread_rng();
        let comment = match self.config.comment_templates.choose(&mut rng) {
            Some(template) => template,
            None => "Hi, I'd like to work on this issue!",
        };

        info!(
            "Requesting assignment for issue #{} in {}/{}",
            issue.number, owner, repo
        );
        self.github_client
            .comment_on_issue(owner, repo, issue.number, comment)
            .await?;

        Ok(())
    }

    async fn mark_issue_as_active(&self, owner: &str, repo: &str, issue: &Issue) -> Result<()> {
        let timeout = Utc::now() + Duration::hours(self.config.cooldown_hours as i64);
        
        let active = ActiveIssue {
            repo_owner: owner.to_string(),
            repo_name: repo.to_string(),
            issue_number: issue.number,
            issue_url: issue.html_url.clone(),
            requested_at: Utc::now(),
            timeout,
        };
        
        // Update in-memory state
        {
            let mut active_lock = self.active_issue.lock().unwrap();
            *active_lock = Some(active.clone());
        }
        
        {
            let mut processed_lock = self.processed_issues.lock().unwrap();
            processed_lock.insert(issue.id);
        }
        
        // Update persistent state
        self.persistence.save_active_issue(&active).await?;
        
        // Clone the processed issues set for persistence
        let processed_issues = {
            let processed_lock = self.processed_issues.lock().unwrap();
            processed_lock.clone()
        };
        self.persistence.save_processed_issues(&processed_issues).await?;
        
        info!("Issue #{} in {}/{} marked as active until {}", 
             issue.number, owner, repo, timeout);
        
        Ok(())
    }
}
