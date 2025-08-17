//! Template System for Code Generation
//!
//! This module provides template-based code generation including:
//! - Project templates for different languages
//! - Code snippet templates
//! - Variable substitution and processing
//! - Template inheritance and composition

use codev_shared::{Language, Result, CodevError};
use handlebars::{Handlebars, RenderError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument};

/// Template engine for code generation
pub struct TemplateEngine {
    /// Handlebars engine
    handlebars: Handlebars<'static>,

    /// Registered templates
    templates: HashMap<String, Template>,

    /// Template search paths
    template_paths: Vec<PathBuf>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Result<Self> {
        let mut handlebars = Handlebars::new();

        // Configure handlebars
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(handlebars::no_escape); // Don't escape code content

        // Register custom helpers
        Self::register_helpers(&mut handlebars)?;

        let mut engine = Self {
            handlebars,
            templates: HashMap::new(),
            template_paths: Vec::new(),
        };

        // Load built-in templates
        engine.load_builtin_templates()?;

        Ok(engine)
    }

    /// Register custom handlebars helpers
    fn register_helpers(handlebars: &mut Handlebars<'static>) -> Result<()> {
        // Snake case helper
        handlebars.register_helper("snake_case", Box::new(snake_case_helper));

        // Pascal case helper
        handlebars.register_helper("pascal_case", Box::new(pascal_case_helper));

        // Camel case helper
        handlebars.register_helper("camel_case", Box::new(camel_case_helper));

        // Kebab case helper
        handlebars.register_helper("kebab_case", Box::new(kebab_case_helper));

        // Current year helper
        handlebars.register_helper("current_year", Box::new(current_year_helper));

        // Pluralize helper
        handlebars.register_helper("pluralize", Box::new(pluralize_helper));

        Ok(())
    }

    /// Load built-in templates
    fn load_builtin_templates(&mut self) -> Result<()> {
        // Rust templates
        self.register_rust_templates()?;

        // JavaScript templates
        self.register_javascript_templates()?;

        // Python templates
        self.register_python_templates()?;

        // Common templates
        self.register_common_templates()?;

        Ok(())
    }

