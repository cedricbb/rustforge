//! Language Parsers for Code Analysis
//!
//! This module provides language-specific parsers for extracting structural information:
//! - Abstract Syntax Tree (AST) parsing
//! - Symbol extraction
//! - Import/dependency analysis
//! - Structure detection

use codev_shared::{Language, Result, CodevError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Result of parsing a code file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// Programming language
    pub language: Language,

    /// Parse success status
    pub success: bool,

    /// Parse errors if any
    pub errors: Vec<ParseError>,

    /// Extracted symbols
    pub symbols: Vec<Symbol>,

    /// Import/dependency statements
    pub imports: Vec<Import>,

    /// Code structure information
    pub structure: CodeStructure,
}

/// Parse error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    /// Error message
    pub message: String,

    /// Line number (1-based)
    pub line: Option<usize>,

    /// Column number (1-based)
    pub column: Option<usize>,

    /// Error severity
    pub severity: ErrorSeverity,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

/// Extracted symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,

    /// Symbol type
    pub symbol_type: SymbolType,

    /// Line where symbol is defined
    pub line: Option<usize>,

    /// Column where symbol is defined
    pub column: Option<usize>,

    /// Visibility/accessibility
    pub visibility: Visibility,

    /// Documentation comment
    pub documentation: Option<String>,

    /// Symbol attributes/annotations
    pub attributes: Vec<String>,
}

/// Types of symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Method,
    Struct,
    Class,
    Interface,
    Enum,
    Variable,
    Constant,
    Type,
    Module,
    Namespace,
}

/// Symbol visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

/// Import/dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    /// Module/package name
    pub module: String,

    /// Specific imports from module
    pub items: Vec<String>,

    /// Import alias
    pub alias: Option<String>,

    /// Line number
    pub line: Option<usize>,

    /// Whether this is a relative import
    pub is_relative: bool,
}

/// Code structure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStructure {
    /// File-level modules/namespaces
    pub modules: Vec<String>,

    /// Main classes/structs
    pub classes: Vec<String>,

    /// Top-level functions
    pub functions: Vec<String>,

    /// Nested structure depth
    pub max_depth: usize,

    /// Code organization pattern
    pub pattern: OrganizationPattern,
}

/// Code organization patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrganizationPattern {
    Procedural,
    ObjectOriented,
    Functional,
    Modular,
    Mixed,
}

/// Language parser trait
pub trait LanguageParser: Send + Sync {
    /// Parse code content
    fn parse(&self, content: &str) -> Result<ParseResult>;

    /// Get supported language
    fn language(&self) -> Language;

    /// Check if this parser can handle the given file
    fn can_parse(&self, path: &Path) -> bool;
}

/// Rust language parser
pub struct RustParser;

impl LanguageParser for RustParser {
    fn parse(&self, content: &str) -> Result<ParseResult> {
        let mut result = ParseResult {
            language: Language::Rust,
            success: false,
            errors: Vec::new(),
            symbols: Vec::new(),
            imports: Vec::new(),
            structure: CodeStructure {
                modules: Vec::new(),
                classes: Vec::new(),
                functions: Vec::new(),
                max_depth: 0,
                pattern: OrganizationPattern::Modular,
            },
        };

        // Parse with syn
        match syn::parse_file(content) {
            Ok(syntax_tree) => {
                result.success = true;

                // Extract symbols and structure
                let mut visitor = RustSymbolVisitor::new();
                visitor.visit_file(&syntax_tree);

                result.symbols = visitor.symbols;
                result.imports = visitor.imports;
                result.structure = visitor.structure;
            }
            Err(e) => {
                result.errors.push(ParseError {
                    message: format!("Syntax error: {}", e),
                    line: None, // syn errors don't provide easy line access
                    column: None,
                    severity: ErrorSeverity::Error,
                });
            }
        }

        Ok(result)
    }

    fn language(&self) -> Language {
        Language::Rust
    }

    fn can_parse(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rs")
            .unwrap_or(false)
    }
}

/// JavaScript/TypeScript parser (simplified)
pub struct JavaScriptParser;

impl LanguageParser for JavaScriptParser {
    fn parse(&self, content: &str) -> Result<ParseResult> {
        let language = Language::JavaScript; // Could detect TypeScript by file extension

        let mut result = ParseResult {
            language,
            success: true, // Assume success for simplified parser
            errors: Vec::new(),
            symbols: Vec::new(),
            imports: Vec::new(),
            structure: CodeStructure {
                modules: Vec::new(),
                classes: Vec::new(),
                functions: Vec::new(),
                max_depth: 0,
                pattern: OrganizationPattern::Mixed,
            },
        };

        // Simple regex-based parsing (in production, use a proper JS parser)
        self.extract_js_symbols(content, &mut result);
        self.extract_js_imports(content, &mut result);
        self.analyze_js_structure(content, &mut result);

        Ok(result)
    }

