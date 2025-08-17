//! Code Analyzer for individual files and code snippets
//!
//! This module provides detailed analysis of individual code files including:
//! - Syntax analysis
//! - Complexity metrics
//! - Code quality assessment
//! - Issue detection

use crate::analysis::{Analyzer, ComplexityLevel, QualityScore};
use codev_shared::{
    CodeAnalysis, CodeIssue, CodeSuggestion, CodeChange, Language, Result, CodevError,
    IssueSeverity, IssueCategory
};
use std::path::{Path, PathBuf};
use syn::{File, Item, ItemFn, visit::Visit};
use tracing::{debug, instrument};

/// Code analyzer for individual files
pub struct CodeAnalyzer {
    /// Supported languages
    supported_languages: Vec<Language>,
}

impl CodeAnalyzer {
    /// Create a new code analyzer
    pub fn new() -> Result<Self> {
        Ok(Self {
            supported_languages: vec![
                Language::Rust,
                Language::JavaScript,
                Language::TypeScript,
                Language::Python,
                Language::Go,
                Language::Java,
                Language::Cpp,
                Language::C,
            ],
        })
    }

    /// Analyze Rust code specifically
    #[instrument(skip(self, content))]
    pub async fn analyze_rust_code(&self, content: &str, file_path: &Path) -> Result<CodeAnalysis> {
        debug!("Analyzing Rust code: {}", file_path.display());

        let mut analysis = CodeAnalysis {
            language: Language::Rust,
            file_path: file_path.to_path_buf(),
            content: content.to_string(),
            lines_of_code: content.lines().count(),
            complexity_score: None,
            issues: Vec::new(),
            suggestions: Vec::new(),
        };

        // Parse syntax tree
        match syn::parse_file(content) {
            Ok(syntax_tree) => {
                // Analyze the syntax tree
                let mut visitor = RustAnalysisVisitor::new();
                visitor.visit_file(&syntax_tree);

                // Calculate complexity
                analysis.complexity_score = Some(visitor.calculate_complexity());

                // Generate suggestions based on analysis
                analysis.suggestions = visitor.generate_suggestions();

                // Detect issues
                analysis.issues = visitor.detect_issues();
            }
            Err(e) => {
                analysis.issues.push(CodeIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Syntax,
                    message: format!("Syntax error: {}", e),
                    line: None,
                    column: None,
                    suggested_fix: None,
                });
            }
        }

        Ok(analysis)
    }

    /// Analyze JavaScript/TypeScript code
    pub async fn analyze_js_code(&self, content: &str, file_path: &Path) -> Result<CodeAnalysis> {
        // Simplified JS analysis - in a full implementation this would use a JS parser
        let language = if file_path.extension().and_then(|s| s.to_str()) == Some("ts") {
            Language::TypeScript
        } else {
            Language::JavaScript
        };

        let lines_of_code = content.lines().count();
        let complexity_score = self.estimate_js_complexity(content);

        Ok(CodeAnalysis {
            language,
            file_path: file_path.to_path_buf(),
            content: content.to_string(),
            lines_of_code,
            complexity_score: Some(complexity_score),
            issues: self.detect_js_issues(content),
            suggestions: self.generate_js_suggestions(content),
        })
    }

    /// Estimate JavaScript complexity (simplified)
    fn estimate_js_complexity(&self, content: &str) -> f32 {
        let mut complexity = 1.0; // Base complexity

        // Count control flow statements
        for line in content.lines() {
            let line = line.trim();
            if line.contains("if ") || line.contains("else") {
                complexity += 1.0;
            }
            if line.contains("for ") || line.contains("while ") {
                complexity += 1.0;
            }
            if line.contains("switch ") {
                complexity += 1.0;
            }
            if line.contains("case ") {
                complexity += 0.5;
            }
            if line.contains("try ") || line.contains("catch ") {
                complexity += 1.0;
            }
        }

        complexity
    }

    /// Detect JavaScript issues (simplified)
    fn detect_js_issues(&self, content: &str) -> Vec<CodeIssue> {
        let mut issues = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num + 1;

            // Check for var usage (should use let/const)
            if line.contains(" var ") {
                issues.push(CodeIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Style,
                    message: "Use 'let' or 'const' instead of 'var'".to_string(),
                    line: Some(line_num),
                    column: None,
                    suggested_fix: Some(line.replace(" var ", " let ")),
                });
            }

            // Check for == usage (should use ===)
            if line.contains(" == ") && !line.contains(" === ") {
                issues.push(CodeIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Bug,
                    message: "Use strict equality '===' instead of '=='".to_string(),
                    line: Some(line_num),
                    column: None,
                    suggested_fix: Some(line.replace(" == ", " === ")),
                });
            }

            // Check for console.log (potential debug leftover)
            if line.contains("console.log") {
                issues.push(CodeIssue {
                    severity: IssueSeverity::Info,
                    category: IssueCategory::Style,
                    message: "Remove debug console.log statement".to_string(),
                    line: Some(line_num),
                    column: None,
                    suggested_fix: None,
                });
            }
        }

        issues
    }

    /// Generate JavaScript suggestions (simplified)
    fn generate_js_suggestions(&self, content: &str) -> Vec<CodeSuggestion> {
        let mut suggestions = Vec::new();

        if !content.contains("'use strict'") && !content.contains("\"use strict\"") {
            suggestions.push(CodeSuggestion {
                title: "Add strict mode".to_string(),
                description: "Consider adding 'use strict' for better error checking".to_string(),
                code_change: None,
                confidence: 0.7,
            });
        }

        if content.lines().count() > 100 {
            suggestions.push(CodeSuggestion {
                title: "Consider splitting large file".to_string(),
                description: "This file is quite large. Consider splitting it into smaller modules.".to_string(),
                code_change: None,
                confidence: 0.6,
            });
        }

        suggestions
    }
}

