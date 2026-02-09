use crate::cli::type_enums::{SourceType, ValueType};
use colored::Colorize;
use dialoguer::{Input, Select};

/// Prompts the user for tool details, filling in any missing information interactively.
///
/// # Arguments
/// * `name` - Optional name from CLI args
/// * `version` - Optional version from CLI args
/// * `source` - Optional source type from CLI args
/// * `url` - Optional URL from CLI args
/// * `repo` - Optional repo from CLI args
/// * `tag` - Optional tag from CLI args
///
/// # Returns
/// A tuple containing (name, version, source, url, repo, tag)
pub fn prompt_for_tool(
    name: Option<String>,
    version: Option<String>,
    source: Option<SourceType>,
    url: Option<String>,
    repo: Option<String>,
    tag: Option<String>,
) -> (
    String,
    String,
    SourceType,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    println!("{}", "Interactive Tool Addition".cyan().bold());

    // 1. Tool Name
    let tool_name = name.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Tool Name")
            .interact_text()
            .expect("Failed to read tool name")
    });

    // 2. Source Type
    let tool_source = source.unwrap_or_else(|| {
        let sources = vec![
            SourceType::Brew,
            SourceType::Github,
            SourceType::Cargo,
            SourceType::Rustup,
            SourceType::Pip,
            SourceType::Go,
            SourceType::Uv,
            SourceType::Url,
        ];

        let selection = Select::new()
            .with_prompt("Installation Source")
            .items(&sources)
            .default(0)
            .interact()
            .expect("Failed to select source");

        sources[selection].clone()
    });

    // 3. Version (default to "latest")
    let tool_version = version.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Version")
            .default("latest".into())
            .interact_text()
            .expect("Failed to read version")
    });

    // 4. Source-specific prompts
    let mut tool_url = url;
    let mut tool_repo = repo;
    let mut tool_tag = tag;

    match tool_source {
        SourceType::Github => {
            if tool_repo.is_none() {
                tool_repo = Some(
                    Input::new()
                        .with_prompt("GitHub Repository (owner/repo)")
                        .interact_text()
                        .expect("Failed to read repo"),
                );
            }
            if tool_tag.is_none() {
                // If version is not "latest" and tag is missing, maybe suggest version as tag?
                // For now, just prompt.
                let default_tag = if tool_version != "latest" {
                    format!("v{}", tool_version.trim_start_matches('v'))
                } else {
                    "".to_string()
                };

                tool_tag = Some(
                    Input::new()
                        .with_prompt("Release Tag (e.g., v1.0.0)")
                        .with_initial_text(default_tag)
                        .interact_text()
                        .expect("Failed to read tag"),
                );
            }
        }
        SourceType::Url => {
            if tool_url.is_none() {
                tool_url = Some(
                    Input::new()
                        .with_prompt("Download URL")
                        .interact_text()
                        .expect("Failed to read URL"),
                );
            }
        }
        _ => {}
    }

    (
        tool_name,
        tool_version,
        tool_source,
        tool_url,
        tool_repo,
        tool_tag,
    )
}

/// Prompts the user for font details.
pub fn prompt_for_font(
    name: Option<String>,
    version: Option<String>,
    repo: Option<String>,
    tag: Option<String>,
) -> (String, String, String, String) {
    println!("{}", "Interactive Font Addition".cyan().bold());

    let font_name = name.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Font Name")
            .interact_text()
            .expect("Failed to read font name")
    });

    let font_repo = repo.unwrap_or_else(|| {
        Input::new()
            .with_prompt("GitHub Repository (owner/repo)")
            .default("ryanoasis/nerd-fonts".into())
            .interact_text()
            .expect("Failed to read repo")
    });

    let font_version = version.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Version")
            .default("latest".into())
            .interact_text()
            .expect("Failed to read version")
    });

    let font_tag = tag.unwrap_or_else(|| {
        let default_tag = if font_version != "latest" {
            format!("v{}", font_version.trim_start_matches('v'))
        } else {
            "".to_string()
        };

        Input::new()
            .with_prompt("Release Tag")
            .with_initial_text(default_tag)
            .interact_text()
            .expect("Failed to read tag")
    });

    (font_name, font_version, font_repo, font_tag)
}

/// Prompts the user for setting details.
pub fn prompt_for_setting(
    domain: Option<String>,
    key: Option<String>,
    value: Option<String>,
    value_type: Option<ValueType>,
) -> (String, String, String, ValueType) {
    println!("{}", "Interactive Setting Addition".cyan().bold());

    let setting_domain = domain.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Domain (e.g., NSGlobalDomain)")
            .interact_text()
            .expect("Failed to read domain")
    });

    let setting_key = key.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Key")
            .interact_text()
            .expect("Failed to read key")
    });

    let setting_type = value_type.unwrap_or_else(|| {
        let types = vec![
            ValueType::String,
            ValueType::Bool,
            ValueType::Int,
            ValueType::Float,
        ];
        let selection = Select::new()
            .with_prompt("Value Type")
            .items(&types)
            .default(0) // Default to String
            .interact()
            .expect("Failed to select type");
        types[selection].clone()
    });

    let setting_value = value.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Value")
            .interact_text()
            .expect("Failed to read value")
    });

    (setting_domain, setting_key, setting_value, setting_type)
}

/// Prompts the user for alias details.
pub fn prompt_for_alias(name: Option<String>, value: Option<String>) -> (String, String) {
    println!("{}", "Interactive Alias Addition".cyan().bold());

    let alias_name = name.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Alias Name")
            .interact_text()
            .expect("Failed to read alias name")
    });

    let alias_value = value.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Command")
            .interact_text()
            .expect("Failed to read command")
    });

    (alias_name, alias_value)
}
