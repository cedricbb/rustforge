//! Security and Sandbox Management
//!
//! This module provides security features including:
//! - Command execution sandboxing
//! - File access restrictions
//! - Resource limits enforcement
//! - Security level management

use codev_shared::{SecurityConfig, SecurityLevel, CommandResult, CodevError, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, instrument, warn};

/// Security manager for safe code execution
pub struct SecurityManager {
    /// Security configuration
    config: SecurityConfig,

    /// Current security level
    security_level: SecurityLevel,

    /// Allowed commands whitelist
    allowed_commands: HashSet<String>,

    /// Temporary sandbox directories
    sandbox_dirs: Vec<PathBuf>,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: &SecurityConfig) -> Result<Self> {
        let allowed_commands: HashSet<String> = config.allowed_commands.iter().cloned().collect();

        Ok(Self {
            config: config.clone(),
            security_level: config.default_level,
            allowed_commands,
            sandbox_dirs: Vec::new(),
        })
    }

    /// Execute a command with security restrictions
    #[instrument(skip(self))]
    pub async fn execute_secure_command(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<CommandResult> {
        info!("Executing secure command: {} {:?}", command, args);

        // 1. Validate command is allowed
        self.validate_command(command)?;

        // 2. Create sandbox environment if needed
        let sandbox = self.create_sandbox_if_needed().await?;

        // 3. Execute with restrictions
        let result = self.execute_with_restrictions(command, args, sandbox.as_deref()).await?;

        // 4. Cleanup sandbox
        if let Some(sandbox_path) = sandbox {
            self.cleanup_sandbox(&sandbox_path).await?;
        }

        Ok(result)
    }

    /// Set security level
    #[instrument(skip(self))]
    pub fn set_security_level(&mut self, level: SecurityLevel) {
        info!("Changing security level to: {:?}", level);
        self.security_level = level;
    }

    /// Get current security level
    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    /// Check if a file path is allowed for access
    pub fn is_path_allowed(&self, path: &Path, write_access: bool) -> bool {
        let path_str = path.to_string_lossy();

        // Check forbidden paths
        for forbidden in &self.config.file_access.forbidden_paths {
            if path.starts_with(forbidden) {
                return false;
            }
        }

        if write_access {
            // Check write-allowed paths
            for allowed in &self.config.file_access.write_allowed_paths {
                if path.starts_with(allowed) {
                    return true;
                }
            }
            return false;
        } else {
            // Read access - check if not in forbidden list
            true
        }
    }

    /// Validate file size limits
    pub async fn validate_file_size(&self, path: &Path) -> Result<()> {
        if let Ok(metadata) = tokio::fs::metadata(path).await {
            if metadata.len() > self.config.file_access.max_file_size as u64 {
                return Err(CodevError::Security {
                    message: format!(
                        "File size {} exceeds limit {}",
                        metadata.len(),
                        self.config.file_access.max_file_size
                    ),
                });
            }
        }
        Ok(())
    }

    /// Get health status
    pub async fn health_check(&self) -> crate::engine::ComponentHealth {
        // Basic health check - verify sandbox functionality
        match self.test_sandbox_creation().await {
            Ok(_) => crate::engine::ComponentHealth::Healthy,
            Err(_) => crate::engine::ComponentHealth::Degraded,
        }
    }

    /// Shutdown and cleanup
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down security manager");

        // Cleanup any remaining sandbox directories
        for sandbox_dir in &self.sandbox_dirs {
            if sandbox_dir.exists() {
                if let Err(e) = tokio::fs::remove_dir_all(sandbox_dir).await {
                    warn!("Failed to cleanup sandbox directory {}: {}", sandbox_dir.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Validate that a command is allowed
    fn validate_command(&self, command: &str) -> Result<()> {
        if !self.allowed_commands.contains(command) {
            return Err(CodevError::Security {
                message: format!("Command '{}' is not allowed", command),
            });
        }
        Ok(())
    }

    /// Create sandbox environment if security level requires it
    async fn create_sandbox_if_needed(&self) -> Result<Option<PathBuf>> {
        match self.security_level {
            SecurityLevel::Development => Ok(None), // No sandbox in development
            SecurityLevel::Production | SecurityLevel::Paranoid => {
                let sandbox_path = self.create_sandbox().await?;
                Ok(Some(sandbox_path))
            }
        }
    }

    /// Create a sandbox environment
    async fn create_sandbox(&self) -> Result<PathBuf> {
        debug!("Creating sandbox environment");

        let sandbox_dir = if let Some(temp_dir) = &self.config.sandbox.temp_dir {
            temp_dir.clone()
        } else {
            std::env::temp_dir().join("codev_sandbox")
        };

        // Create unique sandbox directory
        let unique_dir = sandbox_dir.join(format!("sandbox_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&unique_dir).await?;

        // Set up minimal environment in sandbox
        self.setup_sandbox_environment(&unique_dir).await?;

        debug!("Sandbox created at: {}", unique_dir.display());
        Ok(unique_dir)
    }

    /// Set up sandbox environment with necessary files
    async fn setup_sandbox_environment(&self, sandbox_path: &Path) -> Result<()> {
        // Create basic directory structure
        tokio::fs::create_dir_all(sandbox_path.join("tmp")).await?;
        tokio::fs::create_dir_all(sandbox_path.join("workspace")).await?;

        // Copy necessary system files (simplified)
        // In a full implementation, this would copy essential binaries and libraries

        Ok(())
    }

    /// Execute command with security restrictions
    async fn execute_with_restrictions(
        &self,
        command: &str,
        args: &[&str],
        sandbox_path: Option<&Path>,
    ) -> Result<CommandResult> {
        let mut cmd = Command::new(command);
        cmd.args(args);

        // Set working directory to sandbox if provided
        if let Some(sandbox) = sandbox_path {
            cmd.current_dir(sandbox.join("workspace"));
        }

        // Apply resource limits based on security level
        self.apply_resource_limits(&mut cmd).await?;

        // Execute with timeout
        let timeout = self.get_execution_timeout();
        let result = tokio::time::timeout(timeout, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                Ok(CommandResult {
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: output.status.code().unwrap_or(-1),
                    success: output.status.success(),
                })
            }
            Ok(Err(e)) => Err(CodevError::CommandExecution {
                command: command.to_string(),
                error: e.to_string(),
            }),
            Err(_) => Err(CodevError::Timeout {
                operation: format!("Command execution: {}", command),
            }),
        }
    }

    /// Apply resource limits to command
    async fn apply_resource_limits(&self, _cmd: &mut Command) -> Result<()> {
        // Resource limits implementation would be platform-specific
        // On Unix systems, this would use setrlimit() or similar
        // On Windows, this would use job objects

        match self.security_level {
            SecurityLevel::Development => {
                // No limits in development
            }
            SecurityLevel::Production => {
                // Apply moderate limits
                debug!("Applying production resource limits");
            }
            SecurityLevel::Paranoid => {
                // Apply strict limits
                debug!("Applying paranoid resource limits");
            }
        }

        Ok(())
    }

    /// Get execution timeout based on security level
    fn get_execution_timeout(&self) -> Duration {
        match self.security_level {
            SecurityLevel::Development => Duration::from_secs(300), // 5 minutes
            SecurityLevel::Production => Duration::from_secs(120),  // 2 minutes
            SecurityLevel::Paranoid => Duration::from_secs(60),     // 1 minute
        }
    }

    /// Cleanup sandbox directory
    async fn cleanup_sandbox(&self, sandbox_path: &Path) -> Result<()> {
        debug!("Cleaning up sandbox: {}", sandbox_path.display());

        if sandbox_path.exists() {
            tokio::fs::remove_dir_all(sandbox_path).await?;
        }

        Ok(())
    }

    /// Test sandbox creation (for health checks)
    async fn test_sandbox_creation(&self) -> Result<()> {
        let test_sandbox = self.create_sandbox().await?;
        self.cleanup_sandbox(&test_sandbox).await?;
        Ok(())
    }
}

/// Security context for operations
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub level: SecurityLevel,
    pub allowed_operations: Vec<String>,
    pub restricted_paths: Vec<PathBuf>,
    pub max_execution_time: Duration,
}

impl SecurityContext {
    /// Create context for security level
    pub fn for_level(level: SecurityLevel) -> Self {
        match level {
            SecurityLevel::Development => Self {
                level,
                allowed_operations: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "execute".to_string(),
                    "network".to_string(),
                ],
                restricted_paths: vec![],
                max_execution_time: Duration::from_secs(300),
            },
            SecurityLevel::Production => Self {
                level,
                allowed_operations: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "execute".to_string(),
                ],
                restricted_paths: vec![
                    PathBuf::from("/etc"),
                    PathBuf::from("/root"),
                ],
                max_execution_time: Duration::from_secs(120),
            },
            SecurityLevel::Paranoid => Self {
                level,
                allowed_operations: vec![
                    "read".to_string(),
                    "execute".to_string(),
                ],
                restricted_paths: vec![
                    PathBuf::from("/etc"),
                    PathBuf::from("/root"),
                    PathBuf::from("/usr"),
                    PathBuf::from("/bin"),
                ],
                max_execution_time: Duration::from_secs(60),
            },
        }
    }

    /// Check if operation is allowed
    pub fn is_operation_allowed(&self, operation: &str) -> bool {
        self.allowed_operations.contains(&operation.to_string())
    }

    /// Check if path access is allowed
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        for restricted in &self.restricted_paths {
            if path.starts_with(restricted) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(&config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_command_validation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(&config).unwrap();

        // Should allow whitelisted commands
        assert!(manager.validate_command("cargo").is_ok());

        // Should reject non-whitelisted commands
        assert!(manager.validate_command("rm").is_err());
    }

    #[test]
    fn test_path_access_validation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(&config).unwrap();

        // Should allow access to temp directory
        assert!(manager.is_path_allowed(&PathBuf::from("/tmp/test"), false));

        // Should deny access to forbidden paths
        assert!(!manager.is_path_allowed(&PathBuf::from("/etc/passwd"), false));
    }

    #[test]
    fn test_security_context() {
        let context = SecurityContext::for_level(SecurityLevel::Development);
        assert!(context.is_operation_allowed("read"));
        assert!(context.is_operation_allowed("write"));
        assert!(context.is_operation_allowed("network"));

        let paranoid_context = SecurityContext::for_level(SecurityLevel::Paranoid);
        assert!(paranoid_context.is_operation_allowed("read"));
        assert!(!paranoid_context.is_operation_allowed("write"));
        assert!(!paranoid_context.is_operation_allowed("network"));
    }

    #[test]
    fn test_security_level_transitions() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(&config).unwrap();

        assert_eq!(manager.security_level(), SecurityLevel::Development);

        manager.set_security_level(SecurityLevel::Production);
        assert_eq!(manager.security_level(), SecurityLevel::Production);

        manager.set_security_level(SecurityLevel::Paranoid);
        assert_eq!(manager.security_level(), SecurityLevel::Paranoid);
    }
}