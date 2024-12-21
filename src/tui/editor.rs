use eyre::Result;

use crate::{
    db::PatuiTestDb,
    types::{PatuiTest, PatuiTestDetails},
};

pub(crate) fn create_test() -> Result<PatuiTestDetails> {
    let template = crate::types::PatuiTestDetails::default().to_editable_yaml_string()?;
    let test = PatuiTestDetails::edit_yaml(template)?;

    Ok(test)
}

pub(crate) fn edit_test(test: PatuiTestDb) -> Result<PatuiTest> {
    let template = test.to_editable_yaml_string()?;
    let ret = PatuiTest::edit_from_details(test.id, PatuiTestDetails::edit_yaml(template)?);

    Ok(ret)
}

// pub(crate) fn edit_step(_test: PatuiTestDb, _step_num: PatuiTestStepId) -> Result<PatuiTestDb> {
//     let step = test
//         .details
//         .steps
//         .get(usize::from(step_num))
//         .ok_or_else(|| eyre!("Step {} not found", step_num))?;
//     let template = step.to_editable_yaml()?;
//     let mut step = PatuiStep::edit_yaml(template, step)?;
//
//     test.details
//         .steps
//         .get_mut(usize::from(step_num))
//         .replace(&mut step);
//
//     Ok(test)
// }
