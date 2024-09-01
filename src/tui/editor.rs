use color_eyre::{eyre::eyre, Result};

use crate::types::{PatuiStepDetails, PatuiTest, PatuiTestDetails, PatuiTestStepId};

pub(crate) fn create_test() -> Result<PatuiTestDetails> {
    let template = crate::types::PatuiTestDetails::default().to_editable_yaml_string()?;
    let test = PatuiTestDetails::edit_yaml(template)?;

    Ok(test)
}

pub(crate) fn edit_test(mut test: PatuiTest) -> Result<PatuiTest> {
    let template = test.details.to_editable_yaml_string()?;
    test.details = PatuiTestDetails::edit_yaml(template)?;

    Ok(test)
}

pub(crate) fn edit_step(mut test: PatuiTest, step_num: PatuiTestStepId) -> Result<PatuiTest> {
    let step = test
        .details
        .steps
        .get(usize::from(step_num))
        .ok_or_else(|| eyre!("Step {} not found", step_num))?;
    let template = step.to_editable_yaml()?;
    let mut step = PatuiStepDetails::edit_yaml(template, step)?;

    test.details
        .steps
        .get_mut(usize::from(step_num))
        .replace(&mut step);

    Ok(test)
}
