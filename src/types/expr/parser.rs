use bytes::Bytes;
use eyre::{eyre, Result};
use logos::Logos;

use super::{
    ast::*,
    lexer::{LexerPeekable, Token},
};

enum ParserState {
    None,
    Parsed(PatuiExpr),
}

impl ParserState {
    fn take_expr(self) -> Result<PatuiExpr> {
        match self {
            ParserState::None => Err(eyre!("Nothing parsed successfully")),
            ParserState::Parsed(expr) => Ok(expr),
        }
    }
}

pub(crate) fn parse(input: &str) -> Result<PatuiExpr> {
    let raw = input.to_string();

    let mut lexer = LexerPeekable::new(Token::lexer(input));

    let expr = parse_expr(input, &mut lexer, vec![]);

    if lexer.peek().is_some() {
        let span = lexer.span();
        let rest = &input[span.start..];
        return Err(eyre!(
            "More tokens left to parse after parsing full expression: '{}'",
            rest,
        ));
    }

    expr
}

pub(crate) fn parse_expr(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    parse_until: Vec<Token>,
) -> Result<PatuiExpr> {
    let mut state = ParserState::None;

    let outer_start = lexer.span().start;

    while let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => return Err(eyre!("Error parsing token: {:?}", e)),
        };

        let start = lexer.span().start;
        let end = lexer.span().end;

        match token {
            Token::Integer(int) => {
                let expr = PatuiExpr {
                    raw: input[start..end].to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Integer(int),
                    }),
                };
                state = ParserState::Parsed(expr);
            }
            Token::Decimal(dec) => {
                let expr = PatuiExpr {
                    raw: input[start..end].to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Decimal(dec),
                    }),
                };
                state = ParserState::Parsed(expr);
            }
            Token::Bool(b) => {
                let expr = PatuiExpr {
                    raw: input[start..end].to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bool(b),
                    }),
                };
                state = ParserState::Parsed(expr);
            }
            Token::String(s) => {
                let expr = PatuiExpr {
                    raw: input[start..end].to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Str(s),
                    }),
                };
                state = ParserState::Parsed(expr);
            }
            Token::BytesPrefix => {
                let expr = parse_bytes(input, lexer)?;
                state = ParserState::Parsed(expr);
            }
            Token::Ident(id) => {
                let expr = parse_ident(input, lexer, id)?;
                state = ParserState::Parsed(expr);
            }
            Token::LeftSquareBrace => {
                state = ParserState::Parsed(match state {
                    ParserState::None => parse_list(input, lexer)?,
                    ParserState::Parsed(ref ident) => {
                        parse_index(input, lexer, ident, outer_start)?
                    }
                });
            }
            Token::LeftCurlyBrace => {
                let expr = parse_set_or_map(input, lexer)?;
                state = ParserState::Parsed(expr);
            }
            Token::Minus => {
                let expr = parse_expr(input, lexer, vec![])?;
                let end = lexer.span().end;
                let expr = PatuiExpr {
                    raw: input[start..end].to_string(),
                    kind: ExprKind::UnOp(
                        UnOp::Neg,
                        P {
                            ptr: Box::new(expr),
                        },
                    ),
                };
                state = ParserState::Parsed(expr);
            }
            tok => panic!("Unexpectedly reached token: {:?}", tok),
        }

        if let Some(Ok(ref peek_token)) = lexer.peek() {
            if parse_until.contains(&peek_token) {
                break;
            }
        }
    }

    state.take_expr()
}

fn parse_bytes(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<PatuiExpr> {
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

fn parse_bytes_list(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<Bytes> {
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

fn parse_ident(input: &str, lexer: &mut LexerPeekable<'_>, id: String) -> Result<PatuiExpr> {
    let outer_span = lexer.span();

    let mut expr = PatuiExpr {
        raw: input[outer_span.start..outer_span.end].to_string(),
        kind: ExprKind::Ident(Ident { value: id }),
    };

    // Parse fields if necessary
    while lexer.next_if_match(Token::Period) {
        if let Some(tok) = lexer.next() {
            match tok {
                Ok(Token::Ident(id)) => {
                    let span = lexer.span();
                    expr = PatuiExpr {
                        raw: input[outer_span.start..span.end].to_string(),
                        kind: ExprKind::Field(
                            P {
                                ptr: Box::new(expr),
                            },
                            Ident { value: id },
                        ),
                    };
                }
                Ok(tok) => panic!("TODO: Handle unexpected token - {:?}", tok),
                Err(e) => panic!("TODO: Handle - {:?}", e),
            }
        }
    }

    Ok(expr)
}

fn parse_list(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<PatuiExpr> {
    let start = lexer.span().start;
    let mut end = lexer.span().end;

    let mut elements = Vec::new();

    loop {
        let expr = parse_expr(input, lexer, vec![Token::Comma, Token::RightSquareBrace])?;
        tracing::trace!("Parsed list element: {:?}", expr);
        elements.push(P {
            ptr: Box::new(expr),
        });
        if lexer.next_if_match(Token::RightSquareBrace) {
            end = lexer.span().end;
            tracing::trace!("Parsed list elements: {:?}", elements);
            break;
        } else if !lexer.next_if_match(Token::Comma) {
            return Err(eyre!("Couldn't parse list from string"));
        }
    }

    tracing::trace!("Peek after list: {:?}", lexer.peek());

    Ok(PatuiExpr {
        raw: input[start..end].to_string(),
        kind: ExprKind::List(elements),
    })
}

fn parse_index(
    input: &str,
    lexer: &mut LexerPeekable<'_>,
    ident: &PatuiExpr,
    start: usize,
) -> Result<PatuiExpr> {
    let expr = parse_expr(input, lexer, vec![Token::RightSquareBrace])?;

    if !lexer.next_if_match(Token::RightSquareBrace) {
        return Err(eyre!("Couldn't parse list from string"));
    }

    let end = lexer.span().end;

    Ok(PatuiExpr {
        raw: input[start..end].to_string(),
        kind: ExprKind::Index(
            P {
                ptr: Box::new(ident.clone()),
            },
            P {
                ptr: Box::new(expr),
            },
        ),
    })
}

fn parse_set_or_map(input: &str, lexer: &mut LexerPeekable<'_>) -> Result<PatuiExpr> {
    let start = lexer.span().start;
    let mut end = lexer.span().end;

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
            end = lexer.span().end;
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
        Ok(PatuiExpr {
            raw: input[start..end].to_string(),
            kind: ExprKind::Set(
                set_elements
                    .into_iter()
                    .map(|x| P { ptr: Box::new(x) })
                    .collect::<Vec<_>>(),
            ),
        })
    } else {
        Ok(PatuiExpr {
            raw: input[start..end].to_string(),
            kind: ExprKind::Map(map_elements),
        })
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
