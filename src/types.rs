//! Data types used in the application, these are the types that are used to interact with the
//! database. Every type that is used in the database should be defined here. This is to ensure
//! that the types are consistent across the application and that the database schema is
//! consistent. Every time we have an option wrapping an integer, it means that the field is an
//! auto-incrementing primary key or a foreign key, the option should always be the `Some` after
//! it has been flushed to the DB.

use std::io::Read;

use color_eyre::Result;
use convert_case::{Case, Casing};
use edit::edit;
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantArray, VariantNames};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTest {
    pub(crate) id: Option<i64>,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) creation_date: String,
    pub(crate) last_updated: String,
    pub(crate) last_used_date: Option<String>,
    pub(crate) times_used: u32,
    pub(crate) steps: Vec<PatuiStepDetails>,
}

impl Default for PatuiTest {
    fn default() -> Self {
        let now: String = chrono::Local::now().to_string();

        PatuiTest {
            id: None,
            name: "Default".to_string(),
            description: "Default template".to_string(),
            creation_date: now.clone(),
            last_updated: now,
            last_used_date: None,
            times_used: 0,
            steps: vec![PatuiStepDetails::Shell(PatuiStepShell {
                shell: Some("bash".to_string()),
                contents: "echo 'Hello, world!'".to_string(),
                location: None,
            })],
        }
    }
}

impl PatuiTest {
    pub(crate) fn from_yaml_str(yaml: &str) -> Result<Self> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;

        let now: String = chrono::Local::now().to_string();

        let test = PatuiTest {
            id: None,
            name: yaml_test.name,
            description: yaml_test.description,
            creation_date: now.clone(),
            last_updated: now,
            last_used_date: None,
            times_used: 0,
            steps: yaml_test.steps,
        };

        Ok(test)
    }

    pub(crate) fn edit_yaml(mut yaml_str: String, id: Option<i64>) -> Result<Self> {
        loop {
            yaml_str = edit(&yaml_str)?;
            match PatuiTest::from_yaml_str(&yaml_str) {
                Ok(mut test) => {
                    test.id = id;
                    return Ok(test);
                }
                Err(e) => {
                    eprintln!("Failed to parse yaml: {e}\nPress any key to continue editing or Ctrl-C to cancel...");
                    let buffer = &mut [0u8];
                    let _ = std::io::stdin().read_exact(buffer);
                }
            };
        }
    }

    pub(crate) fn to_editable_yaml_string(&self) -> Result<String> {
        let yaml_test = PatuiTestEditable {
            name: self.name.clone(),
            description: self.description.clone(),
            steps: self.steps.clone(),
        };

        Ok(serde_yaml::to_string(&yaml_test)?)
    }

    pub(crate) fn to_min_display_test(&self) -> Result<PatuiTestMinDisplay> {
        Ok(PatuiTestMinDisplay {
            id: self
                .id
                .ok_or_else(|| color_eyre::eyre::eyre!("No ID found"))?,
            name: self.name.clone(),
            description: self.description.clone(),
        })
    }

    pub(crate) fn to_edited_test(&self, status: String) -> PatuiTestEditStatus {
        PatuiTestEditStatus {
            id: self.id,
            name: Some(self.name.clone()),
            description: Some(self.description.clone()),
            status,
        }
    }

    pub(crate) fn id(&self) -> Result<i64> {
        self.id
            .ok_or_else(|| color_eyre::eyre::eyre!("No ID found"))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestMinDisplay {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestEditStatus {
    pub(crate) id: Option<i64>,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestEditable {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) steps: Vec<PatuiStepDetails>,
}

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Deserialize,
    Serialize,
    EnumIter,
    EnumDiscriminants,
    IntoStaticStr,
    VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum PatuiStepDetails {
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
}

impl PatuiStepDetails {
    pub(crate) fn get_display_yaml(&self) -> Result<Vec<String>> {
        let mut ret = vec![];

        let name: &'static str = self.into();
        ret.push(format!("- {}:", name.to_case(Case::Pascal)));
        let yaml = self.inner_yaml()?;
        yaml.lines().for_each(|line| {
            ret.push(format!("    {}", line));
        });

        Ok(ret)
    }

    pub(crate) fn inner_yaml(&self) -> Result<String> {
        Ok(match self {
            PatuiStepDetails::Shell(shell) => serde_yaml::to_string(shell)?,
            PatuiStepDetails::Assertion(assertion) => serde_yaml::to_string(assertion)?,
        })
    }

    pub(crate) fn to_editable_yaml(&self) -> Result<String> {
        match self {
            PatuiStepDetails::Shell(shell) => Ok(serde_yaml::to_string(&shell.contents)?),
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
            PatuiStepDetails::Shell(shell_step) => {
                let contents = serde_yaml::from_str::<PatuiStepShell>(yaml)?;
                PatuiStepDetails::Shell(PatuiStepShell {
                    shell: shell_step.shell.clone(),
                    contents: contents.contents,
                    location: shell_step.location.clone(),
                })
            }
            _ => serde_yaml::from_str::<PatuiStepDetails>(yaml)?,
        })
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepShell {
    pub(crate) shell: Option<String>,
    pub(crate) contents: String,
    pub(crate) location: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertion {
    pub(crate) assertion: PatuiStepAssertionType,
    pub(crate) negate: bool,
    pub(crate) lhs: String,
    pub(crate) rhs: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize, EnumIter, VariantArray)]
pub(crate) enum PatuiStepAssertionType {
    #[default]
    Equal,
    Contains,
}

#[cfg(test)]
mod tests {
    use super::*;
    use assertor::*;
    use textwrap::dedent;

    #[test]
    fn test_from_simple_yaml_str() {
        let yaml = dedent(
            r#"
            name: test name
            description: test description
            steps: []
            "#,
        );

        let test = PatuiTest::from_yaml_str(&yaml).unwrap();

        assert_that!(test.name).is_equal_to("test name".to_string());
        assert_that!(test.description).is_equal_to("test description".to_string());
        assert_that!(test.steps).is_empty();
    }

    #[test]
    fn test_from_yaml_str_with_steps() {
        let yaml = dedent(
            r#"
            name: test name
            description: test description
            steps:
              - !Shell
                shell: bash
                contents: echo 'Hello, world!'
              - !Assertion
                assertion: Equal
                negate: false
                lhs: foo
                rhs: bar
            "#,
        );

        let test = PatuiTest::from_yaml_str(&yaml).unwrap();

        assert_that!(test.name).is_equal_to("test name".to_string());
        assert_that!(test.description).is_equal_to("test description".to_string());
        assert_that!(test.steps).has_length(2);
        assert_that!(test.steps[0]).is_equal_to(PatuiStepDetails::Shell(PatuiStepShell {
            shell: Some("bash".to_string()),
            contents: "echo 'Hello, world!'".to_string(),
            location: None,
        }));
        assert_that!(test.steps[1]).is_equal_to(PatuiStepDetails::Assertion(PatuiStepAssertion {
            assertion: PatuiStepAssertionType::Equal,
            negate: false,
            lhs: "foo".to_string(),
            rhs: "bar".to_string(),
        }));
    }

    #[test]
    fn test_from_bad_yaml_str_errors() {
        let yaml = dedent(
            r#"
            foo: bar
            "#,
        );

        let test = PatuiTest::from_yaml_str(&yaml);

        assert_that!(test).is_err();
    }
}
