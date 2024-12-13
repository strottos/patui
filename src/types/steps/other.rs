use serde::{Deserialize, Serialize};
use strum::{EnumIter, VariantArray};

use crate::types::expr::PatuiExpr;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepReadEditable {
    pub(crate) r#in: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepRead {
    pub(crate) r#in: PatuiExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWriteEditable {
    pub(crate) out: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWrite {
    pub(crate) out: PatuiExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertionEditable {
    pub(crate) expr: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertion {
    pub(crate) expr: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepSenderEditable {
    pub(crate) expr: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepSender {
    pub(crate) expr: PatuiExpr,
}
