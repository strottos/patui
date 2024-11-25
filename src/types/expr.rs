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
    StepData((String, String)),
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
    /// Raw
    Raw(Bytes),
    /// Reference
    Ref(Ref),
    /// File
    File(String),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PatuiExpr {
    raw: String,
    kind: ExprKind,
}

impl TryFrom<&str> for PatuiExpr {
    type Error = eyre::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(PatuiExpr {
            raw: value.to_string(),
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
            raw: value.clone(),
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Str(value),
            }),
        })
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
                "string(hello)",
                ExprKind::Lit(Lit {
                    kind: LitKind::Str("hello".to_string()),
                }),
            ),
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
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap().kind).is_equal_to(expected);
        }
    }

    #[test]
    fn raw_bytes() {
        for (expr_string, expected) in &[
            ("bytes(\"hello\")", ExprKind::Raw(Bytes::from("hello"))),
            ("bytes(61,63,68,6c)", ExprKind::Raw(Bytes::from("achl"))),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap().kind).is_equal_to(expected);
        }
    }

    #[test]
    fn refs() {
        for (expr_string, expected) in &[
            (
                "ref(stepdata, \"step1\", \"out\")",
                ExprKind::Ref(Ref {
                    kind: RefKind::StepData(("step1".to_string(), "out".to_string())),
                }),
            ),
            (
                "stepdata(\"step1\", \"out\")",
                ExprKind::Ref(Ref {
                    kind: RefKind::StepData(("step1".to_string(), "out".to_string())),
                }),
            ),
        ] {
            let res = PatuiExpr::try_from(*expr_string);
            assert_that!(res).is_ok();
            assert_that!(res.unwrap().kind).is_equal_to(expected);
        }
    }
}
