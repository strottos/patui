//! The main library for Patui. This library is used to run tests and manage them.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod expr;
mod templates;
mod utils;

pub use expr::{PatuiExpr, PatuiExprError};
pub use templates::PatuiTest;
