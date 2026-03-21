use anyhow::Result;
use std::path::Path;
use crate::ast::*;
use crate::compiler::{def_name, has_tag, parse_wst_file, resolve_struct_source, resolve_wst_path, strip_wst_ext};

#[derive(Clone, Copy, PartialEq)]
enum OptionalMode { Implicit, Explicit }

pub fn compile(file: &File, file_dir: &Path) -> Result<String> {
    let optional_mode = file.enforcers.iter()
        .find(|e| e.key == "optional_mode")
        .map(|e| if e.value == "explicit" { OptionalMode::Explicit } else { OptionalMode::Implicit })
        .unwrap_or(OptionalMode::Implicit);

    let mut out = String::new();
    let mut copy_defs: Vec<Definition> = Vec::new();
    let mut has_import_lines = false;

    for import in &file.imports {
        match &import.kind {
            ImportKind::Named { copy: true, types } => {
                let abs = resolve_wst_path(file_dir, &import.path);
                let imported = parse_wst_file(&abs)?;
                for def in imported.definitions {
                    if types.contains(&def_name(&def).to_string()) {
                        copy_defs.push(def);
                    }
                }
            }
            ImportKind::Named { copy: false, types } => {
                let path = strip_wst_ext(&import.path);
                out.push_str(&format!("import type {{ {} }} from \"{}\";\n", types.join(", "), path));
                has_import_lines = true;
            }
            ImportKind::Namespace { name } => {
                let path = strip_wst_ext(&import.path);
                out.push_str(&format!("import type * as {} from \"{}\";\n", name, path));
                has_import_lines = true;
            }
        }
    }

    if has_import_lines {
        out.push('\n');
    }

    let all_defs: Vec<Definition> = copy_defs.iter()
        .chain(file.definitions.iter())
        .cloned()
        .collect();

    for def in &copy_defs {
        emit_definition(&mut out, def, optional_mode, &all_defs);
    }
    for def in &file.definitions {
        emit_definition(&mut out, def, optional_mode, &all_defs);
    }

    Ok(out)
}

fn emit_definition(out: &mut String, def: &Definition, optional_mode: OptionalMode, all_defs: &[Definition]) {
    match def {
        Definition::Enum(e) => emit_enum(out, e),
        Definition::Variant(v) => emit_variant(out, v),
        Definition::Struct(s) => emit_struct(out, s, optional_mode, all_defs),
        Definition::Protocol(_) => {}
        Definition::Const(c) => emit_const(out, c),
    }
}

fn emit_enum(out: &mut String, def: &EnumDef) {
    out.push_str(&format!("export enum {} {{\n", def.name));
    for case in &def.cases {
        if let Some(doc) = jsdoc(&case.tags) {
            out.push_str(&format!("  {}\n", doc));
        }
        out.push_str(&format!("  {} = \"{}\",\n", case.name, case.name));
    }
    out.push_str("}\n\n");
}

fn emit_variant(out: &mut String, def: &VariantDef) {
    out.push_str(&format!("export type {} =\n", def.name));
    for case in &def.cases {
        if has_tag(&case.tags, "deprecated") {
            out.push_str("  // @deprecated\n");
        }
        if has_tag(&case.tags, "banned") {
            out.push_str("  // @banned\n");
        }
        let ty = type_to_ts(&case.ty);
        out.push_str(&format!("  | {{ {}: {} }}\n", case.name, ty));
    }
    out.push_str(";\n\n");
}

fn emit_struct(out: &mut String, def: &StructDef, optional_mode: OptionalMode, all_defs: &[Definition]) {
    out.push_str(&format!("export interface {} {{\n", def.name));
    for source in &def.copies {
        if let Ok(fields) = resolve_struct_source(source, all_defs) {
            for field in &fields {
                emit_struct_field(out, field, optional_mode);
            }
        }
    }
    for field in &def.fields {
        emit_struct_field(out, field, optional_mode);
    }
    out.push_str("}\n\n");
}

