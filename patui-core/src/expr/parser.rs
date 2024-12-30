//! The parser for Patui expressions, takes a string and converts it into an abstract syntax tree
//! (AST), or specifically an `Expr` that can then be used in the `PatuiExpr`.

use bytes::Bytes;
use logos::{Logos, Span};
use rug::{float::ParseFloatError, integer::ParseIntegerError, Complete, Float, Integer};
use thiserror::Error;

use super::{
    ast::*,
    lexer::{LexerPeekable, LexingError, Token},
};

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum ExprParseError {
    #[error("Unexpected tokens remaining after parsing: {0:?}")]
    TokensRemaining(Span, Vec<Token>),
    #[error("Parsing unexpectedly finished when more tokens were expected {0}")]
    UnexpectedEnd(String),
    #[error("Error parsing token: {0}")]
    TokenParseError(#[from] LexingError),
    #[error("Unexpectedly found token while {0}: {1:?}")]
    UnexpectedToken(String, Token),
    #[error("No expression found")]
    NoExpression,
    #[error("Integer must by 8-bit unsigned integer in bytes list")]
    ByteListBadIntCharacter,
    #[error("String must by single character in bytes list")]
    ByteListStringBadCharacter,
    #[error("Error parsing integer: {0}")]
    IntegerParseError(#[from] ParseIntegerError),
    #[error("Error parsing float: {0}")]
    FloatParseError(#[from] ParseFloatError),
    #[error("Parsed both set and map elements, must be either a set or a map")]
    ParsedSetAndMap,
    #[error("Parsed map element with non-string key")]
    ParsedMapNonStringKey,
    #[error("Expected left hand side of binary operation '{0}'")]
    ExpectedLhsForBinOp(BinOp),
}

pub(crate) fn parse(input: &str) -> Result<Expr, ExprParseError> {
    tracing::trace!("Parsing input: '{}'", input);

    let mut lexer = LexerPeekable::new(Token::lexer(input));

    let expr = parse_expr(input, &mut lexer, vec![])?;

    if lexer.peek().is_some() {
        let span = lexer.span();
        let rest = lexer.rest().unwrap_or_default();
        return Err(ExprParseError::TokensRemaining(span, rest));
    }

    Ok(expr)
}

fn parse_expr(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    parse_until: Vec<Token>,
) -> Result<Expr, ExprParseError> {
    let mut expr = None;

    while let Some(token) = lexer.next() {
        let token = token?;

        tracing::trace!("Token: {:?}", token);
        tracing::trace!("Peek token: {:?}", lexer.peek());

        match token {
            Token::Null
            | Token::Integer(_)
            | Token::Decimal(_)
            | Token::Bool(_)
            | Token::String(_)
            | Token::BytesPrefix
            | Token::Ident(_)
            | Token::LeftSquareBrace
            | Token::LeftCurlyBrace => {
                expr = Some(parse_term(input, lexer, token)?);
            }
            Token::LeftBracket => {
                expr = Some(parse_bracket_ordering(input, lexer)?);
            }
            Token::Minus => {
                expr = match expr.take() {
                    None => Some(parse_un_op(input, lexer, UnOp::Neg, parse_until.clone())?),
                    Some(lhs) => Some(parse_bin_op(
                        input,
                        lexer,
                        Some(lhs),
                        BinOp::Subtract,
                        parse_until.clone(),
                    )?),
                };
            }
            Token::Not => {
                expr = Some(parse_un_op(input, lexer, UnOp::Not, parse_until.clone())?);
            }
            Token::Equal => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::Equal,
                    parse_until.clone(),
                )?);
            }
            Token::NotEqual => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::NotEqual,
                    parse_until.clone(),
                )?);
            }
            Token::LessThan => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::LessThan,
                    parse_until.clone(),
                )?);
            }
            Token::LessThanEqual => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::LessThanEqual,
                    parse_until.clone(),
                )?);
            }
            Token::GreaterThan => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::GreaterThan,
                    parse_until.clone(),
                )?);
            }
            Token::GreaterThanEqual => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::GreaterThanEqual,
                    parse_until.clone(),
                )?);
            }
            Token::And => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::And,
                    parse_until.clone(),
                )?);
            }
            Token::Or => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::Or,
                    parse_until.clone(),
                )?);
            }
            Token::Add => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::Add,
                    parse_until.clone(),
                )?);
            }
            Token::Star => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::Multiply,
                    parse_until.clone(),
                )?);
            }
            Token::Slash => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::Divide,
                    parse_until.clone(),
                )?);
            }
            Token::Percent => {
                expr = Some(parse_bin_op(
                    input,
                    lexer,
                    expr,
                    BinOp::Modulo,
                    parse_until.clone(),
                )?);
            }
            tok => {
                return Err(ExprParseError::UnexpectedToken(
                    "parsing general expression".to_string(),
                    tok,
                ))
            }
        }

        if let Some(Ok(ref peek_token)) = lexer.peek() {
            tracing::trace!("Peek token: {:?}", peek_token);
            tracing::trace!("parse_until: {:?}", parse_until);
            if parse_until.contains(peek_token) {
                break;
            }
        }
    }

    expr.ok_or(ExprParseError::NoExpression)
}

