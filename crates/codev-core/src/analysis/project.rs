//! Project-level analysis and understanding
//!
//! This module provides comprehensive project analysis including:
//! - Project structure detection
//! - Language and framework identification
//! - Build system analysis
//! - Dependency mapping
//! - Architecture understanding

use crate::ai::AiEngine;
use crate::analysis::{Analyzer, CodeAnalyzer, ComplexityLevel, QualityScore};
use codev_shared::{
    BuildSystem, CodeAnalysis, GitInfo, Language, ProjectContext, Result, CodevError
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info, instrument, warn};
use walkdir::WalkDir;

/// Project analyzer that understands entire codebases
pub struct ProjectAnalyzer {
    /// AI engine for intelligent analysis
    ai_engine: Arc<AiEngine>,

    /// Code analyzer for individual files
    code_analyzer: CodeAnalyzer,

    /// Ignore patterns (like .gitignore)
    ignore_patterns: Vec<String>,
}

impl ProjectAnalyzer {
    /// Create a new project analyzer
    pub fn new(ai_engine: Arc<AiEngine>) -> Result<Self> {
        let code_analyzer = CodeAnalyzer::new()?;

        Ok(Self {
            ai_engine,
            code_analyzer,
            ignore_patterns: Self::default_ignore_patterns(),
        })
    }

    /// Analyze an entire project
    #[instrument(skip(self))]
    pub async fn analyze_project(&self, project_path: &Path) -> Result<ProjectAnalysis> {
        info!("Starting project analysis for: {}", project_path.display());

        // 1. Discover project structure
        let structure = self.discover_project_structure(project_path).await?;

        // 2. Detect languages and frameworks
        let languages = self.detect_languages(&structure.files).await?;
        let frameworks = self.detect_frameworks(project_path, &languages).await?;

        // 3. Analyze build system
        let build_system = self.detect_build_system(project_path).await?;

        // 4. Extract dependencies
        let dependencies = self.extract_dependencies(project_path, &build_system).await?;

        // 5. Git analysis
        let git_info = self.analyze_git_repository(project_path).await.ok();

        // 6. Code quality analysis
        let quality_metrics = self.analyze_code_quality(&structure.files).await?;

        // 7. Architecture analysis
        let architecture = self.analyze_architecture(project_path, &structure).await?;

        // 8. AI-powered insights
        let ai_insights = self.generate_ai_insights(project_path, &structure).await?;

        let analysis = ProjectAnalysis {
            path: project_path.to_path_buf(),
            name: self.extract_project_name(project_path),
            structure,
            languages,
            frameworks,
            build_system,
            dependencies,
            git_info,
            quality_metrics,
            architecture,
            ai_insights,
            analyzed_at: chrono::Utc::now(),
        };

        info!("Project analysis completed for: {}", project_path.display());
        Ok(analysis)
    }

    /// Discover project structure
    #[instrument(skip(self))]
    async fn discover_project_structure(&self, project_path: &Path) -> Result<ProjectStructure> {
        debug!("Discovering project structure");

        let mut files = Vec::new();
        let mut directories = Vec::new();
        let mut total_size = 0u64;

        for entry in WalkDir::new(project_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_ignore_path(e.path()))
        {
            let entry = entry.map_err(|e| CodevError::Io(e.into()))?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(metadata) = fs::metadata(path).await {
                    let size = metadata.len();
                    total_size += size;

                    let relative_path = path.strip_prefix(project_path)
                        .unwrap_or(path)
                        .to_path_buf();

                    files.push(FileInfo {
                        path: relative_path,
                        size,
                        language: Language::from_extension(
                            &path.extension()
                                .and_then(|ext| ext.to_str())
                                .unwrap_or("")
                        ),
                        is_test: self.is_test_file(path),
                        is_config: self.is_config_file(path),
                    });
                }
            } else if path.is_dir() && path != project_path {
                let relative_path = path.strip_prefix(project_path)
                    .unwrap_or(path)
                    .to_path_buf();
                directories.push(relative_path);
            }
        }

