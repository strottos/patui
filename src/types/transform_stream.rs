//! Types related to streaming and transforming.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepTransformStreamFlavour {
    Utf8,
    #[default]
    Utf8Lines,
    Json,
    Yaml,
    Toml,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStream {
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}
