//! The lexer for the Patui expression parser. This module relies on the `logos` third party crate
//! to turn a string of a Patui expression into tokens that can then be parsed more easily into a
//! `PatuiExpr` struct.

use std::{iter::Peekable, ops::Range};

use logos::{Lexer, Logos};
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Error)]
pub enum LexingError {
    #[error("Error parsing either a number or ident: {0}")]
    BadNumberOrIdent(String),
    #[error("Error parsing string: {0}")]
    BadString(String),
    #[error("Error parsing token: {0}")]
    BadToken(String),
    #[error("Unknown lexing error")]
    #[default]
    Unknown,
}

#[derive(Logos, Clone, Debug, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(error = LexingError)]
pub enum Token {
    #[token("null")]
    Null,

    #[token("false", |_| false)]
    #[token("False", |_| false)]
    #[token("true", |_| true)]
    #[token("True", |_| true)]
    Bool(bool),

    #[regex(r"-?[1-9][0-9_]*|0|0[xX][0-9a-fA-F]+|0[bB][01]+", priority = 6, callback = |lex| lex.slice().to_lowercase())]
    Integer(String),

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)|-?(?:0|[1-9]\d*)(?:[eE][+-]?\d+)|-?(?:0|[1-9]\d*)(?:\.\d+)(?:[eE][+-]?\d+)", priority = 8, callback = |lex| lex.slice().to_string())]
    Decimal(String),

    #[regex(r"-?[1-9][0-9]*[a-zA-Z_][0-9a-zA-Z_]*|0[xX][0-9a-fA-F]+[g-zG-Z_][0-9a-zA-Z_]*|0[bB][01]+[2-9a-zA-Z_][0-9a-zA-Z_]*", priority = 7, callback = |lex| Err(LexingError::BadNumberOrIdent(lex.slice().to_string())))]
    BadNumberOrIdent(String),

    // NB: Needs to be processed in parser to interpret quotes correctly
    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice()[..].to_string())]
    String(String),

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*"#, |lex| Err(LexingError::BadString(lex.slice()[..].to_string())))]
    BadString(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 1, callback = |lex| lex.slice().to_string())]
    Ident(String),

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("b", priority = 10)]
    BytesPrefix,

    #[token("[")]
    LeftSquareBrace,

    #[token("]")]
    RightSquareBrace,

    #[token("{")]
    LeftCurlyBrace,

    #[token("}")]
    RightCurlyBrace,

    #[token("(")]
    LeftBracket,

    #[token(")")]
    RightBracket,

    #[token("..")]
    Range,

    #[token(".")]
    Period,

    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    #[token(";")]
    Semicolon,

    #[token("+")]
    Add,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[allow(clippy::enum_variant_names)]
    #[token("&", |_| Err(LexingError::BadToken("&".to_string())))]
    BadAnd(String),

    #[token("&&")]
    And,

    #[allow(clippy::enum_variant_names)]
    #[token("|", |_| Err(LexingError::BadToken("|".to_string())))]
    BadOr(String),

    #[token("||")]
    Or,

    #[token("!")]
    Not,

    #[allow(clippy::enum_variant_names)]
    #[token("=", |_| Err(LexingError::BadToken("=".to_string())))]
    BadEquality(String),

    #[token("==")]
    Equal,

    #[token("!=")]
    NotEqual,

    #[token("<")]
    LessThan,

    #[token("<=")]
    LessThanEqual,

    #[token(">")]
    GreaterThan,

    #[token(">=")]
    GreaterThanEqual,
}

// Wrapper around Logos Lexer, needs to be peekable and inspectable at the
// same time, i.e. we need to be able to peek at the next token without eating
// the Lexer with a `peekable`, so we clone and get both.
//
// NB: This must not mutate anything internal to Lexer or the clone will have
// caused issues.
pub(crate) struct LexerPeekable<'a> {
    lexer: Lexer<'a, Token>,
    peekable_iter: Peekable<Lexer<'a, Token>>,
}

