//! Code and Project Metrics Collection
//!
//! This module provides comprehensive metrics collection for code quality assessment:
//! - Lines of code metrics
//! - Complexity metrics
//! - Maintainability indices
//! - Technical debt assessment

use codev_shared::{Language, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Code metrics for individual files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    /// Programming language
    pub language: Language,

    /// Lines of code breakdown
    pub loc: LinesOfCode,

    /// Complexity metrics
    pub complexity: ComplexityMetrics,

    /// Maintainability metrics
    pub maintainability: MaintainabilityMetrics,

    /// Code quality score (0-100)
    pub quality_score: f32,
}

/// Lines of code breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinesOfCode {
    /// Total lines in file
    pub total: usize,

    /// Lines containing source code
    pub source: usize,

    /// Comment lines
    pub comments: usize,

    /// Blank lines
    pub blank: usize,

    /// Mixed lines (code + comments)
    pub mixed: usize,
}

/// Complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity
    pub cyclomatic: f32,

    /// Cognitive complexity
    pub cognitive: f32,

    /// Halstead complexity
    pub halstead: Option<HalsteadMetrics>,

    /// Function complexity statistics
    pub function_stats: FunctionComplexityStats,
}

/// Halstead complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalsteadMetrics {
    /// Number of distinct operators
    pub distinct_operators: usize,

    /// Number of distinct operands
    pub distinct_operands: usize,

    /// Total operators
    pub total_operators: usize,

    /// Total operands
    pub total_operands: usize,

    /// Program vocabulary
    pub vocabulary: usize,

    /// Program length
    pub length: usize,

    /// Calculated program length
    pub calculated_length: f32,

    /// Volume
    pub volume: f32,

    /// Difficulty
    pub difficulty: f32,

    /// Effort
    pub effort: f32,

    /// Time required to program (seconds)
    pub time: f32,

    /// Number of delivered bugs
    pub bugs: f32,
}

/// Function complexity statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexityStats {
    /// Total number of functions
    pub total_functions: usize,

    /// Average complexity per function
    pub average_complexity: f32,

    /// Maximum complexity found
    pub max_complexity: f32,

    /// Functions with high complexity (>10)
    pub high_complexity_functions: usize,
}

/// Maintainability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintainabilityMetrics {
    /// Maintainability Index (0-100)
    pub maintainability_index: f32,

    /// Technical debt ratio (0-100)
    pub technical_debt_ratio: f32,

    /// Code duplication percentage
    pub duplication_percentage: f32,

    /// Test coverage percentage
    pub test_coverage: Option<f32>,

    /// Documentation coverage percentage
    pub documentation_coverage: f32,
}

/// Project-level metrics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetrics {
    /// Overall project statistics
    pub overview: ProjectOverview,

    /// Metrics by language
    pub by_language: HashMap<Language, LanguageMetrics>,

    /// Quality trends over time
    pub trends: Option<QualityTrends>,

    /// Technical debt assessment
    pub technical_debt: TechnicalDebtAssessment,
}

/// Project overview statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOverview {
    /// Total files analyzed
    pub total_files: usize,

    /// Total lines of code
    pub total_loc: usize,

    /// Total source lines
    pub source_loc: usize,

    /// Comment lines
    pub comment_loc: usize,

    /// Overall quality score
    pub quality_score: f32,

    /// Overall complexity score
    pub complexity_score: f32,

    /// Overall maintainability index
    pub maintainability_index: f32,
}

/// Language-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMetrics {
    /// Language
    pub language: Language,

    /// Number of files
    pub file_count: usize,

    /// Percentage of total project
    pub percentage: f32,

    /// Aggregated metrics
    pub metrics: CodeMetrics,

    /// File-level breakdown
    pub files: Vec<FileMetrics>,
}

/// Individual file metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    /// File path
    pub path: String,

    /// File size in bytes
    pub size: u64,

    /// Code metrics
    pub metrics: CodeMetrics,
}

