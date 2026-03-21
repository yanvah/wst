use super::{Lexer, Token};

fn tokenize(src: &str) -> Vec<Token> {
    Lexer::new(src).tokenize().unwrap().into_iter().map(|(t, _, _)| t).collect()
}

#[test]
fn empty_input() {
    assert_eq!(tokenize(""), vec![Token::Eof]);
}

#[test]
fn single_char_hash() {
    assert_eq!(tokenize("#"), vec![Token::Hash, Token::Eof]);
}

#[test]
fn single_char_colon() {
    assert_eq!(tokenize(":"), vec![Token::Colon, Token::Eof]);
}

#[test]
fn single_char_equals() {
    assert_eq!(tokenize("="), vec![Token::Equals, Token::Eof]);
}

#[test]
fn single_char_comma() {
    assert_eq!(tokenize(","), vec![Token::Comma, Token::Eof]);
}

#[test]
fn single_char_semicolon() {
    assert_eq!(tokenize(";"), vec![Token::Semicolon, Token::Eof]);
}

#[test]
fn single_char_lbrace() {
    assert_eq!(tokenize("{"), vec![Token::LBrace, Token::Eof]);
}

#[test]
fn single_char_rbrace() {
    assert_eq!(tokenize("}"), vec![Token::RBrace, Token::Eof]);
}

#[test]
fn single_char_langle() {
    assert_eq!(tokenize("<"), vec![Token::LAngle, Token::Eof]);
}

#[test]
fn single_char_rangle() {
    assert_eq!(tokenize(">"), vec![Token::RAngle, Token::Eof]);
}

#[test]
fn single_char_lbracket() {
    assert_eq!(tokenize("["), vec![Token::LBracket, Token::Eof]);
}

#[test]
fn single_char_rbracket() {
    assert_eq!(tokenize("]"), vec![Token::RBracket, Token::Eof]);
}

#[test]
fn single_char_bang() {
    assert_eq!(tokenize("!"), vec![Token::Bang, Token::Eof]);
}

#[test]
fn single_char_at() {
    assert_eq!(tokenize("@"), vec![Token::At, Token::Eof]);
}

#[test]
fn single_char_caret() {
    assert_eq!(tokenize("^"), vec![Token::Caret, Token::Eof]);
}

#[test]
fn single_char_star() {
    assert_eq!(tokenize("*"), vec![Token::Star, Token::Eof]);
}

#[test]
fn single_char_dot() {
    assert_eq!(tokenize("."), vec![Token::Dot, Token::Eof]);
}

#[test]
fn single_char_slash() {
    assert_eq!(tokenize("/"), vec![Token::Slash, Token::Eof]);
}

#[test]
fn identifier() {
    assert_eq!(
        tokenize("hello_world"),
        vec![Token::Ident("hello_world".to_string()), Token::Eof]
    );
}

#[test]
fn identifier_with_digits() {
    assert_eq!(
        tokenize("foo123"),
        vec![Token::Ident("foo123".to_string()), Token::Eof]
    );
}

#[test]
fn bool_true() {
    assert_eq!(tokenize("true"), vec![Token::BoolLit(true), Token::Eof]);
}

#[test]
fn bool_false() {
    assert_eq!(tokenize("false"), vec![Token::BoolLit(false), Token::Eof]);
}

#[test]
fn string_literal() {
    assert_eq!(
        tokenize("\"hello\""),
        vec![Token::StringLit("hello".to_string()), Token::Eof]
    );
}

#[test]
fn string_escape_newline() {
    assert_eq!(
        tokenize("\"\\n\""),
        vec![Token::StringLit("\n".to_string()), Token::Eof]
    );
}

#[test]
fn string_escape_tab() {
    assert_eq!(
        tokenize("\"\\t\""),
        vec![Token::StringLit("\t".to_string()), Token::Eof]
    );
}

#[test]
fn string_escape_quote() {
    assert_eq!(
        tokenize("\"\\\"\""),
        vec![Token::StringLit("\"".to_string()), Token::Eof]
    );
}

#[test]
fn string_escape_backslash() {
    assert_eq!(
        tokenize("\"\\\\\""),
        vec![Token::StringLit("\\".to_string()), Token::Eof]
    );
}

#[test]
fn number_integer() {
    assert_eq!(tokenize("42"), vec![Token::NumberLit(42.0), Token::Eof]);
}

#[test]
fn number_float() {
    assert_eq!(tokenize("3.14"), vec![Token::NumberLit(3.14), Token::Eof]);
}

#[test]
fn number_negative() {
    assert_eq!(tokenize("-7"), vec![Token::NumberLit(-7.0), Token::Eof]);
}

#[test]
fn skips_spaces() {
    assert_eq!(tokenize("  #  "), vec![Token::Hash, Token::Eof]);
}

#[test]
fn skips_newlines() {
    assert_eq!(tokenize("\n#\n"), vec![Token::Hash, Token::Eof]);
}

#[test]
fn skips_line_comment() {
    assert_eq!(tokenize("// comment\n#"), vec![Token::Hash, Token::Eof]);
}

#[test]
fn skips_multiple_comments() {
    assert_eq!(
        tokenize("// first\n// second\n#"),
        vec![Token::Hash, Token::Eof]
    );
}

#[test]
fn struct_keyword_is_ident() {
    assert_eq!(
        tokenize("struct"),
        vec![Token::Ident("struct".to_string()), Token::Eof]
    );
}

#[test]
fn path_tokens() {
    assert_eq!(
        tokenize("./foo/bar.wst"),
        vec![
            Token::Dot,
            Token::Slash,
            Token::Ident("foo".to_string()),
            Token::Slash,
            Token::Ident("bar".to_string()),
            Token::Dot,
            Token::Ident("wst".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn hash_ident_sequence() {
    assert_eq!(
        tokenize("#deprecated"),
        vec![Token::Hash, Token::Ident("deprecated".to_string()), Token::Eof]
    );
}

#[test]
fn hash_namespace_tag() {
    assert_eq!(
        tokenize("#myorg:something"),
        vec![
            Token::Hash,
            Token::Ident("myorg".to_string()),
            Token::Colon,
            Token::Ident("something".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn complete_struct_tokens() {
    assert_eq!(
        tokenize("struct Foo { x = string };"),
        vec![
            Token::Ident("struct".to_string()),
            Token::Ident("Foo".to_string()),
            Token::LBrace,
            Token::Ident("x".to_string()),
            Token::Equals,
            Token::Ident("string".to_string()),
            Token::RBrace,
            Token::Semicolon,
            Token::Eof,
        ]
    );
}
