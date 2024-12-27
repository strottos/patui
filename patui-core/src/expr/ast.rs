use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for PatuiExpr
#[derive(Error, Debug)]
pub enum PatuiExprError {}

/// PatuiExpr is the type used for expressions in Patui. This is the entry point for the AST for
/// expressions in Patui. They are mostly built from strings.
///
/// See documentation TODO: Link.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PatuiExpr {
    raw: String,
}

impl PatuiExpr {
    fn try_from_str(value: &str) -> Result<Self, PatuiExprError> {
        Ok(PatuiExpr {
            raw: value.to_string(),
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
