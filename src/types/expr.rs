//! Expression AST

mod parser;

use bytes::Bytes;
use eyre::Result;
use serde::{Deserialize, Serialize};

use super::PatuiStep;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct P<T: Sized> {
    ptr: Box<T>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum LitKind {
    Bool(bool),
    Bytes(Bytes),
    Integer(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Lit {
    pub(crate) kind: LitKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Ident {
    pub(crate) value: String,
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
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    NotContains,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ExprKind {
    /// Literal
    Lit(Lit),
    /// Identifier
    Ident(Ident),
    /// Field
    Field(P<PatuiExpr>, Ident),
    /// Call
    Call(P<PatuiExpr>, Vec<P<PatuiExpr>>),
    /// TODO: Do we want this? Reference
    // Ref(Ref),
    /// If: expr2 if expr1 else expr3
    If(P<PatuiExpr>, P<PatuiExpr>, P<PatuiExpr>),
    /// List: [expr1, expr2, ...]
    List(Vec<P<PatuiExpr>>),
    /// Unary Operation: -expr, !expr, etc.
    UnOp(UnOp, P<PatuiExpr>),
    /// Binary Operation: expr1 + expr2, expr1 - expr2, etc.
    BinOp(BinOp, P<PatuiExpr>, P<PatuiExpr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PatuiExpr {
    raw: String,
    kind: ExprKind,
}

impl PatuiExpr {
    // Oh so naive right now, need to beef this up to be a full parser at some point but this
    // suffices for our basic use cases right now.
    fn try_from_str(value: &str) -> Result<Self> {
        let mut parser = parser::Parser::new(value);
        parser.parse()
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

    use super::*;

    #[test]
    fn lit_types() {
        for (expr_string, expected) in &[
            (
                "\"hello\"",
                ExprKind::Lit(Lit {
                    kind: LitKind::Str("hello".to_string()),
                }),
            ),
            (
                "123",
                ExprKind::Lit(Lit {
                    kind: LitKind::Integer(123),
                }),
            ),
            (
                "123.45",
                ExprKind::Lit(Lit {
                    kind: LitKind::Float(123.45),
                }),
            ),
            (
                "b\"hello\"",
                ExprKind::Lit(Lit {
                    kind: LitKind::Bytes(Bytes::from("hello")),
                }),
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap().kind).is_equal_to(expected);
        }
    }

    #[test]
    fn field_exprs() {
        for (expr_string, expected) in &[
            (
                "foo",
                ExprKind::Ident(Ident {
                    value: "foo".to_string(),
                }),
            ),
            (
                "foo.bar",
                ExprKind::Field(
                    P {
                        ptr: Box::new(PatuiExpr {
                            raw: "foo".to_string(),
                            kind: ExprKind::Ident(Ident {
                                value: "foo".to_string(),
                            }),
                        }),
                    },
                    Ident {
                        value: "bar".to_string(),
                    },
                ),
            ),
            (
                "foo.bar(\"a\", 1)",
                ExprKind::Call(
                    P {
                        ptr: Box::new(PatuiExpr {
                            raw: "foo.bar".to_string(),
                            kind: ExprKind::Field(
                                P {
                                    ptr: Box::new(PatuiExpr {
                                        raw: "foo".to_string(),
                                        kind: ExprKind::Ident(Ident {
                                            value: "foo".to_string(),
                                        }),
                                    }),
                                },
                                Ident {
                                    value: "bar".to_string(),
                                },
                            ),
                        }),
                    },
                    vec![
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
                                    kind: LitKind::Integer(1),
                                }),
                            }),
                        },
                    ],
                ),
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap().kind).is_equal_to(expected);
        }
    }
}
