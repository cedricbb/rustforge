//! Project Management
//!
//! This module handles project lifecycle management including:
//! - Project initialization and setup
//! - Template management
//! - Dependency installation
//! - Build and test execution

use crate::security::SecurityManager;
use codev_shared::{Result, WorkspaceConfig, CodevError, CommandResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{debug, info, instrument, warn};

/// Project manager for handling project operations
pub struct ProjectManager {
    /// Workspace configuration
    workspace_config: WorkspaceConfig,

    /// Security manager for safe operations
    security_manager: Arc<SecurityManager>,

    /// Available project templates
    templates: HashMap<String, ProjectTemplate>,
}

impl ProjectManager {
    /// Create a new project manager
    pub fn new(
        workspace_config: &WorkspaceConfig,
        security_manager: Arc<SecurityManager>,
    ) -> Result<Self> {
        let templates = Self::load_default_templates();

        Ok(Self {
            workspace_config: workspace_config.clone(),
            security_manager,
            templates,
        })
    }

    /// Initialize a new project
    #[instrument(skip(self))]
    pub async fn init_project(&self, name: &str, path: &Path) -> Result<()> {
        info!("Initializing project '{}' at: {}", name, path.display());

        // Validate project name
        self.validate_project_name(name)?;

        // Ensure target directory exists and is empty
        self.prepare_project_directory(path).await?;

        // Detect project type or use default
        let template = self.select_template(path).await?;

        // Initialize project from template
        self.apply_template(&template, name, path).await?;

        // Initialize git repository
        self.init_git_repository(path).await?;

        // Install dependencies if applicable
        self.install_dependencies(path).await?;

        info!("Project '{}' initialized successfully", name);
        Ok(())
    }

    /// Build a project
    #[instrument(skip(self))]
    pub async fn build_project(&self, path: &Path) -> Result<CommandResult> {
        info!("Building project at: {}", path.display());

        let build_system = self.detect_build_system(path).await?;
        let command = self.get_build_command(&build_system);

        self.execute_secure_command(&command.0, &command.1, path).await
    }

    /// Test a project
    #[instrument(skip(self))]
    pub async fn test_project(&self, path: &Path) -> Result<CommandResult> {
        info!("Testing project at: {}", path.display());

        let build_system = self.detect_build_system(path).await?;
        let command = self.get_test_command(&build_system);

        self.execute_secure_command(&command.0, &command.1, path).await
    }

    /// Run a project
    #[instrument(skip(self))]
    pub async fn run_project(&self, path: &Path, args: &[&str]) -> Result<CommandResult> {
        info!("Running project at: {}", path.display());

        let build_system = self.detect_build_system(path).await?;
        let mut command = self.get_run_command(&build_system);
        command.1.extend(args.iter().map(|s| s.to_string()));

        self.execute_secure_command(&command.0, &command.1, path).await
    }

    /// Clean build artifacts
    #[instrument(skip(self))]
    pub async fn clean_project(&self, path: &Path) -> Result<CommandResult> {
        info!("Cleaning project at: {}", path.display());

        let build_system = self.detect_build_system(path).await?;
        let command = self.get_clean_command(&build_system);

        self.execute_secure_command(&command.0, &command.1, path).await
    }

    /// Install or update dependencies
    #[instrument(skip(self))]
    pub async fn install_dependencies(&self, path: &Path) -> Result<CommandResult> {
        debug!("Installing dependencies for project at: {}", path.display());

        let build_system = self.detect_build_system(path).await?;
        let command = self.get_install_command(&build_system);

        self.execute_secure_command(&command.0, &command.1, path).await
    }

    /// Get project health status
    pub async fn health_check(&self) -> crate::engine::ComponentHealth {
        // Check if workspace directory is accessible
        if !self.workspace_config.default_path.exists() {
            return crate::engine::ComponentHealth::Unhealthy;
        }

        // Check security manager
        match self.security_manager.health_check().await {
            crate::engine::ComponentHealth::Healthy => crate::engine::ComponentHealth::Healthy,
            _ => crate::engine::ComponentHealth::Degraded,
        }
    }

    /// Validate project name
    fn validate_project_name(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(CodevError::InvalidInput {
                message: "Project name cannot be empty".to_string(),
            });
        }

        if name.contains('/') || name.contains('\\') {
            return Err(CodevError::InvalidInput {
                message: "Project name cannot contain path separators".to_string(),
            });
        }

        if name.starts_with('.') {
            return Err(CodevError::InvalidInput {
                message: "Project name cannot start with a dot".to_string(),
            });
        }

        Ok(())
    }

    /// Prepare project directory
    async fn prepare_project_directory(&self, path: &Path) -> Result<()> {
        if path.exists() {
            // Check if directory is empty
            let mut entries = tokio::fs::read_dir(path).await?;
            if entries.next_entry().await?.is_some() {
                return Err(CodevError::InvalidInput {
                    message: "Target directory is not empty".to_string(),
                });
            }
        } else {
            // Create directory
            tokio::fs::create_dir_all(path).await?;
        }

        Ok(())
    }

    /// Select appropriate template for the project
    async fn select_template(&self, _path: &Path) -> Result<ProjectTemplate> {
        // For now, return Rust template as default
        // TODO: Implement smart template detection
        self.templates
            .get("rust-binary")
            .cloned()
            .ok_or_else(|| CodevError::Internal {
                message: "Default template not found".to_string(),
            })
    }

    /// Apply template to project
    async fn apply_template(&self, template: &ProjectTemplate, name: &str, path: &Path) -> Result<()> {
        debug!("Applying template '{}' to project", template.name);

        for file in &template.files {
            let file_path = path.join(&file.path);

            // Create parent directories
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            // Process template content
            let content = self.process_template_content(&file.content, name);

            // Write file
            tokio::fs::write(&file_path, content).await?;
        }

        Ok(())
    }

    /// Process template content with variable substitution
    fn process_template_content(&self, content: &str, project_name: &str) -> String {
        content
            .replace("{{project_name}}", project_name)
            .replace("{{author}}", &self.get_author_name())
            .replace("{{year}}", &chrono::Utc::now().year().to_string())
    }

    /// Get author name from git config or environment
    fn get_author_name(&self) -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "Developer".to_string())
    }

    /// Initialize git repository
    async fn init_git_repository(&self, path: &Path) -> Result<()> {
        debug!("Initializing git repository");

        let result = self.execute_secure_command("git", &["init".to_string()], path).await?;

        if !result.success {
            warn!("Failed to initialize git repository: {}", result.stderr);
        }

        // Create .gitignore
        let gitignore_path = path.join(".gitignore");
        if !gitignore_path.exists() {
            let gitignore_content = self.get_default_gitignore();
            tokio::fs::write(gitignore_path, gitignore_content).await?;
        }

        Ok(())
    }

    /// Get default .gitignore content
    fn get_default_gitignore(&self) -> String {
        "/target/\n**/*.rs.bk\nCargo.lock\n.DS_Store\n*.log\n".to_string()
    }

    /// Detect build system for a project
    async fn detect_build_system(&self, path: &Path) -> Result<BuildSystemType> {
        if path.join("Cargo.toml").exists() {
            Ok(BuildSystemType::Cargo)
        } else if path.join("package.json").exists() {
            if path.join("yarn.lock").exists() {
                Ok(BuildSystemType::Yarn)
            } else {
                Ok(BuildSystemType::Npm)
            }
        } else if path.join("go.mod").exists() {
            Ok(BuildSystemType::Go)
        } else if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
            Ok(BuildSystemType::Python)
        } else if path.join("pom.xml").exists() {
            Ok(BuildSystemType::Maven)
        } else if path.join("build.gradle").exists() {
            Ok(BuildSystemType::Gradle)
        } else if path.join("Makefile").exists() {
            Ok(BuildSystemType::Make)
        } else {
            Err(CodevError::Analysis {
                message: "No supported build system detected".to_string(),
            })
        }
    }

    /// Get build command for build system
    fn get_build_command(&self, build_system: &BuildSystemType) -> (String, Vec<String>) {
        match build_system {
            BuildSystemType::Cargo => ("cargo".to_string(), vec!["build".to_string()]),
            BuildSystemType::Npm => ("npm".to_string(), vec!["run".to_string(), "build".to_string()]),
            BuildSystemType::Yarn => ("yarn".to_string(), vec!["build".to_string()]),
            BuildSystemType::Go => ("go".to_string(), vec!["build".to_string()]),
            BuildSystemType::Python => ("python".to_string(), vec!["-m".to_string(), "build".to_string()]),
            BuildSystemType::Maven => ("mvn".to_string(), vec!["compile".to_string()]),
            BuildSystemType::Gradle => ("gradle".to_string(), vec!["build".to_string()]),
            BuildSystemType::Make => ("make".to_string(), vec![]),
        }
    }

    /// Get test command for build system
    fn get_test_command(&self, build_system: &BuildSystemType) -> (String, Vec<String>) {
        match build_system {
            BuildSystemType::Cargo => ("cargo".to_string(), vec!["test".to_string()]),
            BuildSystemType::Npm => ("npm".to_string(), vec!["test".to_string()]),
            BuildSystemType::Yarn => ("yarn".to_string(), vec!["test".to_string()]),
            BuildSystemType::Go => ("go".to_string(), vec!["test".to_string(), "./...".to_string()]),
            BuildSystemType::Python => ("python".to_string(), vec!["-m".to_string(), "pytest".to_string()]),
            BuildSystemType::Maven => ("mvn".to_string(), vec!["test".to_string()]),
            BuildSystemType::Gradle => ("gradle".to_string(), vec!["test".to_string()]),
            BuildSystemType::Make => ("make".to_string(), vec!["test".to_string()]),
        }
    }

    /// Get run command for build system
    fn get_run_command(&self, build_system: &BuildSystemType) -> (String, Vec<String>) {
        match build_system {
            BuildSystemType::Cargo => ("cargo".to_string(), vec!["run".to_string()]),
            BuildSystemType::Npm => ("npm".to_string(), vec!["start".to_string()]),
            BuildSystemType::Yarn => ("yarn".to_string(), vec!["start".to_string()]),
            BuildSystemType::Go => ("go".to_string(), vec!["run".to_string(), ".".to_string()]),
            BuildSystemType::Python => ("python".to_string(), vec!["main.py".to_string()]),
            BuildSystemType::Maven => ("mvn".to_string(), vec!["exec:java".to_string()]),
            BuildSystemType::Gradle => ("gradle".to_string(), vec!["run".to_string()]),
            BuildSystemType::Make => ("make".to_string(), vec!["run".to_string()]),
        }
    }

    /// Get clean command for build system
    fn get_clean_command(&self, build_system: &BuildSystemType) -> (String, Vec<String>) {
        match build_system {
            BuildSystemType::Cargo => ("cargo".to_string(), vec!["clean".to_string()]),
            BuildSystemType::Npm => ("npm".to_string(), vec!["run".to_string(), "clean".to_string()]),
            BuildSystemType::Yarn => ("yarn".to_string(), vec!["clean".to_string()]),
            BuildSystemType::Go => ("go".to_string(), vec!["clean".to_string()]),
            BuildSystemType::Python => ("python".to_string(), vec!["-c".to_string(), "import shutil; shutil.rmtree('dist', ignore_errors=True)".to_string()]),
            BuildSystemType::Maven => ("mvn".to_string(), vec!["clean".to_string()]),
            BuildSystemType::Gradle => ("gradle".to_string(), vec!["clean".to_string()]),
            BuildSystemType::Make => ("make".to_string(), vec!["clean".to_string()]),
        }
    }

    /// Get install/update dependencies command
    fn get_install_command(&self, build_system: &BuildSystemType) -> (String, Vec<String>) {
        match build_system {
            BuildSystemType::Cargo => ("cargo".to_string(), vec!["fetch".to_string()]),
            BuildSystemType::Npm => ("npm".to_string(), vec!["install".to_string()]),
            BuildSystemType::Yarn => ("yarn".to_string(), vec!["install".to_string()]),
            BuildSystemType::Go => ("go".to_string(), vec!["mod".to_string(), "download".to_string()]),
            BuildSystemType::Python => ("pip".to_string(), vec!["install".to_string(), "-r".to_string(), "requirements.txt".to_string()]),
            BuildSystemType::Maven => ("mvn".to_string(), vec!["dependency:resolve".to_string()]),
            BuildSystemType::Gradle => ("gradle".to_string(), vec!["dependencies".to_string()]),
            BuildSystemType::Make => ("echo".to_string(), vec!["No dependencies to install".to_string()]),
        }
    }

    /// Execute command securely through security manager
    async fn execute_secure_command(
        &self,
        command: &str,
        args: &[String],
        working_dir: &Path,
    ) -> Result<CommandResult> {
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        // Use security manager for safe execution
        self.security_manager.execute_secure_command(command, &args_str).await
    }

    /// Load default project templates
    fn load_default_templates() -> HashMap<String, ProjectTemplate> {
        let mut templates = HashMap::new();

        // Rust binary template
        templates.insert("rust-binary".to_string(), ProjectTemplate {
            name: "Rust Binary".to_string(),
            description: "A simple Rust binary project".to_string(),
            files: vec![
                TemplateFile {
                    path: "Cargo.toml".to_string(),
                    content: r#"[package]
name = "{{project_name}}"
version = "0.1.0"
edition = "2021"
authors = ["{{author}}"]

[dependencies]
"#.to_string(),
                },
                TemplateFile {
                    path: "src/main.rs".to_string(),
                    content: r#"fn main() {
    println!("Hello, {{project_name}}!");
}
"#.to_string(),
                },
                TemplateFile {
                    path: "README.md".to_string(),
                    content: r#"# {{project_name}}

A Rust project created with CoDev.rs.

## Usage

```bash
cargo run
```

## License

Created in {{year}} by {{author}}.
"#.to_string(),
                },
            ],
        });

        templates
    }
}