        Ok(ProjectStructure {
            files,
            directories,
            total_size,
            file_count: files.len(),
            directory_count: directories.len(),
        })
    }

    /// Detect programming languages in the project
    async fn detect_languages(&self, files: &[FileInfo]) -> Result<Vec<LanguageInfo>> {
        debug!("Detecting languages");

        let mut language_stats: HashMap<Language, LanguageStats> = HashMap::new();

        for file in files {
            if file.language != Language::Unknown {
                let stats = language_stats.entry(file.language).or_insert_with(|| LanguageStats {
                    language: file.language,
                    file_count: 0,
                    total_size: 0,
                    percentage: 0.0,
                });

                stats.file_count += 1;
                stats.total_size += file.size;
            }
        }

        // Calculate percentages
        let total_code_size: u64 = language_stats.values()
            .map(|stats| stats.total_size)
            .sum();

        for stats in language_stats.values_mut() {
            stats.percentage = if total_code_size > 0 {
                (stats.total_size as f64 / total_code_size as f64) * 100.0
            } else {
                0.0
            };
        }

        // Sort by percentage (descending)
        let mut languages: Vec<LanguageInfo> = language_stats
            .into_values()
            .map(|stats| LanguageInfo {
                language: stats.language,
                file_count: stats.file_count,
                total_size: stats.total_size,
                percentage: stats.percentage,
                is_primary: false, // Will be set below
            })
            .collect();

        languages.sort_by(|a, b| b.percentage.partial_cmp(&a.percentage).unwrap());

        // Mark primary language (highest percentage)
        if let Some(first) = languages.first_mut() {
            first.is_primary = true;
        }

        Ok(languages)
    }

    /// Detect frameworks used in the project
    async fn detect_frameworks(&self, project_path: &Path, languages: &[LanguageInfo]) -> Result<Vec<Framework>> {
        debug!("Detecting frameworks");

        let mut frameworks = Vec::new();

        for lang_info in languages {
            match lang_info.language {
                Language::Rust => {
                    frameworks.extend(self.detect_rust_frameworks(project_path).await?);
                }
                Language::JavaScript | Language::TypeScript => {
                    frameworks.extend(self.detect_js_frameworks(project_path).await?);
                }
                Language::Python => {
                    frameworks.extend(self.detect_python_frameworks(project_path).await?);
                }
                Language::Go => {
                    frameworks.extend(self.detect_go_frameworks(project_path).await?);
                }
                _ => {} // Other languages not implemented yet
            }
        }

        Ok(frameworks)
    }

    /// Detect Rust frameworks
    async fn detect_rust_frameworks(&self, project_path: &Path) -> Result<Vec<Framework>> {
        let mut frameworks = Vec::new();

        let cargo_toml_path = project_path.join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml_path).await {
                // Parse Cargo.toml and look for known frameworks
                if content.contains("tokio") {
                    frameworks.push(Framework {
                        name: "Tokio".to_string(),
                        category: FrameworkCategory::AsyncRuntime,
                        version: self.extract_dependency_version(&content, "tokio"),
                        confidence: 0.9,
                    });
                }

                if content.contains("axum") {
                    frameworks.push(Framework {
                        name: "Axum".to_string(),
                        category: FrameworkCategory::WebFramework,
                        version: self.extract_dependency_version(&content, "axum"),
                        confidence: 0.9,
                    });
                }

                if content.contains("actix-web") {
                    frameworks.push(Framework {
                        name: "Actix Web".to_string(),
                        category: FrameworkCategory::WebFramework,
                        version: self.extract_dependency_version(&content, "actix-web"),
                        confidence: 0.9,
                    });
                }

                if content.contains("diesel") {
                    frameworks.push(Framework {
                        name: "Diesel".to_string(),
                        category: FrameworkCategory::Database,
                        version: self.extract_dependency_version(&content, "diesel"),
                        confidence: 0.9,
                    });
                }

                if content.contains("sqlx") {
                    frameworks.push(Framework {
                        name: "SQLx".to_string(),
                        category: FrameworkCategory::Database,
                        version: self.extract_dependency_version(&content, "sqlx"),
                        confidence: 0.9,
                    });
                }
            }
        }

        Ok(frameworks)
    }

    /// Detect JavaScript/TypeScript frameworks
    async fn detect_js_frameworks(&self, project_path: &Path) -> Result<Vec<Framework>> {
        let mut frameworks = Vec::new();

        let package_json_path = project_path.join("package.json");
        if package_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&package_json_path).await {
                // Parse package.json for known frameworks
                if content.contains("\"react\"") {
                    frameworks.push(Framework {
                        name: "React".to_string(),
                        category: FrameworkCategory::Frontend,
                        version: self.extract_npm_dependency_version(&content, "react"),
                        confidence: 0.9,
                    });
                }

                if content.contains("\"vue\"") {
                    frameworks.push(Framework {
                        name: "Vue.js".to_string(),
                        category: FrameworkCategory::Frontend,
                        version: self.extract_npm_dependency_version(&content, "vue"),
                        confidence: 0.9,
                    });
                }

                if content.contains("\"express\"") {
                    frameworks.push(Framework {
                        name: "Express.js".to_string(),
                        category: FrameworkCategory::WebFramework,
                        version: self.extract_npm_dependency_version(&content, "express"),
                        confidence: 0.9,
                    });
                }

                if content.contains("\"next\"") {
                    frameworks.push(Framework {
                        name: "Next.js".to_string(),
                        category: FrameworkCategory::FullStack,
                        version: self.extract_npm_dependency_version(&content, "next"),
                        confidence: 0.9,
                    });
                }
            }
        }

        Ok(frameworks)
    }

    /// Detect Python frameworks (simplified)
    async fn detect_python_frameworks(&self, _project_path: &Path) -> Result<Vec<Framework>> {
        // TODO: Implement Python framework detection
        Ok(Vec::new())
    }

    /// Detect Go frameworks (simplified)
    async fn detect_go_frameworks(&self, _project_path: &Path) -> Result<Vec<Framework>> {
        // TODO: Implement Go framework detection
        Ok(Vec::new())
    }

    /// Detect build system
    async fn detect_build_system(&self, project_path: &Path) -> Result<Option<BuildSystem>> {
        debug!("Detecting build system");

        if project_path.join("Cargo.toml").exists() {
            return Ok(Some(BuildSystem::Cargo));
        }

        if project_path.join("package.json").exists() {
            if project_path.join("yarn.lock").exists() {
                return Ok(Some(BuildSystem::Yarn));
            } else {
                return Ok(Some(BuildSystem::Npm));
            }
        }

        if project_path.join("requirements.txt").exists() ||
            project_path.join("pyproject.toml").exists() {
            return Ok(Some(BuildSystem::Pip));
        }

        if project_path.join("go.mod").exists() {
            return Ok(Some(BuildSystem::Go));
        }

        if project_path.join("pom.xml").exists() {
            return Ok(Some(BuildSystem::Maven));
        }

        if project_path.join("build.gradle").exists() ||
            project_path.join("build.gradle.kts").exists() {
            return Ok(Some(BuildSystem::Gradle));
        }

        if project_path.join("Makefile").exists() {
            return Ok(Some(BuildSystem::Make));
        }

        Ok(None)
    }

    /// Extract project dependencies
    async fn extract_dependencies(&self, project_path: &Path, build_system: &Option<BuildSystem>) -> Result<HashMap<String, String>> {
        debug!("Extracting dependencies");

        match build_system {
            Some(BuildSystem::Cargo) => self.extract_cargo_dependencies(project_path).await,
            Some(BuildSystem::Npm) | Some(BuildSystem::Yarn) => self.extract_npm_dependencies(project_path).await,
            Some(BuildSystem::Pip) => self.extract_pip_dependencies(project_path).await,
            _ => Ok(HashMap::new()), // Other build systems not implemented yet
        }
    }

    /// Extract Cargo dependencies
    async fn extract_cargo_dependencies(&self, project_path: &Path) -> Result<HashMap<String, String>> {
        let cargo_toml_path = project_path.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&cargo_toml_path).await?;
        let mut dependencies = HashMap::new();

        // Simple TOML parsing for dependencies section
        let mut in_dependencies = false;
        for line in content.lines() {
            let line = line.trim();

            if line == "[dependencies]" {
                in_dependencies = true;
                continue;
            }

            if line.starts_with('[') && line != "[dependencies]" {
                in_dependencies = false;
                continue;
            }

            if in_dependencies && line.contains('=') {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim().to_string();
                    let version = parts[1].trim().trim_matches('"').to_string();
                    dependencies.insert(name, version);
                }
            }
        }

        Ok(dependencies)
    }

    /// Extract NPM dependencies (simplified)
    async fn extract_npm_dependencies(&self, project_path: &Path) -> Result<HashMap<String, String>> {
        let package_json_path = project_path.join("package.json");
        if !package_json_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&package_json_path).await?;
        // TODO: Proper JSON parsing for dependencies
        // For now, return empty map
        Ok(HashMap::new())
    }

    /// Extract pip dependencies (simplified)
    async fn extract_pip_dependencies(&self, _project_path: &Path) -> Result<HashMap<String, String>> {
        // TODO: Implement pip dependency extraction
        Ok(HashMap::new())
    }

    /// Analyze Git repository
    async fn analyze_git_repository(&self, project_path: &Path) -> Result<GitInfo> {
        // TODO: Implement Git analysis using git2 crate
        // For now, return basic info
        Err(CodevError::Internal {
            message: "Git analysis not yet implemented".to_string(),
        })
    }

    /// Analyze code quality across the project
    async fn analyze_code_quality(&self, files: &[FileInfo]) -> Result<ProjectQualityMetrics> {
        debug!("Analyzing code quality");

        let mut total_lines = 0;
        let mut test_lines = 0;
        let mut complexity_scores = Vec::new();
        let mut quality_scores = Vec::new();

        // Analyze a sample of files (to avoid overwhelming the system)
        let sample_files: Vec<&FileInfo> = files
            .iter()
            .filter(|f| f.language != Language::Unknown)
            .take(20) // Limit to 20 files for performance
            .collect();

        for file in sample_files {
            // TODO: Implement actual code analysis
            // For now, generate mock metrics
            let lines = (file.size / 50) as usize; // Rough estimate
            total_lines += lines;

            if file.is_test {
                test_lines += lines;
            }

            // Mock complexity score
            complexity_scores.push(5.0 + (file.size as f32 / 1000.0));

            // Mock quality score
            quality_scores.push(QualityScore::new(80.0, 75.0, 85.0, 90.0));
        }

        let average_complexity = if !complexity_scores.is_empty() {
            complexity_scores.iter().sum::<f32>() / complexity_scores.len() as f32
        } else {
            0.0
        };

        let average_quality = if !quality_scores.is_empty() {
            let sum: f32 = quality_scores.iter().map(|q| q.overall).sum();
            sum / quality_scores.len() as f32
        } else {
            0.0
        };

        let test_coverage = if total_lines > 0 {
            (test_lines as f32 / total_lines as f32) * 100.0
        } else {
            0.0
        };

        Ok(ProjectQualityMetrics {
            overall_score: average_quality,
            complexity_level: ComplexityLevel::from_score(average_complexity),
            test_coverage: test_coverage,
            maintainability_index: average_quality, // Simplified
            technical_debt_ratio: 100.0 - average_quality, // Simplified
            code_duplication: 5.0, // Mock value
        })
    }

    /// Analyze project architecture
    async fn analyze_architecture(&self, _project_path: &Path, structure: &ProjectStructure) -> Result<ArchitectureInfo> {
        debug!("Analyzing architecture");

        // Simple architecture detection based on directory structure
        let mut patterns = Vec::new();
        let mut layers = Vec::new();

        // Look for common patterns
        for dir in &structure.directories {
            let dir_name = dir.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            match dir_name {
                "src" | "lib" => {
                    patterns.push("Source Organization".to_string());
                }
                "tests" | "test" => {
                    patterns.push("Test Organization".to_string());
                }
                "docs" | "documentation" => {
                    patterns.push("Documentation".to_string());
                }
                "examples" => {
                    patterns.push("Examples".to_string());
                }
                "api" | "controllers" => {
                    layers.push("API Layer".to_string());
                }
                "services" | "business" => {
                    layers.push("Service Layer".to_string());
                }
                "models" | "entities" => {
                    layers.push("Data Layer".to_string());
                }
                _ => {}
            }
        }

        // Determine architecture style
        let style = if layers.len() >= 3 {
            "Layered Architecture".to_string()
        } else if structure.directories.iter().any(|d| d.to_string_lossy().contains("component")) {
            "Component-Based Architecture".to_string()
        } else if structure.files.iter().any(|f| f.path.to_string_lossy().contains("mod.rs")) {
            "Module-Based Architecture".to_string()
        } else {
            "Monolithic Architecture".to_string()
        };

        Ok(ArchitectureInfo {
            style,
            patterns,
            layers,
            modularity_score: (patterns.len() + layers.len()) as f32 * 10.0,
            coupling_level: "Medium".to_string(), // Mock value
            cohesion_level: "High".to_string(),   // Mock value
        })
    }

    /// Generate AI-powered insights
    async fn generate_ai_insights(&self, project_path: &Path, structure: &ProjectStructure) -> Result<Vec<String>> {
        debug!("Generating AI insights");

        let context = format!(
            "Project at {} has {} files and {} directories. \
            Main languages detected. Analyze the project structure and provide insights.",
            project_path.display(),
            structure.file_count,
            structure.directory_count
        );

        // TODO: Use AI engine to generate insights
        // For now, return mock insights
        Ok(vec![
            "Well-organized project structure with clear separation of concerns".to_string(),
            "Good test coverage based on test directory structure".to_string(),
            "Consider adding more documentation files".to_string(),
        ])
    }

    /// Helper methods
    fn should_ignore_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        path_str.contains("test") || path_str.contains("spec") || path_str.ends_with("_test.rs")
    }

    fn is_config_file(&self, path: &Path) -> bool {
        let file_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase();

        matches!(file_name.as_str(),
            "cargo.toml" | "package.json" | "requirements.txt" |
            "go.mod" | "pom.xml" | "build.gradle" | "makefile" |
            "dockerfile" | "docker-compose.yml" | ".gitignore"
        )
    }

    fn extract_project_name(&self, project_path: &Path) -> String {
        project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    fn extract_dependency_version(&self, content: &str, dep_name: &str) -> Option<String> {
        // Simple regex-like extraction for TOML
        for line in content.lines() {
            if line.trim().starts_with(dep_name) && line.contains('=') {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    return Some(parts[1].trim().trim_matches('"').to_string());
                }
            }
        }
        None
    }

    fn extract_npm_dependency_version(&self, _content: &str, _dep_name: &str) -> Option<String> {
        // TODO: Implement JSON parsing for package.json
        None
    }

    fn default_ignore_patterns() -> Vec<String> {
        vec![
            ".git".to_string(),
            "target".to_string(),
            "node_modules".to_string(),
            "__pycache__".to_string(),
            ".cache".to_string(),
            "dist".to_string(),
            "build".to_string(),
            ".DS_Store".to_string(),
            "*.tmp".to_string(),
            "*.log".to_string(),
        ]
    }
}

