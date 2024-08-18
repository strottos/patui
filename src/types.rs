//! Data types used in the application, these are the types that are used to interact with the
//! database. Every type that is used in the database should be defined here. This is to ensure
//! that the types are consistent across the application and that the database schema is
//! consistent. Every time we have an option wrapping an integer, it means that the field is an
//! auto-incrementing primary key or a foreign key, the option should always be the `Some` after
//! it has been flushed to the DB.

use std::io::Read;

use color_eyre::Result;
use edit::edit;
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, VariantArray, VariantNames};

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

    pub(crate) fn edit_yaml(mut yaml_str: String) -> Result<Self> {
        loop {
            yaml_str = edit(&yaml_str)?;
            match PatuiTest::from_yaml_str(&yaml_str) {
                Ok(test) => {
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

    pub(crate) fn edit_with_yaml(&mut self, yaml: &str) -> Result<()> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;
        self.name = yaml_test.name;
        self.description = yaml_test.description;
        self.steps = yaml_test.steps;

        Ok(())
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
    Debug, Clone, Eq, PartialEq, Deserialize, Serialize, EnumIter, EnumDiscriminants, VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum PatuiStepDetails {
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
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

    #[test]
    fn edit_with_yaml() {
        let mut test = PatuiTest {
            id: Some(1),
            name: "test name".to_string(),
            description: "test description".to_string(),
            creation_date: "2021-09-01T00:00:00".to_string(),
            last_updated: "2021-09-01T00:00:00".to_string(),
            last_used_date: None,
            times_used: 0,
            steps: vec![],
        };

        test.edit_with_yaml(
            r#"
            name: new name
            description: new description
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
        )
        .unwrap();

        assert_that!(test.name).is_equal_to("new name".to_string());
        assert_that!(test.description).is_equal_to("new description".to_string());
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
    fn edit_with_bad_yaml_no_changes() {
        let mut test = PatuiTest {
            id: Some(1),
            name: "test name".to_string(),
            description: "test description".to_string(),
            creation_date: "2021-09-01T00:00:00".to_string(),
            last_updated: "2021-09-01T00:00:00".to_string(),
            last_used_date: None,
            times_used: 0,
            steps: vec![],
        };

        let expected_test = test.clone();

        let output = test.edit_with_yaml(
            r#"
            name: new name
            description: new description
            steps:
              - This: Bad
            "#,
        );

        assert_that!(output).is_err();
        assert_that!(test).is_equal_to(expected_test);
    }
}
