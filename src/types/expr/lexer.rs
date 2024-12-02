use std::{fmt, iter::Peekable, ops::Range};

use logos::{Lexer, Logos};

#[derive(Logos, Clone, Debug, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub(crate) enum Token {
    #[token("false", |_| false, ignore(case))]
    #[token("true", |_| true, ignore(case))]
    Bool(bool),

    #[regex(r"-?[1-9][0-9]*|0|0[xX][0-9a-fA-F]+|0[bB][01]+", priority = 5, callback = |lex| lex.slice().to_lowercase())]
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
    #[token("AND", ignore(case))]
    And,

    #[token("||")]
    #[token("OR", ignore(case))]
    Or,

    #[token("!")]
    #[token("NOT", ignore(case))]
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
    pub(crate) fn new(lexer: Lexer<'a, Token>) -> LexerPeekable {
        let peekable_iter = lexer.clone().peekable();

        Self {
            lexer,
            peekable_iter,
        }
    }

    pub(crate) fn next(&mut self) -> Option<Result<Token, ()>> {
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
                Ok(next) => match peek_next {
                    Ok(next_tok) => {
                        if *next_tok == match_token {
                            tracing::trace!("Matched");
                            self.next().unwrap();
                            return true;
                        }
                    }
                    _ => panic!("TODO: Handle {:?}", peek_next),
                },
                Err(e) => panic!("TODO: Handle: {:?}", e),
                _ => {}
            }
        }

        false
    }

    pub(crate) fn peek(&mut self) -> Option<&Result<Token, ()>> {
        self.peekable_iter.peek()
    }

    pub(crate) fn span(&self) -> Range<usize> {
        self.lexer.span()
    }
}
