use std::collections::HashSet;
use std::path::Path;
use crate::ast::*;
use crate::compiler::{def_name, parse_wst_file, resolve_struct_source, resolve_wst_path};

const BUILTIN_TAGS: &[&str] = &["deprecated", "banned", "required", "optional", "nullable"];

pub fn validate(file: &File, file_path: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let base_dir = file_path.parent().unwrap_or(Path::new("."));

    // Check for duplicate definition names
    let mut seen_names: HashSet<String> = HashSet::new();
    for def in &file.definitions {
        let name = def_name(def).to_string();
        if !seen_names.insert(name.clone()) {
            errors.push(format!("'{}' is defined more than once", name));
        }
    }

    let mut available_names: HashSet<String> = file.definitions.iter()
        .filter(|d| !matches!(d, Definition::Const(_)))
        .map(|d| def_name(d).to_string())
        .collect();
    let mut available_namespaces: HashSet<String> = HashSet::new();
    // Definitions available for copy resolution: local + ^copy-imported
    let mut copy_defs: Vec<Definition> = Vec::new();

    for import in &file.imports {
        let import_path = resolve_wst_path(base_dir, &import.path);
        let (exported, private) = match load_exported_types(&import_path) {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("Cannot load import '{}': {}", import.path, e));
                continue;
            }
        };
        match &import.kind {
            ImportKind::Named { copy: true, types } => {
                // Load the full definitions so copy sources can be resolved
                if let Ok(imported_file) = parse_wst_file(&import_path) {
                    for def in imported_file.definitions {
                        if types.contains(&def_name(&def).to_string())
                            && exported.contains(def_name(&def))
                        {
                            available_names.insert(def_name(&def).to_string());
                            copy_defs.push(def);
                        }
                    }
                }
            }
            ImportKind::Named { copy: false, types } => {
                for name in types {
                    if exported.contains(name.as_str()) {
                        available_names.insert(name.clone());
                    } else if private.contains(name.as_str()) {
                        errors.push(format!("'{}' is private in '{}'", name, import.path));
                    } else {
                        errors.push(format!("'{}' is not exported by '{}'", name, import.path));
                    }
                }
            }
            ImportKind::Namespace { name } => {
                available_namespaces.insert(name.clone());
            }
        }
    }

    // All definitions available for copy resolution
    let all_defs: Vec<Definition> = copy_defs.iter()
        .chain(file.definitions.iter())
        .cloned()
        .collect();

    for def in &file.definitions {
        match def {
            Definition::Enum(e) => {
                check_tags(&e.tags, false, &mut errors);
                for case in &e.cases {
                    check_tags(&case.tags, false, &mut errors);
                }
            }
            Definition::Variant(v) => {
                check_tags(&v.tags, false, &mut errors);
                for case in &v.cases {
                    check_type(&case.ty, case.line, &available_names, &available_namespaces, &mut errors);
                    check_tags(&case.tags, false, &mut errors);
                }
            }
            Definition::Struct(s) => {
                check_tags(&s.tags, false, &mut errors);
                validate_struct_copies(s, &all_defs, &available_names, &mut errors);
                validate_struct_asserts(s, &all_defs, &file.assertion_defs, &mut errors);
                for field in &s.fields {
                    check_type(&field.ty, field.line, &available_names, &available_namespaces, &mut errors);
                    check_tags(&field.tags, true, &mut errors);
                    check_enum_field_key(&field.name, field.line, &all_defs, &mut errors);
                }
            }
            Definition::Protocol(p) => {
                check_tags(&p.tags, false, &mut errors);
                for ep in &p.endpoints {
                    check_type(&ep.request, ep.line, &available_names, &available_namespaces, &mut errors);
                    check_type(&ep.response, ep.line, &available_names, &available_namespaces, &mut errors);
                    if let Some(err_ty) = &ep.error {
                        check_type(err_ty, ep.line, &available_names, &available_namespaces, &mut errors);
                    }
                    check_tags(&ep.tags, false, &mut errors);
                }
            }
            Definition::Const(c) => {
                if !is_screaming_case(&c.name) {
                    errors.push(format!(
                        "Constant '{}' must be SCREAMING_CASE (e.g. MY_CONSTANT)",
                        c.name
                    ));
                }
            }
        }
    }

    // Check for duplicate assertion definition names
    let mut seen_assertion_names: HashSet<String> = HashSet::new();
    for adef in &file.assertion_defs {
        if !seen_assertion_names.insert(adef.name.clone()) {
            errors.push(format!("Assertion '{}' is defined more than once", adef.name));
        }
    }

    errors
}

