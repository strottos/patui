//! The main library for Patui. This library is used to run tests and manage them.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod expr;
mod runner;
mod templates;
mod utils;

#[allow(missing_docs)]
pub mod ptplugin {
    tonic::include_proto!("ptplugin");
}

pub use expr::{eval_patui_expr, EvalError, PatuiData, PatuiDataInner, PatuiExpr, PatuiExprError};
pub use runner::{
    PatuiEvent, PatuiEventWithTimestamp, PatuiResultSuccess, PatuiResultType, PatuiRun,
};
pub use templates::PatuiTest;