fn parse_term(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    token: Token,
) -> Result<Expr, ExprParseError> {
    tracing::trace!("Parsing term from {}", input);

    let mut ident_parts = vec![];

    match token {
        Token::Null => ident_parts.push(TermPart::Lit(Lit::Null)),
        Token::Bool(b) => ident_parts.push(TermPart::Lit(Lit::Bool(b))),
        Token::Integer(ref int) => {
            let int = parse_integer(int)?;
            ident_parts.push(TermPart::Lit(Lit::Integer(int)))
        }
        Token::Decimal(ref dec) => {
            let dec = Float::with_val(53, Float::parse(dec)?);
            ident_parts.push(TermPart::Lit(Lit::Decimal(dec)))
        }
        Token::String(ref s) => ident_parts.push(TermPart::Lit(Lit::String(s.clone()))),
        Token::BytesPrefix => {
            ident_parts.push(TermPart::Lit(parse_bytes(lexer)?));
        }
        Token::LeftSquareBrace => {
            ident_parts.push(TermPart::Lit(parse_list(input, lexer)?));
        }
        Token::LeftCurlyBrace => {
            ident_parts.push(TermPart::Lit(parse_set_or_map(input, lexer)?));
        }
        Token::Ident(ref id) => ident_parts.push(TermPart::Ident(id.clone())),
        _ => {
            return Err(ExprParseError::UnexpectedToken(
                "parsing term".to_string(),
                token,
            ))
        }
    }

    loop {
        if lexer.next_if_match(Token::LeftSquareBrace) {
            match lexer.peek() {
                Some(Ok(Token::Integer(_))) => {
                    let Some(Ok(Token::Integer(int))) = lexer.next() else {
                        unreachable!();
                    };
                    let int = parse_integer(&int)?;
                    if lexer.next_if_match(Token::Range) {
                        let start = Expr::Term(vec![TermPart::Lit(Lit::Integer(int))]);
                        let end = match lexer.peek() {
                            Some(Ok(Token::Integer(_))) => {
                                let Some(Ok(Token::Integer(int))) = lexer.next() else {
                                    unreachable!();
                                };
                                let int = parse_integer(&int)?;
                                Expr::Term(vec![TermPart::Lit(Lit::Integer(int))])
                            }
                            Some(Ok(tok)) => {
                                return Err(ExprParseError::UnexpectedToken(
                                    "parsing term index".to_string(),
                                    tok.clone(),
                                ))
                            }
                            Some(Err(e)) => return Err(e.clone().into()),
                            None => {
                                return Err(ExprParseError::UnexpectedEnd(
                                    "parsing term index".to_string(),
                                ))
                            }
                        };
                        let range = Expr::Term(vec![TermPart::Lit(Lit::Range(
                            Box::new(start),
                            Box::new(end),
                        ))]);
                        ident_parts.push(TermPart::Index(Box::new(range)));
                    } else {
                        ident_parts.push(TermPart::Index(Box::new(Expr::Term(vec![
                            TermPart::Lit(Lit::Integer(int)),
                        ]))));
                    }
                }
                Some(Ok(Token::String(_))) => {
                    let Some(Ok(Token::String(s))) = lexer.next() else {
                        unreachable!();
                    };
                    ident_parts.push(TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(
                        Lit::String(s),
                    )]))));
                }
                Some(Ok(Token::Star)) => {
                    debug_assert!(lexer.next() == Some(Ok(Token::Star)));
                    ident_parts.push(TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(
                        Lit::Wildcard,
                    )]))));
                }
                Some(Ok(Token::Ident(_))) => {
                    let expr = parse_expr(input, lexer, vec![Token::RightSquareBrace])?;
                    ident_parts.push(TermPart::Index(Box::new(expr)));
                }
                Some(Ok(ref tok)) => {
                    return Err(ExprParseError::UnexpectedToken(
                        "parsing term index".to_string(),
                        tok.clone(),
                    ))
                }
                Some(Err(e)) => return Err(e.clone().into()),
                None => {
                    return Err(ExprParseError::UnexpectedEnd(
                        "parsing term index".to_string(),
                    ))
                }
            }
            match lexer.next() {
                Some(Ok(Token::RightSquareBrace)) => {}
                Some(Ok(ref tok)) => {
                    return Err(ExprParseError::UnexpectedToken(
                        "parsing term index".to_string(),
                        tok.clone(),
                    ))
                }
                Some(Err(e)) => return Err(e.into()),
                None => {
                    return Err(ExprParseError::UnexpectedEnd(
                        "parsing term index".to_string(),
                    ))
                }
            }
        } else if lexer.next_if_match(Token::Period) {
            let field_ident = match lexer.next() {
                Some(Ok(Token::Ident(id))) => id,
                Some(Ok(ref tok)) => {
                    return Err(ExprParseError::UnexpectedToken(
                        "parsing term ident".to_string(),
                        tok.clone(),
                    ))
                }
                Some(Err(e)) => return Err(e.into()),
                None => {
                    return Err(ExprParseError::UnexpectedEnd(
                        "parsing term ident".to_string(),
                    ))
                }
            };
            ident_parts.push(TermPart::Ident(field_ident));
        } else if lexer.next_if_match(Token::LeftBracket) {
            let function_name = match ident_parts.pop() {
                Some(TermPart::Ident(id)) => id,
                Some(term_part) => {
                    return Err(ExprParseError::UnexpectedToken(
                        format!(
                            "parsing method call with invalid method name preceeding: '{:?}'",
                            term_part
                        ),
                        Token::LeftBracket,
                    ))
                }
                None => {
                    return Err(ExprParseError::UnexpectedToken(
                        "parsing method call without preceding method name".to_string(),
                        Token::LeftBracket,
                    ))
                }
            };

            let mut args = Vec::new();

            while lexer.peek().is_some() {
                if lexer.next_if_match(Token::RightBracket) {
                    break;
                }
                let arg = parse_expr(input, lexer, vec![Token::Comma, Token::RightBracket])?;
                args.push(arg);
                lexer.next_if_match(Token::Comma);
            }

            ident_parts.push(TermPart::Call(function_name, args));
        } else {
            break;
        }
    }

    Ok(Expr::Term(ident_parts))
}