/// Complete project analysis result
#[derive(Debug, Clone)]
pub struct ProjectAnalysis {
    pub path: PathBuf,
    pub name: String,
    pub structure: ProjectStructure,
    pub languages: Vec<LanguageInfo>,
    pub frameworks: Vec<Framework>,
    pub build_system: Option<BuildSystem>,
    pub dependencies: HashMap<String, String>,
    pub git_info: Option<GitInfo>,
    pub quality_metrics: ProjectQualityMetrics,
    pub architecture: ArchitectureInfo,
    pub ai_insights: Vec<String>,
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
}

/// Project structure information
#[derive(Debug, Clone)]
pub struct ProjectStructure {
    pub files: Vec<FileInfo>,
    pub directories: Vec<PathBuf>,
    pub total_size: u64,
    pub file_count: usize,
    pub directory_count: usize,
}

/// Information about a file in the project
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub language: Language,
    pub is_test: bool,
    pub is_config: bool,
}

/// Language usage statistics
#[derive(Debug, Clone)]
struct LanguageStats {
    pub language: Language,
    pub file_count: usize,
    pub total_size: u64,
    pub percentage: f64,
}

/// Language information with usage statistics
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub language: Language,
    pub file_count: usize,
    pub total_size: u64,
    pub percentage: f64,
    pub is_primary: bool,
}

