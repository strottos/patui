mod other;
mod transform_stream;

use std::collections::HashMap;

use bytes::Bytes;
use convert_case::{Case, Casing};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoStaticStr, VariantNames};

pub(crate) use other::{
    PatuiStepAssertion, PatuiStepAssertionEditable, PatuiStepPlugin, PatuiStepPluginEditable,
    PatuiStepRead, PatuiStepReadEditable, PatuiStepSender, PatuiStepSenderEditable, PatuiStepWrite,
    PatuiStepWriteEditable,
};
pub(crate) use transform_stream::{PatuiStepTransformStream, PatuiStepTransformStreamEditable};

#[cfg(test)]
pub(crate) use transform_stream::PatuiStepTransformStreamFlavour;

use super::PatuiExpr;

/// PatuiStepEditable is to endable users ability to edit steps before they
/// are saved to the database, similar to PatuiTestEditable.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepEditable {
    pub(crate) name: String,
    pub(crate) when: Option<Option<String>>,
    pub(crate) depends_on: Option<Vec<PatuiStepEditable>>,
    pub(crate) details: PatuiStepDetailsEditable,
}

impl From<PatuiStep> for PatuiStepEditable {
    fn from(step: PatuiStep) -> Self {
        PatuiStepEditable {
            name: step.name,
            when: Some(step.when),
            depends_on: Some(step.depends_on.into_iter().map(|x| x.into()).collect()),
            details: match step.details {
                PatuiStepDetails::TransformStream(stream) => {
                    PatuiStepDetailsEditable::TransformStream(PatuiStepTransformStreamEditable {
                        r#in: stream.r#in.into(),
                        flavour: stream.flavour,
                    })
                }
                PatuiStepDetails::Assertion(assertion) => {
                    PatuiStepDetailsEditable::Assertion(PatuiStepAssertionEditable {
                        expr: assertion.expr.into(),
                    })
                }
                PatuiStepDetails::Read(patui_step_read) => {
                    PatuiStepDetailsEditable::Read(PatuiStepReadEditable {
                        r#in: patui_step_read.r#in.into(),
                    })
                }
                PatuiStepDetails::Write(patui_step_write) => {
                    PatuiStepDetailsEditable::Write(PatuiStepWriteEditable {
                        out: patui_step_write.out.into(),
                    })
                }
                PatuiStepDetails::Sender(patui_step_sender) => {
                    PatuiStepDetailsEditable::Sender(PatuiStepSenderEditable {
                        expr: patui_step_sender.expr.into(),
                    })
                }
                PatuiStepDetails::Plugin(patui_step_plugin) => {
                    PatuiStepDetailsEditable::Plugin(PatuiStepPluginEditable {
                        path: patui_step_plugin.path.clone(),
                        config: Some(
                            patui_step_plugin
                                .config
                                .into_iter()
                                .map(|(k, v)| (k, v.into()))
                                .collect(),
                        ),
                        r#in: Some(
                            patui_step_plugin
                                .r#in
                                .into_iter()
                                .map(|(k, v)| (k, v.into()))
                                .collect(),
                        ),
                    })
                }
            },
        }
    }
}

impl From<&PatuiStep> for PatuiStepEditable {
    fn from(value: &PatuiStep) -> Self {
        PatuiStepEditable {
            name: value.name.clone(),
            when: Some(value.when.clone()),
            depends_on: Some(value.depends_on.iter().map(|x| x.into()).collect()),
            details: match &value.details {
                PatuiStepDetails::TransformStream(stream) => {
                    PatuiStepDetailsEditable::TransformStream(PatuiStepTransformStreamEditable {
                        r#in: stream.r#in.clone().into(),
                        flavour: stream.flavour.clone(),
                    })
                }
                PatuiStepDetails::Assertion(assertion) => {
                    PatuiStepDetailsEditable::Assertion(PatuiStepAssertionEditable {
                        expr: (&assertion.expr).into(),
                    })
                }
                PatuiStepDetails::Read(patui_step_read) => {
                    PatuiStepDetailsEditable::Read(PatuiStepReadEditable {
                        r#in: (&patui_step_read.r#in).into(),
                    })
                }
                PatuiStepDetails::Write(patui_step_write) => {
                    PatuiStepDetailsEditable::Write(PatuiStepWriteEditable {
                        out: (&patui_step_write.out).into(),
                    })
                }
                PatuiStepDetails::Sender(patui_step_sender) => {
                    PatuiStepDetailsEditable::Sender(PatuiStepSenderEditable {
                        expr: (&patui_step_sender.expr).into(),
                    })
                }
                PatuiStepDetails::Plugin(patui_step_plugin) => {
                    PatuiStepDetailsEditable::Plugin(PatuiStepPluginEditable {
                        path: patui_step_plugin.path.clone(),
                        config: Some(
                            (&patui_step_plugin.config)
                                .into_iter()
                                .map(|(k, v)| (k.clone(), v.into()))
                                .collect(),
                        ),
                        r#in: Some(
                            (&patui_step_plugin.r#in)
                                .into_iter()
                                .map(|(k, v)| (k.clone(), v.into()))
                                .collect(),
                        ),
                    })
                }
            },
        }
    }
}