    fn language(&self) -> Language {
        Language::JavaScript
    }

    fn can_parse(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            matches!(ext, "js" | "ts" | "jsx" | "tsx")
        } else {
            false
        }
    }
}

impl JavaScriptParser {
    fn extract_js_symbols(&self, content: &str, result: &mut ParseResult) {
        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num + 1;
            let trimmed = line.trim();

            // Function declarations
            if let Some(func_name) = self.extract_function_name(trimmed) {
                result.symbols.push(Symbol {
                    name: func_name,
                    symbol_type: SymbolType::Function,
                    line: Some(line_num),
                    column: None,
                    visibility: Visibility::Public, // JS doesn't have explicit visibility
                    documentation: None,
                    attributes: Vec::new(),
                });
            }

            // Class declarations
            if let Some(class_name) = self.extract_class_name(trimmed) {
                result.symbols.push(Symbol {
                    name: class_name,
                    symbol_type: SymbolType::Class,
                    line: Some(line_num),
                    column: None,
                    visibility: Visibility::Public,
                    documentation: None,
                    attributes: Vec::new(),
                });
            }

            // Variable declarations
            if let Some(var_name) = self.extract_variable_name(trimmed) {
                result.symbols.push(Symbol {
                    name: var_name,
                    symbol_type: SymbolType::Variable,
                    line: Some(line_num),
                    column: None,
                    visibility: Visibility::Public,
                    documentation: None,
                    attributes: Vec::new(),
                });
            }
        }
    }

    fn extract_function_name(&self, line: &str) -> Option<String> {
        // Simple regex-like extraction for function names
        if line.starts_with("function ") {
            if let Some(start) = line.find("function ") {
                let rest = &line[start + 9..];
                if let Some(end) = rest.find('(') {
                    return Some(rest[..end].trim().to_string());
                }
            }
        }

        // Arrow functions: const name = () =>
        if line.contains(" = ") && line.contains(" => ") {
            if let Some(eq_pos) = line.find(" = ") {
                let before_eq = &line[..eq_pos];
                if let Some(name_start) = before_eq.rfind(' ') {
                    return Some(before_eq[name_start + 1..].trim().to_string());
                } else if before_eq.starts_with("const ") || before_eq.starts_with("let ") || before_eq.starts_with("var ") {
                    let name = before_eq.split_whitespace().nth(1)?;
                    return Some(name.to_string());
                }
            }
        }

        None
    }

    fn extract_class_name(&self, line: &str) -> Option<String> {
        if line.starts_with("class ") {
            if let Some(start) = line.find("class ") {
                let rest = &line[start + 6..];
                let name = rest.split_whitespace().next()?;
                return Some(name.to_string());
            }
        }
        None
    }

    fn extract_variable_name(&self, line: &str) -> Option<String> {
        for keyword in ["const ", "let ", "var "] {
            if line.starts_with(keyword) {
                let rest = &line[keyword.len()..];
                if let Some(eq_pos) = rest.find('=') {
                    let name = rest[..eq_pos].trim();
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    fn extract_js_imports(&self, content: &str, result: &mut ParseResult) {
        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num + 1;
            let trimmed = line.trim();

            // ES6 imports: import { ... } from 'module'
            if trimmed.starts_with("import ") && trimmed.contains(" from ") {
                if let Some(from_pos) = trimmed.find(" from ") {
                    let import_part = &trimmed[7..from_pos]; // Skip "import "
                    let module_part = &trimmed[from_pos + 6..]; // Skip " from "

                    let module = module_part.trim_matches(|c| c == '\'' || c == '"' || c == ';');
                    let items = self.parse_import_items(import_part);

                    result.imports.push(Import {
                        module: module.to_string(),
                        items,
                        alias: None,
                        line: Some(line_num),
                        is_relative: module.starts_with('.'),
                    });
                }
            }

            // CommonJS requires: const ... = require('module')
            if trimmed.contains("require(") {
                if let Some(start) = trimmed.find("require(") {
                    if let Some(end) = trimmed[start..].find(')') {
                        let module_with_quotes = &trimmed[start + 8..start + end];
                        let module = module_with_quotes.trim_matches(|c| c == '\'' || c == '"');

                        result.imports.push(Import {
                            module: module.to_string(),
                            items: Vec::new(),
                            alias: None,
                            line: Some(line_num),
                            is_relative: module.starts_with('.'),
                        });
                    }
                }
            }
        }
    }

    fn parse_import_items(&self, import_part: &str) -> Vec<String> {
        let trimmed = import_part.trim();

        // Handle different import patterns
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            // Named imports: { a, b, c }
            let inside = &trimmed[1..trimmed.len() - 1];
            inside.split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect()
        } else {
            // Default import
            vec![trimmed.to_string()]
        }
    }

    fn analyze_js_structure(&self, content: &str, result: &mut ParseResult) {
        let mut brace_depth = 0;
        let mut max_depth = 0;
        let mut has_classes = false;
        let mut has_functions = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Count braces for nesting depth
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;
            max_depth = max_depth.max(brace_depth as usize);

            // Check for patterns
            if trimmed.starts_with("class ") {
                has_classes = true;
            }
            if trimmed.contains("function ") || trimmed.contains(" => ") {
                has_functions = true;
            }
        }

        result.structure.max_depth = max_depth;
        result.structure.pattern = match (has_classes, has_functions) {
            (true, true) => OrganizationPattern::Mixed,
            (true, false) => OrganizationPattern::ObjectOriented,
            (false, true) => OrganizationPattern::Functional,
            (false, false) => OrganizationPattern::Procedural,
        };
    }
}