fn check_type(ty: &TypeRef, line: usize, names: &HashSet<String>, namespaces: &HashSet<String>, errors: &mut Vec<String>) {
    match ty {
        TypeRef::Primitive(_) => {}
        TypeRef::Named(name) => {
            if !is_available(name, names, namespaces) {
                errors.push(format!("{}:0: Undefined type '{}'", line, name));
            }
        }
        TypeRef::Vec(inner) => check_type(inner, line, names, namespaces, errors),
        TypeRef::Map { value, .. } => check_type(value, line, names, namespaces, errors),
    }
}

fn is_available(name: &str, names: &HashSet<String>, namespaces: &HashSet<String>) -> bool {
    if names.contains(name) {
        return true;
    }
    if let Some(dot) = name.find('.') {
        return namespaces.contains(&name[..dot]);
    }
    false
}

const STRUCT_ONLY_TAGS: &[&str] = &["required", "optional"];

fn check_tags(tags: &[Tag], allow_struct_tags: bool, errors: &mut Vec<String>) {
    for tag in tags {
        if tag.namespace.is_some() { continue; }
        if !BUILTIN_TAGS.contains(&tag.name.as_str()) {
            errors.push(format!(
                "Unknown tag '#{}'; custom tags require a namespace prefix (e.g. '#myorg:{}')",
                tag.name, tag.name
            ));
        } else if !allow_struct_tags && STRUCT_ONLY_TAGS.contains(&tag.name.as_str()) {
            errors.push(format!(
                "'#{}' is only valid on struct fields",
                tag.name
            ));
        }
    }
}

fn validate_struct_copies(
    s: &StructDef,
    all_defs: &[Definition],
    available_names: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    // Seed seen with direct fields so copy-vs-field conflicts are detected
    let mut seen: HashSet<String> = s.fields.iter().map(|f| f.name.clone()).collect();

    for source in &s.copies {
        let base_name = match source {
            StructSource::Named { name } => name.as_str(),
            StructSource::Exclude { base, .. } => base.as_str(),
        };

        if !available_names.contains(base_name) {
            errors.push(format!("Undefined struct '{}' in copy", base_name));
            continue;
        }

        // Check it is not a non-struct local definition
        let local_def = all_defs.iter().find(|d| def_name(d) == base_name);
        if let Some(def) = local_def {
            if !matches!(def, Definition::Struct(_)) {
                errors.push(format!("'{}' is not a struct; only structs can be copied", base_name));
                continue;
            }
        } else {
            // From a regular import — can't do deep field validation
            continue;
        }

        // For @exclude: validate that excluded field names actually exist
        if let StructSource::Exclude { exclude, .. } = source {
            let named = StructSource::Named { name: base_name.to_string() };
            if let Ok(all_base_fields) = resolve_struct_source(&named, all_defs) {
                for excl_name in exclude {
                    if !all_base_fields.iter().any(|f| &f.name == excl_name) {
                        errors.push(format!(
                            "Field '{}' does not exist in struct '{}'",
                            excl_name, base_name
                        ));
                    }
                }
            }
        }

        match resolve_struct_source(source, all_defs) {
            Ok(fields) => {
                for field in &fields {
                    if !seen.insert(field.name.clone()) {
                        errors.push(format!(
                            "Duplicate field '{}' in struct '{}' (from copy of '{}')",
                            field.name, s.name, base_name
                        ));
                    }
                }
            }
            Err(e) => errors.push(e.to_string()),
        }
    }
}

