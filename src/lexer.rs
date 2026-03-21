use anyhow::{bail, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    StringLit(String),
    NumberLit(f64),
    BoolLit(bool),
    Hash,
    Colon,
    Equals,
    Comma,
    Semicolon,
    LBrace,
    RBrace,
    LAngle,
    RAngle,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Dollar,
    Bang,
    At,
    Caret,
    Star,
    Dot,
    Slash,
    Eof,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self { chars: input.chars().collect(), pos: 0, line: 1, col: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek().map_or(false, |c| c.is_whitespace()) {
                self.advance();
            }
            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                while self.peek().map_or(false, |c| c != '\n') {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<(Token, usize, usize)>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let line = self.line;
            let col = self.col;
            match self.peek() {
                None => {
                    tokens.push((Token::Eof, line, col));
                    break;
                }
                Some(c) => {
                    let tok = self.lex_token(c)?;
                    tokens.push((tok, line, col));
                }
            }
        }
        Ok(tokens)
    }

    fn lex_token(&mut self, c: char) -> Result<Token> {
        Ok(match c {
            '#' => { self.advance(); Token::Hash }
            ':' => { self.advance(); Token::Colon }
            '=' => { self.advance(); Token::Equals }
            ',' => { self.advance(); Token::Comma }
            ';' => { self.advance(); Token::Semicolon }
            '{' => { self.advance(); Token::LBrace }
            '}' => { self.advance(); Token::RBrace }
            '<' => { self.advance(); Token::LAngle }
            '>' => { self.advance(); Token::RAngle }
            '[' => { self.advance(); Token::LBracket }
            ']' => { self.advance(); Token::RBracket }
            '(' => { self.advance(); Token::LParen }
            ')' => { self.advance(); Token::RParen }
            '$' => { self.advance(); Token::Dollar }
            '!' => { self.advance(); Token::Bang }
            '@' => { self.advance(); Token::At }
            '^' => { self.advance(); Token::Caret }
            '*' => { self.advance(); Token::Star }
            '.' => { self.advance(); Token::Dot }
            '/' => { self.advance(); Token::Slash }
            '"' => self.lex_string()?,
            c if c.is_ascii_digit() => self.lex_number()?,
            '-' if self.peek_next().map_or(false, |c| c.is_ascii_digit()) => self.lex_number()?,
            c if c.is_alphabetic() || c == '_' => self.lex_ident()?,
            c => bail!("{}:{}: Unexpected character: {:?}", self.line, self.col, c),
        })
    }

    fn lex_string(&mut self) -> Result<Token> {
        self.advance(); // opening "
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(c) => { s.push('\\'); s.push(c); }
                    None => bail!("Unterminated string escape"),
                },
                Some(c) => s.push(c),
                None => bail!("Unterminated string literal"),
            }
        }
        Ok(Token::StringLit(s))
    }

    fn lex_number(&mut self) -> Result<Token> {
        let mut s = String::new();
        if self.peek() == Some('-') {
            s.push('-');
            self.advance();
        }
        while self.peek().map_or(false, |c| c.is_ascii_digit() || c == '.') {
            s.push(self.advance().unwrap());
        }
        let n: f64 = s.parse().map_err(|_| anyhow::anyhow!("Invalid number: {}", s))?;
        Ok(Token::NumberLit(n))
    }

    fn lex_ident(&mut self) -> Result<Token> {
        let mut s = String::new();
        while self.peek().map_or(false, |c| c.is_alphanumeric() || c == '_') {
            s.push(self.advance().unwrap());
        }
        Ok(match s.as_str() {
            "true" => Token::BoolLit(true),
            "false" => Token::BoolLit(false),
            _ => Token::Ident(s),
        })
    }
}

#[cfg(test)]
#[path = "lexer_tests.rs"]
mod tests;