/// Rust symbol visitor using syn
struct RustSymbolVisitor {
    symbols: Vec<Symbol>,
    imports: Vec<Import>,
    structure: CodeStructure,
}

impl RustSymbolVisitor {
    fn new() -> Self {
        Self {
            symbols: Vec::new(),
            imports: Vec::new(),
            structure: CodeStructure {
                modules: Vec::new(),
                classes: Vec::new(),
                functions: Vec::new(),
                max_depth: 0,
                pattern: OrganizationPattern::Modular,
            },
        }
    }

    fn visit_file(&mut self, file: &syn::File) {
        for item in &file.items {
            self.visit_item(item);
        }
    }

    fn visit_item(&mut self, item: &syn::Item) {
        match item {
            syn::Item::Fn(item_fn) => {
                let name = item_fn.sig.ident.to_string();
                self.structure.functions.push(name.clone());

                self.symbols.push(Symbol {
                    name,
                    symbol_type: SymbolType::Function,
                    line: None, // syn spans are complex to extract line numbers from
                    column: None,
                    visibility: self.extract_visibility(&item_fn.vis),
                    documentation: None, // Would need to extract doc comments
                    attributes: item_fn.attrs.iter()
                        .filter_map(|attr| attr.path().get_ident().map(|i| i.to_string()))
                        .collect(),
                });
            }
            syn::Item::Struct(item_struct) => {
                let name = item_struct.ident.to_string();
                self.structure.classes.push(name.clone());

                self.symbols.push(Symbol {
                    name,
                    symbol_type: SymbolType::Struct,
                    line: None,
                    column: None,
                    visibility: self.extract_visibility(&item_struct.vis),
                    documentation: None,
                    attributes: Vec::new(),
                });
            }
            syn::Item::Enum(item_enum) => {
                let name = item_enum.ident.to_string();

                self.symbols.push(Symbol {
                    name,
                    symbol_type: SymbolType::Enum,
                    line: None,
                    column: None,
                    visibility: self.extract_visibility(&item_enum.vis),
                    documentation: None,
                    attributes: Vec::new(),
                });
            }
            syn::Item::Mod(item_mod) => {
                let name = item_mod.ident.to_string();
                self.structure.modules.push(name.clone());

                self.symbols.push(Symbol {
                    name,
                    symbol_type: SymbolType::Module,
                    line: None,
                    column: None,
                    visibility: self.extract_visibility(&item_mod.vis),
                    documentation: None,
                    attributes: Vec::new(),
                });
            }
            syn::Item::Use(item_use) => {
                // Extract use statements as imports
                let import = self.extract_use_import(&item_use.tree);
                self.imports.push(import);
            }
            _ => {} // Handle other items as needed
        }
    }

    fn extract_visibility(&self, vis: &syn::Visibility) -> Visibility {
        match vis {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Restricted(_) => Visibility::Internal,
            syn::Visibility::Inherited => Visibility::Private,
        }
    }