fn emit_struct_field(out: &mut String, field: &StructField, optional_mode: OptionalMode) {
    if let Some(doc) = jsdoc(&field.tags) {
        out.push_str(&format!("  {}\n", doc));
    }
    let required = has_tag(&field.tags, "required");
    let banned = has_tag(&field.tags, "banned");
    let nullable = has_tag(&field.tags, "nullable");
    let ts_ty = type_to_ts(&field.ty);
    let key = field_key(&field.name);
    if banned {
        out.push_str(&format!("  {}: never;\n", key));
    } else if required && nullable {
        out.push_str(&format!("  {}: {} | null;\n", key, ts_ty));
    } else if required {
        out.push_str(&format!("  {}: {};\n", key, ts_ty));
    } else if optional_mode == OptionalMode::Explicit {
        out.push_str(&format!("  {}: {} | null;\n", key, ts_ty));
    } else {
        out.push_str(&format!("  {}?: {} | null;\n", key, ts_ty));
    }
}

/// Returns the output field name: the portion after the last `.` (strips enum qualifier if present).
fn field_key(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(_, k)| k)
}

fn emit_const(out: &mut String, def: &crate::ast::ConstDef) {
    use crate::ast::Expr;
    match &def.value {
        Expr::EnumCase { ty, case } => {
            out.push_str(&format!("export const {}: {} = {}.{};\n\n", def.name, ty, ty, case));
        }
        Expr::Struct { ty, .. } => {
            out.push_str(&format!("export const {}: {} = {};\n\n",
                def.name, ty, expr_to_ts(&def.value, 0)));
        }
        _ => {
            out.push_str(&format!("export const {} = {};\n\n", def.name, expr_to_ts(&def.value, 0)));
        }
    }
}

fn expr_to_ts(expr: &crate::ast::Expr, depth: usize) -> String {
    use crate::ast::Expr;
    let indent = "  ".repeat(depth);
    let inner = "  ".repeat(depth + 1);
    match expr {
        Expr::Str { value } => format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")),
        Expr::Number { value } => {
            if value.fract() == 0.0 { format!("{}", *value as i64) } else { format!("{}", value) }
        }
        Expr::Bool { value } => value.to_string(),
        Expr::Null => "null".to_string(),
        Expr::EnumCase { ty, case } => format!("{}.{}", ty, case),
        Expr::Struct { fields, .. } => {
            let mut s = "{\n".to_string();
            for f in fields {
                let key = field_key(&f.name);
                s.push_str(&format!("{}{}: {},\n", inner, key, expr_to_ts(&f.value, depth + 1)));
            }
            s.push_str(&format!("{}}}", indent));
            s
        }
    }
}

fn jsdoc(tags: &[Tag]) -> Option<String> {
    let mut items: Vec<&str> = Vec::new();
    if has_tag(tags, "deprecated") { items.push("@deprecated"); }
    if has_tag(tags, "banned") { items.push("@banned"); }
    if items.is_empty() {
        None
    } else if items.len() == 1 {
        Some(format!("/** {} */", items[0]))
    } else {
        Some(format!("/** {} */", items.join(" ")))
    }
}

fn type_to_ts(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Primitive(p) => primitive_to_ts(p).to_string(),
        TypeRef::Vec(inner) => format!("{}[]", type_to_ts(inner)),
        TypeRef::Map { key, value } => {
            format!("Record<{}, {}>", primitive_to_ts(key), type_to_ts(value))
        }
        TypeRef::Named(name) => name.clone(),
    }
}

fn primitive_to_ts(p: &Primitive) -> &str {
    match p {
        Primitive::Int32 | Primitive::Int64 | Primitive::Uin64 | Primitive::Flt64 => "number",
        Primitive::Boolean => "boolean",
        Primitive::Str => "string",
    }
}

#[cfg(test)]
#[path = "typescript_tests.rs"]
mod tests;
