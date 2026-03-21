use super::Parser;
use crate::ast::*;

fn parse(src: &str) -> File {
    let mut lex = crate::lexer::Lexer::new(src);
    let tokens = lex.tokenize().unwrap();
    Parser::new(tokens).parse_file().unwrap()
}

fn parse_err(src: &str) -> String {
    let mut lex = crate::lexer::Lexer::new(src);
    let tokens = lex.tokenize().unwrap();
    Parser::new(tokens).parse_file().unwrap_err().to_string()
}

#[test]
fn empty_file() {
    let f = parse("");
    assert!(f.enforcers.is_empty());
    assert!(f.imports.is_empty());
    assert!(f.definitions.is_empty());
}

#[test]
fn enforcer_snake() {
    let f = parse("!case=snake;");
    assert_eq!(f.enforcers.len(), 1);
    assert_eq!(f.enforcers[0].key, "case");
    assert_eq!(f.enforcers[0].value, "snake");
}

#[test]
fn enforcer_camel() {
    let f = parse("!case=camel;");
    assert_eq!(f.enforcers.len(), 1);
    assert_eq!(f.enforcers[0].key, "case");
    assert_eq!(f.enforcers[0].value, "camel");
}

#[test]
fn named_import() {
    let f = parse("@import ./foo.wst { TypeA, TypeB };");
    assert_eq!(f.imports.len(), 1);
    assert_eq!(f.imports[0].path, "./foo.wst");
    match &f.imports[0].kind {
        ImportKind::Named { copy, types } => {
            assert!(!copy);
            assert_eq!(types, &vec!["TypeA".to_string(), "TypeB".to_string()]);
        }
        _ => panic!("Expected Named import"),
    }
}

#[test]
fn named_import_copy() {
    let f = parse("@import ./foo.wst ^copy { TypeA };");
    assert_eq!(f.imports.len(), 1);
    match &f.imports[0].kind {
        ImportKind::Named { copy, types } => {
            assert!(copy);
            assert_eq!(types, &vec!["TypeA".to_string()]);
        }
        _ => panic!("Expected Named import"),
    }
}

#[test]
fn namespace_import() {
    let f = parse("@import ./foo.wst *MyNs;");
    assert_eq!(f.imports.len(), 1);
    match &f.imports[0].kind {
        ImportKind::Namespace { name } => {
            assert_eq!(name, "MyNs");
        }
        _ => panic!("Expected Namespace import"),
    }
}

#[test]
fn deep_path_import() {
    let f = parse("@import ./shared/user.wst { User };");
    assert_eq!(f.imports[0].path, "./shared/user.wst");
}

#[test]
fn simple_enum() {
    let f = parse("enum Dir { North, South };");
    assert_eq!(f.definitions.len(), 1);
    match &f.definitions[0] {
        Definition::Enum(e) => {
            assert_eq!(e.name, "Dir");
            assert_eq!(e.cases.len(), 2);
            assert_eq!(e.cases[0].name, "North");
            assert_eq!(e.cases[1].name, "South");
            assert!(e.cases[0].tags.is_empty());
        }
        _ => panic!("Expected Enum"),
    }
}

#[test]
fn enum_trailing_comma() {
    let f = parse("enum Dir { North, };");
    match &f.definitions[0] {
        Definition::Enum(e) => {
            assert_eq!(e.cases.len(), 1);
            assert_eq!(e.cases[0].name, "North");
        }
        _ => panic!("Expected Enum"),
    }
}

#[test]
fn enum_with_deprecated_case() {
    let f = parse("enum Dir { North #deprecated, South };");
    match &f.definitions[0] {
        Definition::Enum(e) => {
            assert_eq!(e.cases[0].tags.len(), 1);
            assert_eq!(e.cases[0].tags[0].name, "deprecated");
        }
        _ => panic!("Expected Enum"),
    }
}

#[test]
fn enum_with_multi_tags() {
    let f = parse("enum Dir { North #deprecated #banned, South };");
    match &f.definitions[0] {
        Definition::Enum(e) => {
            assert_eq!(e.cases[0].tags.len(), 2);
            assert_eq!(e.cases[0].tags[0].name, "deprecated");
            assert_eq!(e.cases[0].tags[1].name, "banned");
        }
        _ => panic!("Expected Enum"),
    }
}

#[test]
fn simple_variant() {
    let f = parse("variant Loc { Addr = string, Coords = Coords };");
    match &f.definitions[0] {
        Definition::Variant(v) => {
            assert_eq!(v.name, "Loc");
            assert_eq!(v.cases.len(), 2);
            assert_eq!(v.cases[0].name, "Addr");
            assert_eq!(v.cases[1].name, "Coords");
        }
        _ => panic!("Expected Variant"),
    }
}

#[test]
fn variant_case_named_type() {
    let f = parse("variant Loc { Addr = MyAddr };");
    match &f.definitions[0] {
        Definition::Variant(v) => {
            assert!(matches!(&v.cases[0].ty, TypeRef::Named(n) if n == "MyAddr"));
        }
        _ => panic!("Expected Variant"),
    }
}

