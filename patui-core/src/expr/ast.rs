//! Expression AST
//!
//! This module contains the abstract syntax tree (AST) for expressions in Patui. This is the entry
//! point for the AST. Any time someone writes an expression we convert their string into the
//! defined AST in this file.

use std::fmt::Display;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::parser::parse;

/// Error type for PatuiExpr
#[derive(Error, Debug)]
pub enum PatuiExprError {
    /// Error parsing expression
    #[error("Error parsing expression: {0}")]
    ExprParseError(#[from] super::parser::ExprParseError),
}

/// PatuiExpr is the type used for expressions in Patui. This is the entry point for the AST for
/// expressions in Patui. They are mostly built from strings.
///
/// See documentation TODO: Link.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PatuiExpr {
    raw: String,
    expr: Expr,
}

impl PatuiExpr {
    fn try_from_str(value: &str) -> Result<Self, PatuiExprError> {
        Ok(PatuiExpr {
            raw: value.to_string(),
            expr: parse(value)?,
        })
    }
}

impl TryFrom<&str> for PatuiExpr {
    type Error = PatuiExprError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        PatuiExpr::try_from_str(value)
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lit {
    Null,
    Bool(bool),
    Bytes(Bytes),
    Integer(rug::Integer),
    Decimal(rug::Float),
    String(String),
    List(Vec<Expr>),
    Map(Vec<(String, Expr)>),
    Set(Vec<Expr>),
    Wildcard,
    Range(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TermPart {
    Ident(String),
    Index(Box<Expr>),
    Lit(Lit),
    /// Function Call: (function_name, args)
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinOp {
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

impl Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            BinOp::Add => "+",
            BinOp::Subtract => "-",
            BinOp::Multiply => "*",
            BinOp::Divide => "/",
            BinOp::Modulo => "%",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Equal => "==",
            BinOp::NotEqual => "!=",
            BinOp::LessThan => "<",
            BinOp::LessThanEqual => "<=",
            BinOp::GreaterThan => ">",
            BinOp::GreaterThanEqual => ">=",
            BinOp::Contains => "in",
            BinOp::NotContains => "not in",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Abstract Term
    Term(Vec<TermPart>),
    /// If: expr2 if expr1 else expr3
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    /// Unary Operation: -expr, !expr, etc.
    UnOp(UnOp, Box<Expr>),
    /// Binary Operation: expr1 + expr2, expr1 - expr2, etc.
    BinOp(BinOp, Box<Expr>, Box<Expr>),
}