impl<'a> LexerPeekable<'a> {
    pub(crate) fn new(lexer: Lexer<'a, Token>) -> LexerPeekable<'a> {
        let peekable_iter = lexer.clone().peekable();

        Self {
            lexer,
            peekable_iter,
        }
    }

    pub(crate) fn next(&mut self) -> Option<Result<Token, LexingError>> {
        let peek_next = self.peekable_iter.next();
        let next = self.lexer.next();
        if peek_next.is_some() && next.is_none() {
            panic!("Peekable but no next in LexerPeekable");
        }
        if peek_next.is_none() && next.is_some() {
            panic!("Next but not peekable in LexerPeekable");
        }
        tracing::trace!("Found next token: {:?}", next);
        next
    }

    /// Check if next token matches, if it does eat it, if not leave it as next
    /// edible token.
    pub(crate) fn next_if_match(&mut self, match_token: Token) -> bool {
        if let Some(peek_next) = self.peek() {
            tracing::trace!("Peeking next: {:?}", peek_next);
            match peek_next {
                Ok(next_tok) => {
                    if *next_tok == match_token {
                        tracing::trace!("Matched");
                        self.next().unwrap().unwrap();
                        return true;
                    }
                }
                Err(e) => tracing::error!("Error matching next token, assuming no match: {:?}", e),
            }
        }

        false
    }

    pub(crate) fn peek(&mut self) -> Option<&Result<Token, LexingError>> {
        self.peekable_iter.peek()
    }

    pub(crate) fn span(&self) -> Range<usize> {
        self.lexer.span()
    }

    pub(crate) fn rest(mut self) -> Result<Vec<Token>, LexingError> {
        let mut ret = vec![];
        while let Some(tok) = self.next() {
            match tok {
                Ok(tok) => ret.push(tok),
                Err(e) => return Err(e),
            }
        }
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use assertor::*;

    use super::*;

    fn single_successful_lex(input: &str, parsed: Token, span: Range<usize>, slice: &str) {
        let mut lex = Token::lexer(input);
        let tok = lex.next();
        assert_that!(tok).is_some();
        let tok = tok.unwrap();
        assert_that!(tok).is_ok();
        let tok = tok.unwrap();
        assert_that!(tok).is_equal_to(parsed);
        assert_that!(lex.span()).is_equal_to(span);
        assert_that!(lex.slice()).is_equal_to(slice);
    }

    fn single_unsuccessful_lex(
        input: &str,
        expected_err: LexingError,
        span: Range<usize>,
        slice: &str,
    ) {
        let mut lex = Token::lexer(input);
        let tok = lex.next();
        assert_that!(tok).is_some();
        let tok = tok.unwrap();
        assert_that!(tok).is_err();
        let tok = tok.unwrap_err();
        assert_that!(tok).is_equal_to(expected_err);
        assert_that!(lex.span()).is_equal_to(span);
        assert_that!(lex.slice()).is_equal_to(slice);
    }

    #[test]
    fn lex_number() {
        single_successful_lex("123", Token::Integer("123".to_string()), 0..3, "123");
        single_successful_lex(
            "123.45",
            Token::Decimal("123.45".to_string()),
            0..6,
            "123.45",
        );
        single_successful_lex(
            "123e45",
            Token::Decimal("123e45".to_string()),
            0..6,
            "123e45",
        );
        single_successful_lex(
            "0b00110001",
            Token::Integer("0b00110001".to_string()),
            0..10,
            "0b00110001",
        );
        single_successful_lex(
            "0x123abC",
            Token::Integer("0x123abc".to_string()),
            0..8,
            "0x123abC",
        );
    }

    #[test]
    fn lex_number_errors() {
        single_unsuccessful_lex(
            "123a45",
            LexingError::BadNumberOrIdent("123a45".to_string()),
            0..6,
            "123a45",
        );
        single_unsuccessful_lex(
            "0b123",
            LexingError::BadNumberOrIdent("0b123".to_string()),
            0..5,
            "0b123",
        );
        single_unsuccessful_lex(
            "0x123z",
            LexingError::BadNumberOrIdent("0x123z".to_string()),
            0..6,
            "0x123z",
        );
        single_unsuccessful_lex(
            "123adzzz",
            LexingError::BadNumberOrIdent("123adzzz".to_string()),
            0..8,
            "123adzzz",
        );
    }

    #[test]
    fn lex_null() {
        single_successful_lex("null", Token::Null, 0..4, "null");
    }

    #[test]
    fn lex_bool() {
        single_successful_lex("true", Token::Bool(true), 0..4, "true");
        single_successful_lex("False", Token::Bool(false), 0..5, "False");
    }

    #[test]
    fn lex_string() {
        single_successful_lex(
            r#""foo bar boo""#,
            Token::String(r#""foo bar boo""#.to_string()),
            0..13,
            r#""foo bar boo""#,
        );
        single_successful_lex(
            "\"foo\nbar\nboo\"",
            Token::String("\"foo\nbar\nboo\"".to_string()),
            0..13,
            "\"foo\nbar\nboo\"",
        );
        single_successful_lex(
            r#""foo\"bar\"boo""#,
            Token::String(r#""foo\"bar\"boo""#.to_string()),
            0..15,
            r#""foo\"bar\"boo""#,
        );
    }

    #[test]
    fn lex_string_errors() {
        single_unsuccessful_lex(
            r#""foo bar boo"#,
            LexingError::BadString(r#""foo bar boo"#.to_string()),
            0..12,
            r#""foo bar boo"#,
        );
    }

    #[test]
    fn lex_ident() {
        single_successful_lex("foo", Token::Ident("foo".to_string()), 0..3, "foo");
        single_successful_lex(
            "foo_123_ABC_bar",
            Token::Ident("foo_123_ABC_bar".to_string()),
            0..15,
            "foo_123_ABC_bar",
        );
        single_successful_lex(
            "__foo__123__ABC__bar__",
            Token::Ident("__foo__123__ABC__bar__".to_string()),
            0..22,
            "__foo__123__ABC__bar__",
        );
    }

    #[test]
    fn control_tokens() {
        single_successful_lex("[", Token::LeftSquareBrace, 0..1, "[");
        single_successful_lex("]", Token::RightSquareBrace, 0..1, "]");
        single_successful_lex("{", Token::LeftCurlyBrace, 0..1, "{");
        single_successful_lex("}", Token::RightCurlyBrace, 0..1, "}");
        single_successful_lex("(", Token::LeftBracket, 0..1, "(");
        single_successful_lex(")", Token::RightBracket, 0..1, ")");
        single_successful_lex(".", Token::Period, 0..1, ".");
        single_successful_lex(",", Token::Comma, 0..1, ",");
        single_successful_lex(":", Token::Colon, 0..1, ":");
        single_successful_lex(";", Token::Semicolon, 0..1, ";");
    }

    #[test]
    fn maths_tokens() {
        single_successful_lex("+", Token::Add, 0..1, "+");
        single_successful_lex("-", Token::Minus, 0..1, "-");
        single_successful_lex("*", Token::Star, 0..1, "*");
        single_successful_lex("/", Token::Slash, 0..1, "/");
        single_successful_lex("%", Token::Percent, 0..1, "%");
    }

    #[test]
    fn logical_tokens() {
        single_successful_lex("&&", Token::And, 0..2, "&&");
        single_successful_lex("||", Token::Or, 0..2, "||");
        single_successful_lex("!", Token::Not, 0..1, "!");
    }

    #[test]
    fn comparison_tokens() {
        single_successful_lex("==", Token::Equal, 0..2, "==");
        single_successful_lex("!=", Token::NotEqual, 0..2, "!=");
        single_successful_lex("<", Token::LessThan, 0..1, "<");
        single_successful_lex("<=", Token::LessThanEqual, 0..2, "<=");
        single_successful_lex(">", Token::GreaterThan, 0..1, ">");
        single_successful_lex(">=", Token::GreaterThanEqual, 0..2, ">=");
    }

    #[test]
    fn lex_complex() {
        let mut lex =
            Token::lexer("foo123[1].bar if bar else baz && baz == true || (true && false)");
        for (expected_parsed, expected_span, expected_slice) in vec![
            (Token::Ident("foo123".to_string()), 0..6, "foo123"),
            (Token::LeftSquareBrace, 6..7, "["),
            (Token::Integer("1".to_string()), 7..8, "1"),
            (Token::RightSquareBrace, 8..9, "]"),
            (Token::Period, 9..10, "."),
            (Token::Ident("bar".to_string()), 10..13, "bar"),
            (Token::If, 14..16, "if"),
            (Token::Ident("bar".to_string()), 17..20, "bar"),
            (Token::Else, 21..25, "else"),
            (Token::Ident("baz".to_string()), 26..29, "baz"),
            (Token::And, 30..32, "&&"),
            (Token::Ident("baz".to_string()), 33..36, "baz"),
            (Token::Equal, 37..39, "=="),
            (Token::Bool(true), 40..44, "true"),
            (Token::Or, 45..47, "||"),
            (Token::LeftBracket, 48..49, "("),
            (Token::Bool(true), 49..53, "true"),
            (Token::And, 54..56, "&&"),
            (Token::Bool(false), 57..62, "false"),
            (Token::RightBracket, 62..63, ")"),
        ] {
            let tok = lex.next();
            assert_that!(tok).is_some();
            let tok = tok.unwrap();
            assert_that!(tok).is_ok();
            let tok = tok.unwrap();
            assert_that!(tok).is_equal_to(expected_parsed);
            assert_that!(lex.span()).is_equal_to(expected_span);
            assert_that!(lex.slice()).is_equal_to(expected_slice);
        }
    }

    #[test]
    fn lex_bad_tokens() {
        single_unsuccessful_lex("&", LexingError::BadToken("&".to_string()), 0..1, "&");
        single_unsuccessful_lex("|", LexingError::BadToken("|".to_string()), 0..1, "|");
        single_unsuccessful_lex("=", LexingError::BadToken("=".to_string()), 0..1, "=");
    }
}
