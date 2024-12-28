//! The main library for Patui. This library is used to run tests and manage them.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod expr;
// mod runner;   Concentrating on expressions first, suppress these warnings for now
mod templates;
mod utils;

pub use expr::{
    // TODO: Remove this later
    eval,
    PatuiExpr,
    PatuiExprError,
};
pub use templates::PatuiTest;
