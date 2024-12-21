use bytes::Bytes;
use eyre::{eyre, Result};
use logos::Logos;

use super::{
    ast::*,
    lexer::{LexerPeekable, Token},
};

pub(crate) fn parse(input: &str) -> Result<PatuiExpr> {
    let raw = input.to_string();

    tracing::trace!("Parsing input: '{}'", input);

    let mut lexer = LexerPeekable::new(Token::lexer(input));

    let expr = parse_expr(input, &mut lexer, vec![])?;

    if lexer.peek().is_some() {
        let span = lexer.span();
        let rest = &input[span.start..];
        return Err(eyre!(
            "More tokens left to parse after parsing full expression: '{}'",
            rest,
        ));
    }

    Ok(PatuiExpr { raw, expr })
}

pub(crate) fn parse_expr(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    parse_until: Vec<Token>,
) -> Result<Expr> {
    let mut expr = None;
    let mut expr_start = None;

    while let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => return Err(eyre!("Error parsing token: {:?}", e)),
        };

        if expr_start.is_none() {
            expr_start = Some(lexer.span().start);
        }

        tracing::trace!("Token: {:?}", token);
        tracing::trace!("Peek token: {:?}", lexer.peek());

        match token {
            Token::Null => {
                expr = Some(Expr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Null,
                    }),
                });
            }
            Token::Integer(int) => {
                expr = Some(Expr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Integer(int),
                    }),
                });
            }
            Token::Decimal(dec) => {
                expr = Some(Expr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Decimal(dec),
                    }),
                });
            }
            Token::Bool(b) => {
                expr = Some(Expr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bool(b),
                    }),
                });
            }
            Token::String(s) => {
                expr = Some(Expr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Str(s),
                    }),
                });
            }
            Token::BytesPrefix => expr = Some(parse_bytes(lexer)?),
            Token::Ident(id) => {
                expr = Some(parse_ident(input, lexer, id)?);
            }
            Token::LeftSquareBrace => {
                expr = Some(parse_list(input, lexer)?);
            }
            Token::LeftCurlyBrace => {
                expr = Some(parse_set_or_map(input, lexer)?);
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
                if expr.is_some() {
                    expr = Some(parse_bin_op(
                        input,
                        lexer,
                        expr,
                        BinOp::Multiply,
                        parse_until.clone(),
                    )?);
                } else {
                    // * can be an index, e.g. `foo[*]`, we use a special `Token` lit type for this
                    expr = Some(Expr {
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Str("*".to_string()),
                        }),
                    });
                }
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
            tok => panic!("Unexpectedly reached token: {:?}", tok),
        }

        if let Some(Ok(ref peek_token)) = lexer.peek() {
            tracing::trace!("Peek token: {:?}", peek_token);
            tracing::trace!("parse_until: {:?}", parse_until);
            if parse_until.contains(&peek_token) {
                break;
            }
        }
    }

    expr.ok_or_else(|| eyre!("Couldn't parse expression"))
}

