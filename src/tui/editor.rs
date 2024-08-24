use color_eyre::{eyre::eyre, Result};

use crate::types::{PatuiStepDetails, PatuiTest};

pub(crate) fn create_test() -> Result<PatuiTest> {
    let template = crate::types::PatuiTest::default().to_editable_yaml_string()?;
    let test = PatuiTest::edit_yaml(template, None)?;

    Ok(test)
}

pub(crate) fn edit_test(test: PatuiTest) -> Result<PatuiTest> {
    let template = test.to_editable_yaml_string()?;
    let test = PatuiTest::edit_yaml(template, test.id)?;

    Ok(test)
}

pub(crate) fn edit_step(mut test: PatuiTest, step_num: usize) -> Result<PatuiTest> {
    let step = test
        .steps
        .get(step_num)
        .ok_or_else(|| eyre!("Step {} not found", step_num))?;
    let template = step.to_editable_yaml()?;
    let mut step = PatuiStepDetails::edit_yaml(template, step)?;

    test.steps.get_mut(step_num).replace(&mut step);

    Ok(test)
}