/// PatuiStep is the type used for steps after they have been saved to the
/// database. This is used for running tests and displaying them to the user.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStep {
    pub(crate) name: String,
    pub(crate) when: Option<String>,
    pub(crate) depends_on: Vec<PatuiStep>,
    pub(crate) details: PatuiStepDetails,
}

impl TryFrom<&PatuiStepEditable> for PatuiStep {
    type Error = eyre::Error;

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
                                .collect::<Result<_>>()?,
                            None => HashMap::new(),
                        },
                        r#in: match &patui_step_plugin_editable.r#in {
                            Some(r#in) => r#in
                                .into_iter()
                                .map(|(k, v)| match TryInto::<PatuiExpr>::try_into(&v[..]) {
                                    Ok(v) => Ok((k.clone(), v)),
                                    Err(e) => Err(e),
                                })
                                .collect::<Result<_>>()?,
                            None => HashMap::new(),
                        },
                    })
                }
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDetailsEditable {
    TransformStream(PatuiStepTransformStreamEditable),
    Read(PatuiStepReadEditable),
    Write(PatuiStepWriteEditable),
    Assertion(PatuiStepAssertionEditable),
    Sender(PatuiStepSenderEditable),
    Plugin(PatuiStepPluginEditable),
}

#[derive(
    Debug, Clone, PartialEq, Deserialize, Serialize, EnumDiscriminants, IntoStaticStr, VariantNames,
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

impl PatuiStepDetails {
    pub(crate) fn get_display_yaml(&self) -> Result<String> {
        let mut ret = String::new();

        let name: &'static str = self.into();
        ret += &format!("- {}:\n", name.to_case(Case::Pascal));
        let yaml = self.inner_yaml()?;
        yaml.lines().for_each(|line| {
            ret += &format!("    {}\n", line);
        });

        Ok(ret.trim().to_string())
    }

    pub(crate) fn inner_yaml(&self) -> Result<String> {
        Ok(match self {
            PatuiStepDetails::TransformStream(stream) => serde_yaml::to_string(stream)?,
            PatuiStepDetails::Assertion(assertion) => serde_yaml::to_string(assertion)?,
            PatuiStepDetails::Read(reader) => serde_yaml::to_string(reader)?,
            PatuiStepDetails::Write(writer) => serde_yaml::to_string(writer)?,
            PatuiStepDetails::Sender(sender) => serde_yaml::to_string(sender)?,
            PatuiStepDetails::Plugin(plugin) => serde_yaml::to_string(plugin)?,
        })
    }

    // pub(crate) fn edit_yaml(mut yaml_str: String, step: &PatuiStepDetails) -> Result<Self> {
    //     loop {
    //         yaml_str = edit(&yaml_str)?;
    //         match PatuiStepDetails::from_yaml_str(&yaml_str, step) {
    //             Ok(step) => {
    //                 return Ok(step);
    //             }
    //             Err(e) => {
    //                 eprintln!("Failed to parse yaml: {e}\nPress any key to continue editing or Ctrl-C to cancel...");
    //                 let buffer = &mut [0u8];
    //                 let _ = std::io::stdin().read_exact(buffer);
    //             }
    //         };
    //     }
    // }