fn parse_integer(integer: &str) -> Result<Integer, ExprParseError> {
    tracing::trace!("Parsing integer: {}", integer);
    if let Some(integer) = integer.strip_prefix("0x") {
        Ok(Integer::parse_radix(integer, 16)?.complete())
    } else {
        Ok(Integer::parse(integer)?.complete())
    }
}

fn parse_bytes(lexer: &mut LexerPeekable<'_>) -> Result<Lit, ExprParseError> {
    if let Some(token) = lexer.next() {
        match token {
            Ok(Token::String(s)) => {
                return Ok(Lit::Bytes(Bytes::from(s)));
            }
            Ok(Token::LeftSquareBrace) => {
                let bytes = parse_bytes_list(lexer)?;
                return Ok(Lit::Bytes(bytes));
            }
            Ok(tok) => {
                return Err(ExprParseError::UnexpectedToken(
                    "parsing bytes".to_string(),
                    tok,
                ))
            }
            Err(e) => panic!("Error parsing token: {:?}", e),
        }
    }

    Err(ExprParseError::UnexpectedEnd("parsing bytes".to_string()))
}

fn parse_bytes_list(lexer: &mut LexerPeekable<'_>) -> Result<Bytes, ExprParseError> {
    let mut bytes = Vec::new();

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::Integer(int)) => {
                let int = parse_integer(&int)?;
                if let Some(int) = int.to_u8() {
                    bytes.push(int);
                } else {
                    return Err(ExprParseError::ByteListBadIntCharacter);
                }
            }
            Ok(Token::String(s)) => {
                if s.len() != 1 {
                    return Err(ExprParseError::ByteListStringBadCharacter);
                }
                // TODO: Bad UTF-8 character error
                let byte = s.chars().next().unwrap() as u8;
                bytes.push(byte);
            }
            Ok(Token::Comma) => {}
            Ok(Token::RightSquareBrace) => return Ok(Bytes::from(bytes)),
            Ok(tok) => {
                return Err(ExprParseError::UnexpectedToken(
                    "parsing bytes list".to_string(),
                    tok,
                ))
            }
            Err(e) => panic!("Error parsing token: {:?}", e),
        }
    }

    Err(ExprParseError::UnexpectedEnd(
        "parsing bytes list".to_string(),
    ))
}

