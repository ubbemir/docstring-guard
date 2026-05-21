use anyhow::Result;
use colored::Colorize;
use std::{fs, path::Path};

use crate::Language;

pub fn load_file_content(path: impl AsRef<Path>) -> Result<String> {
    fs::read_to_string(path.as_ref()).map_err(anyhow::Error::from)
}

pub fn load_file_error_formating(path: &Path) -> String {
    format!(
        "{} {} failed to read",
        "error:".red(),
        format!("{}:", path.display()).bold(),
    )
}

pub fn parse_error_formating(path: &Path) -> String {
    format!(
        "{} {} failed to parse",
        "error:".red(),
        format!("{}:", path.display()).bold(),
    )
}

fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

pub fn ignore_validation(lang: Language, line_number: usize, content: &str) -> bool {
    let mut lines = content.lines();
    if let Some(ignore) = lines.nth(line_number - 1) {
        let prefix = match lang {
            Language::Python => "#",
            Language::Rust => "//",
        };
        return remove_whitespace(ignore).contains(&format!("{prefix}docstring-guard=ignore"));
    }
    false
}