    // pub(crate) fn from_yaml_str(yaml: &str, step: &PatuiStepDetails) -> Result<Self> {
    //     Ok(match step {
    //         _ => serde_yaml::from_str::<PatuiStepDetails>(yaml)?,
    //     })
    // }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct PatuiStepData {
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
    pub(crate) data: PatuiStepDataFlavour,
}

impl PartialEq for PatuiStepData {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl PatuiStepData {
    pub(crate) fn new(data: PatuiStepDataFlavour) -> Self {
        let timestamp = chrono::Utc::now();
        Self { timestamp, data }
    }

    // pub(crate) fn into_data(self) -> PatuiStepDataFlavour {
    //     self.data
    // }

    #[cfg(test)]
    pub(crate) fn data(&self) -> &PatuiStepDataFlavour {
        &self.data
    }
}

impl TryFrom<super::ptplugin::PatuiStepData> for PatuiStepData {
    type Error = eyre::Error;

    fn try_from(value: super::ptplugin::PatuiStepData) -> Result<Self, Self::Error> {
        Ok(PatuiStepData::new(rmp_serde::from_slice(&value.bytes)?))
    }
}

impl TryFrom<PatuiStepData> for super::ptplugin::PatuiStepData {
    type Error = eyre::Error;

    fn try_from(value: PatuiStepData) -> Result<Self, Self::Error> {
        Ok(super::ptplugin::PatuiStepData {
            bytes: rmp_serde::to_vec(&value.data)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDataFlavour {
    Null,
    Bool(bool),
    Bytes(Bytes),
    String(String),
    Integer(String),
    Float(String),
    Array(Vec<PatuiStepDataFlavour>),
    Map(HashMap<String, PatuiStepDataFlavour>),
    Set(Vec<PatuiStepDataFlavour>),
}

impl PatuiStepDataFlavour {
    pub(crate) fn as_bytes(&self) -> Result<&Bytes> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            _ => Err(eyre!("not bytes")),
        }
    }

    // pub(crate) fn as_number(&self) -> Result<i64> {
    //     match self {
    //         Self::Number(number) => Ok(*number),
    //         _ => Err(eyre!("not number")),
    //     }
    // }

    // pub(crate) fn is_bytes(&self) -> bool {
    //     matches!(self, Self::Bytes(_))
    // }

    // pub(crate) fn is_string(&self) -> bool {
    //     matches!(self, Self::String(_))
    // }

    // pub(crate) fn is_number(&self) -> bool {
    //     matches!(self, Self::Number(_))
    // }

    #[cfg(test)]
    pub(crate) fn is_object(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    // pub(crate) fn is_yaml(&self) -> bool {
    //     matches!(self, Self::Yaml(_))
    // }
}

impl From<bool> for PatuiStepDataFlavour {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Bytes> for PatuiStepDataFlavour {
    fn from(value: Bytes) -> Self {
        Self::Bytes(value)
    }
}

impl From<String> for PatuiStepDataFlavour {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i64> for PatuiStepDataFlavour {
    fn from(value: i64) -> Self {
        Self::Integer(format!("{}", value))
    }
}

impl From<f64> for PatuiStepDataFlavour {
    fn from(value: f64) -> Self {
        Self::Float(format!("{}", value))
    }
}

impl TryFrom<serde_json::Value> for PatuiStepDataFlavour {
    type Error = eyre::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Null => Ok(Self::Null),
            serde_json::Value::Bool(value) => Ok(Self::Bool(value)),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Ok(Self::Integer(format!("{}", value)))
                } else if let Some(value) = value.as_f64() {
                    Ok(Self::Float(format!("{}", value)))
                } else {
                    Err(eyre!("Invalid number"))
                }
            }
            serde_json::Value::String(value) => Ok(Self::String(value)),
            serde_json::Value::Array(value) => {
                let value = value
                    .into_iter()
                    .map(|x| x.try_into())
                    .collect::<Result<Vec<_>>>()?;
                Ok(Self::Array(value))
            }
            serde_json::Value::Object(value) => {
                let value = value
                    .into_iter()
                    .map(|(k, v)| Ok((k, v.try_into()?)))
                    .collect::<Result<HashMap<_, _>>>()?;
                Ok(Self::Map(value))
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDataTransfer {
    #[default]
    None,
    Fixed(PatuiStepDataFlavour),
    Ref(Box<(PatuiStep, String)>),
}
