//! Expression AST

use bytes::Bytes;
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
pub(crate) enum RefKind {
    Step((P<PatuiStep>, String)),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Ref {
    pub(crate) ident: String,
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
    /// Raw
    Raw(Bytes),
    /// Literal
    Lit(Lit),
    /// Reference
    Ref(Ref),
    /// expr2 if expr1 else expr3
    If(P<PatuiExpr>, P<PatuiExpr>, P<PatuiExpr>),
    /// List
    List(Vec<P<PatuiExpr>>),
    /// Unary Operation
    UnOp(UnOp, P<PatuiExpr>),
    /// Binary Operation
    BinOp(BinOp, P<PatuiExpr>, P<PatuiExpr>),
    /// Call
    Call(P<PatuiExpr>, Vec<P<PatuiExpr>>),
}

impl Default for ExprKind {
    fn default() -> Self {
        ExprKind::Raw(Bytes::default())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PatuiExpr {
    kind: ExprKind,
}

impl TryFrom<&str> for PatuiExpr {
    type Error = eyre::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(PatuiExpr {
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Str(value.to_string()),
            }),
        })
    }
}

impl TryFrom<String> for PatuiExpr {
    type Error = eyre::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(PatuiExpr {
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Str(value),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn simple_types() {
        for (from, expected) in &[
            (
                "String(hello)",
                PatuiExpr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Str("hello".to_string()),
                    }),
                },
            ),
            (
                "\"hello\"",
                PatuiExpr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Str("hello".to_string()),
                    }),
                },
            ),
            (
                "123",
                PatuiExpr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Integer(123),
                    }),
                },
            ),
            (
                "123.45",
                PatuiExpr {
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Float(123.45),
                    }),
                },
            ),
        ] {
            let res = PatuiExpr::try_from(*from);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_equal_to(expected);
        }
    }
}