/// Quality trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrends {
    /// Historical quality scores
    pub quality_history: Vec<QualityDataPoint>,

    /// Complexity trend
    pub complexity_trend: TrendDirection,

    /// Technical debt trend
    pub debt_trend: TrendDirection,

    /// Test coverage trend
    pub coverage_trend: TrendDirection,
}

/// Quality data point for trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDataPoint {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Quality score at this point
    pub quality_score: f32,

    /// Complexity score
    pub complexity_score: f32,

    /// Technical debt ratio
    pub debt_ratio: f32,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
    Unknown,
}

/// Technical debt assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalDebtAssessment {
    /// Total debt in hours
    pub total_debt_hours: f32,

    /// Debt ratio (0-100)
    pub debt_ratio: f32,

    /// Debt by category
    pub debt_by_category: HashMap<DebtCategory, f32>,

    /// Most problematic files
    pub hotspots: Vec<DebtHotspot>,

    /// Recommended actions
    pub recommendations: Vec<String>,
}

/// Technical debt categories
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum DebtCategory {
    Complexity,
    Duplication,
    Coverage,
    Documentation,
    Maintainability,
    Security,
    Performance,
}

/// Technical debt hotspot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtHotspot {
    /// File path
    pub file_path: String,

    /// Debt score for this file
    pub debt_score: f32,

    /// Primary debt category
    pub primary_category: DebtCategory,

    /// Estimated effort to fix (hours)
    pub effort_hours: f32,

    /// Impact level
    pub impact: ImpactLevel,
}