impl Analyzer for CodeAnalyzer {
    async fn analyze_file(&self, path: &Path) -> Result<CodeAnalysis> {
        debug!("Analyzing file: {}", path.display());

        if !path.exists() {
            return Err(CodevError::NotFound {
                resource: path.display().to_string(),
            });
        }

        let content = tokio::fs::read_to_string(path).await?;
        let language = Language::from_extension(
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
        );

        match language {
            Language::Rust => self.analyze_rust_code(&content, path).await,
            Language::JavaScript | Language::TypeScript => {
                self.analyze_js_code(&content, path).await
            }
            Language::Unknown => {
                // Generic analysis for unknown languages
                Ok(CodeAnalysis {
                    language: Language::Unknown,
                    file_path: path.to_path_buf(),
                    content,
                    lines_of_code: content.lines().count(),
                    complexity_score: None,
                    issues: Vec::new(),
                    suggestions: Vec::new(),
                })
            }
            _ => {
                // Other languages not yet implemented
                Err(CodevError::Analysis {
                    message: format!("Analysis not yet implemented for language: {:?}", language),
                })
            }
        }
    }

    async fn analyze_files(&self, paths: &[&Path]) -> Result<Vec<CodeAnalysis>> {
        let mut analyses = Vec::new();

        for path in paths {
            match self.analyze_file(path).await {
                Ok(analysis) => analyses.push(analysis),
                Err(e) => {
                    // Log error but continue with other files
                    tracing::warn!("Failed to analyze {}: {}", path.display(), e);
                }
            }
        }

        Ok(analyses)
    }

    fn supported_languages(&self) -> Vec<Language> {
        self.supported_languages.clone()
    }
}

/// Visitor for analyzing Rust syntax trees
struct RustAnalysisVisitor {
    function_count: usize,
    total_complexity: f32,
    max_function_complexity: f32,
    issues: Vec<CodeIssue>,
    suggestions: Vec<CodeSuggestion>,
    current_line: usize,
}

impl RustAnalysisVisitor {
    fn new() -> Self {
        Self {
            function_count: 0,
            total_complexity: 0.0,
            max_function_complexity: 0.0,
            issues: Vec::new(),
            suggestions: Vec::new(),
            current_line: 1,
        }
    }

    fn calculate_complexity(&self) -> f32 {
        if self.function_count > 0 {
            self.total_complexity / self.function_count as f32
        } else {
            1.0
        }
    }

    fn calculate_function_complexity(&self, func: &ItemFn) -> f32 {
        let mut complexity = 1.0; // Base complexity

        // This is a simplified complexity calculation
        // In a full implementation, this would traverse the function body
        // and count control flow statements, match arms, etc.

        complexity += func.sig.inputs.len() as f32 * 0.1; // Parameter complexity

        // TODO: Implement proper cyclomatic complexity calculation
        // For now, estimate based on function size
        complexity
    }

    fn detect_issues(&self) -> Vec<CodeIssue> {
        self.issues.clone()
    }

    fn generate_suggestions(&self) -> Vec<CodeSuggestion> {
        let mut suggestions = self.suggestions.clone();

        // Add general suggestions based on analysis
        if self.max_function_complexity > 10.0 {
            suggestions.push(CodeSuggestion {
                title: "Reduce function complexity".to_string(),
                description: "Some functions have high complexity. Consider breaking them into smaller functions.".to_string(),
                code_change: None,
                confidence: 0.8,
            });
        }

        if self.function_count > 20 {
            suggestions.push(CodeSuggestion {
                title: "Consider module organization".to_string(),
                description: "This file has many functions. Consider organizing them into modules.".to_string(),
                code_change: None,
                confidence: 0.7,
            });
        }

        suggestions
    }
}

