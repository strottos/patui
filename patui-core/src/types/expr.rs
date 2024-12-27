use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PatuiExprError {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiExpr {}

impl PatuiExpr {
    fn try_from_str(value: &str) -> Result<Self, PatuiExprError> {
        Ok(PatuiExpr {})
    }
}

impl TryFrom<&str> for PatuiExpr {
    type Error = PatuiExprError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        PatuiExpr::try_from_str(value)
    }
}