/// Impact level for debt items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Metrics calculator
pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Calculate metrics for source code content
    pub fn calculate_code_metrics(content: &str, language: Language) -> CodeMetrics {
        let loc = Self::calculate_loc(content);
        let complexity = Self::calculate_complexity(content, language);
        let maintainability = Self::calculate_maintainability(&loc, &complexity);
        let quality_score = Self::calculate_quality_score(&loc, &complexity, &maintainability);

        CodeMetrics {
            language,
            loc,
            complexity,
            maintainability,
            quality_score,
        }
    }

    /// Calculate lines of code breakdown
    fn calculate_loc(content: &str) -> LinesOfCode {
        let mut source = 0;
        let mut comments = 0;
        let mut blank = 0;
        let mut mixed = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                blank += 1;
            } else if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                if line.trim_start().chars().any(|c| !c.is_whitespace() && c != '/' && c != '*' && c != '#') {
                    mixed += 1;
                } else {
                    comments += 1;
                }
            } else {
                if trimmed.contains("//") || trimmed.contains('#') {
                    mixed += 1;
                } else {
                    source += 1;
                }
            }
        }

        let total = content.lines().count();

        LinesOfCode {
            total,
            source,
            comments,
            blank,
            mixed,
        }
    }

    /// Calculate complexity metrics
    fn calculate_complexity(content: &str, language: Language) -> ComplexityMetrics {
        let cyclomatic = Self::calculate_cyclomatic_complexity(content, language);
        let cognitive = Self::calculate_cognitive_complexity(content, language);
        let halstead = Self::calculate_halstead_metrics(content, language);
        let function_stats = Self::calculate_function_stats(content, language);

        ComplexityMetrics {
            cyclomatic,
            cognitive,
            halstead,
            function_stats,
        }
    }

    /// Calculate cyclomatic complexity
    fn calculate_cyclomatic_complexity(content: &str, _language: Language) -> f32 {
        let mut complexity = 1.0; // Base complexity

        // Count decision points
        for line in content.lines() {
            let line = line.trim();

            // Control flow keywords that increase complexity
            if line.contains("if ") || line.contains("if(") {
                complexity += 1.0;
            }
            if line.contains("else if") || line.contains("elif") {
                complexity += 1.0;
            }
            if line.contains("while ") || line.contains("while(") {
                complexity += 1.0;
            }
            if line.contains("for ") || line.contains("for(") {
                complexity += 1.0;
            }
            if line.contains("switch ") || line.contains("match ") {
                complexity += 1.0;
            }
            if line.contains("case ") {
                complexity += 1.0;
            }
            if line.contains("catch ") || line.contains("except ") {
                complexity += 1.0;
            }
            if line.contains("&& ") || line.contains("|| ") {
                complexity += 0.5; // Logical operators add some complexity
            }
        }

        complexity
    }

    /// Calculate cognitive complexity (simplified)
    fn calculate_cognitive_complexity(content: &str, language: Language) -> f32 {
        // Cognitive complexity is similar to cyclomatic but accounts for nesting
        // This is a simplified implementation
        Self::calculate_cyclomatic_complexity(content, language) * 1.2
    }

    /// Calculate Halstead metrics (simplified)
    fn calculate_halstead_metrics(content: &str, _language: Language) -> Option<HalsteadMetrics> {
        // This is a very simplified Halstead calculation
        // A full implementation would need proper tokenization

        let operators = ["=", "+", "-", "*", "/", "==", "!=", "<", ">", "&&", "||"];
        let mut operator_counts = HashMap::new();
        let mut operand_counts = HashMap::new();

        for line in content.lines() {
            for op in &operators {
                let count = line.matches(op).count();
                *operator_counts.entry(op).or_insert(0) += count;
            }
        }

        let distinct_operators = operator_counts.len();
        let total_operators: usize = operator_counts.values().sum();

        // Simplified operand counting (words that aren't keywords)
        let keywords = ["if", "else", "while", "for", "function", "return", "var", "let", "const"];
        for line in content.lines() {
            for word in line.split_whitespace() {
                let word = word.trim_matches(|c: char| !c.is_alphanumeric());
                if !word.is_empty() && !keywords.contains(&word) && word.chars().any(|c| c.is_alphabetic()) {
                    *operand_counts.entry(word.to_string()).or_insert(0) += 1;
                }
            }
        }

        let distinct_operands = operand_counts.len();
        let total_operands: usize = operand_counts.values().sum();

        if distinct_operators == 0 || distinct_operands == 0 {
            return None;
        }

        let vocabulary = distinct_operators + distinct_operands;
        let length = total_operators + total_operands;
        let calculated_length = (distinct_operators as f32 * (distinct_operators as f32).log2()) +
            (distinct_operands as f32 * (distinct_operands as f32).log2());
        let volume = length as f32 * (vocabulary as f32).log2();
        let difficulty = (distinct_operators as f32 / 2.0) * (total_operands as f32 / distinct_operands as f32);
        let effort = difficulty * volume;
        let time = effort / 18.0; // Stroud number
        let bugs = effort.powf(2.0/3.0) / 3000.0;

        Some(HalsteadMetrics {
            distinct_operators,
            distinct_operands,
            total_operators,
            total_operands,
            vocabulary,
            length,
            calculated_length,
            volume,
            difficulty,
            effort,
            time,
            bugs,
        })
    }

    /// Calculate function complexity statistics
    fn calculate_function_stats(content: &str, _language: Language) -> FunctionComplexityStats {
        // This is a simplified function counting
        let function_patterns = ["fn ", "function ", "def ", "func "];
        let mut function_count = 0;

        for line in content.lines() {
            for pattern in &function_patterns {
                if line.contains(pattern) {
                    function_count += 1;
                    break;
                }
            }
        }

        // Mock complexity calculation
        let average_complexity = if function_count > 0 { 3.0 } else { 0.0 };
        let max_complexity = 8.0;
        let high_complexity_functions = function_count / 4; // Assume 25% are complex

        FunctionComplexityStats {
            total_functions: function_count,
            average_complexity,
            max_complexity,
            high_complexity_functions,
        }
    }

    /// Calculate maintainability metrics
    fn calculate_maintainability(loc: &LinesOfCode, complexity: &ComplexityMetrics) -> MaintainabilityMetrics {
        // Maintainability Index calculation (simplified Microsoft formula)
        let volume = complexity.halstead.as_ref()
            .map(|h| h.volume)
            .unwrap_or(loc.source as f32 * 2.0);

        let mi = ((171.0 - 5.2 * volume.ln() - 0.23 * complexity.cyclomatic - 16.2 * (loc.source as f32).ln()) / 171.0 * 100.0)
            .max(0.0)
            .min(100.0);

        // Technical debt ratio (simplified)
        let debt_ratio = ((complexity.cyclomatic - 1.0) * 2.0 + (loc.source as f32 / 100.0))
            .min(100.0);

        // Mock duplication and documentation metrics
        let duplication_percentage = 5.0; // Mock value
        let documentation_coverage = (loc.comments as f32 / loc.source as f32 * 100.0).min(100.0);

        MaintainabilityMetrics {
            maintainability_index: mi,
            technical_debt_ratio: debt_ratio,
            duplication_percentage,
            test_coverage: None, // Would need external tool integration
            documentation_coverage,
        }
    }

    /// Calculate overall quality score
    fn calculate_quality_score(
        _loc: &LinesOfCode,
        complexity: &ComplexityMetrics,
        maintainability: &MaintainabilityMetrics,
    ) -> f32 {
        // Weighted combination of different quality factors
        let complexity_score = (20.0 - complexity.cyclomatic).max(0.0) / 20.0 * 100.0;
        let maintainability_score = maintainability.maintainability_index;
        let documentation_score = maintainability.documentation_coverage;

        // Weighted average
        (complexity_score * 0.4 + maintainability_score * 0.4 + documentation_score * 0.2)
            .min(100.0)
            .max(0.0)
    }

    /// Aggregate project metrics from individual file metrics
    pub fn aggregate_project_metrics(file_metrics: Vec<FileMetrics>) -> ProjectMetrics {
        let mut by_language: HashMap<Language, Vec<FileMetrics>> = HashMap::new();

        // Group by language
        for file_metric in file_metrics {
            by_language.entry(file_metric.metrics.language)
                .or_insert_with(Vec::new)
                .push(file_metric);
        }

        // Calculate language-specific metrics
        let mut language_metrics = HashMap::new();
        let mut total_files = 0;
        let mut total_loc = 0;
        let mut total_source_loc = 0;
        let mut total_comment_loc = 0;
        let mut total_quality_score = 0.0;
        let mut total_complexity_score = 0.0;
        let mut total_maintainability = 0.0;

        for (language, files) in by_language {
            let file_count = files.len();
            total_files += file_count;

            // Aggregate metrics for this language
            let mut lang_loc = 0;
            let mut lang_source_loc = 0;
            let mut lang_comment_loc = 0;
            let mut lang_quality = 0.0;
            let mut lang_complexity = 0.0;
            let mut lang_maintainability = 0.0;

            for file in &files {
                lang_loc += file.metrics.loc.total;
                lang_source_loc += file.metrics.loc.source;
                lang_comment_loc += file.metrics.loc.comments;
                lang_quality += file.metrics.quality_score;
                lang_complexity += file.metrics.complexity.cyclomatic;
                lang_maintainability += file.metrics.maintainability.maintainability_index;
            }

            total_loc += lang_loc;
            total_source_loc += lang_source_loc;
            total_comment_loc += lang_comment_loc;
            total_quality_score += lang_quality;
            total_complexity_score += lang_complexity;
            total_maintainability += lang_maintainability;

            let percentage = (lang_source_loc as f32 / total_source_loc.max(1) as f32) * 100.0;

            // Create aggregated metrics for language
            let avg_metrics = CodeMetrics {
                language,
                loc: LinesOfCode {
                    total: lang_loc,
                    source: lang_source_loc,
                    comments: lang_comment_loc,
                    blank: 0, // Simplified
                    mixed: 0,
                },
                complexity: ComplexityMetrics {
                    cyclomatic: lang_complexity / file_count as f32,
                    cognitive: lang_complexity / file_count as f32 * 1.2,
                    halstead: None,
                    function_stats: FunctionComplexityStats {
                        total_functions: 0,
                        average_complexity: 0.0,
                        max_complexity: 0.0,
                        high_complexity_functions: 0,
                    },
                },
                maintainability: MaintainabilityMetrics {
                    maintainability_index: lang_maintainability / file_count as f32,
                    technical_debt_ratio: 0.0,
                    duplication_percentage: 0.0,
                    test_coverage: None,
                    documentation_coverage: 0.0,
                },
                quality_score: lang_quality / file_count as f32,
            };

            language_metrics.insert(language, LanguageMetrics {
                language,
                file_count,
                percentage,
                metrics: avg_metrics,
                files,
            });
        }

        let overview = ProjectOverview {
            total_files,
            total_loc,
            source_loc: total_source_loc,
            comment_loc: total_comment_loc,
            quality_score: total_quality_score / total_files.max(1) as f32,
            complexity_score: total_complexity_score / total_files.max(1) as f32,
            maintainability_index: total_maintainability / total_files.max(1) as f32,
        };

        let technical_debt = Self::assess_technical_debt(&overview);

        ProjectMetrics {
            overview,
            by_language: language_metrics,
            trends: None, // Would need historical data
            technical_debt,
        }
    }

    /// Assess technical debt for the project
    fn assess_technical_debt(overview: &ProjectOverview) -> TechnicalDebtAssessment {
        let debt_ratio = (100.0 - overview.quality_score).max(0.0);
        let total_debt_hours = (overview.source_loc as f32 * debt_ratio / 100.0 * 0.1).max(0.0);

        let mut debt_by_category = HashMap::new();
        debt_by_category.insert(DebtCategory::Complexity, debt_ratio * 0.3);
        debt_by_category.insert(DebtCategory::Maintainability, debt_ratio * 0.3);
        debt_by_category.insert(DebtCategory::Documentation, debt_ratio * 0.2);
        debt_by_category.insert(DebtCategory::Coverage, debt_ratio * 0.2);

        let recommendations = vec![
            "Reduce cyclomatic complexity in complex functions".to_string(),
            "Improve code documentation coverage".to_string(),
            "Add unit tests for better coverage".to_string(),
            "Refactor large files into smaller modules".to_string(),
        ];

        TechnicalDebtAssessment {
            total_debt_hours,
            debt_ratio,
            debt_by_category,
            hotspots: Vec::new(), // Would need individual file analysis
            recommendations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loc_calculation() {
        let code = r#"
// This is a comment
fn main() {
    println!("Hello"); // Inline comment

    // Another comment
}
"#;

        let loc = MetricsCalculator::calculate_loc(code);
        assert!(loc.source > 0);
        assert!(loc.comments > 0);
        assert!(loc.blank > 0);
        assert!(loc.mixed > 0);
        assert_eq!(loc.total, code.lines().count());
    }

    #[test]
    fn test_complexity_calculation() {
        let simple_code = "fn simple() { return 1; }";
        let complex_code = r#"
fn complex() {
    if (condition) {
        while (loop) {
            if (nested) {
                for (item in items) {
                    // Complex logic
                }
            }
        }
    }
}
"#;

        let simple_complexity = MetricsCalculator::calculate_cyclomatic_complexity(simple_code, Language::Rust);
        let complex_complexity = MetricsCalculator::calculate_cyclomatic_complexity(complex_code, Language::Rust);

        assert!(complex_complexity > simple_complexity);
        assert!(simple_complexity >= 1.0);
    }

    #[test]
    fn test_code_metrics_calculation() {
        let code = r#"
/// Documentation comment
fn example_function(x: i32) -> i32 {
    if x > 0 {
        return x * 2;
    } else {
        return 0;
    }
}
"#;

        let metrics = MetricsCalculator::calculate_code_metrics(code, Language::Rust);
        assert_eq!(metrics.language, Language::Rust);
        assert!(metrics.quality_score >= 0.0 && metrics.quality_score <= 100.0);
        assert!(metrics.complexity.cyclomatic >= 1.0);
    }

    #[test]
    fn test_halstead_metrics() {
        let code = "x = a + b * c;";
        let halstead = MetricsCalculator::calculate_halstead_metrics(code, Language::JavaScript);

        if let Some(h) = halstead {
            assert!(h.distinct_operators > 0);
            assert!(h.vocabulary > 0);
            assert!(h.volume > 0.0);
        }
    }
}