impl<'ast> Visit<'ast> for RustAnalysisVisitor {
    fn visit_item_fn(&mut self, func: &'ast ItemFn) {
        self.function_count += 1;

        let func_complexity = self.calculate_function_complexity(func);
        self.total_complexity += func_complexity;

        if func_complexity > self.max_function_complexity {
            self.max_function_complexity = func_complexity;
        }

        // Check for potential issues
        let func_name = func.sig.ident.to_string();

        // Check function naming conventions
        if func_name.chars().any(|c| c.is_uppercase()) && func_name != "main" {
            self.issues.push(CodeIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::Style,
                message: "Function names should use snake_case".to_string(),
                line: None, // TODO: Extract line number from span
                column: None,
                suggested_fix: Some(format!("Rename to: {}", to_snake_case(&func_name))),
            });
        }

        // Check for very long functions
        if func_complexity > 15.0 {
            self.issues.push(CodeIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::Maintainability,
                message: format!("Function '{}' has high complexity ({})", func_name, func_complexity),
                line: None,
                column: None,
                suggested_fix: Some("Consider breaking this function into smaller functions".to_string()),
            });
        }

        // Continue visiting
        syn::visit::visit_item_fn(self, func);
    }

    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Struct(item_struct) => {
                let struct_name = item_struct.ident.to_string();

                // Check struct naming conventions
                if !struct_name.chars().next().unwrap_or('a').is_uppercase() {
                    self.issues.push(CodeIssue {
                        severity: IssueSeverity::Warning,
                        category: IssueCategory::Style,
                        message: "Struct names should use PascalCase".to_string(),
                        line: None,
                        column: None,
                        suggested_fix: Some(format!("Rename to: {}", to_pascal_case(&struct_name))),
                    });
                }
            }
            Item::Enum(item_enum) => {
                let enum_name = item_enum.ident.to_string();

                // Check enum naming conventions
                if !enum_name.chars().next().unwrap_or('a').is_uppercase() {
                    self.issues.push(CodeIssue {
                        severity: IssueSeverity::Warning,
                        category: IssueCategory::Style,
                        message: "Enum names should use PascalCase".to_string(),
                        line: None,
                        column: None,
                        suggested_fix: Some(format!("Rename to: {}", to_pascal_case(&enum_name))),
                    });
                }
            }
            _ => {}
        }

        // Continue visiting
        syn::visit::visit_item(self, item);
    }
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_char_was_uppercase = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_char_was_uppercase {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_char_was_uppercase = true;
        } else {
            result.push(c);
            prev_char_was_uppercase = false;
        }
    }

    result
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_rust_code_analysis() {
        let rust_code = r#"
fn main() {
    println!("Hello, world!");
}

fn BadFunctionName() {
    // This should trigger a naming convention warning
}

struct badStructName {
    field: i32,
}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(rust_code.as_bytes()).unwrap();

        let analyzer = CodeAnalyzer::new().unwrap();
        let analysis = analyzer.analyze_file(temp_file.path()).await.unwrap();

        assert_eq!(analysis.language, Language::Rust);
        assert!(analysis.complexity_score.is_some());
        assert!(!analysis.issues.is_empty()); // Should have naming convention issues
    }

    #[tokio::test]
    async fn test_javascript_analysis() {
        let js_code = r#"
var x = 5;
if (x == 5) {
    console.log("Debug output");
}
"#;

        let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
        temp_file.write_all(js_code.as_bytes()).unwrap();

        let analyzer = CodeAnalyzer::new().unwrap();
        let analysis = analyzer.analyze_file(temp_file.path()).await.unwrap();

        assert_eq!(analysis.language, Language::JavaScript);
        assert!(analysis.complexity_score.is_some());

        // Should detect var usage, == usage, and console.log
        assert!(analysis.issues.len() >= 3);
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(to_snake_case("BadFunctionName"), "bad_function_name");
        assert_eq!(to_snake_case("HTMLParser"), "html_parser");
        assert_eq!(to_snake_case("already_snake"), "already_snake");

        assert_eq!(to_pascal_case("bad_struct_name"), "BadStructName");
        assert_eq!(to_pascal_case("already_pascal"), "AlreadyPascal");
        assert_eq!(to_pascal_case("PascalCase"), "PascalCase");
    }

    #[test]
    fn test_complexity_calculation() {
        let analyzer = CodeAnalyzer::new().unwrap();

        let simple_js = "function test() { return 1; }";
        let complex_js = r#"
function complex() {
    if (a) {
        for (let i = 0; i < 10; i++) {
            if (b) {
                switch (c) {
                    case 1: break;
                    case 2: break;
                    default: break;
                }
            }
        }
    }
}
"#;

        let simple_complexity = analyzer.estimate_js_complexity(simple_js);
        let complex_complexity = analyzer.estimate_js_complexity(complex_js);

        assert!(complex_complexity > simple_complexity);
    }
}