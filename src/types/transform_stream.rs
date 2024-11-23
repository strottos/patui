//! Types related to streaming and transforming.

use serde::{Deserialize, Serialize};

use super::PatuiStepDataTransfer;

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
    pub(crate) input: Option<PatuiStepDataTransfer>,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStream {
    pub(crate) input: PatuiStepDataTransfer,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}
