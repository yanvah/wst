use anyhow::{bail, Result};
use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<(Token, usize, usize)>,
    pos: usize,
    last_line: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, usize, usize)>) -> Self {
        Self { tokens, pos: 0, last_line: 1 }
    }

    fn current_pos(&self) -> (usize, usize) {
        self.tokens.get(self.pos).map(|(_, l, c)| (*l, *c)).unwrap_or((0, 0))
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).map(|(t, _, _)| t).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        match self.tokens.get(self.pos) {
            Some((tok, line, _)) => {
                self.last_line = *line;
                let result = tok.clone();
                self.pos += 1;
                result
            }
            None => Token::Eof,
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        let (line, col) = self.current_pos();
        match self.advance() {
            Token::Ident(s) => Ok(s),
            tok => bail!("{}:{}: Expected identifier, got {:?}", line, col, tok),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        let (line, col) = self.current_pos();
        let tok = self.advance();
        if tok == expected {
            Ok(())
        } else {
            bail!("{}:{}: Expected {:?}, got {:?}", line, col, expected, tok)
        }
    }

    /// Require a comma between items; trailing comma before `}` is allowed.
    fn expect_comma_or_close(&mut self, close: &Token) -> Result<()> {
        let (line, col) = self.current_pos();
        let next = self.peek().clone();
        if next == Token::Comma {
            self.advance();
        } else if &next != close {
            bail!("{}:{}: Expected ',' or closing delimiter, got {:?}", line, col, next);
        }
        Ok(())
    }

    pub fn parse_file(&mut self) -> Result<File> {
        let mut enforcers = Vec::new();
        let mut imports = Vec::new();
        let mut definitions = Vec::new();
        let mut assertion_defs = Vec::new();

        loop {
            match self.peek().clone() {
                Token::Eof => break,
                Token::Bang => {
                    self.advance();
                    enforcers.push(self.parse_enforcer()?);
                }
                Token::At => {
                    self.advance();
                    imports.push(self.parse_import()?);
                }
                Token::Ident(ref s) if s == "assertion" => {
                    self.advance();
                    assertion_defs.push(self.parse_assertion_def()?);
                }
                _ => {
                    definitions.push(self.parse_definition()?);
                }
            }
        }

        Ok(File { enforcers, imports, definitions, assertion_defs })
    }

    fn parse_enforcer(&mut self) -> Result<Enforcer> {
        let key = self.expect_ident()?;
        self.expect(Token::Equals)?;
        let value = self.expect_ident()?;
        self.expect(Token::Semicolon)?;
        Ok(Enforcer { key, value })
    }

    fn parse_import(&mut self) -> Result<Import> {
        let keyword = self.expect_ident()?;
        if keyword != "import" {
            bail!("Expected 'import' after '@', got '{}'", keyword);
        }
        let path = self.parse_path()?;
        let kind = if matches!(self.peek(), Token::Star) {
            self.advance();
            let name = self.expect_ident()?;
            self.expect(Token::Semicolon)?;
            ImportKind::Namespace { name }
        } else {
            let copy = if matches!(self.peek(), Token::Caret) {
                self.advance();
                let kw = self.expect_ident()?;
                if kw != "copy" {
                    bail!("Expected 'copy' after '^', got '{}'", kw);
                }
                true
            } else {
                false
            };
            self.expect(Token::LBrace)?;
            let mut types = Vec::new();
            loop {
                match self.peek().clone() {
                    Token::RBrace => { self.advance(); break; }
                    Token::Ident(name) => {
                        self.advance();
                        types.push(name);
                        if matches!(self.peek(), Token::Comma) {
                            self.advance();
                        }
                    }
                    other => {
                        let (line, col) = self.current_pos();
                        bail!("{}:{}: Expected type name or '}}', got {:?}", line, col, other);
                    }
                }
            }
            self.expect(Token::Semicolon)?;
            ImportKind::Named { copy, types }
        };
        Ok(Import { path, kind })
    }

    fn parse_path(&mut self) -> Result<String> {
        let mut path = String::new();
        loop {
            match self.peek() {
                Token::Dot => { path.push('.'); self.advance(); }
                Token::Slash => { path.push('/'); self.advance(); }
                Token::Ident(_) => {
                    if let Token::Ident(s) = self.advance() {
                        path.push_str(&s);
                    }
                }
                _ => break,
            }
        }
        if path.is_empty() {
            bail!("Expected import path");
        }
        Ok(path)
    }

    fn parse_definition(&mut self) -> Result<Definition> {
        let (line, col) = self.current_pos();
        let first = self.expect_ident()?;
        let (private, kw_line, kw_col, keyword) = if first == "private" {
            let (l, c) = self.current_pos();
            let kw = self.expect_ident()?;
            (true, l, c, kw)
        } else {
            (false, line, col, first)
        };
        match keyword.as_str() {
            "enum" => Ok(Definition::Enum(self.parse_enum(private)?)),
            "variant" => Ok(Definition::Variant(self.parse_variant(private)?)),
            "struct" => Ok(Definition::Struct(self.parse_struct(private)?)),
            "protocol" => Ok(Definition::Protocol(self.parse_protocol(private)?)),
            "const" => {
                if private {
                    bail!("{}:{}: 'const' cannot be 'private'", kw_line, kw_col);
                }
                Ok(Definition::Const(self.parse_const()?))
            }
            kw => bail!("{}:{}: Expected 'enum', 'variant', 'struct', 'protocol', or 'const'{}got '{}'",
                kw_line, kw_col,
                if private { " after 'private', " } else { ", " },
                kw),
        }
    }

    fn parse_enum(&mut self, private: bool) -> Result<EnumDef> {
        let name = self.expect_ident()?;
        let tags = self.parse_tags(self.last_line)?;
        self.expect(Token::LBrace)?;
        let mut cases = Vec::new();
        loop {
            let (line, col) = self.current_pos();
            match self.peek().clone() {
                Token::RBrace => { self.advance(); break; }
                Token::Ident(case_name) => {
                    self.advance();
                    let tags = self.parse_tags(self.last_line)?;
                    cases.push(EnumCase { name: case_name, tags });
                    self.expect_comma_or_close(&Token::RBrace)?;
                }
                other => bail!("{}:{}: Expected case name or '}}', got {:?}", line, col, other),
            }
        }
        if matches!(self.peek(), Token::Semicolon) { self.advance(); }
        Ok(EnumDef { name, tags, cases, private })
    }

    fn parse_variant(&mut self, private: bool) -> Result<VariantDef> {
        let name = self.expect_ident()?;
        let tags = self.parse_tags(self.last_line)?;
        self.expect(Token::LBrace)?;
        let mut cases = Vec::new();
        loop {
            let (line, col) = self.current_pos();
            match self.peek().clone() {
                Token::RBrace => { self.advance(); break; }
                Token::Ident(case_name) => {
                    self.advance();
                    self.expect(Token::Equals)?;
                    let ty = self.parse_type()?;
                    let tags = self.parse_tags(self.last_line)?;
                    cases.push(VariantCase { name: case_name, ty, tags, line });
                    self.expect_comma_or_close(&Token::RBrace)?;
                }
                other => bail!("{}:{}: Expected case name or '}}', got {:?}", line, col, other),
            }
        }
        if matches!(self.peek(), Token::Semicolon) { self.advance(); }
        Ok(VariantDef { name, tags, cases, private })
    }

    fn parse_struct(&mut self, private: bool) -> Result<StructDef> {
        let name = self.expect_ident()?;
        let tags = self.parse_tags(self.last_line)?;
        self.expect(Token::LBrace)?;
        let mut copies = Vec::new();
        let mut fields = Vec::new();
        let mut asserts = Vec::new();
        loop {
            let (line, col) = self.current_pos();
            match self.peek().clone() {
                Token::RBrace => { self.advance(); break; }
                Token::Ident(kw) if kw == "copy" => {
                    self.advance();
                    if !fields.is_empty() {
                        bail!("{}:{}: 'copy' must appear before all field definitions", line, col);
                    }
                    let source = self.parse_struct_source()?;
                    copies.push(source);
                    self.expect_comma_or_close(&Token::RBrace)?;
                }
                Token::Ident(kw) if kw == "assert" => {
                    self.advance();
                    asserts.push(self.parse_assert_ref(line, col)?);
                    // Optional trailing comma — asserts are block-terminated
                    if matches!(self.peek(), Token::Comma) { self.advance(); }
                }
                Token::Ident(first) => {
                    self.advance();
                    let field_name = if matches!(self.peek(), Token::Dot) {
                        self.advance();
                        let case = self.expect_ident()?;
                        format!("{}.{}", first, case)
                    } else {
                        first
                    };
                    self.expect(Token::Equals)?;
                    let ty = self.parse_type()?;
                    let tags = self.parse_tags(self.last_line)?;
                    fields.push(StructField { name: field_name, ty, tags, line });
                    self.expect_comma_or_close(&Token::RBrace)?;
                }
                other => bail!("{}:{}: Expected 'copy', 'assert', field name, or '}}', got {:?}", line, col, other),
            }
        }
        if matches!(self.peek(), Token::Semicolon) { self.advance(); }
        Ok(StructDef { name, tags, copies, fields, asserts, private })
    }

    /// Parse `($s) { body }` or `AssertionName` after the `assert` keyword.
    fn parse_assert_ref(&mut self, line: usize, col: usize) -> Result<AssertRef> {
        match self.peek().clone() {
            Token::LParen => {
                self.advance();
                let param = self.expect_dollar_ident()?;
                self.expect(Token::RParen)?;
                let body = self.parse_assertion_body()?;
                Ok(AssertRef::Inline { param, body, line })
            }
            Token::Ident(name) => {
                self.advance();
                Ok(AssertRef::Named { name, line })
            }
            other => bail!("{}:{}: Expected '(' or assertion name after 'assert', got {:?}", line, col, other),
        }
    }

    fn parse_assertion_def(&mut self) -> Result<AssertionDef> {
        let name = self.expect_ident()?;
        self.expect(Token::LParen)?;
        let (line, col) = self.current_pos();
        let kind = self.expect_ident()?;
        if kind != "struct" {
            bail!("{}:{}: Expected 'struct' as assertion parameter type, got '{}'", line, col, kind);
        }
        let param = self.expect_dollar_ident()?;
        self.expect(Token::RParen)?;
        let body = self.parse_assertion_body()?;
        if matches!(self.peek(), Token::Semicolon) { self.advance(); }
        Ok(AssertionDef { name, param, body })
    }

    fn parse_assertion_body(&mut self) -> Result<Vec<AssertionStmt>> {
        self.expect(Token::LBrace)?;
        let mut stmts = Vec::new();
        loop {
            let (line, col) = self.current_pos();
            match self.peek().clone() {
                Token::RBrace => { self.advance(); break; }
                Token::Ident(kw) if kw == "for" => {
                    self.advance();
                    stmts.push(self.parse_assertion_for()?);
                }
                other => bail!("{}:{}: Expected 'for' or '}}' in assertion body, got {:?}", line, col, other),
            }
        }
        Ok(stmts)
    }

    fn parse_assertion_for(&mut self) -> Result<AssertionStmt> {
        let for_line = self.last_line;
        let var = self.expect_dollar_ident()?;
        let (line, col) = self.current_pos();
        let kw = self.expect_ident()?;
        if kw != "in" {
            bail!("{}:{}: Expected 'in' after loop variable, got '{}'", line, col, kw);
        }
        let source = self.expect_ident()?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        loop {
            let (line, col) = self.current_pos();
            match self.peek().clone() {
                Token::RBrace => { self.advance(); break; }
                Token::Dollar => {
                    let haskey_line = line;
                    let subject = self.expect_dollar_ident()?;
                    let (line, col) = self.current_pos();
                    let check = self.expect_ident()?;
                    match check.as_str() {
                        "haskey" => {
                            let key = self.expect_dollar_ident()?;
                            body.push(AssertionStmt::HasKey { subject, key, line: haskey_line });
                        }
                        other => bail!("{}:{}: Unknown assertion check '{}'; expected 'haskey'", line, col, other),
                    }
                }
                other => bail!("{}:{}: Expected assertion check or '}}' in for body, got {:?}", line, col, other),
            }
        }
        Ok(AssertionStmt::ForIn { var, source, body, line: for_line })
    }

    /// Expect `$identifier` — consumes Dollar then Ident, returns the identifier string.
    fn expect_dollar_ident(&mut self) -> Result<String> {
        self.expect(Token::Dollar)?;
        self.expect_ident()
    }

    fn parse_struct_source(&mut self) -> Result<StructSource> {
        let (line, col) = self.current_pos();
        match self.peek().clone() {
            Token::At => {
                self.advance();
                let op = self.expect_ident()?;
                match op.as_str() {
                    "exclude" => {
                        self.expect(Token::LParen)?;
                        let base = self.expect_ident()?;
                        self.expect(Token::Comma)?;
                        self.expect(Token::LBracket)?;
                        let mut exclude = Vec::new();
                        loop {
                            match self.peek().clone() {
                                Token::RBracket => { self.advance(); break; }
                                Token::StringLit(s) => {
                                    self.advance();
                                    exclude.push(s);
                                    if matches!(self.peek(), Token::Comma) { self.advance(); }
                                }
                                other => {
                                    let (l, c) = self.current_pos();
                                    bail!("{}:{}: Expected field name string or ']', got {:?}", l, c, other);
                                }
                            }
                        }
                        self.expect(Token::RParen)?;
                        Ok(StructSource::Exclude { base, exclude })
                    }
                    _ => bail!("{}:{}: Unknown @-op '{}'; expected 'exclude'", line, col, op),
                }
            }
            Token::Ident(name) => {
                self.advance();
                Ok(StructSource::Named { name })
            }
            other => bail!("{}:{}: Expected struct name or @-op after 'copy', got {:?}", line, col, other),
        }
    }

    fn parse_const(&mut self) -> Result<ConstDef> {
        let name = self.expect_ident()?;
        self.expect(Token::Equals)?;
        let value = self.parse_expr()?;
        if matches!(self.peek(), Token::Semicolon) { self.advance(); }
        Ok(ConstDef { name, value })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        let (line, col) = self.current_pos();
        match self.peek().clone() {
            Token::StringLit(s) => { self.advance(); Ok(Expr::Str { value: s }) }
            Token::NumberLit(n) => { self.advance(); Ok(Expr::Number { value: n }) }
            Token::BoolLit(b) => { self.advance(); Ok(Expr::Bool { value: b }) }
            Token::Ident(kw) if kw == "null" => { self.advance(); Ok(Expr::Null) }
            Token::Ident(first) => {
                self.advance();
                // Handle qualified name: Either.Case or Namespace.Type { ... }
                let (ty, maybe_case) = if matches!(self.peek(), Token::Dot) {
                    self.advance();
                    let second = self.expect_ident()?;
                    (first, Some(second))
                } else {
                    (first, None)
                };
                if matches!(self.peek(), Token::LBrace) {
                    let full_ty = match maybe_case {
                        Some(part) => format!("{}.{}", ty, part),
                        None => ty,
                    };
                    let fields = self.parse_expr_fields()?;
                    Ok(Expr::Struct { ty: full_ty, fields })
                } else {
                    match maybe_case {
                        Some(case) => Ok(Expr::EnumCase { ty, case }),
                        None => bail!("{}:{}: Expected '{{' or '.Case' after type name in expression", line, col),
                    }
                }
            }
            other => bail!("{}:{}: Expected expression value, got {:?}", line, col, other),
        }
    }

    fn parse_expr_fields(&mut self) -> Result<Vec<ExprField>> {
        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();
        loop {
            let (line, col) = self.current_pos();
            match self.peek().clone() {
                Token::RBrace => { self.advance(); break; }
                Token::Ident(fname) => {
                    self.advance();
                    let fname = if matches!(self.peek(), Token::Dot) {
                        self.advance();
                        let part2 = self.expect_ident()?;
                        format!("{}.{}", fname, part2)
                    } else {
                        fname
                    };
                    self.expect(Token::Equals)?;
                    let value = self.parse_expr()?;
                    fields.push(ExprField { name: fname, value });
                    self.expect_comma_or_close(&Token::RBrace)?;
                }
                other => bail!("{}:{}: Expected field name or '}}' in struct literal, got {:?}", line, col, other),
            }
        }
        Ok(fields)
    }

    fn parse_protocol(&mut self, private: bool) -> Result<ProtocolDef> {
        let name = self.expect_ident()?;
        let tags = self.parse_tags(self.last_line)?;
        self.expect(Token::LBrace)?;
        let mut endpoints = Vec::new();
        loop {
            let (line, col) = self.current_pos();
            match self.peek().clone() {
                Token::RBrace => { self.advance(); break; }
                Token::StringLit(ep_name) => {
                    self.advance();
                    let tags = self.parse_tags(self.last_line)?;
                    self.expect(Token::LAngle)?;
                    let request = self.parse_type()?;
                    self.expect(Token::Comma)?;
                    let response = self.parse_type()?;
                    let error = if matches!(self.peek(), Token::Bang) {
                        self.advance();
                        Some(self.parse_type()?)
                    } else {
                        None
                    };
                    self.expect(Token::RAngle)?;
                    endpoints.push(Endpoint { name: ep_name, tags, request, response, error, line });
                    self.expect_comma_or_close(&Token::RBrace)?;
                }
                other => bail!("{}:{}: Expected endpoint name (string literal) or '}}', got {:?}", line, col, other),
            }
        }
        if matches!(self.peek(), Token::Semicolon) { self.advance(); }
        Ok(ProtocolDef { name, tags, endpoints, private })
    }

    fn parse_type(&mut self) -> Result<TypeRef> {
        let (line, col) = self.current_pos();
        match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                match name.as_str() {
                    "int32" => Ok(TypeRef::Primitive(Primitive::Int32)),
                    "int64" => Ok(TypeRef::Primitive(Primitive::Int64)),
                    "uin64" => Ok(TypeRef::Primitive(Primitive::Uin64)),
                    "flt64" => Ok(TypeRef::Primitive(Primitive::Flt64)),
                    "boolean" => Ok(TypeRef::Primitive(Primitive::Boolean)),
                    "string" => Ok(TypeRef::Primitive(Primitive::Str)),
                    "vec" => {
                        self.expect(Token::LAngle)?;
                        let inner = self.parse_type()?;
                        self.expect(Token::RAngle)?;
                        Ok(TypeRef::Vec(Box::new(inner)))
                    }
                    "map" => {
                        self.expect(Token::LAngle)?;
                        let key = self.parse_primitive()?;
                        self.expect(Token::Comma)?;
                        let value = self.parse_type()?;
                        self.expect(Token::RAngle)?;
                        Ok(TypeRef::Map { key, value: Box::new(value) })
                    }
                    _ => {
                        let mut full = name;
                        while matches!(self.peek(), Token::Dot) {
                            self.advance();
                            let part = self.expect_ident()?;
                            full.push('.');
                            full.push_str(&part);
                        }
                        Ok(TypeRef::Named(full))
                    }
                }
            }
            other => bail!("{}:{}: Expected type, got {:?}", line, col, other),
        }
    }

    fn parse_primitive(&mut self) -> Result<Primitive> {
        let (line, col) = self.current_pos();
        let name = self.expect_ident()?;
        match name.as_str() {
            "int32" => Ok(Primitive::Int32),
            "int64" => Ok(Primitive::Int64),
            "uin64" => Ok(Primitive::Uin64),
            "flt64" => Ok(Primitive::Flt64),
            "boolean" => Ok(Primitive::Boolean),
            "string" => Ok(Primitive::Str),
            _ => bail!("{}:{}: Expected primitive type, got '{}'", line, col, name),
        }
    }

    fn parse_tags(&mut self, prev_line: usize) -> Result<Vec<Tag>> {
        if matches!(self.peek(), Token::LBracket) {
            self.advance();
            let mut tags = Vec::new();
            while !matches!(self.peek(), Token::RBracket | Token::Eof) {
                tags.push(self.parse_tag()?);
            }
            self.expect(Token::RBracket)?;
            Ok(tags)
        } else {
            let mut tags = Vec::new();
            while matches!(self.peek(), Token::Hash) {
                let (tag_line, tag_col) = self.current_pos();
                if tag_line != prev_line {
                    bail!("{}:{}: Inline tags must be on the same line as their field; use [...] for multi-line tags", tag_line, tag_col);
                }
                tags.push(self.parse_tag()?);
            }
            Ok(tags)
        }
    }

    fn parse_tag(&mut self) -> Result<Tag> {
        self.expect(Token::Hash)?;
        let first = self.expect_ident()?;
        let (namespace, name) = if matches!(self.peek(), Token::Colon) {
            self.advance();
            let tag_name = self.expect_ident()?;
            (Some(first), tag_name)
        } else {
            (None, first)
        };
        let value = if matches!(self.peek(), Token::Equals) {
            self.advance();
            let (line, col) = self.current_pos();
            match self.advance() {
                Token::StringLit(s) => TagValue::Str(s),
                Token::NumberLit(n) => TagValue::Number(n),
                Token::BoolLit(b) => TagValue::Bool(b),
                tok => bail!("{}:{}: Expected tag value (string, number, or bool), got {:?}", line, col, tok),
            }
        } else {
            TagValue::Bool(true)
        };
        Ok(Tag { namespace, name, value })
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