    fn extract_use_import(&self, use_tree: &syn::UseTree) -> Import {
        match use_tree {
            syn::UseTree::Path(use_path) => {
                let module = use_path.ident.to_string();
                Import {
                    module,
                    items: Vec::new(),
                    alias: None,
                    line: None,
                    is_relative: false, // Rust doesn't have relative imports in the same way
                }
            }
            syn::UseTree::Name(use_name) => {
                let module = use_name.ident.to_string();
                Import {
                    module,
                    items: Vec::new(),
                    alias: None,
                    line: None,
                    is_relative: false,
                }
            }
            syn::UseTree::Group(use_group) => {
                // Handle grouped imports like `use std::{fs, io};`
                let items: Vec<String> = use_group.items.iter()
                    .filter_map(|item| match item {
                        syn::UseTree::Name(name) => Some(name.ident.to_string()),
                        _ => None,
                    })
                    .collect();

                Import {
                    module: "grouped".to_string(), // Would need more context to get actual module
                    items,
                    alias: None,
                    line: None,
                    is_relative: false,
                }
            }
            _ => Import {
                module: "unknown".to_string(),
                items: Vec::new(),
                alias: None,
                line: None,
                is_relative: false,
            },
        }
    }
}

/// Parser registry for managing multiple language parsers
pub struct ParserRegistry {
    parsers: HashMap<Language, Box<dyn LanguageParser>>,
}

impl ParserRegistry {
    /// Create a new parser registry with default parsers
    pub fn new() -> Self {
        let mut parsers: HashMap<Language, Box<dyn LanguageParser>> = HashMap::new();

        parsers.insert(Language::Rust, Box::new(RustParser));
        parsers.insert(Language::JavaScript, Box::new(JavaScriptParser));

        Self { parsers }
    }

    /// Get parser for a specific language
    pub fn get_parser(&self, language: Language) -> Option<&dyn LanguageParser> {
        self.parsers.get(&language).map(|p| p.as_ref())
    }

    /// Parse a file using the appropriate parser
    pub fn parse_file(&self, path: &Path, content: &str) -> Result<ParseResult> {
        // Detect language from file extension
        let language = Language::from_extension(
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
        );

        if let Some(parser) = self.get_parser(language) {
            parser.parse(content)
        } else {
            Err(CodevError::Analysis {
                message: format!("No parser available for language: {:?}", language),
            })
        }
    }

    /// Get all supported languages
    pub fn supported_languages(&self) -> Vec<Language> {
        self.parsers.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_parser() {
        let rust_code = r#"
use std::collections::HashMap;

pub struct MyStruct {
    field: i32,
}

impl MyStruct {
    pub fn new() -> Self {
        Self { field: 0 }
    }

    fn private_method(&self) -> i32 {
        self.field
    }
}

pub fn public_function() {
    println!("Hello");
}
"#;

        let parser = RustParser;
        let result = parser.parse(rust_code).unwrap();

        assert!(result.success);
        assert_eq!(result.language, Language::Rust);
        assert!(!result.symbols.is_empty());
        assert!(!result.imports.is_empty());
    }

    #[test]
    fn test_javascript_parser() {
        let js_code = r#"
import { Component } from 'react';
const util = require('./util');

class MyClass {
    constructor() {
        this.value = 0;
    }

    method() {
        return this.value;
    }
}

function helper() {
    return 42;
}

const arrow = () => {
    console.log('arrow function');
};
"#;

        let parser = JavaScriptParser;
        let result = parser.parse(js_code).unwrap();

        assert!(result.success);
        assert_eq!(result.language, Language::JavaScript);
        assert!(!result.symbols.is_empty());
        assert!(!result.imports.is_empty());

        // Check that we found the class and functions
        let symbol_names: Vec<String> = result.symbols.iter().map(|s| s.name.clone()).collect();
        assert!(symbol_names.contains(&"MyClass".to_string()));
        assert!(symbol_names.contains(&"helper".to_string()));
    }

    #[test]
    fn test_parser_registry() {
        let registry = ParserRegistry::new();

        assert!(registry.get_parser(Language::Rust).is_some());
        assert!(registry.get_parser(Language::JavaScript).is_some());
        assert!(registry.get_parser(Language::Python).is_none());

        let supported = registry.supported_languages();
        assert!(supported.contains(&Language::Rust));
        assert!(supported.contains(&Language::JavaScript));
    }

    #[test]
    fn test_symbol_types() {
        let symbol = Symbol {
            name: "test".to_string(),
            symbol_type: SymbolType::Function,
            line: Some(10),
            column: Some(5),
            visibility: Visibility::Public,
            documentation: None,
            attributes: vec!["test".to_string()],
        };

        assert_eq!(symbol.name, "test");
        assert!(matches!(symbol.symbol_type, SymbolType::Function));
        assert!(matches!(symbol.visibility, Visibility::Public));
    }
}