//! The main module for the Patui expression parser, abstract syntax tree (AST) and the evaluation
//! code. This module is the entry point for the expression parser and evaluator.
//!
//! The main types in this module are:
//! - PatuiExpr: The AST for expressions in Patui.
//! - PatuiExprError: The error type for PatuiExpr.
//! - PatuiData: The data type for the evaluation of expressions.
//!
//! The main functions in this module are:
//! - PatuiExpr::try_from_str: Convert a string into a PatuiExpr. This is the main entry point for
//!   the AST which can also be accessed as a TryFrom implementation. This is responsible for
//!   parsing a string into the AST.
//! - eval: Evaluate a PatuiExpr with a PatuiData. This is the main entry point for evaluating an
//!   AST against some data.

mod ast;
mod data;
mod eval;
mod lexer;
mod parser;
mod query;
mod visitor;

pub use ast::{PatuiExpr, PatuiExprError};
pub use data::{PatuiData, PatuiDataInner};
pub use eval::EvalError;
pub use query::get_terms as get_expr_terms;

/// Evaluate a PatuiExpr against a PatuiData.
///
/// The PatuiData must have an inner of type Map to be able to evaluate the expression.
pub fn eval_patui_expr(expr: &PatuiExpr, data: &PatuiData) -> Result<PatuiData, EvalError> {
    eval::eval(expr, data)
}

// /// Evaluate a PatuiExpr against a PatuiData returning the minimized PatuiExpr.
// ///
// /// The PatuiData must have an inner of type Map to be able to evaluate the expression.
// pub fn eval_expr_ident(expr: &PatuiExpr, data: &PatuiData) -> Result<String, EvalError> {
//     eval::eval_ident(expr, data)
// }
