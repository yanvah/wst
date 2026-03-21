use super::compile;
use crate::ast::File;

fn parse(src: &str) -> File {
    let mut lex = crate::lexer::Lexer::new(src);
    let tokens = lex.tokenize().unwrap();
    crate::parser::Parser::new(tokens).parse_file().unwrap()
}

fn ts(src: &str) -> String {
    compile(&parse(src), std::path::Path::new(".")).unwrap()
}

#[test]
fn enum_basic() {
    let out = ts("enum Direction { North, South, East, West };");
    assert!(out.contains("export enum Direction {"));
    assert!(out.contains("  North = \"North\","));
    assert!(out.contains("  South = \"South\","));
}

#[test]
fn enum_deprecated_case() {
    let out = ts("enum Dir { North #deprecated, South };");
    assert!(out.contains("  /** @deprecated */\n  North = \"North\","));
}

#[test]
fn enum_banned_case() {
    let out = ts("enum Dir { North #banned, South };");
    assert!(out.contains("  /** @banned */\n  North = \"North\","));
}

#[test]
fn enum_deprecated_and_banned_case() {
    let out = ts("enum Dir { North #deprecated #banned, South };");
    assert!(out.contains("/** @deprecated @banned */"));
}

#[test]
fn variant_basic() {
    let out = ts("variant Location { Address = string, Coords = Coords };");
    assert!(out.contains("export type Location ="));
    assert!(out.contains("  | { Address: string }"));
    assert!(out.contains("  | { Coords: Coords }"));
}

#[test]
fn variant_ends_with_semicolon() {
    let out = ts("variant Location { Address = string };");
    assert!(out.ends_with(";\n\n"));
}

#[test]
fn variant_deprecated_case() {
    let out = ts("variant Loc { Addr = string #deprecated };");
    assert!(out.contains("  // @deprecated\n"));
}

#[test]
fn variant_banned_case() {
    let out = ts("variant Loc { Addr = string #banned };");
    assert!(out.contains("  // @banned\n"));
}

#[test]
fn struct_basic() {
    let out = ts("struct MyStruct { x = string };");
    assert!(out.contains("export interface MyStruct {"));
}

#[test]
fn struct_optional_field() {
    let out = ts("struct Foo { x = string };");
    assert!(out.contains("  x?: string | null;"));
}

#[test]
fn struct_required_field() {
    let out = ts("struct Foo { x = string #required };");
    assert!(out.contains("  x: string;"));
    assert!(!out.contains("x?:"));
}

#[test]
fn struct_deprecated_field() {
    let out = ts("struct Foo { x = string #deprecated };");
    assert!(out.contains("  /** @deprecated */\n  x?:"));
}

#[test]
fn struct_banned_field() {
    let out = ts("struct Foo { x = string #banned };");
    assert!(out.contains("  x: never;"));
}

#[test]
fn struct_optional_and_deprecated() {
    let out = ts("struct Foo { x = string #deprecated };");
    assert!(out.contains("/** @deprecated */"));
    assert!(out.contains("x?:"));
}

#[test]
fn primitive_int64_to_number() {
    let out = ts("struct Foo { x = int64 };");
    assert!(out.contains("x?: number | null;"));
}

#[test]
fn primitive_uin64_to_number() {
    let out = ts("struct Foo { x = uin64 };");
    assert!(out.contains("x?: number | null;"));
}

#[test]
fn primitive_flt64_to_number() {
    let out = ts("struct Foo { x = flt64 };");
    assert!(out.contains("x?: number | null;"));
}

#[test]
fn primitive_boolean() {
    let out = ts("struct Foo { x = boolean };");
    assert!(out.contains("x?: boolean | null;"));
}

#[test]
fn primitive_string() {
    let out = ts("struct Foo { x = string };");
    assert!(out.contains("x?: string | null;"));
}

#[test]
fn vec_type_in_ts() {
    let out = ts("struct Foo { x = vec<string> };");
    assert!(out.contains("x?: string[] | null;"));
}

#[test]
fn vec_of_named_in_ts() {
    let out = ts("struct Foo { x = vec<Foo> };");
    assert!(out.contains("x?: Foo[] | null;"));
}

#[test]
fn map_type_in_ts() {
    let out = ts("struct Foo { x = map<string, int64> };");
    assert!(out.contains("x?: Record<string, number> | null;"));
}

#[test]
fn named_type_in_ts() {
    let out = ts("struct Foo { x = MyType };");
    assert!(out.contains("x?: MyType | null;"));
}

#[test]
fn namespaced_type_in_ts() {
    let out = ts("struct Foo { x = Ns.MyType };");
    assert!(out.contains("x?: Ns.MyType | null;"));
}

#[test]
fn named_import_emits_import() {
    let src = "@import ./foo.wst { A };";
    let out = compile(&parse(src), std::path::Path::new(".")).unwrap();
    assert!(out.contains("import type { A } from \"./foo\";"));
}

#[test]
fn named_import_strips_wst_ext() {
    let src = "@import ./foo.wst { A };";
    let out = compile(&parse(src), std::path::Path::new(".")).unwrap();
    assert!(!out.contains(".wst"));
}

#[test]
fn namespace_import_emits_star() {
    let src = "@import ./foo.wst *Ns;";
    let out = compile(&parse(src), std::path::Path::new(".")).unwrap();
    assert!(out.contains("import type * as Ns from \"./foo\";"));
}

#[test]
fn protocol_is_skipped() {
    let out = ts("protocol P { \"/ep\" <Req, Res> };");
    assert!(!out.contains("protocol"));
    assert!(!out.contains("P"));
}

#[test]
fn copy_import_no_import_line() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let wst_path = dir.path().join("dep.wst");
    let mut f = std::fs::File::create(&wst_path).unwrap();
    writeln!(f, "struct Dep {{ x = string }};").unwrap();

    let src = "@import ./dep.wst ^copy { Dep };";
    let file = parse(src);
    let out = compile(&file, dir.path()).unwrap();
    assert!(!out.contains("import type"));
    assert!(out.contains("export interface Dep"));
}

#[test]
fn multiple_imports_have_blank_line() {
    let src = "@import ./a.wst { A }; @import ./b.wst { B };";
    let out = compile(&parse(src), std::path::Path::new(".")).unwrap();
    assert!(out.contains("import type { A } from \"./a\";\nimport type { B } from \"./b\";\n\n"));
}