/// Build system types
#[derive(Debug, Clone, PartialEq)]
enum BuildSystemType {
    Cargo,
    Npm,
    Yarn,
    Go,
    Python,
    Maven,
    Gradle,
    Make,
}

/// Project template
#[derive(Debug, Clone)]
struct ProjectTemplate {
    pub name: String,
    pub description: String,
    pub files: Vec<TemplateFile>,
}

/// Template file
#[derive(Debug, Clone)]
struct TemplateFile {
    pub path: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_name_validation() {
        let workspace_config = WorkspaceConfig::default();
        let security_manager = Arc::new(
            crate::security::SecurityManager::new(&codev_shared::SecurityConfig::default()).unwrap()
        );
        let manager = ProjectManager::new(&workspace_config, security_manager).unwrap();

        assert!(manager.validate_project_name("valid_name").is_ok());
        assert!(manager.validate_project_name("").is_err());
        assert!(manager.validate_project_name("invalid/name").is_err());
        assert!(manager.validate_project_name(".hidden").is_err());
    }

    #[test]
    fn test_build_system_detection() {
        let build_system = BuildSystemType::Cargo;
        assert_eq!(build_system, BuildSystemType::Cargo);
    }

    #[test]
    fn test_template_content_processing() {
        let workspace_config = WorkspaceConfig::default();
        let security_manager = Arc::new(
            crate::security::SecurityManager::new(&codev_shared::SecurityConfig::default()).unwrap()
        );
        let manager = ProjectManager::new(&workspace_config, security_manager).unwrap();

        let content = "Project: {{project_name}} by {{author}}";
        let processed = manager.process_template_content(content, "test_project");

        assert!(processed.contains("test_project"));
        assert!(!processed.contains("{{project_name}}"));
    }
}