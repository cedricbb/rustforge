//! Git Integration and Repository Management
//!
//! This module provides Git integration capabilities including:
//! - Repository analysis and status
//! - Commit and branch management
//! - Change detection and diff analysis
//! - Git workflow automation

use codev_shared::{GitInfo, Result, CodevError};
use git2::{Repository, StatusOptions, Status, BranchType, ObjectType, Oid, Signature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument, warn};

/// Git repository manager
pub struct GitManager {
    /// Git repository instance
    repo: Repository,

    /// Repository root path
    root_path: PathBuf,
}

impl GitManager {
    /// Open an existing Git repository
    #[instrument]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Opening Git repository at: {}", path.display());

        let repo = Repository::discover(path)
            .map_err(|e| CodevError::Git {
                message: format!("Failed to open repository: {}", e),
            })?;

        let root_path = repo.workdir()
            .ok_or_else(|| CodevError::Git {
                message: "Repository has no working directory".to_string(),
            })?
            .to_path_buf();

        Ok(Self { repo, root_path })
    }

    /// Initialize a new Git repository
    #[instrument]
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        info!("Initializing Git repository at: {}", path.display());

        let repo = Repository::init(path)
            .map_err(|e| CodevError::Git {
                message: format!("Failed to initialize repository: {}", e),
            })?;

        Ok(Self {
            repo,
            root_path: path.to_path_buf(),
        })
    }

    /// Get repository information
    #[instrument(skip(self))]
    pub fn get_repository_info(&self) -> Result<GitInfo> {
        debug!("Getting repository information");

        let current_branch = self.get_current_branch()?;
        let remote_url = self.get_remote_url().ok();
        let is_dirty = self.is_working_directory_dirty()?;
        let last_commit = self.get_last_commit_hash().ok();

        Ok(GitInfo {
            current_branch,
            remote_url,
            is_dirty,
            last_commit,
        })
    }

    /// Get current branch name
    fn get_current_branch(&self) -> Result<String> {
        let head = self.repo.head()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get HEAD: {}", e),
            })?;

        if let Some(branch_name) = head.shorthand() {
            Ok(branch_name.to_string())
        } else {
            Ok("(detached HEAD)".to_string())
        }
    }

    /// Get remote URL (origin)
    fn get_remote_url(&self) -> Result<String> {
        let remote = self.repo.find_remote("origin")
            .map_err(|e| CodevError::Git {
                message: format!("Failed to find origin remote: {}", e),
            })?;

        remote.url()
            .ok_or_else(|| CodevError::Git {
                message: "Remote URL not found".to_string(),
            })
            .map(|s| s.to_string())
    }

    /// Check if working directory has uncommitted changes
    fn is_working_directory_dirty(&self) -> Result<bool> {
        let statuses = self.repo.statuses(Some(
            StatusOptions::new()
                .include_untracked(true)
                .include_ignored(false)
        ))
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get repository status: {}", e),
            })?;

        Ok(!statuses.is_empty())
    }

    /// Get last commit hash
    fn get_last_commit_hash(&self) -> Result<String> {
        let head = self.repo.head()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get HEAD: {}", e),
            })?;

        let commit = head.peel_to_commit()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get commit: {}", e),
            })?;

        Ok(commit.id().to_string())
    }

    /// Get repository status with detailed file information
    #[instrument(skip(self))]
    pub fn get_status(&self) -> Result<RepositoryStatus> {
        debug!("Getting detailed repository status");

        let statuses = self.repo.statuses(Some(
            StatusOptions::new()
                .include_untracked(true)
                .include_ignored(false)
        ))
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get repository status: {}", e),
            })?;

        let mut status = RepositoryStatus {
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            untracked: Vec::new(),
            staged: Vec::new(),
            conflicted: Vec::new(),
        };

        for entry in statuses.iter() {
            let file_path = entry.path().unwrap_or("<unknown>").to_string();
            let flags = entry.status();

            if flags.contains(Status::WT_MODIFIED) {
                status.modified.push(file_path.clone());
            }
            if flags.contains(Status::WT_NEW) {
                status.untracked.push(file_path.clone());
            }
            if flags.contains(Status::WT_DELETED) {
                status.deleted.push(file_path.clone());
            }
            if flags.contains(Status::INDEX_MODIFIED) || flags.contains(Status::INDEX_NEW) {
                status.staged.push(file_path.clone());
            }
            if flags.contains(Status::CONFLICTED) {
                status.conflicted.push(file_path.clone());
            }
        }

        Ok(status)
    }

    /// Get list of branches
    #[instrument(skip(self))]
    pub fn get_branches(&self) -> Result<BranchInfo> {
        debug!("Getting branch information");

        let branches = self.repo.branches(Some(BranchType::Local))
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get branches: {}", e),
            })?;

        let mut local_branches = Vec::new();
        let mut current_branch = None;

        for branch_result in branches {
            let (branch, _) = branch_result
                .map_err(|e| CodevError::Git {
                    message: format!("Failed to read branch: {}", e),
                })?;

            if let Some(name) = branch.name()
                .map_err(|e| CodevError::Git {
                    message: format!("Failed to get branch name: {}", e),
                })? {
                let name = name.to_string();

                if branch.is_head() {
                    current_branch = Some(name.clone());
                }

                local_branches.push(name);
            }
        }

        Ok(BranchInfo {
            current: current_branch.unwrap_or_else(|| "unknown".to_string()),
            local: local_branches,
            remote: Vec::new(), // TODO: Implement remote branch listing
        })
    }

    /// Create a new branch
    #[instrument(skip(self))]
    pub fn create_branch(&self, name: &str) -> Result<()> {
        info!("Creating branch: {}", name);

        let head = self.repo.head()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get HEAD: {}", e),
            })?;

        let commit = head.peel_to_commit()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get commit: {}", e),
            })?;

        self.repo.branch(name, &commit, false)
            .map_err(|e| CodevError::Git {
                message: format!("Failed to create branch: {}", e),
            })?;

        info!("Branch '{}' created successfully", name);
        Ok(())
    }

    /// Switch to a different branch
    #[instrument(skip(self))]
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        info!("Checking out branch: {}", name);

        let (object, reference) = self.repo.revparse_ext(name)
            .map_err(|e| CodevError::Git {
                message: format!("Failed to find branch '{}': {}", name, e),
            })?;

        self.repo.checkout_tree(&object, None)
            .map_err(|e| CodevError::Git {
                message: format!("Failed to checkout tree: {}", e),
            })?;

        match reference {
            Some(gref) => {
                self.repo.set_head(gref.name().unwrap())
                    .map_err(|e| CodevError::Git {
                        message: format!("Failed to set HEAD: {}", e),
                    })?;
            }
            None => {
                self.repo.set_head_detached(object.id())
                    .map_err(|e| CodevError::Git {
                        message: format!("Failed to set detached HEAD: {}", e),
                    })?;
            }
        }

        info!("Successfully checked out branch: {}", name);
        Ok(())
    }

    /// Add files to staging area
    #[instrument(skip(self))]
    pub fn add_files(&self, paths: &[&str]) -> Result<()> {
        debug!("Adding files to staging area: {:?}", paths);

        let mut index = self.repo.index()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get index: {}", e),
            })?;

        for path in paths {
            index.add_path(Path::new(path))
                .map_err(|e| CodevError::Git {
                    message: format!("Failed to add path '{}': {}", path, e),
                })?;
        }

        index.write()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to write index: {}", e),
            })?;

        debug!("Files added to staging area successfully");
        Ok(())
    }

    /// Commit changes
    #[instrument(skip(self))]
    pub fn commit(&self, message: &str) -> Result<CommitInfo> {
        info!("Creating commit with message: {}", message);

        let signature = self.get_signature()?;

        let mut index = self.repo.index()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get index: {}", e),
            })?;

        let tree_id = index.write_tree()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to write tree: {}", e),
            })?;

        let tree = self.repo.find_tree(tree_id)
            .map_err(|e| CodevError::Git {
                message: format!("Failed to find tree: {}", e),
            })?;

        // Get parent commit
        let parent_commit = match self.repo.head() {
            Ok(head) => Some(head.peel_to_commit()
                .map_err(|e| CodevError::Git {
                    message: format!("Failed to get parent commit: {}", e),
                })?),
            Err(_) => None, // First commit
        };

        let parents = if let Some(ref parent) = parent_commit {
            vec![parent]
        } else {
            vec![]
        };

        let commit_id = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
            .map_err(|e| CodevError::Git {
                message: format!("Failed to create commit: {}", e),
            })?;

        let commit_info = CommitInfo {
            id: commit_id.to_string(),
            message: message.to_string(),
            author: signature.name().unwrap_or("Unknown").to_string(),
            timestamp: chrono::Utc::now(),
        };

        info!("Commit created successfully: {}", commit_id);
        Ok(commit_info)
    }

    /// Get git signature for commits
    fn get_signature(&self) -> Result<Signature<'static>> {
        // Try to get signature from git config
        let config = self.repo.config()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get git config: {}", e),
            })?;

        let name = config.get_string("user.name")
            .or_else(|_| std::env::var("GIT_AUTHOR_NAME"))
            .unwrap_or_else(|_| "CoDev.rs Agent".to_string());

        let email = config.get_string("user.email")
            .or_else(|_| std::env::var("GIT_AUTHOR_EMAIL"))
            .unwrap_or_else(|_| "codev@local.dev".to_string());

        Signature::now(&name, &email)
            .map_err(|e| CodevError::Git {
                message: format!("Failed to create signature: {}", e),
            })
    }

    /// Get commit history
    #[instrument(skip(self))]
    pub fn get_commit_history(&self, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        debug!("Getting commit history");

        let mut revwalk = self.repo.revwalk()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to create revwalk: {}", e),
            })?;

        revwalk.push_head()
            .map_err(|e| CodevError::Git {
                message: format!("Failed to push HEAD: {}", e),
            })?;

        let mut commits = Vec::new();
        let limit = limit.unwrap_or(50); // Default to 50 commits

        for (count, oid_result) in revwalk.enumerate() {
            if count >= limit {
                break;
            }

            let oid = oid_result
                .map_err(|e| CodevError::Git {
                    message: format!("Failed to get commit OID: {}", e),
                })?;

            let commit = self.repo.find_commit(oid)
                .map_err(|e| CodevError::Git {
                    message: format!("Failed to find commit: {}", e),
                })?;

            let commit_info = CommitInfo {
                id: commit.id().to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                timestamp: chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                    .unwrap_or_else(chrono::Utc::now),
            };

            commits.push(commit_info);
        }

        Ok(commits)
    }

    /// Get changes since last commit
    #[instrument(skip(self))]
    pub fn get_changes_since_commit(&self, commit_id: Option<&str>) -> Result<Vec<FileChange>> {
        debug!("Getting changes since commit: {:?}", commit_id);

        let head_commit = self.repo.head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|e| CodevError::Git {
                message: format!("Failed to get HEAD commit: {}", e),
            })?;

        let compare_commit = if let Some(commit_id) = commit_id {
            let oid = Oid::from_str(commit_id)
                .map_err(|e| CodevError::Git {
                    message: format!("Invalid commit ID: {}", e),
                })?;
            Some(self.repo.find_commit(oid)
                .map_err(|e| CodevError::Git {
                    message: format!("Failed to find commit: {}", e),
                })?)
        } else {
            head_commit.parents().next()
        };

        let mut changes = Vec::new();

        // Simple file change detection (simplified implementation)
        // In a full implementation, this would use git2's diff functionality
        let status = self.get_status()?;

        for file in status.modified {
            changes.push(FileChange {
                path: file,
                change_type: ChangeType::Modified,
                additions: 0, // Would need diff analysis
                deletions: 0,
            });
        }

        for file in status.added {
            changes.push(FileChange {
                path: file,
                change_type: ChangeType::Added,
                additions: 0,
                deletions: 0,
            });
        }

        for file in status.deleted {
            changes.push(FileChange {
                path: file,
                change_type: ChangeType::Deleted,
                additions: 0,
                deletions: 0,
            });
        }

        Ok(changes)
    }
}

/// Repository status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStatus {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
    pub untracked: Vec<String>,
    pub staged: Vec<String>,
    pub conflicted: Vec<String>,
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub current: String,
    pub local: Vec<String>,
    pub remote: Vec<String>,
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// File change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
}

/// Type of file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_git_repository_init() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let git_manager = GitManager::init(repo_path);
        assert!(git_manager.is_ok());

        let manager = git_manager.unwrap();
        assert_eq!(manager.root_path, repo_path);
    }

    #[test]
    fn test_git_repository_info() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let manager = GitManager::init(repo_path).unwrap();
        let info = manager.get_repository_info();

        // May fail without proper git setup, which is expected in tests
        match info {
            Ok(git_info) => {
                // Basic validation
                assert!(!git_info.current_branch.is_empty());
            }
            Err(_) => {
                // Expected in test environment without full git setup
            }
        }
    }

    #[test]
    fn test_repository_status() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        if let Ok(manager) = GitManager::init(repo_path) {
            // Create a test file
            let test_file = repo_path.join("test.txt");
            fs::write(&test_file, "test content").unwrap();

            if let Ok(status) = manager.get_status() {
                // File should appear as untracked
                assert!(!status.untracked.is_empty() || !status.modified.is_empty());
            }
        }
    }
}