fn parse_bytes(lexer: &mut LexerPeekable<'_>) -> Result<Expr> {
    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::String(s)) => {
                return Ok(Expr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from(s)),
                    }),
                });
            }
            Ok(Token::LeftSquareBrace) => {
                let bytes = parse_bytes_list(lexer)?;
                return Ok(Expr {
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

fn parse_bytes_list(lexer: &mut LexerPeekable<'_>) -> Result<Bytes> {
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

fn parse_ident(input: &str, lexer: &mut LexerPeekable<'_>, id: String) -> Result<Expr> {
    tracing::trace!("Parsing ident from {}", id);

    let mut ident_parts = vec![TermParts::Ident(id)];

    loop {
        if lexer.next_if_match(Token::LeftSquareBrace) {
            match lexer.peek() {
                Some(Ok(Token::Integer(_))) => {
                    let Some(Ok(Token::Integer(int))) = lexer.next() else {
                        unreachable!();
                    };
                    let int = int.parse::<usize>()?;
                    ident_parts.push(TermParts::Index(int));
                }
                Some(Ok(Token::String(_))) => {
                    let Some(Ok(Token::String(s))) = lexer.next() else {
                        unreachable!();
                    };
                    ident_parts.push(TermParts::Ident(s));
                }
                Some(Ok(Token::Star)) => {
                    debug_assert!(lexer.next() == Some(Ok(Token::Star)));
                    ident_parts.push(TermParts::Wildcard);
                }
                Some(Ok(Token::Ident(_))) => {
                    let expr = parse_expr(input, lexer, vec![Token::RightSquareBrace])?;
                    ident_parts.push(TermParts::Expr(P {
                        ptr: Box::new(expr),
                    }));
                }
                Some(Ok(tok)) => panic!("Unexpected token: {:?}", tok),
                Some(Err(e)) => return Err(eyre!("Error parsing token: {:?}", e)),
                None => return Err(eyre!("Ran out of tokens while parsing index")),
            }
            match lexer.next() {
                Some(Ok(Token::RightSquareBrace)) => {}
                Some(Ok(tok)) => return Err(eyre!("Unexpected token: {:?}", tok)),
                Some(Err(e)) => return Err(eyre!("Error parsing token: {:?}", e)),
                None => return Err(eyre!("Ran out of tokens while parsing index")),
            }
        } else if lexer.next_if_match(Token::Period) {
            let field_ident = match lexer.next() {
                Some(Ok(Token::Ident(id))) => id,
                Some(Ok(tok)) => return Err(eyre!("Unexpected token: {:?}", tok)),
                Some(Err(e)) => return Err(eyre!("Error parsing token: {:?}", e)),
                None => return Err(eyre!("Ran out of tokens while parsing field")),
            };
            ident_parts.push(TermParts::Ident(field_ident));
        } else if lexer.next_if_match(Token::LeftBracket) {
            let mut args = Vec::new();

            while lexer.peek().is_some() {
                if lexer.next_if_match(Token::RightBracket) {
                    break;
                }
                let arg = parse_expr(input, lexer, vec![Token::Comma, Token::RightBracket])?;
                args.push(P { ptr: Box::new(arg) });
                lexer.next_if_match(Token::Comma);
            }

            ident_parts.push(TermParts::Call(args));
        } else {
            break;
        }
    }

    let ident = Term {
        values: ident_parts,
    };

    Ok(Expr {
        kind: ExprKind::Term(ident),
    })
}

fn parse_list(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<Expr> {
    let mut elements = Vec::new();

    loop {
        let expr = parse_expr(input, lexer, vec![Token::Comma, Token::RightSquareBrace])?;
        tracing::trace!("Parsed list element: {:?}", expr);
        elements.push(P {
            ptr: Box::new(expr),
        });
        if lexer.next_if_match(Token::RightSquareBrace) {
            tracing::trace!("Parsed list elements: {:?}", elements);
            break;
        } else if !lexer.next_if_match(Token::Comma) {
            return Err(eyre!("Couldn't parse list from string"));
        }
    }

    tracing::trace!("Peek after list: {:?}", lexer.peek());

    Ok(Expr {
        kind: ExprKind::Lit(Lit {
            kind: LitKind::List(elements),
        }),
    })
}

fn parse_set_or_map(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<Expr> {
    let start = lexer.span().start;

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
            map_elements.push(P {
                ptr: Box::new((key, value)),
            });
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
            return Err(eyre!(
                "Couldn't parse map or set from string: {}",
                &input[start..]
            ));
        }
    }

    if set_elements.len() != 0 && map_elements.len() != 0 {
        Err(eyre!(
            "Parsed set and map elements, must be one or the other"
        ))
    } else if set_elements.len() != 0 {
        Ok(Expr {
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Set(
                    set_elements
                        .into_iter()
                        .map(|x| P { ptr: Box::new(x) })
                        .collect::<Vec<_>>(),
                ),
            }),
        })
    } else {
        Ok(Expr {
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Map(map_elements),
            }),
        })
    }
}

fn parse_un_op(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    op: UnOp,
    parse_until: Vec<Token>,
) -> Result<Expr> {
    let expr = parse_expr(input, lexer, parse_until)?;
    Ok(Expr {
        kind: ExprKind::UnOp(
            op,
            P {
                ptr: Box::new(expr),
            },
        ),
    })
}

fn parse_bin_op(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    mut lhs: Option<Expr>,
    op: BinOp,
    parse_until: Vec<Token>,
) -> Result<Expr> {
    let lhs = lhs
        .take()
        .ok_or_else(|| eyre!("Expected left hand side of binary operation"))?;

    let rhs = parse_expr(input, lexer, parse_until)?;

    Ok(Expr {
        kind: ExprKind::BinOp(op, P { ptr: Box::new(lhs) }, P { ptr: Box::new(rhs) }),
    })
}

fn parse_bracket_ordering(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<Expr> {
    let expr = parse_expr(input, lexer, vec![Token::RightBracket])?;
    if !lexer.next_if_match(Token::RightBracket) {
        return Err(eyre!("Couldn't parse bracket ordering from string"));
    }
    Ok(expr)
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
