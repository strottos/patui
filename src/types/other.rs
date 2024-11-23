use either::Either;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, VariantArray};

use super::expr::PatuiExpr;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepReadEditable {
    pub(crate) r#in: PatuiExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepRead {
    pub(crate) r#in: PatuiExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWriteEditable {
    pub(crate) out: PatuiExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWrite {
    pub(crate) out: PatuiExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertionEditable {
    pub(crate) expr: PatuiExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertion {
    pub(crate) expr: PatuiExpr,
}
