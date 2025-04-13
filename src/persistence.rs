use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::bot::ActiveIssue;

#[async_trait]
pub trait Persistence {
    async fn save_active_issue(&self, issue: &ActiveIssue) -> Result<()>;
    async fn load_active_issue(&self) -> Result<Option<ActiveIssue>>;
    async fn save_processed_issues(&self, issues: &HashSet<u64>) -> Result<()>;
    async fn load_processed_issues(&self) -> Result<HashSet<u64>>;
}

pub struct FilePersistence {
    data_dir: PathBuf,
}

impl FilePersistence {
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();

        // Create data directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).await?;
        }

        Ok(Self { data_dir })
    }

    fn active_issue_path(&self) -> PathBuf {
        self.data_dir.join("active_issue.json")
    }

    fn processed_issues_path(&self) -> PathBuf {
        self.data_dir.join("processed_issues.json")
    }
}

#[async_trait]
impl Persistence for FilePersistence {
    async fn save_active_issue(&self, issue: &ActiveIssue) -> Result<()> {
        let content = serde_json::to_string_pretty(issue)?;
        let path = self.active_issue_path();

        fs::write(&path, content)
            .await
            .with_context(|| format!("Failed to write active issue to {}", path.display()))?;

        Ok(())
    }

    async fn load_active_issue(&self) -> Result<Option<ActiveIssue>> {
        let path = self.active_issue_path();

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read active issue from {}", path.display()))?;

        let issue: ActiveIssue =
            serde_json::from_str(&content).with_context(|| "Failed to parse active issue JSON")?;

        Ok(Some(issue))
    }

    async fn save_processed_issues(&self, issues: &HashSet<u64>) -> Result<()> {
        let content = serde_json::to_string_pretty(issues)?;
        let path = self.processed_issues_path();

        fs::write(&path, content)
            .await
            .with_context(|| format!("Failed to write processed issues to {}", path.display()))?;

        Ok(())
    }

    async fn load_processed_issues(&self) -> Result<HashSet<u64>> {
        let path = self.processed_issues_path();

        if !path.exists() {
            return Ok(HashSet::new());
        }

        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read processed issues from {}", path.display()))?;

        let issues: HashSet<u64> = serde_json::from_str(&content)
            .with_context(|| "Failed to parse processed issues JSON")?;

        Ok(issues)
    }
}