fn validate_struct_asserts(
    s: &StructDef,
    all_defs: &[Definition],
    assertion_defs: &[AssertionDef],
    errors: &mut Vec<String>,
) {
    if s.asserts.is_empty() { return; }

    // Collect all output field keys in this struct (direct + from copies).
    // For dotted names like `Namespace.production`, the key is `production`.
    let mut field_names: HashSet<String> = s.fields.iter()
        .map(|f| field_key(&f.name).to_string())
        .collect();
    for source in &s.copies {
        if let Ok(fields) = resolve_struct_source(source, all_defs) {
            for f in fields { field_names.insert(field_key(&f.name).to_string()); }
        }
    }

    for assert_ref in &s.asserts {
        let (_param, body, line_override): (&str, &[AssertionStmt], Option<usize>) = match assert_ref {
            AssertRef::Inline { param, body, .. } => (param.as_str(), body, None),
            AssertRef::Named { name, line } => {
                match assertion_defs.iter().find(|a| &a.name == name) {
                    Some(adef) => (adef.param.as_str(), &adef.body, Some(*line)),
                    None => {
                        errors.push(format!("{}:0: Undefined assertion '{}'", line, name));
                        continue;
                    }
                }
            }
        };
        eval_assertion_stmts(&s.name, body, &std::collections::HashMap::new(), &field_names, all_defs, errors, line_override);
    }
}

fn eval_assertion_stmts(
    struct_name: &str,
    stmts: &[AssertionStmt],
    scope: &std::collections::HashMap<String, String>,
    field_names: &HashSet<String>,
    all_defs: &[Definition],
    errors: &mut Vec<String>,
    line_override: Option<usize>,
) {
    for stmt in stmts {
        match stmt {
            AssertionStmt::ForIn { var, source, body, line } => {
                let err_line = line_override.unwrap_or(*line);
                let cases: Vec<String> = match all_defs.iter().find_map(|d| match d {
                    Definition::Enum(e) if &e.name == source => Some(&e.cases),
                    _ => None,
                }) {
                    Some(cases) => cases.iter().map(|c| c.name.clone()).collect(),
                    None => {
                        errors.push(format!(
                            "{}:0: Enum '{}' not found for assertion on '{}'; only locally-defined and ^copy-imported enums can be used",
                            err_line, source, struct_name
                        ));
                        continue;
                    }
                };
                for case_name in cases {
                    let mut inner_scope = scope.clone();
                    inner_scope.insert(var.clone(), case_name);
                    eval_assertion_stmts(struct_name, body, &inner_scope, field_names, all_defs, errors, line_override);
                }
            }
            AssertionStmt::HasKey { subject: _, key, line } => {
                let err_line = line_override.unwrap_or(*line);
                let field_name = match scope.get(key) {
                    Some(v) => v.as_str(),
                    None => {
                        errors.push(format!(
                            "{}:0: In assertion on '{}': '${}'  is not bound (not a loop variable)",
                            err_line, struct_name, key
                        ));
                        continue;
                    }
                };
                if !field_names.contains(field_name) {
                    errors.push(format!(
                        "{}:0: Assertion failed: struct '{}' is missing field '{}'",
                        err_line, struct_name, field_name
                    ));
                }
            }
        }
    }
}

fn check_enum_field_key(name: &str, line: usize, all_defs: &[Definition], errors: &mut Vec<String>) {
    let Some((qualifier, case)) = name.split_once('.') else { return };
    match all_defs.iter().find(|d| def_name(d) == qualifier) {
        None => errors.push(format!(
            "{}:0: '{}' in field key '{}' is not defined",
            line, qualifier, name
        )),
        Some(Definition::Enum(e)) => {
            if !e.cases.iter().any(|c| c.name == case) {
                errors.push(format!(
                    "{}:0: '{}' is not a case of enum '{}'",
                    line, case, qualifier
                ));
            }
        }
        Some(_) => errors.push(format!(
            "{}:0: '{}' in field key '{}' is not an enum",
            line, qualifier, name
        )),
    }
}

fn is_screaming_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().map_or(false, |c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn field_key(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(_, k)| k)
}

fn load_exported_types(path: &Path) -> anyhow::Result<(HashSet<String>, HashSet<String>)> {
    let file = parse_wst_file(path)?;
    let mut exported = HashSet::new();
    let mut private = HashSet::new();
    for def in &file.definitions {
        let name = def_name(def).to_string();
        if is_private_def(def) {
            private.insert(name);
        } else {
            exported.insert(name);
        }
    }
    Ok((exported, private))
}

fn is_private_def(def: &Definition) -> bool {
    match def {
        Definition::Enum(e) => e.private,
        Definition::Variant(v) => v.private,
        Definition::Struct(s) => s.private,
        Definition::Protocol(p) => p.private,
        Definition::Const(_) => false,
    }
}
