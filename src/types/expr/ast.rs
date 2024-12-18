//! Expression AST

use std::{fmt, hash::Hash, ops::Deref};

use bytes::Bytes;
use eyre::Result;
use serde::{Deserialize, Serialize};

use super::parser;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct P<T: Sized> {
    pub(crate) ptr: Box<T>,
}

impl<T> Deref for P<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum LitKind {
    Bool(bool),
    Bytes(Bytes),
    // A `String` for accuracy, otherwise we're limited to the i64 range (which we might end up
    // doing anyway, but this leaves further options open later).
    Integer(String),
    // Obviously `String` for Decimal for accuracy.
    Decimal(String),
    Str(String),
    List(Vec<P<PatuiExpr>>),
    Map(Vec<P<(PatuiExpr, PatuiExpr)>>),
    Set(Vec<P<PatuiExpr>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Lit {
    pub(crate) kind: LitKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum TermParts {
    Expr(P<PatuiExpr>),
    Ident(String),
    Index(usize),
    Wildcard,
    Range(usize, usize),
    Call(Vec<P<PatuiExpr>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Term {
    pub(crate) value: Vec<TermParts>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum RefKind {
    StepData((String, String)),
    File(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Ref {
    pub(crate) kind: RefKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum BinOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    And,
    Or,
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    Contains,
    NotContains,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ExprKind {
    /// Literal
    Lit(Lit),
    /// Abstract Term
    Term(Term),
    /// If: expr2 if expr1 else expr3
    If(P<PatuiExpr>, P<PatuiExpr>, P<PatuiExpr>),
    /// Unary Operation: -expr, !expr, etc.
    UnOp(UnOp, P<PatuiExpr>),
    /// Binary Operation: expr1 + expr2, expr1 - expr2, etc.
    BinOp(BinOp, P<PatuiExpr>, P<PatuiExpr>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PatuiExpr {
    pub(crate) raw: String,
    pub(crate) kind: ExprKind,
}

impl PatuiExpr {
    // Oh so naive right now, need to beef this up to be a full parser at some point but this
    // suffices for our basic use cases right now.
    fn try_from_str(value: &str) -> Result<Self> {
        parser::parse(value)
    }

    pub(crate) fn kind(&self) -> &ExprKind {
        &self.kind
    }
}

impl PartialEq for PatuiExpr {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for PatuiExpr {}

impl Hash for PatuiExpr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl PartialOrd for PatuiExpr {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        todo!()
    }
}

impl fmt::Display for PatuiExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl TryFrom<&str> for PatuiExpr {
    type Error = eyre::Error;

    fn try_from(value: &str) -> Result<Self> {
        PatuiExpr::try_from_str(value)
    }
}

impl TryFrom<String> for PatuiExpr {
    type Error = eyre::Error;

    fn try_from(value: String) -> Result<Self> {
        PatuiExpr::try_from_str(&value)
    }
}

impl From<PatuiExpr> for String {
    fn from(value: PatuiExpr) -> Self {
        value.raw
    }
}

impl From<&PatuiExpr> for String {
    fn from(value: &PatuiExpr) -> Self {
        value.raw.clone()
    }
}

impl<'a> From<&'a PatuiExpr> for &'a str {
    fn from(value: &'a PatuiExpr) -> Self {
        match &value.kind {
            ExprKind::Lit(Lit {
                kind: LitKind::Str(s),
            }) => s,
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[test]
    fn lits() {
        for (expr_string, expected) in &[
            (
                "123",
                PatuiExpr {
                    raw: "123".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Integer("123".to_string()),
                    }),
                },
            ),
            (
                "123.45",
                PatuiExpr {
                    raw: "123.45".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Decimal("123.45".to_string()),
                    }),
                },
            ),
            (
                "true",
                PatuiExpr {
                    raw: "true".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bool(true),
                    }),
                },
            ),
            (
                "false",
                PatuiExpr {
                    raw: "false".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bool(false),
                    }),
                },
            ),
            (
                "\"hello\"",
                PatuiExpr {
                    raw: "\"hello\"".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Str("hello".to_string()),
                    }),
                },
            ),
            (
                "b\"hello\"",
                PatuiExpr {
                    raw: "b\"hello\"".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from("hello")),
                    }),
                },
            ),
            (
                "b[]",
                PatuiExpr {
                    raw: "b[]".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from("")),
                    }),
                },
            ),
            (
                "b[104]",
                PatuiExpr {
                    raw: "b[104]".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from("h")),
                    }),
                },
            ),
            // (
            //     "b[0x6c]",
            //     PatuiExpr {
            //         raw: "b[0x6c]".to_string(),
            //         kind: ExprKind::Lit(Lit {
            //         kind: LitKind::Bytes(Bytes::from("l")),
            //     }),
            //     },
            // ),
            // (
            //     "b[0x6C]",
            //     PatuiExpr {
            //     raw: "b[0x6C]".to_string(),
            //      kind: ExprKind::Lit(Lit {
            //         kind: LitKind::Bytes(Bytes::from("l")),
            //     }),
            //     },
            // ),
            (
                "b['o']",
                PatuiExpr {
                    raw: "b['o']".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from("o")),
                    }),
                },
            ),
            (
                "b[\"O\"]",
                PatuiExpr {
                    raw: "b[\"O\"]".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from("O")),
                    }),
                },
            ),
            (
                r#"b["h", "e", 108, 108, 'o']"#,
                PatuiExpr {
                    raw: r#"b["h", "e", 108, 108, 'o']"#.to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Bytes(Bytes::from("hello")),
                    }),
                },
            ),
            // (
            //     "b[104, 0x65, 0x6c, 0x6C, 'o']",
            //     PatuiExpr {
            //     raw: "b[104, 0x65, 0x6c, 0x6C, 'o']".to_string(),
            //      kind: ExprKind::Lit(Lit {
            //         kind: LitKind::Bytes(Bytes::from("hello")),
            //     }),
            //     },
            // ),
            // (
            //     "b[104, 0x65, 0x6c, 0x6C, 'o',]",
            //     PatuiExpr {
            //     raw: //     "b[104, 0x65, 0x6c, 0x6C, 'o',]".to_string(),
            //      kind: //     ExprKind::Lit(Lit {
            //         kind: LitKind::Bytes(Bytes::from("hello")),
            //     }),
            //     },
            // ),
            // (
            //     "b[      104    , 0x65      , 0x6c  , 0x6C   , 'o'  , ]",
            //     PatuiExpr {
            //     raw: //     "b[      104    , 0x65      , 0x6c  , 0x6C   , 'o'  ,
            //     ]".to_string(),
            //      kind: //     ExprKind::Lit(Lit {
            //         kind: LitKind::Bytes(Bytes::from("hello")),
            //     }),
            //     },
            // ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
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
                PatuiExpr {
                    raw: "[123, 456, 789]".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::List(vec![
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "123".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Integer("123".to_string()),
                                    }),
                                }),
                            },
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "456".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Integer("456".to_string()),
                                    }),
                                }),
                            },
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "789".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Integer("789".to_string()),
                                    }),
                                }),
                            },
                        ]),
                    }),
                },
            ),
            (
                "{\"a\": 1, \"b\": [1,2,3]}",
                PatuiExpr {
                    raw: "{\"a\": 1, \"b\": [1,2,3]}".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Map(vec![
                            P {
                                ptr: Box::new((
                                    PatuiExpr {
                                        raw: "\"a\"".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::Str("a".to_string()),
                                        }),
                                    },
                                    PatuiExpr {
                                        raw: "1".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::Integer("1".to_string()),
                                        }),
                                    },
                                )),
                            },
                            P {
                                ptr: Box::new((
                                    PatuiExpr {
                                        raw: "\"b\"".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::Str("b".to_string()),
                                        }),
                                    },
                                    PatuiExpr {
                                        raw: "[1,2,3]".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::List(vec![
                                                P {
                                                    ptr: Box::new(PatuiExpr {
                                                        raw: "1".to_string(),
                                                        kind: ExprKind::Lit(Lit {
                                                            kind: LitKind::Integer("1".to_string()),
                                                        }),
                                                    }),
                                                },
                                                P {
                                                    ptr: Box::new(PatuiExpr {
                                                        raw: "2".to_string(),
                                                        kind: ExprKind::Lit(Lit {
                                                            kind: LitKind::Integer("2".to_string()),
                                                        }),
                                                    }),
                                                },
                                                P {
                                                    ptr: Box::new(PatuiExpr {
                                                        raw: "3".to_string(),
                                                        kind: ExprKind::Lit(Lit {
                                                            kind: LitKind::Integer("3".to_string()),
                                                        }),
                                                    }),
                                                },
                                            ]),
                                        }),
                                    },
                                )),
                            },
                        ]),
                    }),
                },
            ),
            (
                "{1, 2, \"foo\", [1,2], 2}",
                PatuiExpr {
                    raw: "{1, 2, \"foo\", [1,2], 2}".to_string(),
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Set(vec![
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "1".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Integer("1".to_string()),
                                    }),
                                }),
                            },
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "2".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Integer("2".to_string()),
                                    }),
                                }),
                            },
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "\"foo\"".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Str("foo".to_string()),
                                    }),
                                }),
                            },
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "[1,2]".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::List(vec![
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "1".to_string(),
                                                    kind: ExprKind::Lit(Lit {
                                                        kind: LitKind::Integer("1".to_string()),
                                                    }),
                                                }),
                                            },
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "2".to_string(),
                                                    kind: ExprKind::Lit(Lit {
                                                        kind: LitKind::Integer("2".to_string()),
                                                    }),
                                                }),
                                            },
                                        ]),
                                    }),
                                }),
                            },
                        ]),
                    }),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn bad_lits() {
        // TODO: Proper error reporting, errors trivially pass currently
        for (expr_string, expected_err) in &[
            ("\"test", ""),
            ("b\"test", ""),
            // ("b[104, 0x65, 0x6c, 0x6C, 'o'", ""),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_err();
            assert_that!(res.unwrap_err().to_string()).contains(*expected_err);
        }
    }

    #[traced_test]
    #[test]
    fn terms() {
        for (expr_string, expected) in &[
            (
                "foo",
                PatuiExpr {
                    raw: "foo".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![TermParts::Ident("foo".to_string())],
                    }),
                },
            ),
            (
                "foo.bar",
                PatuiExpr {
                    raw: "foo.bar".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![
                            TermParts::Ident("foo".to_string()),
                            TermParts::Ident("bar".to_string()),
                        ],
                    }),
                },
            ),
            (
                "foo.bar.baz.boo",
                PatuiExpr {
                    raw: "foo.bar.baz.boo".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![
                            TermParts::Ident("foo".to_string()),
                            TermParts::Ident("bar".to_string()),
                            TermParts::Ident("baz".to_string()),
                            TermParts::Ident("boo".to_string()),
                        ],
                    }),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn indexing() {
        for (expr_string, expected) in &[
            (
                "foo[0]",
                PatuiExpr {
                    raw: "foo[0]".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![TermParts::Ident("foo".to_string()), TermParts::Index(0)],
                    }),
                },
            ),
            (
                "foo.bar[bar.foo]",
                PatuiExpr {
                    raw: "foo.bar[bar.foo]".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![
                            TermParts::Ident("foo".to_string()),
                            TermParts::Ident("bar".to_string()),
                            TermParts::Expr(P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "".to_string(),
                                    kind: ExprKind::Term(Term {
                                        value: vec![
                                            TermParts::Ident("bar".to_string()),
                                            TermParts::Ident("foo".to_string()),
                                        ],
                                    }),
                                }),
                            }),
                        ],
                    }),
                },
            ),
            (
                "bar[*]",
                PatuiExpr {
                    raw: "bar[*]".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![
                            TermParts::Ident("bar".to_string()),
                            TermParts::Ident("*".to_string()),
                        ],
                    }),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
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
                PatuiExpr {
                    raw: "-x".to_string(),
                    kind: ExprKind::UnOp(
                        UnOp::Neg,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "x".to_string(),
                                kind: ExprKind::Term(Term {
                                    value: vec![TermParts::Ident("x".to_string())],
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "!true",
                PatuiExpr {
                    raw: "!true".to_string(),
                    kind: ExprKind::UnOp(
                        UnOp::Not,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 + 2",
                PatuiExpr {
                    raw: "1 + 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Add,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 - 2",
                PatuiExpr {
                    raw: "1 - 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Subtract,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 * 2",
                PatuiExpr {
                    raw: "1 * 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Multiply,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 / 2",
                PatuiExpr {
                    raw: "1 / 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Divide,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 % 2",
                PatuiExpr {
                    raw: "1 % 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Modulo,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
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
                PatuiExpr {
                    raw: "1 == 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Equal,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 != 2",
                PatuiExpr {
                    raw: "1 != 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::NotEqual,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 < 2",
                PatuiExpr {
                    raw: "1 < 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::LessThan,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 <= 2",
                PatuiExpr {
                    raw: "1 <= 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::LessThanEqual,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 > 2",
                PatuiExpr {
                    raw: "1 > 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::GreaterThan,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "1 >= 2",
                PatuiExpr {
                    raw: "1 >= 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::GreaterThanEqual,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "1".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("1".to_string()),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "2".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Integer("2".to_string()),
                                }),
                            }),
                        },
                    ),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
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
                PatuiExpr {
                    raw: "true && false".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::And,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "false".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(false),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "true || false",
                PatuiExpr {
                    raw: "true || false".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Or,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "false".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(false),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "true AND false OR 1 == 2",
                PatuiExpr {
                    raw: "true AND false OR 1 == 2".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::And,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "false OR 1 == 2".to_string(),
                                kind: ExprKind::BinOp(
                                    BinOp::Or,
                                    P {
                                        ptr: Box::new(PatuiExpr {
                                            raw: "false".to_string(),
                                            kind: ExprKind::Lit(Lit {
                                                kind: LitKind::Bool(false),
                                            }),
                                        }),
                                    },
                                    P {
                                        ptr: Box::new(PatuiExpr {
                                            raw: "1 == 2".to_string(),
                                            kind: ExprKind::BinOp(
                                                BinOp::Equal,
                                                P {
                                                    ptr: Box::new(PatuiExpr {
                                                        raw: "1".to_string(),
                                                        kind: ExprKind::Lit(Lit {
                                                            kind: LitKind::Integer("1".to_string()),
                                                        }),
                                                    }),
                                                },
                                                P {
                                                    ptr: Box::new(PatuiExpr {
                                                        raw: "2".to_string(),
                                                        kind: ExprKind::Lit(Lit {
                                                            kind: LitKind::Integer("2".to_string()),
                                                        }),
                                                    }),
                                                },
                                            ),
                                        }),
                                    },
                                ),
                            }),
                        },
                    ),
                },
            ),
            (
                "!true",
                PatuiExpr {
                    raw: "!true".to_string(),
                    kind: ExprKind::UnOp(
                        UnOp::Not,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "not true",
                PatuiExpr {
                    raw: "not true".to_string(),
                    kind: ExprKind::UnOp(
                        UnOp::Not,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                    ),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn functions() {
        for (expr_string, expected) in &[
            (
                "foo()",
                PatuiExpr {
                    raw: "foo()".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![TermParts::Ident("foo".to_string()), TermParts::Call(vec![])],
                    }),
                },
            ),
            (
                "foo(\"a\", 1)",
                PatuiExpr {
                    raw: "foo(\"a\", 1)".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![
                            TermParts::Ident("foo".to_string()),
                            TermParts::Call(vec![
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "\"a\"".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::Str("a".to_string()),
                                        }),
                                    }),
                                },
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "1".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::Integer("1".to_string()),
                                        }),
                                    }),
                                },
                            ]),
                        ],
                    }),
                },
            ),
            (
                "foo(bar(), baz())",
                PatuiExpr {
                    raw: "foo(bar(), baz())".to_string(),
                    kind: ExprKind::Term(Term {
                        value: vec![
                            TermParts::Ident("foo".to_string()),
                            TermParts::Call(vec![
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "bar()".to_string(),
                                        kind: ExprKind::Term(Term {
                                            value: vec![
                                                TermParts::Ident("bar".to_string()),
                                                TermParts::Call(vec![]),
                                            ],
                                        }),
                                    }),
                                },
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "baz()".to_string(),
                                        kind: ExprKind::Term(Term {
                                            value: vec![
                                                TermParts::Ident("baz".to_string()),
                                                TermParts::Call(vec![]),
                                            ],
                                        }),
                                    }),
                                },
                            ]),
                        ],
                    }),
                },
            ),
            // (
            //     "foo.bar()",
            //     PatuiExpr {
            //         raw: "foo.bar()".to_string(),
            //         kind: ExprKind::Call(
            //             P {
            //                 ptr: Box::new(PatuiExpr {
            //                     raw: "foo.bar".to_string(),
            //                     kind: ExprKind::Field(
            //                         P {
            //                             ptr: Box::new(PatuiExpr {
            //                                 raw: "foo".to_string(),
            //                                 kind: ExprKind::Ident(Ident {
            //                                     value: "foo".to_string(),
            //                                 }),
            //                             }),
            //                         },
            //                         Ident {
            //                             value: "bar".to_string(),
            //                         },
            //                     ),
            //                 }),
            //             },
            //             vec![],
            //         ),
            //     },
            // ),
            // (
            //     "foo.bar(1  ,   2   ,  bar.baz( 3, 4, 5)  )",
            //     PatuiExpr {
            //         raw: "foo.bar(1  ,   2   ,  bar.baz( 3, 4, 5)  )".to_string(),
            //         kind: ExprKind::Call(
            //             P {
            //                 ptr: Box::new(PatuiExpr {
            //                     raw: "foo.bar".to_string(),
            //                     kind: ExprKind::Field(
            //                         P {
            //                             ptr: Box::new(PatuiExpr {
            //                                 raw: "foo".to_string(),
            //                                 kind: ExprKind::Ident(Ident {
            //                                     value: "foo".to_string(),
            //                                 }),
            //                             }),
            //                         },
            //                         Ident {
            //                             value: "bar".to_string(),
            //                         },
            //                     ),
            //                 }),
            //             },
            //             vec![
            //                 P {
            //                     ptr: Box::new(PatuiExpr {
            //                         raw: "1".to_string(),
            //                         kind: ExprKind::Lit(Lit {
            //                             kind: LitKind::Integer("1".to_string()),
            //                         }),
            //                     }),
            //                 },
            //                 P {
            //                     ptr: Box::new(PatuiExpr {
            //                         raw: "2".to_string(),
            //                         kind: ExprKind::Lit(Lit {
            //                             kind: LitKind::Integer("2".to_string()),
            //                         }),
            //                     }),
            //                 },
            //                 P {
            //                     ptr: Box::new(PatuiExpr {
            //                         raw: "bar.baz( 3, 4, 5)".to_string(),
            //                         kind: ExprKind::Call(
            //                             P {
            //                                 ptr: Box::new(PatuiExpr {
            //                                     raw: "bar.baz".to_string(),
            //                                     kind: ExprKind::Field(
            //                                         P {
            //                                             ptr: Box::new(PatuiExpr {
            //                                                 raw: "bar".to_string(),
            //                                                 kind: ExprKind::Ident(Ident {
            //                                                     value: "bar".to_string(),
            //                                                 }),
            //                                             }),
            //                                         },
            //                                         Ident {
            //                                             value: "baz".to_string(),
            //                                         },
            //                                     ),
            //                                 }),
            //                             },
            //                             vec![
            //                                 P {
            //                                     ptr: Box::new(PatuiExpr {
            //                                         raw: "3".to_string(),
            //                                         kind: ExprKind::Lit(Lit {
            //                                             kind: LitKind::Integer("3".to_string()),
            //                                         }),
            //                                     }),
            //                                 },
            //                                 P {
            //                                     ptr: Box::new(PatuiExpr {
            //                                         raw: "4".to_string(),
            //                                         kind: ExprKind::Lit(Lit {
            //                                             kind: LitKind::Integer("4".to_string()),
            //                                         }),
            //                                     }),
            //                                 },
            //                                 P {
            //                                     ptr: Box::new(PatuiExpr {
            //                                         raw: "5".to_string(),
            //                                         kind: ExprKind::Lit(Lit {
            //                                             kind: LitKind::Integer("5".to_string()),
            //                                         }),
            //                                     }),
            //                                 },
            //                             ],
            //                         ),
            //                     }),
            //                 },
            //             ],
            //         ),
            //     },
            // ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
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
                PatuiExpr {
                    raw: "(true && false) || true".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::Or,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true && false".to_string(),
                                kind: ExprKind::BinOp(
                                    BinOp::And,
                                    P {
                                        ptr: Box::new(PatuiExpr {
                                            raw: "true".to_string(),
                                            kind: ExprKind::Lit(Lit {
                                                kind: LitKind::Bool(true),
                                            }),
                                        }),
                                    },
                                    P {
                                        ptr: Box::new(PatuiExpr {
                                            raw: "false".to_string(),
                                            kind: ExprKind::Lit(Lit {
                                                kind: LitKind::Bool(false),
                                            }),
                                        }),
                                    },
                                ),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                    ),
                },
            ),
            (
                "true && (false || true)",
                PatuiExpr {
                    raw: "true && (false || true)".to_string(),
                    kind: ExprKind::BinOp(
                        BinOp::And,
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "true".to_string(),
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::Bool(true),
                                }),
                            }),
                        },
                        P {
                            ptr: Box::new(PatuiExpr {
                                raw: "false || true".to_string(),
                                kind: ExprKind::BinOp(
                                    BinOp::Or,
                                    P {
                                        ptr: Box::new(PatuiExpr {
                                            raw: "false".to_string(),
                                            kind: ExprKind::Lit(Lit {
                                                kind: LitKind::Bool(false),
                                            }),
                                        }),
                                    },
                                    P {
                                        ptr: Box::new(PatuiExpr {
                                            raw: "true".to_string(),
                                            kind: ExprKind::Lit(Lit {
                                                kind: LitKind::Bool(true),
                                            }),
                                        }),
                                    },
                                ),
                            }),
                        },
                    ),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn complex() {
        for (expr_string, expected) in &[(
            "((foo.bar[2].baz(1, 2, 3) + 5) == 123) && foobar[\"abc\"]",
            PatuiExpr {
                raw: "((foo.bar[2].baz(1, 2, 3) + 5) == 123) && foobar[\"abc\"]".to_string(),
                kind: ExprKind::BinOp(
                    BinOp::And,
                    P {
                        ptr: Box::new(PatuiExpr {
                            raw: "(foo.bar[2].baz(1, 2, 3) + 5) == 123".to_string(),
                            kind: ExprKind::BinOp(
                                BinOp::Equal,
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "foo.bar[2].baz(1, 2, 3) + 5".to_string(),
                                        kind: ExprKind::BinOp(
                                            BinOp::Add,
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "foo.bar[2].baz(1, 2, 3)".to_string(),
                                                    kind: ExprKind::Term(Term {
                                                        value: vec![
                                                            TermParts::Ident("foo".to_string()),
                                                            TermParts::Ident("bar".to_string()),
                                                            TermParts::Index(2),
                                                            TermParts::Call(vec![
                                                                P {
                                                                    ptr: Box::new(PatuiExpr {
                                                                        raw: "1".to_string(),
                                                                        kind: ExprKind::Lit(Lit {
                                                                            kind: LitKind::Integer("1".to_string()),
                                                                        }),
                                                                    }),
                                                                },
                                                                P {
                                                                    ptr: Box::new(PatuiExpr {
                                                                        raw: "2".to_string(),
                                                                        kind: ExprKind::Lit(Lit {
                                                                            kind: LitKind::Integer("2".to_string()),
                                                                        }),
                                                                    }),
                                                                },
                                                                P {
                                                                    ptr: Box::new(PatuiExpr {
                                                                        raw: "3".to_string(),
                                                                        kind: ExprKind::Lit(Lit {
                                                                            kind: LitKind::Integer("3".to_string()),
                                                                        }),
                                                                    }),
                                                                },
                                                            ]),
                                                        ],
                                                    }),
                                                }),
                                            },
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "5".to_string(),
                                                    kind: ExprKind::Lit(Lit {
                                                        kind: LitKind::Integer("5".to_string()),
                                                    }),
                                                }),
                                            },
                                        ),
                                    }),
                                },
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "123".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::Integer("123".to_string()),
                                        }),
                                    }),
                                },
                            ),
                        }),
                    },
                    P {
                        ptr: Box::new(PatuiExpr {
                            raw: "foobar[\"abc\"]".to_string(),
                            kind: ExprKind::Term(Term {
                                value: vec![
                                    TermParts::Ident("foobar".to_string()),
                                    TermParts::Ident("abc".to_string()),
                                ],
                            }),
                        }),
                    },
                ),
            },
        ), (
            "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[baz()]))) || (\"123\" == 123) || ([1,2,3] == {\"a\": 1}) || !true",
            PatuiExpr {
                raw: "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[baz()]))) || (\"123\" == 123) || ([1,2,3] == {\"a\": 1}) || !true".to_string(),
                kind: ExprKind::BinOp(
                    BinOp::Or,
                    P {
                        ptr: Box::new(PatuiExpr {
                            raw: "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[baz()]))) || (\"123\" == 123) || ([1,2,3] == {\"a\": 1})".to_string(),
                            kind: ExprKind::BinOp(
                                BinOp::Or,
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[baz()]))) || (\"123\" == 123)".to_string(),
                                        kind: ExprKind::BinOp(
                                            BinOp::Or,
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[baz()])))".to_string(),
                                                    kind: ExprKind::BinOp(
                                                        BinOp::And,
                                                        P {
                                                            ptr: Box::new(PatuiExpr {
                                                                raw: "(1 == (2 + 3))".to_string(),
                                                                kind: ExprKind::BinOp(
                                                                    BinOp::Equal,
                                                                    P {
                                                                        ptr: Box::new(PatuiExpr {
                                                                            raw: "1".to_string(),
                                                                            kind: ExprKind::Lit(Lit {
                                                                                kind: LitKind::Integer("1".to_string()),
                                                                            }),
                                                                        }),
                                                                    },
                                                                    P {
                                                                        ptr: Box::new(PatuiExpr {
                                                                            raw: "(2 + 3)".to_string(),
                                                                            kind: ExprKind::BinOp(
                                                                                BinOp::Add,
                                                                                P {
                                                                                    ptr: Box::new(PatuiExpr {
                                                                                        raw: "2".to_string(),
                                                                                        kind: ExprKind::Lit(Lit {
                                                                                            kind: LitKind::Integer("2".to_string()),
                                                                                        }),
                                                                                    }),
                                                                                },
                                                                                P {
                                                                                    ptr: Box::new(PatuiExpr {
                                                                                        raw: "3".to_string(),
                                                                                        kind: ExprKind::Lit(Lit {
                                                                                            kind: LitKind::Integer("3".to_string()),
                                                                                        }),
                                                                                    }),
                                                                                },
                                                                            ),
                                                                        }),
                                                                    },
                                                                ),
                                                            }),
                                                        },
                                                        P {
                                                            ptr: Box::new(PatuiExpr {
                                                                raw: "(true || (foo.bar[1] == (bar[baz()])))".to_string(),
                                                                kind: ExprKind::BinOp(
                                                                    BinOp::Or,
                                                                    P {
                                                                        ptr: Box::new(PatuiExpr {
                                                                            raw: "true".to_string(),
                                                                            kind: ExprKind::Lit(Lit {
                                                                                kind: LitKind::Bool(true),
                                                                            }),
                                                                        }),
                                                                    },
                                                                    P {
                                                                        ptr: Box::new(PatuiExpr {
                                                                            raw: "(foo.bar[1] == (bar[baz()]))".to_string(),
                                                                            kind: ExprKind::BinOp(
                                                                                BinOp::Equal,
                                                                                P {
                                                                                    ptr: Box::new(PatuiExpr {
                                                                                        raw: "foo.bar[1]".to_string(),
                                                                                        kind: ExprKind::BinOp(
                                                                                            BinOp::Equal,
                                                                                            P {
                                                                                                ptr: Box::new(PatuiExpr {
                                                                                                    raw: "foo.bar".to_string(),
                                                                                                    kind: ExprKind::Term(Term {
                                                                                                        value: vec![
                                                                                                            TermParts::Ident("foo".to_string()),
                                                                                                            TermParts::Ident("bar".to_string()),
                                                                                                        ],
                                                                                                    }),
                                                                                                }),
                                                                                            },
                                                                                            P {
                                                                                                ptr: Box::new(PatuiExpr {
                                                                                                    raw: "1".to_string(),
                                                                                                    kind: ExprKind::Lit(Lit {
                                                                                                        kind: LitKind::Integer("1".to_string()),
                                                                                                    }),
                                                                                                }),
                                                                                            },
                                                                                        ),
                                                                                    }),
                                                                                },
                                                                                P {
                                                                                    ptr: Box::new(PatuiExpr {
                                                                                        raw: "bar[baz()]".to_string(),
                                                                                        kind: ExprKind::BinOp(
                                                                                            BinOp::Equal,
                                                                                            P {
                                                                                                ptr: Box::new(PatuiExpr {
                                                                                                    raw: "bar[baz()]".to_string(),
                                                                                                    kind: ExprKind::Term(Term {
                                                                                                        value: vec![
                                                                                                            TermParts::Ident("bar".to_string()),
                                                                                                            TermParts::Ident("baz()".to_string()),
                                                                                                        ],
                                                                                                    }),
                                                                                                }),
                                                                                            },
                                                                                            P {
                                                                                                ptr: Box::new(PatuiExpr {
                                                                                                    raw: "1".to_string(),
                                                                                                    kind: ExprKind::Lit(Lit {
                                                                                                        kind: LitKind::Integer("1".to_string()),
                                                                                                    }),
                                                                                                }),
                                                                                            },
                                                                                        ),
                                                                                    }),
                                                                                },
                                                                            ),
                                                                        }),
                                                                    }
                                                                ),
                                                            }),
                                                        }
                                                    )
                                                }),
                                            },
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "(\"123\" == 123)".to_string(),
                                                    kind: ExprKind::BinOp(
                                                        BinOp::Equal,
                                                        P {
                                                            ptr: Box::new(PatuiExpr {
                                                                raw: "\"123\"".to_string(),
                                                                kind: ExprKind::Lit(Lit {
                                                                    kind: LitKind::Str("123".to_string()),
                                                                }),
                                                            }),
                                                        },
                                                        P {
                                                            ptr: Box::new(PatuiExpr {
                                                                raw: "123".to_string(),
                                                                kind: ExprKind::Lit(Lit {
                                                                    kind: LitKind::Integer("123".to_string()),
                                                                }),
                                                            }),
                                                        },
                                                    ),
                                                }),
                                            }
                                        )
                                    }),
                                },
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "([1,2,3] == {\"a\": 1})".to_string(),
                                        kind: ExprKind::BinOp(
                                            BinOp::Equal,
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "[1,2,3]".to_string(),
                                                    kind: ExprKind::Lit(Lit {
                                                        kind: LitKind::List(vec![
                                                            P {
                                                                ptr: Box::new(PatuiExpr {
                                                                    raw: "1".to_string(),
                                                                    kind: ExprKind::Lit(Lit {
                                                                        kind: LitKind::Integer("1".to_string()),
                                                                    }),
                                                                }),
                                                            },
                                                            P {
                                                                ptr: Box::new(PatuiExpr {
                                                                    raw: "2".to_string(),
                                                                    kind: ExprKind::Lit(Lit {
                                                                        kind: LitKind::Integer("2".to_string()),
                                                                    }),
                                                                }),
                                                            },
                                                            P {
                                                                ptr: Box::new(PatuiExpr {
                                                                    raw: "3".to_string(),
                                                                    kind: ExprKind::Lit(Lit {
                                                                        kind: LitKind::Integer("3".to_string()),
                                                                    }),
                                                                }),
                                                            },
                                                        ]),
                                                    }),
                                                }),
                                            },
                                            P {
                                                ptr: Box::new(PatuiExpr {
                                                    raw: "{\"a\": 1}".to_string(),
                                                    kind: ExprKind::Lit(Lit {
                                                        kind: LitKind::Map(vec![
                                                            P {
                                                                ptr: Box::new((
                                                                    PatuiExpr {
                                                                        raw: "\"a\"".to_string(),
                                                                        kind: ExprKind::Lit(Lit {
                                                                            kind: LitKind::Str("a".to_string()),
                                                                        }),
                                                                    },
                                                                    PatuiExpr {
                                                                        raw: "1".to_string(),
                                                                        kind: ExprKind::Lit(Lit {
                                                                            kind: LitKind::Integer("1".to_string()),
                                                                        }),
                                                                    }
                                                                )),
                                                            },
                                                        ]),
                                                    }),
                                                }),
                                            },
                                        ),
                                    }),
                                },
                            ),
                        }),
                    },
                    P {
                        ptr: Box::new(PatuiExpr {
                            raw: "!true".to_string(),
                            kind: ExprKind::UnOp(
                                UnOp::Not,
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "true".to_string(),
                                        kind: ExprKind::Lit(Lit {
                                            kind: LitKind::Bool(true),
                                        }),
                                    }),
                                },
                            ),
                        }),
                    }
                )
            },
        )] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }

    // TODO: Precedence
}
