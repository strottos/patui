mod other;
mod transform_stream;

use std::io::Read;

use bytes::Bytes;
use convert_case::{Case, Casing};
use edit::edit;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoStaticStr, VariantArray, VariantNames};

pub(crate) use other::{
    PatuiStepAssertion, PatuiStepAssertionEditable, PatuiStepRead, PatuiStepReadEditable,
    PatuiStepSender, PatuiStepSenderEditable, PatuiStepWrite, PatuiStepWriteEditable,
};
pub(crate) use transform_stream::{
    PatuiStepTransformStream, PatuiStepTransformStreamEditable, PatuiStepTransformStreamFlavour,
};

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
    // TODO: Plugin
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
            PatuiStepDetails::Read(patui_step_read) => serde_yaml::to_string(patui_step_read)?,
            PatuiStepDetails::Write(patui_step_write) => serde_yaml::to_string(patui_step_write)?,
            PatuiStepDetails::Sender(patui_step_sender) => {
                serde_yaml::to_string(patui_step_sender)?
            }
        })
    }

    pub(crate) fn to_editable_yaml(&self) -> Result<String> {
        match self {
            _ => self.inner_yaml(),
        }
    }

    pub(crate) fn edit_yaml(mut yaml_str: String, step: &PatuiStepDetails) -> Result<Self> {
        loop {
            yaml_str = edit(&yaml_str)?;
            match PatuiStepDetails::from_yaml_str(&yaml_str, step) {
                Ok(step) => {
                    return Ok(step);
                }
                Err(e) => {
                    eprintln!("Failed to parse yaml: {e}\nPress any key to continue editing or Ctrl-C to cancel...");
                    let buffer = &mut [0u8];
                    let _ = std::io::stdin().read_exact(buffer);
                }
            };
        }
    }

    pub(crate) fn from_yaml_str(yaml: &str, step: &PatuiStepDetails) -> Result<Self> {
        Ok(match step {
            _ => serde_yaml::from_str::<PatuiStepDetails>(yaml)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepData {
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
    pub(crate) data: PatuiStepDataFlavour,
}

impl PatuiStepData {
    pub(crate) fn new(data: PatuiStepDataFlavour) -> Self {
        let timestamp = chrono::Utc::now();
        Self { timestamp, data }
    }

    pub(crate) fn into_data(self) -> PatuiStepDataFlavour {
        self.data
    }

    pub(crate) fn data(&self) -> &PatuiStepDataFlavour {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDataFlavour {
    Bytes(Bytes),
    String(String),
    Number(i64),
    Json(serde_json::Value),
    Yaml(serde_yaml::Value),
}

impl PatuiStepDataFlavour {
    pub(crate) fn as_bytes(&self) -> Result<&Bytes> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            _ => Err(eyre!("not bytes")),
        }
    }

    pub(crate) fn as_number(&self) -> Result<i64> {
        match self {
            Self::Number(number) => Ok(*number),
            _ => Err(eyre!("not number")),
        }
    }

    pub(crate) fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    pub(crate) fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub(crate) fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    pub(crate) fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    pub(crate) fn is_yaml(&self) -> bool {
        matches!(self, Self::Yaml(_))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDataTransfer {
    #[default]
    None,
    Fixed(PatuiStepDataFlavour),
    Ref(Box<(PatuiStep, String)>),
}