fn parse_list(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<Lit, ExprParseError> {
    let mut elements = Vec::new();

    loop {
        let expr = parse_expr(input, lexer, vec![Token::Comma, Token::RightSquareBrace])?;
        tracing::trace!("Parsed list element: {:?}", expr);
        elements.push(expr);
        if lexer.next_if_match(Token::RightSquareBrace) {
            tracing::trace!("Parsed list elements: {:?}", elements);
            break;
        } else if !lexer.next_if_match(Token::Comma) {
            return Err(ExprParseError::UnexpectedEnd("parsing list".to_string()));
        }
    }

    tracing::trace!("Peek after list: {:?}", lexer.peek());

    Ok(Lit::List(elements))
}

fn parse_set_or_map(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<Lit, ExprParseError> {
    let mut map_elements = Vec::new();
    let mut set_elements = Vec::new();

    loop {
        let key = parse_expr(
            input,
            lexer,
            vec![Token::Comma, Token::Colon, Token::RightCurlyBrace],
        )?;

        if lexer.next_if_match(Token::Colon) {
            let value = parse_expr(input, lexer, vec![Token::Comma, Token::RightCurlyBrace])?;
            tracing::trace!("Parsed dict element: {:?}={:?}", key, value);
            let key = match key {
                Expr::Term(parts) => {
                    if parts.len() != 1 {
                        return Err(ExprParseError::ParsedMapNonStringKey);
                    }
                    if let TermPart::Lit(Lit::String(key)) = &parts[0] {
                        key.clone()
                    } else {
                        return Err(ExprParseError::ParsedMapNonStringKey);
                    }
                }
                _ => return Err(ExprParseError::ParsedMapNonStringKey),
            };
            map_elements.push((key, value));
        } else {
            tracing::trace!("Parsed set element: {:?}", key);
            if !set_elements.contains(&key) {
                set_elements.push(key);
            }
        }

        if lexer.next_if_match(Token::RightCurlyBrace) {
            tracing::trace!("Parsed set elements: {:?}", set_elements);
            tracing::trace!("Parsed map elements: {:?}", map_elements);
            tracing::trace!("Peek after set/map: {:?}", lexer.peek());
            break;
        } else if !lexer.next_if_match(Token::Comma) {
            return Err(ExprParseError::UnexpectedEnd(
                "parsing map or set".to_string(),
            ));
        }
    }

    if !set_elements.is_empty() && !map_elements.is_empty() {
        Err(ExprParseError::ParsedSetAndMap)
    } else if !set_elements.is_empty() {
        Ok(Lit::Set(set_elements))
    } else {
        Ok(Lit::Map(map_elements))
    }
}

fn parse_un_op(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    op: UnOp,
    parse_until: Vec<Token>,
) -> Result<Expr, ExprParseError> {
    let expr = parse_expr(input, lexer, parse_until)?;
    Ok(Expr::UnOp(op, Box::new(expr)))
}

fn parse_bin_op(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    mut lhs: Option<Expr>,
    op: BinOp,
    parse_until: Vec<Token>,
) -> Result<Expr, ExprParseError> {
    let lhs = lhs
        .take()
        .ok_or_else(|| ExprParseError::ExpectedLhsForBinOp(op.clone()))?;

    let rhs = parse_expr(input, lexer, parse_until)?;

    Ok(Expr::BinOp(op, Box::new(lhs), Box::new(rhs)))
}

fn parse_bracket_ordering(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
) -> Result<Expr, ExprParseError> {
    let expr = parse_expr(input, lexer, vec![Token::RightBracket])?;
    if !lexer.next_if_match(Token::RightBracket) {
        return Err(ExprParseError::UnexpectedEnd(
            "parsing brackets".to_string(),
        ));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use bytes::Bytes;
    use rug::Float;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[test]
    fn lits() {
        for (expr_string, expected) in &[
            (
                "123",
                Expr::Term(vec![TermPart::Lit(Lit::Integer(123.into()))]),
            ),
            (
                "123.45",
                Expr::Term(vec![TermPart::Lit(Lit::Decimal(Float::with_val(
                    53, 123.45,
                )))]),
            ),
            ("true", Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
            ("false", Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
            (
                "\"hello\"",
                Expr::Term(vec![TermPart::Lit(Lit::String("hello".to_string()))]),
            ),
            (
                "b\"hello\"",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("hello")))]),
            ),
            (
                "b[]",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("")))]),
            ),
            (
                "b[104]",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("h")))]),
            ),
            (
                "b[0x6c]",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("l")))]),
            ),
            (
                "b[0x6C]",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("l")))]),
            ),
            (
                "b['o']",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("o")))]),
            ),
            (
                "b[\"O\"]",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("O")))]),
            ),
            (
                r#"b["h", "e", 108, 108, 'o']"#,
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("hello")))]),
            ),
            (
                "b[104, 0x65, 0x6c, 0x6C, 'o']",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("hello")))]),
            ),
            (
                "b[104, 0x65, 0x6c, 0x6C, 'o',]",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("hello")))]),
            ),
            (
                "b[      104    , 0x65      , 0x6c  , 0x6C   , 'o'  , ]",
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("hello")))]),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn extended_lits() {
        for (expr_string, expected) in &[
            (
                "[123, 456, 789]",
                Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(123.into()))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(456.into()))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(789.into()))]),
                ]))]),
            ),
            (
                "{\"a\": 1, \"b\": [1,2,3]}",
                Expr::Term(vec![TermPart::Lit(Lit::Map(vec![
                    (
                        "a".to_string(),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                    ),
                    (
                        "b".to_string(),
                        Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(3.into()))]),
                        ]))]),
                    ),
                ]))]),
            ),
            (
                "{1, 2, \"foo\", [1,2], 2}",
                Expr::Term(vec![TermPart::Lit(Lit::Set(vec![
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                    Expr::Term(vec![TermPart::Lit(Lit::String("foo".to_string()))]),
                    Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                    ]))]),
                ]))]),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn lit_errors() {
        for (expr_string, expected) in &[
            (
                "123,",
                ExprParseError::UnexpectedToken("parsing term".to_string(), Token::Comma),
            ),
            (
                "\"abc",
                ExprParseError::UnexpectedToken("parsing term".to_string(), Token::Comma),
            ),
            (
                "\"abc\"\"",
                ExprParseError::UnexpectedToken("parsing term".to_string(), Token::Comma),
            ),
            (
                "a\"abc\"",
                ExprParseError::UnexpectedToken("parsing term".to_string(), Token::Comma),
            ),
            (
                "b\"test",
                ExprParseError::UnexpectedEnd("parsing bytes".to_string()),
            ),
            (
                "b[104, 0x65, 0x6c, 0x6C, 'o'",
                ExprParseError::UnexpectedEnd("parsing bytes list".to_string()),
            ),
            (
                "[104, 0x65, 0x6c, 0x6C, 'o'",
                ExprParseError::UnexpectedEnd("parsing list".to_string()),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_err();
            assert_that!(res.unwrap_err()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn terms() {
        for (expr_string, expected) in &[
            ("foo", Expr::Term(vec![TermPart::Ident("foo".to_string())])),
            (
                "foo.bar",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Ident("bar".to_string()),
                ]),
            ),
            (
                "foo.bar.baz.boo",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Ident("bar".to_string()),
                    TermPart::Ident("baz".to_string()),
                    TermPart::Ident("boo".to_string()),
                ]),
            ),
            (
                "foo[0]",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        0.into(),
                    ))]))),
                ]),
            ),
            (
                "foo.bar[bar.foo]",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Ident("bar".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![
                        TermPart::Ident("bar".to_string()),
                        TermPart::Ident("foo".to_string()),
                    ]))),
                ]),
            ),
            (
                "bar[*]",
                Expr::Term(vec![
                    TermPart::Ident("bar".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Wildcard)]))),
                ]),
            ),
            (
                "\"string\"[1..3]",
                Expr::Term(vec![
                    TermPart::Lit(Lit::String("string".to_string())),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Range(
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(3.into()))])),
                    ))]))),
                ]),
            ),
            (
                "b\"bytes\"[1]",
                Expr::Term(vec![
                    TermPart::Lit(Lit::Bytes(Bytes::from("bytes"))),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
            ),
            (
                "[1,2,3][1]",
                Expr::Term(vec![
                    TermPart::Lit(Lit::List(vec![
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(3.into()))]),
                    ])),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
            ),
            (
                "{\"a\":1,\"b\":2}[\"a\"]",
                Expr::Term(vec![
                    TermPart::Lit(Lit::Map(vec![
                        (
                            "a".to_string(),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        ),
                        (
                            "b".to_string(),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                        ),
                    ])),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "a".to_string(),
                    ))]))),
                ]),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn maths() {
        for (expr_string, expected) in &[
            (
                "-x",
                Expr::UnOp(
                    UnOp::Neg,
                    Box::new(Expr::Term(vec![TermPart::Ident("x".to_string())])),
                ),
            ),
            (
                "!true",
                Expr::UnOp(
                    UnOp::Not,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                ),
            ),
            (
                "1 + 2",
                Expr::BinOp(
                    BinOp::Add,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 - 2",
                Expr::BinOp(
                    BinOp::Subtract,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 * 2",
                Expr::BinOp(
                    BinOp::Multiply,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 / 2",
                Expr::BinOp(
                    BinOp::Divide,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 % 2",
                Expr::BinOp(
                    BinOp::Modulo,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn comparison() {
        for (expr_string, expected) in &[
            (
                "1 == 2",
                Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 != 2",
                Expr::BinOp(
                    BinOp::NotEqual,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 < 2",
                Expr::BinOp(
                    BinOp::LessThan,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 <= 2",
                Expr::BinOp(
                    BinOp::LessThanEqual,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 > 2",
                Expr::BinOp(
                    BinOp::GreaterThan,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
            (
                "1 >= 2",
                Expr::BinOp(
                    BinOp::GreaterThanEqual,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                ),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn boolean_logic() {
        for (expr_string, expected) in &[
            (
                "true && false",
                Expr::BinOp(
                    BinOp::And,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                ),
            ),
            (
                "true || false",
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                ),
            ),
            (
                "true && false || 1 == 2",
                Expr::BinOp(
                    BinOp::And,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                        Box::new(Expr::BinOp(
                            BinOp::Equal,
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                        )),
                    )),
                ),
            ),
            (
                "!true",
                Expr::UnOp(
                    UnOp::Not,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                ),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn functions() {
        for (expr_string, expected) in &[
            (
                "foo.bar()",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Call("bar".to_string(), vec![]),
                ]),
            ),
            (
                "foo.bar(\"a\", 1)",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Call(
                        "bar".to_string(),
                        vec![
                            Expr::Term(vec![TermPart::Lit(Lit::String("a".to_string()))]),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        ],
                    ),
                ]),
            ),
            (
                "foo.bar(foo.baz(), foo.boo())",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Call(
                        "bar".to_string(),
                        vec![
                            Expr::Term(vec![
                                TermPart::Ident("foo".to_string()),
                                TermPart::Call("baz".to_string(), vec![]),
                            ]),
                            Expr::Term(vec![
                                TermPart::Ident("foo".to_string()),
                                TermPart::Call("boo".to_string(), vec![]),
                            ]),
                        ],
                    ),
                ]),
            ),
            (
                "foo.bar(1  ,   2   ,  bar.baz( 3, 4, 5)  )",
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Call(
                        "bar".to_string(),
                        vec![
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                            Expr::Term(vec![
                                TermPart::Ident("bar".to_string()),
                                TermPart::Call(
                                    "baz".to_string(),
                                    vec![
                                        Expr::Term(vec![TermPart::Lit(Lit::Integer(3.into()))]),
                                        Expr::Term(vec![TermPart::Lit(Lit::Integer(4.into()))]),
                                        Expr::Term(vec![TermPart::Lit(Lit::Integer(5.into()))]),
                                    ],
                                ),
                            ]),
                        ],
                    ),
                ]),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn brackets() {
        for (expr_string, expected) in &[
            (
                "(true && false) || true",
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::BinOp(
                        BinOp::And,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                    )),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                ),
            ),
            (
                "true && (false || true)",
                Expr::BinOp(
                    BinOp::And,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    )),
                ),
            ),
        ] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn complex() {
        for (expr_string, expected) in &[(
            "((foo.bar[2].baz(1, 2, 3) + 5) == 123) && foobar[\"abc\"]",
            Expr::BinOp(
                BinOp::And,
                Box::new(Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::BinOp(
                        BinOp::Add,
                        Box::new(Expr::Term(vec![
                            TermPart::Ident("foo".to_string()),
                            TermPart::Ident("bar".to_string()),
                            TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                                2.into()
                            ))]))),
                            TermPart::Call(
                                "baz".to_string(),
                                vec![
                                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                                    Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                                    Expr::Term(vec![TermPart::Lit(Lit::Integer(3.into()))]),
                                ],
                            ),
                        ])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(5.into()))])),
                    )),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(123.into()))])),
                )),
                Box::new(Expr::Term(vec![
                    TermPart::Ident("foobar".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "abc".to_string()
                    ))]))),
                ])),
            ),
        ), (
            "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[foo.baz()]))) || (\"123\" == 123) || ([1,2,3] == {\"a\": 1}) || !true",
            Expr::BinOp(
                BinOp::And,
                Box::new(Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),),
                    Box::new(Expr::BinOp(
                        BinOp::Add,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(3.into()))])),
                    ))
                )),
                Box::new(Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                        Box::new(Expr::BinOp(
                            BinOp::Equal,
                            Box::new(Expr::Term(vec![
                                TermPart::Ident("foo".to_string()),
                                TermPart::Ident("bar".to_string()),
                                TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]))),
                            ])),
                            Box::new(Expr::Term(vec![
                                TermPart::Ident("bar".to_string()),
                                TermPart::Index(Box::new(
                                    Expr::Term(vec![
                                        TermPart::Ident("foo".to_string()),
                                        TermPart::Call("baz".to_string(), vec![]),
                                    ])
                                ))
                            ]))
                        ))
                    )),
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::BinOp(
                            BinOp::Equal,
                            Box::new(Expr::Term(vec![
                                TermPart::Lit(Lit::String("123".to_string()))
                            ])),
                            Box::new(Expr::Term(vec![
                                TermPart::Lit(Lit::Integer(123.into()))
                            ])),
                        )),
                        Box::new(Expr::BinOp(
                            BinOp::Or,
                            Box::new(Expr::BinOp(
                                BinOp::Equal,
                                Box::new(Expr::Term(vec![
                                    TermPart::Lit(Lit::List(vec![
                                        Expr::Term(vec![
                                            TermPart::Lit(Lit::Integer(1.into())),
                                        ]),
                                        Expr::Term(vec![
                                            TermPart::Lit(Lit::Integer(2.into())),
                                        ]),
                                        Expr::Term(vec![
                                            TermPart::Lit(Lit::Integer(3.into())),
                                        ]),
                                    ])),
                                ])),
                                Box::new(Expr::Term(
                                    vec![TermPart::Lit(
                                        Lit::Map(vec![
                                            (
                                                "a".to_string(),
                                                Expr::Term(vec![
                                                    TermPart::Lit(Lit::Integer(1.into()))
                                                ]),
                                            ),
                                        ])
                                    )]
                                ))
                            )),
                            Box::new(Expr::UnOp(
                                UnOp::Not,
                                Box::new(Expr::Term(vec![
                                    TermPart::Lit(Lit::Bool(true))
                                ]))
                            )),
                        ))
                    ))
                )
            ))
        )] {
            let res = parse(expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    // TODO: Precedence
}
