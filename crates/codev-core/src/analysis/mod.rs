//! Code Analysis and Project Understanding
//!
//! This module provides comprehensive code analysis capabilities including:
//! - Project structure analysis
//! - Code quality assessment
//! - Dependency analysis
//! - Language detection and parsing
//! - Metrics collection

pub mod analyzer;
pub mod project;
pub mod metrics;
pub mod parser;

// Re-export main types
pub use analyzer::CodeAnalyzer;
pub use project::ProjectAnalyzer;
pub use metrics::{CodeMetrics, ProjectMetrics};
pub use parser::{LanguageParser, ParseResult};

use codev_shared::{CodeAnalysis, Language, Result};
use std::path::Path;

/// Trait for analyzing different types of code artifacts
pub trait Analyzer {
    /// Analyze a single file
    async fn analyze_file(&self, path: &Path) -> Result<CodeAnalysis>;

    /// Analyze multiple files
    async fn analyze_files(&self, paths: &[&Path]) -> Result<Vec<CodeAnalysis>>;

    /// Get supported languages
    fn supported_languages(&self) -> Vec<Language>;
}

/// Code complexity levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

impl ComplexityLevel {
    /// Convert from numeric score
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s < 5.0 => ComplexityLevel::Low,
            s if s < 10.0 => ComplexityLevel::Medium,
            s if s < 20.0 => ComplexityLevel::High,
            _ => ComplexityLevel::VeryHigh,
        }
    }

    /// Get color for UI display
    pub fn color(&self) -> &'static str {
        match self {
            ComplexityLevel::Low => "green",
            ComplexityLevel::Medium => "yellow",
            ComplexityLevel::High => "orange",
            ComplexityLevel::VeryHigh => "red",
        }
    }
}

/// Quality score for code
#[derive(Debug, Clone)]
pub struct QualityScore {
    pub overall: f32,
    pub maintainability: f32,
    pub readability: f32,
    pub performance: f32,
    pub security: f32,
}

impl QualityScore {
    /// Create a new quality score
    pub fn new(
        maintainability: f32,
        readability: f32,
        performance: f32,
        security: f32,
    ) -> Self {
        let overall = (maintainability + readability + performance + security) / 4.0;

        Self {
            overall,
            maintainability,
            readability,
            performance,
            security,
        }
    }

    /// Get grade (A-F) for overall score
    pub fn grade(&self) -> char {
        match self.overall {
            s if s >= 90.0 => 'A',
            s if s >= 80.0 => 'B',
            s if s >= 70.0 => 'C',
            s if s >= 60.0 => 'D',
            _ => 'F',
        }
    }
}