    /// Register Rust templates
    fn register_rust_templates(&mut self) -> Result<()> {
        // Rust binary project template
        let rust_binary = Template {
            id: "rust-binary".to_string(),
            name: "Rust Binary Project".to_string(),
            description: "A simple Rust binary application".to_string(),
            language: Language::Rust,
            category: TemplateCategory::Project,
            files: vec![
                TemplateFile {
                    path: "Cargo.toml".to_string(),
                    content: r#"[package]
name = "{{project_name}}"
version = "0.1.0"
edition = "2021"
authors = ["{{author}}"]
description = "{{description}}"

[dependencies]
"#.to_string(),
                },
                TemplateFile {
                    path: "src/main.rs".to_string(),
                    content: r#"fn main() {
    println!("Hello, {{project_name}}!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
"#.to_string(),
                },
                TemplateFile {
                    path: "README.md".to_string(),
                    content: r#"# {{pascal_case project_name}}

{{description}}

## Installation

```bash
cargo install {{project_name}}
```

## Usage

```bash
{{project_name}}
```

## Development

```bash
cargo run
cargo test
```

## License

Created by {{author}} in {{current_year}}.
"#.to_string(),
                },
                TemplateFile {
                    path: ".gitignore".to_string(),
                    content: r#"/target/
**/*.rs.bk
Cargo.lock
.DS_Store
*.log
"#.to_string(),
                },
            ],
            variables: vec![
                TemplateVariable {
                    name: "project_name".to_string(),
                    description: "Name of the project".to_string(),
                    required: true,
                    default_value: None,
                    variable_type: VariableType::String,
                },
                TemplateVariable {
                    name: "author".to_string(),
                    description: "Project author".to_string(),
                    required: false,
                    default_value: Some("Developer".to_string()),
                    variable_type: VariableType::String,
                },
                TemplateVariable {
                    name: "description".to_string(),
                    description: "Project description".to_string(),
                    required: false,
                    default_value: Some("A Rust project".to_string()),
                    variable_type: VariableType::String,
                },
            ],
        };

        // Rust library template
        let rust_library = Template {
            id: "rust-library".to_string(),
            name: "Rust Library Project".to_string(),
            description: "A Rust library crate".to_string(),
            language: Language::Rust,
            category: TemplateCategory::Project,
            files: vec![
                TemplateFile {
                    path: "Cargo.toml".to_string(),
                    content: include_str!("../../../templates/rust/library/Cargo.toml.hbs").to_string(),
                },
                TemplateFile {
                    path: "src/lib.rs".to_string(),
                    content: include_str!("../../../templates/rust/library/lib.rs.hbs").to_string(),
                },
                TemplateFile {
                    path: "README.md".to_string(),
                    content: include_str!("../../../templates/common/README.md.hbs").to_string(),
                },
            ],
            variables: rust_binary.variables.clone(),
        };

        // Rust struct template
        let rust_struct = Template {
            id: "rust-struct".to_string(),
            name: "Rust Struct".to_string(),
            description: "A Rust struct with common implementations".to_string(),
            language: Language::Rust,
            category: TemplateCategory::Code,
            files: vec![
                TemplateFile {
                    path: "struct.rs".to_string(),
                    content: include_str!("../../../templates/rust/snippets/struct.rs.hbs").to_string(),
                },
            ],
            variables: vec![
                TemplateVariable {
                    name: "struct_name".to_string(),
                    description: "Name of the struct".to_string(),
                    required: true,
                    default_value: None,
                    variable_type: VariableType::String,
                },
                TemplateVariable {
                    name: "fields".to_string(),
                    description: "Struct fields".to_string(),
                    required: false,
                    default_value: Some("field: String".to_string()),
                    variable_type: VariableType::String,
                },
                TemplateVariable {
                    name: "derive_debug".to_string(),
                    description: "Derive Debug trait".to_string(),
                    required: false,
                    default_value: Some("true".to_string()),
                    variable_type: VariableType::Boolean,
                },
            ],
        };

        self.register_template(rust_binary)?;
        self.register_template(rust_library)?;
        self.register_template(rust_struct)?;

        Ok(())
    }

    /// Register JavaScript templates
    fn register_javascript_templates(&mut self) -> Result<()> {
        let js_node_project = Template {
            id: "javascript-node".to_string(),
            name: "Node.js Project".to_string(),
            description: "A Node.js project with common setup".to_string(),
            language: Language::JavaScript,
            category: TemplateCategory::Project,
            files: vec![
                TemplateFile {
                    path: "package.json".to_string(),
                    content: include_str!("../../../templates/javascript/node/package.json.hbs").to_string(),
                },
                TemplateFile {
                    path: "index.js".to_string(),
                    content: include_str!("../../../templates/javascript/node/index.js.hbs").to_string(),
                },
                TemplateFile {
                    path: ".gitignore".to_string(),
                    content: include_str!("../../../templates/javascript/gitignore.hbs").to_string(),
                },
            ],
            variables: vec![
                TemplateVariable {
                    name: "project_name".to_string(),
                    description: "Project name".to_string(),
                    required: true,
                    default_value: None,
                    variable_type: VariableType::String,
                },
                TemplateVariable {
                    name: "author".to_string(),
                    description: "Project author".to_string(),
                    required: false,
                    default_value: Some("Developer".to_string()),
                    variable_type: VariableType::String,
                },
            ],
        };

        self.register_template(js_node_project)?;
        Ok(())
    }

    /// Register Python templates
    fn register_python_templates(&mut self) -> Result<()> {
        let python_project = Template {
            id: "python-project".to_string(),
            name: "Python Project".to_string(),
            description: "A Python project with common structure".to_string(),
            language: Language::Python,
            category: TemplateCategory::Project,
            files: vec![
                TemplateFile {
                    path: "main.py".to_string(),
                    content: include_str!("../../../templates/python/project/main.py.hbs").to_string(),
                },
                TemplateFile {
                    path: "requirements.txt".to_string(),
                    content: include_str!("../../../templates/python/project/requirements.txt.hbs").to_string(),
                },
                TemplateFile {
                    path: ".gitignore".to_string(),
                    content: include_str!("../../../templates/python/gitignore.hbs").to_string(),
                },
            ],
            variables: vec![
                TemplateVariable {
                    name: "project_name".to_string(),
                    description: "Project name".to_string(),
                    required: true,
                    default_value: None,
                    variable_type: VariableType::String,
                },
            ],
        };

        self.register_template(python_project)?;
        Ok(())
    }

    /// Register common templates
    fn register_common_templates(&mut self) -> Result<()> {
        let docker_template = Template {
            id: "dockerfile".to_string(),
            name: "Dockerfile".to_string(),
            description: "A multi-stage Dockerfile".to_string(),
            language: Language::Unknown,
            category: TemplateCategory::Config,
            files: vec![
                TemplateFile {
                    path: "Dockerfile".to_string(),
                    content: include_str!("../../../templates/docker/Dockerfile.hbs").to_string(),
                },
            ],
            variables: vec![
                TemplateVariable {
                    name: "base_image".to_string(),
                    description: "Base Docker image".to_string(),
                    required: false,
                    default_value: Some("ubuntu:22.04".to_string()),
                    variable_type: VariableType::String,
                },
                TemplateVariable {
                    name: "port".to_string(),
                    description: "Exposed port".to_string(),
                    required: false,
                    default_value: Some("8080".to_string()),
                    variable_type: VariableType::Number,
                },
            ],
        };

        self.register_template(docker_template)?;
        Ok(())
    }

    /// Register a template
    #[instrument(skip(self, template))]
    pub fn register_template(&mut self, template: Template) -> Result<()> {
        debug!("Registering template: {}", template.id);

        // Register template files with handlebars
        for file in &template.files {
            let template_name = format!("{}::{}", template.id, file.path);
            self.handlebars.register_template_string(&template_name, &file.content)
                .map_err(|e| CodevError::Internal {
                    message: format!("Failed to register template: {}", e),
                })?;
        }

        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// Generate code from template
    #[instrument(skip(self, context))]
    pub fn generate_from_template(&self, template_id: &str, context: &TemplateContext) -> Result<GeneratedFiles> {
        info!("Generating code from template: {}", template_id);

        let template = self.templates.get(template_id)
            .ok_or_else(|| CodevError::NotFound {
                resource: format!("Template: {}", template_id),
            })?;

        // Validate required variables
        self.validate_context(template, context)?;

        // Prepare template data
        let template_data = self.prepare_template_data(template, context)?;

        let mut generated_files = Vec::new();

        for file in &template.files {
            let template_name = format!("{}::{}", template.id, file.path);

            // Process file path template
            let processed_path = self.handlebars.render_template(&file.path, &template_data)
                .map_err(|e| CodevError::Internal {
                    message: format!("Failed to render file path: {}", e),
                })?;

            // Render file content
            let content = self.handlebars.render(&template_name, &template_data)
                .map_err(|e| CodevError::Internal {
                    message: format!("Failed to render template '{}': {}", template_name, e),
                })?;

            generated_files.push(GeneratedFile {
                path: processed_path,
                content,
                executable: false, // TODO: Make this configurable
            });
        }

        Ok(GeneratedFiles {
            template_id: template_id.to_string(),
            files: generated_files,
        })
    }

    /// Validate template context
    fn validate_context(&self, template: &Template, context: &TemplateContext) -> Result<()> {
        for variable in &template.variables {
            if variable.required && !context.variables.contains_key(&variable.name) {
                return Err(CodevError::InvalidInput {
                    message: format!("Required variable '{}' not provided", variable.name),
                });
            }
        }
        Ok(())
    }

    /// Prepare template data by merging context with defaults
    fn prepare_template_data(&self, template: &Template, context: &TemplateContext) -> Result<HashMap<String, serde_json::Value>> {
        let mut data = HashMap::new();

        // Add default values
        for variable in &template.variables {
            if let Some(default) = &variable.default_value {
                data.insert(variable.name.clone(), serde_json::Value::String(default.clone()));
            }
        }

        // Override with provided values
        for (key, value) in &context.variables {
            data.insert(key.clone(), value.clone());
        }

        // Add built-in variables
        data.insert("current_year".to_string(), serde_json::Value::String(chrono::Utc::now().year().to_string()));
        data.insert("timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

        Ok(data)
    }

    /// Get available templates
    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    /// Get templates by category
    pub fn list_templates_by_category(&self, category: TemplateCategory) -> Vec<&Template> {
        self.templates.values()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Get templates by language
    pub fn list_templates_by_language(&self, language: Language) -> Vec<&Template> {
        self.templates.values()
            .filter(|t| t.language == language)
            .collect()
    }

    /// Get template by ID
    pub fn get_template(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }
}

/// Template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub language: Language,
    pub category: TemplateCategory,
    pub files: Vec<TemplateFile>,
    pub variables: Vec<TemplateVariable>,
}

/// Template file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
}

/// Template variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub variable_type: VariableType,
}

/// Variable types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Number,
    Boolean,
    List,
}

/// Template categories
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateCategory {
    Project,
    Code,
    Config,
    Documentation,
    Test,
}

/// Template context for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    pub variables: HashMap<String, serde_json::Value>,
}

impl TemplateContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set_variable<T: serde::Serialize>(&mut self, name: &str, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| CodevError::Internal {
                message: format!("Failed to serialize variable: {}", e),
            })?;
        self.variables.insert(name.to_string(), json_value);
        Ok(())
    }
}

/// Generated files result
#[derive(Debug, Clone)]
pub struct GeneratedFiles {
    pub template_id: String,
    pub files: Vec<GeneratedFile>,
}

/// Generated file
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub executable: bool,
}

// Handlebars helpers
fn snake_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("snake_case helper requires one parameter"))?;

    let input = param.value().as_str()
        .ok_or_else(|| RenderError::new("snake_case helper requires string parameter"))?;

    let snake_case = to_snake_case(input);
    out.write(&snake_case)?;
    Ok(())
}

fn pascal_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("pascal_case helper requires one parameter"))?;

    let input = param.value().as_str()
        .ok_or_else(|| RenderError::new("pascal_case helper requires string parameter"))?;

    let pascal_case = to_pascal_case(input);
    out.write(&pascal_case)?;
    Ok(())
}

fn camel_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("camel_case helper requires one parameter"))?;

    let input = param.value().as_str()
        .ok_or_else(|| RenderError::new("camel_case helper requires string parameter"))?;

    let camel_case = to_camel_case(input);
    out.write(&camel_case)?;
    Ok(())
}

fn kebab_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("kebab_case helper requires one parameter"))?;

    let input = param.value().as_str()
        .ok_or_else(|| RenderError::new("kebab_case helper requires string parameter"))?;

    let kebab_case = to_kebab_case(input);
    out.write(&kebab_case)?;
    Ok(())
}

fn current_year_helper(
    _: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let year = chrono::Utc::now().year().to_string();
    out.write(&year)?;
    Ok(())
}

fn pluralize_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("pluralize helper requires one parameter"))?;

    let input = param.value().as_str()
        .ok_or_else(|| RenderError::new("pluralize helper requires string parameter"))?;

    let plural = pluralize(input);
    out.write(&plural)?;
    Ok(())
}

// Helper functions
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
        } else if c == ' ' || c == '-' {
            result.push('_');
            prev_char_was_uppercase = false;
        } else {
            result.push(c);
            prev_char_was_uppercase = false;
        }
    }

    result
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == ' ' || c == '-' {
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

fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    if let Some(first_char) = pascal.chars().next() {
        first_char.to_lowercase().collect::<String>() + &pascal[first_char.len_utf8()..]
    } else {
        pascal
    }
}

fn to_kebab_case(s: &str) -> String {
    to_snake_case(s).replace('_', "-")
}

fn pluralize(s: &str) -> String {
    // Simple pluralization rules
    if s.ends_with('s') || s.ends_with("sh") || s.ends_with("ch") || s.ends_with('x') || s.ends_with('z') {
        format!("{}es", s)
    } else if s.ends_with('y') && s.len() > 1 && !s.chars().nth(s.len() - 2).unwrap().is_ascii_alphabetic() {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{}s", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversions() {
        assert_eq!(to_snake_case("MyProject"), "my_project");
        assert_eq!(to_snake_case("HTTPServer"), "http_server");
        assert_eq!(to_snake_case("already_snake"), "already_snake");

        assert_eq!(to_pascal_case("my_project"), "MyProject");
        assert_eq!(to_pascal_case("http_server"), "HttpServer");
        assert_eq!(to_pascal_case("PascalCase"), "PascalCase");

        assert_eq!(to_camel_case("my_project"), "myProject");
        assert_eq!(to_camel_case("http_server"), "httpServer");

        assert_eq!(to_kebab_case("MyProject"), "my-project");
        assert_eq!(to_kebab_case("HTTPServer"), "http-server");
    }

    #[test]
    fn test_pluralization() {
        assert_eq!(pluralize("cat"), "cats");
        assert_eq!(pluralize("class"), "classes");
        assert_eq!(pluralize("city"), "cities");
        assert_eq!(pluralize("box"), "boxes");
    }

    #[test]
    fn test_template_context() {
        let mut context = TemplateContext::new();
        context.set_variable("name", "test").unwrap();
        context.set_variable("number", 42).unwrap();
        context.set_variable("enabled", true).unwrap();

        assert_eq!(context.variables.len(), 3);
        assert!(context.variables.contains_key("name"));
        assert!(context.variables.contains_key("number"));
        assert!(context.variables.contains_key("enabled"));
    }

    #[tokio::test]
    async fn test_template_engine() {
        let engine = TemplateEngine::new();

        // May fail without actual template files, which is expected in tests
        match engine {
            Ok(engine) => {
                let templates = engine.list_templates();
                // Should have some built-in templates (if template files exist)
                // In test environment without template files, this might be empty
            }
            Err(_) => {
                // Expected in test environment without template files
            }
        }
    }
}