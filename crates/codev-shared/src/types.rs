//! Core types used throughout CoDev.rs

use serde::{ Deserilalize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Programming languages supported by CoDev.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Java,
    Cpp,
    C,
    React,
    Unknown
}

impl Language {
    /// Get file extensions for this language
    pub fn extension(self) -> &[&str] {
        match self {
            Language::Rust => &["rs"],
            Language::JavaScript => &["js", "mjs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Python => &["py", "pyi"],
            Language::Go => &["go"],
            Language::Java => &["java"],
            Language::Cpp => &["cpp", "cxx", "cc", "hpp", "hxx"],
            Language::C => &["c", "h"],
            Language::React => &["react"],
            Language::Unknown => &[],
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "js" | "mjs" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "py" | "pyi" => Language::Python,
            "go" | "goi" => Language::Go,
            "java" | "jav" => Language::Java,
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Language::Cpp,
            "c" | "h" => Language::C,
            "tsx" | "jsx" => Language::React,
            _ => Language::Unknown,
        }
    }
}

/// Environment where CoDev.rs is running
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Environment {
    Development,
    Production,
    Testing,
}

/// Security levels for sandbox execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum SecurityLevel {
    Development, // Full access for local development
    Production, // Restricted sandbox
    Paranoid, // Maximum isolation
}

/// LLM provider identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum ProviderId {
    Ollama,
    OpenAI,
    Claude,
    Mistral,
    Gemini,
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderId::Ollama => write!(f, "ollama"),
            ProviderId::OpenAI => write!(f, "Openai"),
            ProviderId::Claude => write!(f, "claude"),
            ProviderId::Mistral => write!(f, "mistral"),
            ProviderId::Gemini => write!(f, "gemini"),
        }
    }
}

/// Health status of a component
#[derive(Debug, Clone,PartialEq, Deserialize, Serialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

/// Command execution result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

impl CommandResult {
    pub fn new(stdout: String, stderr: String, exit_code: i32) -> Self {
        Self {
            stdout,
            stderr,
            success: exit_code = 0,
            exit_code,
        }
    }

    pub fn success(output: String) -> Self {
        Self {
            stdout: output,
            stderr: String::new(),
            exit_code: 0,
            success: true,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            stdout: String::new(),
            stderr: error,
            exit_code: 1,
            success: false,
        }
    }
}

/// Project context information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectContext {
    pub root_path: PathBuf,
    pub name: String,
    pub languages: Vec<Language>,
    pub git_repository: Option<GitInfo>,
    pub dependencies: HashMap<String, String>,
    pub build_system: Option<BuildSystem>,
}

/// Git repository information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitInfo {
    pub current_branch: String,
    pub remote_url: Option<String>,
    pub is_dirty: bool,
    pub last_commit: Option<String>,
}

/// Build system type
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum BuildSystem {
    Cargo,
    Npm,
    Yarn,
    Pip,
    Go,
    Maven,
    Gradle,
    Make,
    Unknown,
}

/// AI model configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    pub name: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
}

/// Chat message in conversation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<MessageMetadata>,
}

/// Role of a chat message
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Additional metadata for message
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageMetadata {
    pub provider: ProviderId,
    pub model: String,
    pub token_used: Option<usize>,
    pub response_time: Option<std::time::Duration>,
}

/// Code analysis result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeAnalysis {
    pub language: Language,
    pub file_path: PathBuf,
    pub content: String,
    pub lines_of_code: usize,
    pub complexity_score: Option<f32>,
    pub issues: Vec<CodeIssue>,
    pub suggestions: Vec<CodeSuggestion>,
}

/// Code issue detected during analysis
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggested_fix: Option<String>,
}

/// Severity of a code issue
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Categorie of a code issue
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum IssueCategory {
    Syntax,
    Performance,
    Security,
    Maintainability,
    Style,
    Bug,
    Deprecated,
}

/// Code improvement suggestion
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeSuggestion {
    pub title: String,
    pub description: String,
    pub code_change: Option<CodeChange>,
    pub confidence: f32, // 0.0 to 1.0
}

/// A suggested code change
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeChange {
    pub file_path: PathBuf,
    pub old_code: String,
    pub new_code: String,
    pub start_line: usize,
    pub end_line: usize,
}
