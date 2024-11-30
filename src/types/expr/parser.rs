use bytes::{Bytes, BytesMut};
use eyre::{eyre, Result};
use logos::{Lexer, Logos};

use super::ast::*;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub(crate) enum Token {
    #[token("false", |_| false, ignore(case))]
    #[token("true", |_| true, ignore(case))]
    Bool(bool),

    #[regex(r"-?[1-9][0-9]*|0[xX][0-9a-fA-F]+|0[bB][01]+", priority = 5, callback = |lex| lex.slice().to_lowercase())]
    Integer(String),

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", priority = 4, callback = |lex| lex.slice().to_string())]
    Decimal(String),

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice()[1..lex.slice().len() - 1].to_string())]
    #[regex(r#"'([^'\\]|\\['\\bnfrt]|u[a-fA-F0-9]{4})*'"#, |lex| lex.slice()[1..lex.slice().len() - 1].to_string())]
    String(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 1, callback = |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r"[0-9]+[a-zA-Z_]+", priority = 10, callback = |_| None)]
    BadIdentifier,

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

    #[token("&&")]
    And,

    #[token("||")]
    Or,

    #[token("!")]
    Not,

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

#[derive(Debug, PartialEq, Eq)]
enum ParserState {
    Standard,
    BytesPrefix,
}

pub(crate) fn parse(input: &str) -> Result<PatuiExpr> {
    let raw = input.to_string();

    let mut lexer = Token::lexer(input);

    parse_expr(input, &mut lexer)
}

pub(crate) fn parse_expr(input: &str, lexer: &mut Lexer<Token>) -> Result<PatuiExpr> {
    let mut expr: Option<PatuiExpr> = None;

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::Integer(int)) => {
                expr = Some(PatuiExpr {
                    raw: input.to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Integer(int),
                    }),
                });
            }
            Ok(Token::Decimal(dec)) => {
                expr = Some(PatuiExpr {
                    raw: input.to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Decimal(dec),
                    }),
                });
            }
            Ok(Token::Bool(b)) => {
                expr = Some(PatuiExpr {
                    raw: input.to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bool(b),
                    }),
                });
            }
            Ok(Token::String(s)) => {
                expr = Some(PatuiExpr {
                    raw: input.to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Str(s),
                    }),
                });
            }
            Ok(Token::BytesPrefix) => {
                expr = Some(parse_bytes(input, lexer)?);
            }
            Ok(tok) => panic!("Unexpectedly reached token: {:?}", tok),
            Err(e) => return Err(eyre!("Error parsing token: {:?}", e)),
        }
    }

    expr.ok_or_else(|| eyre!("No expression found"))
}

fn parse_bytes(input: &str, lexer: &mut Lexer<Token>) -> Result<PatuiExpr> {
    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::String(s)) => {
                return Ok(PatuiExpr {
                    raw: input.to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from(s)),
                    }),
                });
            }
            Ok(Token::LeftSquareBrace) => {
                let bytes = parse_bytes_list(input, lexer)?;
                return Ok(PatuiExpr {
                    raw: input.to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(bytes),
                    }),
                });
            }
            Ok(tok) => panic!("Unexpectedly reached token: {:?}", tok),
            Err(e) => return Err(eyre!("Error parsing token: {:?}", e)),
        }
    }

    Err(eyre!("Error, ran out of tokens while parsing bytes",))
}

fn parse_bytes_list(input: &str, lexer: &mut Lexer<Token>) -> Result<Bytes> {
    let mut bytes = Vec::new();

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::Integer(int)) => {
                let byte = int.parse::<u8>()?;
                bytes.push(byte);
            }
            Ok(Token::String(s)) => {
                if s.len() != 1 {
                    return Err(eyre!(
                        "Error, string in bytes list must be a single character"
                    ));
                }
                let byte = s.chars().next().unwrap() as u8;
                bytes.push(byte);
            }
            Ok(Token::Comma) => {}
            Ok(Token::RightSquareBrace) => return Ok(Bytes::from(bytes)),
            Ok(tok) => panic!("Unexpectedly reached token: {:?}", tok),
            Err(e) => return Err(eyre!("Error parsing token: {:?}", e)),
        }
    }

    Err(eyre!("Error, ran out of tokens while parsing bytes list",))
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
        let mut lex = Token::lexer("123az");
        let tok = lex.next();
        assert_that!(tok).is_some();
        let tok = tok.unwrap();
        assert_that!(tok).is_err();
    }

    #[test]
    fn lex_bool() {
        single_successful_lex("true", Token::Bool(true), 0..4, "true");
        single_successful_lex("FaLse", Token::Bool(false), 0..5, "FaLse");
    }

    #[test]
    fn lex_string() {
        single_successful_lex(
            r#""foo bar boo""#,
            Token::String("foo bar boo".to_string()),
            0..13,
            r#""foo bar boo""#,
        );
        single_successful_lex(
            "\"foo\nbar\nboo\"",
            Token::String("foo\nbar\nboo".to_string()),
            0..13,
            "\"foo\nbar\nboo\"",
        );
        single_successful_lex(
            r#""foo\"bar\"boo""#,
            Token::String("foo\\\"bar\\\"boo".to_string()),
            0..15,
            r#""foo\"bar\"boo""#,
        );
    }

    #[test]
    fn lex_string_errors() {
        let mut lex = Token::lexer("\"foo bar boo");
        let tok = lex.next();
        assert_that!(tok).is_some();
        let tok = tok.unwrap();
        assert_that!(tok).is_err();
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
}
