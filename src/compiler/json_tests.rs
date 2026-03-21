use super::compile;
use crate::ast::File;

fn parse(src: &str) -> File {
    let mut lex = crate::lexer::Lexer::new(src);
    let tokens = lex.tokenize().unwrap();
    crate::parser::Parser::new(tokens).parse_file().unwrap()
}

fn compile_str(src: &str) -> String {
    compile(&parse(src)).unwrap()
}

#[test]
fn empty_file_json() {
    let out = compile_str("");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["enforcers"], serde_json::json!([]));
    assert_eq!(v["imports"], serde_json::json!([]));
    assert_eq!(v["definitions"], serde_json::json!([]));
}

#[test]
fn enum_in_json() {
    let out = compile_str("enum Dir { North, South };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let defs = &v["definitions"];
    assert_eq!(defs[0]["data"]["name"], "Dir");
    assert_eq!(defs[0]["data"]["cases"][0]["name"], "North");
    assert_eq!(defs[0]["data"]["cases"][1]["name"], "South");
}

#[test]
fn variant_in_json() {
    let out = compile_str("variant Loc { Addr = string, Coords = Coords };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let defs = &v["definitions"];
    assert_eq!(defs[0]["kind"], "Variant");
    assert_eq!(defs[0]["data"]["cases"][0]["name"], "Addr");
    assert_eq!(defs[0]["data"]["cases"][1]["name"], "Coords");
}

#[test]
fn struct_in_json() {
    let out = compile_str("struct Foo { x = string };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let defs = &v["definitions"];
    assert_eq!(defs[0]["kind"], "Struct");
    assert_eq!(defs[0]["data"]["name"], "Foo");
    assert_eq!(defs[0]["data"]["fields"][0]["name"], "x");
}

#[test]
fn primitive_types_in_json() {
    let out = compile_str("struct P { a = int64, b = uin64, c = flt64, d = boolean, e = string };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let fields = &v["definitions"][0]["data"]["fields"];
    assert_eq!(fields[0]["ty"]["of"], "Int64");
    assert_eq!(fields[1]["ty"]["of"], "Uin64");
    assert_eq!(fields[2]["ty"]["of"], "Flt64");
    assert_eq!(fields[3]["ty"]["of"], "Boolean");
    assert_eq!(fields[4]["ty"]["of"], "String");
}

#[test]
fn vec_type_in_json() {
    let out = compile_str("struct Foo { x = vec<string> };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let ty = &v["definitions"][0]["data"]["fields"][0]["ty"];
    assert_eq!(ty["type"], "Vec");
    assert_eq!(ty["of"]["of"], "String");
}

#[test]
fn map_type_in_json() {
    let out = compile_str("struct Foo { x = map<string, int64> };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let ty = &v["definitions"][0]["data"]["fields"][0]["ty"];
    assert_eq!(ty["type"], "Map");
    assert_eq!(ty["of"]["key"], "String");
    assert_eq!(ty["of"]["value"]["of"], "Int64");
}

#[test]
fn tag_bool_default() {
    let out = compile_str("struct Foo { x = string #deprecated };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let tag = &v["definitions"][0]["data"]["fields"][0]["tags"][0];
    assert_eq!(tag["name"], "deprecated");
    assert_eq!(tag["value"], true);
}

#[test]
fn tag_string_value() {
    let out = compile_str("struct Foo { x = string #k=\"v\" };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let tag = &v["definitions"][0]["data"]["fields"][0]["tags"][0];
    assert_eq!(tag["value"], "v");
}

#[test]
fn tag_number_value() {
    let out = compile_str("struct Foo { x = string #k=1.5 };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let tag = &v["definitions"][0]["data"]["fields"][0]["tags"][0];
    assert!((tag["value"].as_f64().unwrap() - 1.5).abs() < 1e-9);
}

#[test]
fn import_named_in_json() {
    let out = compile_str("@import ./f.wst { T };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["imports"][0];
    assert_eq!(imp["path"], "./f.wst");
    assert_eq!(imp["kind"]["type"], "Named");
    assert_eq!(imp["kind"]["copy"], false);
    assert_eq!(imp["kind"]["types"][0], "T");
}

#[test]
fn import_copy_in_json() {
    let out = compile_str("@import ./f.wst ^copy { T };");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["imports"][0];
    assert_eq!(imp["kind"]["copy"], true);
}

#[test]
fn import_namespace_in_json() {
    let out = compile_str("@import ./f.wst *Ns;");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["imports"][0];
    assert_eq!(imp["kind"]["type"], "Namespace");
    assert_eq!(imp["kind"]["name"], "Ns");
}