#[test]
fn variant_case_vec_type() {
    let f = parse("variant Loc { Items = vec<string> };");
    match &f.definitions[0] {
        Definition::Variant(v) => {
            assert!(matches!(
                &v.cases[0].ty,
                TypeRef::Vec(inner) if matches!(inner.as_ref(), TypeRef::Primitive(Primitive::Str))
            ));
        }
        _ => panic!("Expected Variant"),
    }
}

#[test]
fn simple_struct() {
    let f = parse("struct Foo { x = string };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert_eq!(s.name, "Foo");
            assert_eq!(s.fields.len(), 1);
            assert_eq!(s.fields[0].name, "x");
            assert!(s.fields[0].tags.is_empty());
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_trailing_comma() {
    let f = parse("struct Foo { x = string, };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert_eq!(s.fields.len(), 1);
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_required_tag() {
    let f = parse("struct Foo { x = string #required };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert_eq!(s.fields[0].tags[0].name, "required");
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_deprecated_tag() {
    let f = parse("struct Foo { x = string #deprecated };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert_eq!(s.fields[0].tags[0].name, "deprecated");
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_banned_tag() {
    let f = parse("struct Foo { x = string #banned };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert_eq!(s.fields[0].tags[0].name, "banned");
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_inline_multiple_tags() {
    let f = parse("struct Foo { x = string #required #deprecated, };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            let tags = &s.fields[0].tags;
            assert_eq!(tags.len(), 2);
            assert_eq!(tags[0].name, "required");
            assert_eq!(tags[1].name, "deprecated");
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_bracket_tags() {
    let f = parse("struct Foo { x = string [ #deprecated #required ], };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            let tags = &s.fields[0].tags;
            assert_eq!(tags.len(), 2);
            assert_eq!(tags[0].name, "deprecated");
            assert_eq!(tags[1].name, "required");
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_custom_tag_no_value() {
    let f = parse("struct Foo { x = string #myorg:foo };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            let tag = &s.fields[0].tags[0];
            assert_eq!(tag.namespace, Some("myorg".to_string()));
            assert_eq!(tag.name, "foo");
            assert!(matches!(tag.value, TagValue::Bool(true)));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_tag_string_value() {
    let f = parse("struct Foo { x = string #myorg:key=\"hello\" };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            let tag = &s.fields[0].tags[0];
            assert!(matches!(&tag.value, TagValue::Str(s) if s == "hello"));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_tag_number_value() {
    let f = parse("struct Foo { x = string #tag=3.14 };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            let tag = &s.fields[0].tags[0];
            assert!(matches!(tag.value, TagValue::Number(n) if (n - 3.14).abs() < 1e-9));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn struct_tag_bool_false() {
    let f = parse("struct Foo { x = string #tag=false };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            let tag = &s.fields[0].tags[0];
            assert!(matches!(tag.value, TagValue::Bool(false)));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn all_primitives() {
    let f = parse("struct P { a = int64, b = uin64, c = flt64, d = boolean, e = string };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert!(matches!(s.fields[0].ty, TypeRef::Primitive(Primitive::Int64)));
            assert!(matches!(s.fields[1].ty, TypeRef::Primitive(Primitive::Uin64)));
            assert!(matches!(s.fields[2].ty, TypeRef::Primitive(Primitive::Flt64)));
            assert!(matches!(s.fields[3].ty, TypeRef::Primitive(Primitive::Boolean)));
            assert!(matches!(s.fields[4].ty, TypeRef::Primitive(Primitive::Str)));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn vec_type() {
    let f = parse("struct Foo { x = vec<string> };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert!(matches!(
                &s.fields[0].ty,
                TypeRef::Vec(inner) if matches!(inner.as_ref(), TypeRef::Primitive(Primitive::Str))
            ));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn map_type() {
    let f = parse("struct Foo { x = map<string, int64> };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert!(matches!(
                &s.fields[0].ty,
                TypeRef::Map { key: Primitive::Str, value }
                if matches!(value.as_ref(), TypeRef::Primitive(Primitive::Int64))
            ));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn vec_of_named() {
    let f = parse("struct Foo { x = vec<MyType> };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert!(matches!(
                &s.fields[0].ty,
                TypeRef::Vec(inner) if matches!(inner.as_ref(), TypeRef::Named(n) if n == "MyType")
            ));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn map_of_named() {
    let f = parse("struct Foo { x = map<string, MyType> };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert!(matches!(
                &s.fields[0].ty,
                TypeRef::Map { key: Primitive::Str, value }
                if matches!(value.as_ref(), TypeRef::Named(n) if n == "MyType")
            ));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn namespaced_type() {
    let f = parse("struct Foo { x = Ns.MyType };");
    match &f.definitions[0] {
        Definition::Struct(s) => {
            assert!(matches!(&s.fields[0].ty, TypeRef::Named(n) if n == "Ns.MyType"));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn protocol_simple() {
    let f = parse("protocol P { \"/ep\" <Req, Res> };");
    match &f.definitions[0] {
        Definition::Protocol(p) => {
            assert_eq!(p.name, "P");
            assert_eq!(p.endpoints.len(), 1);
            assert_eq!(p.endpoints[0].name, "/ep");
        }
        _ => panic!("Expected Protocol"),
    }
}

#[test]
fn protocol_with_error() {
    let f = parse("protocol P { \"/ep\" <Req, Res !Err> };");
    match &f.definitions[0] {
        Definition::Protocol(p) => {
            assert!(matches!(
                &p.endpoints[0].error,
                Some(TypeRef::Named(n)) if n == "Err"
            ));
        }
        _ => panic!("Expected Protocol"),
    }
}

#[test]
fn protocol_with_inline_tag() {
    let f = parse("protocol P { \"/ep\" #deprecated <Req, Res> };");
    match &f.definitions[0] {
        Definition::Protocol(p) => {
            assert_eq!(p.endpoints[0].tags.len(), 1);
            assert_eq!(p.endpoints[0].tags[0].name, "deprecated");
        }
        _ => panic!("Expected Protocol"),
    }
}

#[test]
fn protocol_with_bracket_tags() {
    let f = parse("protocol P { \"/ep\" [ #deprecated #required ] <Req, Res> };");
    match &f.definitions[0] {
        Definition::Protocol(p) => {
            assert_eq!(p.endpoints[0].tags.len(), 2);
            assert_eq!(p.endpoints[0].tags[0].name, "deprecated");
            assert_eq!(p.endpoints[0].tags[1].name, "required");
        }
        _ => panic!("Expected Protocol"),
    }
}

#[test]
fn multiple_definitions() {
    let f = parse("enum E { A }; variant V { X = string }; struct S { y = int64 };");
    assert_eq!(f.definitions.len(), 3);
    assert!(matches!(f.definitions[0], Definition::Enum(_)));
    assert!(matches!(f.definitions[1], Definition::Variant(_)));
    assert!(matches!(f.definitions[2], Definition::Struct(_)));
}

#[test]
fn enforcer_and_import_and_def() {
    let f = parse("!case=snake; @import ./foo.wst { T }; struct S { x = string };");
    assert_eq!(f.enforcers.len(), 1);
    assert_eq!(f.imports.len(), 1);
    assert_eq!(f.definitions.len(), 1);
}

#[test]
fn error_unknown_keyword() {
    let err = parse_err("blah Foo {};");
    assert!(err.contains("blah") || err.contains("enum") || err.contains("variant") || err.contains("struct") || err.contains("protocol"));
}

#[test]
fn error_bad_import() {
    let err = parse_err("@notimport");
    assert!(!err.is_empty());
}

// ── Constants ─────────────────────────────────────────────────────────────────

#[test]
fn const_enum_case() {
    let f = parse("enum E { A, B } const MY_CONST = E.A;");
    assert_eq!(f.definitions.len(), 2);
    let Definition::Const(c) = &f.definitions[1] else { panic!() };
    assert_eq!(c.name, "MY_CONST");
    assert!(matches!(&c.value, Expr::EnumCase { ty, case } if ty == "E" && case == "A"));
}

#[test]
fn const_struct_literal() {
    let f = parse("struct S { x = string #required } const MY_S = S { x = \"hello\" };");
    let Definition::Const(c) = &f.definitions[1] else { panic!() };
    assert_eq!(c.name, "MY_S");
    let Expr::Struct { ty, fields } = &c.value else { panic!() };
    assert_eq!(ty, "S");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "x");
    assert!(matches!(&fields[0].value, Expr::Str { value } if value == "hello"));
}

#[test]
fn const_nested_struct() {
    let f = parse("struct Inner { n = int32 #required } struct Outer { inner = Inner #required } const C = Outer { inner = Inner { n = 42 } };");
    let Definition::Const(c) = &f.definitions[2] else { panic!() };
    let Expr::Struct { ty, fields } = &c.value else { panic!() };
    assert_eq!(ty, "Outer");
    let Expr::Struct { ty: inner_ty, fields: inner_fields } = &fields[0].value else { panic!() };
    assert_eq!(inner_ty, "Inner");
    assert!(matches!(&inner_fields[0].value, Expr::Number { value } if *value == 42.0));
}

#[test]
fn const_null_field() {
    let f = parse("struct S { x = string } const C = S { x = null };");
    let Definition::Const(c) = &f.definitions[1] else { panic!() };
    let Expr::Struct { fields, .. } = &c.value else { panic!() };
    assert!(matches!(fields[0].value, Expr::Null));
}

#[test]
fn const_screaming_case_enforced() {
    // Parser accepts any identifier — screaming case is a validator concern
    let f = parse("enum E { A } const bad_name = E.A;");
    let Definition::Const(c) = &f.definitions[1] else { panic!() };
    assert_eq!(c.name, "bad_name"); // parser accepts it
}