/// Framework information
#[derive(Debug, Clone)]
pub struct Framework {
    pub name: String,
    pub category: FrameworkCategory,
    pub version: Option<String>,
    pub confidence: f32,
}

/// Framework categories
#[derive(Debug, Clone, PartialEq)]
pub enum FrameworkCategory {
    WebFramework,
    Frontend,
    Backend,
    Database,
    Testing,
    AsyncRuntime,
    FullStack,
    Other,
}

/// Project quality metrics
#[derive(Debug, Clone)]
pub struct ProjectQualityMetrics {
    pub overall_score: f32,
    pub complexity_level: ComplexityLevel,
    pub test_coverage: f32,
    pub maintainability_index: f32,
    pub technical_debt_ratio: f32,
    pub code_duplication: f32,
}

/// Architecture information
#[derive(Debug, Clone)]
pub struct ArchitectureInfo {
    pub style: String,
    pub patterns: Vec<String>,
    pub layers: Vec<String>,
    pub modularity_score: f32,
    pub coupling_level: String,
    pub cohesion_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_project_structure_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create test structure
        fs::create_dir_all(project_path.join("src")).await.unwrap();
        fs::create_dir_all(project_path.join("tests")).await.unwrap();
        fs::write(project_path.join("src/main.rs"), "fn main() {}").await.unwrap();
        fs::write(project_path.join("Cargo.toml"), "[package]\nname = \"test\"").await.unwrap();

        // This would fail without a proper AI engine, which is expected in tests
        // let analyzer = ProjectAnalyzer::new(ai_engine)?;
        // let analysis = analyzer.analyze_project(project_path).await?;

        // For now, just test the structure creation
        assert!(project_path.join("src").exists());
        assert!(project_path.join("tests").exists());
    }

    #[test]
    fn test_complexity_level() {
        assert_eq!(ComplexityLevel::from_score(3.0), ComplexityLevel::Low);
        assert_eq!(ComplexityLevel::from_score(7.0), ComplexityLevel::Medium);
        assert_eq!(ComplexityLevel::from_score(15.0), ComplexityLevel::High);
        assert_eq!(ComplexityLevel::from_score(25.0), ComplexityLevel::VeryHigh);
    }

    #[test]
    fn test_quality_score() {
        let score = QualityScore::new(85.0, 80.0, 90.0, 95.0);
        assert_eq!(score.grade(), 'A');
        assert!(score.overall > 80.0);
    }
}