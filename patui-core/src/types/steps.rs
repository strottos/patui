use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, IntoStaticStr, VariantNames};
use thiserror::Error;

use super::expr::{PatuiExpr, PatuiExprError};

#[derive(Debug, Error)]
pub enum PatuiStepError {
    #[error("Error converting step: {0}")]
    Conversion(#[from] PatuiExprError),
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// PatuiStep is the type used for steps after they have been saved to the
/// database. This is used for running tests and displaying them to the user.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStep {
    pub(crate) name: String,
    pub(crate) when: Option<String>,
    pub(crate) depends_on: Vec<PatuiStep>,
    pub(crate) details: PatuiStepDetails,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    EnumDiscriminants,
    IntoStaticStr,
    VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum PatuiStepDetails {
    Read(PatuiStepRead),
    Write(PatuiStepWrite),
    Sender(PatuiStepSender),
    TransformStream(PatuiStepTransformStream),
    Assertion(PatuiStepAssertion),
    Plugin(PatuiStepPlugin),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepRead {
    pub(crate) r#in: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWrite {
    pub(crate) out: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertion {
    pub(crate) expr: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepSender {
    pub(crate) expr: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepPlugin {
    pub(crate) path: String, // TODO: Find a better solution when we're publishing plugins
    pub(crate) config: HashMap<String, PatuiExpr>,
    pub(crate) r#in: HashMap<String, PatuiExpr>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepTransformStreamFlavour {
    Utf8,
    #[default]
    Utf8Lines,
    Json,
    Yaml,
    Toml,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStream {
    pub(crate) r#in: PatuiExpr,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}

// Editable types

/// PatuiStepEditable is to endable users ability to edit steps before they
/// are saved to the database, similar to PatuiTestEditable.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepEditable {
    pub(crate) name: String,
    pub(crate) when: Option<Option<String>>,
    pub(crate) depends_on: Option<Vec<PatuiStepEditable>>,
    pub(crate) details: PatuiStepDetailsEditable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDetailsEditable {
    TransformStream(PatuiStepTransformStreamEditable),
    Read(PatuiStepReadEditable),
    Write(PatuiStepWriteEditable),
    Assertion(PatuiStepAssertionEditable),
    Sender(PatuiStepSenderEditable),
    Plugin(PatuiStepPluginEditable),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepReadEditable {
    pub(crate) r#in: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWriteEditable {
    pub(crate) out: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertionEditable {
    pub(crate) expr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepSenderEditable {
    pub(crate) expr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepPluginEditable {
    pub(crate) path: String, // TODO: Find a better solution when we're publishing plugins
    pub(crate) config: Option<HashMap<String, String>>,
    pub(crate) r#in: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStreamEditable {
    pub(crate) r#in: String,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}

impl TryFrom<&PatuiStepEditable> for PatuiStep {
    type Error = PatuiStepError;

    fn try_from(value: &PatuiStepEditable) -> Result<Self, Self::Error> {
        Ok(PatuiStep {
            name: value.name.clone(),
            when: value.when.clone().unwrap_or(None),
            depends_on: value
                .depends_on
                .as_ref()
                .map(|x| x.iter().map(|x| x.try_into()).collect())
                .unwrap_or_else(|| Ok(Vec::new()))?,
            details: match &value.details {
                PatuiStepDetailsEditable::TransformStream(stream) => {
                    PatuiStepDetails::TransformStream(PatuiStepTransformStream {
                        r#in: (&stream.r#in[..]).try_into()?,
                        flavour: stream.flavour.clone(),
                    })
                }
                PatuiStepDetailsEditable::Assertion(assertion) => {
                    PatuiStepDetails::Assertion(PatuiStepAssertion {
                        expr: (&assertion.expr[..]).try_into()?,
                    })
                }
                PatuiStepDetailsEditable::Read(patui_step_read_editable) => {
                    PatuiStepDetails::Read(PatuiStepRead {
                        r#in: (&patui_step_read_editable.r#in[..]).try_into()?,
                    })
                }
                PatuiStepDetailsEditable::Write(patui_step_write_editable) => {
                    PatuiStepDetails::Write(PatuiStepWrite {
                        out: (&patui_step_write_editable.out[..]).try_into()?,
                    })
                }
                PatuiStepDetailsEditable::Sender(patui_step_sender_editable) => {
                    PatuiStepDetails::Sender(PatuiStepSender {
                        expr: (&patui_step_sender_editable.expr[..]).try_into()?,
                    })
                }
                PatuiStepDetailsEditable::Plugin(patui_step_plugin_editable) => {
                    PatuiStepDetails::Plugin(PatuiStepPlugin {
                        path: patui_step_plugin_editable.path.clone(),
                        config: match &patui_step_plugin_editable.config {
                            Some(config) => config
                                .into_iter()
                                .map(|(k, v)| match TryInto::<PatuiExpr>::try_into(&v[..]) {
                                    Ok(v) => Ok((k.clone(), v)),
                                    Err(e) => Err(e),
                                })
                                .collect::<Result<_, _>>()?,
                            None => HashMap::new(),
                        },
                        r#in: match &patui_step_plugin_editable.r#in {
                            Some(r#in) => r#in
                                .into_iter()
                                .map(|(k, v)| match TryInto::<PatuiExpr>::try_into(&v[..]) {
                                    Ok(v) => Ok((k.clone(), v)),
                                    Err(e) => Err(e),
                                })
                                .collect::<Result<_, _>>()?,
                            None => HashMap::new(),
                        },
                    })
                }
            },
        })
    }
}
