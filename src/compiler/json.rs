use anyhow::Result;
use crate::ast::File;

pub fn compile(file: &File) -> Result<String> {
    Ok(serde_json::to_string_pretty(file)?)
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
