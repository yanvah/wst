use anyhow::Result;
use std::path::Path;
use crate::{ast, lexer, parser};

pub mod json;
pub mod typescript;
pub mod rust;

pub fn parse_wst_file(path: &Path) -> Result<ast::File> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {:?}: {}", path, e))?;
    let mut lex = lexer::Lexer::new(&src);
    let tokens = lex.tokenize()?;
    let mut p = parser::Parser::new(tokens);
    p.parse_file()
}

pub fn def_name(def: &ast::Definition) -> &str {
    match def {
        ast::Definition::Enum(e) => &e.name,
        ast::Definition::Variant(v) => &v.name,
        ast::Definition::Struct(s) => &s.name,
        ast::Definition::Protocol(p) => &p.name,
        ast::Definition::Const(c) => &c.name,
    }
}

pub fn has_tag(tags: &[ast::Tag], name: &str) -> bool {
    tags.iter().any(|t| t.namespace.is_none() && t.name == name)
}

pub fn resolve_wst_path(base: &Path, path: &str) -> std::path::PathBuf {
    let clean = path.strip_prefix("./").unwrap_or(path);
    base.join(clean)
}

pub fn strip_wst_ext(path: &str) -> &str {
    path.strip_suffix(".wst").unwrap_or(path)
}

/// Resolve a copy source to its final list of fields, applying any @-ops.
/// Only works for structs defined in `defs` (local / ^copy-imported definitions).
pub fn resolve_struct_source(
    source: &ast::StructSource,
    defs: &[ast::Definition],
) -> anyhow::Result<Vec<ast::StructField>> {
    let base_name = match source {
        ast::StructSource::Named { name } => name.as_str(),
        ast::StructSource::Exclude { base, .. } => base.as_str(),
    };
    let struct_def = defs.iter()
        .find_map(|d| match d {
            ast::Definition::Struct(s) if s.name == base_name => Some(s),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve struct '{}' for copy", base_name))?;

    let mut fields = Vec::new();
    for copy in &struct_def.copies {
        fields.extend(resolve_struct_source(copy, defs)?);
    }
    fields.extend(struct_def.fields.iter().cloned());

    if let ast::StructSource::Exclude { exclude, .. } = source {
        fields.retain(|f| !exclude.contains(&f.name));
    }

    Ok(fields)
}
