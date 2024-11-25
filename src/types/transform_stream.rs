//! Types related to streaming and transforming.

use serde::{Deserialize, Serialize};

use super::expr::PatuiExpr;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepTransformStreamFlavour {
    Utf8,
    #[default]
    Utf8Lines,
    Json,
    Yaml,
    Toml,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStreamEditable {
    pub(crate) r#in: String,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStream {
    pub(crate) r#in: PatuiExpr,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}
